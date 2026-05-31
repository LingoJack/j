//! `j read` 命令的本地 HTTP 服务（多文件、多 Tab 编辑器）。
//!
//! 设计要点：
//! - 仅绑定 `127.0.0.1`，不暴露到局域网。
//! - 单用户本机使用；不加敏感路径 deny list（用户已确认）。
//! - 路径全部 `canonicalize` 后处理；写入用 `tempfile::NamedTempFile` + `persist()` 原子替换。
//! - 静态资源（reader SPA）来自编译期嵌入的 [`ReaderAssets`]。

use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Notify;

use super::embed::ReaderAssets;
use super::renderer::{RenderedDoc, render_file};
use super::{MAX_ASSET_SIZE, MAX_DIR_ENTRIES, MAX_FILE_SIZE};

/// 服务端共享状态。
#[derive(Clone)]
struct AppState {
    /// 启动文件（initial tab 路径）；目录入口时为 None
    initial_path: Arc<Option<PathBuf>>,
    /// 启动时左侧文件树的根目录（initial_path 的父目录或目录入口本身）
    root_dir: Arc<PathBuf>,
    /// shutdown 信号（前端在全部 tab 干净后通过 `/api/shutdown` 触发）
    shutdown: Arc<Notify>,
}

/// 启动 server 并阻塞当前线程，直到 server 退出。
///
/// `initial_path` 为 None 表示「目录入口」—— 前端只显示文件树，不预选任何文件。
pub fn serve_blocking(
    initial_path: Option<PathBuf>,
    root_dir: PathBuf,
    port: Option<u16>,
) -> Result<(), String> {
    // 多线程 runtime —— 避免 CPU-bound 路由（`/api/parse` 解析大 markdown）阻塞
    // 整个事件循环。worker_threads 留 tokio 自己挑（默认 = 物理核数）。
    let runtime = tokio::runtime::Builder::new_multi_thread()
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
            initial_path: Arc::new(initial_path),
            root_dir: Arc::new(root_dir),
            shutdown: shutdown.clone(),
        };
        let app = Router::new()
            .route("/api/initial", get(api_initial))
            .route("/api/file", get(api_file))
            .route("/api/list", get(api_list))
            .route("/api/parse", post(api_parse))
            .route("/api/save", post(api_save))
            .route("/api/asset", get(api_asset))
            .route("/api/shutdown", post(api_shutdown))
            .route("/", get(index_handler))
            .fallback(static_handler)
            .with_state(state);

        let url = format!("http://{}/", local_addr);
        println!("📖 reader 已启动：{url}");
        println!("   按 Enter 键停止，或关闭浏览器页面自动停止");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(shutdown))
            .await
            .map_err(|e| format!("server 异常退出：{e}"))?;
        Ok::<(), String>(())
    })
}

async fn shutdown_signal(shutdown: Arc<Notify>) {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let stdin_line = async {
        let mut reader = BufReader::new(tokio::io::stdin());
        let mut buf = String::new();
        let _ = reader.read_line(&mut buf).await;
    };

    tokio::select! {
        _ = stdin_line => {
            println!("📖 reader 已关闭");
        }
        _ = shutdown.notified() => {
            println!("📖 reader 已关闭（页面已关闭）");
        }
    }
}

// ---------------------------------------------------------------------------
// /api/initial — 启动信息
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct InitialResp {
    /// 目录入口时为 null
    initial_path: Option<String>,
    root_dir: String,
}

async fn api_initial(State(state): State<AppState>) -> Json<InitialResp> {
    Json(InitialResp {
        initial_path: state
            .initial_path
            .as_ref()
            .as_ref()
            .map(|p| p.display().to_string()),
        root_dir: state.root_dir.display().to_string(),
    })
}

// ---------------------------------------------------------------------------
// /api/file — 读取单文件并渲染
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct FileQuery {
    path: String,
}

