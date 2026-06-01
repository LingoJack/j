pub mod bridge;
pub mod server;
pub mod setup;

use rust_embed::RustEmbed;

// Re-export protocol and crypto from j-cli-core
pub use j_agent::crypto;
pub use j_agent::protocol;

pub use setup::start_remote_and_wait;

/// Remote 前端 SPA 静态资源 — 由 `assets/remote/vite.config.js` 用
/// `vite-plugin-singlefile` 打成单文件 `assets/remote/dist/remote.html`，
/// 编译时通过 rust-embed 嵌入到 `j` 二进制。
///
/// 构建命令：`cd assets/remote && npm run build`（或 `make build-remote`）。
#[derive(Debug, RustEmbed)]
#[folder = "assets/remote/dist/"]
pub struct RemoteAssets;
