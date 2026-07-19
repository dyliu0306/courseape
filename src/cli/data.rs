use crate::storage;
use crate::{Cli, DataCommands};
use anyhow::Context;
use std::io::{Read, Write};

pub async fn run(cmd: &DataCommands, _cli: &Cli) -> anyhow::Result<()> {
    match cmd {
        DataCommands::Export {
            scope,
            format: _,
            output_file: _,
        } => {
            let db = storage::db::open()?;
            let repo = storage::repo::Repository::new(&db);

            match scope.as_str() {
                "profile" => {
                    let profile = repo.get_profile()?;
                    println!("{}", serde_json::to_string_pretty(&profile)?);
                }
                "departments" => {
                    let depts = repo.list_departments(114)?;
                    println!("{}", serde_json::to_string_pretty(&depts)?);
                }
                "grades" => {
                    // Export analyzed grades from DB
                    let grades = repo.list_analyzed_grades()?;
                    if grades.is_empty() {
                        eprintln!("No analyzed grades found. Run `sync grades` then ask Agent to analyze.");
                        eprintln!("Or import directly: courseape data import --scope grades --file <path>");
                    } else {
                        println!("{}", serde_json::to_string_pretty(&grades)?);
                    }
                }
                "grade-html" => {
                    // Export raw grade HTML for Agent analysis
                    let snap_dir = dirs::data_dir()
                        .or_else(dirs::config_dir)
                        .ok_or_else(|| anyhow::anyhow!("Cannot find data dir"))?
                        .join("courseape")
                        .join("snapshots");
                    let mut grade_file = None;
                    if snap_dir.exists() {
                        for entry in std::fs::read_dir(&snap_dir)? {
                            let entry = entry?;
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.starts_with("grades") {
                                grade_file = Some(entry.path());
                                break;
                            }
                        }
                    }
                    let path = grade_file.ok_or_else(|| anyhow::anyhow!("No grade snapshot found. Run `sync grades` first."))?;
                    let bytes = std::fs::read(&path)?;
                    // Output raw bytes (Agent will handle encoding)
                    std::io::stdout().write_all(&bytes)?;
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
                    eprintln!("Exported {} offerings from all terms (for category lookup).", all_offerings.len());
                    println!("{}", serde_json::to_string_pretty(&all_offerings)?);
                }
                _ => {
                    anyhow::bail!("Unknown scope '{}'. Valid: profile, departments, grades, grade-html, offerings", scope);
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
                        serde_json::from_str(&json_str)
                            .context("Invalid grades JSON. Expected array of {name, credits, status, term}")?;

                    let count = grades.len();
                    repo.upsert_analyzed_grades(&grades)?;
                    eprintln!("Imported {} analyzed courses into database.", count);
                }
                _ => {
                    anyhow::bail!("Import scope '{}' not supported. Valid: grades", scope);
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
