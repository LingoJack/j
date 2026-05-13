# @ 文件搜索性能优化方案

## 问题分析

### 根因

`draw_at_popup()` 和 `draw_file_popup()` 在 **每帧渲染** 时调用 `get_filtered_all_items(app)` / `get_filtered_files(app)`，这些函数使用 `ignore::WalkBuilder` 递归扫描项目目录（max_depth=8），在大项目中单次调用 100-500ms。渲染帧率约 30fps，每帧都触发目录扫描导致严重卡顿。

### 关键代码路径

```
每帧 → draw_chat_ui() → draw_at_popup() → get_filtered_all_items() → WalkBuilder 扫描目录 ← 阻塞！
```

---

## 解决方案：全局文件索引 + 缓存

### 核心思路

1. 启动 Chat 时构建一次全局文件索引（后台线程）
2. 用 `notify` crate 监控项目目录变化（新建/删除/重命名），增量更新索引
3. 弹窗渲染时只读索引 + 内存中过滤，不触发任何文件系统操作
4. 每次打开弹窗时触发一次索引刷新（确保最新）

### 详细设计

#### 1. 新增 `FileIndex` 结构体

在 `src/command/chat/input/file_index.rs` 中：

```rust
/// 项目文件索引：维护一个内存中的文件路径列表
pub struct FileIndex {
    /// 缓存的相对文件路径列表（目录以 '/' 结尾）
    files: Vec<String>,
    /// 上次完整扫描时间
    last_scan: std::time::Instant,
    /// 后台文件监控线程的停止标记
    watch_stop: Arc<AtomicBool>,
    /// 缓存是否就绪（首次扫描完成）
    ready: Arc<AtomicBool>,
}

impl FileIndex {
    /// 创建并启动后台扫描 + 文件监控
    pub fn new() -> Self;
    
    /// 获取所有缓存的文件路径（用于渲染，只读）
    pub fn files(&self) -> &[String];
    
    /// 按前缀过滤文件（目录导航模式）
    pub fn filter_by_prefix(&self, prefix: &str) -> Vec<&String>;
    
    /// 按关键词模糊搜索文件（替换 WalkBuilder 扫描）
    pub fn fuzzy_search(&self, keyword: &str, max_results: usize) -> Vec<String>;
    
    /// 主动触发一次重新扫描（弹窗打开时调用）
    pub fn refresh(&mut self);
    
    /// 停止后台监控（ChatApp drop 时调用）
    pub fn shutdown(&self);
}
```

#### 2. 后台文件监控

使用 `notify` crate（Rust 生态标准的跨平台文件监控库）：

```rust
fn start_watcher(watch_stop: Arc<AtomicBool>, index: Arc<Mutex<Vec<String>>>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx).unwrap();
    watcher.watch(Path::new("."), RecursiveMode::Recursive);
    
    // 后台线程处理文件变化事件
    thread::spawn(move || {
        while !watch_stop.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(event)) => {
                    match event.kind {
                        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {
                            // 标记索引需要刷新
                            needs_refresh.store(true, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    });
}
```

#### 3. 渲染时使用索引

改造 `get_filtered_all_items` / `get_filtered_files`：

```rust
// 改造前（每帧扫描目录）：
pub fn get_filtered_files(app: &ChatApp) -> Vec<String> {
    let walker = ignore::WalkBuilder::new(".").build(); // 100-500ms!
    // ...
}

// 改造后（只读内存索引）：
pub fn get_filtered_files(app: &ChatApp) -> Vec<String> {
    let index = &app.file_index;
    // 内存中过滤：纯字符串操作，< 1ms
    index.fuzzy_search(&filter, 15)
}
```

#### 4. ChatApp 集成

```rust
pub struct ChatApp {
    // ... existing fields ...
    pub file_index: FileIndex,  // 新增
}
```

---

## 实现步骤

### Step 1: 添加 `notify` 依赖
- `Cargo.toml` 中添加 `notify = "7"` (或最新稳定版)

### Step 2: 创建 `file_index.rs` 模块
- 实现 `FileIndex` 结构体
- 实现初始扫描逻辑（复用现有 WalkBuilder 配置）
- 实现后台文件监控 + 增量更新
- 实现 `fuzzy_search()` / `filter_by_prefix()` 方法

### Step 3: 在 `ChatApp` 中集成
- `ChatApp::new()` 时创建 `FileIndex`
- `ChatApp::drop` 或退出时调用 `file_index.shutdown()`

### Step 4: 改造 `autocomplete.rs` 中的搜索函数
- `get_filtered_files()` → 使用 `app.file_index.fuzzy_search()`
- `get_filtered_files_for_at()` → 使用 `app.file_index.fuzzy_search()`
- 路径导航模式（filter 含 `/`）→ 使用 `app.file_index.filter_by_prefix()`

### Step 5: 验证
- `cargo fmt` + `cargo clippy` 通过
- 测试大项目（>5000 文件）下的弹窗响应速度

---

## 预期效果

| 场景 | 改造前 | 改造后 |
|------|--------|--------|
| 打开 @弹窗 | 100-500ms 阻塞 | < 1ms（读内存） |
| 输入过滤字符 | 每帧 100-500ms | < 1ms |
| 渲染帧率 | 弹窗激活时 2-10fps | 稳定 30fps |
| 新建/删除文件 | 下次扫描可见 | 监控触发后自动更新 |
| 内存开销 | 无额外 | 缓存文件列表（通常 < 1MB） |

---

## 注意事项

1. **文件监控可靠性**：`notify` 在 macOS 上使用 FSEvents，Linux 上使用 inotify，Windows 上使用 ReadDirectoryChangesW，均为操作系统原生 API
2. **首次扫描时机**：Chat 启动时后台扫描，弹窗打开时如果索引未就绪则等待（或显示 Loading...）
3. **.gitignore 兼容**：初始扫描仍使用 `ignore::WalkBuilder`，确保尊重 .gitignore 规则
4. **大仓库优化**：对 node_modules 等大目录，WalkBuilder 已通过 .gitignore 过滤
5. **watcher 错误处理**：监控失败时静默降级为"打开弹窗时同步扫描"模式
