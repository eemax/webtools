use crate::{error::AppError, search::SearchOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Search(SearchOptions),
    Fetch(FetchOptions),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchOptions {
    pub url: String,
    pub md: bool,
}

pub fn parse<I, S>(args: I) -> Result<Command, AppError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    if args.is_empty() {
        return Ok(Command::Help);
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "search" => parse_search(&args[1..]),
        "fetch" => parse_fetch(&args[1..]),
        command => Err(AppError::Usage(format!(
            "unknown command `{command}`\n\n{}",
            usage()
        ))),
    }
}

pub fn usage() -> &'static str {
    "Usage:
  webtools search [--count <n>] [--type <auto|neural|keyword>] <query...>
  webtools fetch [--json|--md] <url>

Defaults:
  Output is JSON unless --md is passed to fetch.

Environment:
  EXA_API_KEY is required for search."
}

fn parse_search(args: &[String]) -> Result<Command, AppError> {
    let mut count = 5usize;
    let mut search_type = "auto".to_string();
    let mut query = Vec::new();
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => index += 1,
            "--count" | "-n" => {
                index += 1;
                let raw = args
                    .get(index)
                    .ok_or_else(|| AppError::Usage("missing value for --count".to_string()))?;
                count = raw.parse::<usize>().map_err(|_| {
                    AppError::Usage("--count must be an integer between 1 and 20".to_string())
                })?;
                if !(1..=20).contains(&count) {
                    return Err(AppError::Usage(
                        "--count must be between 1 and 20".to_string(),
                    ));
                }
                index += 1;
            }
            "--type" => {
                index += 1;
                let raw = args
                    .get(index)
                    .ok_or_else(|| AppError::Usage("missing value for --type".to_string()))?;
                if !matches!(raw.as_str(), "auto" | "neural" | "keyword") {
                    return Err(AppError::Usage(
                        "--type must be one of auto, neural, keyword".to_string(),
                    ));
                }
                search_type = raw.to_string();
                index += 1;
            }
            value if value.starts_with('-') => {
                return Err(AppError::Usage(format!("unknown search flag `{value}`")));
            }
            value => {
                query.push(value.to_string());
                index += 1;
            }
        }
    }

    let query = query.join(" ");
    if query.trim().is_empty() {
        return Err(AppError::Usage("missing search query".to_string()));
    }

    Ok(Command::Search(SearchOptions {
        query,
        count,
        search_type,
    }))
}

fn parse_fetch(args: &[String]) -> Result<Command, AppError> {
    let mut md = false;
    let mut json = false;
    let mut url = None;

    for arg in args {
        match arg.as_str() {
            "--md" => md = true,
            "--json" => json = true,
            value if value.starts_with('-') => {
                return Err(AppError::Usage(format!("unknown fetch flag `{value}`")));
            }
            value => {
                if url.is_some() {
                    return Err(AppError::Usage("fetch accepts exactly one URL".to_string()));
                }
                url = Some(value.to_string());
            }
        }
    }

    if md && json {
        return Err(AppError::Usage(
            "choose only one output mode: --md or --json".to_string(),
        ));
    }

    let url = url.ok_or_else(|| AppError::Usage("missing fetch URL".to_string()))?;
    Ok(Command::Fetch(FetchOptions { url, md }))
}

#[cfg(test)]
mod tests {
    use super::{Command, parse};

    #[test]
    fn fetch_defaults_to_json() {
        let parsed = parse(["fetch", "https://example.com"]).expect("parse");
        assert_eq!(
            parsed,
            Command::Fetch(super::FetchOptions {
                url: "https://example.com".to_string(),
                md: false
            })
        );
    }

    #[test]
    fn fetch_accepts_md() {
        let parsed = parse(["fetch", "--md", "https://example.com"]).expect("parse");
        assert_eq!(
            parsed,
            Command::Fetch(super::FetchOptions {
                url: "https://example.com".to_string(),
                md: true
            })
        );
    }

    #[test]
    fn search_joins_query_terms() {
        let parsed = parse(["search", "--count", "3", "rust", "async"]).expect("parse");
        assert_eq!(
            parsed,
            Command::Search(crate::search::SearchOptions {
                query: "rust async".to_string(),
                count: 3,
                search_type: "auto".to_string()
            })
        );
    }
}
