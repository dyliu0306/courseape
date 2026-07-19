use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

mod analysis;
mod auth;
mod cli;
mod connectors;
mod domain;
mod error;
mod output;
mod parsers;
mod redact;
mod storage;

#[derive(Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    Json,
    Csv,
    #[default]
    Table,
}

#[derive(Parser)]
#[command(
    name = "courseape",
    version,
    about = "CYCU course-planning CLI: graduation analysis, course browsing, conflict detection"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[arg(long, global = true, default_value_t = false)]
    redact_personal: bool,

    #[arg(long, global = true, default_value_t = false)]
    no_redact_personal: bool,

    #[arg(long, global = true, default_value_t = false)]
    offline: bool,

    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::default())]
    output: OutputFormat,

    #[arg(long, global = true)]
    verbose: bool,

    #[arg(long, global = true)]
    silent: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Login to iTouch and save session
    Login,
    /// Check current session status
    Status,
    /// Remove saved session
    Logout {
        /// Also clear OS keyring credentials
        #[arg(long)]
        clear_credentials: bool,
    },
    #[command(subcommand)]
    Credentials(CredentialsCommands),
    #[command(subcommand)]
    Profile(ProfileCommands),
    #[command(subcommand)]
    Sync(SyncCommands),
    #[command(subcommand)]
    Courses(Box<CoursesCommands>),
    #[command(subcommand)]
    Shortlist(ShortlistCommands),
    #[command(subcommand)]
    Data(Box<DataCommands>),
    #[command(subcommand)]
    Skills(SkillsCommands),
}

#[derive(Subcommand)]
pub enum CredentialsCommands {
    /// Update student ID and password in OS keyring
    Set,
}

#[derive(Subcommand)]
pub enum ProfileCommands {
    /// Show current profile
    Show,
    /// Edit profile interactively
    Edit,
}

#[derive(Subcommand)]
pub enum SyncCommands {
    /// Sync department list for a year
    Departments {
        #[arg(long)]
        year: u32,
    },
    /// Download graduation requirement PDF for your department
    Requirements {
        #[arg(long)]
        year: u32,
    },
    /// Sync historical grades from iTouch
    Grades,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum CoursesCommands {
    /// List course offerings for a term
    Offerings {
        #[arg(long)]
        term: String,
    },
    /// Filter cached course offerings
    Filter {
        #[arg(long)]
        term: String,
        /// 課程代碼
        #[arg(long)]
        code: Option<String>,
        /// 課程名稱(中/英)
        #[arg(long)]
        keyword: Option<String>,
        /// 授課教師
        #[arg(long)]
        teacher: Option<String>,
        /// 人事代碼
        #[arg(long)]
        teacher_id: Option<String>,
        /// 系所代碼(AUTHORITY_DEPT)
        #[arg(long)]
        dept: Option<String>,
        /// 班級
        #[arg(long)]
        class_dept: Option<String>,
        /// 必修/選修
        #[arg(long)]
        r#type: Option<String>,
        /// 學分
        #[arg(long)]
        credit: Option<u32>,
        /// 部別(B=學士, M=碩士, D=博士, H=學士後)
        #[arg(long)]
        div: Option<String>,
        /// 授課語言
        #[arg(long)]
        language: Option<String>,
        /// 上課日(1-7)
        #[arg(long)]
        day: Option<u32>,
        /// 上課時段
        #[arg(long)]
        period: Option<String>,
        /// 教室
        #[arg(long)]
        classroom: Option<String>,
        /// 通識向度
        #[arg(long)]
        general: Option<String>,
        /// 只顯示全英語課程(EMI)
        #[arg(long)]
        emi: bool,
        /// 只顯示English授課
        #[arg(long)]
        english: bool,
        /// 只顯示遠距教學課程
        #[arg(long)]
        distance: bool,
        /// 只顯示PBL課程
        #[arg(long)]
        pbl: bool,
        /// 只顯示程式設計課程
        #[arg(long)]
        programming: bool,
        /// 只顯示有餘額課程
        #[arg(long)]
        available: bool,
        /// 期程(全學期/半學期)
        #[arg(long)]
        semester: Option<String>,
        /// 只顯示跨系/聯盟課程
        #[arg(long)]
        cross: bool,
        /// SDGs目標
        #[arg(long)]
        sdgs: Option<String>,
    },
    /// Detect time-slot conflicts in planned courses
    Conflicts {
        #[arg(long)]
        term: String,
    },
    /// Download course syllabus PDF
    Syllabus {
        course_code: String,
        #[arg(long)]
        term: String,
    },
    /// Show timetable for planned courses (shortlist + required)
    Timetable {
        #[arg(long)]
        term: String,
    },
    /// Auto-scan: match required/retake courses against offerings, add to shortlist
    Plan {
        #[arg(long)]
        term: String,
        /// Only show matches, don't add to shortlist
        #[arg(long)]
        dry_run: bool,
    },
    /// Sync historical offerings for all past terms (for grade analysis)
    History {
        /// Student ID (first 3 digits = enrollment year)
        #[arg(long)]
        student_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ShortlistCommands {
    /// Add a course to shortlist
    Add {
        course_code: String,
        #[arg(long)]
        term: String,
    },
    /// Remove a course from shortlist
    Remove {
        course_code: String,
        #[arg(long)]
        term: String,
    },
    /// List shortlisted courses
    List {
        #[arg(long)]
        term: String,
    },
    /// Clear all shortlisted courses for a term
    Clear {
        #[arg(long)]
        term: String,
    },
}

#[derive(Subcommand)]
pub enum DataCommands {
    /// Export local data as work-package or standalone file
    Export {
        #[arg(long)]
        scope: String,
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        output_file: Option<PathBuf>,
    },
    /// Import Agent analysis results (JSON from file or stdin)
    Import {
        #[arg(long)]
        scope: String,
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Purge all cached data, session, snapshots (keeps keyring)
    Purge,
}

#[derive(Subcommand)]
pub enum SkillsCommands {
    /// Install CourseApe skill to an agent platform
    Install {
        /// Agent platform (claude, codex, opencode)
        platform: Option<String>,
        /// Detect installed agents and install to all
        #[arg(long)]
        all: bool,
    },
    /// Print the raw SKILL.md content
    Show,
}

fn main() {
    let cli = Cli::parse();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        if let Err(e) = run_command(cli).await {
            eprintln!("Error: {e:#}");
            std::process::exit(1);
        }
    });
}

async fn run_command(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Login => cli::auth::run_login(&cli).await,
        Commands::Status => cli::auth::run_status(&cli).await,
        Commands::Logout { clear_credentials } => {
            cli::auth::run_logout(&cli, clear_credentials).await
        }
        Commands::Credentials(ref cmd) => cli::auth::run_credentials(cmd, &cli).await,
        Commands::Profile(ref cmd) => cli::profile::run(cmd, &cli).await,
        Commands::Sync(ref cmd) => cli::sync::run(cmd, &cli).await,
        Commands::Courses(ref cmd) => cli::courses::run(cmd, &cli).await,
        Commands::Shortlist(ref cmd) => cli::shortlist::run(cmd, &cli).await,
        Commands::Data(ref cmd) => cli::data::run(cmd, &cli).await,
        Commands::Skills(ref cmd) => cli::skills::run(cmd, &cli).await,
    }
}
