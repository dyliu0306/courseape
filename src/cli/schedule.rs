use crate::storage;
use crate::{Cli, ScheduleCommands};

pub async fn run(cmd: &ScheduleCommands, cli: &Cli) -> anyhow::Result<()> {
    match cmd {
        ScheduleCommands::Show { term } => {
            let db = storage::db::open()?;
            let repo = storage::repo::Repository::new(&db);
            let phases = repo.list_schedule(term)?;

            if phases.is_empty() {
                eprintln!("尚未匯入 {} 選課時程。", term);
                eprintln!("請先執行：");
                eprintln!(
                    "  courseape schedule template --term {} > schedule.json",
                    term
                );
                eprintln!("  編輯 schedule.json 後執行：");
                eprintln!("  courseape data import --scope schedule --file schedule.json");
                return Ok(());
            }

            // Show next phase
            if let Some(next) = repo.next_schedule_phase(term)? {
                eprintln!("▸ 下一個階段：{}", next.phase);
                if let Some(ref start) = next.start_time {
                    eprintln!("  開始：{}", start);
                }
                if let Some(ref end) = next.end_time {
                    eprintln!("  結束：{}", end);
                }
                if !next.description.is_empty() {
                    eprintln!("  說明：{}", next.description);
                }
                eprintln!();
            }

            match cli.output {
                crate::OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&phases)?);
                }
                crate::OutputFormat::Csv => {
                    let mut wtr = csv::WriterBuilder::new().from_writer(std::io::stdout());
                    wtr.write_record(["phase", "category", "start", "end", "description"])?;
                    for p in &phases {
                        wtr.write_record([
                            &p.phase,
                            &p.category,
                            p.start_time.as_deref().unwrap_or(""),
                            p.end_time.as_deref().unwrap_or(""),
                            &p.description,
                        ])?;
                    }
                    wtr.flush()?;
                }
                crate::OutputFormat::Table => {
                    use comfy_table::presets::UTF8_FULL_CONDENSED;
                    let mut table = comfy_table::Table::new();
                    table.load_preset(UTF8_FULL_CONDENSED);
                    table.set_header(["階段", "類別", "開始", "結束", "說明"]);
                    for p in &phases {
                        table.add_row([
                            &p.phase,
                            &p.category,
                            p.start_time.as_deref().unwrap_or("-"),
                            p.end_time.as_deref().unwrap_or("-"),
                            &p.description,
                        ]);
                    }
                    println!("{table}");
                }
            }
            Ok(())
        }
        ScheduleCommands::Template { term } => {
            let template = crate::parsers::schedule::schedule_template(term);
            println!("{}", serde_json::to_string_pretty(&template)?);
            eprintln!("已輸出 {} 選課時程模板。", term);
            eprintln!("請編輯後執行：");
            eprintln!("  courseape data import --scope schedule --file schedule.json");
            Ok(())
        }
    }
}
