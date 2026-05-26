//! `j read` 命令的本地 HTTP 服务。
//!
//! 设计要点：
//! - 仅绑定 `127.0.0.1`，不暴露到局域网。
//! - 启动时一次性读取目标文件并渲染为 [`RenderedDoc`]，缓存在内存中；
//!   `/api/doc` 只返回这一份，**不接受任意路径参数**，杜绝越权读盘。
//! - 静态资源（reader SPA）来自编译期嵌入的 [`ReaderAssets`]。

use axum::{
    Router,
    extract::State,
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Notify;

use super::embed::ReaderAssets;
use super::renderer::RenderedDoc;

/// 服务端共享状态：渲染好的文档（不可变，多线程共享）+ shutdown 信号。
#[derive(Clone)]
struct AppState {
    doc: Arc<RenderedDoc>,
    shutdown: Arc<Notify>,
}

/// 启动 server 并阻塞当前线程，直到 server 退出（Ctrl-C 或浏览器页面关闭）。
///
/// 返回实际监听的地址（用于打印 URL / 打开浏览器）。
pub fn serve_blocking(doc: RenderedDoc, port: Option<u16>) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("创建 tokio 运行时失败：{e}"))?;

    runtime.block_on(async move {
        let bind_port = port.unwrap_or(0);
        let addr: SocketAddr = ([127, 0, 0, 1], bind_port).into();
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("无法监听 127.0.0.1:{bind_port}：{e}"))?;
        let local_addr = listener
            .local_addr()
            .map_err(|e| format!("获取监听地址失败：{e}"))?;

        let shutdown = Arc::new(Notify::new());
        let state = AppState {
            doc: Arc::new(doc),
            shutdown: shutdown.clone(),
        };
        let app = Router::new()
            .route("/api/doc", get(api_doc))
            .route("/api/shutdown", post(api_shutdown))
            .route("/", get(index_handler))
            .fallback(static_handler)
            .with_state(state);

        let url = format!("http://{}/", local_addr);
        println!("📖 reader 已启动：{url}");
        println!("   关闭浏览器页面将自动停止服务，或按 Ctrl-C 停止");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(shutdown))
            .await
            .map_err(|e| format!("server 异常退出：{e}"))?;
        Ok::<(), String>(())
    })
}

async fn shutdown_signal(shutdown: Arc<Notify>) {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n📖 reader 已关闭");
        }
        _ = shutdown.notified() => {
            println!("📖 reader 已关闭（页面已关闭）");
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn api_doc(State(state): State<AppState>) -> Json<serde_json::Value> {
    // RenderedDoc 内部是 serde::Serialize；通过 to_value 一次性序列化，避免对 Arc<T> 直接派生。
    let value = serde_json::to_value(&*state.doc)
        .unwrap_or_else(|e| serde_json::json!({ "error": format!("序列化失败：{e}") }));
    Json(value)
}

/// 浏览器页面关闭时通过 `navigator.sendBeacon` 调用此接口，触发服务 shutdown。
async fn api_shutdown(State(state): State<AppState>) -> &'static str {
    state.shutdown.notify_one();
    "ok"
}

async fn index_handler() -> Response {
    // Reader SPA 入口固定为 reader.html（由 `web/vite.config.reader.ts` 决定）。
    serve_embedded("reader.html").unwrap_or_else(not_found)
}

async fn static_handler(uri: Uri) -> Response {
    // 去掉前导 `/`
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        return index_handler().await;
    }
    serve_embedded(path).unwrap_or_else(not_found)
}

fn serve_embedded(path: &str) -> Option<Response> {
    let file = ReaderAssets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut response = Response::new(axum::body::Body::from(file.data.into_owned()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    Some(response)
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404 Not Found").into_response()
}
