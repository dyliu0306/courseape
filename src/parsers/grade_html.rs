use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedCourse {
    #[serde(default)]
    pub code: String,
    pub name: String,
    pub credits: u32,
    pub status: String, // 及格, 不及格, 停修
    pub term: String,
    pub score: Option<u32>,
    pub category: String, // 必修/選修
}

/// Parse grade HTML from iTouch s_grade.jsp into CompletedCourse list.
/// HTML is UTF-8 encoded.
///
/// Cell layout (10 cells per row):
/// 0: Term (1142)
/// 1: Department category (基礎通識GQ, 資管一甲)
/// 2: Sub-category (宗哲, 一年級)
/// 3: Course name (宗教哲學 / Philosophy of Religion)
/// 4: Compulsory/Selective (必修)
/// 5: Grade score (96)
/// 6: Credits (2)
/// 7: Note 1 - Status (及格/不及格/停修)
/// 8: Note 2
/// 9: Note 3
#[allow(dead_code)]
pub fn parse_grade_html(html: &str) -> Vec<CompletedCourse> {
    let mut courses = Vec::new();
    let lines: Vec<&str> = html.lines().collect();
    let mut current_term = String::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Detect term from semester header like "114學年度 (第2學期)"
        if line.contains("學年度") || line.contains("semester") {
            if let Some(term) = extract_term_from_header(line) {
                current_term = term;
            }
        }

        // Also detect from bare 4-digit term in <font> tags
        if current_term.is_empty() || line.contains("<font") {
            if let Some(term) = extract_term_from_font(line) {
                current_term = term;
            }
        }

        // Collect a full <tr>...</tr> block
        if line.contains("<tr") {
            let row_html = collect_row(&lines, i);
            if let Some(course) = parse_grade_row(&row_html, &current_term) {
                if !course.name.is_empty()
                    && !course.name.contains("Department")
                    && !course.name.contains("Transcript")
                {
                    courses.push(course);
                }
            }
        }

        i += 1;
    }

    courses
}

fn extract_term_from_header(line: &str) -> Option<String> {
    // "114學年度 (第2學期)" -> "1142"
    let text = strip_html_tags(line);
    if let Some(year_idx) = text.find("學年") {
        let year_part = text[..year_idx].trim();
        let year: String = year_part.chars().filter(|c| c.is_ascii_digit()).collect();
        if year.len() >= 3 {
            let year3 = &year[year.len() - 3..];
            if text.contains('1')
                && text.contains("學期")
                && text.find('1').unwrap() < text.find("學期").unwrap()
            {
                return Some(format!("{}1", year3));
            }
            if text.contains('2')
                && text.contains("學期")
                && text.find('2').unwrap() < text.find("學期").unwrap()
            {
                return Some(format!("{}2", year3));
            }
        }
    }
    None
}

fn extract_term_from_font(line: &str) -> Option<String> {
    let text = strip_html_tags(line).trim().to_string();
    if text.len() == 4 {
        let year: u32 = text[..3].parse().ok()?;
        let sem: u32 = text[3..].parse().ok()?;
        if (100..=200).contains(&year) && (sem == 1 || sem == 2) {
            return Some(text);
        }
    }
    None
}

fn collect_row(lines: &[&str], start: usize) -> String {
    let mut row = String::new();
    for line in &lines[start..lines.len().min(start + 30)] {
        row.push_str(line);
        row.push(' ');
        if line.contains("</tr>") || line.contains("</TR>") {
            break;
        }
    }
    row
}

