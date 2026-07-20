use crate::cli;
use crate::{AgentCommands, Cli, PrepareCommands};

/// One-key initialization: login + setup + prepare planning.
pub async fn run(cli: &Cli, department: &str, term: Option<&str>) -> anyhow::Result<()> {
    eprintln!("=== CourseApe 一鍵初始化 ===");
    eprintln!();

    // Step 1: Login
    eprintln!("▸ 步驟 1/3：登入...");
    cli::auth::run_login(cli).await?;

    // Step 2: Setup with department
    eprintln!();
    eprintln!("▸ 步驟 2/3：設定個人資料...");
    cli::agent::run(
        &AgentCommands::Setup {
            department: Some(department.to_string()),
        },
        cli,
    )
    .await?;

    // Step 3: Prepare planning
    eprintln!();
    eprintln!("▸ 步驟 3/3：準備選課資料...");
    let planning_term = term.unwrap_or("auto").to_string();
    cli::agent::run(
        &AgentCommands::Prepare(PrepareCommands::Planning {
            term: planning_term,
        }),
        cli,
    )
    .await?;

    eprintln!();
    eprintln!("=== 初始化完成 ===");
    Ok(())
}
