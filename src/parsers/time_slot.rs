/// Parse CYCU time slot code into (day, start_period, end_period).
///
/// CYCU codes: "2-A" = day 2 (Tue), period A (1-2)
/// "4-1" = day 4 (Thu), period 1
/// "5-2" = day 5 (Fri), period 2
#[allow(dead_code)]
pub fn parse_time_slot(code: &str) -> Option<TimeSlot> {
    let parts: Vec<&str> = code.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let day: u32 = parts[0].parse().ok()?;
    if !(1..=7).contains(&day) {
        return None;
    }
    let period = parts[1];
    let (start, end) = match period {
        "A" => (1, 2),
        "B" => (3, 4),
        "C" => (1, 4),
        "D" => (1, 4),
        "E" => (1, 4),
        "F" => (5, 8),
        "G" => (5, 8),
        "1" => (1, 1),
        "2" => (2, 2),
        "3" => (3, 3),
        "4" => (4, 4),
        "5" => (5, 5),
        "6" => (6, 6),
        "7" => (7, 7),
        "8" => (8, 8),
        _ => return None,
    };
    Some(TimeSlot {
        day,
        start_period: start,
        end_period: end,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct TimeSlot {
    pub day: u32,
    pub start_period: u32,
    pub end_period: u32,
}

impl TimeSlot {
    #[allow(dead_code)]
    pub fn overlaps(&self, other: &TimeSlot) -> bool {
        self.day == other.day && self.start_period <= other.end_period && other.start_period <= self.end_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_period() {
        let ts = parse_time_slot("4-1").unwrap();
        assert_eq!(ts.day, 4);
        assert_eq!(ts.start_period, 1);
        assert_eq!(ts.end_period, 1);
    }

    #[test]
    fn parse_combo_period_a() {
        let ts = parse_time_slot("2-A").unwrap();
        assert_eq!(ts.day, 2);
        assert_eq!(ts.start_period, 1);
        assert_eq!(ts.end_period, 2);
    }

    #[test]
    fn parse_combo_period_b() {
        let ts = parse_time_slot("5-B").unwrap();
        assert_eq!(ts.day, 5);
        assert_eq!(ts.start_period, 3);
        assert_eq!(ts.end_period, 4);
    }

    #[test]
    fn overlap_same_day() {
        let a = TimeSlot { day: 2, start_period: 1, end_period: 2 };
        let b = TimeSlot { day: 2, start_period: 2, end_period: 3 };
        assert!(a.overlaps(&b));
    }

    #[test]
    fn no_overlap_different_day() {
        let a = TimeSlot { day: 1, start_period: 1, end_period: 2 };
        let b = TimeSlot { day: 3, start_period: 1, end_period: 2 };
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn no_overlap_same_day_gap() {
        let a = TimeSlot { day: 2, start_period: 1, end_period: 2 };
        let b = TimeSlot { day: 2, start_period: 5, end_period: 6 };
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn invalid_format() {
        assert!(parse_time_slot("invalid").is_none());
    }

    #[test]
    fn invalid_day_zero() {
        assert!(parse_time_slot("0-1").is_none());
    }

    #[test]
    fn invalid_day_eight() {
        assert!(parse_time_slot("8-1").is_none());
    }
}
