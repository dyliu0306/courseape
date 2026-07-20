use crate::domain::course_offering::CourseOffering;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};

/// Format a list of CourseOffering as a comfy-table string.
pub fn offerings_table(offerings: &[CourseOffering]) -> String {
    if offerings.is_empty() {
        return "(no data)".to_string();
    }
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Code",
            "Name",
            "Teacher",
            "Cr",
            "Dept",
            "Div",
            "Time Slots",
            "Cap",
            "Lang",
        ]);

    for o in offerings {
        let tags = {
            let mut t = Vec::new();
            if o.is_emi {
                t.push("EMI");
            }
            if o.is_pbl {
                t.push("PBL");
            }
            if o.is_distance {
                t.push("遠距");
            }
            t.join(" ")
        };
        let name = if tags.is_empty() {
            o.name.clone()
        } else {
            format!("{} [{}]", o.name, tags)
        };
        let remaining = match o.remaining {
            Some(r) if r >= 0 => r.to_string(),
            _ => "?".to_string(),
        };
        let max = match o.max_capacity {
            Some(m) if m >= 0 => m.to_string(),
            _ => "?".to_string(),
        };
        table.add_row(vec![
            &o.code,
            &name,
            &o.teacher,
            &o.credits.to_string(),
            &o.dept_code,
            &o.div,
            &o.time_slots.join(", "),
            &format!("{}/{}", remaining, max),
            &o.language,
        ]);
    }
    table.to_string()
}

pub fn offerings_csv(offerings: &[CourseOffering]) -> anyhow::Result<String> {
    let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
    writer.write_record([
        "code",
        "course_code",
        "name",
        "teacher",
        "credits",
        "category",
        "class_dept",
        "time_slots",
        "classroom",
        "remaining",
        "language",
    ])?;
    for offering in offerings {
        writer.write_record([
            offering.code.as_str(),
            offering.course_code.as_str(),
            offering.name.as_str(),
            offering.teacher.as_str(),
            &offering.credits.to_string(),
            offering.category.as_str(),
            offering.class_dept.as_str(),
            &offering.time_slots.join(";"),
            offering.classroom.as_str(),
            &offering
                .remaining
                .map(|value| value.to_string())
                .unwrap_or_default(),
            offering.language.as_str(),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

pub fn offerings_summary_json(offerings: &[CourseOffering]) -> serde_json::Value {
    use std::collections::BTreeMap;
    let mut sections: BTreeMap<&str, Vec<&CourseOffering>> = BTreeMap::new();
    for offering in offerings {
        sections.entry(&offering.code).or_default().push(offering);
    }

    serde_json::Value::Array(sections.into_values().map(|assignments| {
        let first = assignments[0];
        let unique = |values: Vec<String>| {
            let mut values = values.into_iter().filter(|value| !value.is_empty()).collect::<Vec<_>>();
            values.sort();
            values.dedup();
            values
        };
        serde_json::json!({
            "code": first.code,
            "course_code": first.course_code,
            "name": first.name,
            "teachers": unique(assignments.iter().map(|o| o.teacher.clone()).collect()),
            "credits": first.credits,
            "category": first.category,
            "class_dept": first.class_dept,
            "time_slots": unique(assignments.iter().flat_map(|o| o.time_slots.clone()).collect()),
            "classrooms": unique(assignments.iter().map(|o| o.classroom.clone()).collect()),
            "remaining": first.remaining,
            "language": first.language,
        })
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escapes_commas_and_quotes() {
        let offering = CourseOffering {
            code: "MI001A".into(),
            course_code: "MI001".into(),
            assignment_key: "teacher:1".into(),
            name: "課程,進階".into(),
            name_en: String::new(),
            teacher: "王\"老師".into(),
            teacher_id: "1".into(),
            credits: 3,
            category: "選修".into(),
            dept_code: "5400B".into(),
            dept_name: String::new(),
            class_dept: "5431B".into(),
            class_dept_name: String::new(),
            admin_dept: String::new(),
            admin_dept_name: String::new(),
            time_slots: vec!["2-12".into()],
            classroom: "教室,101".into(),
            max_capacity: Some(60),
            enrolled: None,
            remaining: None,
            div: "B".into(),
            course_type: String::new(),
            language: String::new(),
            is_emi: false,
            is_english: false,
            is_distance: false,
            is_pbl: false,
            is_programming: false,
            sdgs: String::new(),
            spec: String::new(),
            cross_name: String::new(),
            memo: String::new(),
            is_stop: false,
            auto_set: false,
            semester_half: "全學期".into(),
            op_clock: 3.0,
            tch_clock: 3.0,
            op_type: "一般".into(),
            cos_usr: String::new(),
        };
        let csv = offerings_csv(&[offering]).unwrap();
        assert!(csv.contains("\"課程,進階\""));
        assert!(csv.contains("\"王\"\"老師\""));
    }
}
