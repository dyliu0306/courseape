use crate::connectors::ConnectorResult;
use anyhow::Context;
use url::Url;

const LOGIN_URL: &str = "https://itouch.cycu.edu.tw/active_system/login/login2.jsp?a=b";
const GRADE_URL: &str = "https://itouch.cycu.edu.tw/active_system/quary/s_grade.jsp";

pub struct ItouchConnector;

impl ItouchConnector {
    /// Login to iTouch and return (cookie_string, login_token).
    /// Follows redirects manually to capture loginToken from any response.
    pub async fn login(
        student_id: &str,
        password: &str,
    ) -> anyhow::Result<(String, Option<String>)> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let params = [("UserNm", student_id), ("UserPasswd", password)];

        // Step 1: POST to login endpoint
        let resp = client
            .post(LOGIN_URL)
            .form(&params)
            .send()
            .await
            .context("Failed to connect to iTouch login")?;

        let status = resp.status();
        let mut all_cookies: Vec<(String, String)> = Vec::new();

        // Collect cookies from initial response
        collect_cookies_from_response(&resp, &mut all_cookies);

        // Step 2: Follow redirects manually
        let mut next_url = resp
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| resolve_url(LOGIN_URL, s))
            .transpose()?;

        let mut depth = 0;
        while let Some(ref url) = next_url {
            if depth >= 5 || url.is_empty() {
                break;
            }
            eprintln!("  Redirect[{}]: approved iTouch path", depth);

            let cookie_header = build_cookie_header(&all_cookies);

            let redirect_resp = client
                .get(url)
                .header("Cookie", &cookie_header)
                .send()
                .await;

            match redirect_resp {
                Ok(r) => {
                    collect_cookies_from_response(&r, &mut all_cookies);
                    next_url = r
                        .headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| resolve_url(url, s))
                        .transpose()?;
                }
                Err(_) => {
                    eprintln!("    Redirect request failed");
                    break;
                }
            }
            depth += 1;
        }

        // Extract loginToken
        let login_token = all_cookies
            .iter()
            .find(|(name, _)| name == "loginToken")
            .map(|(_, val)| val.replace("s%3A", "").replace("%3A", ":"))
            .filter(|s| !s.is_empty());

        // Build final cookie string (name=value pairs)
        let cookie = all_cookies
            .iter()
            .map(|(name, value)| format!("{}={}", name, value))
            .collect::<Vec<_>>()
            .join("; ");

        if cookie.is_empty() {
            anyhow::bail!(
                "Login failed with status {}. Check credentials.",
                status.as_u16()
            );
        }

        if !Self::validate_session(&cookie).await? {
            anyhow::bail!("Login failed. iTouch did not accept the authenticated session.");
        }

        Ok((cookie, login_token))
    }

    /// Fetch grade HTML (requires session cookie).
    pub async fn fetch_grades(cookie: &str) -> anyhow::Result<ConnectorResult> {
        let client = reqwest::Client::new();
        let resp = client
            .get(GRADE_URL)
            .header("Cookie", cookie)
            .send()
            .await
            .context("Failed to fetch grades")?;

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

    pub async fn validate_session(cookie: &str) -> anyhow::Result<bool> {
        if cookie.trim().is_empty() {
            return Ok(false);
        }
        let result = Self::fetch_grades(cookie).await?;
        if result.status != 200 || result.body_bytes.is_empty() {
            return Ok(false);
        }
        Ok(is_authenticated_grade_body(&result.body_bytes))
    }
}

pub fn is_authenticated_grade_body(body: &[u8]) -> bool {
    if body.is_empty() {
        return false;
    }
    let body = String::from_utf8_lossy(body).to_lowercase();
    let rejected = body.contains("log in")
        || body.contains("login2.jsp")
        || body.contains("loginfail")
        || body.contains("name=\"usernm\"")
        || body.contains("name='usernm'")
        || body.contains("登入超時")
        || body.contains("重新登入");
    let table = body.contains("<table") || body.contains("<td");
    let grade_marker = [
        "歷年成績",
        "學年度",
        "學分",
        "及格",
        "不及格",
        "停修",
        "查無",
    ]
    .iter()
    .any(|marker| body.contains(marker));
    !rejected && table && grade_marker
}

/// Collect cookie name=value pairs from Set-Cookie headers.
fn collect_cookies_from_response(resp: &reqwest::Response, cookies: &mut Vec<(String, String)>) {
    for header_value in resp.headers().get_all("set-cookie").iter() {
        if let Ok(s) = header_value.to_str() {
            if let Some((name, value)) = parse_set_cookie(s) {
                // Update existing or add new
                if let Some(existing) = cookies.iter_mut().find(|(n, _)| n == &name) {
                    existing.1 = value;
                } else {
                    cookies.push((name, value));
                }
            }
        }
    }
}

/// Parse a Set-Cookie header: extract name=value from "name=value; attr; ..."
fn parse_set_cookie(set_cookie: &str) -> Option<(String, String)> {
    let name_value = set_cookie.split(';').next()?.trim();
    let eq_pos = name_value.find('=')?;
    let name = name_value[..eq_pos].trim().to_string();
    let value = name_value[eq_pos + 1..].trim().to_string();
    Some((name, value))
}

/// Build Cookie header from collected cookie pairs.
fn build_cookie_header(cookies: &[(String, String)]) -> String {
    cookies
        .iter()
        .map(|(name, value)| format!("{}={}", name, value))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Resolve a possibly-relative URL against a base URL.
fn resolve_url(base: &str, relative: &str) -> anyhow::Result<String> {
    let target = Url::parse(base)?.join(relative)?;
    if target.scheme() != "https" || target.host_str() != Some("itouch.cycu.edu.tw") {
        anyhow::bail!("Rejected login redirect outside approved iTouch host");
    }
    Ok(target.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirect_stays_on_itouch_https() {
        assert!(resolve_url(LOGIN_URL, "/active_system/index.jsp").is_ok());
        assert!(resolve_url(LOGIN_URL, "https://attacker.example/collect").is_err());
        assert!(resolve_url(LOGIN_URL, "http://itouch.cycu.edu.tw/unsafe").is_err());
        assert!(resolve_url(LOGIN_URL, "https://itouch.cycu.edu.tw.attacker.example/").is_err());
    }

    #[test]
    fn rejects_expired_grade_page() {
        let body = "<script>alert('登入超時！請重新登入。');window.location.href='/active_system/login/loginfailt.jsp';</script>";
        assert!(!is_authenticated_grade_body(body.as_bytes()));
        assert!(!is_authenticated_grade_body(
            "<html>系統維護中</html>".as_bytes()
        ));
        assert!(is_authenticated_grade_body(
            "<html><table><td>歷年成績</td></table></html>".as_bytes()
        ));
    }
}
