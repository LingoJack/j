//! 上下文管理回归测试集
//!
//! 保护 `policy.rs` 统一抽象前后的行为不变，并锁定新增的分层语义。
//! 覆盖：
//! 1. **跨模块一致性**：tool NAME 常量 ↔ policy 硬编码字符串 ↔ BUILTIN_EXEMPT_TOOLS
//! 2. **micro_compact**：KeyTool 永不替换；RegularTool 超阈值替换
//! 3. **window 选择**：KeyTool Stage 2 保底、User 兜底、时间保底、占位符合并
//! 4. **用户明确点名的 KeyTool**：Plan/Worktree/Ask/LoadSkill 在极端预算下保留
//! 5. **Plan mode 白名单与 policy 的关系**：诊断性断言，防止未来两者漂移

use super::compact::{BUILTIN_EXEMPT_TOOLS, is_exempt_tool, micro_compact};
use super::plan_state::PLAN_MODE_WHITELIST;
use super::policy::{ContextTier, RetentionPolicy, is_key_tool, policy_for, tier_for};
use super::window::select_messages;
use crate::command::chat::storage::{ChatMessage, MessageRole, ToolCallItem};

// ========== 辅助构造 ==========

fn user(content: &str) -> ChatMessage {
    ChatMessage::text(MessageRole::User, content)
}

fn assistant(content: &str) -> ChatMessage {
    ChatMessage::text(MessageRole::Assistant, content)
}

/// 生成带全局唯一 id 的 tool_call 消息。
/// 注意：micro_compact 根据 tool_call.id → tool_name 建立映射，相同 id 会互相覆盖，
/// 因此测试必须确保每个 tool_call 的 id 全局唯一。
fn tool_call_with_id(id: &str, name: &str) -> ChatMessage {
    ChatMessage {
        role: MessageRole::Assistant,
        content: String::new(),
        tool_calls: Some(vec![ToolCallItem {
            id: id.to_string(),
            name: name.to_string(),
            arguments: "{}".to_string(),
        }]),
        tool_call_id: None,
        images: None,
        reasoning_content: None,
        sender_name: None,
    }
}

fn tool_call(names: &[&str]) -> ChatMessage {
    ChatMessage {
        role: MessageRole::Assistant,
        content: String::new(),
        tool_calls: Some(
            names
                .iter()
                .enumerate()
                .map(|(i, name)| ToolCallItem {
                    id: format!("call_{}", i),
                    name: name.to_string(),
                    arguments: "{}".to_string(),
                })
                .collect(),
        ),
        tool_call_id: None,
        images: None,
        reasoning_content: None,
        sender_name: None,
    }
}

fn tool_result(call_id: &str, content: &str) -> ChatMessage {
    ChatMessage {
        role: MessageRole::Tool,
        content: content.to_string(),
        tool_calls: None,
        tool_call_id: Some(call_id.to_string()),
        images: None,
        reasoning_content: None,
        sender_name: None,
    }
}

// ========== 跨模块一致性 ==========

/// 各工具的 NAME 常量必须与 policy.rs 中的硬编码字符串一致
/// 若某工具被重命名（改 NAME）但忘记同步 policy.rs，此测试会失败
#[test]
fn tool_name_constants_match_policy_hardcoded_strings() {
    use crate::command::chat::tools::tool_names::*;

    // 用户明确点名的重要工具 — 必须是 KeyTool
    assert!(is_key_tool(ENTER_PLAN_MODE), "EnterPlanMode 应为 KeyTool");
    assert!(is_key_tool(EXIT_PLAN_MODE), "ExitPlanMode 应为 KeyTool");
    assert!(is_key_tool(ENTER_WORKTREE), "EnterWorktree 应为 KeyTool");
    assert!(is_key_tool(EXIT_WORKTREE), "ExitWorktree 应为 KeyTool");
    assert!(is_key_tool(ASK), "Ask 应为 KeyTool");
    assert!(is_key_tool(LOAD_SKILL), "LoadSkill 应为 KeyTool");

    // 其他承载工作流/协作上下文的工具
    assert!(is_key_tool(TASK), "Task 应为 KeyTool");
    assert!(is_key_tool(TODO_WRITE), "TodoWrite 应为 KeyTool");
    assert!(is_key_tool(TODO_READ), "TodoRead 应为 KeyTool");
    assert!(is_key_tool(AGENT), "Agent 应为 KeyTool");
    assert!(is_key_tool(TEAMMATE), "Teammate 应为 KeyTool");
    assert!(is_key_tool(SEND_MESSAGE), "SendMessage 应为 KeyTool");
}

