use anyhow::Context;

const BASE_URL: &str = "https://itouch.cycu.edu.tw/active_system/CourseQuerySystem/mvc/courseQuery.jsp";

pub struct CourseQueryConnector;

impl CourseQueryConnector {
    pub async fn query_offerings(
        session_cookie: &str,
        year_term: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let url = format!("{}?method=getCourseData", BASE_URL);

        let label = format!(
            "{}學年第{}學期",
            &year_term[..3],
            if &year_term[3..] == "1" { "1" } else { "2" }
        );

        let payload = serde_json::json!({
            "YEAR_TERM": {
                "label": label,
                "value": year_term,
                "isGeneralTime": "N",
                "elective_lock": "N",
                "ISSELCOURSETIME": "N"
            },
            "elective_lock": "N",
            "locale": "zh-TW",
            "ISSELCOURSETIME": "N"
        });

        let resp = client
            .post(&url)
            .header("Cookie", session_cookie)
            .header("Content-Type", "application/json")
            .header("Origin", "https://itouch.cycu.edu.tw")
            .header("Referer", "https://itouch.cycu.edu.tw/active_system/CourseQuerySystem/spa/")
            .json(&payload)
            .send()
            .await
            .context("Failed to query course offerings")?;

        let status = resp.status().as_u16();
        if status != 200 {
            anyhow::bail!("CourseQuery API returned status {}", status);
        }

        let body = resp.text().await?;
        let json: serde_json::Value = serde_json::from_str(&body)?;

        if json.get("done_YN").and_then(|v| v.as_str()) != Some("Y") {
            let msg = json.get("d_Message_C").and_then(|v| v.as_str()).unwrap_or("unknown error");
            anyhow::bail!("CourseQuery API error: {}", msg);
        }

        Ok(json)
    }
}

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn f64_or(v: &serde_json::Value, key: &str) -> f64 {
    v.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

fn i32_opt(v: &serde_json::Value, key: &str) -> Option<i32> {
    v.get(key).and_then(|v| v.as_f64()).map(|v| v as i32)
}

fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

pub fn parse_offerings(json: &serde_json::Value) -> anyhow::Result<Vec<crate::domain::course_offering::CourseOffering>> {
    let mut offerings = Vec::new();
    let items = json.get("datas").and_then(|v| v.as_array());
    let Some(items) = items else { return Ok(offerings) };

    for item in items {
        let code = s(item, "OP_CODE");
        if code.is_empty() { continue; }

        let name = s(item, "CURS_NM_C_S");
        let name_en = s(item, "CURS_NM_E_A");
        let teacher = s(item, "TEACHER_CNAME");
        let teacher_id = s(item, "IDCODE");
        let credits = f64_or(item, "OP_CREDIT") as u32;
        let category = s(item, "OP_STDY");
        let dept_code = s(item, "AUTHORITY_DEPT");
        let dept_name = s(item, "AUTHORITY_NAME");
        let class_dept = s(item, "DEPT_CODE");
        let class_dept_name = s(item, "DEPT_ABVI_C");
        let admin_dept = s(item, "DEPT_BLN_ADMIN");
        let admin_dept_name = s(item, "ADMIN_DEPT_NAME");

        let mut time_slots = Vec::new();
        for i in 1..=3 {
            let key = format!("OP_TIME_{}", i);
            if let Some(slot) = item.get(&key).and_then(|v| v.as_str()) {
                if !slot.trim().is_empty() {
                    time_slots.push(slot.to_string());
                }
            }
        }

        let classroom = s(item, "CLS_NAME_1");
        let max_capacity = i32_opt(item, "OP_MAN");
        let enrolled = i32_opt(item, "ACT_MAN");
        let remaining = i32_opt(item, "ACT_REMAIN");
        let div = s(item, "DIV");
        let course_type = s(item, "OP_TYPE");
        let language = s(item, "CURS_LANG");
        let all_english = s(item, "ALL_ENGLISH");
        let eng_course = s(item, "ENG_COURSE");
        let distance = s(item, "DISTANCE");
        let cross_pbl = s(item, "CROSS_PBL");
        let cross_prog = s(item, "CROSS_PROG");
        let sdgs = s(item, "SDGS");
        let spec = s(item, "SPEC");
        let cross_name = s(item, "CROSS_NAME");
        let memo = s(item, "MEMO1");
        let is_stop = contains(&s(item, "IS_STOP"), "停修") || contains(&s(item, "IS_STOP"), "不可");
        let auto_set = s(item, "AUTOSET") == "V";

        let op_clock = f64_or(item, "OP_T_COUNT");
        let tch_clock = f64_or(item, "TCH_T_COUNT");
        let op_type = s(item, "OP_TYPE");
        let cos_usr = s(item, "COS_USR");
        let semester_half = if tch_clock > 0.0 && tch_clock < op_clock {
            "半學期".to_string()
        } else {
            "全學期".to_string()
        };

        offerings.push(crate::domain::course_offering::CourseOffering {
            code, name, name_en, teacher, teacher_id, credits, category,
            dept_code, dept_name, class_dept, class_dept_name, admin_dept, admin_dept_name,
            time_slots, classroom,
            max_capacity, enrolled, remaining,
            div, course_type, language,
            is_emi: contains(&all_english, "全英語"),
            is_english: !eng_course.is_empty(),
            is_distance: contains(&distance, "遠距"),
            is_pbl: contains(&cross_pbl, "PBL"),
            is_programming: contains(&cross_prog, "程式設計"),
            sdgs, spec, cross_name, memo,
            is_stop, auto_set, semester_half,
            op_clock, tch_clock, op_type, cos_usr,
        });
    }

    Ok(offerings)
}
