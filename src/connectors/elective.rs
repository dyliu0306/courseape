use anyhow::Context;
use sha2::{Digest, Sha256};

const BASE_URL: &str =
    "https://itouch.cycu.edu.tw/active_system/CourseQuerySystem/mvc/courseQuery.jsp";

pub struct CourseQueryConnector;

impl CourseQueryConnector {
    pub async fn query_offerings(
        session_cookie: &str,
        year_term: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let label = term_label(year_term)?;
        let client = reqwest::Client::new();
        let url = format!("{}?method=getCourseData", BASE_URL);

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
            .header(
                "Referer",
                "https://itouch.cycu.edu.tw/active_system/CourseQuerySystem/spa/",
            )
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
            let msg = json
                .get("d_Message_C")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("CourseQuery API error: {}", msg);
        }

        Ok(json)
    }
}

fn term_label(year_term: &str) -> anyhow::Result<String> {
    let bytes = year_term.as_bytes();
    if bytes.len() != 4 || !bytes.iter().all(u8::is_ascii_digit) || !matches!(bytes[3], b'1' | b'2')
    {
        anyhow::bail!("Invalid term '{year_term}'. Expected YYY1 or YYY2");
    }
    Ok(format!("{}學年第{}學期", &year_term[..3], &year_term[3..]))
}

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
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

pub fn parse_offerings(
    json: &serde_json::Value,
) -> anyhow::Result<Vec<crate::domain::course_offering::CourseOffering>> {
    let mut offerings = Vec::new();
    let items = json
        .get("datas")
        .and_then(|v| v.as_array())
        .context("CourseQuery response is missing the datas array")?;

    for (row_index, item) in items.iter().enumerate() {
        let code = s(item, "OP_CODE");
        if code.is_empty() {
            anyhow::bail!("CourseQuery row {row_index} is missing OP_CODE");
        }

        let course_code = s(item, "CURS_CODE");
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

        let classroom = (1..=3)
            .map(|i| s(item, &format!("CLS_NAME_{}", i)))
            .filter(|room| !room.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        let max_capacity = i32_opt(item, "OP_MAN");
        let enrolled = i32_opt(item, "ACT_MAN");
        let remaining = match (i32_opt(item, "ACT_REMAIN"), enrolled) {
            (Some(0), None | Some(0)) => None,
            (value, _) => value,
        };
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
        let is_stop =
            contains(&s(item, "IS_STOP"), "停修") || contains(&s(item, "IS_STOP"), "不可");
        let auto_set = s(item, "AUTOSET") == "V";

        let op_clock = f64_or(item, "OP_T_COUNT");
        let tch_clock = f64_or(item, "TCH_T_COUNT");
        let op_type = s(item, "OP_TYPE");
        let cos_usr = s(item, "COS_USR");
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_vec(item)?);
        let assignment_key = format!("row:{row_index}:{}", hex::encode(hasher.finalize()));
        let semester_half = if tch_clock > 0.0 && tch_clock < op_clock {
            "半學期".to_string()
        } else {
            "全學期".to_string()
        };

        offerings.push(crate::domain::course_offering::CourseOffering {
            code,
            course_code,
            assignment_key,
            name,
            name_en,
            teacher,
            teacher_id,
            credits,
            category,
            dept_code,
            dept_name,
            class_dept,
            class_dept_name,
            admin_dept,
            admin_dept_name,
            time_slots,
            classroom,
            max_capacity,
            enrolled,
            remaining,
            div,
            course_type,
            language,
            is_emi: contains(&all_english, "全英語"),
            is_english: !eng_course.is_empty(),
            is_distance: contains(&distance, "遠距"),
            is_pbl: contains(&cross_pbl, "PBL"),
            is_programming: contains(&cross_prog, "程式設計"),
            sdgs,
            spec,
            cross_name,
            memo,
            is_stop,
            auto_set,
            semester_half,
            op_clock,
            tch_clock,
            op_type,
            cos_usr,
        });
    }

    Ok(offerings)
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn rejects_partial_rows_instead_of_returning_partial_snapshot() {
        let json = serde_json::json!({
            "datas": [
                {"OP_CODE": "CS101", "CURS_NM_C_S": "程式設計"},
                {"CURS_NM_C_S": "缺少代碼"}
            ]
        });
        assert!(parse_offerings(&json).is_err());
    }

    #[test]
    fn rejects_missing_datas_array() {
        assert!(parse_offerings(&serde_json::json!({"done_YN": "Y"})).is_err());
    }

    #[test]
    fn validates_term_before_slicing() {
        assert_eq!(term_label("1151").unwrap(), "115學年第1學期");
        assert!(term_label("").is_err());
        assert!(term_label("11").is_err());
        assert!(term_label("學期").is_err());
        assert!(term_label("1153").is_err());
    }
}
