pub mod cmap;
pub mod elective;
pub mod itouch;
pub mod necessary_course;

/// Raw connector result from any CYCU endpoint.
pub struct ConnectorResult {
    pub status: u16,
    #[allow(dead_code)]
    pub headers: Vec<(String, String)>,
    pub body_bytes: Vec<u8>,
    #[allow(dead_code)]
    pub elapsed_ms: u64,
}
