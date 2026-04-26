//! 优先级消息窗口选择（三阶段 + 比例配额 + 溢出）
//!
//! 核心原则：
//! 1. **时间保底**：最近 K 个 unit 无条件保留（K 与 micro_compact.keep_recent 对齐）
//! 2. **豁免保底**：属于 EXEMPT_TOOLS 的 ToolGroup 优先保留（技能/任务上下文）
//! 3. **比例配额**：剩余预算按比例分配给 User / AssistantText / ToolGroup，
//!    而不是层层堆叠；某 tier 配额用不完时按时间倒序溢出到未保留 unit
//!
//! 输出顺序始终保持原始时间顺序；丢弃的 ToolGroup 用统一占位符替换。

use super::compact::is_exempt_tool;
use super::policy::ContextTier;
use crate::command::chat::constants::{
    WINDOW_KEEP_RECENT_MULTIPLIER, WINDOW_QUOTA_ASST_TEXT, WINDOW_QUOTA_TOOL_GROUP,
    WINDOW_QUOTA_USER,
};
use crate::command::chat::storage::{ChatMessage, MessageRole};
use crate::util::log::write_info_log;

/// 简单 token 估算：每 3 个字符 ≈ 1 token
const SIMPLE_CHARS_PER_TOKEN: usize = 3;

/// token_K 到实际 token 数的换算乘数
const TOKEN_K_MULTIPLIER: usize = 1000;

// ========== MessageUnit 定义 ==========

/// 消息分组 — 原子单元，要么全部保留，要么全部丢弃
#[derive(Debug, Clone)]
enum MessageUnit {
    /// 系统消息，始终保留
    System { message_index: usize },
    /// 用户消息，最高优先级
    User { message_index: usize },
    /// Assistant 纯文字消息（有 content，无 tool_calls）
    AssistantText { message_index: usize },
    /// 工具调用组 — assistant(tool_calls) + 所有对应 tool result，原子单元
    ToolGroup {
        /// assistant(tool_calls) 消息的索引
        assistant_message_index: usize,
        /// 对应 tool result 消息的索引列表（紧跟在 assistant 后面）
        tool_result_indices: Vec<usize>,
    },
}

impl MessageUnit {
    /// 消息单元的优先级（数值越小优先级越高）
    ///
    /// 数值与 `context::policy::ContextTier` 对齐：
    /// - System=0, User=1, KeyTool=2, Assistant=3, RegularTool=4
    ///
    /// ToolGroup 的 tier 取组内 tool_call 的最高优先级（最小数值）。
    /// Stage 2 的豁免保底已经把 KeyTool ToolGroup 单独保留，Stage 3 的比例
    /// 配额只对未保留 unit 生效，故实际参与 Stage 3 筛选的 ToolGroup 通常是 RegularTool。
    fn priority(&self) -> u8 {
        match self {
            MessageUnit::System { .. } => ContextTier::System.priority(),
            MessageUnit::User { .. } => ContextTier::User.priority(),
            MessageUnit::AssistantText { .. } => ContextTier::Assistant.priority(),
            MessageUnit::ToolGroup { .. } => ContextTier::RegularTool.priority(),
        }
    }

    /// 该单元包含的消息条数
    fn msg_count(&self) -> usize {
        match self {
            MessageUnit::System { .. }
            | MessageUnit::User { .. }
            | MessageUnit::AssistantText { .. } => 1,
            MessageUnit::ToolGroup {
                tool_result_indices,
                ..
            } => 1 + tool_result_indices.len(),
        }
    }

    /// 该单元中第一条消息的索引（用于时间排序）
    fn first_idx(&self) -> usize {
        match self {
            MessageUnit::System { message_index }
            | MessageUnit::User { message_index }
            | MessageUnit::AssistantText { message_index } => *message_index,
            MessageUnit::ToolGroup {
                assistant_message_index,
                ..
            } => *assistant_message_index,
        }
    }

    /// 估算该单元的 token 数（用 chars 计数 + /3，兼顾中文场景）
    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize {
        let total_chars: usize = match self {
            MessageUnit::System { message_index }
            | MessageUnit::User { message_index }
            | MessageUnit::AssistantText { message_index } => {
                messages[*message_index].content.chars().count()
            }
            MessageUnit::ToolGroup {
                assistant_message_index,
                tool_result_indices,
            } => {
                let mut chars = messages[*assistant_message_index].content.chars().count();
                for &result_index in tool_result_indices {
                    chars += messages[result_index].content.chars().count();
                }
                if let Some(ref tcs) = messages[*assistant_message_index].tool_calls {
                    for tc in tcs {
                        chars += tc.name.chars().count() + tc.arguments.chars().count();
                    }
                }
                chars
            }
        };
        total_chars / SIMPLE_CHARS_PER_TOKEN
    }

