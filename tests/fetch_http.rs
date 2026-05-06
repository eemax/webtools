use std::thread;

use tiny_http::{Header, Response, Server, StatusCode};
use webtools::fetch::{self, FetchConfig};

#[test]
fn fetches_html_from_local_server() {
    let url = spawn_once(
        200,
        "text/html",
        r#"<!doctype html>
        <html>
          <head><title>Local Article</title></head>
          <body><main><h1>Local Article</h1><p>Hello <a href="/next">there</a>.</p></main></body>
        </html>"#,
    );

    let result = fetch::fetch_with_config(&url, FetchConfig { allow_local: true }).expect("fetch");
    let json = serde_json::to_value(&result).expect("json");

    assert_eq!(json["ok"], true);
    assert_eq!(json["status"], 200);
    assert_eq!(json["kind"], "html");
    assert_eq!(json["title"], "Local Article");
    assert!(
        json["content"]
            .as_str()
            .unwrap()
            .contains("# Local Article")
    );
    assert!(
        json["content"]
            .as_str()
            .unwrap()
            .contains("[there](<http://127.0.0.1:")
    );
    assert_eq!(json["error"], serde_json::Value::Null);
}

#[test]
fn fetches_json_from_local_server() {
    let url = spawn_once(200, "application/json", r#"{"name":"webtools","ok":true}"#);

    let result = fetch::fetch_with_config(&url, FetchConfig { allow_local: true }).expect("fetch");
    let json = serde_json::to_value(&result).expect("json");

    assert_eq!(json["ok"], true);
    assert_eq!(json["kind"], "json");
    assert!(
        json["content"]
            .as_str()
            .unwrap()
            .contains("\"name\": \"webtools\"")
    );
}

#[test]
fn fetches_text_from_local_server() {
    let url = spawn_once(200, "text/plain", "hello from text\n");

    let result = fetch::fetch_with_config(&url, FetchConfig { allow_local: true }).expect("fetch");
    let json = serde_json::to_value(&result).expect("json");

    assert_eq!(json["ok"], true);
    assert_eq!(json["kind"], "text");
    assert_eq!(json["content"], "hello from text");
}

#[test]
fn http_status_failure_is_structured_json() {
    let url = spawn_once(404, "text/plain", "not found");

    let result = fetch::fetch_with_config(&url, FetchConfig { allow_local: true }).expect("fetch");
    let json = serde_json::to_value(&result).expect("json");

    assert_eq!(json["ok"], false);
    assert_eq!(json["status"], 404);
    assert_eq!(json["kind"], "error");
    assert_eq!(json["content"], "");
    assert_eq!(json["error"], "http_status");
}

fn spawn_once(status: u16, content_type: &'static str, body: &'static str) -> String {
    let server = Server::http("127.0.0.1:0").expect("server");
    let address = server.server_addr().to_ip().expect("ip address");
    thread::spawn(move || {
        let request = server.recv().expect("request");
        let response = Response::from_string(body)
            .with_status_code(StatusCode(status))
            .with_header(
                Header::from_bytes("Content-Type", content_type).expect("content type header"),
            );
        request.respond(response).expect("respond");
    });
    format!("http://{address}")
}
