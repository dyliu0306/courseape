use crate::domain::course_offering::CourseOffering;
use crate::parsers::time_slot::expand_time_slots;
use crate::storage;

#[derive(Default)]
pub struct FilterParams {
    pub code: Option<String>,             // 課程代碼
    pub keyword: Option<String>,          // 課程名稱(中/英)
    pub teacher: Option<String>,          // 授課教師
    pub teacher_id: Option<String>,       // 人事代碼
    pub dept: Option<String>,             // 系所代碼
    pub class_dept: Option<String>,       // 班級
    pub course_type: Option<String>,      // 必修/選修
    pub credit: Option<u32>,              // 學分
    pub div: Option<String>,              // 部別(B/M/D/H)
    pub language: Option<String>,         // 授課語言
    pub day: Option<u32>,                 // 上課日(1-7)
    pub period: Option<String>,           // 上課時段
    pub classroom: Option<String>,        // 教室
    pub general: Option<String>,          // 通識向度
    pub emi: bool,                        // 全英語課程
    pub english: bool,                    // English授課
    pub distance: bool,                   // 遠距教學
    pub pbl: bool,                        // PBL課程
    pub programming: bool,                // 程式設計課程
    pub available_only: bool,             // 只顯示有餘額
    pub semester_half: Option<String>,    // 全學期/半學期
    pub cross: bool,                      // 跨系/聯盟
    pub sdgs: Option<String>,             // SDGs目標
    pub no_conflict_with: Option<String>, // 排除與 shortlist 衝突
}

