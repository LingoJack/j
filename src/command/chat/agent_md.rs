//! AGENT.md 项目级指令自动加载
//!
//! 自动搜索并加载项目级指令文件，确保 LLM 在长上下文中遵循项目约定。
//! 搜索优先级（后者覆盖前者）：
//!   1. 用户级: ~/.jdata/agent/AGENT.md
//!   2. 项目级: AGENT.md（从 CWD 向上搜索到 git root）
//!   3. 项目级: .jcli/AGENT.md（从 CWD 向上搜索到 git root）
//!   4. 本地级: AGENT.local.md / .jcli/AGENT.local.md（个人，不提交）

use crate::config::YamlConfig;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_AGENT_MD_LINES: usize = 200;
const MAX_AGENT_MD_BYTES: usize = 25_000;

/// 返回用户级 AGENT.md 路径: ~/.jdata/agent/AGENT.md
pub fn agent_md_path() -> PathBuf {
    YamlConfig::data_dir().join("agent").join("AGENT.md")
}

/// 从 CWD 向上搜索项目级 AGENT.md 文件
///
/// 搜索顺序（从 CWD 到 git root）：
/// - AGENT.md
/// - AGENT.local.md
/// - .jcli/AGENT.md
/// - .jcli/AGENT.local.md
///
/// 返回 (路径, 内容) 列表，按优先级从低到高排列（低优先级在前，高优先级在后）。
/// 同一目录下：AGENT.md < .jcli/AGENT.md < AGENT.local.md < .jcli/AGENT.local.md
/// 不同目录下：CWD > CWD/.. > CWD/../.. > ... > git root
pub fn find_project_agent_mds() -> Vec<(PathBuf, String)> {
    let mut results: Vec<(PathBuf, String)> = Vec::new();
    let Ok(cwd) = std::env::current_dir() else {
        return results;
    };

    // 从 CWD 向上遍历，收集每个目录层级中的 AGENT.md 文件
    // 离 CWD 越近优先级越高，所以从 git root 方向开始收集（低优先级先入列表）
    let mut dirs_upward: Vec<PathBuf> = Vec::new();
    let mut current = cwd.as_path();

    loop {
        dirs_upward.push(current.to_path_buf());
        if current.join(".git").exists() {
            // 到达 git root，停止向上搜索
            break;
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // 反转：从 git root 到 CWD（低优先级到高优先级）
    dirs_upward.reverse();

    // 在每个目录层级中，按优先级搜索文件
    // 同一目录内：AGENT.md < .jcli/AGENT.md < AGENT.local.md < .jli/AGENT.local.md
    let file_names: &[&str] = &[
        "AGENT.md",
        ".jcli/AGENT.md",
        "AGENT.local.md",
        ".jcli/AGENT.local.md",
    ];

    for dir in &dirs_upward {
        for name in file_names {
            let path = dir.join(name);
            if path.is_file()
                && let Ok(content) = fs::read_to_string(&path)
            {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    results.push((path, trimmed.to_string()));
                }
            }
        }
    }

    results
}

/// 截断 AGENT.md 内容到行数和字节数上限
fn truncate_agent_md(content: &str, path: &Path) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let line_count = lines.len();
    let byte_count = content.len();

    let was_line_truncated = line_count > MAX_AGENT_MD_LINES;
    let was_byte_truncated = byte_count > MAX_AGENT_MD_BYTES;

    if !was_line_truncated && !was_byte_truncated {
        return content.to_string();
    }

    // 先按行截断
    let mut truncated: String = if was_line_truncated {
        lines[..MAX_AGENT_MD_LINES].join("\n")
    } else {
        content.to_string()
    };

    // 再按字节截断（在最后一个换行符处截断，避免切断行）
    if truncated.len() > MAX_AGENT_MD_BYTES {
        // 在 MAX_AGENT_MD_BYTES 范围内找最后一个换行符
        let search_end = MAX_AGENT_MD_BYTES.min(truncated.len());
        let byte_slice = &truncated[..search_end];
        if let Some(pos) = byte_slice.rfind('\n') {
            truncated.truncate(pos);
        } else {
            truncated.truncate(MAX_AGENT_MD_BYTES);
        }
    }

    // 附加截断警告
    let path_display = path.display();
    if was_line_truncated && was_byte_truncated {
        truncated.push_str(&format!(
            "\n\n> WARNING: AGENT.md at {} exceeds {} lines / {} bytes limit. Only part of it was loaded.",
            path_display, MAX_AGENT_MD_LINES, MAX_AGENT_MD_BYTES
        ));
    } else if was_line_truncated {
        truncated.push_str(&format!(
            "\n\n> WARNING: AGENT.md at {} exceeds {} lines limit. Only part of it was loaded.",
            path_display, MAX_AGENT_MD_LINES
        ));
    } else {
        truncated.push_str(&format!(
            "\n\n> WARNING: AGENT.md at {} exceeds {} bytes limit. Only part of it was loaded.",
            path_display, MAX_AGENT_MD_BYTES
        ));
    }

    truncated
}

