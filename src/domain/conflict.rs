use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub conflict_count: usize,
    pub pairs: Vec<ConflictPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPair {
    pub course_a: String,
    pub course_b: String,
    pub overlapping_slots: Vec<String>,
}
