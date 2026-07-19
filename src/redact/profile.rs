pub fn mask_student_id(student_id: &str) -> String {
    let len = student_id.len();
    if len <= 4 {
        return "*".repeat(len);
    }
    format!("{}{}", "*".repeat(len - 4), &student_id[len - 4..])
}

#[allow(dead_code)]
pub fn mask_name(_name: &str) -> String {
    "[REDACTED]".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_student_id_preserves_last_four() {
        assert_eq!(mask_student_id("11244151"), "****4151");
    }

    #[test]
    fn mask_student_id_short() {
        assert_eq!(mask_student_id("123"), "***");
        assert_eq!(mask_student_id("1234"), "****");
    }

    #[test]
    fn mask_name_returns_redacted() {
        assert_eq!(mask_name("劉道元"), "[REDACTED]");
        assert_eq!(mask_name(""), "[REDACTED]");
    }
}
