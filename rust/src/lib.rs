// fqapi_core - the Rust native core for the fqapp Android client.
//
// NOTE: flutter_rust_bridge must be able to find its generated module
// declaration in this file; it is declared explicitly below so code generation
// is a no-op on lib.rs. Keep this file free of inner (`//!`) crate docs: the
// generator inserts its declaration as the first line, and an inner doc comment
// after an item is a compile error (E0753).
//
// Layering:
//   api       - flutter_rust_bridge surface (transport for the Flutter app)
//   dispatch  - the single request dispatcher shared by FFI and loopback HTTP
//   endpoints - route table and business handlers
//   upstream  - signed HTTP access, device rotation, DH/CM key exchange
//   device    - persistent device pool + registration
//   sign      - request signing primitives
//
// See docs/migration/rust-migration-contracts.md for the behaviour contract and
// docs/rust-migration-progress.md for the current verification state.

#[cfg(feature = "app")]
pub mod api;
pub mod config;
pub mod core_log;
pub mod crypto;
pub mod device;
pub mod dispatch;
pub mod endpoints;
pub mod error;
pub mod filter;
pub mod json;
mod network;
#[cfg(any(feature = "app", feature = "service"))]
pub mod server;
#[cfg(feature = "service")]
pub mod service;
pub mod sign;
pub mod static_files;
pub mod timeutil;
pub mod upstream;

#[cfg(feature = "app")]
mod frb_generated;

// The legacy loopback adapter is shared with the host-test harness. In a
// service-only build its `crate::api` reference resolves to this private
// compatibility binding instead of compiling the Flutter bridge module.
#[cfg(all(feature = "service", not(feature = "app")))]
pub(crate) use crate::service as api;

#[cfg(all(test, feature = "app"))]
mod network_proxy_tests;

// 桌面播放器在 macOS 上也要能起本地回环服务，`fq_desktop_media_*` 才会被导出；
// 漏掉 macos 时 dylib 里没有这些符号，macOS 端 FFI 一调就崩（#489）。
#[cfg(all(
    feature = "app",
    any(
        target_os = "windows",
        target_os = "linux",
        target_os = "ios",
        target_os = "macos"
    )
))]
mod desktop_fetch;
#[cfg(all(
    feature = "app",
    any(
        target_os = "windows",
        target_os = "linux",
        target_os = "ios",
        target_os = "macos"
    )
))]
mod desktop_media;
