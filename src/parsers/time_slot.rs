use std::collections::BTreeSet;

pub const PERIOD_ORDER: [char; 15] = [
    'A', '1', '2', '3', '4', 'B', '5', '6', '7', '8', 'C', 'D', 'E', 'F', 'G',
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeCell {
    pub day: u32,
    pub period: char,
}

/// Parse a CYCU slot such as `2-A`, `4-123`, or `6-CDE`.
/// Letter periods are independent periods, not aliases for numeric ranges.
pub fn parse_time_slot(code: &str) -> Option<Vec<TimeCell>> {
    let (day, periods) = code.split_once('-')?;
    let day: u32 = day.parse().ok()?;
    if !(1..=7).contains(&day) || periods.is_empty() {
        return None;
    }

    let mut cells = BTreeSet::new();
    for period in periods.chars() {
        if !PERIOD_ORDER.contains(&period) {
            return None;
        }
        cells.insert(TimeCell { day, period });
    }
    Some(cells.into_iter().collect())
}

pub fn expand_time_slots(slots: &[String]) -> Vec<TimeCell> {
    slots
        .iter()
        .filter_map(|slot| parse_time_slot(slot))
        .flatten()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_independent_periods() {
        assert_eq!(
            parse_time_slot("2-A").unwrap(),
            vec![TimeCell {
                day: 2,
                period: 'A'
            }]
        );
        assert_eq!(
            parse_time_slot("2-1").unwrap(),
            vec![TimeCell {
                day: 2,
                period: '1'
            }]
        );
        assert_ne!(parse_time_slot("2-A"), parse_time_slot("2-1"));
        assert_ne!(parse_time_slot("2-B"), parse_time_slot("2-3"));
    }

    #[test]
    fn parses_multi_period_slots() {
        assert_eq!(
            parse_time_slot("2-123").unwrap(),
            vec![
                TimeCell {
                    day: 2,
                    period: '1'
                },
                TimeCell {
                    day: 2,
                    period: '2'
                },
                TimeCell {
                    day: 2,
                    period: '3'
                },
            ]
        );
        assert_eq!(
            parse_time_slot("2-ABC").unwrap(),
            vec![
                TimeCell {
                    day: 2,
                    period: 'A'
                },
                TimeCell {
                    day: 2,
                    period: 'B'
                },
                TimeCell {
                    day: 2,
                    period: 'C'
                },
            ]
        );
    }

    #[test]
    fn rejects_invalid_codes() {
        for invalid in ["invalid", "0-1", "8-1", "2-X", "2-9", "2-"] {
            assert!(parse_time_slot(invalid).is_none(), "accepted {invalid}");
        }
    }

    #[test]
    fn period_order_matches_cycu_schedule() {
        assert_eq!(
            PERIOD_ORDER,
            ['A', '1', '2', '3', '4', 'B', '5', '6', '7', '8', 'C', 'D', 'E', 'F', 'G']
        );
    }
}
