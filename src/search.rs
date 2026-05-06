use std::{env, time::Duration};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

const EXA_URL: &str = "https://api.exa.ai/search";
const SEARCH_TIMEOUT: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOptions {
    pub query: String,
    pub count: usize,
    pub search_type: String,
}

#[derive(Debug, Serialize)]
pub struct SearchOutput {
    pub ok: bool,
    pub provider: &'static str,
    pub query: String,
    #[serde(rename = "type")]
    pub search_type: String,
    pub results: Vec<SearchResultOutput>,
}

#[derive(Debug, Serialize)]
pub struct SearchResultOutput {
    pub title: Option<String>,
    pub url: String,
    pub published_date: Option<String>,
    pub score: Option<f64>,
    pub highlights: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ExaRequest {
    query: String,
    #[serde(rename = "type")]
    search_type: String,
    #[serde(rename = "numResults")]
    num_results: usize,
    contents: ExaContents,
}

#[derive(Debug, Serialize)]
struct ExaContents {
    highlights: ExaHighlights,
}

#[derive(Debug, Serialize)]
struct ExaHighlights {}

#[derive(Debug, Deserialize)]
struct ExaResponse {
    #[serde(default)]
    results: Vec<ExaResult>,
    #[serde(rename = "searchType")]
    search_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExaResult {
    title: Option<String>,
    url: String,
    score: Option<f64>,
    #[serde(rename = "publishedDate")]
    published_date: Option<String>,
    highlights: Option<Vec<String>>,
}

pub fn search(options: &SearchOptions) -> Result<SearchOutput, AppError> {
    let api_key = env::var("EXA_API_KEY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Runtime("missing EXA_API_KEY".to_string()))?;
    let request = ExaRequest {
        query: options.query.clone(),
        search_type: options.search_type.clone(),
        num_results: options.count,
        contents: ExaContents {
            highlights: ExaHighlights {},
        },
    };

    let response = ureq::post(EXA_URL)
        .timeout(SEARCH_TIMEOUT)
        .set("x-api-key", &api_key)
        .set("accept", "application/json")
        .set("content-type", "application/json")
        .send_json(serde_json::to_value(request)?);

    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(code, response)) => {
            let body = response.into_string().unwrap_or_default();
            return Err(AppError::Runtime(format!(
                "exa returned HTTP {code}: {}",
                compact_error_body(&body)
            )));
        }
        Err(ureq::Error::Transport(error)) => {
            return Err(AppError::Runtime(format!("exa request failed: {error}")));
        }
    };

    let decoded = response
        .into_json::<ExaResponse>()
        .map_err(|error| AppError::Runtime(format!("failed to decode exa response: {error}")))?;
    Ok(SearchOutput {
        ok: true,
        provider: "exa",
        query: options.query.clone(),
        search_type: decoded
            .search_type
            .unwrap_or_else(|| options.search_type.clone()),
        results: decoded
            .results
            .into_iter()
            .map(|result| SearchResultOutput {
                title: clean_optional(result.title),
                url: result.url,
                published_date: clean_optional(result.published_date),
                score: result.score,
                highlights: result
                    .highlights
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|value| {
                        let cleaned = normalize_space(&value);
                        (!cleaned.is_empty()).then_some(cleaned)
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|entry| {
        let cleaned = normalize_space(&entry);
        (!cleaned.is_empty()).then_some(cleaned)
    })
}

fn normalize_space(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn compact_error_body(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return "(empty body)".to_string();
    }
    body.chars().take(500).collect()
}
