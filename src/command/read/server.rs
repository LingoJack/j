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
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

use super::embed::ReaderAssets;
use super::renderer::{RenderedDoc, is_image_path, render_file};
use super::{MAX_ASSET_SIZE, MAX_DIR_ENTRIES, MAX_FILE_SIZE};

/// 心跳超时：前端断联超过这个时长，server 自动 shutdown。
/// 前端每 5 秒发一次，给宽 6 倍裕度。
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(30);
/// watcher 检查心跳的频率
const HEARTBEAT_CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// 服务端共享状态。
#[derive(Clone)]
struct AppState {
    /// 启动文件（initial tab 路径）；目录入口时为 None
    initial_path: Arc<Option<PathBuf>>,
    /// 启动时左侧文件树的根目录（initial_path 的父目录或目录入口本身）
    root_dir: Arc<PathBuf>,
    /// shutdown 信号（前端在全部 tab 干净后通过 `/api/shutdown` 触发；
    /// 心跳超时也会触发它）
    shutdown: Arc<Notify>,
    /// 最近一次收到心跳（或任何 API 请求）的 monotonic 毫秒数。
    /// 用 process-relative 的 `Instant::elapsed()` 表示，AtomicU64 存毫秒。
    last_heartbeat_ms: Arc<AtomicU64>,
    /// server 启动时刻（用于心跳的相对时间基准）
    start: Arc<Instant>,
}

impl AppState {
    /// 在收到任何活跃信号（heartbeat / 实际 API 请求）时调用。
    fn touch(&self) {
        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        self.last_heartbeat_ms.store(elapsed_ms, Ordering::Relaxed);
    }
}

/// 启动 server 并阻塞当前线程，直到 server 退出。
///
/// `initial_path` 为 None 表示「目录入口」—— 前端只显示文件树，不预选任何文件。
pub fn serve_blocking(
    initial_path: Option<PathBuf>,
    root_dir: PathBuf,
    port: Option<u16>,
) -> Result<(), String> {
    // 多线程 runtime —— 让 file IO（read_to_string / canonicalize / 列目录）
    // 不阻塞 server worker。worker_threads 留 tokio 自己挑（默认 = 物理核数）。
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
        let start = Arc::new(Instant::now());
        let last_heartbeat_ms = Arc::new(AtomicU64::new(0));
        let state = AppState {
            initial_path: Arc::new(initial_path),
            root_dir: Arc::new(root_dir),
            shutdown: shutdown.clone(),
            last_heartbeat_ms: last_heartbeat_ms.clone(),
            start: start.clone(),
        };

        // 心跳 watcher：定期检查上次心跳；超过 HEARTBEAT_TIMEOUT 即 shutdown。
        // 这是浏览器关窗口时 beforeunload / sendBeacon 偶尔不触发的兜底。
        {
            let shutdown = shutdown.clone();
            let last = last_heartbeat_ms.clone();
            let start = start.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(HEARTBEAT_CHECK_INTERVAL);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    let now_ms = start.elapsed().as_millis() as u64;
                    let last_ms = last.load(Ordering::Relaxed);
                    // last_ms == 0 时还没有任何心跳到达 —— 服务刚启动给宽限期
                    if last_ms == 0 {
                        // 给 60 秒宽限：浏览器还没打开 / 还没注入 heartbeat
                        if now_ms > 60_000 {
                            eprintln!("📖 reader: 60s 内未收到任何心跳，自动退出");
                            shutdown.notify_one();
                            return;
                        }
                        continue;
                    }
                    let stale_ms = now_ms.saturating_sub(last_ms);
                    if stale_ms > HEARTBEAT_TIMEOUT.as_millis() as u64 {
                        eprintln!(
                            "📖 reader: 心跳超时（{}s 未收到），自动退出",
                            stale_ms / 1000
                        );
                        shutdown.notify_one();
                        return;
                    }
                }
            });
        }

        let app = Router::new()
            .route("/api/initial", get(api_initial))
            .route("/api/file", get(api_file))
            .route("/api/list", get(api_list))
            .route("/api/save", post(api_save))
            .route("/api/create", post(api_create))
            .route("/api/asset", get(api_asset))
            .route("/api/heartbeat", post(api_heartbeat))
            .route("/api/shutdown", post(api_shutdown))
            .route("/", get(index_handler))
            .fallback(static_handler)
            .with_state(state);

        let url = format!("http://{}/", local_addr);
        println!("📖 reader 已启动：{url}");
        println!("   关闭浏览器页面或按 Ctrl+C 停止");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(shutdown))
            .await
            .map_err(|e| format!("server 异常退出：{e}"))?;
        Ok::<(), String>(())
    })
}

