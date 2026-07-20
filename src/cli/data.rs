use crate::storage;
use crate::{Cli, DataCommands};
use anyhow::Context;
use std::io::Read;

pub async fn run(cmd: &DataCommands, cli: &Cli) -> anyhow::Result<()> {
    match cmd {
        DataCommands::Export { scope } => {
            let db = storage::db::open()?;
            let repo = storage::repo::Repository::new(&db);

            match scope.as_str() {
                "profile" => {
                    let profile = repo.get_profile()?;
                    let export =
                        profile.map(|profile| export_profile(&profile, cli.no_redact_personal));
                    println!("{}", serde_json::to_string_pretty(&export)?);
                }
                "departments" => {
                    let profile = repo.get_profile()?;
                    let year = profile.as_ref().and_then(|p| p.enroll_year).unwrap_or(115);
                    let depts = repo.list_departments(year)?;
                    println!("{}", serde_json::to_string_pretty(&depts)?);
                }
                "grades" => {
                    // Export analyzed grades from DB
                    let grades = repo.list_analyzed_grades()?;
                    if grades.is_empty() {
                        eprintln!("No analyzed grades found.");
                        eprintln!("Steps to import grades:");
                        eprintln!("  1. courseape sync grades          # 下載成績 HTML");
                        eprintln!("  2. 在 AI Agent 中說「分析我的成績」  # Agent 解析 HTML 後匯入");
                        eprintln!("  3. courseape data import --scope grades --file <path.json>  # 或手動匯入");
                    } else {
                        println!("{}", serde_json::to_string_pretty(&grades)?);
                    }
                }
                "grade-html" => {
                    // Export grade HTML path and metadata for Agent analysis
                    let path = storage::snapshot::SnapshotArchive::newest_valid_grade(None)?
                        .ok_or_else(|| {
                            anyhow::anyhow!("No grade snapshot found. Run `sync grades` first.")
                        })?;
                    let bytes = std::fs::read(&path)?;
                    let hash = {
                        use sha2::Digest;
                        let mut hasher = sha2::Sha256::new();
                        hasher.update(&bytes);
                        format!("{:x}", hasher.finalize())
                    };
                    let metadata = serde_json::json!({
                        "schema_version": "1",
                        "content_type": "text/html",
                        "encoding": "utf-8",
                        "path": path.to_string_lossy(),
                        "size_bytes": bytes.len(),
                        "sha256": hash,
                    });
                    println!("{}", serde_json::to_string_pretty(&metadata)?);
                }
                "offerings" => {
                    // Export all offerings from all terms (for category lookup)
                    let mut all_offerings = Vec::new();
                    let terms = repo.list_offering_terms()?;
                    for term in &terms {
                        let offerings = repo.list_offerings(term).unwrap_or_default();
                        for o in &offerings {
                            all_offerings.push(serde_json::json!({
                                "name": o.name,
                                "op_type": o.op_type,
                                "cos_usr": o.cos_usr,
                                "term": term,
                            }));
                        }
                    }
                    eprintln!(
                        "Exported {} offerings from all terms (for category lookup).",
                        all_offerings.len()
                    );
                    if all_offerings.is_empty() {
                        eprintln!("提示：執行 courseape agent prepare graduation 同步歷史開課資料。");
                    }
                    println!("{}", serde_json::to_string_pretty(&all_offerings)?);
                }
                "schedule" => {
                    let terms = repo.list_offering_terms()?;
                    let current = crate::domain::resolver::current_term_code();
                    let target_term = terms.last().unwrap_or(&current);
                    let schedule = repo.list_schedule(target_term)?;
                    if schedule.is_empty() {
                        eprintln!("No schedule found. Import with: courseape data import --scope schedule --file <path>");
                    } else {
                        println!("{}", serde_json::to_string_pretty(&schedule)?);
                    }
                }
                _ => {
                    anyhow::bail!("Unknown scope '{}'. Valid: profile, departments, grades, grade-html, offerings, schedule", scope);
                }
            }
            Ok(())
        }
        DataCommands::Import { scope, file } => {
            let db = storage::db::open()?;
            let repo = storage::repo::Repository::new(&db);

            // Read JSON from file or stdin
            let json_str = if let Some(path) = file {
                let raw = std::fs::read_to_string(path)?;
                // Strip UTF-8 BOM if present
                raw.strip_prefix('\u{FEFF}').unwrap_or(&raw).to_string()
            } else {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            };

            match scope.as_str() {
                "grades" => {
                    // Import Agent-analyzed grade data
                    // Expected format: array of {name, credits, status, term, score?, category?}
                    let grades: Vec<crate::parsers::grade_html::CompletedCourse> =
                        serde_json::from_str(&json_str).context(
                            "Invalid grades JSON. Expected array of {name, credits, status, term}",
                        )?;
                    validate_grades(&grades)?;

                    let count = grades.len();
                    repo.upsert_analyzed_grades(&grades)?;
                    eprintln!("Imported {} analyzed courses into database.", count);
                }
                "schedule" => {
                    let phases = crate::parsers::schedule::parse_schedule_json(&json_str)?;
                    let term = phases
                        .first()
                        .map(|p| {
                            // Try to extract term from start date (YYYY-MM-DD -> YYY1/YYY2)
                            p.start.as_deref().unwrap_or("").get(..4).unwrap_or("115")
                        })
                        .unwrap_or("115");
                    let term_code = format!("{}1", term);
                    let tuples: Vec<_> = phases
                        .iter()
                        .map(|p| {
                            (
                                p.phase.clone(),
                                p.category.clone(),
                                p.start.clone(),
                                p.end.clone(),
                                p.description.clone(),
                            )
                        })
                        .collect();
                    repo.upsert_schedule(&term_code, &tuples)?;
                    eprintln!(
                        "Imported {} schedule phases for term {}.",
                        phases.len(),
                        term_code
                    );
                }
                _ => {
                    anyhow::bail!(
                        "Import scope '{}' not supported. Valid: grades, schedule",
                        scope
                    );
                }
            }
            Ok(())
        }
        DataCommands::Purge => {
            eprintln!("Purging all cached data, session, and snapshots...");
            crate::auth::session::Session::delete()?;
            storage::snapshot::SnapshotArchive::purge()?;
            if let Some(dir) = dirs::data_dir().or_else(dirs::config_dir) {
                let db_path = dir.join("courseape").join("courseape.db");
                if db_path.exists() {
                    std::fs::remove_file(&db_path)?;
                }
            }
            eprintln!("Purge complete. Keyring credentials preserved.");
            Ok(())
        }
    }
}

