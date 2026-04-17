//! 优先级消息窗口选择
//!
//! 核心原则：**优先级决定丢弃顺序，时间顺序决定输出顺序。**
//!
//! 当上下文窗口预算不足时，按优先级丢弃消息：
//! - P1 (最高): User 消息 — 尽量保留
//! - P2: Assistant 纯文字回复 — 其次保留
//! - P3 (最低): ToolGroup (assistant+tool_calls + tool results) — 最先丢弃
//!
//! 丢弃的 ToolGroup 用占位符替换（类似 micro_compact），保持对话时间顺序连贯。

use super::super::constants::{ROLE_ASSISTANT, ROLE_SYSTEM, ROLE_TOOL, ROLE_USER};
use super::super::storage::ChatMessage;
use crate::util::log::write_info_log;

// ========== MessageUnit 定义 ==========

/// 消息分组 — 原子单元，要么全部保留，要么全部丢弃
#[derive(Debug, Clone)]
enum MessageUnit {
    /// 系统消息，始终保留
    System { idx: usize },
    /// 用户消息，最高优先级
    User { idx: usize },
    /// Assistant 纯文字消息（有 content，无 tool_calls）
    AssistantText { idx: usize },
    /// 工具调用组 — assistant(tool_calls) + 所有对应 tool result，原子单元
    ToolGroup {
        /// assistant(tool_calls) 消息的索引
        assistant_idx: usize,
        /// 对应 tool result 消息的索引列表（紧跟在 assistant 后面）
        tool_result_indices: Vec<usize>,
    },
}

impl MessageUnit {
    /// 消息单元的优先级（数值越小优先级越高）
    fn priority(&self) -> u8 {
        match self {
            MessageUnit::System { .. } => 0,
            MessageUnit::User { .. } => 1,
            MessageUnit::AssistantText { .. } => 2,
            MessageUnit::ToolGroup { .. } => 3,
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
            MessageUnit::System { idx }
            | MessageUnit::User { idx }
            | MessageUnit::AssistantText { idx } => *idx,
            MessageUnit::ToolGroup { assistant_idx, .. } => *assistant_idx,
        }
    }

    /// 估算该单元的 token 数
    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize {
        let total_chars: usize = match self {
            MessageUnit::System { idx } => messages[*idx].content.len(),
            MessageUnit::User { idx } => messages[*idx].content.len(),
            MessageUnit::AssistantText { idx } => messages[*idx].content.len(),
            MessageUnit::ToolGroup {
                assistant_idx,
                tool_result_indices,
            } => {
                let mut chars = messages[*assistant_idx].content.len();
                for &ri in tool_result_indices {
                    chars += messages[ri].content.len();
                }
                // tool_calls 的 name + arguments 也占 token
                if let Some(ref tcs) = messages[*assistant_idx].tool_calls {
                    for tc in tcs {
                        chars += tc.name.len() + tc.arguments.len();
                    }
                }
                chars
            }
        };
        // ~4 chars per token
        total_chars / 4
    }
}

// ========== 解析 ==========

