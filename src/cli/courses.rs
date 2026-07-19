use crate::analysis;
use crate::storage;
use crate::{Cli, CoursesCommands, OutputFormat};
use chrono::Datelike;

const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const PERIODS: [&str; 15] = ["1", "2", "3", "4", "A", "B", "5", "6", "7", "8", "C", "D", "E", "F", "G"];

pub async fn run(cmd: &CoursesCommands, cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);

    match cmd {
        CoursesCommands::Offerings { term } => {
            let offerings = repo.list_offerings(term)?;
            if offerings.is_empty() {
                eprintln!("No cached offerings for term {}. Fetching from iTouch...", term);
                let fetched = fetch_offerings_from_api(term).await?;
                let count = fetched.len();
                repo.upsert_offerings(term, &fetched)?;
                eprintln!("Synced {} offerings for term {}.", count, term);
                let display: Vec<_> = fetched.iter().take(20).cloned().collect();
                match cli.output {
                    OutputFormat::Table => println!("{}", crate::output::formatter::offerings_table(&display)),
                    _ => println!("{}", serde_json::to_string_pretty(&display)?),
                }
                if fetched.len() > 20 {
                    eprintln!("... and {} more. Use `courses filter` to narrow down.", fetched.len() - 20);
                }
                return Ok(());
            }
            eprintln!("{} offerings cached for term {}.", offerings.len(), term);
            let display: Vec<_> = offerings.iter().take(20).cloned().collect();
            match cli.output {
                OutputFormat::Table => println!("{}", crate::output::formatter::offerings_table(&display)),
                _ => println!("{}", serde_json::to_string_pretty(&display)?),
            }
            if offerings.len() > 20 {
                eprintln!("... and {} more. Use `courses filter` to narrow down.", offerings.len() - 20);
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
        } => {
            let offerings = repo.list_offerings(term)?;
            if offerings.is_empty() {
                eprintln!("No cached offerings for term {}. Run `courses offerings --term {}` first.", term, term);
                return Ok(());
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
            };
            let filtered = analysis::filter::apply_filters(&offerings, &params);
            eprintln!("{} results (from {} total).", filtered.len(), offerings.len());
            match cli.output {
                OutputFormat::Table => {
                    println!("{}", crate::output::formatter::offerings_table(&filtered));
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&filtered)?);
                }
                OutputFormat::Csv => {
                    // CSV fallback to JSON for now
                    println!("{}", serde_json::to_string_pretty(&filtered)?);
                }
            }
            Ok(())
        }
        CoursesCommands::Conflicts { term } => {
            let profile = repo.get_profile()?;
            let dept_code = profile.as_ref().and_then(|p| p.dept_code.as_deref());
            let planned = repo.get_planned_courses(term, dept_code)?;

            if planned.is_empty() {
                eprintln!("No planned courses for term {}. Add courses with `shortlist add` first.", term);
                return Ok(());
            }

            let report = analysis::conflict::detect_conflicts(&planned);
            let required_count = planned.iter().filter(|o| o.category == "必修").count();
            let shortlist_count = planned.len() - required_count;

            eprintln!("Planned courses: {} ({} required + {} shortlisted)",
                planned.len(), required_count, shortlist_count);

            if report.conflict_count == 0 {
                eprintln!("No conflicts found.");
            } else {
                eprintln!();
                eprintln!("WARNING: {} conflict(s) found (advisory only, not blocking):", report.conflict_count);
                for pair in &report.pairs {
                    let a = planned.iter().find(|o| o.code == pair.course_a);
                    let b = planned.iter().find(|o| o.code == pair.course_b);
                    let a_name = a.map(|o| o.name.as_str()).unwrap_or("?");
                    let b_name = b.map(|o| o.name.as_str()).unwrap_or("?");
                    println!("  {} ({}) <-> {} ({})  [{}]",
                        pair.course_a, a_name, pair.course_b, b_name,
                        pair.overlapping_slots.join(", "));
                }
            }
            Ok(())
        }
        CoursesCommands::Syllabus {
            course_code,
            term,
        } => {
            let _url = crate::connectors::cmap::CmapConnector::syllabus_url(term, course_code);
            eprintln!("Downloading syllabus for {} (term {})...", course_code, term);
            let result = crate::connectors::cmap::CmapConnector::download_syllabus(term, course_code).await?;

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
            let profile = repo.get_profile()?;
            let dept_code = profile.as_ref().and_then(|p| p.dept_code.as_deref());
            let planned = repo.get_planned_courses(term, dept_code)?;

            if planned.is_empty() {
                eprintln!("No planned courses for term {}. Add courses with `shortlist add` first.", term);
                return Ok(());
            }

            let required_count = planned.iter().filter(|o| o.category == "必修").count();
            let shortlist_count = planned.len() - required_count;

            eprintln!("Timetable for term {} ({} required + {} shortlisted):", term, required_count, shortlist_count);
            eprintln!();

            // Parse all time slots into (day_idx, period, course) tuples
            let mut grid: Vec<Vec<Vec<String>>> = vec![vec![vec![]; PERIODS.len()]; 7];

            for o in &planned {
                for slot in &o.time_slots {
                    for (day_idx, period_idx) in parse_slot_to_grid(slot) {
                        // Truncate name to fit column (Unicode-aware: CJK chars are double-width)
                        let display_name = truncate_unicode(&o.name, 12);
                        grid[day_idx][period_idx].push(display_name);
                    }
                }
            }

            // Print table header
            print!("{:<5}", "");
            for day in &DAYS {
                print!(" {:<16}", day);
            }
            println!();

            // Print each period row
            for (pi, period) in PERIODS.iter().enumerate() {
                print!("{:<5}", period);
                for day_col in &grid {
                    let cell = day_col[pi].join(", ");
                    let padded = pad_unicode(&cell, 16);
                    print!(" {}", if cell.is_empty() { "·".to_string() } else { padded });
                }
                println!();
            }
            Ok(())
        }
        CoursesCommands::Plan { term, dry_run } => {
            let offerings = repo.list_offerings(term)?;

            if offerings.is_empty() {
                eprintln!("No cached offerings for term {}. Run `courses offerings --term {}` first.", term, term);
                return Ok(());
            }

            let profile = repo.get_profile()?;
            let dept_code = profile.as_ref().and_then(|p| p.dept_code.as_deref());

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
                let matched = offerings.iter().find(|o| o.name == fc.name)
                    .or_else(|| offerings.iter().find(|o| {
                        o.name.contains(fc.name.as_str()) || fc.name.contains(o.name.as_str())
                    }));
                if let Some(offering) = matched {
                    retake_matched += 1;
                    let in_shortlist = shortlist_codes.contains(&offering.code);
                    let tag = if in_shortlist { " [已加入]" } else { "" };
                    eprintln!("  {} ({}) -> {} {} | {}cr | slots: {} | {}/{}{}",
                        fc.name, fc.term, offering.code, offering.name, offering.credits,
                        offering.time_slots.join(", "),
                        offering.remaining.unwrap_or(-1), offering.max_capacity.unwrap_or(-1),
                        tag);

                    if !*dry_run && !in_shortlist {
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

            // Step 4: Show required courses for dept
            if let Some(dept) = dept_code {
                let required: Vec<_> = offerings.iter()
                    .filter(|o| o.dept_code == dept && o.category == "必修")
                    .collect();
                let shortlist_codes = repo.list_shortlist(term)?;

                eprintln!("該系必修課程（本學期開設）：");
                for o in &required {
                    let in_shortlist = shortlist_codes.contains(&o.code);
                    let tag = if in_shortlist { " [備選]" } else { "" };
                    eprintln!("  {} | {} | {}cr | slots: {} | {}/{}{}",
                        o.code, o.name, o.credits, o.time_slots.join(", "),
                        o.remaining.unwrap_or(-1), o.max_capacity.unwrap_or(-1), tag);
                }
                eprintln!();
            }

            // Step 5: Summary
            let shortlist = repo.list_shortlist(term)?;
            let planned = repo.get_planned_courses(term, dept_code)?;
            let report = crate::analysis::conflict::detect_conflicts(&planned);

            eprintln!("=== 摘要 ===");
            eprintln!("需重修：{} 門（{} 門本學期有開課）", failed_courses.len(), retake_matched);
            eprintln!("備選清單：{} 門", shortlist.len());
            if report.conflict_count > 0 {
                eprintln!("衝堂警告：{} 組（僅提示，不阻擋）", report.conflict_count);
            } else {
                eprintln!("衝堂：無");
            }
            if *dry_run {
                eprintln!("[dry-run] 未修改備選清單");
            }
            Ok(())
        }
        CoursesCommands::History { student_id } => {
            // Determine enrollment year from student ID or profile
            let profile = repo.get_profile()?;
            let sid = student_id.clone()
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
            eprintln!("完成！同步 {} 個學期，共 {} 筆開課資料。", terms_synced, total_synced);
            eprintln!("這些資料將用於判斷已修課程的通識向度。");
            Ok(())
        }
    }
}

