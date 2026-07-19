use crate::storage;
use crate::{Cli, AgentCommands, PrepareCommands};
use crate::domain::resolver;
use serde_json::json;

pub async fn run(cmd: &AgentCommands, cli: &Cli) -> anyhow::Result<()> {
    match cmd {
        AgentCommands::Doctor => run_doctor(cli).await,
        AgentCommands::Setup => run_setup(cli).await,
        AgentCommands::Prepare(sub) => run_prepare(sub, cli).await,
        AgentCommands::Resolve { query } => run_resolve(query, cli),
        AgentCommands::Context { task } => run_context(task, cli),
        AgentCommands::Refresh { stale_only: _ } => run_refresh(cli).await,
    }
}

/// doctor: report login, profile, cache, PDF skill, data freshness as JSON.
async fn run_doctor(_cli: &Cli) -> anyhow::Result<()> {
    let session = crate::auth::session::Session::load().ok().flatten();
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let profile = repo.get_profile().ok().flatten();
    let terms = repo.list_offering_terms().unwrap_or_default();

    let has_requirements = {
        let p = profile.as_ref();
        if let (Some(dept), Some(y)) = (p.and_then(|p| p.dept_code.as_deref()), p.and_then(|p| p.enroll_year)) {
            dirs::data_dir().or_else(dirs::config_dir).unwrap_or_default()
                .join("courseape").join("snapshots")
                .join(format!("requirements_{}_{}.pdf", y, dept))
                .exists()
        } else { false }
    };

    let has_grades = {
        let snap_dir = dirs::data_dir().or_else(dirs::config_dir).unwrap_or_default()
            .join("courseape").join("snapshots");
        snap_dir.exists() && snap_dir.read_dir().ok()
            .map(|mut d| d.any(|e| e.ok().map(|e| e.file_name().to_string_lossy().starts_with("grades")).unwrap_or(false)))
            .unwrap_or(false)
    };

    let has_analyzed_grades = repo.list_analyzed_grades().ok().map(|g| !g.is_empty()).unwrap_or(false);

    let (current_year, current_sem) = resolver::current_term();
    let current_term_code = format!("{}{}", current_year, current_sem);

    let status = json!({
        "logged_in": session.is_some(),
        "profile_set": profile.is_some(),
        "profile": profile.as_ref().map(|p| json!({
            "student_id": if _cli.no_redact_personal { &p.student_id } else { "***" },
            "dept_code": p.dept_code,
            "dept_name": p.dept_name,
            "enroll_year": p.enroll_year,
            "degree": p.degree,
        })),
        "departments_synced": !repo.list_departments(114).unwrap_or_default().is_empty(),
        "requirements_downloaded": has_requirements,
        "grades_downloaded": has_grades,
        "grades_analyzed": has_analyzed_grades,
        "cached_terms": terms,
        "current_term": current_term_code,
        "next_term": resolver::next_term(),
    });

    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

/// setup: auto-login, sync departments, auto-detect profile from student ID.
async fn run_setup(cli: &Cli) -> anyhow::Result<()> {
    let mut steps_completed = Vec::new();

    // Step 1: Check login
    eprintln!("[1/4] 檢查登入狀態...");
    let _session = match crate::auth::session::Session::load()? {
        Some(s) => {
            eprintln!("  ✓ 已登入");
            steps_completed.push("login_ok");
            s
        }
        None => {
            eprintln!("  尚未登入，嘗試自動登入...");
            // Try env vars or stored credentials
            match crate::auth::keyring::StoredCredentials::load() {
                Ok(Some(creds)) => {
                    let (cookie, login_token) = crate::connectors::itouch::ItouchConnector::login(
                        &creds.student_id, &creds.password
                    ).await?;
                    let session = crate::auth::session::Session {
                        cookie,
                        login_token,
                        logged_in_at: chrono::Utc::now(),
                    };
                    session.save()?;
                    eprintln!("  ✓ 自動登入成功");
                    steps_completed.push("login_ok");
                    session
                }
                _ => {
                    if cli.silent {
                        anyhow::bail!("NOT_LOGGED_IN");
                    }
                    anyhow::bail!("無法自動登入。請先執行 courseape login");
                }
            }
        }
    };

    // Step 2: Get student ID
    eprintln!("[2/4] 讀取學號...");
    let student_id = {
        let db = storage::db::open()?;
        let repo = storage::repo::Repository::new(&db);
        repo.get_profile()?.map(|p| p.student_id).unwrap_or_default()
    };
    let student_id = if student_id.is_empty() {
        // Derive from session or env
        std::env::var("CYCU_USERNAME").unwrap_or_default()
    } else {
        student_id
    };

    if student_id.is_empty() {
        anyhow::bail!("無法取得學號。請執行 courseape login");
    }
    eprintln!("  ✓ 學號: {}", student_id);
    steps_completed.push("student_id_ok");

    // Step 3: Auto-detect profile from student ID
    eprintln!("[3/4] 自動推導個人資料...");
    let enroll_year = resolver::derive_enroll_year(&student_id);
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let current_profile = repo.get_profile()?;

    if current_profile.is_none() {
        let profile = crate::domain::profile::StudentProfile {
            student_id: student_id.clone(),
            dept_code: None,
            dept_name: None,
            enroll_year,
            degree: Some("學士".to_string()),
        };
        repo.upsert_profile(&profile)?;
        eprintln!("  ✓ 自動設定入學年度: {:?}", enroll_year);
        eprintln!("  ⚠ 系所尚未設定。Agent 可從自然語言推導。");
        steps_completed.push("profile_partial");
    } else {
        eprintln!("  ✓ 已有個人資料");
        steps_completed.push("profile_ok");
    }

    // Step 4: Sync departments
    eprintln!("[4/4] 同步系所清單...");
    let depts = repo.list_departments(114)?;
    if depts.is_empty() {
        eprintln!("  正在下載...");
        let result = crate::connectors::necessary_course::NecessaryCourseConnector::query_departments(114).await?;
        let json: serde_json::Value = serde_json::from_slice(&result.body_bytes)?;
        let departments = crate::parsers::department_json::parse_departments(&json, 114)?;
        let count = departments.len();
        repo.upsert_departments(&departments)?;
        let _ = storage::snapshot::SnapshotArchive::save("departments_114", &result.body_bytes);
        eprintln!("  ✓ 同步 {} 個系所", count);
    } else {
        eprintln!("  ✓ 已有 {} 個系所", depts.len());
    }
    steps_completed.push("departments_ok");

    // Output JSON summary
    let summary = json!({
        "status": "ok",
        "steps": steps_completed,
        "student_id": if cli.no_redact_personal { &student_id } else { "***" },
        "enroll_year": enroll_year,
        "profile_needs_dept": current_profile.is_none(),
    });
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

/// prepare: one-shot data preparation for graduation analysis or course planning.
async fn run_prepare(sub: &PrepareCommands, cli: &Cli) -> anyhow::Result<()> {
    match sub {
        PrepareCommands::Graduation => prepare_graduation(cli).await,
        PrepareCommands::Planning { term } => prepare_planning(term, cli).await,
    }
}

async fn prepare_graduation(_cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let profile = repo.get_profile()?.ok_or(crate::error::CourseapeError::ProfileNotSet)?;
    let dept_code = profile.dept_code.as_deref().ok_or(crate::error::CourseapeError::ProfileNotSet)?;
    let enroll_year = profile.enroll_year.ok_or(crate::error::CourseapeError::ProfileNotSet)?;

    let mut result = json!({});

    // 1. Requirements PDF
    eprintln!("[1/3] 下載修業辦法...");
    let snap_dir = dirs::data_dir().or_else(dirs::config_dir).unwrap_or_default()
        .join("courseape").join("snapshots");
    let req_path = snap_dir.join(format!("requirements_{}_{}.pdf", enroll_year, dept_code));
    if req_path.exists() {
        eprintln!("  ✓ 已有修業辦法 PDF");
    } else {
        let res = crate::connectors::necessary_course::NecessaryCourseConnector::download_requirement_pdf(
            enroll_year, dept_code
        ).await?;
        let (hash, path) = storage::snapshot::SnapshotArchive::save(
            &format!("requirements_{}_{}.pdf", enroll_year, dept_code), &res.body_bytes,
        )?;
        repo.upsert_requirement(enroll_year, dept_code, &path.to_string_lossy(), "1.0.0")?;
        eprintln!("  ✓ 已下載 (hash: {}...)", &hash[..16]);
    }
    result["requirements_path"] = json!(req_path.to_string_lossy());

    // 2. Grades HTML
    eprintln!("[2/3] 下載歷年成績...");
    let session = crate::auth::session::Session::load()?.ok_or(crate::error::CourseapeError::NotLoggedIn)?;
    let has_grades = snap_dir.exists() && snap_dir.read_dir().ok()
        .map(|mut d| d.any(|e| e.ok().map(|e| e.file_name().to_string_lossy().starts_with("grades")).unwrap_or(false)))
        .unwrap_or(false);

    if has_grades {
        eprintln!("  ✓ 已有成績資料");
    } else {
        let grade_result = crate::connectors::itouch::ItouchConnector::fetch_grades(&session.cookie).await?;
        let (hash, path) = storage::snapshot::SnapshotArchive::save("grades", &grade_result.body_bytes)?;
        eprintln!("  ✓ 已下載 (hash: {}...)", &hash[..16]);
        result["grade_html_path"] = json!(path.to_string_lossy());
    }

    // 3. Historical offerings
    eprintln!("[3/3] 同步歷史開課資料...");
    let (current_year, current_sem) = resolver::current_term();
    let mut total = 0;
    for year in enroll_year..=current_year {
        for sem in 1..=2 {
            if year == current_year && sem > current_sem { continue; }
            let term = format!("{}{}", year, sem);
            let existing = repo.list_offerings(&term).unwrap_or_default();
            if !existing.is_empty() {
                total += existing.len();
                continue;
            }
            if let Ok(offerings) = crate::cli::courses::fetch_offerings_from_api(&term).await {
                let count = offerings.len();
                repo.upsert_offerings(&term, &offerings)?;
                total += count;
            }
        }
    }
    eprintln!("  ✓ 共 {} 筆開課資料", total);

    result["status"] = json!("ok");
    result["total_offerings"] = json!(total);
    result["has_analyzed_grades"] = json!(repo.list_analyzed_grades().ok().map(|g| !g.is_empty()).unwrap_or(false));

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn prepare_planning(term_arg: &str, _cli: &Cli) -> anyhow::Result<()> {
    let term = if term_arg == "auto" || term_arg.is_empty() {
        resolver::next_term()
    } else {
        term_arg.to_string()
    };

    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);

    let mut result = json!({ "term": term });

    // 1. Ensure offerings are cached
    eprintln!("[1/2] 確認開課資料...");
    let offerings = repo.list_offerings(&term)?;
    if offerings.is_empty() {
        eprintln!("  正在同步 {} 開課清單...", term);
        let fetched = crate::cli::courses::fetch_offerings_from_api(&term).await?;
        let count = fetched.len();
        repo.upsert_offerings(&term, &fetched)?;
        eprintln!("  ✓ 同步 {} 筆", count);
        result["offerings_count"] = json!(count);
    } else {
        eprintln!("  ✓ 已有 {} 筆開課資料", offerings.len());
        result["offerings_count"] = json!(offerings.len());
    }

    // 2. Show shortlist status
    eprintln!("[2/2] 檢查備選清單...");
    let shortlist = repo.list_shortlist(&term)?;
    let profile = repo.get_profile()?;
    let dept_code = profile.as_ref().and_then(|p| p.dept_code.as_deref());
    let planned = repo.get_planned_courses(&term, dept_code)?;
    let report = crate::analysis::conflict::detect_conflicts(&planned);

    result["shortlist_count"] = json!(shortlist.len());
    result["planned_count"] = json!(planned.len());
    result["conflicts"] = json!(report.conflict_count);
    result["status"] = json!("ok");

    eprintln!("  ✓ 備選 {} 門，衝堂 {} 組", shortlist.len(), report.conflict_count);

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// resolve: resolve a natural-language department name to code.
fn run_resolve(query: &str, _cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let departments = repo.list_departments(114)?;

    if departments.is_empty() {
        anyhow::bail!("系所清單尚未同步。請先執行 courseape sync departments --year 114");
    }

    let candidates = resolver::resolve_department(query, &departments);

    let result = json!({
        "query": query,
        "candidates": candidates,
        "auto_select": candidates.first().map(|c| {
            matches!(c.confidence, resolver::MatchConfidence::Exact | resolver::MatchConfidence::High)
        }).unwrap_or(false),
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// context: return agent-needed data state and next steps.
fn run_context(task: &str, _cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let session = crate::auth::session::Session::load().ok().flatten();
    let profile = repo.get_profile().ok().flatten();
    let terms = repo.list_offering_terms().unwrap_or_default();
    let has_analyzed = repo.list_analyzed_grades().ok().map(|g| !g.is_empty()).unwrap_or(false);

    let current_term = resolver::current_term_code();
    let next = resolver::next_term();

    let mut result = json!({
        "task": task,
        "logged_in": session.is_some(),
        "profile_set": profile.is_some(),
        "current_term": current_term,
        "next_term": next,
    });

    let mut next_steps: Vec<String> = Vec::new();

    if session.is_none() {
        next_steps.push("courseape login".to_string());
    }

    if profile.is_none() {
        next_steps.push("courseape setup".to_string());
    }

    match task {
        "graduation" => {
            let has_req = profile.as_ref().and_then(|p| {
                let dept = p.dept_code.as_deref()?;
                let year = p.enroll_year?;
                let path = dirs::data_dir().or_else(dirs::config_dir)?
                    .join("courseape").join("snapshots")
                    .join(format!("requirements_{}_{}.pdf", year, dept));
                Some(path.exists())
            }).unwrap_or(false);

            let has_grades = {
                let snap_dir = dirs::data_dir().or_else(dirs::config_dir).unwrap_or_default()
                    .join("courseape").join("snapshots");
                snap_dir.exists() && snap_dir.read_dir().ok()
                    .map(|mut d| d.any(|e| e.ok().map(|e| e.file_name().to_string_lossy().starts_with("grades")).unwrap_or(false)))
                    .unwrap_or(false)
            };

            result["has_requirements"] = json!(has_req);
            result["has_grade_html"] = json!(has_grades);
            result["has_analyzed_grades"] = json!(has_analyzed);

            if !has_req {
                next_steps.push("courseape prepare graduation".to_string());
            }
            if !has_analyzed {
                next_steps.push("(Agent) 分析成績 HTML 並匯入".to_string());
            }
        }
        "planning" => {
            let has_offerings = terms.contains(&next);
            result["has_next_term_offerings"] = json!(has_offerings);

            if !has_offerings {
                next_steps.push(format!("courseape prepare planning --term {}", next));
            }
            if !has_analyzed {
                next_steps.push("(Agent) 先完成畢業分析".to_string());
            }
        }
        _ => {}
    }

    result["next_steps"] = json!(next_steps);
    result["cached_terms"] = json!(terms);

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// refresh: re-download stale or missing data.
async fn run_refresh(_cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let session = crate::auth::session::Session::load()?.ok_or(crate::error::CourseapeError::NotLoggedIn)?;
    let _profile = repo.get_profile()?.ok_or(crate::error::CourseapeError::ProfileNotSet)?;

    let mut refreshed = Vec::new();

    // Refresh departments
    eprintln!("更新系所清單...");
    let result = crate::connectors::necessary_course::NecessaryCourseConnector::query_departments(114).await?;
    let json: serde_json::Value = serde_json::from_slice(&result.body_bytes)?;
    let departments = crate::parsers::department_json::parse_departments(&json, 114)?;
    let count = departments.len();
    repo.upsert_departments(&departments)?;
    refreshed.push(format!("departments: {}", count));

    // Refresh grades
    eprintln!("更新成績...");
    let grade_result = crate::connectors::itouch::ItouchConnector::fetch_grades(&session.cookie).await?;
    let (hash, _) = storage::snapshot::SnapshotArchive::save("grades", &grade_result.body_bytes)?;
    refreshed.push(format!("grades: hash={}", &hash[..16]));

    // Refresh current term offerings
    let current = resolver::current_term_code();
    eprintln!("更新 {} 開課清單...", current);
    match crate::cli::courses::fetch_offerings_from_api(&current).await {
        Ok(offerings) => {
            let count = offerings.len();
            repo.upsert_offerings(&current, &offerings)?;
            refreshed.push(format!("offerings_{}: {}", current, count));
        }
        Err(e) => {
            refreshed.push(format!("offerings_{}: failed ({})", current, e));
        }
    }

    let result = json!({
        "status": "ok",
        "refreshed": refreshed,
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
