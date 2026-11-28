//! Boot path for the standalone service from files outside the repository.
#![cfg(feature = "service")]

mod common;

use std::path::PathBuf;

use base64::Engine as _;
use common::{pool_json, MockReply, MockUpstream, TempDir};
use fqapi_core::device::pool_crypto;
use fqapi_core::dispatch::{dispatch, Request, ResponseBody};
use fqapi_core::service::{build_server, serve, ServiceOptions};

const MOCK_JSON: &str = r#"{"code":0,"data":{}}"#;
const POOL_KEY: [u8; 32] = [0x42; 32];

#[tokio::test]
async fn runtime_loader_decrypts_an_external_pool_without_a_mock_origin() {
    let dir = TempDir::new("service-runtime-boot");
    let config_path = dir.write(
        "config.json",
        r#"{"algorithm_type":"service-runtime-config","port":0,"anti_crawler":{"enabled":false,"redirect_url":""}}"#,
    );
    let encoded_key = base64::engine::general_purpose::STANDARD.encode(POOL_KEY);
    let pool_key_path = dir.write("pool.key", &format!("{encoded_key}\n"));
    pool_crypto::provision_pool_key(POOL_KEY.to_vec()).expect("pool key");
    let encrypted =
        pool_crypto::encrypt_pool_file(pool_json(5).as_bytes()).expect("encrypted pool");
    let pool_path = dir.join("device_pool.enc");
    std::fs::write(&pool_path, encrypted).expect("write encrypted pool");
    let options = ServiceOptions {
        config_path,
        pool_path,
        filter_path: PathBuf::new(),
        pool_key_path,
        listen: "127.0.0.1:0".parse().expect("test listen address"),
        mock_upstream: None,
    };

    let server = build_server(&options).await.expect("runtime service build");
    assert_eq!(server.ctx.cfg.algorithm_type, "service-runtime-config");
    assert_eq!(server.ctx.pool.count().await, 5);
    assert_eq!(pool_crypto::pool_key(), Some(POOL_KEY));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0")
        .await
        .expect("non-loopback listener");
    let error = serve(listener, server)
        .await
        .expect_err("non-loopback listener must be refused");
    assert!(error.contains("loopback"), "{error}");
}

#[tokio::test]
async fn service_boots_from_external_config_pool_and_key_file() {
    let dir = TempDir::new("service-boot");
    let config_path = dir.write(
        "config.json",
        r#"{"algorithm_type":"service-boot-config","port":0,"anti_crawler":{"enabled":false,"redirect_url":""}}"#,
    );
    let pool_path = dir.write("device_pool.json", &pool_json(5));
    let filter_path = dir.write("filter.json", r#"{"routes":{}}"#);
    let encoded_key = base64::engine::general_purpose::STANDARD.encode(POOL_KEY);
    let pool_key_path = dir.write("pool.key", &format!("{encoded_key}\n"));

    let mock = MockUpstream::start(|_| MockReply {
        status: 200,
        headers: vec![("content-type".to_string(), "application/json".to_string())],
        body: MOCK_JSON.as_bytes().to_vec(),
    })
    .await;
    let options = ServiceOptions {
        config_path,
        pool_path,
        filter_path,
        pool_key_path,
        listen: "127.0.0.1:0".parse().expect("test listen address"),
        mock_upstream: Some(mock.origin.clone()),
    };

    let server = build_server(&options).await.expect("service build");
    assert_eq!(server.ctx.cfg.algorithm_type, "service-boot-config");
    assert!(server.ctx.src_dir.is_empty());
    assert_eq!(server.ctx.pool.count().await, 5);
    assert_eq!(pool_crypto::pool_key(), Some(POOL_KEY));

    for path in [
        "/api/v1/books/123",
        "/api/v1/books/123/directory",
        "/api/v1/novels/123",
    ] {
        let request = Request {
            method: "GET".to_string(),
            path: path.to_string(),
            ..Request::default()
        };
        let response = dispatch(&server, &request).await;
        assert_eq!(response.status, 200, "{path}");
        match response.body {
            ResponseBody::Bytes(body) => assert_eq!(body, MOCK_JSON.as_bytes(), "{path}"),
            _ => panic!("service emitted a file response for {path}"),
        }
    }
    assert_eq!(mock.requests().len(), 3);
}
