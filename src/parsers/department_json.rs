use crate::domain::department::Department;

/// Parse queryNecessary JSON into Department list.
///
/// Expected format: `{ "DEPT_list": [{ "DEPT_CODE": "...", "SEPARATE_DEPT_CNAME": "...", ... }] }`
pub fn parse_departments(json: &serde_json::Value, year: u32) -> anyhow::Result<Vec<Department>> {
    let arr = json
        .get("DEPT_list")
        .and_then(|v| v.as_array())
        .or_else(|| json.as_array()) // fallback: if already an array
        .ok_or_else(|| anyhow::anyhow!("Expected JSON object with 'DEPT_list' array or a JSON array"))?;

    let mut departments = Vec::new();
    for item in arr {
        let dept_code = item
            .get("DEPT_CODE")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // Try both field names
        let name = item
            .get("SEPARATE_DEPT_CNAME")
            .or_else(|| item.get("DEPT_NAME"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !dept_code.is_empty() {
            departments.push(Department {
                dept_code,
                name,
                year,
            });
        }
    }
    Ok(departments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_from_dept_list_key() {
        let json = serde_json::json!({
            "DEPT_list": [
                {"DEPT_CODE": "127", "SEPARATE_DEPT_CNAME": "資訊管理學系"},
                {"DEPT_CODE": "001", "SEPARATE_DEPT_CNAME": "中國文學系"}
            ]
        });
        let depts = parse_departments(&json, 114).unwrap();
        assert_eq!(depts.len(), 2);
        assert_eq!(depts[0].dept_code, "127");
        assert_eq!(depts[0].name, "資訊管理學系");
        assert_eq!(depts[0].year, 114);
    }

    #[test]
    fn parse_fallback_array() {
        let json = serde_json::json!([
            {"DEPT_CODE": "127", "DEPT_NAME": "資訊管理學系"},
            {"DEPT_CODE": "001", "DEPT_NAME": "中國文學系"}
        ]);
        let depts = parse_departments(&json, 114).unwrap();
        assert_eq!(depts.len(), 2);
    }

    #[test]
    fn skip_empty_dept_code() {
        let json = serde_json::json!({
            "DEPT_list": [
                {"DEPT_CODE": "127", "SEPARATE_DEPT_CNAME": "資訊管理學系"},
                {"DEPT_CODE": "", "SEPARATE_DEPT_CNAME": "空系所"}
            ]
        });
        let depts = parse_departments(&json, 114).unwrap();
        assert_eq!(depts.len(), 1);
    }

    #[test]
    fn non_object_input_fails() {
        let json = serde_json::json!("not an object");
        assert!(parse_departments(&json, 114).is_err());
    }
}