async fn shutdown_signal(shutdown: Arc<Notify>) {
    // 历史教训：以前这里同时监听 stdin "按 Enter 停止"。但 `tokio::io::stdin()`
    // 在内部用 blocking 线程持有真实 stdin，select! 赢的另一分支无法 cancel
    // 这个 syscall —— 进程虽然 graceful shutdown 了，stdin 还被孤儿线程占着，
    // 控制权返回 j 交互式 REPL（rustyline）后用户必须再敲一次 Enter 才能恢复
    // 输入。因此**不再读 stdin**：浏览器关窗 / ⌘W / 心跳超时都能触发 notify。
    // 终端里直接跑 `j read` 想中断的话，Ctrl+C 仍然有效（tokio runtime 接到
    // SIGINT 会立刻退）。
    shutdown.notified().await;
    println!("📖 reader 已关闭");
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
        // 图片：放宽到 MAX_ASSET_SIZE（不读字节进 source，所以也不用担心内存）
        // 其它：MAX_FILE_SIZE
        let limit = if is_image_path(&path) {
            MAX_ASSET_SIZE
        } else {
            MAX_FILE_SIZE
        };
        if metadata.len() > limit {
            return Err(ApiError::bad_request(format!(
                "文件过大（{} 字节，超过 {} 字节上限）",
                metadata.len(),
                limit
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
// /api/create — 在指定目录创建一个新文件（默认空内容）
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateReq {
    /// 父目录绝对路径（必须已存在且是目录）
    dir: String,
    /// 新文件名（不能含 `/` 或 `\`，不能为 `.` / `..`）
    name: String,
    /// 可选：初始内容；缺省为空字符串
    #[serde(default)]
    source: String,
}

#[derive(Serialize)]
struct CreateResp {
    /// 新文件的规范化绝对路径，前端拿到后直接 open
    path: String,
}

async fn api_create(Json(req): Json<CreateReq>) -> Result<Json<CreateResp>, ApiError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("文件名不能为空"));
    }
    if name == "." || name == ".." {
        return Err(ApiError::bad_request("非法文件名"));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(ApiError::bad_request("文件名不能包含路径分隔符"));
    }
    if req.source.len() as u64 > MAX_FILE_SIZE {
        return Err(ApiError::bad_request(format!(
            "初始内容过大（{} 字节，超过 {} 字节上限）",
            req.source.len(),
            MAX_FILE_SIZE
        )));
    }

    let dir = canonicalize(&req.dir)?;
    let dir_meta =
        std::fs::metadata(&dir).map_err(|e| ApiError::bad_request(format!("无法读取目录：{e}")))?;
    if !dir_meta.is_dir() {
        return Err(ApiError::bad_request("目标不是一个目录"));
    }
    let target = dir.join(name);
    if target.exists() {
        return Err(ApiError::bad_request(format!(
            "已存在同名文件或目录：{}",
            target.display()
        )));
    }

    // 用 tempfile + persist_noclobber 避免覆盖（极小竞态窗口里也不会写入已存在的文件）
    let mut tmp = tempfile::NamedTempFile::new_in(&dir)
        .map_err(|e| ApiError::internal(format!("创建临时文件失败：{e}")))?;
    use std::io::Write;
    if !req.source.is_empty() {
        tmp.write_all(req.source.as_bytes())
            .map_err(|e| ApiError::internal(format!("写入临时文件失败：{e}")))?;
    }
    tmp.as_file()
        .sync_all()
        .map_err(|e| ApiError::internal(format!("同步临时文件失败：{e}")))?;
    tmp.persist_noclobber(&target)
        .map_err(|e| ApiError::internal(format!("落盘失败：{e}")))?;

    let canonical = std::fs::canonicalize(&target)
        .map_err(|e| ApiError::internal(format!("解析新建文件路径失败：{e}")))?;
    Ok(Json(CreateResp {
        path: canonical.display().to_string(),
    }))
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
// /api/heartbeat — 前端 5s 一次心跳；超时 30s 自动 shutdown
// ---------------------------------------------------------------------------

async fn api_heartbeat(State(state): State<AppState>) -> &'static str {
    state.touch();
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
