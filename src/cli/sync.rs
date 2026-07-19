use crate::connectors::necessary_course::NecessaryCourseConnector;
use crate::parsers::department_json;
use crate::storage;
use crate::{Cli, SyncCommands};

pub async fn run(cmd: &SyncCommands, _cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);

    match cmd {
        SyncCommands::Departments { year } => {
            eprintln!("Fetching department list for year {}...", year);
            let result = NecessaryCourseConnector::query_departments(*year).await?;

            if result.status != 200 {
                anyhow::bail!("API returned status {}", result.status);
            }

            let json: serde_json::Value = serde_json::from_slice(&result.body_bytes)?;
            let departments = department_json::parse_departments(&json, *year)?;
            let count = departments.len();

            repo.upsert_departments(&departments)?;

            // Save snapshot
            let (hash, _path) = storage::snapshot::SnapshotArchive::save(
                &format!("departments_{}", year),
                &result.body_bytes,
            )?;

            eprintln!("Synced {} departments (hash: {}...).", count, &hash[..16]);
            Ok(())
        }
        SyncCommands::Requirements { year } => {
            let profile = repo.get_profile()?.ok_or(
                crate::error::CourseapeError::ProfileNotSet
            )?;
            let dept_code = profile.dept_code.as_deref().ok_or(
                crate::error::CourseapeError::ProfileNotSet
            )?;

            // Determine year from student ID first 3 digits if not specified
            let effective_year = if *year > 0 {
                *year
            } else {
                let sid = &profile.student_id;
                if sid.len() >= 3 {
                    sid[..3].parse::<u32>().unwrap_or(114)
                } else {
                    114
                }
            };

            eprintln!("Downloading requirement PDF for {} (year {})...", dept_code, effective_year);
            let result = NecessaryCourseConnector::download_requirement_pdf(effective_year, dept_code).await?;

            if result.status != 200 {
                anyhow::bail!("PDF download returned status {}", result.status);
            }

            let (hash, path) = storage::snapshot::SnapshotArchive::save(
                &format!("requirements_{}_{}.pdf", effective_year, dept_code),
                &result.body_bytes,
            )?;

            repo.upsert_requirement(effective_year, dept_code, &path.to_string_lossy(), "1.0.0")?;

            eprintln!("Requirement PDF saved (hash: {}...).", &hash[..16]);
            eprintln!("Work-package ready at: {}", path.display());
            Ok(())
        }
        SyncCommands::Grades => {
            let session = crate::auth::session::Session::load()?.ok_or(
                crate::error::CourseapeError::NotLoggedIn
            )?;

            eprintln!("Fetching grade HTML...");
            let result = crate::connectors::itouch::ItouchConnector::fetch_grades(&session.cookie).await?;

            if result.status != 200 {
                anyhow::bail!("Grade fetch returned status {}", result.status);
            }

            let (hash, path) = storage::snapshot::SnapshotArchive::save(
                "grades",
                &result.body_bytes,
            )?;

            eprintln!("Grade HTML saved (hash: {}...).", &hash[..16]);
            eprintln!("Work-package ready at: {}", path.display());
            Ok(())
        }
    }
}
