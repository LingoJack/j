use crate::util::path_utils::expand_tilde;
use serde_json::Value;
use std::env;
use std::path::{Component, Path, PathBuf};

/// 安全沙箱：限制 AI 工具只能操作指定目录范围内的文件
///
/// 默认安全目录为启动时的当前工作目录及其子目录。
/// 操作超出安全目录的路径时，工具调用将强制要求用户确认。
#[derive(Debug, Clone)]
pub struct Sandbox {
    /// 安全目录列表（已 canonicalize）
    safe_dirs: Vec<PathBuf>,
    /// 是否启用沙箱
    enabled: bool,
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Sandbox {
    /// 创建沙箱，默认安全目录为 cwd
    pub fn new() -> Self {
        let cwd = env::current_dir().ok().and_then(|p| p.canonicalize().ok());
        Self {
            safe_dirs: cwd.into_iter().collect(),
            enabled: true,
        }
    }

    /// 获取除 cwd 以外的额外安全目录（用于 session 持久化）
    /// 第一个目录是 cwd，后续是 add_safe_dir 添加的
    pub fn extra_safe_dirs(&self) -> Vec<PathBuf> {
        if self.safe_dirs.len() <= 1 {
            Vec::new()
        } else {
            self.safe_dirs[1..].to_vec()
        }
    }

    /// 恢复额外安全目录（session 恢复时使用）
    pub fn restore_extra_safe_dirs(&mut self, dirs: Vec<PathBuf>) {
        for dir in dirs {
            if !self.safe_dirs.contains(&dir) {
                self.safe_dirs.push(dir);
            }
        }
    }

    /// 检查工具调用是否涉及沙箱外路径
    ///
    /// 返回 true 表示操作在沙箱外（需要用户确认）
    pub fn is_outside(&self, tool_name: &str, arguments: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let paths = extract_paths(tool_name, arguments);
        if paths.is_empty() {
            return false;
        }

        for path_str in &paths {
            if !self.is_path_safe(path_str) {
                return true;
            }
        }
        false
    }

    /// 检查单个路径是否在安全目录内
    fn is_path_safe(&self, path_str: &str) -> bool {
        let expanded = expand_tilde(path_str);
        let path = Path::new(&expanded);

        // 转为绝对路径
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match env::current_dir() {
                Ok(cwd) => cwd.join(path),
                Err(_) => return false,
            }
        };

        // canonicalize：解析 .. 和符号链接
        // 如果路径不存在，尝试 canonicalize 父目录 + 文件名
        let canonical = match absolute.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // 文件可能还不存在（Write 场景），检查其父目录
                if let Some(parent) = absolute.parent() {
                    match parent.canonicalize() {
                        Ok(p) => p.join(absolute.file_name().unwrap_or_default()),
                        Err(_) => {
                            // 父目录也不存在，用规范化的绝对路径做最佳判断
                            normalize_path(&absolute)
                        }
                    }
                } else {
                    return false;
                }
            }
        };

        // 检查是否在任一安全目录下
        for safe_dir in &self.safe_dirs {
            if canonical.starts_with(safe_dir) {
                return true;
            }
        }
        false
    }

    /// 生成沙箱外路径的提示信息
    pub fn outside_message(&self, tool_name: &str, arguments: &str) -> String {
        let paths = extract_paths(tool_name, arguments);
        let outside: Vec<String> = paths
            .into_iter()
            .filter(|p| !self.is_path_safe(p))
            .collect();
        format!(
            "⚠️ {} 正在操作当前目录以外的文件: {}",
            tool_name,
            outside.join(", "),
        )
    }
}

/// 从工具参数中提取所有路径
fn extract_paths(tool_name: &str, arguments: &str) -> Vec<String> {
    let parsed: Value = match serde_json::from_str(arguments) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut paths = Vec::new();

    match tool_name {
        // 文件工具：提取 path / file_path
        "Read" | "Write" | "Edit" | "Glob" | "Grep" => {
            if let Some(p) = parsed
                .get("path")
                .or_else(|| parsed.get("file_path"))
                .and_then(|v| v.as_str())
            {
                paths.push(p.to_string());
            }
        }
        // Shell 工具：提取 cwd 参数
        "Shell" => {
            if let Some(cwd) = parsed.get("cwd").and_then(|v| v.as_str()) {
                paths.push(cwd.to_string());
            }
        }
        _ => {}
    }

    paths
}

/// 规范化路径：消除 `.` 和 `..` 组件（不依赖文件系统）
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                components.pop();
            }
            Component::CurDir => {}
            _ => components.push(comp),
        }
    }
    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_in_sandbox() {
        let sandbox = Sandbox::new(); // cwd 为安全目录
        let cwd = env::current_dir().unwrap();

        // cwd 下的文件应该是安全的
        let child = cwd.join("src/main.rs");
        assert!(sandbox.is_path_safe(&child.display().to_string()));

        // 相对路径也应该安全
        assert!(sandbox.is_path_safe("src/main.rs"));
        assert!(sandbox.is_path_safe("./src/main.rs"));
    }

    #[test]
    fn test_path_outside_sandbox() {
        let sandbox = Sandbox::new();
        // /etc 不应该在项目目录内
        assert!(!sandbox.is_path_safe("/etc/passwd"));
        assert!(!sandbox.is_path_safe("/tmp/something"));
    }

    #[test]
    fn test_path_traversal_blocked() {
        let sandbox = Sandbox::new();
        // 使用 .. 尝试逃逸
        assert!(!sandbox.is_path_safe("../../../etc/passwd"));
    }

    #[test]
    fn test_extract_paths_read() {
        let args = r#"{"path": "/Users/test/file.rs"}"#;
        let paths = extract_paths("Read", args);
        assert_eq!(paths, vec!["/Users/test/file.rs"]);
    }

    #[test]
    fn test_extract_paths_bash_cwd() {
        let args = r#"{"command": "ls", "cwd": "/tmp"}"#;
        let paths = extract_paths("Shell", args);
        assert_eq!(paths, vec!["/tmp"]);
    }

    #[test]
    fn test_extract_paths_bash_no_cwd() {
        let args = r#"{"command": "cargo build"}"#;
        let paths = extract_paths("Shell", args);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_is_outside() {
        let sandbox = Sandbox::new();
        let args = r#"{"path": "/etc/passwd"}"#;
        assert!(sandbox.is_outside("Read", args));

        let args2 = r#"{"path": "src/main.rs"}"#;
        assert!(!sandbox.is_outside("Read", args2));
    }

    #[test]
    fn test_normalize_path() {
        let path = Path::new("/a/b/../c/./d");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/a/c/d"));
    }
}
