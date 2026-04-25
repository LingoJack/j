use crate::command::chat::constants::WORKTREE_NAME_MAX_LEN;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

// ========== Worktree Session State ==========

/// 当前 worktree 会话信息
#[derive(Clone, Debug)]
pub struct WorktreeSession {
    /// 进入 worktree 前的工作目录
    pub original_cwd: PathBuf,
    /// worktree 路径
    pub worktree_path: PathBuf,
    /// worktree 分支名
    pub branch: String,
    /// 进入时的 HEAD commit（用于检测新 commits）
    pub original_head_commit: Option<String>,
}

/// 跨工具共享的 worktree 状态
#[derive(Debug)]
pub struct WorktreeState {
    session: Mutex<Option<WorktreeSession>>,
}

impl Default for WorktreeState {
    fn default() -> Self {
        Self::new()
    }
}

impl WorktreeState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
        }
    }

    pub fn get_session(&self) -> Option<WorktreeSession> {
        self.session.lock().ok()?.clone()
    }

    pub fn set_session(&self, session: WorktreeSession) {
        if let Ok(mut s) = self.session.lock() {
            *s = Some(session);
        }
    }

    /// 清除当前会话并返回被清除的 WorktreeSession
    pub fn clear_session(&self) -> Option<WorktreeSession> {
        self.session.lock().ok()?.take()
    }
}

// ========== Helpers ==========

/// 获取 git 仓库根目录
fn git_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("执行 git 失败: {}", e))?;
    if !output.status.success() {
        return Err("当前目录不在 git 仓库中".to_string());
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

/// 获取当前 HEAD commit SHA
fn head_commit() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// 验证 worktree 名称
fn validate_slug(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("名称不能为空".to_string());
    }
    if name.len() > WORKTREE_NAME_MAX_LEN {
        return Err(format!("名称不能超过 {WORKTREE_NAME_MAX_LEN} 个字符"));
    }
    if name.contains("..") {
        return Err("名称不能包含 '..'".to_string());
    }
    for ch in name.chars() {
        if !ch.is_alphanumeric() && ch != '.' && ch != '_' && ch != '-' {
            return Err(format!("名称包含非法字符: '{}'", ch));
        }
    }
    Ok(())
}

/// 生成随机 slug
fn random_slug() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("wt-{:x}", ts & 0xFFFFFF)
}

/// 统计 worktree 中的变更
fn count_changes(worktree_path: &str, original_head: Option<&str>) -> (usize, usize) {
    // 未提交文件数
    let changed_files = Command::new("git")
        .args(["-C", worktree_path, "status", "--porcelain"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count()
        })
        .unwrap_or(0);

    // 新 commits 数
    let commits = original_head
        .and_then(|base| {
            Command::new("git")
                .args([
                    "-C",
                    worktree_path,
                    "rev-list",
                    "--count",
                    &format!("{}..HEAD", base),
                ])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| {
                    String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .parse::<usize>()
                        .unwrap_or(0)
                })
        })
        .unwrap_or(0);

    (changed_files, commits)
}

// ========== Agent Worktree Helpers ==========
// 供 TeammateTool / AgentTool 调用，自动为并行 agent 创建/删除 worktree

