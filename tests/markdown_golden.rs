use webtools::markdown;

#[test]
fn article_fixture_matches_expected_markdown() {
    let html = include_str!("fixtures/article.html");
    let expected = include_str!("fixtures/article.md").trim();

    let extracted = markdown::extract(html, "https://example.com/post");

    assert_eq!(extracted.content, expected);
    assert_eq!(extracted.title.as_deref(), Some("Fast Tools"));
}

#[test]
fn table_fixture_matches_expected_markdown() {
    let html = include_str!("fixtures/table.html");
    let expected = include_str!("fixtures/table.md").trim();

    let extracted = markdown::extract(html, "https://example.com/api");

    assert_eq!(extracted.content, expected);
    assert_eq!(extracted.title.as_deref(), Some("API Fields"));
}
