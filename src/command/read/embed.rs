//! Reader SPA 静态资源 — 由 `web/vite.config.reader.ts` 构建到 `assets/reader_web/`，
//! 编译时通过 rust-embed 嵌入到 `j` 二进制。
//!
//! 构建命令：`cd web && npm run build:reader`（或 `make build-reader-web`）。

use rust_embed::RustEmbed;

#[derive(Debug, RustEmbed)]
#[folder = "assets/reader_web/"]
pub struct ReaderAssets;