fn parse_grade_row(row_html: &str, term: &str) -> Option<CompletedCourse> {
    let cells = extract_cells(row_html);

    // Need at least 8 cells for the grade data
    if cells.len() < 8 {
        return None;
    }

    // Skip header rows
    let first_cell = cells[0].trim();
    if first_cell.contains("學年")
        || first_cell.contains("Transcript")
        || first_cell.contains("Semester")
        || first_cell.contains("Total")
        || first_cell.contains("Department")
        || first_cell.contains("Category")
    {
        return None;
    }

    // Skip "No data" rows
    if row_html.contains("No data") || row_html.contains("查無") {
        return None;
    }

    // Cell 0: Term
    let row_term = cells[0].trim();
    let effective_term = if row_term.len() == 4 && row_term.chars().all(|c| c.is_ascii_digit()) {
        row_term.to_string()
    } else {
        term.to_string()
    };

    // Skip if no valid term
    if effective_term.is_empty() {
        return None;
    }

    // Cell 3: Course name (may contain English after newline/br)
    let course_name_raw = cells[3].trim();
    if course_name_raw.is_empty() || course_name_raw.len() < 2 {
        return None;
    }
    // Take only Chinese name (before English)
    let course_name = course_name_raw
        .lines()
        .next()
        .unwrap_or(course_name_raw)
        .trim()
        .to_string();

    // Cell 4: Category (必修/選修)
    let category_raw = cells[4].trim();
    let category = if category_raw.contains("必修") || category_raw.contains("Compulsory") {
        "必修".to_string()
    } else if category_raw.contains("選修") || category_raw.contains("Selective") {
        "選修".to_string()
    } else {
        category_raw.to_string()
    };

    // Cell 5: Score (may be empty for failed courses with red background)
    let score_text = cells[5].trim();
    let score: Option<u32> = score_text.parse().ok();

    // Check for red background (bgcolor="#FF9999") which indicates failed course
    let has_red_bg = row_html.contains("#FF9999") || row_html.contains("#ff9999");

    // Cell 6: Credits
    let credits_text = cells[6].trim();
    let credits: u32 = credits_text.parse().unwrap_or(0);

    // Cell 7: Status (Note 1)
    let status_text = normalize_status(cells[7].trim());
    let status = if status_text.contains("停修") || status_text.contains("Withdrawn") {
        "停修".to_string()
    } else if status_text.contains("不及格")
        || status_text.contains("Fail")
        || status_text.contains("當")
    {
        "不及格".to_string()
    } else if status_text.contains("及格")
        || status_text.contains("Pass")
        || status_text.contains("通過")
    {
        "及格".to_string()
    } else if let Some(s) = score {
        if s < 60 {
            "不及格".to_string()
        } else {
            "及格".to_string()
        }
    } else if has_red_bg && score.is_none() {
        // Red background with empty score = failed/withdrawn
        "停修".to_string()
    } else {
        "及格".to_string()
    };

    Some(CompletedCourse {
        code: String::new(),
        name: course_name,
        credits,
        status,
        term: effective_term,
        score,
        category,
    })
}

fn extract_cells(row_html: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut s = row_html;
    while let Some(start) = s.find("<td").or_else(|| s.find("<TD")) {
        let after_start = &s[start..];
        // Find the closing > of the <td> tag
        let tag_end = after_start.find('>').unwrap_or(0);
        let content_start = &after_start[tag_end + 1..];
        if let Some(end) = content_start
            .find("</td>")
            .or_else(|| content_start.find("</TD>"))
        {
            let cell_html = &content_start[..end];
            let cell_text = strip_html_tags(cell_html).trim().to_string();
            cells.push(cell_text);
            s = &content_start[end + 5..];
        } else {
            break;
        }
    }
    cells
}

fn strip_html_tags(s: &str) -> String {
    // Convert <br> / <br/> / <br /> to newlines before stripping tags
    let br_replaced = s
        .replace("<br/>", "\n")
        .replace("<BR/>", "\n")
        .replace("<br />", "\n")
        .replace("<BR />", "\n")
        .replace("<br>", "\n")
        .replace("<BR>", "\n");
    let mut result = String::new();
    let mut in_tag = false;
    for ch in br_replaced.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '\u{00A0}' => result.push(' '), // &nbsp; -> space
            '\u{FEFF}' => {}                // BOM
            '\u{200B}' => {}                // zero-width space
            '\u{200C}' => {}                // zero-width non-joiner
            '\u{200D}' => {}                // zero-width joiner
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect::<String>()
        .trim()
        .to_string()
}

/// Normalize status text by removing invisible characters and whitespace variations.
fn normalize_status(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && !c.is_control() && *c != '\u{00A0}' && *c != '\u{FEFF}')
        .collect::<String>()
}

