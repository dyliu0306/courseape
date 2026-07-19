use anyhow::Context;
use crate::connectors::ConnectorResult;

const BASE_URL: &str = "https://cmap.cycu.edu.tw:8443/Syllabus/syllabus/outPutCoursePreView.action";

pub struct CmapConnector;

impl CmapConnector {
    /// Build syllabus PDF URL.
    pub fn syllabus_url(year_term: &str, op_code: &str) -> String {
        format!(
            "{}?yearTerm={}&opCode={}&langId=zh_TW",
            BASE_URL, year_term, op_code
        )
    }

    /// Download course syllabus PDF.
    pub async fn download_syllabus(
        year_term: &str,
        op_code: &str,
    ) -> anyhow::Result<ConnectorResult> {
        let url = Self::syllabus_url(year_term, op_code);
        let client = reqwest::Client::new();

        let resp = client
            .get(&url)
            .send()
            .await
            .context("Failed to download syllabus PDF")?;

        let status = resp.status().as_u16();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.bytes().await?.to_vec();

        Ok(ConnectorResult {
            status,
            headers,
            body_bytes: body,
            elapsed_ms: 0,
        })
    }
}
