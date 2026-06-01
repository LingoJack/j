//! Reader SPA 静态资源 — 由 `assets/reader/vite.config.ts` 构建到 `assets/reader/dist/`，
//! 编译时通过 rust-embed 嵌入到 `j` 二进制。
//!
//! 构建命令：`cd assets/reader && npm run build`（或 `make build-reader-web`）。

use rust_embed::RustEmbed;

#[derive(Debug, RustEmbed)]
#[folder = "assets/reader/dist/"]
pub struct ReaderAssets;
