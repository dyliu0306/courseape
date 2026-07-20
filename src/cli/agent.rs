use crate::domain::resolver;
use crate::storage;
use crate::{AgentCommands, Cli, PrepareCommands};
use serde_json::json;

fn profile_is_complete(profile: &crate::domain::profile::StudentProfile) -> bool {
    profile.dept_code.is_some()
        && profile.dept_name.is_some()
        && profile.enroll_year.is_some()
        && profile.degree.is_some()
}

fn valid_pdf(path: &str) -> bool {
    std::fs::read(path)
        .map(|bytes| bytes.starts_with(b"%PDF-"))
        .unwrap_or(false)
}

fn has_valid_grade_snapshot() -> bool {
    storage::snapshot::SnapshotArchive::newest_valid_grade(None)
        .ok()
        .flatten()
        .is_some()
}

pub async fn run(cmd: &AgentCommands, cli: &Cli) -> anyhow::Result<()> {
    let result = match cmd {
        AgentCommands::Doctor => run_doctor(cli).await,
        AgentCommands::Setup { department } => run_setup(cli, department.as_deref()).await,
        AgentCommands::Prepare(sub) => run_prepare(sub, cli).await,
        AgentCommands::Resolve { query } => run_resolve(query, cli),
        AgentCommands::Context { task } => run_context(task, cli).await,
        AgentCommands::Refresh { stale_only } => run_refresh(cli, *stale_only).await,
    };
    if let Err(error) = &result {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": false,
                "error": {
                    "code": error_code(error),
                    "message": error.to_string(),
                    "recoverable": true,
                }
            }))?
        );
    }
    result
}

fn error_code(error: &anyhow::Error) -> &'static str {
    let message = error.to_string();
    if message.contains("logged in") || message.contains("Session expired") {
        "NOT_AUTHENTICATED"
    } else if message.contains("Profile") || message.contains("department") {
        "PROFILE_INCOMPLETE"
    } else if message.contains("offerings") {
        "OFFERINGS_UNAVAILABLE"
    } else {
        "COURSEAPE_ERROR"
    }
}