/// Parse a CYCU time slot code like "2-A", "4-123", "5-567" into grid indices.
/// Returns multiple (day_index 0-6, period_index) for multi-period slots like "4-123".
fn parse_slot_to_grid(slot: &str) -> Vec<(usize, usize)> {
    let parts: Vec<&str> = slot.splitn(2, '-').collect();
    if parts.len() != 2 { return vec![]; }
    let day: usize = match parts[0].parse() {
        Ok(d) if (1..=7).contains(&d) => d,
        _ => return vec![],
    };
    let day_idx = day - 1;

    let period_part = parts[1];
    let mut result = Vec::new();
    for ch in period_part.chars() {
        let period_idx = match ch {
            '1' => Some(0),
            '2' => Some(1),
            '3' => Some(2),
            '4' => Some(3),
            'A' => Some(4),
            'B' => Some(5),
            '5' => Some(6),
            '6' => Some(7),
            '7' => Some(8),
            '8' => Some(9),
            'C' => Some(10),
            'D' => Some(11),
            'E' => Some(12),
            'F' => Some(13),
            'G' => Some(14),
            _ => None,
        };
        if let Some(pi) = period_idx {
            result.push((day_idx, pi));
        }
    }
    result
}

/// Display width of a string (CJK chars = 2, ASCII = 1).
fn unicode_display_width(s: &str) -> usize {
    s.chars().map(|c| {
        if c.is_ascii() { 1 } else { 2 }
    }).sum()
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
pub async fn fetch_offerings_from_api(term: &str) -> anyhow::Result<Vec<crate::domain::course_offering::CourseOffering>> {
    let session = crate::auth::session::Session::load()?.ok_or(
        crate::error::CourseapeError::NotLoggedIn
    )?;

    eprintln!("Querying iTouch courseQuery API for term {}...", term);
    let json = crate::connectors::elective::CourseQueryConnector::query_offerings(&session.cookie, term).await?;

    // Save snapshot
    let body = serde_json::to_string(&json)?;
    let (hash, _) = storage::snapshot::SnapshotArchive::save(
        &format!("offerings_{}", term),
        body.as_bytes(),
    )?;
    eprintln!("CourseQuery response saved (hash: {}...).", &hash[..16]);

    // Parse offerings
    let offerings = crate::connectors::elective::parse_offerings(&json)?;
    Ok(offerings)
}