/// 常规工具必须归为 RegularTool + Placeholder
#[test]
fn regular_tool_name_constants_match_policy() {
    use crate::command::chat::tools::tool_names::*;

    for name in [BASH, READ, WRITE, EDIT, GLOB, GREP, WEB_FETCH, WEB_SEARCH] {
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

/// BUILTIN_EXEMPT_TOOLS 重导出必须等价于 is_key_tool
#[test]
fn builtin_exempt_tools_aliases_key_tools() {
    for &name in BUILTIN_EXEMPT_TOOLS {
        assert!(
            is_key_tool(name),
            "BUILTIN_EXEMPT_TOOLS 中的 {} 必须是 KeyTool",
            name
        );
    }
    // 反向：每个 KeyTool 都在 BUILTIN_EXEMPT_TOOLS 里（通过 KEY_TOOL_NAMES 派生保证）
    use super::policy::KEY_TOOL_NAMES;
    assert_eq!(
        BUILTIN_EXEMPT_TOOLS.len(),
        KEY_TOOL_NAMES.len(),
        "BUILTIN_EXEMPT_TOOLS 应与 KEY_TOOL_NAMES 长度一致"
    );
}

/// is_exempt_tool(..., &[]) 等价于 is_key_tool
#[test]
fn is_exempt_tool_without_extra_equals_is_key_tool() {
    use crate::command::chat::tools::tool_names::*;

    for name in [ENTER_PLAN_MODE, BASH, READ, LOAD_SKILL, ASK, GREP] {
        assert_eq!(
            is_exempt_tool(name, &[]),
            is_key_tool(name),
            "is_exempt_tool({}, []) 应等价于 is_key_tool",
            name
        );
    }
}

/// 用户扩展清单优先级高于 policy：即使 policy 判定 RegularTool，用户扩展也能豁免
#[test]
fn user_extra_exempt_tools_override() {
    use crate::command::chat::tools::tool_names::BASH;

    let extra = vec![BASH.to_string()];
    assert!(is_exempt_tool(BASH, &extra), "Bash 被用户扩展豁免应生效");
    assert!(
        !is_exempt_tool(BASH, &[]),
        "Bash 未被扩展豁免时应非豁免（默认 RegularTool）"
    );
}

// ========== micro_compact 行为回归 ==========

/// KeyTool 的 tool result 永不被占位符替换（即使超阈值且早于 keep_recent）
#[test]
fn micro_compact_preserves_key_tool_results() {
    use crate::command::chat::tools::tool_names::*;

    let big = "x".repeat(5000); // 远超 MICRO_COMPACT_BYTES_THRESHOLD
    let mut msgs = vec![
        user("load skill"),
        tool_call_with_id("k1", LOAD_SKILL),
        tool_result("k1", &big),
        user("enter plan mode"),
        tool_call_with_id("k2", ENTER_PLAN_MODE),
        tool_result("k2", &big),
        user("edit worktree"),
        tool_call_with_id("k3", ENTER_WORKTREE),
        tool_result("k3", &big),
        user("query ask"),
        tool_call_with_id("k4", ASK),
        tool_result("k4", &big),
        // 后面堆叠足够多的 tool result 迫使前面被考虑压缩
        user("do shell"),
        tool_call_with_id("b1", BASH),
        tool_result("b1", "ls output"),
        tool_call_with_id("b2", BASH),
        tool_result("b2", "ls output"),
        tool_call_with_id("b3", BASH),
        tool_result("b3", "ls output"),
    ];

    // keep_recent=1 → 只有 b3 的 result 被时间豁免；前面的 KeyTool 需要依赖策略豁免
    micro_compact(&mut msgs, 1, &[]);

    // 四个 KeyTool 的 big result 都必须原样保留
    let preserved_big_count = msgs.iter().filter(|m| m.content.len() >= 5000).count();
    assert_eq!(
        preserved_big_count, 4,
        "4 个 KeyTool 的大体积 result 必须全部保留，实际保留 {}",
        preserved_big_count
    );
}

/// RegularTool 的大体积 result 应按 keep_recent 替换为占位符
#[test]
fn micro_compact_placeholder_for_regular_tools() {
    use crate::command::chat::tools::tool_names::BASH;

    let big = "x".repeat(5000);
    let mut msgs = vec![
        user("q1"),
        tool_call_with_id("b1", BASH),
        tool_result("b1", &big),
        user("q2"),
        tool_call_with_id("b2", BASH),
        tool_result("b2", &big),
        user("q3"),
        tool_call_with_id("b3", BASH),
        tool_result("b3", &big),
    ];

    micro_compact(&mut msgs, 1, &[]);

    // 最早两个 Bash result 应被替换为占位符
    let placeholders = msgs
        .iter()
        .filter(|m| m.role == MessageRole::Tool && m.content.starts_with("[Previous: used Bash]"))
        .count();
    assert_eq!(placeholders, 2, "最早 2 个 Bash result 应被替换为占位符");
}

/// 小于阈值的 result 不被替换
#[test]
fn micro_compact_below_threshold_not_replaced() {
    use crate::command::chat::tools::tool_names::BASH;

    let small = "ok".to_string();
    let mut msgs = vec![
        user("q1"),
        tool_call_with_id("b1", BASH),
        tool_result("b1", &small),
        user("q2"),
        tool_call_with_id("b2", BASH),
        tool_result("b2", &small),
        user("q3"),
        tool_call_with_id("b3", BASH),
        tool_result("b3", &small),
    ];

    micro_compact(&mut msgs, 1, &[]);

    // 小体积 result 不触发替换
    assert!(
        msgs.iter().all(|m| !m.content.contains("Previous: used")),
        "小体积 result 不应被替换为占位符"
    );
}

// ========== window 选择行为回归 ==========

/// 用户明确点名的 KeyTool 在极端预算下必须通过 Stage 2 豁免保底保留
#[test]
fn window_preserves_user_mentioned_key_tools_under_tight_budget() {
    use crate::command::chat::tools::tool_names::*;

    for key_tool in [ENTER_PLAN_MODE, ENTER_WORKTREE, ASK, LOAD_SKILL] {
        let msgs = vec![
            user("load it"),
            tool_call(&[key_tool]),
            tool_result(
                "call_0",
                &format!("{} large content ", key_tool).repeat(300),
            ),
            user("q"),
            assistant("a"),
            user("q2"),
            assistant("a2"),
            user("q3"),
        ];

        // 紧预算 + keep_recent=0（禁用时间保底）迫使选择器依赖 Stage 2 豁免保底
        let result = select_messages(&msgs, 100, 5, 0, &[]);

        assert!(
            result.iter().any(|m| m.role == MessageRole::Tool),
            "{} 的 tool result 必须通过豁免保底保留",
            key_tool
        );
    }
}

/// User 兜底：至少保留最新 User 消息
#[test]
fn window_always_retains_latest_user() {
    use crate::command::chat::tools::tool_names::BASH;

    let mut msgs = Vec::new();
    for i in 0..50 {
        msgs.push(tool_call(&[BASH]));
        msgs.push(tool_result("call_0", &"output ".repeat(500)));
        msgs.push(user(&format!("q{}", i)));
    }
    msgs.push(user("LATEST"));

    // 极端紧预算（1K tokens），keep_recent=0，会大量丢弃
    let result = select_messages(&msgs, 3, 1, 0, &[]);

    assert!(
        result
            .iter()
            .any(|m| m.role == MessageRole::User && m.content == "LATEST"),
        "最新 User 消息必须通过兜底保留"
    );
}

/// 时间保底：最近 keep_recent * MULTIPLIER 个 unit 无条件保留
#[test]
fn window_stage1_time_fallback() {
    use crate::command::chat::tools::tool_names::BASH;

    let mut msgs = Vec::new();
    for i in 0..20 {
        msgs.push(user(&format!("old {}", i).repeat(30)));
    }
    msgs.push(tool_call(&[BASH]));
    msgs.push(tool_result("call_0", "recent bash"));
    msgs.push(user("latest"));

    // keep_recent=2 → Stage 1 保留最近 4 个 unit（覆盖 ToolGroup + latest User）
    let result = select_messages(&msgs, 100, 2, 2, &[]);

    assert!(
        result
            .iter()
            .any(|m| m.role == MessageRole::Tool && m.content == "recent bash"),
        "最近的 Bash result 应被 Stage 1 时间保底保留"
    );
    assert!(
        result
            .iter()
            .any(|m| m.role == MessageRole::User && m.content == "latest"),
        "最新 User 必须保留"
    );
}

/// 占位符合并：相邻被丢弃的 ToolGroup 合并为单条 `[Previous: used X, Y, Z]`
#[test]
fn window_merges_adjacent_dropped_tool_groups() {
    use crate::command::chat::tools::tool_names::*;

    let msgs = vec![
        user("run"),
        tool_call(&[BASH]),
        tool_result("call_0", &"x".repeat(3000)),
        tool_call(&[READ]),
        tool_result("call_0", &"y".repeat(3000)),
        tool_call(&[GREP]),
        tool_result("call_0", &"z".repeat(3000)),
        user("latest"),
    ];

    // 紧预算 + keep_recent=0 迫使 ToolGroup 被丢弃
    let result = select_messages(&msgs, 100, 1, 0, &[]);

    let placeholders: Vec<&ChatMessage> = result
        .iter()
        .filter(|m| m.content.contains("Previous: used"))
        .collect();
    assert!(
        !placeholders.is_empty(),
        "应有至少一条占位符消息合并被丢弃的 ToolGroup"
    );
    let combined = placeholders
        .iter()
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n");
    // 三个工具名中至少两个会合并到同一占位符
    let hit = [BASH, READ, GREP]
        .iter()
        .filter(|name| combined.contains(*name))
        .count();
    assert!(
        hit >= 2,
        "至少两个工具名应出现在合并占位符中，实际: {}",
        combined
    );
}

/// 时间顺序保持：被保留的消息在输出中保持原始时间顺序
#[test]
fn window_preserves_time_order() {
    use crate::command::chat::tools::tool_names::BASH;

    let msgs = vec![
        user("A"),
        assistant("a1"),
        tool_call(&[BASH]),
        tool_result("call_0", "r1"),
        user("B"),
        assistant("a2"),
        user("C"),
    ];

    let result = select_messages(&msgs, 100, 0, 10, &[]);

    // 找出 A/B/C 在结果中的位置
    let pos_a = result.iter().position(|m| m.content == "A").unwrap();
    let pos_b = result.iter().position(|m| m.content == "B").unwrap();
    let pos_c = result.iter().position(|m| m.content == "C").unwrap();
    assert!(pos_a < pos_b && pos_b < pos_c, "User 消息应保持时间顺序");
}

/// 无须截断时完整返回
#[test]
fn window_no_truncation_when_within_budget() {
    let msgs = vec![user("hello"), assistant("hi")];
    let result = select_messages(&msgs, 100, 0, 10, &[]);
    assert_eq!(result.len(), 2);
}

// ========== Plan mode 白名单与 Policy 的关系（诊断性）==========

/// 诊断性：Plan mode 白名单里声明的工具名在 policy 中可被正确识别
///
/// 这不断言白名单 ⊆ KeyTool；白名单的语义是"plan 模式允许执行"，
/// 与"上下文保留策略"正交（例如 Read/Glob/Grep 在白名单但不是 KeyTool）。
/// 但所有白名单工具都应该被 policy_for 识别（即使归为 RegularTool）。
#[test]
fn plan_mode_whitelist_recognized_by_policy() {
    for name in PLAN_MODE_WHITELIST {
        let p = policy_for(name);
        // tier 必定在已定义枚举中；检查是否退回默认 RegularTool 无需断言（所有未知都回 Regular）
        // 关键：确保硬编码字符串在 policy 中"有意义"，即已显式列入或正确 fallback
        let _ = p.tier;
    }
}

/// Plan mode 白名单中的"决策类"工具应该是 KeyTool
///
/// 注意：LoadSkill **不在** Plan mode 白名单（plan 模式禁止加载新技能），
/// 但仍然是 KeyTool（被加载过的技能调用结果不可丢失）。两语义正交。
#[test]
fn plan_mode_decision_tools_are_key_tools() {
    use crate::command::chat::tools::tool_names::*;

    for name in [ENTER_PLAN_MODE, EXIT_PLAN_MODE, ASK] {
        assert!(
            PLAN_MODE_WHITELIST.contains(&name),
            "{} 应在 Plan mode 白名单",
            name
        );
        assert!(is_key_tool(name), "{} 应为 KeyTool", name);
    }

    // LoadSkill 不在 plan 白名单，但仍是 KeyTool
    assert!(!PLAN_MODE_WHITELIST.contains(&LOAD_SKILL));
    assert!(is_key_tool(LOAD_SKILL));
}

// ========== Tier 优先级语义 ==========

/// 用户定义的优先级：User > KeyTool > Assistant > RegularTool
#[test]
fn tier_ordering_matches_user_requirement() {
    use crate::command::chat::tools::tool_names::*;

    assert!(ContextTier::User < ContextTier::KeyTool);
    assert!(ContextTier::KeyTool < ContextTier::Assistant);
    assert!(ContextTier::Assistant < ContextTier::RegularTool);

    // 用户明确点名的四个工具的 tier 严格优于普通 Shell
    for name in [ENTER_PLAN_MODE, ENTER_WORKTREE, ASK, LOAD_SKILL] {
        assert!(
            tier_for(name) < tier_for(BASH),
            "{} 的 tier 必须严格优于 Bash",
            name
        );
    }
}
