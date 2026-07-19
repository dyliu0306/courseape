use rusqlite::Connection;
use crate::domain::department::Department;
use crate::domain::course_offering::CourseOffering;
use crate::domain::profile::StudentProfile;

pub struct Repository<'a> {
    conn: &'a Connection,
}

impl<'a> Repository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ── Departments ──────────────────────────────────────────────

    pub fn upsert_departments(&self, departments: &[Department]) -> anyhow::Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO departments (dept_code, name, year) VALUES (?1, ?2, ?3)
             ON CONFLICT(dept_code, year) DO UPDATE SET name = excluded.name"
        )?;
        for d in departments {
            stmt.execute((&d.dept_code, &d.name, d.year))?;
        }
        Ok(())
    }

    pub fn list_departments(&self, year: u32) -> anyhow::Result<Vec<Department>> {
        let mut stmt = self.conn.prepare(
            "SELECT dept_code, name, year FROM departments WHERE year = ?1 ORDER BY dept_code"
        )?;
        let rows = stmt.query_map([year], |row| {
            Ok(Department {
                dept_code: row.get(0)?,
                name: row.get(1)?,
                year: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ── Profile ─────────────────────────────────────────────────

    pub fn get_profile(&self) -> anyhow::Result<Option<StudentProfile>> {
        let mut stmt = self.conn.prepare(
            "SELECT student_id, dept_code, dept_name, enroll_year, degree FROM profile WHERE id = 1"
        )?;
        let mut rows = stmt.query_map([], |row| {
            Ok(StudentProfile {
                student_id: row.get(0)?,
                dept_code: row.get(1)?,
                dept_name: row.get(2)?,
                enroll_year: row.get(3)?,
                degree: row.get(4)?,
            })
        })?;
        match rows.next() {
            Some(Ok(p)) => Ok(Some(p)),
            _ => Ok(None),
        }
    }

    pub fn upsert_profile(&self, profile: &StudentProfile) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO profile (id, student_id, dept_code, dept_name, enroll_year, degree)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               student_id = excluded.student_id,
               dept_code = excluded.dept_code,
               dept_name = excluded.dept_name,
               enroll_year = excluded.enroll_year,
               degree = excluded.degree",
            (
                &profile.student_id,
                &profile.dept_code,
                &profile.dept_name,
                profile.enroll_year,
                &profile.degree,
            ),
        )?;
        Ok(())
    }

    // ── Offerings ───────────────────────────────────────────────

    pub fn upsert_offerings(&self, term: &str, offerings: &[CourseOffering]) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "INSERT INTO offerings (code, term, name, name_en, teacher, teacher_id, credits, dept_code, dept_name,
             class_dept, class_dept_name, admin_dept, admin_dept_name, time_slots, classroom, category,
             max_capacity, enrolled, remaining, div, course_type, language, is_emi, is_english,
             is_distance, is_pbl, is_programming, sdgs, spec, cross_name, memo, is_stop, auto_set,
             semester_half, op_clock, tch_clock, op_type, cos_usr, synced_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,?39)
             ON CONFLICT(code, term, class_dept) DO UPDATE SET
               name=excluded.name, name_en=excluded.name_en, teacher=excluded.teacher, teacher_id=excluded.teacher_id,
               credits=excluded.credits, dept_code=excluded.dept_code, dept_name=excluded.dept_name,
               class_dept_name=excluded.class_dept_name,
               admin_dept=excluded.admin_dept, admin_dept_name=excluded.admin_dept_name,
               time_slots=excluded.time_slots, classroom=excluded.classroom, category=excluded.category,
               max_capacity=excluded.max_capacity, enrolled=excluded.enrolled, remaining=excluded.remaining,
               div=excluded.div, course_type=excluded.course_type, language=excluded.language,
               is_emi=excluded.is_emi, is_english=excluded.is_english, is_distance=excluded.is_distance,
               is_pbl=excluded.is_pbl, is_programming=excluded.is_programming, sdgs=excluded.sdgs,
               spec=excluded.spec, cross_name=excluded.cross_name, memo=excluded.memo,
               is_stop=excluded.is_stop, auto_set=excluded.auto_set, semester_half=excluded.semester_half,
               op_clock=excluded.op_clock, tch_clock=excluded.tch_clock, op_type=excluded.op_type,
               cos_usr=excluded.cos_usr, synced_at=excluded.synced_at"
        )?;
        for o in offerings {
            let slots = serde_json::to_string(&o.time_slots)?;
            stmt.execute(rusqlite::params![
                &o.code, term, &o.name, &o.name_en, &o.teacher, &o.teacher_id,
                o.credits, &o.dept_code, &o.dept_name, &o.class_dept, &o.class_dept_name,
                &o.admin_dept, &o.admin_dept_name, &slots, &o.classroom, &o.category,
                o.max_capacity, o.enrolled, o.remaining, &o.div, &o.course_type, &o.language,
                o.is_emi as i32, o.is_english as i32, o.is_distance as i32,
                o.is_pbl as i32, o.is_programming as i32, &o.sdgs, &o.spec, &o.cross_name,
                &o.memo, o.is_stop as i32, o.auto_set as i32, &o.semester_half,
                o.op_clock, o.tch_clock, &o.op_type, &o.cos_usr, &now,
            ])?;
        }
        Ok(())
    }

    pub fn list_offerings(&self, term: &str) -> anyhow::Result<Vec<CourseOffering>> {
        let mut stmt = self.conn.prepare(
            "SELECT code, name, name_en, teacher, teacher_id, credits, dept_code, dept_name,
             class_dept, class_dept_name, admin_dept, admin_dept_name, time_slots, classroom,
             category, max_capacity, enrolled, remaining, div, course_type, language,
             is_emi, is_english, is_distance, is_pbl, is_programming, sdgs, spec, cross_name,
             memo, is_stop, auto_set, semester_half, op_clock, tch_clock, op_type, cos_usr
             FROM offerings WHERE term = ?1 ORDER BY code"
        )?;
        let rows = stmt.query_map([term], |row| {
            let slots_str: String = row.get(12)?;
            Ok(CourseOffering {
                code: row.get(0)?,
                name: row.get(1)?,
                name_en: row.get(2)?,
                teacher: row.get(3)?,
                teacher_id: row.get(4)?,
                credits: row.get(5)?,
                dept_code: row.get(6)?,
                dept_name: row.get(7)?,
                class_dept: row.get(8)?,
                class_dept_name: row.get(9)?,
                admin_dept: row.get(10)?,
                admin_dept_name: row.get(11)?,
                time_slots: serde_json::from_str(&slots_str).unwrap_or_default(),
                classroom: row.get(13)?,
                category: row.get(14)?,
                max_capacity: row.get(15)?,
                enrolled: row.get(16)?,
                remaining: row.get(17)?,
                div: row.get(18)?,
                course_type: row.get(19)?,
                language: row.get(20)?,
                is_emi: row.get::<_, i32>(21)? != 0,
                is_english: row.get::<_, i32>(22)? != 0,
                is_distance: row.get::<_, i32>(23)? != 0,
                is_pbl: row.get::<_, i32>(24)? != 0,
                is_programming: row.get::<_, i32>(25)? != 0,
                sdgs: row.get(26)?,
                spec: row.get(27)?,
                cross_name: row.get(28)?,
                memo: row.get(29)?,
                is_stop: row.get::<_, i32>(30)? != 0,
                auto_set: row.get::<_, i32>(31)? != 0,
                semester_half: row.get(32)?,
                op_clock: row.get(33)?,
                tch_clock: row.get(34)?,
                op_type: row.get(35)?,
                cos_usr: row.get(36)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// List all distinct terms that have offerings in the DB.
    pub fn list_offering_terms(&self) -> anyhow::Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT term FROM offerings ORDER BY term"
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ── Requirements ────────────────────────────────────────────

    pub fn upsert_requirement(&self, year: u32, dept_code: &str, pdf_path: &str, schema_version: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO requirements (year, dept_code, raw_pdf_path, schema_version, parsed, synced_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)
             ON CONFLICT(year, dept_code) DO UPDATE SET
               raw_pdf_path = excluded.raw_pdf_path, schema_version = excluded.schema_version, synced_at = excluded.synced_at",
            (year, dept_code, pdf_path, schema_version, &now),
        )?;
        Ok(())
    }

    // ── Shortlist ──────────────────────────────────────────────

    pub fn add_to_shortlist(&self, code: &str, term: &str) -> anyhow::Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute("INSERT OR IGNORE INTO shortlist (code, term, added_at) VALUES (?1, ?2, ?3)", (code, term, &now))?;
        Ok(self.conn.changes() > 0)
    }

    pub fn remove_from_shortlist(&self, code: &str, term: &str) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM shortlist WHERE code = ?1 AND term = ?2", (code, term))?;
        Ok(())
    }

    pub fn list_shortlist(&self, term: &str) -> anyhow::Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT code FROM shortlist WHERE term = ?1 ORDER BY code")?;
        let rows = stmt.query_map([term], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn clear_shortlist(&self, term: &str) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM shortlist WHERE term = ?1", [term])?;
        Ok(())
    }

    pub fn get_planned_courses(&self, term: &str, dept_code: Option<&str>) -> anyhow::Result<Vec<CourseOffering>> {
        let mut offerings = Vec::new();
        let shortlist_codes = self.list_shortlist(term)?;
        let all_offerings = self.list_offerings(term)?;

        for code in &shortlist_codes {
            for o in all_offerings.iter().filter(|o| &o.code == code) {
                if !offerings.iter().any(|x: &CourseOffering| x.code == o.code && x.class_dept == o.class_dept) {
                    offerings.push(o.clone());
                }
            }
        }

        if let Some(dept) = dept_code {
            for o in &all_offerings {
                if o.dept_code == dept && o.category == "必修" && !offerings.iter().any(|x| x.code == o.code && x.class_dept == o.class_dept) {
                    offerings.push(o.clone());
                }
            }
        }

        Ok(offerings)
    }

    // ── Analyzed Grades ─────────────────────────────────────────

    pub fn upsert_analyzed_grades(&self, grades: &[crate::parsers::grade_html::CompletedCourse]) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "INSERT INTO analyzed_grades (name, credits, status, term, score, category, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(name, term) DO UPDATE SET
               credits=excluded.credits, status=excluded.status, score=excluded.score,
               category=excluded.category, imported_at=excluded.imported_at"
        )?;
        for g in grades {
            stmt.execute(rusqlite::params![
                &g.name, g.credits, &g.status, &g.term, g.score, &g.category, &now,
            ])?;
        }
        Ok(())
    }

    pub fn list_analyzed_grades(&self) -> anyhow::Result<Vec<crate::parsers::grade_html::CompletedCourse>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, credits, status, term, score, category FROM analyzed_grades ORDER BY term DESC, name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::parsers::grade_html::CompletedCourse {
                code: String::new(),
                name: row.get(0)?,
                credits: row.get(1)?,
                status: row.get(2)?,
                term: row.get(3)?,
                score: row.get(4)?,
                category: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_failed_grades(&self) -> anyhow::Result<Vec<crate::parsers::grade_html::CompletedCourse>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, credits, status, term, score, category FROM analyzed_grades
             WHERE status = '不及格' OR status = '停修'
             ORDER BY term DESC, name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::parsers::grade_html::CompletedCourse {
                code: String::new(),
                name: row.get(0)?,
                credits: row.get(1)?,
                status: row.get(2)?,
                term: row.get(3)?,
                score: row.get(4)?,
                category: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
