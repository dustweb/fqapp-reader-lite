//! HTTP boundary for the standalone read-only service.
#![cfg(feature = "service")]

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use common::{build_server, pool_json, MockReply, MockUpstream, TempDir};
use fqapi_core::service::serve;
use reqwest::header::{HeaderMap, ALLOW, CACHE_CONTROL, REFERRER_POLICY};
use reqwest::{Client, Method, StatusCode};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MOCK_JSON: &str = r#"{"code":0,"data":{}}"#;

struct Fixture {
    _dir: TempDir,
    mock: MockUpstream,
    addr: SocketAddr,
    task: tokio::task::JoinHandle<Result<(), String>>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn fixture(tag: &str) -> Fixture {
    let dir = TempDir::new(tag);
    let mock = MockUpstream::start(|_| MockReply {
        status: 200,
        headers: vec![("content-type".to_string(), "application/json".to_string())],
        body: MOCK_JSON.as_bytes().to_vec(),
    })
    .await;
    let server = build_server(
        &dir,
        Some(mock.origin.clone()),
        &[],
        &[("secret.txt", "DO_NOT_SERVE")],
        &pool_json(5),
    )
    .await;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind service");
    let addr = listener.local_addr().expect("service address");
    let task = tokio::spawn(serve(listener, server));
    Fixture {
        _dir: dir,
        mock,
        addr,
        task,
    }
}

fn client() -> Client {
    Client::builder().no_proxy().build().expect("http client")
}

async fn send(
    client: &Client,
    method: Method,
    addr: SocketAddr,
    path: &str,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let response = client
        .request(method, format!("http://{addr}{path}"))
        .send()
        .await
        .expect("service response");
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.expect("response body").to_vec();
    (status, headers, body)
}

async fn send_raw(addr: SocketAddr, request: &str) -> (StatusCode, Vec<u8>) {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("raw service connection");
    stream
        .write_all(request.as_bytes())
        .await
        .expect("raw request write");
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut response))
        .await
        .expect("raw response timeout")
        .expect("raw response read");
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .expect("response headers");
    let head = String::from_utf8_lossy(&response[..header_end]);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .expect("response status");
    (
        StatusCode::from_u16(status).expect("valid response status"),
        response[header_end..].to_vec(),
    )
}

fn raw_get(target: &str, host: &str, origin: Option<&str>) -> String {
    let mut request = format!(
        "GET {target} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nAccept: application/json\r\n"
    );
    if let Some(origin) = origin {
        request.push_str(&format!("Origin: {origin}\r\n"));
    }
    request.push_str("\r\n");
    request
}

