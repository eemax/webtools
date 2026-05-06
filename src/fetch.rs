use std::{io::Read, net::IpAddr, time::Duration};

use serde::Serialize;
use url::{Host, Url};

use crate::markdown;

const FETCH_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_REDIRECTS: usize = 3;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/135.0.0.0 Safari/537.36 webtools/0.1"
);

#[derive(Debug, Serialize)]
pub struct FetchOutput {
    pub ok: bool,
    pub url: String,
    pub final_url: Option<String>,
    pub status: Option<u16>,
    pub content_type: Option<String>,
    pub title: Option<String>,
    pub kind: FetchKind,
    pub content: String,
    pub warnings: Vec<String>,
    pub truncated: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FetchKind {
    Html,
    Text,
    Json,
    Binary,
    Error,
}

pub fn fetch(raw_url: &str) -> Result<FetchOutput, crate::error::AppError> {
    Ok(fetch_inner(raw_url))
}

fn fetch_inner(raw_url: &str) -> FetchOutput {
    let url = match validate_url(raw_url) {
        Ok(url) => url,
        Err(error) => return failure(raw_url, None, None, None, error),
    };

    let agent = ureq::AgentBuilder::new()
        .timeout(FETCH_TIMEOUT)
        .redirects(MAX_REDIRECTS as u32)
        .build();
    let response = agent
        .get(url.as_str())
        .set("user-agent", USER_AGENT)
        .set(
            "accept",
            "text/html,application/xhtml+xml,text/plain,application/json,*/*;q=0.8",
        )
        .call();

    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(code, response)) => {
            let final_url = response.get_url().to_string();
            let content_type = response.content_type().to_string();
            return failure(
                raw_url,
                Some(final_url),
                Some(code),
                Some(content_type),
                "http_status",
            );
        }
        Err(ureq::Error::Transport(error)) => {
            return failure(raw_url, None, None, None, &format!("transport: {error}"));
        }
    };

    let final_url = response.get_url().to_string();
    if let Err(error) = validate_url(&final_url) {
        return failure(raw_url, Some(final_url), None, None, error);
    }

    let status = response.status();
    let content_type = clean_content_type(response.header("content-type"));
    let content_length = response
        .header("content-length")
        .and_then(|value| value.parse::<usize>().ok());
    if matches!(content_length, Some(length) if length > MAX_BYTES) {
        return failure(
            raw_url,
            Some(final_url),
            Some(status),
            content_type,
            "content_too_large",
        );
    }

    let mut reader = response.into_reader().take((MAX_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    if let Err(error) = reader.read_to_end(&mut bytes) {
        return failure(
            raw_url,
            Some(final_url),
            Some(status),
            None,
            &format!("read: {error}"),
        );
    }
    let truncated = bytes.len() > MAX_BYTES;
    if truncated {
        bytes.truncate(MAX_BYTES);
    }

    let body = String::from_utf8_lossy(&bytes).to_string();
    let kind = classify(&content_type, &body);
    match kind {
        FetchKind::Html => {
            let extracted = markdown::extract(&body, &final_url);
            let mut warnings = extracted.warnings;
            if truncated {
                warnings.push("content_truncated".to_string());
            }
            FetchOutput {
                ok: true,
                url: raw_url.to_string(),
                final_url: Some(final_url),
                status: Some(status),
                content_type,
                title: extracted.title,
                kind,
                content: extracted.content,
                warnings,
                truncated,
                error: None,
            }
        }
        FetchKind::Json => FetchOutput {
            ok: true,
            url: raw_url.to_string(),
            final_url: Some(final_url),
            status: Some(status),
            content_type,
            title: None,
            kind,
            content: pretty_json_or_raw(&body),
            warnings: truncation_warning(truncated),
            truncated,
            error: None,
        },
        FetchKind::Text => FetchOutput {
            ok: true,
            url: raw_url.to_string(),
            final_url: Some(final_url),
            status: Some(status),
            content_type,
            title: None,
            kind,
            content: normalize_text(&body),
            warnings: truncation_warning(truncated),
            truncated,
            error: None,
        },
        FetchKind::Binary | FetchKind::Error => FetchOutput {
            ok: false,
            url: raw_url.to_string(),
            final_url: Some(final_url),
            status: Some(status),
            content_type,
            title: None,
            kind: FetchKind::Binary,
            content: String::new(),
            warnings: truncation_warning(truncated),
            truncated,
            error: Some("binary_content".to_string()),
        },
    }
}

fn validate_url(raw_url: &str) -> Result<Url, &'static str> {
    let url = Url::parse(raw_url).map_err(|_| "invalid_url")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("unsupported_scheme");
    }
    let Some(host) = url.host() else {
        return Err("missing_host");
    };
    match host {
        Host::Domain(domain) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            if domain == "localhost"
                || domain.ends_with(".localhost")
                || domain == "metadata.google.internal"
            {
                return Err("blocked_host");
            }
        }
        Host::Ipv4(address) => {
            if is_blocked_ip(IpAddr::V4(address)) {
                return Err("blocked_host");
            }
        }
        Host::Ipv6(address) => {
            if is_blocked_ip(IpAddr::V6(address)) {
                return Err("blocked_host");
            }
        }
    }
    Ok(url)
}

fn is_blocked_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_multicast()
                || ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

fn clean_content_type(value: Option<&str>) -> Option<String> {
    value.and_then(|raw| {
        let cleaned = raw
            .split(';')
            .next()
            .unwrap_or(raw)
            .trim()
            .to_ascii_lowercase();
        (!cleaned.is_empty()).then_some(cleaned)
    })
}

fn classify(content_type: &Option<String>, body: &str) -> FetchKind {
    let lower = content_type.as_deref().unwrap_or("");
    if lower.contains("html") || looks_like_html(body) {
        FetchKind::Html
    } else if lower.contains("json") || looks_like_json(body) {
        FetchKind::Json
    } else if lower.starts_with("text/") || lower.is_empty() {
        FetchKind::Text
    } else {
        FetchKind::Binary
    }
}

fn looks_like_html(body: &str) -> bool {
    let head = body.trim_start().chars().take(200).collect::<String>();
    let lower = head.to_ascii_lowercase();
    lower.starts_with("<!doctype html") || lower.starts_with("<html") || lower.contains("<body")
}

fn looks_like_json(body: &str) -> bool {
    let trimmed = body.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn pretty_json_or_raw(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .and_then(|value| serde_json::to_string_pretty(&value))
        .unwrap_or_else(|_| normalize_text(body))
}

fn normalize_text(body: &str) -> String {
    body.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_string()
}

fn truncation_warning(truncated: bool) -> Vec<String> {
    if truncated {
        vec!["content_truncated".to_string()]
    } else {
        Vec::new()
    }
}

fn failure(
    raw_url: &str,
    final_url: Option<String>,
    status: Option<u16>,
    content_type: Option<String>,
    error: &str,
) -> FetchOutput {
    FetchOutput {
        ok: false,
        url: raw_url.to_string(),
        final_url,
        status,
        content_type,
        title: None,
        kind: FetchKind::Error,
        content: String::new(),
        warnings: Vec::new(),
        truncated: false,
        error: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_url;

    #[test]
    fn rejects_localhost() {
        assert!(validate_url("http://localhost:3000").is_err());
        assert!(validate_url("http://127.0.0.1").is_err());
    }

    #[test]
    fn accepts_public_https() {
        assert!(validate_url("https://example.com").is_ok());
    }
}