/// Deduplicate retakes: for each course name, keep only the latest attempt.
/// If the latest attempt is failed but an earlier attempt passed, keep the passed one.
pub fn dedup_retakes(courses: Vec<CompletedCourse>) -> Vec<CompletedCourse> {
    use std::collections::BTreeMap;
    // Group by course name
    let mut by_name: BTreeMap<String, Vec<CompletedCourse>> = BTreeMap::new();
    for c in courses {
        by_name.entry(c.name.clone()).or_default().push(c);
    }
    let mut result = Vec::new();
    for (_name, mut entries) in by_name {
        if entries.len() == 1 {
            result.push(entries.into_iter().next().unwrap());
            continue;
        }
        // Sort by term descending (latest first)
        entries.sort_by(|a, b| b.term.cmp(&a.term));
        // If latest is passed/withdrawn, keep it
        if entries[0].status == "及格" || entries[0].status == "停修" {
            result.push(entries.into_iter().next().unwrap());
        } else {
            // Latest is failed — check if any earlier attempt passed
            let passed = entries.iter().find(|e| e.status == "及格");
            if let Some(p) = passed {
                result.push(p.clone());
            } else {
                // All failed — keep latest
                result.push(entries.into_iter().next().unwrap());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_tags() {
        assert_eq!(strip_html_tags("<b>hello</b>"), "hello");
        assert_eq!(strip_html_tags("<td class='x'>123</td>"), "123");
    }

    #[test]
    fn test_extract_term_from_font() {
        assert_eq!(
            extract_term_from_font("<font size=\"2\">1121</font>"),
            Some("1121".to_string())
        );
        assert_eq!(
            extract_term_from_font("<font size=\"2\">1132</font>"),
            Some("1132".to_string())
        );
    }

    #[test]
    fn test_parse_grade_row_valid() {
        let row = "<tr bgcolor=\"#33FFFF\">
            <td><font size=\"2\">1142</font></td>
            <td><font size=\"2\">基礎通識GQ</font></td>
            <td><font size=\"2\">宗哲</font></td>
            <td><font size=\"2\">宗教哲學<br/>Philosophy of Religion</font></td>
            <td><font size=\"2\">必修</font></td>
            <td><font size=\"2\">96</font></td>
            <td><font size=\"2\">2</font></td>
            <td><font size=\"2\">及格</font></td>
            <td><font size=\"2\"></font></td>
            <td><font size=\"2\"></font></td>
        </tr>";
        let course = parse_grade_row(row, "1142").unwrap();
        assert_eq!(course.name, "宗教哲學");
        assert_eq!(course.credits, 2);
        assert_eq!(course.score, Some(96));
        assert_eq!(course.status, "及格");
        assert_eq!(course.term, "1142");
    }

    #[test]
    fn test_parse_grade_row_failed() {
        let row = "<tr bgcolor=\"#33FFFF\">
            <td><font size=\"2\">1121</font></td>
            <td><font size=\"2\">資管一甲</font></td>
            <td><font size=\"2\">一年級</font></td>
            <td><font size=\"2\">英語聽講(一)<br/>English Listening</font></td>
            <td><font size=\"2\">必修</font></td>
            <td><font size=\"2\">58</font></td>
            <td><font size=\"2\">1</font></td>
            <td><font size=\"2\">不及格</font></td>
            <td><font size=\"2\"></font></td>
            <td><font size=\"2\"></font></td>
        </tr>";
        let course = parse_grade_row(row, "1121").unwrap();
        assert_eq!(course.name, "英語聽講(一)");
        assert_eq!(course.status, "不及格");
        assert_eq!(course.score, Some(58));
    }

    #[test]
    fn test_dedup_retakes_keeps_passed() {
        let courses = vec![
            CompletedCourse {
                code: "".into(),
                name: "微積分".into(),
                credits: 3,
                status: "不及格".into(),
                term: "1121".into(),
                score: Some(45),
                category: "必修".into(),
            },
            CompletedCourse {
                code: "".into(),
                name: "微積分".into(),
                credits: 3,
                status: "及格".into(),
                term: "1131".into(),
                score: Some(72),
                category: "必修".into(),
            },
        ];
        let result = dedup_retakes(courses);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].term, "1131");
        assert_eq!(result[0].status, "及格");
    }

    #[test]
    fn test_dedup_retakes_latest_failed_no_pass() {
        let courses = vec![
            CompletedCourse {
                code: "".into(),
                name: "物理".into(),
                credits: 3,
                status: "不及格".into(),
                term: "1121".into(),
                score: Some(50),
                category: "必修".into(),
            },
            CompletedCourse {
                code: "".into(),
                name: "物理".into(),
                credits: 3,
                status: "不及格".into(),
                term: "1131".into(),
                score: Some(30),
                category: "必修".into(),
            },
        ];
        let result = dedup_retakes(courses);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].term, "1131"); // latest
    }
}
