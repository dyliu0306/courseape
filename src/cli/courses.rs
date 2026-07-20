use crate::analysis;
use crate::parsers::time_slot::{expand_time_slots, PERIOD_ORDER};
use crate::storage;
use crate::{Cli, CoursesCommands, OutputFormat};
use chrono::Datelike;

const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

pub async fn run(cmd: &CoursesCommands, cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);

    match cmd {
        CoursesCommands::Offerings { term } => {
            let offerings = repo.list_offerings(term)?;
            if offerings.is_empty() {
                eprintln!(
                    "No cached offerings for term {}. Fetching from iTouch...",
                    term
                );
                let fetched = fetch_offerings_from_api(term).await?;
                let count = fetched.len();
                if count == 0 {
                    anyhow::bail!("No offerings published for term {term}");
                }
                repo.upsert_offerings(term, &fetched)?;
                eprintln!("Synced {} offerings for term {}.", count, term);
                let sections = crate::storage::repo::merge_offering_rows(&fetched);
                let display: Vec<_> = sections.iter().take(20).cloned().collect();
                match cli.output {
                    OutputFormat::Table => {
                        println!("{}", crate::output::formatter::offerings_table(&display))
                    }
                    OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&display)?),
                    OutputFormat::Csv => {
                        print!("{}", crate::output::formatter::offerings_csv(&display)?)
                    }
                }
                if sections.len() > 20 {
                    eprintln!(
                        "... and {} more. Use `courses filter` to narrow down.",
                        sections.len() - 20
                    );
                }
                return Ok(());
            }
            eprintln!("{} offerings cached for term {}.", offerings.len(), term);
            let sections = crate::storage::repo::merge_offering_rows(&offerings);
            let display: Vec<_> = sections.iter().take(20).cloned().collect();
            match cli.output {
                OutputFormat::Table => {
                    println!("{}", crate::output::formatter::offerings_table(&display))
                }
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&display)?),
                OutputFormat::Csv => {
                    print!("{}", crate::output::formatter::offerings_csv(&display)?)
                }
            }
            if sections.len() > 20 {
                eprintln!(
                    "... and {} more. Use `courses filter` to narrow down.",
                    sections.len() - 20
                );
            }
            Ok(())
        }
        CoursesCommands::Filter {
            term,
            code,
            keyword,
            teacher,
            teacher_id,
            dept,
            class_dept,
            r#type,
            credit,
            div,
            language,
            day,
            period,
            classroom,
            general,
            emi,
            english,
            distance,
            pbl,
            programming,
            available,
            semester,
            cross,
            sdgs,
            no_conflict_with,
        } => {
            let offerings = repo.list_offerings(term)?;
            if offerings.is_empty() {
                eprintln!(
                    "No cached offerings for term {}. Run `courses offerings --term {}` first.",
                    term, term
                );
                return Ok(());
            }
            // TTL check: warn if data is older than 4 hours
            let snapshot_prefix = format!("offerings_{}", term);
            if !storage::snapshot::SnapshotArchive::is_fresh(&snapshot_prefix, 4)? {
                eprintln!(
                    "⚠ 開課資料超過 4 小時，可能不是最新。執行 courses offerings --term {} 更新。",
                    term
                );
            }
            let params = analysis::filter::FilterParams {
                code: code.clone(),
                keyword: keyword.clone(),
                teacher: teacher.clone(),
                teacher_id: teacher_id.clone(),
                dept: dept.clone(),
                class_dept: class_dept.clone(),
                course_type: r#type.clone(),
                credit: *credit,
                div: div.clone(),
                language: language.clone(),
                day: *day,
                period: period.clone(),
                classroom: classroom.clone(),
                general: general.clone(),
                emi: *emi,
                english: *english,
                distance: *distance,
                pbl: *pbl,
                programming: *programming,
                available_only: *available,
                semester_half: semester.clone(),
                cross: *cross,
                sdgs: sdgs.clone(),
                no_conflict_with: no_conflict_with.clone(),
            };
            let filtered_rows = analysis::filter::apply_section_filters(&offerings, &params);
            let filtered = crate::storage::repo::merge_offering_rows(&filtered_rows);
            eprintln!(
                "{} section(s) (from {} raw assignment rows).",
                filtered.len(),
                offerings.len()
            );
            match cli.output {
                OutputFormat::Table => {
                    println!("{}", crate::output::formatter::offerings_table(&filtered));
                }
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &crate::output::formatter::offerings_summary_json(&filtered)
                        )?
                    );
                }
                OutputFormat::Csv => {
                    print!("{}", crate::output::formatter::offerings_csv(&filtered)?);
                }
            }
            Ok(())
        }
        CoursesCommands::Conflicts { term } => {
            let planned = repo.get_planned_courses(term)?;

            if planned.is_empty() {
                eprintln!(
                    "No planned courses for term {}. Add courses with `shortlist add` first.",
                    term
                );
                return Ok(());
            }

            let report = analysis::conflict::detect_conflicts(&planned);
            eprintln!("Planned courses: {} shortlisted section(s)", planned.len());

            if report.conflict_count == 0 {
                eprintln!("No conflicts found.");
            } else {
                eprintln!();
                eprintln!(
                    "WARNING: {} conflict(s) found (advisory only, not blocking):",
                    report.conflict_count
                );
                for pair in &report.pairs {
                    let a = planned.iter().find(|o| o.code == pair.course_a);
                    let b = planned.iter().find(|o| o.code == pair.course_b);
                    let a_name = a.map(|o| o.name.as_str()).unwrap_or("?");
                    let b_name = b.map(|o| o.name.as_str()).unwrap_or("?");
                    println!(
                        "  {} ({}) <-> {} ({})  [{}]",
                        pair.course_a,
                        a_name,
                        pair.course_b,
                        b_name,
                        pair.overlapping_slots.join(", ")
                    );
                }
            }
            Ok(())
        }
        CoursesCommands::Syllabus { course_code, term } => {
            let _url = crate::connectors::cmap::CmapConnector::syllabus_url(term, course_code);
            eprintln!(
                "Downloading syllabus for {} (term {})...",
                course_code, term
            );
            let result =
                crate::connectors::cmap::CmapConnector::download_syllabus(term, course_code)
                    .await?;

            if result.status != 200 {
                anyhow::bail!("Syllabus download returned status {}", result.status);
            }

            let (hash, path) = storage::snapshot::SnapshotArchive::save(
                &format!("syllabus_{}_{}", course_code, term),
                &result.body_bytes,
            )?;

            eprintln!("Syllabus PDF saved (hash: {}...).", &hash[..16]);
            eprintln!("Path: {}", path.display());
            Ok(())
        }
        CoursesCommands::Timetable { term } => {
            let planned = repo.get_planned_courses(term)?;

            if planned.is_empty() {
                eprintln!(
                    "No planned courses for term {}. Add courses with `shortlist add` first.",
                    term
                );
                return Ok(());
            }

            eprintln!(
                "Timetable for term {} ({} shortlisted section(s)):",
                term,
                planned.len()
            );
            eprintln!();

            // Parse all time slots into (day_idx, period, course) tuples
            let mut grid: Vec<Vec<Vec<String>>> = vec![vec![vec![]; PERIOD_ORDER.len()]; 7];

            for o in &planned {
                for cell in expand_time_slots(&o.time_slots) {
                    let Some(period_idx) = PERIOD_ORDER
                        .iter()
                        .position(|period| *period == cell.period)
                    else {
                        continue;
                    };
                    let display_name = truncate_unicode(&o.name, 12);
                    grid[cell.day as usize - 1][period_idx].push(display_name);
                }
            }

            // Print table header
            print!("{:<5}", "");
            for day in &DAYS {
                print!(" {:<16}", day);
            }
            println!();

            // Print each period row
            for (pi, period) in PERIOD_ORDER.iter().enumerate() {
                print!("{:<5}", period);
                for day_col in &grid {
                    let cell = day_col[pi].join(", ");
                    let padded = pad_unicode(&cell, 16);
                    print!(
                        " {}",
                        if cell.is_empty() {
                            "·".to_string()
                        } else {
                            padded
                        }
                    );
                }
                println!();
            }
            Ok(())
        }
        CoursesCommands::Plan { term, apply } => {
            let offerings = repo.list_offerings(term)?;

            if offerings.is_empty() {
                eprintln!(
                    "No cached offerings for term {}. Run `courses offerings --term {}` first.",
                    term, term
                );
                return Ok(());
            }

            eprintln!("=== CourseApe Auto-Plan for term {} ===", term);
            eprintln!();

            // Step 1: Read analyzed grades from DB
            let failed_courses = repo.list_failed_grades()?;

            if failed_courses.is_empty() {
                eprintln!("尚無分析過的成績資料。");
                eprintln!("請先執行以下步驟：");
                eprintln!("  1. courseape sync grades");
                eprintln!("  2. 請 Agent 分析成績 HTML 並匯入：");
                eprintln!("     courseape data import --scope grades --file <analysis.json>");
                eprintln!();
            }

            // Step 2: Match failed courses against offerings
            eprintln!("需重修課程（不及格/停修）：");
            let mut retake_matched = 0;
            let shortlist_codes = repo.list_shortlist(term)?;
            for fc in &failed_courses {
                // Match by name: exact match first, then contains
                let matched = (!fc.code.is_empty())
                    .then(|| {
                        offerings
                            .iter()
                            .find(|o| o.course_code == fc.code || o.code == fc.code)
                    })
                    .flatten()
                    .or_else(|| offerings.iter().find(|o| o.name == fc.name))
                    .or_else(|| {
                        offerings.iter().find(|o| {
                            o.name.contains(fc.name.as_str()) || fc.name.contains(o.name.as_str())
                        })
                    });
                if let Some(offering) = matched {
                    retake_matched += 1;
                    let in_shortlist = shortlist_codes.contains(&offering.code);
                    let tag = if in_shortlist { " [已加入]" } else { "" };
                    eprintln!(
                        "  {} ({}) -> {} {} | {}cr | slots: {} | {}/{}{}",
                        fc.name,
                        fc.term,
                        offering.code,
                        offering.name,
                        offering.credits,
                        offering.time_slots.join(", "),
                        offering.remaining.unwrap_or(-1),
                        offering.max_capacity.unwrap_or(-1),
                        tag
                    );

                    if *apply && !in_shortlist {
                        let _ = repo.add_to_shortlist(&offering.code, term)?;
                    }
                } else {
                    eprintln!("  {} ({}) -> 本學期未開課", fc.name, fc.term);
                }
            }
            if failed_courses.is_empty() {
                eprintln!("  (無不及格/停修課程)");
            }
            eprintln!();

            eprintln!("未自動納入全系必修；學生適用必修需由修業辦法分析結果確認。");
            eprintln!();

            // Step 5: Summary
            let shortlist = repo.list_shortlist(term)?;
            let planned = repo.get_planned_courses(term)?;
            let report = crate::analysis::conflict::detect_conflicts(&planned);

            eprintln!("=== 摘要 ===");
            eprintln!(
                "需重修：{} 門（{} 門本學期有開課）",
                failed_courses.len(),
                retake_matched
            );
            eprintln!("備選清單：{} 門", shortlist.len());
            if report.conflict_count > 0 {
                eprintln!("衝堂警告：{} 組（僅提示，不阻擋）", report.conflict_count);
            } else {
                eprintln!("衝堂：無");
            }
            if *apply {
                eprintln!("已套用推薦至備選清單");
            } else {
                eprintln!("僅顯示推薦；加上 --apply 才會修改備選清單");
            }
            Ok(())
        }
        CoursesCommands::History { student_id } => {
            // Determine enrollment year from student ID or profile
            let profile = repo.get_profile()?;
            let sid = student_id
                .clone()
                .or_else(|| profile.as_ref().map(|p| p.student_id.clone()))
                .unwrap_or_default();

            if sid.len() < 3 {
                anyhow::bail!("無法判斷入學年度。請提供 --student-id 或先執行 profile edit。");
            }

            let enroll_year: u32 = sid[..3].parse()?;
            let ad_year = chrono::Utc::now().year();
            let month = chrono::Utc::now().month();
            let (current_year, current_semester) = if month >= 9 {
                ((ad_year - 1911) as u32, 1u32)
            } else if (2..=6).contains(&month) {
                ((ad_year - 1912) as u32, 2u32)
            } else if month == 1 {
                ((ad_year - 1912) as u32, 1u32)
            } else {
                // Jul-Aug: between semesters, upcoming term 1 is available
                ((ad_year - 1911) as u32, 1u32)
            };

            eprintln!("入學年度：{}，同步歷史開課清單...", enroll_year);

            let mut total_synced = 0;
            let mut terms_synced = 0;

            for year in enroll_year..=current_year {
                for sem in 1..=2 {
                    // Skip future terms
                    if year == current_year && sem > current_semester {
                        continue;
                    }
                    let term = format!("{}{}", year, sem);

                    // Skip if already synced
                    let existing = repo.list_offerings(&term).unwrap_or_default();
                    if !existing.is_empty() {
                        eprintln!("  {} (已同步，{} 筆)", term, existing.len());
                        continue;
                    }

                    // Fetch from API
                    match fetch_offerings_from_api(&term).await {
                        Ok(offerings) => {
                            let count = offerings.len();
                            if count == 0 {
                                eprintln!("  {} (尚未發布)", term);
                                continue;
                            }
                            repo.upsert_offerings(&term, &offerings)?;
                            eprintln!("  {} ({} 筆)", term, count);
                            total_synced += count;
                            terms_synced += 1;
                        }
                        Err(e) => {
                            eprintln!("  {} (失敗: {})", term, e);
                        }
                    }
                }
            }

            eprintln!();
            eprintln!(
                "完成！同步 {} 個學期，共 {} 筆開課資料。",
                terms_synced, total_synced
            );
            eprintln!("這些資料將用於判斷已修課程的通識向度。");
            Ok(())
        }
    }
}

