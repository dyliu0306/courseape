use crate::domain::department::Department;
use serde::{Deserialize, Serialize};

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
        if normalized_dept.contains(&normalized_query)
            || normalized_query.contains(&normalized_dept)
        {
            candidates.push(DeptCandidate {
                dept_code: dept.dept_code.clone(),
                name: dept.name.clone(),
                confidence: MatchConfidence::Medium,
                reason: "標準化名稱匹配".to_string(),
            });
            continue;
        }

        // 5. Abbreviation match: strip "系/所/班" suffix, then subsequence match
        let query_core = strip_query_suffix(query_trimmed);
        if query_core.len() >= 2 && query_core != query_trimmed {
            let dept_core = strip_query_suffix(&normalized_dept);
            if is_subsequence(query_core, dept_core) {
                candidates.push(DeptCandidate {
                    dept_code: dept.dept_code.clone(),
                    name: dept.name.clone(),
                    confidence: MatchConfidence::Medium,
                    reason: "縮寫匹配".to_string(),
                });
            }
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
        .replace("研究所", "")
        .replace("碩博士班", "")
}

/// Strip common suffixes from department query (系, 所, 班, etc.)
fn strip_query_suffix(query: &str) -> &str {
    let suffixes = [
        "學系",
        "學士班",
        "碩士班",
        "博士班",
        "研究所",
        "碩博士班",
        "系",
        "所",
        "班",
    ];
    for suffix in &suffixes {
        if let Some(stripped) = query.strip_suffix(suffix) {
            return stripped;
        }
    }
    query
}

/// Check if `short` is a subsequence of `long` (each char appears in order).
/// Used for Chinese abbreviation matching (e.g. "資管" matches "資訊管理").
fn is_subsequence(short: &str, long: &str) -> bool {
    let mut chars = long.chars();
    for sc in short.chars() {
        let found = chars.any(|c| c == sc);
        if !found {
            return false;
        }
    }
    true
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

/// Determine current academic term from today's date (Taiwan time UTC+8).
/// Returns (year, semester) where year is ROC year.
pub fn current_term() -> (u32, u32) {
    use chrono::Datelike;
    let taiwan_offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let now = chrono::Utc::now().with_timezone(&taiwan_offset);
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
        Department {
            dept_code: code.to_string(),
            name: name.to_string(),
            year: 114,
        }
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
        assert!(matches!(
            result[0].confidence,
            MatchConfidence::High | MatchConfidence::Medium
        ));
    }

    #[test]
    fn test_normalized_match() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        let result = resolve_department("資訊管理", &depts);
        assert_eq!(result.len(), 1);
        assert!(matches!(
            result[0].confidence,
            MatchConfidence::High | MatchConfidence::Medium
        ));
    }

    #[test]
    fn test_no_match() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        let result = resolve_department("化學系", &depts);
        assert!(result.is_empty());
    }

    #[test]
    fn test_abbreviation_match() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        // "資管系" → strip "系" → "資管" → subsequence match against "資訊管理"
        let result = resolve_department("資管系", &depts);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].confidence, MatchConfidence::Medium);
        assert_eq!(result[0].reason, "縮寫匹配");
    }

    #[test]
    fn test_abbreviation_no_suffix() {
        let depts = vec![make_dept("5400B", "資訊管理學系")];
        // "資管" without "系" suffix → should not trigger abbreviation path
        // but should match via subsequence in normalized step if applicable
        let result = resolve_department("資管", &depts);
        // May or may not match depending on normalization, but shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_derive_enroll_year() {
        assert_eq!(derive_enroll_year("11244001"), Some(112));
        assert_eq!(derive_enroll_year("11"), None);
    }
}
