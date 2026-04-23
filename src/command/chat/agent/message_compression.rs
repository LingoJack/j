//! 消息压缩模块：压缩来自其他 agent 的 tool call 消息
//!
//! 在 teammate/subagent 调用 LLM 前，对消息列表进行压缩：
//! - 保留最近 N 条完整的 tool call 消息
//! - 较早的消息压缩为摘要，减少上下文占用
//!
//! # 压缩策略
//!
//! 广播消息格式：`<AgentName> [调用工具 ToolName]`
//! - 按 agent 来源分组
//! - 保留最近 threshold 条完整消息
//! - 较早的合并为摘要：`<AgentName> [早期工具调用摘要: ToolA×5, ToolB×3, 共 8 次]`

use crate::command::chat::storage::ChatMessage;
use std::collections::HashMap;

/// 默认压缩阈值：保留最近 5 条完整消息
pub const DEFAULT_OTHER_AGENT_TOOLCALL_THRESHOLD: usize = 5;

/// 从消息内容中提取 agent 来源
///
/// 广播消息格式：`<AgentName> message`
/// 返回 (agent_name, remainder) 或 None（非广播消息）
fn extract_agent_source(content: &str) -> Option<(String, &str)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with('<') {
        return None;
    }
    let end_bracket = trimmed.find('>')?;
    let agent_name = trimmed[1..end_bracket].to_string();
    let remainder = &trimmed[end_bracket + 1..];
    Some((agent_name, remainder))
}

/// 判断是否为 tool call 广播消息
///
/// 格式：`<AgentName> [调用工具 ToolName]`
fn is_tool_call_broadcast(content: &str) -> Option<(String, String)> {
    let (agent_name, remainder) = extract_agent_source(content)?;
    let trimmed = remainder.trim_start();
    if !trimmed.starts_with("[调用工具 ") {
        return None;
    }
    let end_bracket = trimmed.find(']')?;
    let tool_name = trimmed["[调用工具 ".len()..end_bracket].to_string();
    Some((agent_name, tool_name))
}

