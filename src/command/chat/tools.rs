pub mod ask;
pub mod background;
mod browser;
pub mod classification;
pub mod compact_tool;
#[cfg(target_os = "macos")]
mod computer_use;
pub mod definition;
pub mod derived_shared;
mod file;
mod grep;
pub mod hook;
pub mod ignore_message;
pub mod load_tool;
pub mod plan;
#[cfg(windows)]
mod powershell;
pub mod send_message;
mod shell;
pub mod skill;
pub mod sub_agent;
pub mod task;
pub mod teammate_tool;
pub mod todo;
mod web_fetch;
mod web_search;
pub mod work_done;
pub mod worktree;

pub use crate::util::path_utils::{effective_cwd, expand_tilde, resolve_path};
pub use crate::util::shell_safety::{check_blocking_command, is_dangerous_command};
pub use definition::{
    ImageData, PlanDecision, Tool, ToolRegistry, ToolResult, parse_tool_args, schema_to_tool_params,
};

/// 工具名称常量 — 所有工具的 NAME 统一在此导出，避免硬编码
#[allow(dead_code)] // 常量按需引用，非全部立即使用
pub mod tool_names {
    pub const BASH: &str = super::shell::ShellTool::NAME;
    #[cfg(windows)]
    pub const POWERSHELL: &str = super::powershell::PowerShellTool::NAME;
    pub const READ: &str = super::file::ReadFileTool::NAME;
    pub const WRITE: &str = super::file::WriteFileTool::NAME;
    pub const EDIT: &str = super::file::EditFileTool::NAME;
    pub const GLOB: &str = super::file::GlobTool::NAME;
    pub const GREP: &str = super::grep::GrepTool::NAME;
    pub const BROWSER: &str = super::browser::BrowserTool::NAME;
    pub const WEB_FETCH: &str = super::web_fetch::WebFetchTool::NAME;
    pub const WEB_SEARCH: &str = super::web_search::WebSearchTool::NAME;
    pub const ASK: &str = super::ask::AskTool::NAME;
    pub const TASK_OUTPUT: &str = super::background::TaskOutputTool::NAME;
    pub const TASK: &str = super::task::TaskTool::NAME;
    pub const TODO_WRITE: &str = super::todo::TodoWriteTool::NAME;
    pub const TODO_READ: &str = super::todo::TodoReadTool::NAME;
    pub const COMPACT: &str = super::compact_tool::CompactTool::NAME;
    pub const REGISTER_HOOK: &str = super::hook::RegisterHookTool::NAME;
    #[cfg(target_os = "macos")]
    pub const COMPUTER_USE: &str = super::computer_use::ComputerUseTool::NAME;
    pub const ENTER_PLAN_MODE: &str = super::plan::EnterPlanModeTool::NAME;
    pub const EXIT_PLAN_MODE: &str = super::plan::ExitPlanModeTool::NAME;
    pub const ENTER_WORKTREE: &str = super::worktree::EnterWorktreeTool::NAME;
    pub const EXIT_WORKTREE: &str = super::worktree::ExitWorktreeTool::NAME;
    pub const LOAD_SKILL: &str = super::skill::LoadSkillTool::NAME;
    pub const LOAD_TOOL: &str = super::load_tool::LoadTool::NAME;
    pub const AGENT: &str = super::sub_agent::SubAgentTool::NAME;
    pub const TEAMMATE: &str = super::teammate_tool::TeammateTool::NAME;
    pub const SEND_MESSAGE: &str = super::send_message::SendMessageTool::NAME;
    pub const WORK_DONE: &str = super::work_done::WorkDoneTool::NAME;
    pub const IGNORE_MESSAGE: &str = super::ignore_message::IgnoreMessageTool::NAME;
}