async fn api_file(Query(q): Query<FileQuery>) -> Result<Json<RenderedDoc>, ApiError> {
    // 整个流程是 CPU + 磁盘 IO bound（read + parse_markdown）—— 丢去 blocking
    // 线程池，避免阻塞 tokio worker
    let path = canonicalize(&q.path)?;
    let doc = tokio::task::spawn_blocking(move || -> Result<RenderedDoc, ApiError> {
        let metadata = std::fs::metadata(&path)
            .map_err(|e| ApiError::bad_request(format!("无法读取文件：{e}")))?;
        if !metadata.is_file() {
            return Err(ApiError::bad_request("不是一个普通文件"));
        }
        if metadata.len() > MAX_FILE_SIZE {
            return Err(ApiError::bad_request(format!(
                "文件过大（{} 字节，超过 {} 字节上限）",
                metadata.len(),
                MAX_FILE_SIZE
            )));
        }
        render_file(&path).map_err(ApiError::bad_request)
    })
    .await
    .map_err(|e| ApiError::internal(format!("blocking 任务 join 失败：{e}")))??;
    Ok(Json(doc))
}

// ---------------------------------------------------------------------------
// /api/list — 列出目录内容
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ListQuery {
    dir: String,
    /// 是否包含 dotfile，默认 false。
    /// 接受 "1" / "true" / "yes" / "on"（大小写不敏感）等任意 truthy 字符串；
    /// 其余值视为 false。这样前端无论传 `'true'` 还是 `'1'` 都不会 400。
    #[serde(default, deserialize_with = "deserialize_flexible_bool")]
    hidden: bool,
}

#[derive(Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
}

#[derive(Serialize)]
struct ListResp {
    dir: String,
    /// 父目录绝对路径；为根（无父）时为 null。前端用于「上一级」按钮。
    parent: Option<String>,
    entries: Vec<FileEntry>,
    /// 目录条目数超过 [`MAX_DIR_ENTRIES`] 时为 true，前端展示「目录过大，仅显示前 N 条」
    truncated: bool,
}

async fn api_list(Query(q): Query<ListQuery>) -> Result<Json<ListResp>, ApiError> {
    let dir = canonicalize(&q.dir)?;
    let metadata =
        std::fs::metadata(&dir).map_err(|e| ApiError::bad_request(format!("无法读取目录：{e}")))?;
    if !metadata.is_dir() {
        return Err(ApiError::bad_request("不是一个目录"));
    }

    let read_dir =
        std::fs::read_dir(&dir).map_err(|e| ApiError::bad_request(format!("无法列出目录：{e}")))?;

    let mut all: Vec<FileEntry> = Vec::new();
    for entry in read_dir.flatten() {
        let name = match entry.file_name().to_str() {
            Some(s) => s.to_string(),
            None => continue, // 忽略非 UTF-8 文件名
        };
        if !q.hidden && name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        all.push(FileEntry {
            name,
            path: path.display().to_string(),
            is_dir: meta.is_dir(),
            size: meta.len(),
        });
    }

    // 排序：目录在前；同类按名（不区分大小写）
    all.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let truncated = all.len() > MAX_DIR_ENTRIES;
    if truncated {
        all.truncate(MAX_DIR_ENTRIES);
    }

    Ok(Json(ListResp {
        dir: dir.display().to_string(),
        parent: dir.parent().map(|p| p.display().to_string()),
        entries: all,
        truncated,
    }))
}

// ---------------------------------------------------------------------------
// /api/parse — 解析 Markdown 字符串为 IR（不写盘）
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ParseReq {
    source: String,
}

async fn api_parse(Json(req): Json<ParseReq>) -> Json<serde_json::Value> {
    // parse_markdown 是 CPU-bound 的同步函数；多线程 runtime + spawn_blocking
    // 双管齐下，确保不会阻塞其它请求（list / file / save / asset）。
    let value = tokio::task::spawn_blocking(move || {
        let doc = crate::markdown::parser::parse_markdown(&req.source, 120);
        serde_json::to_value(&doc).unwrap_or(serde_json::Value::Null)
    })
    .await
    .unwrap_or(serde_json::Value::Null);
    Json(value)
}

// ---------------------------------------------------------------------------
// /api/save — 原子写回文件
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SaveReq {
    path: String,
    source: String,
}

