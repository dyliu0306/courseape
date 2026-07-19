use crate::storage;
use crate::{Cli, ShortlistCommands};

pub async fn run(cmd: &ShortlistCommands, _cli: &Cli) -> anyhow::Result<()> {
    let db = storage::db::open()?;
    let repo = storage::repo::Repository::new(&db);

    match cmd {
        ShortlistCommands::Add { course_code, term } => {
            // Verify course exists in offerings
            let offerings = repo.list_offerings(term)?;
            let offering = offerings.iter().find(|o| o.code == *course_code);
            if offering.is_none() {
                anyhow::bail!("Course {} not found in cached offerings for term {}. Run `courses offerings --term {}` first.", course_code, term, term);
            }
            let o = offering.unwrap();
            let added = repo.add_to_shortlist(course_code, term)?;
            if added {
                eprintln!("Added {} ({}) to shortlist for term {}.", course_code, o.name, term);
            } else {
                eprintln!("{} ({}) already in shortlist for term {}.", course_code, o.name, term);
            }
            Ok(())
        }
        ShortlistCommands::Remove { course_code, term } => {
            repo.remove_from_shortlist(course_code, term)?;
            eprintln!("Removed {} from shortlist for term {}.", course_code, term);
            Ok(())
        }
        ShortlistCommands::List { term } => {
            let codes = repo.list_shortlist(term)?;
            if codes.is_empty() {
                eprintln!("Shortlist is empty for term {}.", term);
                return Ok(());
            }
            let offerings = repo.list_offerings(term)?;
            eprintln!("Shortlist (term {}): {} courses", term, codes.len());
            for code in &codes {
                if let Some(o) = offerings.iter().find(|o| &o.code == code) {
                    println!("  {} | {} | {} | {}cr | slots: {}",
                        o.code, o.name, o.teacher, o.credits, o.time_slots.join(", "));
                } else {
                    println!("  {} (not in cache)", code);
                }
            }
            Ok(())
        }
        ShortlistCommands::Clear { term } => {
            repo.clear_shortlist(term)?;
            eprintln!("Shortlist cleared for term {}.", term);
            Ok(())
        }
    }
}
