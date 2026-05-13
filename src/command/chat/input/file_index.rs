//! 项目文件索引：维护内存中的文件路径列表，支持后台监控自动更新。
//!
//! 解决 @ 弹窗和文件弹窗每帧渲染时触发 WalkBuilder 扫描导致的性能问题。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::util::log::{write_error_log, write_info_log};
use ignore::WalkBuilder;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// 文件索引刷新间隔（秒）
const REFRESH_CHECK_INTERVAL_SECS: u64 = 1;

/// 项目文件索引：维护一个内存中的文件路径列表
pub struct FileIndex {
    /// 缓存的相对文件路径列表（目录以 '/' 结尾）
    files: Arc<Mutex<Vec<String>>>,
    /// 上次完整扫描时间
    #[allow(dead_code)]
    last_scan: Arc<Mutex<Instant>>,
    /// 后台文件监控线程的停止标记
    watch_stop: Arc<AtomicBool>,
    /// 缓存是否就绪（首次扫描完成）
    ready: Arc<AtomicBool>,
    /// 是否需要刷新（文件变化事件触发）
    needs_refresh: Arc<AtomicBool>,
    /// 文件监控器（非 blocking，生命周期与 FileIndex 绑定）
    watcher: Option<RecommendedWatcher>,
}

impl FileIndex {
    /// 创建并启动后台扫描 + 文件监控
    ///
    /// 首次扫描在后台线程执行，不阻塞主线程。
    /// 文件监控使用 notify crate 监听项目目录变化。
    pub fn new() -> Self {
        let files: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let last_scan: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
        let watch_stop = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let needs_refresh = Arc::new(AtomicBool::new(true)); // 启动时触发首次扫描

        // 启动文件监控
        let needs_refresh_clone = Arc::clone(&needs_refresh);
        let watch_stop_clone = Arc::clone(&watch_stop);
        let watcher = Self::start_watcher(needs_refresh_clone, watch_stop_clone);

        // 启动后台扫描 + 刷新检查线程
        let files_clone = Arc::clone(&files);
        let last_scan_clone = Arc::clone(&last_scan);
        let ready_clone = Arc::clone(&ready);
        let needs_refresh_clone = Arc::clone(&needs_refresh);
        let watch_stop_clone = Arc::clone(&watch_stop);

        thread::spawn(move || {
            Self::background_scan_loop(
                files_clone,
                last_scan_clone,
                ready_clone,
                needs_refresh_clone,
                watch_stop_clone,
            );
        });

        Self {
            files,
            last_scan,
            watch_stop,
            ready,
            needs_refresh,
            watcher,
        }
    }