/// 将消息序列解析为 MessageUnit 列表
fn parse_message_units(messages: &[ChatMessage]) -> Vec<MessageUnit> {
    let mut units = Vec::new();
    let mut i = 0;

    while i < messages.len() {
        let msg = &messages[i];

        if msg.role == ROLE_SYSTEM {
            units.push(MessageUnit::System { idx: i });
            i += 1;
        } else if msg.role == ROLE_USER {
            units.push(MessageUnit::User { idx: i });
            i += 1;
        } else if msg.role == ROLE_ASSISTANT {
            if msg.tool_calls.is_some() {
                // assistant + tool_calls → 收集后续 tool result
                let assistant_idx = i;
                let mut tool_result_indices = Vec::new();
                i += 1;
                while i < messages.len() && messages[i].role == ROLE_TOOL {
                    tool_result_indices.push(i);
                    i += 1;
                }
                units.push(MessageUnit::ToolGroup {
                    assistant_idx,
                    tool_result_indices,
                });
            } else {
                // 纯文字 assistant 消息
                units.push(MessageUnit::AssistantText { idx: i });
                i += 1;
            }
        } else if msg.role == ROLE_TOOL {
            // 孤立的 tool result（没有前面的 assistant+tool_calls）
            // 作为 ToolGroup 处理（只有 result 没有 assistant）
            let start = i;
            let mut tool_result_indices = vec![i];
            i += 1;
            while i < messages.len() && messages[i].role == ROLE_TOOL {
                tool_result_indices.push(i);
                i += 1;
            }
            // 孤立 tool results 最低优先级，作为 ToolGroup 处理
            units.push(MessageUnit::ToolGroup {
                assistant_idx: start, // 没有真正的 assistant，用第一个 tool result 的索引
                tool_result_indices,
            });
        } else {
            // 未知角色，作为单条处理
            units.push(MessageUnit::System { idx: i });
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

/// 按优先级和预算选择消息单元
fn select_units(
    units: &[MessageUnit],
    messages: &[ChatMessage],
    max_history_messages: usize,
    max_context_tokens: usize,
) -> SelectionResult {
    let mut retained = vec![false; units.len()];
    let mut used_msg_count = 0usize;
    let mut used_tokens = 0usize;

    // 按优先级分层，每层内按时间倒序（新的优先保留）
    // 构建按优先级分组的索引列表
    let mut tiers: Vec<Vec<usize>> = vec![Vec::new(); 4]; // 0=System, 1=User, 2=AssistantText, 3=ToolGroup
    for (i, unit) in units.iter().enumerate() {
        tiers[unit.priority() as usize].push(i);
    }

    // Tier 0: System — 始终保留
    for &idx in &tiers[0] {
        retained[idx] = true;
        used_msg_count += units[idx].msg_count();
        used_tokens += units[idx].estimate_tokens(messages);
    }

    // Tier 1-3: 按优先级从高到低，每层内从新到旧
    for tier in &tiers[1..] {
        // 每层内按 first_idx 倒序排列（新的先选）
        let mut sorted: Vec<usize> = tier.clone();
        sorted.sort_by(|&a, &b| units[b].first_idx().cmp(&units[a].first_idx()));

        for &idx in &sorted {
            let unit = &units[idx];
            let msg_count = unit.msg_count();
            let tokens = unit.estimate_tokens(messages);

            // 检查双重预算
            if used_msg_count + msg_count > max_history_messages {
                continue;
            }
            if used_tokens + tokens > max_context_tokens {
                continue;
            }

            retained[idx] = true;
            used_msg_count += msg_count;
            used_tokens += tokens;
        }
    }

    // 安全兜底：至少保留一个 User unit（如果有的话）
    let has_user_retained = units
        .iter()
        .enumerate()
        .any(|(i, u)| matches!(u, MessageUnit::User { .. }) && retained[i]);
    if !has_user_retained {
        // 找最新的 User unit，强制保留
        if let Some(&last_user_idx) = tiers[1].last() {
            retained[last_user_idx] = true;
        }
    }

    SelectionResult { retained }
}

// ========== 占位符替换 ==========

/// 为被丢弃的 ToolGroup 创建占位符消息
fn create_placeholder(unit: &MessageUnit, messages: &[ChatMessage]) -> ChatMessage {
    match unit {
        MessageUnit::ToolGroup {
            assistant_idx,
            tool_result_indices,
        } => {
            // 从 assistant 的 tool_calls 中提取工具名称
            let tool_names: Vec<String> = messages[*assistant_idx]
                .tool_calls
                .as_ref()
                .map(|tcs| tcs.iter().map(|tc| tc.name.clone()).collect())
                .unwrap_or_default();

            let count = if tool_result_indices.is_empty() {
                tool_names.len()
            } else {
                tool_result_indices.len()
            };

            let content = if tool_names.is_empty() {
                format!("[{} tool calls]", count)
            } else {
                format!("[{} tool calls: {}]", count, tool_names.join(", "))
            };

            ChatMessage {
                role: ROLE_ASSISTANT.to_string(),
                content,
                tool_calls: None,
                tool_call_id: None,
                images: None,
            }
        }
        // 非 ToolGroup 不应该走到这里，但做防御处理
        _ => ChatMessage {
            role: ROLE_ASSISTANT.to_string(),
            content: "[message dropped]".to_string(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        },
    }
}

// ========== 公开接口 ==========

/// 优先级消息窗口选择
///
/// 按优先级选择消息（User > AssistantText > ToolGroup），在预算内保留尽可能多的高优先级消息。
/// 被丢弃的 ToolGroup 用占位符替换，保持对话时间顺序连贯。
///
/// # 参数
/// - `messages`: 原始消息列表
/// - `max_history_messages`: 消息条数上限（0 = 不限制）
/// - `max_context_tokens_k`: token 预算上限，单位 K（0 = 不限制，100 = 100K tokens）
pub fn select_messages(
    messages: &[ChatMessage],
    max_history_messages: usize,
    max_context_tokens_k: usize,
) -> Vec<ChatMessage> {
    // 0 = 不限制
    let max_msgs = if max_history_messages == 0 {
        usize::MAX
    } else {
        max_history_messages
    };
    let max_tokens = if max_context_tokens_k == 0 {
        usize::MAX
    } else {
        max_context_tokens_k * 1000 // K → 实际 token 数
    };

    // 不超预算时直接返回
    let total_tokens = estimate_tokens_simple(messages);
    if messages.len() <= max_msgs && total_tokens <= max_tokens {
        return messages.to_vec();
    }

    let units = parse_message_units(messages);
    let selection = select_units(&units, messages, max_msgs, max_tokens);

    // 按原始顺序重组消息
    let mut result = Vec::new();
    for (i, unit) in units.iter().enumerate() {
        if selection.retained[i] {
            // 保留：原样输出
            match unit {
                MessageUnit::System { idx }
                | MessageUnit::User { idx }
                | MessageUnit::AssistantText { idx } => {
                    result.push(messages[*idx].clone());
                }
                MessageUnit::ToolGroup {
                    assistant_idx,
                    tool_result_indices,
                } => {
                    result.push(messages[*assistant_idx].clone());
                    for &ri in tool_result_indices {
                        result.push(messages[ri].clone());
                    }
                }
            }
        } else {
            // 丢弃：ToolGroup 用占位符替换，其他类型直接跳过
            match unit {
                MessageUnit::ToolGroup { .. } => {
                    result.push(create_placeholder(unit, messages));
                }
                // User / AssistantText 理论上不会被丢弃（优先级最高），
                // 但如果预算极度紧张，跳过即可
                _ => {}
            }
        }
    }

    let dropped_count = units
        .iter()
        .enumerate()
        .filter(|(i, _)| !selection.retained[*i])
        .count();
    if dropped_count > 0 {
        write_info_log(
            "window_select",
            &format!(
                "优先级选择: 保留 {}/{} 消息单元, 丢弃 {} (tokens: {}→{})",
                units.len() - dropped_count,
                units.len(),
                dropped_count,
                total_tokens,
                estimate_tokens_simple(&result),
            ),
        );
    }

    result
}

/// 简易 token 估算（用于整体判断）
fn estimate_tokens_simple(messages: &[ChatMessage]) -> usize {
    let total_chars: usize = messages
        .iter()
        .map(|m| {
            let mut chars = m.content.len();
            if let Some(ref tcs) = m.tool_calls {
                for tc in tcs {
                    chars += tc.name.len() + tc.arguments.len();
                }
            }
            chars
        })
        .sum();
    total_chars / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_msg(content: &str) -> ChatMessage {
        ChatMessage {
            role: ROLE_USER.to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        }
    }

    fn assistant_msg(content: &str) -> ChatMessage {
        ChatMessage {
            role: ROLE_ASSISTANT.to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        }
    }

    fn tool_call_msg(names: &[&str]) -> ChatMessage {
        ChatMessage {
            role: ROLE_ASSISTANT.to_string(),
            content: String::new(),
            tool_calls: Some(
                names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| super::super::super::storage::ToolCallItem {
                        id: format!("call_{}", i),
                        name: name.to_string(),
                        arguments: "{}".to_string(),
                    })
                    .collect(),
            ),
            tool_call_id: None,
            images: None,
        }
    }

    fn tool_result_msg(call_id: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: ROLE_TOOL.to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: Some(call_id.to_string()),
            images: None,
        }
    }

    #[test]
    fn test_no_truncation_needed() {
        let msgs = vec![user_msg("hello"), assistant_msg("hi")];
        let result = select_messages(&msgs, 100, 0); // 0 = 不限制
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, ROLE_USER);
        assert_eq!(result[1].role, ROLE_ASSISTANT);
    }

    #[test]
    fn test_tool_group_dropped_first() {
        // U1 → A1(text) → TG1(Shell+result) → U2 → A2(text)
        let msgs = vec![
            user_msg("do something"),
            assistant_msg("let me check"),
            tool_call_msg(&["Shell"]),
            tool_result_msg("call_0", &"huge output ".repeat(1000)),
            user_msg("what about this"),
            assistant_msg("here's the answer"),
        ];

        // 设置极小预算，迫使丢弃 (1K tokens)
        let result = select_messages(&msgs, 100, 1);

        // User 和 AssistantText 应该保留，ToolGroup 应该被占位符替换
        assert!(result.iter().any(|m| m.role == ROLE_USER));
        assert!(result.iter().any(|m| m.role == ROLE_TOOL) == false); // tool result 被丢弃
        assert!(result.iter().any(|m| m.content.contains("tool calls"))); // 占位符
    }

    #[test]
    fn test_time_order_preserved() {
        let msgs = vec![
            user_msg("first"),
            assistant_msg("ok1"),
            tool_call_msg(&["Shell"]),
            tool_result_msg("call_0", "output"),
            user_msg("second"),
            assistant_msg("ok2"),
        ];

        let result = select_messages(&msgs, 100, 0); // 0 = 不限制

        // 时间顺序保持
        let user_positions: Vec<usize> = result
            .iter()
            .enumerate()
            .filter(|(_, m)| m.role == ROLE_USER)
            .map(|(i, _)| i)
            .collect();
        assert!(user_positions[0] < user_positions[1]);
    }

    #[test]
    fn test_placeholder_format() {
        let msgs = vec![
            user_msg("run"),
            tool_call_msg(&["Shell", "Read"]),
            tool_result_msg("call_0", &"x".repeat(2000)),
            tool_result_msg("call_1", &"y".repeat(2000)),
        ];

        // 极小 token 预算迫使 ToolGroup 丢弃 (1K tokens)
        let result = select_messages(&msgs, 100, 1);

        let placeholder = result.iter().find(|m| m.content.contains("tool calls"));
        assert!(placeholder.is_some());
        let p = placeholder.unwrap();
        assert!(p.content.contains("Shell"));
        assert!(p.content.contains("Read"));
        assert!(p.tool_calls.is_none());
    }
}