/// Display width of a string (CJK chars = 2, ASCII = 1).
fn unicode_display_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}

/// Truncate string to max display width, appending ".." if truncated.
fn truncate_unicode(s: &str, max_width: usize) -> String {
    let mut width = 0;
    let mut result = String::new();
    for c in s.chars() {
        let cw = if c.is_ascii() { 1 } else { 2 };
        if width + cw > max_width.saturating_sub(2) {
            result.push_str("..");
            return result;
        }
        result.push(c);
        width += cw;
    }
    result
}

/// Right-pad string to target display width with spaces.
fn pad_unicode(s: &str, target_width: usize) -> String {
    let w = unicode_display_width(s);
    if w >= target_width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(target_width - w))
    }
}

/// Fetch course offerings from iTouch courseQuery API.
pub async fn fetch_offerings_from_api(
    term: &str,
) -> anyhow::Result<Vec<crate::domain::course_offering::CourseOffering>> {
    let session =
        crate::auth::session::Session::load()?.ok_or(crate::error::CourseapeError::NotLoggedIn)?;

    eprintln!("Querying iTouch courseQuery API for term {}...", term);
    let json =
        crate::connectors::elective::CourseQueryConnector::query_offerings(&session.cookie, term)
            .await?;

    // Validate every row before marking the response as a fresh snapshot.
    let offerings = crate::connectors::elective::parse_offerings(&json)?;
    let body = serde_json::to_string(&json)?;
    let (hash, _) =
        storage::snapshot::SnapshotArchive::save(&format!("offerings_{}", term), body.as_bytes())?;
    eprintln!("CourseQuery response saved (hash: {}...).", &hash[..16]);

    Ok(offerings)
}
