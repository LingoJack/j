# Plan: `j read` 服务在浏览器标签页关闭时自动关闭

## 背景

当前 `j read` 启动一个本地 axum HTTP 服务，服务仅通过 Ctrl-C 手动关闭。用户希望：当浏览器中关闭了 reader 页面后，服务能自动关闭。

## 方案：前端 `beforeunload` + `/api/shutdown` GET 接口

### 原理

1. 后端新增一个 `/api/shutdown` GET 接口，调用时触发 graceful shutdown。
2. 前端在 `window.onbeforeunload` 事件中用 `navigator.sendBeacon('/api/shutdown')` 发送关闭请求。
3. 后端收到请求后触发 shutdown 信号，服务自动退出。

> `sendBeacon` 保证在页面卸载时可靠发送，不会被浏览器取消（不像 `fetch` 可能被 abort）。

### 需要修改的文件

#### 1. `src/command/read/server.rs` — 后端改动

- 新增 `shutdown_tx: tokio::sync::Notify` 放入 `AppState`（用 `Arc<Notify>`）
- 新增路由 `/api/shutdown`，handler 中调用 `notify.notify_one()`
- 修改 `shutdown_signal()` 同时监听 Ctrl-C 和 `Notify`

核心改动很小，大约 20 行新增代码：

```rust
// AppState 新增字段
struct AppState {
    doc: Arc<RenderedDoc>,
    shutdown: Arc<tokio::sync::Notify>,
}

// 新增 handler
async fn api_shutdown(State(state): State<AppState>) -> &'static str {
    state.shutdown.notify_one();
    "ok"
}

// shutdown_signal 改为 select
async fn shutdown_signal(shutdown: Arc<tokio::sync::Notify>) {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n📖 reader 已关闭");
        }
        _ = shutdown.notified() => {
            println!("📖 reader 已关闭（页面已关闭）");
        }
    }
}
```

#### 2. `web/src/reader/Reader.tsx` — 前端改动

在已有的 `useEffect` 中添加 `beforeunload` 监听器：

```tsx
useEffect(() => {
  const handleUnload = () => {
    navigator.sendBeacon('/api/shutdown');
  };
  window.addEventListener('beforeunload', handleUnload);
  return () => window.removeEventListener('beforeunload', handleUnload);
}, []);
```

同时将提示文本从 "在终端按 Ctrl-C 关闭" 改为 "关闭此页面将自动停止服务"。

#### 3. 不需要新增任何依赖

### 边界情况

1. **`sendBeacon` 发的是 POST**：axum 路由用 `post()` 或 `any()` 即可
2. **用户刷新页面**：`beforeunload` 也会触发。但这不是问题——如果用户刷新了，页面会重新打开，服务已关闭时用户自然重新 `j read` 即可
3. **Ctrl-C 仍然有效**：`tokio::select!` 同时监听两种信号
4. **`sendBeacon` 兼容性**：所有现代浏览器都支持

### 构建步骤

1. 修改 `server.rs`（后端新增 shutdown 接口 + Notify 信号）
2. 修改 `Reader.tsx`（前端 beforeunload + sendBeacon）
3. 运行 `cargo fmt` 和 `cargo clippy`
4. 构建前端 `cd web && npm run build:reader`
