//! 消息压缩模块：压缩来自其他 agent 的 tool call 消息
//!
//! 在 teammate/subagent 调用 LLM 前，对消息列表进行压缩：
//! - 保留最近 N 条完整的 tool call 消息
//! - 较早的消息压缩为摘要，减少上下文占用
//!
//! # 上下文注入机制（Feature，非 Bug）
//!
//! SubAgent/Teammate 通过各自推送逻辑写入双通道：
//! - `display_messages`：干净文本 + sender_name 字段 → UI 渲染
//! - `context_messages`：XML 包裹（如 `<AgentName>text</AgentName>`） → LLM context
//!
//! 数据流：
//! - `display_messages` → UI 渲染数据源
//! - `context_messages` → `build_api_messages`（LLM context 数据源）
//!
//! 本模块的压缩功能用于减少这些消息对上下文的占用，而非完全过滤它们。
//! 参见 `agent/tool_processor.rs` 中 `push_both` 函数的文档注释。
//!
//! # 压缩策略
//!
//! 广播消息格式：`<Type: AgentName> [调用工具 ToolName]`
//! - 按 agent 来源分组
//! - 保留最近 threshold 条完整消息
//! - 较早的合并为摘要：`<Type: AgentName> [早期工具调用摘要: ToolA×5, ToolB×3, 共 8 次]`

use crate::storage::{ChatMessage, MessageRole};
use std::collections::HashMap;

/// 默认压缩阈值：保留最近 5 条完整消息
pub const DEFAULT_OTHER_AGENT_TOOLCALL_THRESHOLD: usize = 5;

/// 从消息内容中提取 agent 来源
///
/// 广播消息格式：`<Type: AgentName> message` 或 `<AgentName> message`（旧格式兼容）
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
/// 格式：`<Type: AgentName> [调用工具 ToolName]` 或 `<AgentName> [调用工具 ToolName]`
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
            // 处理 User role（通过 pending_user_messages 接收的广播）
            // 和 Assistant role（通过 context_messages 同步的 teammate/subagent 活动消息）
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
    let agent_groups: HashMap<String, Vec<(usize, String)>> =
        other_agent_tool_calls
            .iter()
            .fold(HashMap::new(), |mut acc, (idx, agent, tool)| {
                acc.entry(agent.clone())
                    .or_default()
                    .push((*idx, tool.clone()));
                acc
            });

    // 3. 对于每个 agent，决定哪些索引需要压缩
    let mut indices_to_compress: Vec<usize> = Vec::new(); // 大小依赖运行时计算
    let mut summary_by_first_idx: HashMap<usize, (String, HashMap<String, usize>)> = HashMap::new();

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
        let tool_counts: HashMap<String, usize> =
            to_compress
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
    let mut result: Vec<ChatMessage> = Vec::with_capacity(messages.len());
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
                "<{}> [早期工具调用摘要: {}, 共 {} 次]</{}>",
                agent_name, tools_summary, total_calls, agent_name
            );
            result.push(ChatMessage::text(MessageRole::User, summary_content));
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
mod tests;
