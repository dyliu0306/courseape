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
         PRAGMA foreign_keys=ON;",
    )?;
    migrate(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> anyhow::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

const CREATE_OFFERINGS: &str = "CREATE TABLE offerings (
    code TEXT NOT NULL,
    term TEXT NOT NULL,
    course_code TEXT NOT NULL DEFAULT '',
    assignment_key TEXT NOT NULL,
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
    PRIMARY KEY (code, term, assignment_key)
)";

fn migrate(conn: &Connection) -> anyhow::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(
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

        CREATE TABLE IF NOT EXISTS schedule (
            term TEXT NOT NULL,
            phase TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT '',
            start_time TEXT,
            end_time TEXT,
            description TEXT NOT NULL DEFAULT '',
            synced_at TEXT NOT NULL,
            PRIMARY KEY (term, phase, category)
        );

        CREATE TABLE IF NOT EXISTS analyzed_grades (
            code TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL,
            credits INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT '及格',
            term TEXT NOT NULL DEFAULT '',
            score INTEGER,
            category TEXT NOT NULL DEFAULT '',
            imported_at TEXT NOT NULL,
            PRIMARY KEY (code, name, term)
        );",
    )?;

    migrate_analyzed_grades(&tx)?;
    migrate_offerings(&tx)?;
    migrate_requirements(&tx)?;
    tx.commit()?;
    Ok(())
}

fn migrate_analyzed_grades(conn: &Connection) -> anyhow::Result<()> {
    let sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='analyzed_grades'",
        [],
        |row| row.get(0),
    )?;
    if sql.contains("PRIMARY KEY (code, name, term)") {
        return Ok(());
    }

    let has_code = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('analyzed_grades') WHERE name='code'",
        [],
        |row| row.get::<_, i32>(0),
    )? > 0;
    conn.execute_batch("ALTER TABLE analyzed_grades RENAME TO _analyzed_grades_old;")?;
    conn.execute_batch(
        "CREATE TABLE analyzed_grades (
            code TEXT NOT NULL DEFAULT '', name TEXT NOT NULL,
            credits INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT '及格',
            term TEXT NOT NULL DEFAULT '', score INTEGER, category TEXT NOT NULL DEFAULT '',
            imported_at TEXT NOT NULL, PRIMARY KEY (code, name, term)
        );",
    )?;
    let code = if has_code { "code" } else { "''" };
    conn.execute_batch(&format!(
        "INSERT INTO analyzed_grades
         (code, name, credits, status, term, score, category, imported_at)
         SELECT {code}, name, credits, status, term, score, category, imported_at
         FROM _analyzed_grades_old;
         DROP TABLE _analyzed_grades_old;"
    ))?;
    Ok(())
}

fn migrate_offerings(conn: &Connection) -> anyhow::Result<()> {
    let current_sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='offerings'",
            [],
            |row| row.get(0),
        )
        .ok();

    match current_sql {
        None => {
            // Table doesn't exist, create with full schema
            conn.execute_batch(CREATE_OFFERINGS)?;
        }
        Some(ref sql) if sql.contains("PRIMARY KEY (code, term, assignment_key)") => {
            // Already correct schema
        }
        Some(_) => {
            // Old schema already collapsed multi-teacher API rows. Do not copy corrupted
            // cache forward; rebuild it from the next complete API sync.
            conn.execute_batch("ALTER TABLE offerings RENAME TO _offerings_old;")?;
            conn.execute_batch(CREATE_OFFERINGS)?;
            conn.execute_batch("DROP TABLE _offerings_old;")?;
        }
    }

    Ok(())
}

fn migrate_requirements(conn: &Connection) -> anyhow::Result<()> {
    // Add parsed_json_path column if it doesn't exist
    let has_col: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('requirements') WHERE name='parsed_json_path'",
        [],
        |row| row.get::<_, i32>(0),
    )? > 0;
    if !has_col {
        conn.execute_batch("ALTER TABLE requirements ADD COLUMN parsed_json_path TEXT NOT NULL DEFAULT ''")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_collapsed_offerings_cache_is_invalidated() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE offerings (
                code TEXT NOT NULL, term TEXT NOT NULL, class_dept TEXT NOT NULL DEFAULT '',
                PRIMARY KEY (code, term, class_dept)
            );
            INSERT INTO offerings (code, term, class_dept) VALUES ('GE481C', '1151', '2061B');",
        )
        .unwrap();
        migrate_offerings(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM offerings", [], |row| row.get(0))
            .unwrap();
        let has_assignment_key: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('offerings') WHERE name='assignment_key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(has_assignment_key, 1);
    }

    #[test]
    fn analyzed_grades_migration_preserves_existing_rows() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE analyzed_grades (
                code TEXT NOT NULL DEFAULT '', name TEXT NOT NULL, credits INTEGER NOT NULL,
                status TEXT NOT NULL, term TEXT NOT NULL, score INTEGER, category TEXT NOT NULL,
                imported_at TEXT NOT NULL, PRIMARY KEY (name, term)
            );
            INSERT INTO analyzed_grades VALUES ('CS101', '程式設計', 3, '及格', '1141', 80, '必修', 'now');",
        )
        .unwrap();
        migrate_analyzed_grades(&conn).unwrap();
        let code: String = conn
            .query_row("SELECT code FROM analyzed_grades", [], |row| row.get(0))
            .unwrap();
        assert_eq!(code, "CS101");
    }
}
