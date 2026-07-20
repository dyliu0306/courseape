use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulePhase {
    pub phase: String,
    #[serde(default)]
    pub category: String,
    pub start: Option<String>,
    pub end: Option<String>,
    #[serde(default)]
    pub description: String,
}

/// Parse schedule phases from JSON array.
/// Each element: { "phase": "...", "category": "...", "start": "YYYY-MM-DD HH:MM", "end": "...", "description": "..." }
pub fn parse_schedule_json(json: &str) -> anyhow::Result<Vec<SchedulePhase>> {
    let phases: Vec<SchedulePhase> = serde_json::from_str(json)?;
    Ok(phases)
}

/// Generate a sample schedule template for the given term.
pub fn schedule_template(term: &str) -> Vec<SchedulePhase> {
    let year: &str = &term[..3];
    vec![
        SchedulePhase {
            phase: "第一階段登記篩選".into(),
            category: "全部".into(),
            start: Some(format!("{year}-07-22 09:00")),
            end: Some(format!("{year}-07-26 23:59")),
            description: "登記後依序篩選，第一篩選優先".into(),
        },
        SchedulePhase {
            phase: "詢問篩選結果".into(),
            category: "全部".into(),
            start: Some(format!("{year}-07-29 09:00")),
            end: Some(format!("{year}-08-03 08:59")),
            description: "確認篩選結果，退選未要的課程".into(),
        },
        SchedulePhase {
            phase: "第一階段選課".into(),
            category: "通識/體育/軍訓".into(),
            start: Some(format!("{year}-08-04 09:00")),
            end: Some(format!("{year}-08-06 16:00")),
            description: "每日16:00-16:30篩選".into(),
        },
        SchedulePhase {
            phase: "詢問&退選".into(),
            category: "全部".into(),
            start: Some(format!("{year}-08-07 09:00")),
            end: Some(format!("{year}-08-09 21:00")),
            description: "查詢篩選結果並退選".into(),
        },
        SchedulePhase {
            phase: "第二階段選課".into(),
            category: "全校課程".into(),
            start: Some(format!("{year}-08-10 22:00")),
            end: Some(format!("{year}-08-14 08:00")),
            description: "每日22:00至次日08:00".into(),
        },
        SchedulePhase {
            phase: "線上表單加課".into(),
            category: "學系/通識/體育".into(),
            start: Some(format!("{year}-08-19")),
            end: Some(format!("{year}-08-26")),
            description: "填表截止後統一處理".into(),
        },
        SchedulePhase {
            phase: "加退選".into(),
            category: "全部".into(),
            start: Some(format!("{year}-09-09 22:00")),
            end: Some(format!("{year}-09-21 08:00")),
            description: "開學後加退選期間".into(),
        },
        SchedulePhase {
            phase: "選課結束".into(),
            category: "全部".into(),
            start: Some(format!("{year}-09-21 08:00")),
            end: Some(format!("{year}-09-21 08:00")),
            description: "選課系統關閉".into(),
        },
    ]
}
