use scraper::{ElementRef, Html, Selector};
use url::Url;

#[derive(Debug)]
pub struct Extracted {
    pub title: Option<String>,
    pub content: String,
    pub warnings: Vec<String>,
}

pub fn extract(html: &str, final_url: &str) -> Extracted {
    let document = Html::parse_document(html);
    let base_url = Url::parse(final_url).ok();
    let title = first_text(&document, "title")
        .or_else(|| meta_content(&document, "meta[property='og:title']"))
        .or_else(|| meta_content(&document, "meta[name='twitter:title']"));
    let description = meta_content(&document, "meta[name='description']")
        .or_else(|| meta_content(&document, "meta[property='og:description']"));
    let root = best_root(&document);
    let mut content = String::new();

    if let Some(title) = &title {
        content.push_str("# ");
        content.push_str(title);
        content.push_str("\n\n");
    }
    if let Some(description) = description.filter(|value| title.as_deref() != Some(value.as_str()))
    {
        content.push_str(&description);
        content.push_str("\n\n");
    }

    if let Some(root) = root {
        let mut body = render_children(root, base_url.as_ref());
        if let Some(title) = &title {
            body = suppress_leading_duplicate_heading(&body, title);
        }
        content.push_str(&body);
    }

    let content = normalize_markdown(&content);
    let mut warnings = Vec::new();
    if content.chars().count() < 200 {
        warnings.push("low_content_yield".to_string());
    }

    Extracted {
        title,
        content,
        warnings,
    }
}

fn best_root(document: &Html) -> Option<ElementRef<'_>> {
    let selectors = [
        "article",
        "main",
        "[role='main']",
        ".markdown-body",
        ".entry-content",
        ".post-content",
        ".article-content",
        ".content",
        "#content",
        "body",
    ];
    let mut best = None;
    let mut best_score = f64::MIN;

    for raw_selector in selectors {
        let selector = Selector::parse(raw_selector).ok()?;
        for element in document.select(&selector) {
            if is_noisy_element(&element) {
                continue;
            }
            let score = score_element(&element);
            if score > best_score {
                best = Some(element);
                best_score = score;
            }
        }
    }

    best
}

fn score_element(element: &ElementRef<'_>) -> f64 {
    let text_len = visible_text(element).chars().count() as f64;
    if text_len == 0.0 {
        return 0.0;
    }
    let paragraph_count = count_descendants(element, "p") as f64;
    let heading_count = count_descendants(element, "h1,h2,h3,h4,h5,h6") as f64;
    let code_count = count_descendants(element, "pre,code") as f64;
    let link_text_len = descendant_text(element, "a").chars().count() as f64;
    let link_penalty = link_text_len / text_len;

    text_len + paragraph_count * 120.0 + heading_count * 40.0 + code_count * 60.0
        - link_penalty * text_len * 0.8
}

fn render_children(element: ElementRef<'_>, base_url: Option<&Url>) -> String {
    let mut out = String::new();
    for child in element.children() {
        if let Some(text) = child.value().as_text() {
            let text = clean_inline(text);
            if !text.is_empty() {
                out.push_str(&text);
                out.push_str("\n\n");
            }
            continue;
        }
        let Some(child_element) = ElementRef::wrap(child) else {
            continue;
        };
        out.push_str(&render_element(child_element, base_url));
    }
    out
}

fn render_element(element: ElementRef<'_>, base_url: Option<&Url>) -> String {
    if is_noisy_element(&element) {
        return String::new();
    }

    match element.value().name() {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let level = element.value().name()[1..]
                .parse::<usize>()
                .unwrap_or(2)
                .min(6);
            let text = render_inline(element, base_url);
            if text.is_empty() {
                String::new()
            } else {
                format!("{} {}\n\n", "#".repeat(level), text)
            }
        }
        "p" | "figcaption" => {
            let text = render_inline(element, base_url);
            if text.is_empty() {
                String::new()
            } else {
                format!("{text}\n\n")
            }
        }
        "br" => "\n".to_string(),
        "pre" => render_pre(element),
        "blockquote" => render_blockquote(element, base_url),
        "ul" => render_list(element, base_url, false),
        "ol" => render_list(element, base_url, true),
        "table" => render_table(element, base_url),
        "hr" => "---\n\n".to_string(),
        "script" | "style" | "svg" | "canvas" | "iframe" | "noscript" => String::new(),
        _ => render_children(element, base_url),
    }
}

