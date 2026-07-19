use crate::domain::course_offering::CourseOffering;

#[derive(Default)]
pub struct FilterParams {
    pub code: Option<String>,           // 課程代碼
    pub keyword: Option<String>,        // 課程名稱(中/英)
    pub teacher: Option<String>,        // 授課教師
    pub teacher_id: Option<String>,     // 人事代碼
    pub dept: Option<String>,           // 系所代碼
    pub class_dept: Option<String>,     // 班級
    pub course_type: Option<String>,    // 必修/選修
    pub credit: Option<u32>,            // 學分
    pub div: Option<String>,            // 部別(B/M/D/H)
    pub language: Option<String>,       // 授課語言
    pub day: Option<u32>,               // 上課日(1-7)
    pub period: Option<String>,         // 上課時段
    pub classroom: Option<String>,      // 教室
    pub general: Option<String>,        // 通識向度
    pub emi: bool,                      // 全英語課程
    pub english: bool,                  // English授課
    pub distance: bool,                 // 遠距教學
    pub pbl: bool,                      // PBL課程
    pub programming: bool,              // 程式設計課程
    pub available_only: bool,           // 只顯示有餘額
    pub semester_half: Option<String>,  // 全學期/半學期
    pub cross: bool,                    // 跨系/聯盟
    pub sdgs: Option<String>,           // SDGs目標
}

pub fn apply_filters(offerings: &[CourseOffering], params: &FilterParams) -> Vec<CourseOffering> {
    offerings.iter().filter(|o| {
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
                && !o.name_en.to_lowercase().contains(&kw_lower) {
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
            if !o.class_dept.contains(cls.as_str()) && !o.class_dept_name.contains(cls.as_str()) {
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
        // 上課日+時段
        if let Some(day) = params.day {
            let day_prefix = format!("{}-", day);
            let has_day = o.time_slots.iter().any(|s| s.starts_with(&day_prefix));
            if !has_day {
                return false;
            }
            if let Some(ref period) = params.period {
                let target = format!("{}-{}", day, period);
                let has_slot = o.time_slots.iter().any(|s| s.contains(&target) || s == &target);
                if !has_slot {
                    return false;
                }
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
            if !o.spec.contains(gen.as_str()) && !o.course_type.contains(gen.as_str()) {
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
                Some(r) if r > 0 => {},
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
    }).cloned().collect()
}