/// 压缩来自其他 agent 的 tool call 消息
///
/// # 参数
///
/// - `messages`: 原始消息列表
/// - `self_agent_name`: 当前 agent 名（Main/SubAgent 名或 Teammate 名）
/// - `threshold`: 保留最近多少条完整消息
///
/// # 返回
///
/// 压缩后的消息列表：
/// - 自己的消息保留完整
/// - 其他 agent 的 tool call：最近 threshold 条保留，较早的压缩为摘要
pub fn compress_other_agent_toolcalls(
    messages: &[ChatMessage],
    self_agent_name: &str,
    threshold: usize,
) -> Vec<ChatMessage> {
    if messages.is_empty() || threshold == 0 {
        return messages.to_vec();
    }

    // 1. 收集所有来自其他 agent 的 tool call 消息及其索引
    let other_agent_tool_calls: Vec<(usize, String, String)> = messages
        .iter()
        .enumerate()
        .filter_map(|(idx, msg)| {
            // 只处理 User role 的广播消息（teammate/subagent 通过 pending_user_messages 接收）
            if msg.role != crate::command::chat::storage::MessageRole::User {
                return None;
            }
            let content = &msg.content;
            let (agent_name, tool_name) = is_tool_call_broadcast(content)?;
            // 排除自己发出的 tool call 广播
            if agent_name == self_agent_name {
                return None;
            }
            Some((idx, agent_name, tool_name))
        })
        .collect();

    // 无其他 agent 的 tool call，直接返回原列表
    if other_agent_tool_calls.is_empty() {
        return messages.to_vec();
    }

    // 2. 按 agent 来源分组，记录每个 agent 的消息索引和工具名
    let agent_groups: HashMap<String, Vec<(usize, String)>> = other_agent_tool_calls
        .iter()
        .fold(HashMap::new(), |mut acc, (idx, agent, tool)| {
            acc.entry(agent.clone()).or_default().push((*idx, tool.clone()));
            acc
        });

    // 3. 对于每个 agent，决定哪些索引需要压缩
    let mut indices_to_compress: Vec<usize> = Vec::new();
    let mut summary_by_first_idx: HashMap<usize, (String, HashMap<String, usize>)> =
        HashMap::new();

    for (agent_name, calls) in agent_groups {
        let total = calls.len();
        if total <= threshold {
            // 不需要压缩
            continue;
        }

        // 保留最近 threshold 条（索引大的）
        let recent_start = total - threshold;
        let (to_compress, _to_keep) = calls.split_at(recent_start);

        // 记录需要压缩的索引
        for (idx, _) in to_compress {
            indices_to_compress.push(*idx);
        }

        // 统计压缩部分的工具调用次数
        let tool_counts: HashMap<String, usize> = to_compress
            .iter()
            .fold(HashMap::new(), |mut acc, (_, tool)| {
                *acc.entry(tool.clone()).or_default() += 1;
                acc
            });

        // 记录摘要信息，放在该 agent 最早出现的位置（to_compress 的第一个索引）
        if let Some((first_idx, _)) = to_compress.first() {
            summary_by_first_idx.insert(*first_idx, (agent_name.clone(), tool_counts));
        }
    }

    // 4. 构建压缩后的消息列表
    let mut result: Vec<ChatMessage> = Vec::new();
    for (idx, msg) in messages.iter().enumerate() {
        if let Some((agent_name, tool_counts)) = summary_by_first_idx.get(&idx) {
            // 在此位置插入压缩摘要
            let total_calls: usize = tool_counts.values().sum();
            let tools_summary: String = tool_counts
                .iter()
                .map(|(tool, count)| format!("{}×{}", tool, count))
                .collect::<Vec<_>>()
                .join(", ");
            let summary_content = format!(
                "<{}> [早期工具调用摘要: {}, 共 {} 次]",
                agent_name, tools_summary, total_calls
            );
            result.push(ChatMessage::text(
                crate::command::chat::storage::MessageRole::User,
                summary_content,
            ));
        }

        if indices_to_compress.contains(&idx) {
            // 跳过被压缩的消息
            continue;
        }

        // 保留原消息（未被压缩的）
        result.push(msg.clone());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::chat::storage::{ChatMessage, MessageRole};

    #[test]
    fn test_extract_agent_source() {
        assert_eq!(
            extract_agent_source("<Frontend> hello"),
            Some(("Frontend".to_string(), " hello"))
        );
        assert_eq!(
            extract_agent_source("  <Backend> [调用工具 Read]"),
            Some(("Backend".to_string(), " [调用工具 Read]"))
        );
        assert_eq!(extract_agent_source("no prefix"), None);
        assert_eq!(extract_agent_source("<no-close"), None);
    }

    #[test]
    fn test_is_tool_call_broadcast() {
        assert_eq!(
            is_tool_call_broadcast("<Frontend> [调用工具 Read]"),
            Some(("Frontend".to_string(), "Read".to_string()))
        );
        assert_eq!(
            is_tool_call_broadcast("<Backend>  [调用工具 Edit] "),
            Some(("Backend".to_string(), "Edit".to_string()))
        );
        // 不是 tool call
        assert_eq!(
            is_tool_call_broadcast("<Frontend> hello world"),
            None
        );
        // 不是广播格式
        assert_eq!(is_tool_call_broadcast("regular message"), None);
    }

    #[test]
    fn test_compress_no_other_agent() {
        // 只有自己的消息，不压缩
        let messages = vec![
            ChatMessage::text(MessageRole::User, "user question".to_string()),
            ChatMessage::text(MessageRole::Assistant, "response".to_string()),
        ];
        let result = compress_other_agent_toolcalls(&messages, "Main", 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "user question");
        assert_eq!(result[1].content, "response");
    }

    #[test]
    fn test_compress_within_threshold() {
        // 其他 agent 的 tool call 数量 <= threshold，不压缩
        let messages = vec![
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Read]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Edit]".to_string()),
        ];
        let result = compress_other_agent_toolcalls(&messages, "Backend", 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "<Frontend> [调用工具 Read]");
        assert_eq!(result[1].content, "<Frontend> [调用工具 Edit]");
    }

    #[test]
    fn test_compress_exceed_threshold() {
        // 其他 agent 的 tool call 数量 > threshold，压缩早期消息
        let messages = vec![
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Read]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Edit]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Bash]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Read]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Edit]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Bash]".to_string()),
        ];
        let result = compress_other_agent_toolcalls(&messages, "Backend", 3);
        // 前 3 条压缩为摘要 + 后 3 条保留 = 4 条
        assert_eq!(result.len(), 4);
        // 第一条应该是摘要
        assert!(result[0].content.contains("[早期工具调用摘要"));
        assert!(result[0].content.contains("Read×1"));
        assert!(result[0].content.contains("Edit×1"));
        assert!(result[0].content.contains("Bash×1"));
        assert!(result[0].content.contains("共 3 次"));
        // 后 3 条保留完整
        assert_eq!(result[1].content, "<Frontend> [调用工具 Read]");
        assert_eq!(result[2].content, "<Frontend> [调用工具 Edit]");
        assert_eq!(result[3].content, "<Frontend> [调用工具 Bash]");
    }

    #[test]
    fn test_compress_multiple_agents() {
        // 多个其他 agent，各自独立压缩
        let messages = vec![
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Read]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Edit]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Bash]".to_string()),
            ChatMessage::text(MessageRole::User, "<Backend> [调用工具 Bash]".to_string()),
            ChatMessage::text(MessageRole::User, "<Backend> [调用工具 Edit]".to_string()),
            ChatMessage::text(MessageRole::User, "<Backend> [调用工具 Read]".to_string()),
        ];
        let result = compress_other_agent_toolcalls(&messages, "DevOps", 2);
        // Frontend: 1 摘要 + 2 保留 = 3
        // Backend: 1 摘要 + 2 保留 = 3
        // 总共 6 条消息
        assert_eq!(result.len(), 6);
        // 验证 Frontend 摘要
        assert!(result[0].content.contains("<Frontend>"));
        assert!(result[0].content.contains("[早期工具调用摘要"));
        // 验证 Backend 摘要（在其第一条消息的位置）
        let backend_summary_idx = result
            .iter()
            .position(|m| m.content.contains("<Backend> [早期工具调用摘要"))
            .expect("Backend summary should exist");
        assert!(result[backend_summary_idx].content.contains("Bash×1"));
    }

    #[test]
    fn test_compress_preserves_other_messages() {
        // 其他类型的消息保留完整
        let messages = vec![
            ChatMessage::text(MessageRole::User, "user question".to_string()),
            ChatMessage::text(MessageRole::Assistant, "assistant response".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Read]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Edit]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Bash]".to_string()),
            ChatMessage::text(MessageRole::User, "<Frontend> [调用工具 Read]".to_string()),
            ChatMessage::text(MessageRole::User, "another user message".to_string()),
        ];
        let result = compress_other_agent_toolcalls(&messages, "Backend", 2);
        // user question + assistant response + 摘要 + 保留的 2 条 tool call + another user message
        assert!(result.iter().any(|m| m.content == "user question"));
        assert!(result.iter().any(|m| m.content == "assistant response"));
        assert!(result.iter().any(|m| m.content == "another user message"));
    }
}