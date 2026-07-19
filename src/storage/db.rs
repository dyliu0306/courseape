use anyhow::Context;
use rusqlite::Connection;
use std::path::PathBuf;

fn db_path() -> anyhow::Result<PathBuf> {
    let dir = dirs::data_dir()
        .or_else(dirs::config_dir)
        .context("Cannot determine app data directory")?
        .join("courseape");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("courseape.db"))
}

pub fn open() -> anyhow::Result<Connection> {
    let path = db_path()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;"
    )?;
    migrate(&conn)?;
    Ok(conn)
}

const CREATE_OFFERINGS: &str = "CREATE TABLE offerings (
    code TEXT NOT NULL,
    term TEXT NOT NULL,
    name TEXT NOT NULL,
    name_en TEXT NOT NULL DEFAULT '',
    teacher TEXT NOT NULL,
    teacher_id TEXT NOT NULL DEFAULT '',
    credits INTEGER NOT NULL,
    dept_code TEXT NOT NULL,
    dept_name TEXT NOT NULL DEFAULT '',
    class_dept TEXT NOT NULL DEFAULT '',
    class_dept_name TEXT NOT NULL DEFAULT '',
    admin_dept TEXT NOT NULL DEFAULT '',
    admin_dept_name TEXT NOT NULL DEFAULT '',
    time_slots TEXT NOT NULL,
    classroom TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    max_capacity INTEGER,
    enrolled INTEGER,
    remaining INTEGER,
    div TEXT NOT NULL DEFAULT '',
    course_type TEXT NOT NULL DEFAULT '',
    language TEXT NOT NULL DEFAULT '',
    is_emi INTEGER NOT NULL DEFAULT 0,
    is_english INTEGER NOT NULL DEFAULT 0,
    is_distance INTEGER NOT NULL DEFAULT 0,
    is_pbl INTEGER NOT NULL DEFAULT 0,
    is_programming INTEGER NOT NULL DEFAULT 0,
    sdgs TEXT NOT NULL DEFAULT '',
    spec TEXT NOT NULL DEFAULT '',
    cross_name TEXT NOT NULL DEFAULT '',
    memo TEXT NOT NULL DEFAULT '',
    is_stop INTEGER NOT NULL DEFAULT 0,
    auto_set INTEGER NOT NULL DEFAULT 0,
    semester_half TEXT NOT NULL DEFAULT '全學期',
    op_clock REAL NOT NULL DEFAULT 0,
    tch_clock REAL NOT NULL DEFAULT 0,
    op_type TEXT NOT NULL DEFAULT '',
    cos_usr TEXT NOT NULL DEFAULT '',
    synced_at TEXT NOT NULL,
    PRIMARY KEY (code, term, class_dept)
)";

fn migrate(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS departments (
            dept_code TEXT NOT NULL,
            name TEXT NOT NULL,
            year INTEGER NOT NULL,
            PRIMARY KEY (dept_code, year)
        );

        CREATE TABLE IF NOT EXISTS profile (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            student_id TEXT NOT NULL,
            dept_code TEXT,
            dept_name TEXT,
            enroll_year INTEGER,
            degree TEXT
        );

        CREATE TABLE IF NOT EXISTS requirements (
            year INTEGER NOT NULL,
            dept_code TEXT NOT NULL,
            raw_pdf_path TEXT NOT NULL,
            schema_version TEXT NOT NULL,
            parsed INTEGER NOT NULL DEFAULT 0,
            synced_at TEXT NOT NULL,
            PRIMARY KEY (year, dept_code)
        );

        CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            file_path TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS shortlist (
            code TEXT NOT NULL,
            term TEXT NOT NULL,
            added_at TEXT NOT NULL,
            PRIMARY KEY (code, term)
        );

        CREATE TABLE IF NOT EXISTS analyzed_grades (
            name TEXT NOT NULL,
            credits INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT '及格',
            term TEXT NOT NULL DEFAULT '',
            score INTEGER,
            category TEXT NOT NULL DEFAULT '',
            imported_at TEXT NOT NULL,
            PRIMARY KEY (name, term)
        );"
    )?;

    migrate_offerings(conn)?;
    Ok(())
}

fn migrate_offerings(conn: &Connection) -> anyhow::Result<()> {
    let current_sql: Option<String> = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='offerings'",
        [], |row| row.get(0),
    ).ok();

    match current_sql {
        None => {
            // Table doesn't exist, create with full schema
            conn.execute_batch(CREATE_OFFERINGS)?;
        }
        Some(ref sql) if sql.contains("PRIMARY KEY (code, term, class_dept)") => {
            // Already correct schema
        }
        Some(_) => {
            // Old schema, migrate data
            let has_class_dept: bool = conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('offerings') WHERE name='class_dept'",
                [], |row| row.get::<_, i32>(0),
            ).unwrap_or(0) > 0;

            conn.execute_batch("ALTER TABLE offerings RENAME TO _offerings_old;")?;
            conn.execute_batch(CREATE_OFFERINGS)?;

            if has_class_dept {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO offerings
                     (code, term, name, name_en, teacher, teacher_id, credits, dept_code, dept_name,
                      class_dept, class_dept_name, admin_dept, admin_dept_name, time_slots, classroom,
                      category, max_capacity, enrolled, remaining, div, course_type, language,
                      is_emi, is_english, is_distance, is_pbl, is_programming, sdgs, spec, cross_name,
                      memo, is_stop, auto_set, semester_half, op_clock, tch_clock, op_type, cos_usr, synced_at)
                     SELECT code, term, name, name_en, teacher, teacher_id, credits, dept_code, dept_name,
                      class_dept, class_dept_name, admin_dept, admin_dept_name, time_slots, classroom,
                      category, max_capacity, enrolled, remaining, div, course_type, language,
                      is_emi, is_english, is_distance, is_pbl, is_programming, sdgs, spec, cross_name,
                      memo, is_stop, auto_set, semester_half, op_clock, tch_clock, op_type, cos_usr, synced_at
                     FROM _offerings_old;"
                )?;
            } else {
                conn.execute_batch(
                    "INSERT OR IGNORE INTO offerings
                     (code, term, name, teacher, credits, dept_code, time_slots, category,
                      max_capacity, remaining, synced_at, class_dept)
                     SELECT code, term, name, teacher, credits, dept_code, time_slots, category,
                      max_capacity, remaining, synced_at, '' FROM _offerings_old;"
                )?;
            }
            conn.execute_batch("DROP TABLE _offerings_old;")?;
        }
    }

    Ok(())
}