fn validate_grades(grades: &[crate::parsers::grade_html::CompletedCourse]) -> anyhow::Result<()> {
    for (index, grade) in grades.iter().enumerate() {
        if grade.name.trim().is_empty() {
            anyhow::bail!("Grade row {index} has an empty name");
        }
        let term = grade.term.as_bytes();
        if term.len() != 4
            || !term.iter().all(u8::is_ascii_digit)
            || !matches!(term[3], b'1' | b'2')
        {
            anyhow::bail!("Grade row {index} has invalid term '{}'.", grade.term);
        }
        if !matches!(grade.status.as_str(), "及格" | "不及格" | "停修") {
            anyhow::bail!(
                "Grade row {index} has unsupported status '{}'.",
                grade.status
            );
        }
        if grade.score.is_some_and(|score| score > 100) {
            anyhow::bail!("Grade row {index} has a score above 100");
        }
        if !matches!(grade.category.as_str(), "" | "必修" | "選修") {
            anyhow::bail!(
                "Grade row {index} has unsupported category '{}'.",
                grade.category
            );
        }
    }
    Ok(())
}

fn export_profile(
    profile: &crate::domain::profile::StudentProfile,
    include_personal: bool,
) -> serde_json::Value {
    serde_json::json!({
        "student_id": if include_personal {
            profile.student_id.clone()
        } else {
            crate::redact::profile::mask_student_id(&profile.student_id)
        },
        "dept_code": profile.dept_code,
        "dept_name": profile.dept_name,
        "enroll_year": profile.enroll_year,
        "degree": profile.degree,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_export_redacts_by_default() {
        let profile = crate::domain::profile::StudentProfile {
            student_id: "11244151".into(),
            dept_code: Some("5400B".into()),
            dept_name: Some("資訊管理學系".into()),
            enroll_year: Some(112),
            degree: Some("學士".into()),
        };
        let redacted = export_profile(&profile, false).to_string();
        assert!(!redacted.contains("11244151"));
        assert!(redacted.contains("****4151"));
        assert!(export_profile(&profile, true)
            .to_string()
            .contains("11244151"));
    }

    #[test]
    fn grade_validation_rejects_invalid_rows() {
        let grade = crate::parsers::grade_html::CompletedCourse {
            code: "CS101".into(),
            name: "程式設計".into(),
            credits: 3,
            status: "未知".into(),
            term: "1141".into(),
            score: Some(80),
            category: "必修".into(),
        };
        assert!(validate_grades(&[grade]).is_err());
    }
}