fn render_inline(element: ElementRef<'_>, base_url: Option<&Url>) -> String {
    let mut out = String::new();
    for child in element.children() {
        if let Some(text) = child.value().as_text() {
            out.push_str(text);
            continue;
        }
        let Some(child_element) = ElementRef::wrap(child) else {
            continue;
        };
        if is_noisy_element(&child_element) {
            continue;
        }
        match child_element.value().name() {
            "br" => out.push('\n'),
            "code" => {
                let code = clean_inline(&child_element.text().collect::<String>());
                if !code.is_empty() {
                    out.push_str(&inline_code(&code));
                }
            }
            "a" => {
                let text = clean_inline(&render_inline(child_element, base_url));
                if text.is_empty() {
                    continue;
                }
                if let Some(href) = child_element
                    .value()
                    .attr("href")
                    .and_then(|href| absolutize(href, base_url))
                {
                    out.push_str(&format!("[{}](<{href}>)", escape_link_text(&text)));
                } else {
                    out.push_str(&text);
                }
            }
            "strong" | "b" => {
                let text = clean_inline(&render_inline(child_element, base_url));
                if !text.is_empty() {
                    out.push_str("**");
                    out.push_str(&text);
                    out.push_str("**");
                }
            }
            "em" | "i" => {
                let text = clean_inline(&render_inline(child_element, base_url));
                if !text.is_empty() {
                    out.push('*');
                    out.push_str(&text);
                    out.push('*');
                }
            }
            _ => out.push_str(&render_inline(child_element, base_url)),
        }
    }
    clean_inline(&out)
}

fn render_pre(element: ElementRef<'_>) -> String {
    let code = element.text().collect::<String>().replace("\r\n", "\n");
    let code = code.trim_matches('\n');
    if code.is_empty() {
        return String::new();
    }
    let fence = code_fence_for(code);
    format!("{fence}\n{code}\n{fence}\n\n")
}

