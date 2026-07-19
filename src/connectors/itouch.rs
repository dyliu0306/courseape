use anyhow::Context;
use crate::connectors::ConnectorResult;

const LOGIN_URL: &str = "https://itouch.cycu.edu.tw/active_system/login/login2.jsp?a=b";
const GRADE_URL: &str = "https://itouch.cycu.edu.tw/active_system/quary/s_grade.jsp";

pub struct ItouchConnector;

impl ItouchConnector {
    /// Login to iTouch and return (cookie_string, login_token).
    /// Follows redirects manually to capture loginToken from any response.
    pub async fn login(student_id: &str, password: &str) -> anyhow::Result<(String, Option<String>)> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let params = [
            ("UserNm", student_id),
            ("UserPasswd", password),
        ];

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
        let mut next_url = resp.headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| resolve_url(LOGIN_URL, s));

        let mut depth = 0;
        while let Some(ref url) = next_url {
            if depth >= 5 || url.is_empty() {
                break;
            }
            eprintln!("  Redirect[{}]: {}...", depth, &url[..url.len().min(60)]);

            let cookie_header = build_cookie_header(&all_cookies);

            let redirect_resp = client
                .get(url)
                .header("Cookie", &cookie_header)
                .send()
                .await;

            match redirect_resp {
                Ok(r) => {
                    collect_cookies_from_response(&r, &mut all_cookies);
                    next_url = r.headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| resolve_url(url, s));
                }
                Err(e) => {
                    eprintln!("    Redirect error: {}", e);
                    break;
                }
            }
            depth += 1;
        }

        // Extract loginToken
        let login_token = all_cookies.iter()
            .find(|(name, _)| name == "loginToken")
            .map(|(_, val)| val.replace("s%3A", "").replace("%3A", ":"))
            .filter(|s| !s.is_empty());

        // Build final cookie string (name=value pairs)
        let cookie = all_cookies
            .iter()
            .map(|(name, value)| format!("{}={}", name, value))
            .collect::<Vec<_>>()
            .join("; ");

        if cookie.is_empty() && !status.is_success() {
            anyhow::bail!(
                "Login failed with status {}. Check credentials.",
                status.as_u16()
            );
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
fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    // Extract scheme + host from base
    if let Some(scheme_end) = base.find("://") {
        let after_scheme = &base[scheme_end + 3..];
        if let Some(host_end) = after_scheme.find('/') {
            let host = &base[..scheme_end + 3 + host_end];
            if relative.starts_with('/') {
                return format!("{}{}", host, relative);
            } else {
                let base_path = &base[scheme_end + 3 + host_end..];
                let parent = if let Some(last_slash) = base_path.rfind('/') {
                    &base_path[..last_slash + 1]
                } else {
                    "/"
                };
                return format!("{}{}{}", host, parent, relative);
            }
        }
    }
    relative.to_string()
}
