use serde::{Deserialize, Serialize};
use crate::domain::department::Department;

/// Confidence level for a department resolution match.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MatchConfidence {
    Exact,
    High,
    Medium,
    Low,
}

/// A candidate department match with confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeptCandidate {
    pub dept_code: String,
    pub name: String,
    pub confidence: MatchConfidence,
    pub reason: String,
}

/// Resolve a natural-language department query against the department list.
///
/// Matching priority:
/// 1. Exact code match (e.g. "5400B")
/// 2. Exact full name match (e.g. "資訊管理學系")
/// 3. Contains match on name (e.g. "資管" matches "資訊管理學系")
/// 4. Normalized match (strip spaces, parentheses)
pub fn resolve_department(query: &str, departments: &[Department]) -> Vec<DeptCandidate> {
    let query_trimmed = query.trim();
    if query_trimmed.is_empty() {
        return vec![];
    }

    let mut candidates: Vec<DeptCandidate> = Vec::new();

    for dept in departments {
        // 1. Exact code match
        if dept.dept_code.eq_ignore_ascii_case(query_trimmed) {
            candidates.push(DeptCandidate {
                dept_code: dept.dept_code.clone(),
                name: dept.name.clone(),
                confidence: MatchConfidence::Exact,
                reason: "代碼完全符合".to_string(),
            });
            continue;
        }

        // 2. Exact full name match
        if dept.name == query_trimmed {
            candidates.push(DeptCandidate {
                dept_code: dept.dept_code.clone(),
                name: dept.name.clone(),
                confidence: MatchConfidence::Exact,
                reason: "系所全名完全符合".to_string(),
            });
            continue;
        }

        // 3. Contains match
        if dept.name.contains(query_trimmed) || query_trimmed.contains(&dept.name) {
            candidates.push(DeptCandidate {
                dept_code: dept.dept_code.clone(),
                name: dept.name.clone(),
                confidence: MatchConfidence::High,
                reason: "系所名稱包含關鍵字".to_string(),
            });
            continue;
        }

        // 4. Normalized match (remove spaces, parentheses, common suffixes)
        let normalized_dept = normalize_dept_name(&dept.name);
        let normalized_query = normalize_dept_name(query_trimmed);
        if normalized_dept.contains(&normalized_query) || normalized_query.contains(&normalized_dept) {
            candidates.push(DeptCandidate {
                dept_code: dept.dept_code.clone(),
                name: dept.name.clone(),
                confidence: MatchConfidence::Medium,
                reason: "標準化名稱匹配".to_string(),
            });
        }
    }

    // Sort by confidence (exact > high > medium > low)
    candidates.sort_by_key(|a| confidence_order(&a.confidence));
    candidates
}

fn normalize_dept_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace() && *c != '（' && *c != '）' && *c != '(' && *c != ')')
        .collect::<String>()
        .replace("學系", "")
        .replace("學士班", "")
        .replace("碩士班", "")
        .replace("博士班", "")
}

fn confidence_order(c: &MatchConfidence) -> u8 {
    match c {
        MatchConfidence::Exact => 0,
        MatchConfidence::High => 1,
        MatchConfidence::Medium => 2,
        MatchConfidence::Low => 3,
    }
}

/// Derive enrollment year from student ID (first 3 digits = ROC year).
pub fn derive_enroll_year(student_id: &str) -> Option<u32> {
    if student_id.len() >= 3 {
        student_id[..3].parse::<u32>().ok()
    } else {
        None
    }
}

/// Determine current academic term from today's date.
/// Returns (year, semester) where year is ROC year.
pub fn current_term() -> (u32, u32) {
    use chrono::Datelike;
    let now = chrono::Utc::now();
    let ad_year = now.year();
    let month = now.month();

    match month {
        9..=12 => ((ad_year - 1911) as u32, 1),
        2..=6 => ((ad_year - 1912) as u32, 2),
        1 => ((ad_year - 1912) as u32, 1),
        _ => ((ad_year - 1911) as u32, 1), // Jul-Aug: upcoming term 1
    }
}

/// Return the term code for the next academic term.
pub fn next_term() -> String {
    let (year, sem) = current_term();
    if sem == 1 {
        format!("{}2", year)
    } else {
        format!("{}1", year + 1)
    }
}

/// Return the term code for the current academic term.
pub fn current_term_code() -> String {
    let (year, sem) = current_term();
    format!("{}{}", year, sem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dept(code: &str, name: &str) -> Department {
        Department { dept_code: code.to_string(), name: name.to_string(), year: 114 }
    }

    #[test]
    fn test_exact_code_match() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        let result = resolve_department("5400B", &depts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].confidence, MatchConfidence::Exact);
    }

    #[test]
    fn test_exact_name_match() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        let result = resolve_department("資訊管理學系", &depts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].confidence, MatchConfidence::Exact);
    }

    #[test]
    fn test_contains_match() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        let result = resolve_department("資訊管理", &depts);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].confidence, MatchConfidence::High | MatchConfidence::Medium));
    }

    #[test]
    fn test_normalized_match() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        let result = resolve_department("資訊管理", &depts);
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].confidence, MatchConfidence::High | MatchConfidence::Medium));
    }

    #[test]
    fn test_no_match() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        let result = resolve_department("化學系", &depts);
        assert!(result.is_empty());
    }

    #[test]
    fn test_derive_enroll_year() {
        assert_eq!(derive_enroll_year("11244001"), Some(112));
        assert_eq!(derive_enroll_year("11"), None);
    }
}