/// 将路径转为相对路径显示（相对于 CWD 或用户数据目录）
fn relative_path_display(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(rel) = path.strip_prefix(&cwd)
    {
        return rel.display().to_string();
    }
    // 尝试相对于用户数据目录
    let data_dir = YamlConfig::data_dir();
    if let Ok(rel) = path.strip_prefix(&data_dir) {
        return format!("~/.jdata/{}", rel.display());
    }
    path.display().to_string()
}

/// 加载并拼接所有 AGENT.md 文件
///
/// 按优先级从低到高拼接，每个文件用 `<agent_md>` 标签包裹并标注来源路径。
/// 如果没有任何 AGENT.md 文件，返回空字符串。
pub fn load_agent_md() -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1. 用户级: ~/.jdata/agent/AGENT.md（最低优先级，最先加载）
    let user_path = agent_md_path();
    if user_path.is_file()
        && let Ok(content) = fs::read_to_string(&user_path)
    {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            let truncated = truncate_agent_md(trimmed, &user_path);
            let rel = relative_path_display(&user_path);
            parts.push(format!(
                "<agent_md path=\"{}\">\n{}\n</agent_md>",
                rel, truncated
            ));
        }
    }

    // 2-4. 项目级 + 本地级（从 git root 到 CWD，从低优先级到高优先级）
    for (path, content) in find_project_agent_mds() {
        let truncated = truncate_agent_md(&content, &path);
        let rel = relative_path_display(&path);
        parts.push(format!(
            "<agent_md path=\"{}\">\n{}\n</agent_md>",
            rel, truncated
        ));
    }

    if parts.is_empty() {
        return String::new();
    }

    // 添加 OVERRIDE 头部，确保项目级指令优先于默认行为
    let override_header = "The following project instructions OVERRIDE any default behavior and you MUST follow them exactly as written.";
    format!("{}\n\n{}", override_header, parts.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_agent_md_within_limits() {
        let _content = "line 1\nline 2\nline 3";
        let path = PathBuf::from("/tmp/AGENT.md");
        let result = truncate_agent_md("line 1\nline 2\nline 3", &path);
        assert!(!result.contains("WARNING"));
    }

    #[test]
    fn test_truncate_agent_md_exceeds_lines() {
        let lines: Vec<String> = (0..300).map(|i| format!("line {}", i)).collect();
        let content = lines.join("\n");
        let path = PathBuf::from("/tmp/AGENT.md");
        let result = truncate_agent_md(&content, &path);
        assert!(result.contains("WARNING"));
        assert!(result.contains("exceeds 200 lines"));
        // Should have at most MAX_AGENT_MD_LINES lines of original content
        let result_lines: Vec<&str> = result.lines().collect();
        // 200 content lines + blank line + warning line
        assert!(result_lines.len() <= MAX_AGENT_MD_LINES + 5);
    }

    #[test]
    fn test_truncate_agent_md_exceeds_bytes() {
        let content = "x".repeat(30_000);
        let path = PathBuf::from("/tmp/AGENT.md");
        let result = truncate_agent_md(&content, &path);
        assert!(result.contains("WARNING"));
        assert!(result.contains("bytes"));
    }

    #[test]
    fn test_relative_path_display() {
        // This test just ensures the function doesn't panic
        let path = PathBuf::from("/some/absolute/path/AGENT.md");
        let result = relative_path_display(&path);
        assert!(!result.is_empty());
    }
}