/// 为 agent 创建专用 worktree。
/// - `agent_name`: 用于生成目录名和分支名（会被 slug 化）
/// - 返回 `(worktree_path, branch_name)`
pub fn create_agent_worktree(agent_name: &str) -> Result<(PathBuf, String), String> {
    let repo_root = git_root()?;

    // slug 化：只保留字母数字、连字符、下划线
    let slug: String = agent_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = format!("agent-{}", slug);
    let branch = format!("worktree-{}", slug);
    let wt_path = repo_root.join(".jcli").join("worktrees").join(&slug);

    // 如果 worktree 目录已存在，直接复用
    if wt_path.exists() {
        return Ok((wt_path, branch));
    }

    let worktrees_dir = repo_root.join(".jcli").join("worktrees");
    std::fs::create_dir_all(&worktrees_dir)
        .map_err(|e| format!("创建 worktrees 目录失败: {}", e))?;

    let output = Command::new("git")
        .current_dir(&repo_root)
        .args([
            "worktree",
            "add",
            "-B",
            &branch,
            &wt_path.to_string_lossy(),
            "HEAD",
        ])
        .output()
        .map_err(|e| format!("执行 git worktree add 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("创建 worktree 失败: {}", stderr.trim()));
    }

    Ok((wt_path, branch))
}

/// 删除 agent worktree（最大努力，忽略错误）
pub fn remove_agent_worktree(worktree_path: &std::path::Path, branch: &str) {
    let wt_str = worktree_path.to_string_lossy().to_string();
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force", &wt_str])
        .output();
    // 等 git 释放内部锁
    std::thread::sleep(std::time::Duration::from_millis(200));
    let _ = Command::new("git").args(["branch", "-D", branch]).output();
}

// ========== EnterWorktreeTool ==========

#[derive(Deserialize, JsonSchema)]
struct EnterWorktreeParams {
    /// Optional name for the worktree. Only letters, digits, dots, underscores, dashes allowed; max WORKTREE_NAME_MAX_LEN chars. A random name is generated if not provided.
    #[serde(default)]
    name: Option<String>,
}

/// 进入工作树工具，创建隔离的 git worktree 并切换会话到其中
#[derive(Debug)]
pub struct EnterWorktreeTool {
    /// 跨工具共享的 worktree 状态
    pub state: Arc<WorktreeState>,
}

impl EnterWorktreeTool {
    pub const NAME: &'static str = "EnterWorktree";
}

impl Tool for EnterWorktreeTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Creates an isolated git worktree and switches the session into it.
        Use this when you need to work on code in isolation — for example, when multiple
        sessions may be editing the same repository simultaneously.

        The worktree is created at .jcli/worktrees/{name} under the git root,
        with a branch named worktree-{name}.

        Use ExitWorktree to leave the worktree (keep or remove it).
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<EnterWorktreeParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: EnterWorktreeParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        // 检查是否已在 worktree 中
        if self.state.get_session().is_some() {
            return ToolResult {
                output: "已在 worktree 会话中，请先使用 ExitWorktree 退出".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 获取 git 根目录
        let repo_root = match git_root() {
            Ok(r) => r,
            Err(e) => {
                return ToolResult {
                    output: e,
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let slug = params.name.unwrap_or_else(random_slug);
        if let Err(e) = validate_slug(&slug) {
            return ToolResult {
                output: format!("无效的 worktree 名称: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        let branch = format!("worktree-{}", slug);
        let wt_path = repo_root.join(".jcli").join("worktrees").join(&slug);

        // 如果目录已存在，说明 worktree 可能已存在
        if wt_path.exists() {
            return ToolResult {
                output: format!(
                    "Worktree 目录已存在: {}。请使用其他名称或先手动清理。",
                    wt_path.display()
                ),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 确保 .jcli/worktrees 目录存在
        let worktrees_dir = repo_root.join(".jcli").join("worktrees");
        if let Err(e) = std::fs::create_dir_all(&worktrees_dir) {
            return ToolResult {
                output: format!("创建 worktrees 目录失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 记录原始工作目录和 HEAD
        let original_cwd = std::env::current_dir().unwrap_or_default();
        let orig_head = head_commit();

        // 创建 worktree: git worktree add -B <branch> <path> HEAD
        let output = Command::new("git")
            .current_dir(&repo_root)
            .args([
                "worktree",
                "add",
                "-B",
                &branch,
                &wt_path.to_string_lossy(),
                "HEAD",
            ])
            .output();

        match output {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                return ToolResult {
                    output: format!("创建 worktree 失败: {}", stderr.trim()),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
            Err(e) => {
                return ToolResult {
                    output: format!("执行 git worktree add 失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        }

        // 切换工作目录
        if let Err(e) = std::env::set_current_dir(&wt_path) {
            return ToolResult {
                output: format!("切换到 worktree 目录失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 保存会话状态
        self.state.set_session(WorktreeSession {
            original_cwd,
            worktree_path: wt_path.clone(),
            branch: branch.clone(),
            original_head_commit: orig_head,
        });

        ToolResult {
            output: format!(
                "已创建并进入 worktree:\n  路径: {}\n  分支: {}\n\n当前会话在隔离的工作目录中，所有文件操作不会影响主仓库。\n完成后使用 ExitWorktree 退出（可选择保留或删除）。",
                wt_path.display(),
                branch,
            ),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        let name = serde_json::from_str::<EnterWorktreeParams>(arguments)
            .ok()
            .and_then(|p| p.name)
            .unwrap_or_else(|| "(auto)".to_string());
        format!("创建并进入 git worktree: {}", name)
    }
}

// ========== ExitWorktreeTool ==========

#[derive(Deserialize, JsonSchema)]
struct ExitWorktreeParams {
    /// "keep" preserves the worktree and branch on disk; "remove" deletes both.
    action: String,
    /// Required true when action is "remove" and the worktree has uncommitted files or unmerged commits.
    #[serde(default)]
    discard_changes: bool,
}

/// 退出工作树工具，退出当前 worktree 会话并选择保留或删除
#[derive(Debug)]
pub struct ExitWorktreeTool {
    /// 跨工具共享的 worktree 状态
    pub state: Arc<WorktreeState>,
}

impl ExitWorktreeTool {
    pub const NAME: &'static str = "ExitWorktree";
}

impl Tool for ExitWorktreeTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Exit the current worktree session created by EnterWorktree.
        - action "keep": preserves the worktree directory and branch for later use
        - action "remove": deletes the worktree and its branch (requires discard_changes: true if there are uncommitted changes or new commits)
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<ExitWorktreeParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: ExitWorktreeParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let session = match self.state.get_session() {
            Some(s) => s,
            None => {
                return ToolResult {
                    output: "当前不在 worktree 会话中（仅对 EnterWorktree 创建的 worktree 有效）"
                        .to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let wt_path_str = session.worktree_path.to_string_lossy().to_string();

        match params.action.as_str() {
            "keep" => {
                // 切回原目录
                if let Err(e) = std::env::set_current_dir(&session.original_cwd) {
                    return ToolResult {
                        output: format!("切换回原目录失败: {}", e),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
                self.state.clear_session();

                ToolResult {
                    output: format!(
                        "已退出 worktree，工作已保留:\n  路径: {}\n  分支: {}\n\n已切回原目录: {}",
                        wt_path_str,
                        session.branch,
                        session.original_cwd.display(),
                    ),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            "remove" => {
                // 检查变更
                let (changed_files, commits) =
                    count_changes(&wt_path_str, session.original_head_commit.as_deref());

                if (changed_files > 0 || commits > 0) && !params.discard_changes {
                    let mut parts = Vec::new();
                    if changed_files > 0 {
                        parts.push(format!("{} 个未提交的文件", changed_files));
                    }
                    if commits > 0 {
                        parts.push(format!("{} 个新 commit", commits));
                    }
                    return ToolResult {
                        output: format!(
                            "Worktree 中有 {}。删除将永久丢弃这些工作。\n请向用户确认后，使用 discard_changes: true 重新调用；或使用 action: \"keep\" 保留 worktree。",
                            parts.join(" 和 "),
                        ),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }

                // 切回原目录
                if let Err(e) = std::env::set_current_dir(&session.original_cwd) {
                    return ToolResult {
                        output: format!("切换回原目录失败: {}", e),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }

                // 删除 worktree
                let remove_result = Command::new("git")
                    .args(["worktree", "remove", "--force", &wt_path_str])
                    .output();

                let mut messages = Vec::new();

                match remove_result {
                    Ok(o) if o.status.success() => {
                        messages.push(format!("已删除 worktree: {}", wt_path_str));
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        messages.push(format!("删除 worktree 警告: {}", stderr.trim()));
                        // 尝试强制删除目录
                        let _ = std::fs::remove_dir_all(&session.worktree_path);
                    }
                    Err(e) => {
                        messages.push(format!("执行 git worktree remove 失败: {}", e));
                    }
                }

                // 等待 git 释放锁
                std::thread::sleep(std::time::Duration::from_millis(100));

                // 删除分支
                let branch_result = Command::new("git")
                    .args(["branch", "-D", &session.branch])
                    .output();

                match branch_result {
                    Ok(o) if o.status.success() => {
                        messages.push(format!("已删除分支: {}", session.branch));
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        messages.push(format!("删除分支警告: {}", stderr.trim()));
                    }
                    Err(_) => {}
                }

                self.state.clear_session();

                let mut output = messages.join("\n");
                if changed_files > 0 || commits > 0 {
                    output.push_str(&format!(
                        "\n已丢弃 {} 个未提交文件和 {} 个 commit。",
                        changed_files, commits
                    ));
                }
                output.push_str(&format!(
                    "\n已切回原目录: {}",
                    session.original_cwd.display()
                ));

                ToolResult {
                    output,
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            other => ToolResult {
                output: format!(
                    "无效的 action: \"{}\"，只支持 \"keep\" 或 \"remove\"",
                    other
                ),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        let action = serde_json::from_str::<ExitWorktreeParams>(arguments)
            .ok()
            .map(|p| p.action)
            .unwrap_or_else(|| "?".to_string());
        match action.as_str() {
            "keep" => "退出 worktree（保留工作目录和分支）".to_string(),
            "remove" => "退出并删除 worktree（包括工作目录和分支）".to_string(),
            _ => format!("退出 worktree (action: {})", action),
        }
    }
}
