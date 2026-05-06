use webtools::{
    fetch,
    search::{self, SearchOptions},
};

#[test]
#[ignore = "hits the public internet"]
fn live_fetch_example_com() {
    let result = fetch::fetch("https://example.com").expect("fetch result");

    assert!(result.ok);
    assert_eq!(result.status, Some(200));
    assert!(result.content.contains("Example Domain"));
}

#[test]
#[ignore = "requires EXA_API_KEY and hits Exa"]
fn live_exa_search() {
    if std::env::var("EXA_API_KEY").is_err() {
        eprintln!("skipping live_exa_search because EXA_API_KEY is not set");
        return;
    }

    let result = search::search(&SearchOptions {
        query: "rust async runtime comparison".to_string(),
        count: 3,
        search_type: "auto".to_string(),
    })
    .expect("search result");

    assert!(result.ok);
    assert_eq!(result.provider, "exa");
    assert!(!result.results.is_empty());
}
