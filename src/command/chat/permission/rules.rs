use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// .jcli/ 目录权限配置
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct JcliConfig {
    #[serde(default)]
    pub permissions: PermissionConfig,
}

/// 权限配置
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct PermissionConfig {
    /// 完全放开（跳过所有工具确认）
    #[serde(default)]
    pub allow_all: bool,
    /// 允许列表（匹配则跳过确认）
    #[serde(default)]
    pub allow: Vec<String>,
    /// 拒绝列表（优先于 allow，匹配则直接拒绝执行）
    #[serde(default)]
    pub deny: Vec<String>,
}

impl JcliConfig {
    /// 从 cwd 向上查找 .jcli/ 目录并加载 permissions.yaml
    pub fn load() -> Self {
        if let Some(dir) = Self::find_config_dir() {
            let perm_path = dir.join("permissions.yaml");
            match std::fs::read_to_string(&perm_path) {
                Ok(content) => {
                    let permissions =
                        serde_yaml::from_str::<PermissionConfig>(&content).unwrap_or_default();
                    JcliConfig { permissions }
                }
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    /// 从当前目录向上查找 .jcli/ 目录
    pub fn find_config_dir() -> Option<PathBuf> {
        let mut dir = std::env::current_dir().ok()?;
        loop {
            let candidate = dir.join(".jcli");
            if candidate.is_dir() {
                return Some(candidate);
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    /// 确保 cwd 下存在 .jcli/ 目录，返回该目录路径
    pub fn ensure_config_dir() -> Option<PathBuf> {
        let dir = std::env::current_dir().ok()?.join(".jcli");
        let _ = std::fs::create_dir_all(&dir);

        // 创建 hooks/example 目录和 HOOK.yaml.example 模板（仅在首次创建时）
        let example_dir = dir.join("hooks").join("example");
        if !example_dir.exists() {
            let _ = std::fs::create_dir_all(&example_dir);
            let example_yaml = example_dir.join("HOOK.yaml.example");
            if !example_yaml.exists() {
                const HOOK_YAML_EXAMPLE: &str =
                    include_str!("../../../../assets/hook_yaml_example.yaml");
                let _ = std::fs::write(&example_yaml, HOOK_YAML_EXAMPLE);
            }
        }

        Some(dir)
    }

    /// 检查某个工具调用是否被自动允许（跳过确认）
    ///
    /// - tool_name: "Bash", "Read", "Write" 等
    /// - arguments: JSON 字符串（用于提取 command/path 等）
    ///
    /// 返回 true 表示该调用无需用户确认
    pub fn is_allowed(&self, tool_name: &str, arguments: &str) -> bool {
        // 先检查 deny（deny 优先于 allow）
        if self.is_denied(tool_name, arguments) {
            return false;
        }

        // allow_all 模式
        if self.permissions.allow_all {
            return true;
        }

        // 逐条匹配 allow 列表
        for rule in &self.permissions.allow {
            if matches_rule(rule, tool_name, arguments) {
                return true;
            }
        }

        false
    }

    /// 检查是否被 deny 列表拦截（deny 中匹配则直接拒绝执行）
    pub fn is_denied(&self, tool_name: &str, arguments: &str) -> bool {
        for rule in &self.permissions.deny {
            if matches_rule(rule, tool_name, arguments) {
                return true;
            }
        }
        false
    }

    /// 将一条 allow 规则追加到 .jcli/permissions.yaml（若目录/文件不存在则创建）
    /// 去重：如果 allow 列表已包含该规则则不重复添加
    pub fn add_allow_rule(&mut self, rule: &str) {
        // 去重
        if self.permissions.allow.contains(&rule.to_string()) {
            return;
        }

        // 更新内存
        self.permissions.allow.push(rule.to_string());

        // 确保 .jcli/ 目录存在
        let config_dir = match Self::ensure_config_dir() {
            Some(dir) => dir,
            None => return,
        };
        let perm_path = config_dir.join("permissions.yaml");

        // 如果文件已存在，尝试加载已有内容再追加
        let mut permissions = if perm_path.is_file() {
            match std::fs::read_to_string(&perm_path) {
                Ok(content) => {
                    serde_yaml::from_str::<PermissionConfig>(&content).unwrap_or_default()
                }
                Err(_) => PermissionConfig::default(),
            }
        } else {
            PermissionConfig::default()
        };

        if !permissions.allow.contains(&rule.to_string()) {
            permissions.allow.push(rule.to_string());
        }

        if let Ok(yaml) = serde_yaml::to_string(&permissions) {
            let _ = std::fs::write(&perm_path, yaml);
        }
    }
}

/// 匹配单条规则
///
/// 支持的格式：
/// - `"*"` → 匹配所有工具所有调用
/// - `"Read"` → 匹配该工具所有调用（工具名不带括号）
/// - `"Bash(cargo build:*)"` → Bash 命令前缀匹配
/// - `"Write(path:/foo/bar/*)"` → 文件路径前缀匹配
/// - `"WebFetch(domain:docs.rs)"` → URL 域名匹配
fn matches_rule(rule: &str, tool_name: &str, arguments: &str) -> bool {
    let rule = rule.trim();

    // 通配符：匹配所有
    if rule == "*" {
        return true;
    }

    // 带括号的规则：ToolName(condition)
    if let Some(paren_start) = rule.find('(') {
        if !rule.ends_with(')') {
            return false;
        }
        let rule_tool = &rule[..paren_start];
        if rule_tool != tool_name {
            return false;
        }
        let condition = &rule[paren_start + 1..rule.len() - 1];
        return match_condition(tool_name, condition, arguments);
    }

    // 不带括号：纯工具名，匹配该工具所有调用
    rule == tool_name
}

/// 匹配条件部分
///
/// - `"cargo build:*"` → Bash 命令前缀（取 arguments.command）
/// - `"path:/foo/*"` → 文件路径前缀（取 arguments.file_path）
/// - `"domain:docs.rs"` → URL 域名（取 arguments.url）
/// - 支持 regex: `/pattern/` 语法，如 `"path:/\.rs$/"`, `"/^cargo (build|test)/"`
/// - `"domain:/.*\.google\.com$/"` → regex 域名匹配
fn match_condition(tool_name: &str, condition: &str, arguments: &str) -> bool {
    let parsed: Value = match serde_json::from_str(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // path: 前缀 → 文件路径匹配（Write, Edit, Read, Glob, Grep）
    if let Some(path_pattern) = condition.strip_prefix("path:") {
        let file_path = parsed
            .get("file_path")
            .or_else(|| parsed.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if is_regex_pattern(path_pattern) {
            return match_regex(path_pattern, file_path);
        }
        return match_glob_prefix(path_pattern, file_path);
    }

    // domain: 前缀 → URL 域名匹配（WebFetch, WebSearch）
    if let Some(domain) = condition.strip_prefix("domain:") {
        let url = parsed.get("url").and_then(|v| v.as_str()).unwrap_or("");
        if is_regex_pattern(domain) {
            // 提取 host 后对 host 做 regex 匹配
            let host = extract_host(url);
            return match_regex(domain, &host);
        }
        return url_matches_domain(url, domain);
    }

    // ComputerUse: action 前缀匹配（格式 "action:screenshot:*" 或 "screenshot:*"）
    if tool_name == "ComputerUse" {
        let action = parsed.get("action").and_then(|v| v.as_str()).unwrap_or("");
        // 支持 "action:screenshot:*" 和 "screenshot:*" 两种格式
        let action_pattern = if let Some(rest) = condition.strip_prefix("action:") {
            rest
        } else {
            condition
        };
        if is_regex_pattern(action_pattern) {
            return match_regex(action_pattern, action);
        }
        return match_command_prefix(action_pattern, action);
    }

    // 默认：Bash 命令前缀匹配（格式 "command_prefix:*"）
    if tool_name == "Bash" || tool_name == "Shell" {
        let command = parsed.get("command").and_then(|v| v.as_str()).unwrap_or("");
        if is_regex_pattern(condition) {
            return match_regex(condition, command);
        }
        return match_command_prefix(condition, command);
    }

    false
}

// ========== Regex 辅助函数 ==========

/// 全局 regex 编译缓存（避免重复编译同一正则表达式）
static REGEX_CACHE: LazyLock<Mutex<HashMap<String, Regex>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 判断是否为 `/pattern/` 格式的 regex 模式
fn is_regex_pattern(pattern: &str) -> bool {
    pattern.starts_with('/') && pattern.ends_with('/') && pattern.len() >= 2
}

/// 用 `/pattern/` 匹配 input，带编译缓存
/// 返回 regex 是否匹配（`is_match` 语义）
fn match_regex(pattern: &str, input: &str) -> bool {
    // 去掉首尾的 /
    let regex_str = &pattern[1..pattern.len() - 1];
    if regex_str.is_empty() {
        return false;
    }

    let mut cache = match REGEX_CACHE.lock() {
        Ok(c) => c,
        Err(poisoned) => poisoned.into_inner(),
    };

    let re = cache
        .entry(regex_str.to_string())
        .or_insert_with(|| match Regex::new(regex_str) {
            Ok(r) => r,
            // 永不匹配的兜底正则：编译失败说明正则语法错误，此静态模式不可能失败
            Err(_) => Regex::new("^$").expect("静态正则 ^$ 不可能编译失败"),
        });

    re.is_match(input)
}

/// 从 URL 中提取 host 部分
fn extract_host(url: &str) -> String {
    let url_lower = url.to_lowercase();
    let after_scheme = if let Some(pos) = url_lower.find("://") {
        &url_lower[pos + 3..]
    } else {
        &url_lower
    };
    after_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_string()
}

/// Bash 命令前缀匹配
///
/// 规则格式：`"cargo build:*"` 表示命令以 "cargo build" 开头
/// 也支持 `"ls"` 不带 `:*` 后缀的精确前缀匹配
fn match_command_prefix(pattern: &str, command: &str) -> bool {
    // 去掉尾部的 `:*` 通配符
    let prefix = pattern.strip_suffix(":*").unwrap_or(pattern).trim();

    let command = command.trim();
    // 前缀匹配：命令以 prefix 开头，后续要么是空、要么是空格/参数
    if command == prefix {
        return true;
    }
    if let Some(rest) = command.strip_prefix(prefix) {
        return rest.starts_with(' ') || rest.starts_with('\t');
    }
    false
}

/// 简单的 glob 前缀匹配（支持尾部 `*` 通配符）
///
/// - `/foo/bar/*` → 匹配 /foo/bar/ 下的所有文件
/// - `/foo/bar/baz.rs` → 精确匹配
fn match_glob_prefix(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    path == pattern
}

/// URL 域名匹配
fn url_matches_domain(url: &str, domain: &str) -> bool {
    // 简单解析：从 url 中提取 host
    let url_lower = url.to_lowercase();
    let domain_lower = domain.to_lowercase();

    // 跳过 scheme
    let after_scheme = if let Some(pos) = url_lower.find("://") {
        &url_lower[pos + 3..]
    } else {
        &url_lower
    };

    // 取 host 部分（到 / 或 : 为止）
    let host = after_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");

    host == domain_lower || host.ends_with(&format!(".{}", domain_lower))
}

/// 根据工具名和参数生成对应的 allow 规则
///
/// - Bash: 提取 command 字段的前两个词（如 `cargo build --release` → `Bash(cargo build:*)`）
/// - Write/Edit: 提取 file_path 所在目录 → `Write(path:/dir/*)`
/// - WebFetch: 提取 url 域名 → `WebFetch(domain:xxx)`
/// - 其他工具: 直接用工具名 → `"Read"`
pub fn generate_allow_rule(tool_name: &str, arguments: &str) -> String {
    let parsed: Value = serde_json::from_str(arguments).unwrap_or(Value::Null);

    match tool_name {
        "ComputerUse" => {
            let action = parsed.get("action").and_then(|v| v.as_str()).unwrap_or("");
            if !action.is_empty() {
                format!("ComputerUse({}:*)", action)
            } else {
                "ComputerUse".to_string()
            }
        }
        "Bash" | "Shell" => {
            let command = parsed.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let words: Vec<&str> = command.split_whitespace().collect();
            let prefix = if words.len() >= 2 {
                format!("{} {}", words[0], words[1])
            } else if words.len() == 1 {
                words[0].to_string()
            } else {
                return tool_name.to_string();
            };
            format!("{}({}:*)", tool_name, prefix)
        }
        "Write" | "Edit" => {
            let file_path = parsed
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(dir) = std::path::Path::new(file_path).parent() {
                format!("{}(path:{}/*)", tool_name, dir.display())
            } else {
                tool_name.to_string()
            }
        }
        "WebFetch" => {
            let url = parsed.get("url").and_then(|v| v.as_str()).unwrap_or("");
            // 提取域名
            let after_scheme = if let Some(pos) = url.find("://") {
                &url[pos + 3..]
            } else {
                url
            };
            let host = after_scheme
                .split('/')
                .next()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("");
            if !host.is_empty() {
                format!("WebFetch(domain:{})", host)
            } else {
                "WebFetch".to_string()
            }
        }
        _ => tool_name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_rule() {
        assert!(matches_rule("*", "Bash", "{}"));
        assert!(matches_rule("*", "Read", "{}"));
    }

    #[test]
    fn test_tool_name_rule() {
        assert!(matches_rule("Read", "Read", "{}"));
        assert!(matches_rule("Grep", "Grep", "{}"));
        assert!(!matches_rule("Read", "Write", "{}"));
    }

    #[test]
    fn test_bash_command_prefix() {
        let args = r#"{"command": "cargo build --release"}"#;
        assert!(matches_rule("Bash(cargo build:*)", "Bash", args));
        assert!(!matches_rule("Bash(cargo test:*)", "Bash", args));

        let args2 = r#"{"command": "ls -la"}"#;
        assert!(matches_rule("Bash(ls:*)", "Bash", args2));

        // 精确匹配（无参数）
        let args3 = r#"{"command": "cargo fmt"}"#;
        assert!(matches_rule("Bash(cargo fmt:*)", "Bash", args3));
    }

    #[test]
    fn test_path_rule() {
        let args = r#"{"file_path": "/Users/jack/projects/foo/bar.rs"}"#;
        assert!(matches_rule(
            "Write(path:/Users/jack/projects/*)",
            "Write",
            args
        ));
        assert!(!matches_rule("Write(path:/Users/other/*)", "Write", args));
    }

    #[test]
    fn test_domain_rule() {
        let args = r#"{"url": "https://docs.rs/serde/latest"}"#;
        assert!(matches_rule("WebFetch(domain:docs.rs)", "WebFetch", args));
        assert!(!matches_rule(
            "WebFetch(domain:github.com)",
            "WebFetch",
            args
        ));
    }

    #[test]
    fn test_deny_priority() {
        let config = JcliConfig {
            permissions: PermissionConfig {
                allow_all: false,
                allow: vec!["Bash(cargo:*)".to_string()],
                deny: vec!["Bash(cargo build:*)".to_string()],
            },
            ..Default::default()
        };
        // cargo test 被 allow 的 cargo:* 覆盖
        let args_test = r#"{"command": "cargo test"}"#;
        assert!(config.is_allowed("Bash", args_test));

        // cargo build 被 deny 拦截
        let args_build = r#"{"command": "cargo build"}"#;
        assert!(!config.is_allowed("Bash", args_build));
        assert!(config.is_denied("Bash", args_build));
    }

    #[test]
    fn test_allow_all() {
        let config = JcliConfig {
            permissions: PermissionConfig {
                allow_all: true,
                allow: vec![],
                deny: vec![],
            },
            ..Default::default()
        };
        assert!(config.is_allowed("Bash", r#"{"command": "rm -rf /"}"#));

        // deny 仍然优先
        let config2 = JcliConfig {
            permissions: PermissionConfig {
                allow_all: true,
                allow: vec![],
                deny: vec!["Bash(rm -rf:*)".to_string()],
            },
            ..Default::default()
        };
        assert!(!config2.is_allowed("Bash", r#"{"command": "rm -rf /"}"#));
    }

    #[test]
    fn test_url_domain_matching() {
        assert!(url_matches_domain("https://docs.rs/foo", "docs.rs"));
        assert!(url_matches_domain(
            "https://api.github.com/repos",
            "github.com"
        ));
        assert!(!url_matches_domain("https://evil.com/docs.rs", "docs.rs"));
    }

    #[test]
    fn test_generate_allow_rule_bash() {
        let args = r#"{"command": "cargo build --release"}"#;
        assert_eq!(generate_allow_rule("Bash", args), "Bash(cargo build:*)");

        let args2 = r#"{"command": "ls -la"}"#;
        assert_eq!(generate_allow_rule("Bash", args2), "Bash(ls -la:*)");

        let args3 = r#"{"command": "ls"}"#;
        assert_eq!(generate_allow_rule("Bash", args3), "Bash(ls:*)");
    }

    #[test]
    fn test_generate_allow_rule_write() {
        let args = r#"{"file_path": "/Users/jack/projects/foo/bar.rs"}"#;
        assert_eq!(
            generate_allow_rule("Write", args),
            "Write(path:/Users/jack/projects/foo/*)"
        );

        let args2 = r#"{"file_path": "/Users/jack/projects/foo/src/main.rs"}"#;
        assert_eq!(
            generate_allow_rule("Edit", args2),
            "Edit(path:/Users/jack/projects/foo/src/*)"
        );
    }

    #[test]
    fn test_generate_allow_rule_webfetch() {
        let args = r#"{"url": "https://docs.rs/serde/latest"}"#;
        assert_eq!(
            generate_allow_rule("WebFetch", args),
            "WebFetch(domain:docs.rs)"
        );
    }

    #[test]
    fn test_generate_allow_rule_other() {
        assert_eq!(generate_allow_rule("Read", "{}"), "Read");
        assert_eq!(generate_allow_rule("Grep", "{}"), "Grep");
        assert_eq!(generate_allow_rule("Glob", "{}"), "Glob");
    }

    // ========== Regex 匹配测试 ==========

    #[test]
    fn test_regex_bash_command() {
        // Bash(/^cargo (build|test)/) 匹配 cargo build 和 cargo test
        let args_build = r#"{"command": "cargo build --release"}"#;
        let args_test = r#"{"command": "cargo test --lib"}"#;
        let args_run = r#"{"command": "cargo run"}"#;
        assert!(matches_rule(
            "Bash(/^cargo (build|test)/)",
            "Bash",
            args_build
        ));
        assert!(matches_rule(
            "Bash(/^cargo (build|test)/)",
            "Bash",
            args_test
        ));
        assert!(!matches_rule(
            "Bash(/^cargo (build|test)/)",
            "Bash",
            args_run
        ));
    }

    #[test]
    fn test_regex_path_pattern() {
        // Write(path:/\.rs$/) 匹配所有 .rs 文件
        let args_rs = r#"{"file_path": "/Users/jack/projects/foo/bar.rs"}"#;
        let args_ts = r#"{"file_path": "/Users/jack/projects/foo/bar.ts"}"#;
        assert!(matches_rule("Write(path:/\\.rs$/)", "Write", args_rs));
        assert!(!matches_rule("Write(path:/\\.rs$/)", "Write", args_ts));
    }

    #[test]
    fn test_regex_domain_pattern() {
        // WebFetch(domain:/.*\.google\.com$/) 匹配所有 google.com 子域名
        let args_google = r#"{"url": "https://maps.google.com/api"}"#;
        let args_docs = r#"{"url": "https://docs.google.com/document"}"#;
        let args_other = r#"{"url": "https://evil.com/google.com"}"#;
        assert!(matches_rule(
            "WebFetch(domain:/.*\\.google\\.com$/)",
            "WebFetch",
            args_google
        ));
        assert!(matches_rule(
            "WebFetch(domain:/.*\\.google\\.com$/)",
            "WebFetch",
            args_docs
        ));
        assert!(!matches_rule(
            "WebFetch(domain:/.*\\.google\\.com$/)",
            "WebFetch",
            args_other
        ));
    }

    #[test]
    fn test_regex_invalid_pattern() {
        // 无效的 regex 不匹配
        let args = r#"{"command": "anything"}"#;
        assert!(!matches_rule("Bash(/[invalid/)", "Bash", args));
        // 空 regex 不匹配
        assert!(!matches_rule("Bash(//)", "Bash", args));
    }
}
