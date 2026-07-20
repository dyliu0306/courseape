use crate::auth::keyring::StoredCredentials;
use crate::auth::session::Session;
use crate::connectors::itouch::ItouchConnector;
use crate::storage;
use crate::{Cli, CredentialsCommands};

pub async fn run_login(_cli: &Cli) -> anyhow::Result<()> {
    // Try loading from keyring, then env vars, then interactive prompt
    let creds = match StoredCredentials::load() {
        Ok(Some(c)) => {
            eprintln!(
                "Using stored credentials for ****{}",
                &c.student_id[c.student_id.len().min(4)..]
            );
            c
        }
        Ok(None) => {
            eprintln!("Keyring: entry not found (NoEntry).");
            // Check env vars for non-interactive mode
            if let (Ok(id), Ok(pass)) = (
                std::env::var("CYCU_USERNAME"),
                std::env::var("CYCU_PASSWORD"),
            ) {
                eprintln!("Using credentials from environment variables.");
                StoredCredentials::new(id, pass)?
            } else {
                eprintln!("No stored credentials found.");
                eprintln!(
                    "Please set credentials first: courseape credentials set"
                );
                eprintln!("Or set environment variables: CYCU_USERNAME and CYCU_PASSWORD");
                eprintln!();
                eprintln!("Enter your CYCU credentials to continue:");
                let id = rprompt::prompt_reply("Student ID: ")?;
                let pass = rpassword::prompt_password("Password: ")?;
                StoredCredentials::new(id, pass)?
            }
        }
        Err(e) => {
            eprintln!("Keyring error: {e:#}");
            if let (Ok(id), Ok(pass)) = (
                std::env::var("CYCU_USERNAME"),
                std::env::var("CYCU_PASSWORD"),
            ) {
                eprintln!("Using credentials from environment variables (fallback).");
                StoredCredentials::new(id, pass)?
            } else {
                anyhow::bail!("Keyring error and no env vars set: {e:#}");
            }
        }
    };

    eprintln!("Logging in to iTouch...");
    let (cookie, login_token) = ItouchConnector::login(&creds.student_id, &creds.password).await?;

    // Save credentials to keyring on first successful login (from any source)
    if StoredCredentials::load()?.is_none() {
        creds.save()?;
        eprintln!("Credentials saved to OS keyring.");
    }

    let has_token = login_token.is_some();
    let session = Session {
        cookie,
        login_token,
        logged_in_at: chrono::Utc::now(),
    };
    session.save()?;

    eprintln!("Login successful. Session saved.");
    if has_token {
        eprintln!("loginToken obtained for elective API.");
    }
    Ok(())
}

pub async fn run_status(_cli: &Cli) -> anyhow::Result<()> {
    match Session::load()? {
        Some(session) => {
            let valid = ItouchConnector::validate_session(&session.cookie).await?;
            eprintln!("Session: {}", if valid { "valid" } else { "expired" });
            eprintln!(
                "Logged in at: {}",
                session.logged_in_at.format("%Y-%m-%d %H:%M:%S UTC")
            );

            // Try to show profile if exists
            let db = storage::db::open()?;
            let repo = storage::repo::Repository::new(&db);
            if let Some(profile) = repo.get_profile()? {
                let masked_id = crate::redact::profile::mask_student_id(&profile.student_id);
                eprintln!(
                    "Profile: {} / {} / {}",
                    masked_id,
                    profile.dept_name.as_deref().unwrap_or("(未設定)"),
                    profile
                        .enroll_year
                        .map_or("(未設定)".to_string(), |y| format!("{}學年", y)),
                );
            }
            if valid {
                Ok(())
            } else {
                anyhow::bail!("Session expired. Run `courseape login` again.")
            }
        }
        None => {
            eprintln!("Status: not logged in");
            std::process::exit(1);
        }
    }
}

pub async fn run_logout(_cli: &Cli, clear_credentials: bool) -> anyhow::Result<()> {
    Session::delete()?;
    eprintln!("Session cleared.");

    if clear_credentials {
        eprintln!("WARNING: This will delete the stored credentials from OS keyring.");
        eprintln!("You will need to re-enter credentials next time.");
        eprintln!("Type 'y' to confirm:");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().eq_ignore_ascii_case("y") {
            StoredCredentials::delete()?;
            eprintln!("Credentials cleared from OS keyring.");
        } else {
            eprintln!("Credentials kept.");
        }
    }

    Ok(())
}

pub async fn run_credentials(cmd: &CredentialsCommands, _cli: &Cli) -> anyhow::Result<()> {
    match cmd {
        CredentialsCommands::Set => {
            eprintln!("WARNING: This will update the stored credentials in OS keyring.");
            eprintln!();
            let (id, pass) = if let (Ok(id), Ok(pass)) = (
                std::env::var("CYCU_USERNAME"),
                std::env::var("CYCU_PASSWORD"),
            ) {
                (id, pass)
            } else {
                let id = rprompt::prompt_reply("New Student ID: ")?;
                let pass = rpassword::prompt_password("New Password: ")?;
                (id, pass)
            };
            let creds = StoredCredentials::new(id, pass)?;

            // Validate by logging in
            eprintln!("Validating new credentials...");
            let (cookie, login_token) =
                ItouchConnector::login(&creds.student_id, &creds.password).await?;

            // Replace stored state only after the new credentials are verified.
            creds.save()?;
            eprintln!("Credentials saved to OS keyring.");

            let has_token = login_token.is_some();
            let session = Session {
                cookie,
                login_token,
                logged_in_at: chrono::Utc::now(),
            };
            session.save()?;
            eprintln!("Validation successful. Session refreshed.");
            if has_token {
                eprintln!("loginToken obtained for elective API.");
            }
            Ok(())
        }
    }
}