    /// ToolGroup 是否包含豁免工具（任一 tool_call 命中豁免清单即返回 true）
    fn has_exempt_tool(&self, messages: &[ChatMessage], exempt_tools: &[String]) -> bool {
        match self {
            MessageUnit::ToolGroup {
                assistant_message_index,
                ..
            } => messages[*assistant_message_index]
                .tool_calls
                .as_ref()
                .map(|tcs| tcs.iter().any(|tc| is_exempt_tool(&tc.name, exempt_tools)))
                .unwrap_or(false),
            _ => false,
        }
    }
}

// ========== 解析 ==========

/// 将消息序列解析为 MessageUnit 列表
fn parse_message_units(messages: &[ChatMessage]) -> Vec<MessageUnit> {
    let mut units = Vec::with_capacity(messages.len());
    let mut i = 0;

    while i < messages.len() {
        let msg = &messages[i];

        if msg.role == MessageRole::System {
            units.push(MessageUnit::System { message_index: i });
            i += 1;
        } else if msg.role == MessageRole::User {
            units.push(MessageUnit::User { message_index: i });
            i += 1;
        } else if msg.role == MessageRole::Assistant {
            if msg.tool_calls.is_some() {
                // assistant + tool_calls → 收集后续 tool result
                let assistant_message_index = i;
                let mut tool_result_indices = Vec::new(); // 大小未知，无法预分配
                i += 1;
                while i < messages.len() && messages[i].role == MessageRole::Tool {
                    tool_result_indices.push(i);
                    i += 1;
                }
                units.push(MessageUnit::ToolGroup {
                    assistant_message_index,
                    tool_result_indices,
                });
            } else {
                // 纯文字 assistant 消息
                units.push(MessageUnit::AssistantText { message_index: i });
                i += 1;
            }
        } else if msg.role == MessageRole::Tool {
            // 孤立的 tool result（没有前面的 assistant+tool_calls）
            // 作为 ToolGroup 处理（只有 result 没有 assistant）
            let start = i;
            let mut tool_result_indices = vec![i];
            i += 1;
            while i < messages.len() && messages[i].role == MessageRole::Tool {
                tool_result_indices.push(i);
                i += 1;
            }
            // 孤立 tool results 最低优先级，作为 ToolGroup 处理
            units.push(MessageUnit::ToolGroup {
                assistant_message_index: start, // 没有真正的 assistant，用第一个 tool result 的索引
                tool_result_indices,
            });
        } else {
            // 未知角色，作为单条处理
            units.push(MessageUnit::System { message_index: i });
            i += 1;
        }
    }

    units
}

// ========== 优先级选择 ==========

/// 选择结果
struct SelectionResult {
    /// 保留的 unit 索引（在 units 中的位置）
    retained: Vec<bool>,
}

/// 三阶段预算选择：时间保底 → 豁免保底 → 比例配额（+ 溢出）
///
/// select_units 的只读配置参数（units/messages 单独传）
struct SelectUnitsParams<'a> {
    max_history_messages: usize,
    max_context_tokens: usize,
    keep_recent: usize,
    exempt_tools: &'a [String],
}