fn assert_security_headers(headers: &HeaderMap) {
    assert_eq!(
        headers
            .get(CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    assert_eq!(
        headers
            .get(REFERRER_POLICY)
            .and_then(|value| value.to_str().ok()),
        Some("no-referrer")
    );
    assert_eq!(
        headers
            .get("x-content-type-options")
            .and_then(|value| value.to_str().ok()),
        Some("nosniff")
    );
}

#[tokio::test]
async fn health_and_allowed_read_shapes_return_json_unchanged() {
    let fixture = fixture("service-allowed").await;
    let client = client();

    let (status, headers, body) = send(&client, Method::GET, fixture.addr, "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_security_headers(&headers);
    let health: Value = serde_json::from_slice(&body).expect("health json");
    assert_eq!(health["devices"], 5);
    assert_eq!(fixture.mock.requests().len(), 0);

    for path in [
        "/api/v1/books/123",
        "/api/v1/books/123/detail",
        "/api/v1/books/123/directory",
        "/api/v1/books/123/directory/novel",
        "/api/v1/books/123/directory/fanqie",
        "/api/v1/novels/123",
    ] {
        let before = fixture.mock.requests().len();
        let (status, headers, body) = send(&client, Method::GET, fixture.addr, path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_security_headers(&headers);
        assert_eq!(body, MOCK_JSON.as_bytes(), "{path}");
        assert_eq!(fixture.mock.requests().len(), before + 1, "{path}");
    }

    let before = fixture.mock.requests().len();
    let (status, _, body) = send(
        &client,
        Method::GET,
        fixture.addr,
        "/api/v1/books/123/detail/",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, MOCK_JSON.as_bytes());
    assert_eq!(fixture.mock.requests().len(), before + 1);
}

#[tokio::test]
async fn forbidden_routes_are_404_before_dispatch_and_do_not_touch_upstream() {
    let fixture = fixture("service-forbidden").await;
    let client = client();
    let before = fixture.mock.requests().len();

    for (method, path) in [
        (Method::POST, "/api/v1/account/fanqie/history"),
        (Method::POST, "/api/v1/account/fanqie/library"),
        (Method::GET, "/api/v1/account/fanqie/history"),
        (Method::GET, "/api/v1/account/fanqie/library"),
        (Method::GET, "/api/v1/chapters/123"),
        (Method::GET, "/api/v1/chapters/123/"),
        (Method::GET, "/api/v1/chapters/123/summary"),
        (Method::GET, "/api/v1/chapters/123/timeline"),
        (Method::GET, "/api/v1/chapters/123/timepoints"),
        (Method::GET, "/src"),
        (Method::GET, "/src/secret.txt"),
        (Method::GET, "/src/"),
        (Method::GET, "/api/v1/shuqi/search"),
        (Method::GET, "/api/v1/shuqi/search/"),
        (Method::GET, "/api/v1/comments/123/digg"),
        (Method::GET, "/api/v1/books/123/reviews"),
        (Method::GET, "/api/v1/books/123/tones"),
        (Method::GET, "/api/v1/books/123/share"),
        (Method::GET, "/api/v1/books/123/video-match"),
        (Method::GET, "/api/v1/books/123/chapters/summary"),
        (Method::GET, "/api/v1/manga/123"),
        (Method::GET, "/api/v1/series/123"),
        (Method::GET, "/api/v1/videos/123"),
        (Method::GET, "/api/v1/audio/books/123"),
        (Method::GET, "/api/v1/audio/books/123/"),
        (Method::GET, "/api/v1/audio/books/123/chapters/456"),
        (Method::GET, "/api/v1/audio/play"),
        (Method::GET, "/api/v1/audio/wkcontent/123"),
        (Method::POST, "/api/v1/series/123/comments/add"),
        (Method::POST, "/api/v1/videos/456/danmaku/add"),
        (Method::GET, "/api/v1/search"),
        (Method::GET, "/api/v1/search/"),
        (Method::GET, "/api/v1/authors/123"),
        (Method::GET, "/api/v1/items/123"),
    ] {
        let (status, headers, body) = send(&client, method, fixture.addr, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert_security_headers(&headers);
        assert!(!String::from_utf8_lossy(&body).contains("DO_NOT_SERVE"));
    }
    assert_eq!(fixture.mock.requests().len(), before);

    for method in [
        Method::POST,
        Method::HEAD,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ] {
        for path in ["/api/v1/books/123", "/health"] {
            let before = fixture.mock.requests().len();
            let (status, headers, _) = send(&client, method.clone(), fixture.addr, path).await;
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
            assert_eq!(
                headers.get(ALLOW).and_then(|value| value.to_str().ok()),
                Some("GET")
            );
            assert_security_headers(&headers);
            assert_eq!(fixture.mock.requests().len(), before, "{method} {path}");
        }
    }
}

#[tokio::test]
async fn malformed_book_paths_are_404_and_health_is_exact() {
    let fixture = fixture("service-path-shape").await;
    let client = client();
    let before = fixture.mock.requests().len();

    for path in [
        "/health/",
        "/api/v1/books",
        "/api/v1/books/",
        "/api/v1/books//detail",
        "/api/v1/books/123/detail/extra",
        "/api/v1/books/123/directory/novel/extra",
        "/api/v1/books/123//",
        "/api/v1/books/12.3",
        "/api/v1/books/123%20",
    ] {
        let (status, _, _) = send(&client, Method::GET, fixture.addr, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
    assert_eq!(fixture.mock.requests().len(), before);
}

#[tokio::test]
async fn raw_encoded_paths_and_foreign_authorities_are_rejected() {
    let fixture = fixture("service-raw-boundary").await;
    let authority = fixture.addr.to_string();
    let before = fixture.mock.requests().len();

    for target in [
        "/api/v1/books/123%2Fchapters",
        "/api/v1/books/123%2f..%2fchapters",
        "/api/v1/books/123%5Cchapters",
        "/api/v1/books/123%252Fchapters",
        "/api/v1/books/123/detail%2F..%2F..%2Fchapters",
        "/api/v1/%62ooks/123",
        "/api/v1/chapters/123?api=books&book_id=123",
    ] {
        let (status, body) = send_raw(fixture.addr, &raw_get(target, &authority, None)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{target}");
        assert!(!String::from_utf8_lossy(&body).contains("DO_NOT_SERVE"));
    }

    let (status, _) = send_raw(
        fixture.addr,
        &raw_get("/api/v1/books/123", "evil.example", None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let wrong_port_authority = format!("127.0.0.1:{}", fixture.addr.port() + 1);
    let (status, _) = send_raw(
        fixture.addr,
        &raw_get("/api/v1/books/123", &wrong_port_authority, None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = send_raw(
        fixture.addr,
        &raw_get("/api/v1/books/123", &authority, Some("http://evil.example")),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(fixture.mock.requests().len(), before);

    let origin = format!("http://{authority}");
    let (status, _) = send_raw(fixture.addr, &raw_get("/health", &authority, Some(&origin))).await;
    assert_eq!(status, StatusCode::OK);
}
