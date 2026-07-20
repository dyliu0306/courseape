use crate::storage;
use crate::{Cli, ProfileCommands};

pub async fn run(cmd: &ProfileCommands, cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);

    match cmd {
        ProfileCommands::Show => {
            match repo.get_profile()? {
                Some(profile) => {
                    if cli.no_redact_personal {
                        println!("Student ID : {}", profile.student_id);
                    } else {
                        println!(
                            "Student ID : {}",
                            crate::redact::profile::mask_student_id(&profile.student_id)
                        );
                    }
                    println!(
                        "Dept Code  : {}",
                        profile.dept_code.as_deref().unwrap_or("(未設定)")
                    );
                    println!(
                        "Dept Name  : {}",
                        profile.dept_name.as_deref().unwrap_or("(未設定)")
                    );
                    println!(
                        "Enroll Year: {}",
                        profile
                            .enroll_year
                            .map_or("(未設定)".to_string(), |y| format!("{}學年", y))
                    );
                    println!(
                        "Degree     : {}",
                        profile.degree.as_deref().unwrap_or("(未設定)")
                    );
                }
                None => {
                    eprintln!("No profile set. Run `courseape profile edit` to set up.");
                }
            }
            Ok(())
        }
        ProfileCommands::Edit => {
            let current = repo.get_profile()?;

            // Non-interactive mode: auto-detect from login session
            if std::env::var("CYCU_USERNAME").is_ok() || current.is_some() {
                let _session = crate::auth::session::Session::load()?;
                let student_id = std::env::var("CYCU_USERNAME")
                    .or_else(|_| {
                        current
                            .as_ref()
                            .map(|p| p.student_id.clone())
                            .ok_or(std::env::VarError::NotPresent)
                    })
                    .unwrap_or_default();
                if student_id.is_empty() {
                    anyhow::bail!("No student ID available. Run `courseape login` first.");
                }
                let dept_code = std::env::var("CYCU_DEPT")
                    .ok()
                    .or_else(|| current.as_ref().and_then(|p| p.dept_code.clone()));
                let dept_name = std::env::var("CYCU_DEPT_NAME")
                    .ok()
                    .or_else(|| current.as_ref().and_then(|p| p.dept_name.clone()));
                let enroll_year = std::env::var("CYCU_YEAR")
                    .ok()
                    .and_then(|y| y.parse().ok())
                    .or_else(|| current.as_ref().and_then(|p| p.enroll_year));
                let profile = crate::domain::profile::StudentProfile {
                    student_id,
                    dept_code,
                    dept_name,
                    enroll_year,
                    degree: current
                        .as_ref()
                        .and_then(|p| p.degree.clone())
                        .or_else(|| Some("學士".to_string())),
                };
                repo.upsert_profile(&profile)?;
                eprintln!("Profile saved.");
                return Ok(());
            }

            let student_id = rprompt::prompt_reply(format!(
                "Student ID [{}]: ",
                current
                    .as_ref()
                    .map_or("".to_string(), |p| p.student_id.clone())
            ))?;
            let student_id = if student_id.is_empty() {
                current.as_ref().map_or_else(
                    || anyhow::bail!("Student ID required"),
                    |p| Ok(p.student_id.clone()),
                )?
            } else {
                student_id
            };

            let dept_code = rprompt::prompt_reply(format!(
                "Department Code [{}]: ",
                current
                    .as_ref()
                    .and_then(|p| p.dept_code.as_deref())
                    .unwrap_or("")
            ))?;
            let dept_code = if dept_code.is_empty() {
                current.as_ref().and_then(|p| p.dept_code.clone())
            } else {
                Some(dept_code)
            };

            let dept_name = rprompt::prompt_reply(format!(
                "Department Name [{}]: ",
                current
                    .as_ref()
                    .and_then(|p| p.dept_name.as_deref())
                    .unwrap_or("")
            ))?;
            let dept_name = if dept_name.is_empty() {
                current.as_ref().and_then(|p| p.dept_name.clone())
            } else {
                Some(dept_name)
            };

            let enroll_year_str = rprompt::prompt_reply(format!(
                "Enrollment Year (e.g. 112) [{}]: ",
                current
                    .as_ref()
                    .and_then(|p| p.enroll_year)
                    .map_or("".to_string(), |y| y.to_string())
            ))?;
            let enroll_year = if enroll_year_str.is_empty() {
                current.as_ref().and_then(|p| p.enroll_year)
            } else {
                Some(enroll_year_str.parse()?)
            };

            let degree = rprompt::prompt_reply(format!(
                "Degree [{}]: ",
                current
                    .as_ref()
                    .and_then(|p| p.degree.as_deref())
                    .unwrap_or("學士")
            ))?;
            let degree = if degree.is_empty() {
                current
                    .as_ref()
                    .and_then(|p| p.degree.clone())
                    .or_else(|| Some("學士".to_string()))
            } else {
                Some(degree)
            };

            let profile = crate::domain::profile::StudentProfile {
                student_id,
                dept_code,
                dept_name,
                enroll_year,
                degree,
            };
            repo.upsert_profile(&profile)?;
            eprintln!("Profile saved.");
            Ok(())
        }
        ProfileCommands::Set {
            department,
            enroll_year,
            degree,
        } => {
            let mut profile = repo
                .get_profile()?
                .ok_or(crate::error::CourseapeError::ProfileNotSet)?;
            if let Some(query) = department {
                let departments = repo.list_departments(114)?;
                let candidates = crate::domain::resolver::resolve_department(query, &departments);
                if candidates.len() != 1 {
                    anyhow::bail!(
                        "Department must resolve to exactly one candidate; found {}",
                        candidates.len()
                    );
                }
                profile.dept_code = Some(candidates[0].dept_code.clone());
                profile.dept_name = Some(candidates[0].name.clone());
            }
            if let Some(year) = enroll_year {
                profile.enroll_year = Some(*year);
            }
            if let Some(value) = degree {
                profile.degree = Some(value.clone());
            }
            repo.upsert_profile(&profile)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "dept_code": profile.dept_code,
                    "dept_name": profile.dept_name,
                    "enroll_year": profile.enroll_year,
                    "degree": profile.degree,
                }))?
            );
            Ok(())
        }
    }
}
