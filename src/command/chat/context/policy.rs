//! 上下文优先级策略 — 统一源头
//!
//! 把原先散落在多处的"工具重要性"声明收敛到本模块：
//! - `context/compact.rs::BUILTIN_EXEMPT_TOOLS`（micro_compact 豁免）
//! - `context/window.rs::MessageUnit::priority`（窗口优先级）
//! - `context/window.rs::has_exempt_tool`（Stage 2 豁免保底）
//!
//! 每个工具声明一份 `ToolContextPolicy`（tier + retention），
//! micro_compact / window / UI 全部查询 `policy_for()` 获取策略，
//! 而不是各自维护列表。新增工具时只需在下方 match 里加一行即可同步所有路径。
//!
//! 所有工具名称通过 `tools::tool_names` 模块的常量引用，避免硬编码字符串。
//!
//! 参考用户诉求：
//! > 用户消息 > 重要 Tool 消息（PLAN、WORKTREE、ASK、LOADSKILL）> 帮手消息 > 其他工具消息
//! > 其中 PLAN/WORKTREE/ASK/LOADSKILL 的 tool call 与 tool result 可根据特点进行重要性保留

use crate::command::chat::tools::tool_names;

/// 消息优先级层级（数值越小优先级越高，用于 window 选择）
///
/// - `System`：始终保留（不计配额）
/// - `User`：用户输入，兜底保证最新一条必保留
/// - `KeyTool`：承载决策/协议/工作流状态的关键工具，Stage 2 豁免保底
/// - `Assistant`：帮手纯文字回复
/// - `RegularTool`：常规执行类工具，预算紧张时优先丢弃
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContextTier {
    System = 0,
    User = 1,
    KeyTool = 2,
    Assistant = 3,
    RegularTool = 4,
}

impl ContextTier {
    /// 数值形式的优先级（与原 `MessageUnit::priority` 对齐，越小优先级越高）
    pub fn priority(self) -> u8 {
        self as u8
    }
}

/// 保留策略 — micro_compact / auto_compact 根据此决定如何处理历史 tool result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionPolicy {
    /// 永不压缩：micro_compact 跳过、window 在 Stage 2 豁免保底保留
    /// 适用于承载决策/协议/技能指令等不可丢失内容的工具
    AlwaysPreserve,
    /// 压缩为简短占位符 `[Previous: used X]`（当前 micro_compact 默认行为）
    Placeholder,
}

/// 工具的上下文处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolContextPolicy {
    pub tier: ContextTier,
    pub retention: RetentionPolicy,
}

impl ToolContextPolicy {
    const fn key_preserve() -> Self {
        Self {
            tier: ContextTier::KeyTool,
            retention: RetentionPolicy::AlwaysPreserve,
        }
    }
    const fn regular_placeholder() -> Self {
        Self {
            tier: ContextTier::RegularTool,
            retention: RetentionPolicy::Placeholder,
        }
    }
}

/// KeyTool + AlwaysPreserve 的工具名单（静态常量，UI 展示用）
///
/// 所有名称均引用 `tool_names` 常量，与各工具的 `NAME` 定义自动对齐。
/// 新增 KeyTool 时必须同时更新此列表和 `policy_for()` 中的 match 分支。
/// 与 `policy_for()` 保持一致通过 debug_assert 校验（见 tests）。
pub const KEY_TOOL_NAMES: &[&str] = &[
    // 用户明确点名的"重要 Tool"
    tool_names::ENTER_PLAN_MODE,
    tool_names::EXIT_PLAN_MODE,
    tool_names::ENTER_WORKTREE,
    tool_names::EXIT_WORKTREE,
    tool_names::ASK,
    tool_names::LOAD_SKILL,
    // 工作流/任务记账 — 承载 todo/子任务状态
    tool_names::TODO_WRITE,
    tool_names::TODO_READ,
    tool_names::TASK,
    // 协作类 — 承载 teammate/subagent 会话上下文
    tool_names::AGENT,
    tool_names::AGENT_TEAM,
    tool_names::SEND_MESSAGE,
    tool_names::CREATE_TEAMMATE,
];

/// 查询指定工具的上下文策略
///
/// 未知工具按 RegularTool + Placeholder 处理（最保守策略）。
pub fn policy_for(tool_name: &str) -> ToolContextPolicy {
    match tool_name {
        // Key tools — Always preserve
        tool_names::ENTER_PLAN_MODE
        | tool_names::EXIT_PLAN_MODE
        | tool_names::ENTER_WORKTREE
        | tool_names::EXIT_WORKTREE
        | tool_names::ASK
        | tool_names::LOAD_SKILL
        | tool_names::TODO_WRITE
        | tool_names::TODO_READ
        | tool_names::TASK
        | tool_names::AGENT
        | tool_names::AGENT_TEAM
        | tool_names::SEND_MESSAGE
        | tool_names::CREATE_TEAMMATE => ToolContextPolicy::key_preserve(),

        // 其他所有工具（Bash/Read/Write/Edit/Glob/Grep/WebFetch/WebSearch/
        // Browser/ComputerUse/TaskOutput/WorkDone/Compact/RegisterHook 等）
        _ => ToolContextPolicy::regular_placeholder(),
    }
}

/// 便捷查询：工具是否为 KeyTool（AlwaysPreserve）
pub fn is_key_tool(tool_name: &str) -> bool {
    policy_for(tool_name).retention == RetentionPolicy::AlwaysPreserve
}

/// 便捷查询：工具所属 tier
#[cfg(test)]
pub fn tier_for(tool_name: &str) -> ContextTier {
    policy_for(tool_name).tier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_tool_list_matches_policy() {
        // KEY_TOOL_NAMES 必须与 policy_for() 返回 AlwaysPreserve 的工具一致
        for name in KEY_TOOL_NAMES {
            assert!(
                is_key_tool(name),
                "KEY_TOOL_NAMES 中的 {} 应返回 AlwaysPreserve",
                name
            );
        }
    }

    #[test]
    fn regular_tools_are_placeholder() {
        for name in [
            tool_names::BASH,
            tool_names::READ,
            tool_names::WRITE,
            tool_names::EDIT,
            tool_names::GLOB,
            tool_names::GREP,
            tool_names::WEB_FETCH,
        ] {
            let p = policy_for(name);
            assert_eq!(
                p.tier,
                ContextTier::RegularTool,
                "{} 应为 RegularTool",
                name
            );
            assert_eq!(
                p.retention,
                RetentionPolicy::Placeholder,
                "{} 应为 Placeholder",
                name
            );
        }
    }

    #[test]
    fn unknown_tool_fallback_regular() {
        let p = policy_for("SomeNewFutureTool");
        assert_eq!(p.tier, ContextTier::RegularTool);
        assert_eq!(p.retention, RetentionPolicy::Placeholder);
    }

    #[test]
    fn tier_priority_ordering() {
        assert!(ContextTier::System.priority() < ContextTier::User.priority());
        assert!(ContextTier::User.priority() < ContextTier::KeyTool.priority());
        assert!(ContextTier::KeyTool.priority() < ContextTier::Assistant.priority());
        assert!(ContextTier::Assistant.priority() < ContextTier::RegularTool.priority());
    }

    #[test]
    fn user_mentioned_key_tools_are_preserved() {
        // 用户明确点名的 4 个重要工具必须是 KeyTool
        for name in [
            tool_names::ENTER_PLAN_MODE,
            tool_names::ENTER_WORKTREE,
            tool_names::ASK,
            tool_names::LOAD_SKILL,
        ] {
            assert!(is_key_tool(name), "{} 必须是 KeyTool", name);
        }
    }
}