fn render_blockquote(element: ElementRef<'_>, base_url: Option<&Url>) -> String {
    let body = normalize_markdown(&render_children(element, base_url));
    if body.is_empty() {
        return String::new();
    }
    body.lines()
        .map(|line| {
            if line.trim().is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n\n"
}

fn render_list(element: ElementRef<'_>, base_url: Option<&Url>, ordered: bool) -> String {
    let mut out = String::new();
    let mut index = 1usize;
    for child in element.children().filter_map(ElementRef::wrap) {
        if child.value().name() != "li" {
            continue;
        }
        let item = normalize_markdown(&render_children(child, base_url));
        if item.is_empty() {
            continue;
        }
        let marker = if ordered {
            let marker = format!("{index}.");
            index += 1;
            marker
        } else {
            "-".to_string()
        };
        for (line_index, line) in item.lines().enumerate() {
            if line_index == 0 {
                out.push_str(&format!("{marker} {line}\n"));
            } else if line.trim().is_empty() {
                out.push('\n');
            } else {
                out.push_str(&format!("  {line}\n"));
            }
        }
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn render_table(element: ElementRef<'_>, base_url: Option<&Url>) -> String {
    let row_selector = Selector::parse("tr").expect("valid selector");
    let cell_selector = Selector::parse("th,td").expect("valid selector");
    let mut rows = Vec::new();

    for row in element.select(&row_selector) {
        let cells = row
            .select(&cell_selector)
            .map(|cell| render_inline(cell, base_url))
            .filter(|cell| !cell.is_empty())
            .collect::<Vec<_>>();
        if !cells.is_empty() {
            rows.push(cells);
        }
    }

    if rows.is_empty() {
        return String::new();
    }
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    for row in &mut rows {
        row.resize(width, String::new());
    }

    let mut out = String::new();
    out.push('|');
    for cell in &rows[0] {
        out.push(' ');
        out.push_str(&cell.replace('|', "\\|"));
        out.push_str(" |");
    }
    out.push('\n');
    out.push('|');
    for _ in 0..width {
        out.push_str(" --- |");
    }
    out.push('\n');
    for row in rows.iter().skip(1) {
        out.push('|');
        for cell in row {
            out.push(' ');
            out.push_str(&cell.replace('|', "\\|"));
            out.push_str(" |");
        }
        out.push('\n');
    }
    out.push('\n');
    out
}

fn first_text(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    document
        .select(&selector)
        .next()
        .map(|element| clean_inline(&element.text().collect::<String>()))
        .filter(|value| !value.is_empty())
}

fn meta_content(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|element| element.value().attr("content"))
        .map(clean_inline)
        .filter(|value| !value.is_empty())
}

fn visible_text(element: &ElementRef<'_>) -> String {
    element
        .text()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn descendant_text(element: &ElementRef<'_>, selector: &str) -> String {
    let selector = Selector::parse(selector).expect("valid selector");
    element
        .select(&selector)
        .flat_map(|entry| entry.text())
        .collect::<Vec<_>>()
        .join(" ")
}

fn count_descendants(element: &ElementRef<'_>, selector: &str) -> usize {
    let selector = Selector::parse(selector).expect("valid selector");
    element.select(&selector).count()
}

fn is_noisy_element(element: &ElementRef<'_>) -> bool {
    let tag = element.value().name();
    if matches!(
        tag,
        "script"
            | "style"
            | "header"
            | "nav"
            | "footer"
            | "form"
            | "button"
            | "iframe"
            | "svg"
            | "canvas"
    ) {
        return true;
    }

    ["class", "id", "role", "aria-label"]
        .iter()
        .any(|name| element.value().attr(name).is_some_and(has_noisy_token))
}

fn has_noisy_token(value: &str) -> bool {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| {
            let token = token.to_ascii_lowercase();
            matches!(
                token.as_str(),
                "nav"
                    | "navbar"
                    | "menu"
                    | "footer"
                    | "header"
                    | "cookie"
                    | "consent"
                    | "modal"
                    | "popup"
                    | "share"
                    | "social"
                    | "breadcrumb"
                    | "advert"
                    | "ads"
                    | "promo"
                    | "newsletter"
                    | "subscribe"
                    | "login"
                    | "signup"
            )
        })
}

fn absolutize(href: &str, base_url: Option<&Url>) -> Option<String> {
    if href.starts_with('#') || href.starts_with("javascript:") || href.starts_with("mailto:") {
        return None;
    }
    if let Some(base_url) = base_url {
        return base_url.join(href).ok().map(|url| url.to_string());
    }
    Url::parse(href).ok().map(|url| url.to_string())
}

fn inline_code(code: &str) -> String {
    let fence = if code.contains('`') { "``" } else { "`" };
    format!("{fence}{code}{fence}")
}

fn code_fence_for(code: &str) -> String {
    let mut longest = 0usize;
    let mut current = 0usize;
    for ch in code.chars() {
        if ch == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    "`".repeat(longest.max(2) + 1)
}

fn escape_link_text(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn clean_inline(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_markdown(input: &str) -> String {
    let mut out = String::new();
    let mut blank_count = 0usize;
    for raw_line in input.replace("\r\n", "\n").replace('\r', "\n").lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                out.push('\n');
            }
        } else {
            blank_count = 0;
            out.push_str(line);
            out.push('\n');
        }
    }
    out.trim().to_string()
}

fn suppress_leading_duplicate_heading(body: &str, title: &str) -> String {
    let normalized_title = clean_inline(title);
    let mut lines = body.lines();
    let Some(first_line) = lines.next() else {
        return String::new();
    };
    let heading_text = first_line.trim_start_matches('#').trim();
    if first_line.starts_with('#') && clean_inline(heading_text) == normalized_title {
        return lines.collect::<Vec<_>>().join("\n");
    }
    body.to_string()
}

#[cfg(test)]
mod tests {
    use super::extract;

    #[test]
    fn extracts_article_markdown() {
        let html = r#"
            <html>
              <head><title>Hello</title><meta name="description" content="A tiny page"></head>
              <body>
                <nav>ignore me</nav>
                <article><h1>Hello</h1><p>Read <a href="/docs">the docs</a>.</p></article>
              </body>
            </html>
        "#;
        let extracted = extract(html, "https://example.com/post");
        assert!(extracted.content.contains("# Hello"));
        assert!(!extracted.content.contains("# Hello\n\n# Hello"));
        assert!(
            extracted
                .content
                .contains("[the docs](<https://example.com/docs>)")
        );
        assert!(!extracted.content.contains("ignore me"));
    }

    #[test]
    fn renders_code_fence() {
        let html = "<html><body><main><pre>fn main() {}</pre></main></body></html>";
        let extracted = extract(html, "https://example.com");
        assert!(extracted.content.contains("```"));
        assert!(extracted.content.contains("fn main() {}"));
    }
}
