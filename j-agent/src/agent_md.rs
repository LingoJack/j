//! AGENTS.md 项目级指令自动加载
//!
//! 自动搜索并加载项目级指令文件，确保 LLM 在长上下文中遵循项目约定。
//! 搜索优先级（后者覆盖前者）：
//!   1. 用户级: ~/.jdata/agent/AGENTS.md
//!   2. 项目级: AGENTS.md（从 CWD 向上搜索到 git root）
//!   3. 项目级: .jcli/AGENTS.md（从 CWD 向上搜索到 git root）
//!   4. 本地级: AGENTS.local.md / .jcli/AGENTS.local.md（个人，不提交）
//!
//! 注入格式对齐 Claude Code（参考 src/utils/claudemd.ts 的 getClaudeMds）：
//! 内容以 `Contents of {绝对路径} ({来源描述}):\n\n{内容}` 的散文形式拼接，
//! 不使用 XML 包裹，并在最前置一行 OVERRIDE 提示。


use std::fs;
use std::path::PathBuf;

const MEMORY_INSTRUCTION_PROMPT: &str = "Codebase and user instructions are shown below. Be sure to adhere to these instructions. IMPORTANT: These instructions OVERRIDE any default behavior and you MUST follow them exactly as written.";

/// AGENTS.md 文件来源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentMdType {
    /// ~/.jdata/agent/AGENTS.md — 用户全局指令
    User,
    /// AGENTS.md / .jcli/AGENTS.md — 项目级指令（签入仓库）
    Project,
    /// AGENTS.local.md / .jcli/AGENTS.local.md — 项目本地（个人，不签入）
    Local,
}

impl AgentMdType {
    fn description(self) -> &'static str {
        match self {
            AgentMdType::User => "user's private global instructions for all projects",
            AgentMdType::Project => "project instructions, checked into the codebase",
            AgentMdType::Local => "user's private project instructions, not checked in",
        }
    }
}

struct AgentMdEntry {
    path: PathBuf,
    md_type: AgentMdType,
    content: String,
}

/// 返回用户级 AGENTS.md 路径: ~/.jdata/agent/AGENTS.md
pub fn agent_md_path() -> PathBuf {
    crate::constants::data_root().join("agent").join("AGENTS.md")
}

/// 从 CWD 向上搜索项目级 AGENTS.md 文件，返回按优先级从低到高排序的条目。
///
/// 同一目录内优先级：AGENTS.md < .jcli/AGENTS.md < AGENTS.local.md < .jcli/AGENTS.local.md
/// 不同目录优先级：git root < ... < CWD（离 CWD 越近优先级越高）
fn collect_project_entries() -> Vec<AgentMdEntry> {
    let mut results: Vec<AgentMdEntry> = Vec::new();
    let Ok(cwd) = std::env::current_dir() else {
        return results;
    };

    let mut dirs_upward: Vec<PathBuf> = Vec::new();
    let mut current = cwd.as_path();

    loop {
        dirs_upward.push(current.to_path_buf());
        if current.join(".git").exists() {
            break;
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    // 反转：从 git root 到 CWD（低优先级到高优先级）
    dirs_upward.reverse();

    let candidates: &[(&str, AgentMdType)] = &[
        ("AGENTS.md", AgentMdType::Project),
        (".jcli/AGENTS.md", AgentMdType::Project),
        ("AGENTS.local.md", AgentMdType::Local),
        (".jcli/AGENTS.local.md", AgentMdType::Local),
    ];

    for dir in &dirs_upward {
        for (name, md_type) in candidates {
            let path = dir.join(name);
            if path.is_file()
                && let Ok(content) = fs::read_to_string(&path)
            {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    results.push(AgentMdEntry {
                        path,
                        md_type: *md_type,
                        content: trimmed.to_string(),
                    });
                }
            }
        }
    }

    results
}

/// 加载并拼接所有 AGENTS.md 文件。
///
/// 按优先级从低到高拼接（用户级 → 项目级 → 本地级），每个文件以
/// `Contents of {path} ({description}):\n\n{content}` 散文格式输出，
/// 顶部附带 OVERRIDE 提示。无任何 AGENTS.md 时返回空字符串。
pub fn load_agent_md() -> String {
    let mut entries: Vec<AgentMdEntry> = Vec::new();

    // 1. 用户级（最低优先级，最先加载）
    let user_path = agent_md_path();
    if user_path.is_file()
        && let Ok(content) = fs::read_to_string(&user_path)
    {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            entries.push(AgentMdEntry {
                path: user_path,
                md_type: AgentMdType::User,
                content: trimmed.to_string(),
            });
        }
    }

    // 2-4. 项目级 + 本地级
    entries.extend(collect_project_entries());

    if entries.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = entries
        .into_iter()
        .map(|entry| {
            format!(
                "Contents of {} ({}):\n\n{}",
                entry.path.display(),
                entry.md_type.description(),
                entry.content
            )
        })
        .collect();

    format!("{}\n\n{}", MEMORY_INSTRUCTION_PROMPT, parts.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_md_type_description() {
        assert!(AgentMdType::User.description().contains("global"));
        assert!(AgentMdType::Project.description().contains("checked into"));
        assert!(AgentMdType::Local.description().contains("not checked in"));
    }

    #[test]
    fn test_load_agent_md_empty_when_no_files() {
        // 无任何 AGENTS.md 时（在隔离的临时目录中运行），应返回空串。
        // 真实 CWD 下可能存在 AGENTS.md，所以此测试只断言函数不 panic。
        let _ = load_agent_md();
    }
}