pub fn apply_filters(offerings: &[CourseOffering], params: &FilterParams) -> Vec<CourseOffering> {
    let mut filtered: Vec<CourseOffering> = offerings
        .iter()
        .filter(|o| {
            // 課程代碼
            if let Some(ref code) = params.code {
                if !o.code.to_uppercase().contains(&code.to_uppercase()) {
                    return false;
                }
            }
            // 課程名稱(中/英)
            if let Some(ref kw) = params.keyword {
                let kw_lower = kw.to_lowercase();
                if !o.name.to_lowercase().contains(&kw_lower)
                    && !o.name_en.to_lowercase().contains(&kw_lower)
                {
                    return false;
                }
            }
            // 授課教師
            if let Some(ref t) = params.teacher {
                if !o.teacher.contains(t.as_str()) {
                    return false;
                }
            }
            // 人事代碼
            if let Some(ref tid) = params.teacher_id {
                if !o.teacher_id.contains(tid.as_str()) {
                    return false;
                }
            }
            // 系所
            if let Some(ref dept) = params.dept {
                if o.dept_code != *dept && o.admin_dept != *dept {
                    return false;
                }
            }
            // 班級
            if let Some(ref cls) = params.class_dept {
                if !o.class_dept.contains(cls.as_str()) && !o.class_dept_name.contains(cls.as_str())
                {
                    return false;
                }
            }
            // 必選修
            if let Some(ref cat) = params.course_type {
                if !o.category.contains(cat.as_str()) {
                    return false;
                }
            }
            // 學分
            if let Some(credit) = params.credit {
                if o.credits != credit {
                    return false;
                }
            }
            // 部別
            if let Some(ref div) = params.div {
                if o.div != *div {
                    return false;
                }
            }
            // 授課語言
            if let Some(ref lang) = params.language {
                if !o.language.contains(lang.as_str()) {
                    return false;
                }
            }
            // 上課日+時段。字母與數字節次都是獨立節次。
            if params.day.is_some() || params.period.is_some() {
                let cells = expand_time_slots(&o.time_slots);
                let period = params.period.as_deref().and_then(|value| {
                    let mut chars = value.chars();
                    let period = chars.next()?;
                    chars.next().is_none().then_some(period)
                });
                if params.period.is_some() && period.is_none() {
                    return false;
                }
                if !cells.iter().any(|cell| {
                    params.day.is_none_or(|day| cell.day == day)
                        && period.is_none_or(|period| cell.period == period)
                }) {
                    return false;
                }
            }
            // 教室
            if let Some(ref room) = params.classroom {
                if !o.classroom.contains(room.as_str()) {
                    return false;
                }
            }
            // 通識向度
            if let Some(ref gen) = params.general {
                if !o.spec.contains(gen.as_str())
                    && !o.course_type.contains(gen.as_str())
                    && !o.op_type.contains(gen.as_str())
                {
                    return false;
                }
            }
            // 全英語
            if params.emi && !o.is_emi {
                return false;
            }
            // English授課
            if params.english && !o.is_english {
                return false;
            }
            // 遠距
            if params.distance && !o.is_distance {
                return false;
            }
            // PBL
            if params.pbl && !o.is_pbl {
                return false;
            }
            // 程式設計
            if params.programming && !o.is_programming {
                return false;
            }
            // 有餘額
            if params.available_only {
                match o.remaining {
                    Some(r) if r > 0 => {}
                    _ => return false,
                }
            }
            // 期程
            if let Some(ref half) = params.semester_half {
                if o.semester_half != *half {
                    return false;
                }
            }
            // 跨系/聯盟
            if params.cross && o.cross_name.is_empty() {
                return false;
            }
            // SDGs
            if let Some(ref sdgs) = params.sdgs {
                if !o.sdgs.contains(sdgs.as_str()) && !o.spec.contains(sdgs.as_str()) {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect();

    // Conflict filtering: exclude courses that overlap with shortlisted courses
    if let Some(ref conflict_term) = params.no_conflict_with {
        if let Ok(db) = storage::db::open() {
            let repo = storage::repo::Repository::new(&db);
            if let Ok(planned) = repo.get_planned_courses(conflict_term) {
                use std::collections::HashSet;
                let planned_cells: HashSet<(u32, char)> = planned
                    .iter()
                    .flat_map(|o| expand_time_slots(&o.time_slots))
                    .map(|cell| (cell.day, cell.period))
                    .collect();
                if !planned_cells.is_empty() {
                    filtered.retain(|o| {
                        let cells = expand_time_slots(&o.time_slots);
                        !cells
                            .iter()
                            .any(|cell| planned_cells.contains(&(cell.day, cell.period)))
                    });
                }
            }
        }
    }

    filtered
}

pub fn apply_section_filters(
    offerings: &[CourseOffering],
    params: &FilterParams,
) -> Vec<CourseOffering> {
    let matching_codes: std::collections::HashSet<_> = apply_filters(offerings, params)
        .into_iter()
        .map(|offering| offering.code)
        .collect();
    offerings
        .iter()
        .filter(|offering| matching_codes.contains(&offering.code))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offering(slot: &str) -> CourseOffering {
        CourseOffering {
            code: slot.into(),
            course_code: slot.into(),
            assignment_key: slot.into(),
            name: slot.into(),
            name_en: String::new(),
            teacher: String::new(),
            teacher_id: String::new(),
            credits: 1,
            category: "選修".into(),
            dept_code: String::new(),
            dept_name: String::new(),
            class_dept: String::new(),
            class_dept_name: String::new(),
            admin_dept: String::new(),
            admin_dept_name: String::new(),
            time_slots: vec![slot.into()],
            classroom: String::new(),
            max_capacity: None,
            enrolled: None,
            remaining: None,
            div: String::new(),
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
            op_clock: 0.0,
            tch_clock: 0.0,
            op_type: String::new(),
            cos_usr: String::new(),
        }
    }

    #[test]
    fn period_filters_without_day_and_keeps_letters_independent() {
        let offerings = vec![offering("1-A"), offering("2-1"), offering("3-123")];
        let a = apply_filters(
            &offerings,
            &FilterParams {
                period: Some("A".into()),
                ..Default::default()
            },
        );
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].code, "1-A");

        let one = apply_filters(
            &offerings,
            &FilterParams {
                period: Some("1".into()),
                ..Default::default()
            },
        );
        assert_eq!(one.len(), 2);
    }

    #[test]
    fn section_filter_keeps_every_assignment_after_one_row_matches() {
        let json: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/offerings/multi_assignment_same_teacher.json"
        ))
        .unwrap();
        let offerings = crate::connectors::elective::parse_offerings(&json).unwrap();
        let filtered = apply_section_filters(
            &offerings,
            &FilterParams {
                period: Some("1".into()),
                ..Default::default()
            },
        );
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|o| o.time_slots == ["3-12"]));
        assert!(filtered.iter().any(|o| o.time_slots == ["3-34"]));
    }
}