/// 三阶段选择逻辑：
/// Stage 1: 保留最近 N 个 unit（时间保底）
/// Stage 2: 保留豁免 ToolGroup（技能/任务保底）
/// Stage 3: 按比例配额分配剩余预算
fn select_units(
    units: &[MessageUnit],
    messages: &[ChatMessage],
    params: &SelectUnitsParams,
) -> SelectionResult {
    // 解构出局部变量，保持函数体不变
    let max_history_messages = params.max_history_messages;
    let max_context_tokens = params.max_context_tokens;
    let keep_recent = params.keep_recent;
    let exempt_tools = params.exempt_tools;
    let mut retained_flags = vec![false; units.len()];
    let mut used_message_count = 0usize;
    let mut used_token_count = 0usize;

    // 记账 + 预算检查的闭包式辅助（rust 中改为函数避免借用问题）
    let try_retain_unit = |message_index: usize,
                           retained: &mut [bool],
                           used_message_count: &mut usize,
                           used_token_count: &mut usize|
     -> bool {
        if retained[message_index] {
            return false;
        }
        let unit = &units[message_index];
        let unit_msg_count = unit.msg_count();
        let unit_tokens = unit.estimate_tokens(messages);
        if *used_message_count + unit_msg_count > max_history_messages
            || *used_token_count + unit_tokens > max_context_tokens
        {
            return false;
        }
        retained[message_index] = true;
        *used_message_count += unit_msg_count;
        *used_token_count += unit_tokens;
        true
    };

    // ── System：始终保留（不计配额）──
    for (i, unit) in units.iter().enumerate() {
        if matches!(unit, MessageUnit::System { .. }) {
            // System 即使超预算也保留（通常极少极短）
            retained_flags[i] = true;
            used_message_count += unit.msg_count();
            used_token_count += unit.estimate_tokens(messages);
        }
    }

    // ── Stage 1: 时间保底 ── 最近 K 个非 System unit 无条件保留
    let recent_units_to_keep = keep_recent.saturating_mul(WINDOW_KEEP_RECENT_MULTIPLIER);
    let mut stage1_retained_count = 0usize;
    for i in (0..units.len()).rev() {
        if stage1_retained_count >= recent_units_to_keep {
            break;
        }
        if matches!(units[i], MessageUnit::System { .. }) {
            continue;
        }
        if try_retain_unit(
            i,
            &mut retained_flags,
            &mut used_message_count,
            &mut used_token_count,
        ) {
            stage1_retained_count += 1;
        } else {
            // 预算耗尽即停（最新的装不下，更老的也别试了）
            break;
        }
    }

    // ── Stage 2: 豁免保底 ── 含豁免工具的 ToolGroup 按时间倒序保留
    for i in (0..units.len()).rev() {
        if retained_flags[i] {
            continue;
        }
        if units[i].has_exempt_tool(messages, exempt_tools) {
            try_retain_unit(
                i,
                &mut retained_flags,
                &mut used_message_count,
                &mut used_token_count,
            );
        }
    }

    // ── Stage 3: 比例配额 ── 剩余预算按比例分给三个 tier，tier 内按时间倒序
    let remaining_msgs = max_history_messages.saturating_sub(used_message_count);
    let remaining_toks = max_context_tokens.saturating_sub(used_token_count);

    // tier 数值与 ContextTier::priority() 对齐：
    // User=1, Assistant=3, RegularTool=4（KeyTool=2 已在 Stage 2 豁免保底）
    let quotas: [(u8, f32); 3] = [
        (ContextTier::User.priority(), WINDOW_QUOTA_USER),
        (ContextTier::Assistant.priority(), WINDOW_QUOTA_ASST_TEXT),
        (ContextTier::RegularTool.priority(), WINDOW_QUOTA_TOOL_GROUP),
    ];

    for (tier_prio, ratio) in quotas {
        // tier 子预算（向下取整；溢出阶段会吸收未用完部分）
        let tier_message_budget = ((remaining_msgs as f32) * ratio) as usize;
        let tier_token_budget = ((remaining_toks as f32) * ratio) as usize;
        let tier_start_msg_count = used_message_count;
        let tier_start_token_count = used_token_count;

        // 该 tier 未保留的 unit，按时间倒序
        let mut tier_candidates: Vec<usize> = (0..units.len())
            .filter(|&i| !retained_flags[i] && units[i].priority() == tier_prio)
            .collect();
        tier_candidates.sort_by(|&a, &b| units[b].first_idx().cmp(&units[a].first_idx()));

        for idx in tier_candidates {
            let unit = &units[idx];
            let unit_msg_count = unit.msg_count();
            let unit_tokens = unit.estimate_tokens(messages);
            // 子预算 + 全局预算双检查
            if used_message_count - tier_start_msg_count + unit_msg_count > tier_message_budget {
                continue;
            }
            if used_token_count - tier_start_token_count + unit_tokens > tier_token_budget {
                continue;
            }
            try_retain_unit(
                idx,
                &mut retained_flags,
                &mut used_message_count,
                &mut used_token_count,
            );
        }
    }

    // ── Stage 4: 溢出 ── 剩余预算按时间倒序贪心填充未保留 unit（任意 tier）
    for i in (0..units.len()).rev() {
        try_retain_unit(
            i,
            &mut retained_flags,
            &mut used_message_count,
            &mut used_token_count,
        );
    }

    // ── 兜底 ── 至少保留最新 User unit
    let has_user_retained = units
        .iter()
        .enumerate()
        .any(|(i, u)| matches!(u, MessageUnit::User { .. }) && retained_flags[i]);
    if !has_user_retained
        && let Some(last_user_idx) = (0..units.len())
            .rev()
            .find(|&i| matches!(units[i], MessageUnit::User { .. }))
    {
        retained_flags[last_user_idx] = true;
    }

    SelectionResult {
        retained: retained_flags,
    }
}

// ========== 占位符替换 ==========

