use crate::domain::course_offering::CourseOffering;
use comfy_table::{Table, presets::UTF8_FULL, modifiers::UTF8_ROUND_CORNERS};

/// Format a list of CourseOffering as a comfy-table string.
pub fn offerings_table(offerings: &[CourseOffering]) -> String {
    if offerings.is_empty() {
        return "(no data)".to_string();
    }
    let mut table = Table::new();
    table.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Code", "Name", "Teacher", "Cr", "Dept", "Div",
            "Time Slots", "Cap", "Lang",
        ]);

    for o in offerings {
        let tags = {
            let mut t = Vec::new();
            if o.is_emi { t.push("EMI"); }
            if o.is_pbl { t.push("PBL"); }
            if o.is_distance { t.push("遠距"); }
            t.join(" ")
        };
        let name = if tags.is_empty() { o.name.clone() } else { format!("{} [{}]", o.name, tags) };
        table.add_row(vec![
            &o.code,
            &name,
            &o.teacher,
            &o.credits.to_string(),
            &o.dept_code,
            &o.div,
            &o.time_slots.join(", "),
            &format!("{}/{}", o.remaining.unwrap_or(-1), o.max_capacity.unwrap_or(-1)),
            &o.language,
        ]);
    }
    table.to_string()
}