/// doctor: report login, profile, cache, PDF skill, data freshness as JSON.
async fn run_doctor(_cli: &Cli) -> anyhow::Result<()> {
    let session_result = crate::auth::session::Session::load();
    let session = session_result
        .as_ref()
        .ok()
        .and_then(|value| value.as_ref());
    let session_status = match session {
        None if session_result.is_err() => "unreadable",
        None => "absent",
        Some(session)
            if crate::connectors::itouch::ItouchConnector::validate_session(&session.cookie)
                .await? =>
        {
            "valid"
        }
        Some(_) => "expired",
    };
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let profile = repo.get_profile().ok().flatten();
    let terms = repo.list_offering_terms().unwrap_or_default();

    let has_requirements = if let (Some(dept), Some(year)) = (
        profile.as_ref().and_then(|p| p.dept_code.as_deref()),
        profile.as_ref().and_then(|p| p.enroll_year),
    ) {
        repo.get_requirement_path(year, dept)
            .ok()
            .flatten()
            .is_some_and(|path| valid_pdf(&path))
    } else {
        false
    };

    let has_grades = has_valid_grade_snapshot();

    let has_analyzed_grades = repo
        .list_analyzed_grades()
        .ok()
        .map(|g| !g.is_empty())
        .unwrap_or(false);

    let (current_year, current_sem) = resolver::current_term();
    let current_term_code = format!("{}{}", current_year, current_sem);

    // Profile completeness: exists AND has all required fields
    let profile_complete = profile.as_ref().is_some_and(profile_is_complete);

    let missing_fields: Vec<&str> = profile
        .as_ref()
        .map(|p| {
            let mut missing = Vec::new();
            if p.dept_code.is_none() {
                missing.push("department");
            }
            if p.enroll_year.is_none() {
                missing.push("enroll_year");
            }
            if p.degree.is_none() {
                missing.push("degree");
            }
            missing
        })
        .unwrap_or_else(|| vec!["profile"]);

    let status = json!({
        "logged_in": session_status == "valid",
        "session_present": session.is_some(),
        "session_status": session_status,
        "profile_exists": profile.is_some(),
        "profile_complete": profile_complete,
        "missing_fields": missing_fields,
        "profile": profile.as_ref().map(|p| json!({
            "student_id": if _cli.no_redact_personal { &p.student_id } else { "***" },
            "dept_code": p.dept_code,
            "dept_name": p.dept_name,
            "enroll_year": p.enroll_year,
            "degree": p.degree,
        })),
        "departments_synced": !repo.list_departments(
            profile.as_ref().and_then(|p| p.enroll_year).unwrap_or(115)
        ).unwrap_or_default().is_empty(),
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
async fn run_setup(cli: &Cli, department_query: Option<&str>) -> anyhow::Result<()> {
    let mut steps_completed = Vec::new();

    // Step 1: Check login
    eprintln!("[1/4] 檢查登入狀態...");
    let _session = match crate::auth::session::Session::load()? {
        Some(s)
            if crate::connectors::itouch::ItouchConnector::validate_session(&s.cookie).await? =>
        {
            eprintln!("  ✓ 已登入");
            steps_completed.push("login_ok");
            s
        }
        _ => {
            eprintln!("  尚未登入，嘗試自動登入...");
            // Try env vars or stored credentials
            match crate::auth::keyring::StoredCredentials::load() {
                Ok(Some(creds)) => {
                    let (cookie, login_token) = crate::connectors::itouch::ItouchConnector::login(
                        &creds.student_id,
                        &creds.password,
                    )
                    .await?;
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
                _ => anyhow::bail!("無法自動登入。請先執行 courseape login"),
            }
        }
    };

    // Step 2: Get student ID
    eprintln!("[2/4] 讀取學號...");
    let student_id = {
        let db = storage::db::open()?;
        let repo = storage::repo::Repository::new(&db);
        repo.get_profile()?
            .map(|p| p.student_id)
            .unwrap_or_default()
    };
    let student_id = if student_id.is_empty() {
        std::env::var("CYCU_USERNAME")
            .ok()
            .or_else(|| {
                crate::auth::keyring::StoredCredentials::load()
                    .ok()
                    .flatten()
                    .map(|c| c.student_id.clone())
            })
            .unwrap_or_default()
    } else {
        student_id
    };

    if student_id.is_empty() {
        anyhow::bail!("無法取得學號。請執行 courseape login");
    }
    let displayed_student_id = if cli.no_redact_personal {
        student_id.clone()
    } else {
        crate::redact::profile::mask_student_id(&student_id)
    };
    eprintln!("  ✓ 學號: {}", displayed_student_id);
    steps_completed.push("student_id_ok");

    // Step 3: Auto-detect profile from student ID
    eprintln!("[3/4] 自動推導個人資料...");
    let enroll_year = resolver::derive_enroll_year(&student_id);
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let current_profile = repo.get_profile()?;

    if current_profile.is_none()
        || current_profile.as_ref().is_some_and(|p| {
            p.enroll_year.is_none() || p.degree.is_none() || p.student_id != student_id
        })
    {
        let profile = crate::domain::profile::StudentProfile {
            student_id: student_id.clone(),
            dept_code: current_profile.as_ref().and_then(|p| p.dept_code.clone()),
            dept_name: current_profile.as_ref().and_then(|p| p.dept_name.clone()),
            enroll_year: enroll_year
                .or_else(|| current_profile.as_ref().and_then(|p| p.enroll_year)),
            degree: current_profile
                .as_ref()
                .and_then(|p| p.degree.clone())
                .or_else(|| Some("學士".to_string())),
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
    let dept_year = enroll_year.unwrap_or(115);
    let depts = repo.list_departments(dept_year)?;
    if depts.is_empty() {
        eprintln!("  正在下載...");
        let result =
            crate::connectors::necessary_course::NecessaryCourseConnector::query_departments(
                dept_year,
            )
            .await?;
        let json: serde_json::Value = serde_json::from_slice(&result.body_bytes)?;
        let departments = crate::parsers::department_json::parse_departments(&json, dept_year)?;
        let count = departments.len();
        repo.upsert_departments(&departments)?;
        let _ = storage::snapshot::SnapshotArchive::save(
            &format!("departments_{dept_year}"),
            &result.body_bytes,
        );
        eprintln!("  ✓ 同步 {} 個系所", count);
    } else {
        eprintln!("  ✓ 已有 {} 個系所", depts.len());
    }
    steps_completed.push("departments_ok");

    if let Some(query) = department_query {
        let departments = repo.list_departments(dept_year)?;
        let candidates = resolver::resolve_department(query, &departments);
        if candidates.len() != 1 {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "status": "needs_input",
                    "field": "department",
                    "candidates": candidates,
                }))?
            );
            return Ok(());
        }
        let candidate = &candidates[0];
        let mut profile = repo
            .get_profile()?
            .ok_or(crate::error::CourseapeError::ProfileNotSet)?;
        profile.dept_code = Some(candidate.dept_code.clone());
        profile.dept_name = Some(candidate.name.clone());
        repo.upsert_profile(&profile)?;
        steps_completed.push("department_ok");
    } else {
        anyhow::bail!(
            "系所為必填欄位。請執行：\n\
             courseape agent setup --department \"你的系所名稱\"\n\
             例如：courseape agent setup --department \"資管系\""
        );
    }

    // Output JSON summary
    let profile_complete = repo
        .get_profile()?
        .as_ref()
        .is_some_and(profile_is_complete);
    let summary = json!({
        "status": if profile_complete { "ok" } else { "needs_input" },
        "steps": steps_completed,
        "student_id": if cli.no_redact_personal { &student_id } else { "***" },
        "enroll_year": enroll_year,
        "profile_needs_dept": repo.get_profile()?.is_none_or(|p| p.dept_code.is_none()),
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
    let profile = repo
        .get_profile()?
        .ok_or(crate::error::CourseapeError::ProfileNotSet)?;
    let dept_code = profile
        .dept_code
        .as_deref()
        .ok_or(crate::error::CourseapeError::ProfileNotSet)?;
    let enroll_year = profile
        .enroll_year
        .ok_or(crate::error::CourseapeError::ProfileNotSet)?;

    let mut result = json!({});

    // 1. Requirements PDF
    eprintln!("[1/3] 下載修業辦法...");
    let existing_requirement = repo
        .get_requirement_path(enroll_year, dept_code)?
        .filter(|path| valid_pdf(path));
    let req_path = if let Some(path) = existing_requirement {
        eprintln!("  ✓ 已有修業辦法 PDF");
        std::path::PathBuf::from(path)
    } else {
        let res = crate::connectors::necessary_course::NecessaryCourseConnector::download_requirement_pdf(
            enroll_year, dept_code
        ).await?;
        if res.status != 200 || !res.body_bytes.starts_with(b"%PDF-") {
            anyhow::bail!("Requirement download did not return a valid PDF");
        }
        let (hash, path) = storage::snapshot::SnapshotArchive::save_as(
            &format!("requirements_{}_{}", enroll_year, dept_code),
            "pdf",
            &res.body_bytes,
        )?;
        repo.upsert_requirement(enroll_year, dept_code, &path.to_string_lossy(), "1.0.0")?;
        eprintln!("  ✓ 已下載 (hash: {}...)", &hash[..16]);
        path
    };
    result["requirements_path"] = json!(req_path.to_string_lossy());

    // 2. Grades HTML
    eprintln!("[2/3] 下載歷年成績...");
    let session =
        crate::auth::session::Session::load()?.ok_or(crate::error::CourseapeError::NotLoggedIn)?;
    let has_grades = has_valid_grade_snapshot();

    if has_grades {
        eprintln!("  ✓ 已有成績資料");
    } else {
        let grade_result =
            crate::connectors::itouch::ItouchConnector::fetch_grades(&session.cookie).await?;
        if grade_result.status != 200
            || !crate::connectors::itouch::is_authenticated_grade_body(&grade_result.body_bytes)
        {
            anyhow::bail!("Grade fetch returned an unauthenticated login page");
        }
        let (hash, path) =
            storage::snapshot::SnapshotArchive::save("grades", &grade_result.body_bytes)?;
        eprintln!("  ✓ 已下載 (hash: {}...)", &hash[..16]);
        result["grade_html_path"] = json!(path.to_string_lossy());
    }

    // 3. Historical offerings
    eprintln!("[3/3] 同步歷史開課資料...");
    let (current_year, current_sem) = resolver::current_term();
    let mut total = 0;
    let mut failed_terms = Vec::new();
    for year in enroll_year..=current_year {
        for sem in 1..=2 {
            if year == current_year && sem > current_sem {
                continue;
            }
            let term = format!("{}{}", year, sem);
            let existing = repo.list_offerings(&term).unwrap_or_default();
            if !existing.is_empty() {
                total += existing.len();
                continue;
            }
            match crate::cli::courses::fetch_offerings_from_api(&term).await {
                Ok(offerings) => {
                    let count = offerings.len();
                    if count > 0 {
                        repo.upsert_offerings(&term, &offerings)?;
                        total += count;
                    }
                }
                Err(error) => {
                    eprintln!("  {} (失敗: {})", term, error);
                    failed_terms.push(json!({"term": term, "error": error.to_string()}));
                }
            }
        }
    }
    eprintln!("  ✓ 共 {} 筆開課資料", total);

    result["status"] = json!(if failed_terms.is_empty() {
        "ok"
    } else {
        "partial"
    });
    result["failed_terms"] = json!(failed_terms);
    result["total_offerings"] = json!(total);
    result["has_analyzed_grades"] = json!(repo
        .list_analyzed_grades()
        .ok()
        .map(|g| !g.is_empty())
        .unwrap_or(false));

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
    let offerings_count = if offerings.is_empty() {
        eprintln!("  正在同步 {} 開課清單...", term);
        let fetched = crate::cli::courses::fetch_offerings_from_api(&term).await?;
        let count = fetched.len();
        if count > 0 {
            repo.upsert_offerings(&term, &fetched)?;
        }
        eprintln!("  ✓ 同步 {} 筆", count);
        count
    } else {
        eprintln!("  ✓ 已有 {} 筆開課資料", offerings.len());
        offerings.len()
    };

    result["offerings_count"] = json!(offerings_count);

    // If no offerings available, return unavailable status
    if offerings_count == 0 {
        result["status"] = json!("unavailable");
        result["reason"] = json!("offerings_not_published");
        result["fallback_term"] = json!(resolver::current_term_code());
        eprintln!("  ⚠ {} 尚無開課資料", term);
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // 2. Show shortlist status
    eprintln!("[2/2] 檢查備選清單...");
    let shortlist = repo.list_shortlist(&term)?;
    let planned = repo.get_planned_courses(&term)?;
    let report = crate::analysis::conflict::detect_conflicts(&planned);

    result["shortlist_count"] = json!(shortlist.len());
    result["planned_count"] = json!(planned.len());
    result["conflicts"] = json!(report.conflict_count);
    result["status"] = json!("ok");

    eprintln!(
        "  ✓ 備選 {} 門，衝堂 {} 組",
        shortlist.len(),
        report.conflict_count
    );

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// resolve: resolve a natural-language department name to code.
fn run_resolve(query: &str, _cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let dept_year = repo
        .get_profile()
        .ok()
        .flatten()
        .and_then(|p| p.enroll_year)
        .unwrap_or(115);
    let departments = repo.list_departments(dept_year)?;

    if departments.is_empty() {
        anyhow::bail!("系所清單尚未同步。請先執行 courseape sync departments --year 114");
    }

    let candidates = resolver::resolve_department(query, &departments);

    let result = json!({
        "query": query,
        "candidates": candidates,
        "auto_select": candidates.len() == 1
            || candidates.first().map(|c| {
                matches!(c.confidence, resolver::MatchConfidence::Exact | resolver::MatchConfidence::High)
            }).unwrap_or(false),
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// context: return agent-needed data state and next steps.
async fn run_context(task: &str, _cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let session = crate::auth::session::Session::load().ok().flatten();
    let logged_in = if let Some(session) = &session {
        crate::connectors::itouch::ItouchConnector::validate_session(&session.cookie).await?
    } else {
        false
    };
    let profile = repo.get_profile().ok().flatten();
    let terms = repo.list_offering_terms().unwrap_or_default();
    let has_analyzed = repo
        .list_analyzed_grades()
        .ok()
        .map(|g| !g.is_empty())
        .unwrap_or(false);

    let current_term = resolver::current_term_code();
    let next = resolver::next_term();

    let profile_complete = profile.as_ref().is_some_and(profile_is_complete);

    let missing_fields: Vec<&str> = profile
        .as_ref()
        .map(|p| {
            let mut missing = Vec::new();
            if p.dept_code.is_none() {
                missing.push("department");
            }
            if p.enroll_year.is_none() {
                missing.push("enroll_year");
            }
            if p.degree.is_none() {
                missing.push("degree");
            }
            missing
        })
        .unwrap_or_else(|| vec!["profile"]);

    let mut result = json!({
        "task": task,
        "logged_in": logged_in,
        "profile_exists": profile.is_some(),
        "profile_complete": profile_complete,
        "missing_fields": missing_fields,
        "current_term": current_term,
        "next_term": next,
    });

    let mut actions: Vec<serde_json::Value> = Vec::new();

    if !logged_in {
        actions.push(json!({"type": "login", "message": "請執行 courseape login"}));
    }

    if !profile_complete {
        actions.push(json!({"type": "run", "command": ["agent", "setup"]}));
    }

    match task {
        "graduation" => {
            let has_req = profile
                .as_ref()
                .and_then(|p| {
                    let dept = p.dept_code.as_deref()?;
                    let year = p.enroll_year?;
                    Some(
                        repo.get_requirement_path(year, dept)
                            .ok()
                            .flatten()
                            .is_some_and(|path| valid_pdf(&path)),
                    )
                })
                .unwrap_or(false);

            let has_grades = has_valid_grade_snapshot();

            result["has_requirements"] = json!(has_req);
            result["has_grade_html"] = json!(has_grades);
            result["has_analyzed_grades"] = json!(has_analyzed);

            if !has_req || !has_grades {
                actions.push(json!({"type": "run", "command": ["agent", "prepare", "graduation"]}));
            }
            if !has_analyzed {
                actions.push(
                    json!({"type": "agent_analyze_grades", "message": "分析成績 HTML 並匯入"}),
                );
            }
        }
        "planning" => {
            let has_offerings = terms.contains(&next);
            result["has_next_term_offerings"] = json!(has_offerings);

            if !has_offerings {
                actions.push(json!({"type": "run", "command": ["agent", "prepare", "planning", "--term", &next]}));
            }
            if !has_analyzed {
                actions.push(json!({"type": "dependency", "message": "先完成畢業分析", "command": ["agent", "context", "--task", "graduation"]}));
            }
        }
        _ => {}
    }

    result["actions"] = json!(actions);
    result["cached_terms"] = json!(terms);

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// refresh: re-download stale or missing data.
async fn run_refresh(_cli: &Cli, stale_only: bool) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);
    let session =
        crate::auth::session::Session::load()?.ok_or(crate::error::CourseapeError::NotLoggedIn)?;
    let _profile = repo
        .get_profile()?
        .ok_or(crate::error::CourseapeError::ProfileNotSet)?;
    let dept_year = _profile.enroll_year.unwrap_or(115);

    let mut refreshed = Vec::new();
    let mut skipped = Vec::new();

    // Refresh departments
    let dept_snapshot = format!("departments_{dept_year}");
    if stale_only && crate::storage::snapshot::SnapshotArchive::is_fresh(&dept_snapshot, 24)? {
        skipped.push(serde_json::json!({"resource":"departments","reason":"fresh"}));
    } else {
        eprintln!("更新系所清單...");
        let result =
            crate::connectors::necessary_course::NecessaryCourseConnector::query_departments(
                dept_year,
            )
            .await?;
        let json: serde_json::Value = serde_json::from_slice(&result.body_bytes)?;
        let departments = crate::parsers::department_json::parse_departments(&json, dept_year)?;
        let count = departments.len();
        repo.upsert_departments(&departments)?;
        let _ = storage::snapshot::SnapshotArchive::save(&dept_snapshot, &result.body_bytes)?;
        refreshed.push(format!("departments: {}", count));
    }

    // Refresh grades
    if stale_only
        && crate::storage::snapshot::SnapshotArchive::newest_valid_grade(Some(6))?.is_some()
    {
        skipped.push(serde_json::json!({"resource":"grades","reason":"fresh"}));
    } else {
        eprintln!("更新成績...");
        let grade_result =
            crate::connectors::itouch::ItouchConnector::fetch_grades(&session.cookie).await?;
        if grade_result.status != 200
            || !crate::connectors::itouch::is_authenticated_grade_body(&grade_result.body_bytes)
        {
            anyhow::bail!("Grade refresh returned an unauthenticated login page");
        }
        let (hash, _) =
            storage::snapshot::SnapshotArchive::save("grades", &grade_result.body_bytes)?;
        refreshed.push(format!("grades: hash={}", &hash[..16]));
    }

    // Refresh current term offerings
    let current = resolver::current_term_code();
    if stale_only
        && crate::storage::snapshot::SnapshotArchive::is_fresh(&format!("offerings_{current}"), 6)?
    {
        skipped
            .push(serde_json::json!({"resource":format!("offerings_{current}"),"reason":"fresh"}));
    } else {
        eprintln!("更新 {} 開課清單...", current);
        match crate::cli::courses::fetch_offerings_from_api(&current).await {
            Ok(offerings) => {
                let count = offerings.len();
                if count > 0 {
                    repo.upsert_offerings(&current, &offerings)?;
                    refreshed.push(format!("offerings_{}: {}", current, count));
                } else {
                    refreshed.push(format!("offerings_{}: unavailable", current));
                }
            }
            Err(e) => {
                refreshed.push(format!("offerings_{}: failed ({})", current, e));
            }
        }
    }

    let result = json!({
        "status": "ok",
        "refreshed": refreshed,
        "skipped": skipped,
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
