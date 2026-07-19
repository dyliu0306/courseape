use anyhow::Context;
use crate::connectors::ConnectorResult;

const BASE_URL: &str = "https://itouch.cycu.edu.tw/active_project/cycu2000h_03/necessaryCourse/mvc";

pub struct NecessaryCourseConnector;

impl NecessaryCourseConnector {
    /// Fetch department list for a given academic year.
    pub async fn query_departments(year: u32) -> anyhow::Result<ConnectorResult> {
        let client = reqwest::Client::new();
        let url = format!("{}/queryNecessary.jsp?method=query", BASE_URL);

        let params = serde_json::json!({
            "YEAR": year,
            "DEGREE_KIND": "學士",
            "PRACTICE_TYPE": 1,
        });

        let resp = client
            .post(&url)
            .json(&params)
            .send()
            .await
            .context("Failed to query departments")?;

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

    /// Build graduation requirement PDF download URL.
    pub fn requirement_pdf_url(year: u32, dept_code: &str) -> String {
        format!(
            "{}/export_PDF.jsp?method=downloadPDF&YEAR={}&DEPT_CODE={}&DEGREE_KIND=%E5%AD%B8%E5%A3%AB&PRACTICE_TYPE=1&lang=zh-TW&DOC_TYPE=-9",
            BASE_URL, year, dept_code
        )
    }

    /// Download graduation requirement PDF.
    pub async fn download_requirement_pdf(year: u32, dept_code: &str) -> anyhow::Result<ConnectorResult> {
        let url = Self::requirement_pdf_url(year, dept_code);
        let client = reqwest::Client::new();

        let resp = client
            .get(&url)
            .send()
            .await
            .context("Failed to download requirement PDF")?;

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