    /// 启动文件监控器
    fn start_watcher(
        needs_refresh: Arc<AtomicBool>,
        watch_stop: Arc<AtomicBool>,
    ) -> Option<RecommendedWatcher> {
        // 创建事件通道
        let (tx, rx) = std::sync::mpsc::channel::<Result<Event, notify::Error>>();

        // 创建监控器
        let mut watcher: RecommendedWatcher = match Watcher::new(tx, notify::Config::default()) {
            Ok(w) => w,
            Err(e) => {
                write_error_log(
                    "[FileIndex::start_watcher]",
                    &format!("创建 watcher 失败: {}", e),
                );
                return None;
            }
        };

        // 监控当前工作目录
        let cwd = Path::new(".");
        if let Err(e) = watcher.watch(cwd, RecursiveMode::Recursive) {
            write_error_log(
                "[FileIndex::start_watcher]",
                &format!("启动监控失败: {}", e),
            );
            return None;
        }

        write_info_log("[FileIndex]", "文件监控已启动");

        // 启动事件处理线程
        thread::spawn(move || {
            while !watch_stop.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Ok(event)) => {
                        // 只关注文件创建/删除/修改事件
                        match event.kind {
                            EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {
                                // 过滤 .git 等无关目录的变化
                                let paths = &event.paths;
                                let should_refresh = paths.iter().any(|p| {
                                    let path_str = p.to_string_lossy();
                                    // 忽略 .git、.jcli/worktrees 等目录的变化
                                    !path_str.starts_with(".git/")
                                        && !path_str.starts_with(".jcli/worktrees/")
                                        && !path_str.contains("/.git/")
                                });
                                if should_refresh {
                                    needs_refresh.store(true, Ordering::Relaxed);
                                    write_info_log(
                                        "[FileIndex::watcher]",
                                        "检测到文件变化，标记需要刷新",
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Err(e)) => {
                        write_error_log("[FileIndex::watcher]", &format!("监控事件错误: {}", e));
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // 超时继续等待
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        // 通道关闭，退出
                        break;
                    }
                }
            }
            write_info_log("[FileIndex::watcher]", "监控线程已退出");
        });

        Some(watcher)
    }

    /// 后台扫描循环：定期检查 needs_refresh 标记，触发重新扫描
    fn background_scan_loop(
        files: Arc<Mutex<Vec<String>>>,
        last_scan: Arc<Mutex<Instant>>,
        ready: Arc<AtomicBool>,
        needs_refresh: Arc<AtomicBool>,
        watch_stop: Arc<AtomicBool>,
    ) {
        write_info_log("[FileIndex::scan_loop]", "后台扫描线程已启动");

        while !watch_stop.load(Ordering::Relaxed) {
            // 检查是否需要刷新
            if needs_refresh.load(Ordering::Relaxed) {
                needs_refresh.store(false, Ordering::Relaxed);

                write_info_log("[FileIndex::scan_loop]", "开始扫描项目目录...");
                let start = Instant::now();

                // 执行扫描
                let new_files = Self::scan_files_internal();

                // 更新缓存
                if let Ok(mut guard) = files.lock() {
                    *guard = new_files;
                }
                if let Ok(mut guard) = last_scan.lock() {
                    *guard = Instant::now();
                }
                ready.store(true, Ordering::Relaxed);

                let elapsed = start.elapsed();
                write_info_log(
                    "[FileIndex::scan_loop]",
                    &format!(
                        "扫描完成，共 {} 个文件，耗时 {}ms",
                        if let Ok(guard) = files.lock() {
                            guard.len()
                        } else {
                            0
                        },
                        elapsed.as_millis()
                    ),
                );
            }

            // 定期检查
            thread::sleep(Duration::from_secs(REFRESH_CHECK_INTERVAL_SECS));
        }

        write_info_log("[FileIndex::scan_loop]", "后台扫描线程已退出");
    }

    /// 执行目录扫描的核心逻辑（复用现有 WalkBuilder 配置）
    fn scan_files_internal() -> Vec<String> {
        let search_root = Path::new(".");
        let walker = WalkBuilder::new(search_root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .max_depth(Some(8))
            .build();

        let mut files: Vec<String> = Vec::new();
        for entry in walker.flatten() {
            let path = entry.path();
            // 跳过根目录本身
            if path == search_root {
                continue;
            }
            let rel = path
                .strip_prefix(search_root)
                .unwrap_or(path)
                .to_string_lossy();
            let rel_str = rel.as_ref();

            // 跳过隐藏路径段（除非以 . 开头的合法项目配置）
            if rel_str
                .split('/')
                .any(|seg| seg.starts_with('.') && seg != ".jcli")
            {
                continue;
            }

            // 跳过 .jcli/worktrees 目录（避免扫描大量 worktree 文件）
            if rel_str.starts_with(".jcli/worktrees/") {
                continue;
            }

            let is_dir = path.is_dir();
            let display = if is_dir {
                format!("{}/", rel_str)
            } else {
                rel_str.to_string()
            };
            files.push(display);
        }

        // 排序：目录优先，然后按名称排序
        files.sort_by(|a, b| {
            let a_dir = a.ends_with('/');
            let b_dir = b.ends_with('/');
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.to_lowercase().cmp(&b.to_lowercase()),
            }
        });

        files
    }

    /// 获取所有缓存的文件路径（用于渲染，只读）
    pub fn files(&self) -> Vec<String> {
        if let Ok(guard) = self.files.lock() {
            guard.clone()
        } else {
            Vec::new()
        }
    }

    /// 缓存是否就绪
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    /// 按前缀过滤文件（目录导航模式）
    ///
    /// 当 filter 包含 `/` 时，提取目录部分和文件前缀，
    /// 从缓存中匹配该目录下的文件/子目录。
    #[allow(dead_code)]
    pub fn filter_by_prefix(&self, filter: &str) -> Vec<String> {
        let files = self.files();

        // 处理 ~ 路径展开（仅用于显示，不实际展开）
        let effective_filter = if filter.starts_with("~/") {
            // ~/ 路径无法从项目索引中匹配，返回空
            return Vec::new();
        } else {
            filter
        };

        // 如果 filter 包含 /，提取目录部分和前缀
        if let Some(last_slash) = effective_filter.rfind('/') {
            let dir_part = &effective_filter[..=last_slash];
            let prefix = &effective_filter[last_slash + 1..];
            let prefix_lower = prefix.to_lowercase();

            let mut entries: Vec<String> = Vec::new();
            for path in &files {
                // 检查是否以 dir_part 开头
                if !path.starts_with(dir_part) {
                    continue;
                }
                // 提取该目录下的名称
                let after_dir = &path[dir_part.len()..];
                // 检查是否有更深层的路径（不匹配）
                if after_dir.contains('/') && !after_dir.ends_with('/') {
                    continue;
                }
                // 检查前缀匹配
                let name = if let Some(stripped) = after_dir.strip_suffix('/') {
                    stripped
                } else {
                    after_dir
                };
                if !prefix_lower.is_empty() && !name.to_lowercase().starts_with(&prefix_lower) {
                    continue;
                }
                entries.push(path.clone());
            }
            entries.truncate(15);
            return entries;
        }

        // 无 / 时，直接前缀匹配文件名
        let filter_lower = effective_filter.to_lowercase();
        let mut entries: Vec<String> = Vec::new();
        for path in &files {
            let name = if let Some(stripped) = path.strip_suffix('/') {
                stripped
            } else {
                path.as_str()
            };
            // 取最后一段（文件名/目录名）
            let file_name = if let Some(slash) = name.rfind('/') {
                &name[slash + 1..]
            } else {
                name
            };
            if file_name.to_lowercase().starts_with(&filter_lower) {
                entries.push(path.clone());
            }
        }
        entries.truncate(15);
        entries
    }

    /// 模糊搜索文件（替换 WalkBuilder 扫描）
    ///
    /// 使用增强版模糊匹配：文件名匹配 + 评分排序。
    /// max_results 限制返回数量。
    pub fn fuzzy_search(&self, filter: &str, max_results: usize) -> Vec<String> {
        if filter.is_empty() {
            // 返回前 max_results 个文件（已排序）
            let files = self.files();
            return files.into_iter().take(max_results).collect();
        }

        let files = self.files();
        let filter_lower = filter.to_lowercase();

        let mut scored: Vec<(i32, String)> = Vec::new();
        for path in &files {
            let name = if let Some(stripped) = path.strip_suffix('/') {
                stripped
            } else {
                path.as_str()
            };

            // 取文件名部分
            let file_name = if let Some(slash) = name.rfind('/') {
                &name[slash + 1..]
            } else {
                name
            };

            // 模糊匹配
            if let Some(score) = Self::fuzzy_match_enhanced(file_name, &filter_lower, path) {
                scored.push((score, path.clone()));
            }
        }

        // 按分数排序
        scored.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
        });