/// 提取 ToolGroup 的工具名称列表（用于占位符）
fn tool_names_of(unit: &MessageUnit, messages: &[ChatMessage]) -> Vec<String> {
    match unit {
        MessageUnit::ToolGroup {
            assistant_message_index,
            ..
        } => messages[*assistant_message_index]
            .tool_calls
            .as_ref()
            .map(|tcs| tcs.iter().map(|tc| tc.name.clone()).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// 合并一批被丢弃的 ToolGroup 名称为单条占位符 assistant 消息
/// 与 micro_compact 的占位符风格对齐：`[Previous: used X, Y, Z]`
fn merged_placeholder(names: &[String]) -> ChatMessage {
    let content = if names.is_empty() {
        "[Previous tool calls dropped]".to_string()
    } else {
        format!("[Previous: used {}]", names.join(", "))
    };
    ChatMessage::text(MessageRole::Assistant, content)
}

// ========== 公开接口 ==========

/// 优先级消息窗口选择（三阶段 + 比例配额 + 溢出 + 占位符合并）
///
/// # 参数
/// - `messages`: 原始消息列表
/// - `max_history_messages`: 消息条数上限（0 = 不限制）
/// - `max_context_tokens_k`: token 预算上限，单位 K（0 = 不限制，100 = 100K tokens）
/// - `keep_recent`: 与 `CompactConfig.keep_recent` 对齐；最近 `keep_recent * WINDOW_KEEP_RECENT_MULTIPLIER`
///   个非 System unit 在 Stage 1 无条件保留
/// - `exempt_tools`: 来自 `CompactConfig.micro_compact_exempt_tools`；含豁免工具的 ToolGroup
///   在 Stage 2 优先保留，保护 skill/task 等承载关键上下文的调用
pub fn select_messages(
    messages: &[ChatMessage],
    max_history_messages: usize,
    max_context_tokens_k: usize,
    keep_recent: usize,
    exempt_tools: &[String],
) -> Vec<ChatMessage> {
    let max_msgs = if max_history_messages == 0 {
        usize::MAX
    } else {
        max_history_messages
    };
    let max_tokens = if max_context_tokens_k == 0 {
        usize::MAX
    } else {
        max_context_tokens_k * TOKEN_K_MULTIPLIER
    };

    let total_tokens = estimate_tokens_simple(messages);
    if messages.len() <= max_msgs && total_tokens <= max_tokens {
        return messages.to_vec();
    }

    let units = parse_message_units(messages);
    let selection = select_units(
        &units,
        messages,
        &SelectUnitsParams {
            max_history_messages: max_msgs,
            max_context_tokens: max_tokens,
            keep_recent,
            exempt_tools,
        },
    );

    // 按原始顺序重组消息；被丢弃的相邻 ToolGroup 合并为单个占位符
    let mut result = Vec::with_capacity(messages.len());
    let mut pending_dropped_names: Vec<String> = Vec::new(); // 大小未知

    let flush_pending = |pending: &mut Vec<String>, out: &mut Vec<ChatMessage>| {
        if !pending.is_empty() {
            out.push(merged_placeholder(pending));
            pending.clear();
        }
    };

    for (i, unit) in units.iter().enumerate() {
        if selection.retained[i] {
            flush_pending(&mut pending_dropped_names, &mut result);
            match unit {
                MessageUnit::System { message_index }
                | MessageUnit::User { message_index }
                | MessageUnit::AssistantText { message_index } => {
                    result.push(messages[*message_index].clone());
                }
                MessageUnit::ToolGroup {
                    assistant_message_index,
                    tool_result_indices,
                } => {
                    result.push(messages[*assistant_message_index].clone());
                    for &result_index in tool_result_indices {
                        result.push(messages[result_index].clone());
                    }
                }
            }
        } else if matches!(unit, MessageUnit::ToolGroup { .. }) {
            // 累积相邻被丢弃的 ToolGroup，后续一次性输出合并占位符
            pending_dropped_names.extend(tool_names_of(unit, messages));
        }
        // User / AssistantText 丢弃时直接跳过（兜底保证最新 User 一定保留）
    }
    flush_pending(&mut pending_dropped_names, &mut result);

    let dropped_count = selection.retained.iter().filter(|&&r| !r).count();
    if dropped_count > 0 {
        write_info_log(
            "window_select",
            &format!(
                "三阶段窗口选择: 保留 {}/{} 单元, 丢弃 {} (tokens: {}→{}, keep_recent={})",
                units.len() - dropped_count,
                units.len(),
                dropped_count,
                total_tokens,
                estimate_tokens_simple(&result),
                keep_recent,
            ),
        );
    }

    result
}

/// 简易 token 估算（用于整体判断；与 MessageUnit::estimate_tokens 保持相同系数）
fn estimate_tokens_simple(messages: &[ChatMessage]) -> usize {
    let total_chars: usize = messages
        .iter()
        .map(|m| {
            let mut chars = m.content.chars().count();
            if let Some(ref tcs) = m.tool_calls {
                for tc in tcs {
                    chars += tc.name.chars().count() + tc.arguments.chars().count();
                }
            }
            chars
        })
        .sum();
    total_chars / 3
}

#[cfg(test)]
mod tests;
