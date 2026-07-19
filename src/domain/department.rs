use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub dept_code: String,
    pub name: String,
    pub year: u32,
}