        scored
            .into_iter()
            .take(max_results)
            .map(|(_, path)| path)
            .collect()
    }

    /// 增强版模糊匹配：返回匹配分数（越小越好）
    fn fuzzy_match_enhanced(file_name: &str, filter: &str, rel_path: &str) -> Option<i32> {
        let base_score = Self::fuzzy_match(file_name, filter)?;

        // 匹配位置加成：开头匹配更优
        let file_name_lower = file_name.to_lowercase();
        let position_bonus = if file_name_lower.starts_with(filter) {
            -50 // 开头匹配，大幅加分
        } else if file_name_lower.contains(filter) {
            -20 // 包含匹配，中等加分
        } else {
            0
        };

        // 扩展名优先级：代码文件更优
        let ext_bonus = if let Some(ext) = Path::new(file_name).extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" | "swift" => -15,
                "json" | "yaml" | "yml" | "toml" | "md" => -10,
                _ => 0,
            }
        } else {
            0
        };

        // 路径深度惩罚
        let depth = rel_path.matches('/').count() as i32;

        Some(base_score * 10 + depth + position_bonus + ext_bonus)
    }

    /// 模糊匹配：filter 的每个字符按顺序出现在 text 中即可匹配
    fn fuzzy_match(text: &str, filter: &str) -> Option<i32> {
        if filter.is_empty() {
            return Some(0);
        }
        let text_lower: Vec<char> = text.to_lowercase().chars().collect();
        let filter_lower: Vec<char> = filter.to_lowercase().chars().collect();
        let mut ti = 0;
        let mut score: i32 = 0;
        let mut last_match: Option<usize> = None;
        for &fc in &filter_lower {
            let mut found = false;
            while ti < text_lower.len() {
                if text_lower[ti] == fc {
                    // 连续匹配加分（间距小更好）
                    if let Some(lm) = last_match {
                        score += (ti - lm - 1) as i32;
                    }
                    last_match = Some(ti);
                    ti += 1;
                    found = true;
                    break;
                }
                ti += 1;
            }
            if !found {
                return None;
            }
        }
        Some(score)
    }

    /// 主动触发一次重新扫描（弹窗打开时调用）
    pub fn refresh(&self) {
        self.needs_refresh.store(true, Ordering::Relaxed);
    }

    /// 停止后台监控（ChatApp drop 时调用）
    pub fn shutdown(&self) {
        self.watch_stop.store(true, Ordering::Relaxed);
        write_info_log("[FileIndex]", "已标记停止，后台线程将退出");
    }
}

impl Default for FileIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for FileIndex {
    fn drop(&mut self) {
        self.shutdown();
        // 停止 watcher
        if let Some(ref mut watcher) = self.watcher {
            let _ = watcher.unwatch(Path::new("."));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_match_basic() {
        assert_eq!(fuzzy_match("main.rs", "mai"), Some(0));
        assert_eq!(fuzzy_match("main.rs", "mrs"), Some(2)); // m..r..s, 间隔 2
        assert_eq!(fuzzy_match("main.rs", "xyz"), None);
    }

    #[test]
    fn fuzzy_match_empty_filter() {
        assert_eq!(fuzzy_match("anything", ""), Some(0));
    }
}