#[derive(Serialize)]
struct SaveResp {
    ok: bool,
    saved_at: u128,
}

async fn api_save(Json(req): Json<SaveReq>) -> Result<Json<SaveResp>, ApiError> {
    if req.source.len() as u64 > MAX_FILE_SIZE {
        return Err(ApiError::bad_request(format!(
            "保存内容过大（{} 字节，超过 {} 字节上限）",
            req.source.len(),
            MAX_FILE_SIZE
        )));
    }
    let path = canonicalize(&req.path)?;
    if !path.is_file() {
        return Err(ApiError::bad_request("目标不是一个普通文件"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| ApiError::bad_request("无法定位父目录"))?;

    // tempfile + persist：在同目录创建临时文件 → 写入 → 原子 rename
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| ApiError::internal(format!("创建临时文件失败：{e}")))?;
    use std::io::Write;
    tmp.write_all(req.source.as_bytes())
        .map_err(|e| ApiError::internal(format!("写入临时文件失败：{e}")))?;
    tmp.as_file()
        .sync_all()
        .map_err(|e| ApiError::internal(format!("同步临时文件失败：{e}")))?;
    tmp.persist(&path)
        .map_err(|e| ApiError::internal(format!("原子替换失败：{e}")))?;

    let saved_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    Ok(Json(SaveResp { ok: true, saved_at }))
}

// ---------------------------------------------------------------------------
// /api/asset — 返回静态资源（图片等）
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AssetQuery {
    path: String,
}

async fn api_asset(Query(q): Query<AssetQuery>) -> Result<Response, ApiError> {
    let path = canonicalize(&q.path)?;
    let metadata = std::fs::metadata(&path)
        .map_err(|e| ApiError::bad_request(format!("无法读取文件：{e}")))?;
    if !metadata.is_file() {
        return Err(ApiError::bad_request("不是一个普通文件"));
    }
    if metadata.len() > MAX_ASSET_SIZE {
        return Err(ApiError::bad_request(format!(
            "资源过大（{} 字节，超过 {} 字节上限）",
            metadata.len(),
            MAX_ASSET_SIZE
        )));
    }
    let bytes =
        std::fs::read(&path).map_err(|e| ApiError::internal(format!("读取资源失败：{e}")))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let mut response = Response::new(axum::body::Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    Ok(response)
}

// ---------------------------------------------------------------------------
// /api/shutdown — 浏览器关闭页面时触发
// ---------------------------------------------------------------------------

async fn api_shutdown(State(state): State<AppState>) -> &'static str {
    state.shutdown.notify_one();
    "ok"
}

// ---------------------------------------------------------------------------
// Static assets
// ---------------------------------------------------------------------------

async fn index_handler() -> Response {
    serve_embedded("reader.html").unwrap_or_else(not_found)
}

async fn static_handler(uri: Uri) -> Response {
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// 规范化路径（解析 `..`、软链、`~`），失败返回 4xx 错误。
fn canonicalize(input: &str) -> Result<PathBuf, ApiError> {
    let expanded = expand_tilde(input);
    std::fs::canonicalize(&expanded)
        .map_err(|e| ApiError::bad_request(format!("无法解析路径 \"{input}\"：{e}")))
}

fn expand_tilde(path: &str) -> String {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = dirs::home_dir()
    {
        if path == "~" {
            home.display().to_string()
        } else {
            format!("{}{}", home.display(), &path[1..])
        }
    } else {
        path.to_string()
    }
}

/// 宽容的 bool 反序列化 —— 用于 query string 字段。
///
/// 标准 serde bool 只接受 `true` / `false` 字面量；query string 里前端常用
/// `1` / `0`、`yes` / `no`，直接走默认 deserializer 会 400。这里统一兜住。
fn deserialize_flexible_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer).unwrap_or_default();
    Ok(matches!(
        raw.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on" | "y" | "t"
    ))
}

// ---------------------------------------------------------------------------
// 统一错误响应
// ---------------------------------------------------------------------------

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
    fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "error": self.message });
        (self.status, Json(body)).into_response()
    }
}
