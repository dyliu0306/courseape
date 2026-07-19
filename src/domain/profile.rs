use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentProfile {
    pub student_id: String,
    pub dept_code: Option<String>,
    pub dept_name: Option<String>,
    pub enroll_year: Option<u32>,
    pub degree: Option<String>,
}
