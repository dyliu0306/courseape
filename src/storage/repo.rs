use crate::domain::course_offering::CourseOffering;
use crate::domain::department::Department;
use crate::domain::profile::StudentProfile;
use rusqlite::Connection;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleEntry {
    pub term: String,
    pub phase: String,
    pub category: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub description: String,
}

pub struct Repository<'a> {
    conn: &'a Connection,
}

/// (phase, category, start_time, end_time, description)
pub type SchedulePhaseTuple = (String, String, Option<String>, Option<String>, String);

impl<'a> Repository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    // ── Departments ──────────────────────────────────────────────

    pub fn upsert_departments(&self, departments: &[Department]) -> anyhow::Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO departments (dept_code, name, year) VALUES (?1, ?2, ?3)
             ON CONFLICT(dept_code, year) DO UPDATE SET name = excluded.name",
        )?;
        for d in departments {
            stmt.execute((&d.dept_code, &d.name, d.year))?;
        }
        Ok(())
    }

    pub fn list_departments(&self, year: u32) -> anyhow::Result<Vec<Department>> {
        let mut stmt = self.conn.prepare(
            "SELECT dept_code, name, year FROM departments WHERE year = ?1 ORDER BY dept_code",
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
        if offerings.is_empty() {
            anyhow::bail!("Refusing to replace term {term} with an empty API snapshot");
        }
        let now = chrono::Utc::now().to_rfc3339();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM offerings WHERE term = ?1", [term])?;
        let mut stmt = tx.prepare(
            "INSERT INTO offerings (code, term, course_code, assignment_key, name, name_en, teacher, teacher_id, credits, dept_code, dept_name,
             class_dept, class_dept_name, admin_dept, admin_dept_name, time_slots, classroom, category,
             max_capacity, enrolled, remaining, div, course_type, language, is_emi, is_english,
             is_distance, is_pbl, is_programming, sdgs, spec, cross_name, memo, is_stop, auto_set,
             semester_half, op_clock, tch_clock, op_type, cos_usr, synced_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,?41)
             ON CONFLICT(code, term, assignment_key) DO UPDATE SET
               course_code=excluded.course_code, name=excluded.name, name_en=excluded.name_en,
               teacher=excluded.teacher, teacher_id=excluded.teacher_id,
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
                &o.code,
                term,
                &o.course_code,
                &o.assignment_key,
                &o.name,
                &o.name_en,
                &o.teacher,
                &o.teacher_id,
                o.credits,
                &o.dept_code,
                &o.dept_name,
                &o.class_dept,
                &o.class_dept_name,
                &o.admin_dept,
                &o.admin_dept_name,
                &slots,
                &o.classroom,
                &o.category,
                o.max_capacity,
                o.enrolled,
                o.remaining,
                &o.div,
                &o.course_type,
                &o.language,
                o.is_emi as i32,
                o.is_english as i32,
                o.is_distance as i32,
                o.is_pbl as i32,
                o.is_programming as i32,
                &o.sdgs,
                &o.spec,
                &o.cross_name,
                &o.memo,
                o.is_stop as i32,
                o.auto_set as i32,
                &o.semester_half,
                o.op_clock,
                o.tch_clock,
                &o.op_type,
                &o.cos_usr,
                &now,
            ])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(())
    }

    pub fn list_offerings(&self, term: &str) -> anyhow::Result<Vec<CourseOffering>> {
        let mut stmt = self.conn.prepare(
            "SELECT code, course_code, assignment_key, name, name_en, teacher, teacher_id, credits, dept_code, dept_name,
             class_dept, class_dept_name, admin_dept, admin_dept_name, time_slots, classroom,
             category, max_capacity, enrolled, remaining, div, course_type, language,
             is_emi, is_english, is_distance, is_pbl, is_programming, sdgs, spec, cross_name,
             memo, is_stop, auto_set, semester_half, op_clock, tch_clock, op_type, cos_usr
             FROM offerings WHERE term = ?1 ORDER BY code"
        )?;
        let rows = stmt.query_map([term], |row| {
            let slots_str: String = row.get(14)?;
            Ok(CourseOffering {
                code: row.get(0)?,
                course_code: row.get(1)?,
                assignment_key: row.get(2)?,
                name: row.get(3)?,
                name_en: row.get(4)?,
                teacher: row.get(5)?,
                teacher_id: row.get(6)?,
                credits: row.get(7)?,
                dept_code: row.get(8)?,
                dept_name: row.get(9)?,
                class_dept: row.get(10)?,
                class_dept_name: row.get(11)?,
                admin_dept: row.get(12)?,
                admin_dept_name: row.get(13)?,
                time_slots: serde_json::from_str(&slots_str).unwrap_or_default(),
                classroom: row.get(15)?,
                category: row.get(16)?,
                max_capacity: row.get(17)?,
                enrolled: row.get(18)?,
                remaining: row.get(19)?,
                div: row.get(20)?,
                course_type: row.get(21)?,
                language: row.get(22)?,
                is_emi: row.get::<_, i32>(23)? != 0,
                is_english: row.get::<_, i32>(24)? != 0,
                is_distance: row.get::<_, i32>(25)? != 0,
                is_pbl: row.get::<_, i32>(26)? != 0,
                is_programming: row.get::<_, i32>(27)? != 0,
                sdgs: row.get(28)?,
                spec: row.get(29)?,
                cross_name: row.get(30)?,
                memo: row.get(31)?,
                is_stop: row.get::<_, i32>(32)? != 0,
                auto_set: row.get::<_, i32>(33)? != 0,
                semester_half: row.get(34)?,
                op_clock: row.get(35)?,
                tch_clock: row.get(36)?,
                op_type: row.get(37)?,
                cos_usr: row.get(38)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// List all distinct terms that have offerings in the DB.
    pub fn list_offering_terms(&self) -> anyhow::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT term FROM offerings ORDER BY term")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ── Requirements ────────────────────────────────────────────

    pub fn upsert_requirement(
        &self,
        year: u32,
        dept_code: &str,
        pdf_path: &str,
        schema_version: &str,
    ) -> anyhow::Result<()> {
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

    pub fn get_requirement_path(
        &self,
        year: u32,
        dept_code: &str,
    ) -> anyhow::Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT raw_pdf_path FROM requirements WHERE year = ?1 AND dept_code = ?2")?;
        let mut rows = stmt.query([year.to_string(), dept_code.to_string()])?;
        Ok(rows.next()?.map(|row| row.get(0)).transpose()?)
    }

    // ── Shortlist ──────────────────────────────────────────────

    pub fn add_to_shortlist(&self, code: &str, term: &str) -> anyhow::Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR IGNORE INTO shortlist (code, term, added_at) VALUES (?1, ?2, ?3)",
            (code, term, &now),
        )?;
        Ok(self.conn.changes() > 0)
    }

    pub fn remove_from_shortlist(&self, code: &str, term: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "DELETE FROM shortlist WHERE code = ?1 AND term = ?2",
            (code, term),
        )?;
        Ok(())
    }

    pub fn list_shortlist(&self, term: &str) -> anyhow::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT code FROM shortlist WHERE term = ?1 ORDER BY code")?;
        let rows = stmt.query_map([term], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn clear_shortlist(&self, term: &str) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM shortlist WHERE term = ?1", [term])?;
        Ok(())
    }

    // ── Schedule ───────────────────────────────────────────────

    pub fn upsert_schedule(&self, term: &str, phases: &[SchedulePhaseTuple]) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "INSERT INTO schedule (term, phase, category, start_time, end_time, description, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(term, phase, category) DO UPDATE SET
               start_time=excluded.start_time, end_time=excluded.end_time,
               description=excluded.description, synced_at=excluded.synced_at",
        )?;
        for (phase, category, start, end, desc) in phases {
            stmt.execute(rusqlite::params![
                term, phase, category, start, end, desc, &now,
            ])?;
        }
        Ok(())
    }

    pub fn list_schedule(&self, term: &str) -> anyhow::Result<Vec<ScheduleEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT term, phase, category, start_time, end_time, description
             FROM schedule WHERE term = ?1 ORDER BY start_time, phase, category",
        )?;
        let rows = stmt.query_map([term], |row| {
            Ok(ScheduleEntry {
                term: row.get(0)?,
                phase: row.get(1)?,
                category: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                description: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn next_schedule_phase(&self, term: &str) -> anyhow::Result<Option<ScheduleEntry>> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "SELECT term, phase, category, start_time, end_time, description
             FROM schedule WHERE term = ?1 AND end_time > ?2
             ORDER BY start_time LIMIT 1",
        )?;
        let mut rows = stmt.query_map([term, &now], |row| {
            Ok(ScheduleEntry {
                term: row.get(0)?,
                phase: row.get(1)?,
                category: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                description: row.get(5)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    pub fn get_planned_courses(&self, term: &str) -> anyhow::Result<Vec<CourseOffering>> {
        let mut offerings = Vec::new();
        let shortlist_codes = self.list_shortlist(term)?;
        let all_offerings = self.list_offerings(term)?;

        for code in &shortlist_codes {
            let matching: Vec<_> = all_offerings.iter().filter(|o| &o.code == code).collect();
            if let Some(merged) = merge_assignments(&matching) {
                offerings.push(merged);
            }
        }

        Ok(offerings)
    }

    // ── Analyzed Grades ─────────────────────────────────────────

    pub fn upsert_analyzed_grades(
        &self,
        grades: &[crate::parsers::grade_html::CompletedCourse],
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let tx = self.conn.unchecked_transaction()?;
        let mut stmt = tx.prepare(
            "INSERT INTO analyzed_grades (code, name, credits, status, term, score, category, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(code, name, term) DO UPDATE SET
               credits=excluded.credits, status=excluded.status, score=excluded.score,
               category=excluded.category, imported_at=excluded.imported_at"
        )?;
        for g in grades {
            stmt.execute(rusqlite::params![
                &g.code,
                &g.name,
                g.credits,
                &g.status,
                &g.term,
                g.score,
                &g.category,
                &now,
            ])?;
        }
        drop(stmt);
        tx.commit()?;
        Ok(())
    }

    pub fn list_analyzed_grades(
        &self,
    ) -> anyhow::Result<Vec<crate::parsers::grade_html::CompletedCourse>> {
        let mut stmt = self.conn.prepare(
            "SELECT code, name, credits, status, term, score, category FROM analyzed_grades ORDER BY term DESC, name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::parsers::grade_html::CompletedCourse {
                code: row.get(0)?,
                name: row.get(1)?,
                credits: row.get(2)?,
                status: row.get(3)?,
                term: row.get(4)?,
                score: row.get(5)?,
                category: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_failed_grades(
        &self,
    ) -> anyhow::Result<Vec<crate::parsers::grade_html::CompletedCourse>> {
        let mut stmt = self.conn.prepare(
            "SELECT code, name, credits, status, term, score, category FROM analyzed_grades
             WHERE status = '不及格' OR status = '停修'
             ORDER BY term DESC, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::parsers::grade_html::CompletedCourse {
                code: row.get(0)?,
                name: row.get(1)?,
                credits: row.get(2)?,
                status: row.get(3)?,
                term: row.get(4)?,
                score: row.get(5)?,
                category: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

pub fn merge_offering_rows(offerings: &[CourseOffering]) -> Vec<CourseOffering> {
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<&str, Vec<&CourseOffering>> = BTreeMap::new();
    for offering in offerings {
        grouped.entry(&offering.code).or_default().push(offering);
    }
    grouped
        .into_values()
        .filter_map(|rows| merge_assignments(&rows))
        .collect()
}

fn merge_assignments(assignments: &[&CourseOffering]) -> Option<CourseOffering> {
    let mut merged = (*assignments.first()?).clone();
    let mut teachers = vec![merged.teacher.clone()];
    let mut teacher_ids = vec![merged.teacher_id.clone()];
    let mut slots = merged.time_slots.clone();
    let mut classrooms = vec![merged.classroom.clone()];

    for assignment in assignments.iter().skip(1) {
        if !assignment.teacher.is_empty() && !teachers.contains(&assignment.teacher) {
            teachers.push(assignment.teacher.clone());
        }
        if !assignment.teacher_id.is_empty() && !teacher_ids.contains(&assignment.teacher_id) {
            teacher_ids.push(assignment.teacher_id.clone());
        }
        for slot in &assignment.time_slots {
            if !slots.contains(slot) {
                slots.push(slot.clone());
            }
        }
        if !assignment.classroom.is_empty() && !classrooms.contains(&assignment.classroom) {
            classrooms.push(assignment.classroom.clone());
        }
    }

    merged.teacher = teachers
        .into_iter()
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    merged.teacher_id = teacher_ids
        .into_iter()
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    merged.time_slots = slots;
    merged.classroom = classrooms
        .into_iter()
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    merged.assignment_key = "section".to_string();
    Some(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed_assignments() -> Vec<CourseOffering> {
        let json: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/offerings/multi_assignment_same_teacher.json"
        ))
        .unwrap();
        crate::connectors::elective::parse_offerings(&json).unwrap()
    }

    #[test]
    fn persists_all_teacher_assignments_and_replaces_term_snapshot() {
        let db = crate::storage::db::open_in_memory().unwrap();
        let repo = Repository::new(&db);
        let assignments = parsed_assignments();
        repo.upsert_offerings("1151", &assignments).unwrap();
        assert_eq!(repo.list_offerings("1151").unwrap().len(), 2);

        repo.upsert_offerings("1151", &assignments[..1]).unwrap();
        assert_eq!(repo.list_offerings("1151").unwrap().len(), 1);
    }

    #[test]
    fn planned_courses_merge_assignments_and_do_not_auto_add_required() {
        let db = crate::storage::db::open_in_memory().unwrap();
        let repo = Repository::new(&db);
        let assignments = parsed_assignments();
        repo.upsert_offerings("1151", &assignments).unwrap();
        assert!(repo.get_planned_courses("1151").unwrap().is_empty());

        repo.add_to_shortlist("CE154M", "1151").unwrap();
        let planned = repo.get_planned_courses("1151").unwrap();
        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].teacher, "林老師");
        assert!(planned[0].time_slots.contains(&"3-12".to_string()));
        assert!(planned[0].time_slots.contains(&"3-34".to_string()));
    }

    #[test]
    fn real_snapshot_preserves_every_assignment_when_available() {
        let Ok(path) = std::env::var("COURSEAPE_OFFERINGS_FIXTURE") else {
            return;
        };
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let assignments = crate::connectors::elective::parse_offerings(&json).unwrap();
        let raw_rows = json["datas"].as_array().unwrap().len();
        assert_eq!(assignments.len(), raw_rows);

        let db = crate::storage::db::open_in_memory().unwrap();
        let repo = Repository::new(&db);
        repo.upsert_offerings("1151", &assignments).unwrap();
        assert_eq!(repo.list_offerings("1151").unwrap().len(), raw_rows);
    }

    #[test]
    fn analyzed_grades_keep_same_name_with_different_codes() {
        let db = crate::storage::db::open_in_memory().unwrap();
        let repo = Repository::new(&db);
        let grade = |code: &str| crate::parsers::grade_html::CompletedCourse {
            code: code.to_string(),
            name: "專題".to_string(),
            credits: 1,
            status: "及格".to_string(),
            term: "1141".to_string(),
            score: Some(80),
            category: "選修".to_string(),
        };
        repo.upsert_analyzed_grades(&[grade("CS101"), grade("IM201")])
            .unwrap();
        assert_eq!(repo.list_analyzed_grades().unwrap().len(), 2);
    }
}
