use crate::domain::conflict::{ConflictPair, ConflictReport};
use crate::domain::course_offering::CourseOffering;
use crate::parsers::time_slot::{expand_time_slots, TimeCell};

/// Deterministic conflict detection: expand time slots to (day, period) sets and check overlap.
pub fn detect_conflicts(offerings: &[CourseOffering]) -> ConflictReport {
    let mut pairs = Vec::new();

    // Pre-expand each offering's time slots into a set of (day, period) cells
    let expanded: Vec<Vec<TimeCell>> = offerings
        .iter()
        .map(|o| expand_time_slots(&o.time_slots))
        .collect();

    for i in 0..offerings.len() {
        for j in (i + 1)..offerings.len() {
            let overlapping_cells: Vec<TimeCell> = expanded[i]
                .iter()
                .filter(|c| expanded[j].contains(c))
                .cloned()
                .collect();
            if !overlapping_cells.is_empty() {
                // Build human-readable overlapping slot descriptions
                let overlapping_slots: Vec<String> = overlapping_cells
                    .iter()
                    .map(|cell| format!("{}-{}", cell.day, cell.period))
                    .collect();
                pairs.push(ConflictPair {
                    course_a: offerings[i].code.clone(),
                    course_b: offerings[j].code.clone(),
                    overlapping_slots,
                });
            }
        }
    }
    ConflictReport {
        conflict_count: pairs.len(),
        pairs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offering(code: &str, slots: &[&str]) -> CourseOffering {
        CourseOffering {
            code: code.to_string(),
            course_code: code.to_string(),
            assignment_key: format!("test:{code}"),
            name: format!("Course {}", code),
            name_en: String::new(),
            teacher: "Teacher".to_string(),
            teacher_id: String::new(),
            credits: 3,
            dept_code: "127".to_string(),
            dept_name: String::new(),
            class_dept: String::new(),
            class_dept_name: String::new(),
            admin_dept: String::new(),
            admin_dept_name: String::new(),
            time_slots: slots.iter().map(|s| s.to_string()).collect(),
            classroom: String::new(),
            category: "必修".to_string(),
            max_capacity: Some(60),
            enrolled: None,
            remaining: Some(30),
            div: "B".to_string(),
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
            semester_half: "全學期".to_string(),
            op_clock: 3.0,
            tch_clock: 3.0,
            op_type: "一般".to_string(),
            cos_usr: String::new(),
        }
    }

    #[test]
    fn no_conflict_different_times() {
        let offerings = vec![offering("A", &["1-1"]), offering("B", &["2-1"])];
        let report = detect_conflicts(&offerings);
        assert_eq!(report.conflict_count, 0);
    }

    #[test]
    fn detect_single_conflict_same_slot() {
        let offerings = vec![offering("A", &["2-A"]), offering("B", &["2-A", "3-1"])];
        let report = detect_conflicts(&offerings);
        assert_eq!(report.conflict_count, 1);
        assert_eq!(report.pairs[0].course_a, "A");
        assert_eq!(report.pairs[0].course_b, "B");
        // "2-A" expands to (2, 'A')
        assert!(report.pairs[0]
            .overlapping_slots
            .contains(&"2-A".to_string()));
    }

    #[test]
    fn detect_multi_period_overlap() {
        // "2-123" contains period 1; "2-1" also period 1 → conflict
        let offerings = vec![offering("A", &["2-123"]), offering("B", &["2-1"])];
        let report = detect_conflicts(&offerings);
        assert_eq!(report.conflict_count, 1);
    }

    #[test]
    fn detect_multi_period_overlap_abc() {
        // "2-ABC" overlaps "2-A" (both have 'A')
        let offerings = vec![offering("A", &["2-ABC"]), offering("B", &["2-A"])];
        let report = detect_conflicts(&offerings);
        assert_eq!(report.conflict_count, 1);
    }

    #[test]
    fn no_conflict_non_overlapping_periods() {
        // "2-123" = periods 1,2,3; "2-567" = periods 5,6,7 → no overlap
        let offerings = vec![offering("A", &["2-123"]), offering("B", &["2-567"])];
        let report = detect_conflicts(&offerings);
        assert_eq!(report.conflict_count, 0);
    }

    #[test]
    fn detect_multiple_conflicts() {
        let offerings = vec![
            offering("A", &["2-A", "3-1"]),
            offering("B", &["2-A", "4-1"]),
            offering("C", &["3-1", "5-1"]),
        ];
        let report = detect_conflicts(&offerings);
        assert_eq!(report.conflict_count, 2); // A-B (2-A), A-C (3-1)
    }

    #[test]
    fn empty_offerings_no_conflict() {
        let offerings: Vec<CourseOffering> = vec![];
        let report = detect_conflicts(&offerings);
        assert_eq!(report.conflict_count, 0);
    }
}
