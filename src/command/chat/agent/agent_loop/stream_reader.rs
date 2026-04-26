//! 流式响应读取 + 错误恢复 + 非流式 fallback
//!
//! 对应原 `run_main_agent_loop` 中创建流式请求、读取流、处理可重试错误、
//! 以及 deserialize_failed / stream_empty 时的非流式 fallback 路径。

use super::super::config::AgentLoopConfig;
use super::super::tool_processor::{
    ToolCallContext, flush_streaming_as_message, process_tool_calls,
};
use super::compact_phase::push_compact_tool_messages;
use crate::command::chat::app::types::StreamMsg;
use crate::command::chat::context::compact::{self, AutoCompactParams, InvokedSkillsMap};
use crate::command::chat::error::ChatError;
use crate::command::chat::storage::{ChatMessage, MessageRole};
use crate::llm::{ChatRequest, ChatStreamChunk, LlmClient, SseStream};
use crate::util::log::{write_error_log, write_info_log};
use crate::util::safe_lock;
use futures::StreamExt;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// 调试日志中记录的最大 chunk 数量
const DEBUG_LOG_CHUNK_LIMIT: u32 = 3;
/// reasoning 内容日志输出的最小长度阈值
const REASONING_LOG_THRESHOLD: usize = 50;

/// 流式响应中逐步聚合的工具调用片段（按 chunk index 聚合 id/name/arguments）
pub(super) struct StreamingToolCallPart {
    pub call_id: String,
    pub function_name: String,
    pub function_arguments: String,
}

/// 读取流式响应后的产出物
pub(super) struct StreamReadResult {
    pub finish_reason: Option<String>,
    pub assistant_text: String,
    pub assistant_reasoning: String,
    pub active_tool_call_parts: BTreeMap<u32, StreamingToolCallPart>,
    pub deserialize_failed: bool,
    pub received_chunks: u32,
    /// 需要压缩上下文恢复 tool_call_id 不一致
    pub needs_compact_for_tool_id_mismatch: bool,
    /// 流式读取中途遇到的可重试错误
    pub stream_retriable_error: Option<ChatError>,
    /// 是否应该 return（致命错误已发送到 tx）
    pub should_return: bool,
}

/// 流式请求 + 读取过程的运行时上下文
pub(super) struct StreamCreateContext<'a> {
    pub client: &'a LlmClient,
    pub request: &'a ChatRequest,
    pub cancel_token: &'a CancellationToken,
    pub tx: &'a mpsc::Sender<StreamMsg>,
    pub retry_attempt: u32,
}

/// 创建流式请求的结果
pub(super) enum StreamCreateResult {
    /// 成功创建流
    Ok(SseStream),
    /// 需要重试（retry_attempt 已更新）
    Retry { retry_attempt: u32 },
    /// 应直接 return（错误已发送）
    Return,
}

/// 创建流式请求，处理创建失败时的重试逻辑。
pub(super) async fn create_stream_with_retry(ctx: &StreamCreateContext<'_>) -> StreamCreateResult {
    use super::super::retry::{backoff_delay_ms, retry_policy_for};

    write_info_log(
        "agent_loop",
        &format!("开始创建流式请求 (attempt={})...", ctx.retry_attempt),
    );
    match ctx.client.chat_completion_stream(ctx.request).await {
        Ok(s) => {
            write_info_log("agent_loop", "流式请求创建成功");
            StreamCreateResult::Ok(s)
        }
        Err(e) => {
            let err = ChatError::from(e);
            write_error_log("Chat API 流式请求创建", &err.to_string());
            if let Some(policy) = retry_policy_for(&err)
                && ctx.retry_attempt <= policy.max_attempts
            {
                let delay_ms = backoff_delay_ms(ctx.retry_attempt, policy.base_ms, policy.cap_ms);
                write_info_log(
                    "agent_loop",
                    &format!(
                        "流式创建失败，{}ms 后重试 ({}/{})",
                        delay_ms, ctx.retry_attempt, policy.max_attempts
                    ),
                );
                let _ = ctx.tx.send(StreamMsg::Retrying {
                    attempt: ctx.retry_attempt,
                    max_attempts: policy.max_attempts,
                    delay_ms,
                    error: err.display_message(),
                });
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {
                        StreamCreateResult::Retry { retry_attempt: ctx.retry_attempt + 1 }
                    }
                    _ = ctx.cancel_token.cancelled() => {
                        let _ = ctx.tx.send(StreamMsg::Cancelled);
                        StreamCreateResult::Return
                    }
                }
            } else {
                let _ = ctx.tx.send(StreamMsg::Error(err));
                StreamCreateResult::Return
            }
        }
    }
}

/// 读取流式响应，收集文本/reasoning/tool_calls，处理中途错误。
pub(super) async fn read_stream(
    stream: &mut SseStream,
    streaming_content: &Arc<Mutex<String>>,
    streaming_reasoning_content: &Arc<Mutex<String>>,
    cancel_token: &CancellationToken,
    tx: &mpsc::Sender<StreamMsg>,
) -> StreamReadResult {
    let mut finish_reason: Option<String> = None;
    let mut assistant_text = String::new();
    let mut assistant_reasoning = String::new();
    let mut active_tool_call_parts: BTreeMap<u32, StreamingToolCallPart> = BTreeMap::new();
    let mut deserialize_failed = false;
    let mut needs_compact_for_tool_id_mismatch = false;
    let mut stream_retriable_error: Option<ChatError> = None;
    let mut received_chunks: u32 = 0;

    'stream: loop {
        tokio::select! {
            result = stream.next() => {
                match result {
                    Some(Ok(response)) => {
                        let mut acc = ChunkAccumulator {
                            received_chunks: &mut received_chunks,
                            assistant_text: &mut assistant_text,
                            assistant_reasoning: &mut assistant_reasoning,
                            active_tool_call_parts: &mut active_tool_call_parts,
                            finish_reason: &mut finish_reason,
                            streaming_content,
                            streaming_reasoning_content,
                            tx,
                        };
                        process_stream_chunk(response, &mut acc);
                    }
                    Some(Err(e)) => {
                        let error_str = e.to_string();
                        write_error_log("Chat API 流式响应 error", &error_str);
                        let err = ChatError::from(e);
                        if matches!(err, ChatError::StreamDeserialize(_))
                            || error_str.contains("missing field `index`")
                            || error_str.contains("tool_calls")
                        {
                            write_info_log(
                                "Chat API 流式响应",
                                &format!("检测到反序列化错误，将 fallback 到非流式: {}", err),
                            );
                            deserialize_failed = true;
                            break 'stream;
                        }
                        if matches!(&err, ChatError::ApiBadRequest(msg) if msg.contains("tool_call_id")) {
                            write_error_log(
                                "Chat API 流式响应",
                                &format!("检测到 tool_call_id 不一致错误，将压缩上下文后重试: {}", err),
                            );
                            needs_compact_for_tool_id_mismatch = true;
                            break 'stream;
                        }
                        if super::super::retry::retry_policy_for(&err).is_some() {
                            stream_retriable_error = Some(err);
                            break 'stream;
                        }
                        write_error_log("Chat API 流式响应（不可重试）", &err.to_string());
                        let _ = tx.send(StreamMsg::Error(err));
                        return StreamReadResult {
                            finish_reason, assistant_text, assistant_reasoning,
                            active_tool_call_parts, deserialize_failed, received_chunks,
                            needs_compact_for_tool_id_mismatch, stream_retriable_error,
                            should_return: true,
                        };
                    }
                    None => {
                        write_info_log("agent_loop", "流式结束 (stream returned None)");
                        break 'stream;
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                let _ = tx.send(StreamMsg::Cancelled);
                return StreamReadResult {
                    finish_reason, assistant_text, assistant_reasoning,
                    active_tool_call_parts, deserialize_failed, received_chunks,
                    needs_compact_for_tool_id_mismatch, stream_retriable_error,
                    should_return: true,
                };
            }
        }
    }

    StreamReadResult {
        finish_reason,
        assistant_text,
        assistant_reasoning,
        active_tool_call_parts,
        deserialize_failed,
        received_chunks,
        needs_compact_for_tool_id_mismatch,
        stream_retriable_error,
        should_return: false,
    }
}

/// 处理单个流式 chunk 的可变状态
struct ChunkAccumulator<'a> {
    received_chunks: &'a mut u32,
    assistant_text: &'a mut String,
    assistant_reasoning: &'a mut String,
    active_tool_call_parts: &'a mut BTreeMap<u32, StreamingToolCallPart>,
    finish_reason: &'a mut Option<String>,
    streaming_content: &'a Arc<Mutex<String>>,
    streaming_reasoning_content: &'a Arc<Mutex<String>>,
    tx: &'a mpsc::Sender<StreamMsg>,
}

/// 处理单个流式 chunk
fn process_stream_chunk(chunk: ChatStreamChunk, acc: &mut ChunkAccumulator<'_>) {
    *acc.received_chunks += 1;
    if *acc.received_chunks <= DEBUG_LOG_CHUNK_LIMIT {
        let choices_debug: Vec<String> = chunk
            .choices
            .iter()
            .map(|choice| {
                format!(
                    "idx={}, finish_reason={:?}, has_content={}, has_tool_calls={}",
                    choice.index,
                    choice.finish_reason,
                    choice.delta.content.is_some(),
                    choice.delta.tool_calls.is_some(),
                )
            })
            .collect();
        write_info_log(
            "stream_chunk",
            &format!(
                "chunk #{}: choices=[{}]",
                *acc.received_chunks,
                choices_debug.join("; ")
            ),
        );
    }
    for choice in &chunk.choices {
        if let Some(ref content) = choice.delta.content {
            acc.assistant_text.push_str(content);
            let mut stream_buf = safe_lock(acc.streaming_content, "agent::stream_chunk");
            stream_buf.push_str(content);
            drop(stream_buf);
            let _ = acc.tx.send(StreamMsg::Chunk);
        }
        if let Some(ref reasoning) = choice.delta.reasoning_content {
            acc.assistant_reasoning.push_str(reasoning);
            {
                let mut reason_buf =
                    safe_lock(acc.streaming_reasoning_content, "agent::stream_reasoning");
                reason_buf.push_str(reasoning);
            }
            let _ = acc.tx.send(StreamMsg::Chunk);
        }
        if !acc.assistant_reasoning.is_empty()
            && choice.delta.reasoning_content.is_some()
            && acc.assistant_reasoning.len() < REASONING_LOG_THRESHOLD
        {
            write_info_log(
                "agent_loop",
                &format!("reasoning积累中 len={}", acc.assistant_reasoning.len()),
            );
        }
        if let Some(ref toolcall_chunks) = choice.delta.tool_calls {
            for tc in toolcall_chunks {
                let entry = acc
                    .active_tool_call_parts
                    .entry(tc.index)
                    .or_insert_with(|| StreamingToolCallPart {
                        call_id: tc.id.clone().unwrap_or_default(),
                        function_name: String::new(),
                        function_arguments: String::new(),
                    });
                if entry.call_id.is_empty()
                    && let Some(ref id) = tc.id
                {
                    entry.call_id = id.clone();
                }
                if let Some(ref tool_function) = tc.function {
                    if let Some(ref name) = tool_function.name {
                        entry.function_name.push_str(name);
                    }
                    if let Some(ref args) = tool_function.arguments {
                        entry.function_arguments.push_str(args);
                    }
                }
            }
        }
        if let Some(ref finish_reason_val) = choice.finish_reason {
            *acc.finish_reason = Some(finish_reason_val.clone());
        }
    }
}

/// 处理流式读取中途的可重试错误。
pub(super) enum RetryDecision {
    /// 继续重试；`error_message` 用于 StreamMsg::Retrying 的 UI 提示
    Retry {
        delay_ms: u64,
        max_attempts: u32,
        error_message: String,
    },
    /// 重试耗尽或不可重试：调用方应向 tx 发送 StreamMsg::Error(err) 后 return
    Return { err: ChatError },
}

pub(super) fn handle_stream_retriable_error(
    err: ChatError,
    retry_attempt: u32,
    streaming_content: &Arc<Mutex<String>>,
    streaming_reasoning_content: &Arc<Mutex<String>>,
) -> RetryDecision {
    use super::super::retry::{backoff_delay_ms, retry_policy_for};

    write_error_log("Chat API 流式响应（将重试）", &err.to_string());
    if let Some(policy) = retry_policy_for(&err)
        && retry_attempt <= policy.max_attempts
    {
        {
            let mut stream_buf = safe_lock(streaming_content, "agent::stream_retry_clear");
            stream_buf.clear();
        }
        {
            let mut reason_buf = safe_lock(
                streaming_reasoning_content,
                "agent::stream_retry_reason_clear",
            );
            reason_buf.clear();
        }
        let delay_ms = backoff_delay_ms(retry_attempt, policy.base_ms, policy.cap_ms);
        write_info_log(
            "agent_loop",
            &format!(
                "流式中断，{}ms 后重试 ({}/{})",
                delay_ms, retry_attempt, policy.max_attempts
            ),
        );
        RetryDecision::Retry {
            delay_ms,
            max_attempts: policy.max_attempts,
            error_message: err.display_message(),
        }
    } else {
        RetryDecision::Return { err }
    }
}

/// fallback 非流式调用的运行时上下文
pub(super) struct FallbackContext<'a> {
    pub config: &'a AgentLoopConfig,
    pub messages: &'a mut Vec<ChatMessage>,
    pub display_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub context_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub streaming_content: &'a Arc<Mutex<String>>,
    pub streaming_reasoning_content: &'a Arc<Mutex<String>>,
    pub invoked_skills: &'a InvokedSkillsMap,
    pub session_id: &'a str,
    pub tx: &'a mpsc::Sender<StreamMsg>,
    pub tool_ctx: &'a ToolCallContext<'a>,
    pub cancel_token: &'a CancellationToken,
    pub pending_user_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
}

/// 流式路径决策结果
pub(super) enum FallbackResult {
    ContinueRound,
    BreakRound,
    Return,
}

/// 处理 deserialize_failed / stream_empty 的非流式 fallback 路径。
pub(super) async fn handle_fallback(
    ctx: &mut FallbackContext<'_>,
    request: &ChatRequest,
    retry_attempt: &mut u32,
) -> FallbackResult {
    use super::super::api::call_llm_non_stream;
    use super::super::retry::{backoff_delay_ms, retry_policy_for};
    use super::super::tool_processor::clear_channels;

    // 清空流式内容
    {
        let mut stream_buf = safe_lock(ctx.streaming_content, "agent::fallback_clear");
        stream_buf.clear();
    }
    {
        let mut reason_buf = safe_lock(
            ctx.streaming_reasoning_content,
            "agent::fallback_reason_clear",
        );
        reason_buf.clear();
    }

    // 非流式调用（含重试）
    let fallback_result = loop {
        let create_fut = call_llm_non_stream(&ctx.config.provider, request);
        let result = tokio::select! {
            result = create_fut => result,
            _ = ctx.cancel_token.cancelled() => {
                let _ = ctx.tx.send(StreamMsg::Cancelled);
                return FallbackResult::Return;
            }
        };
        match result {
            Ok(r) => break r,
            Err(e) => {
                write_error_log("Sprite API fallback 非流式", &e.to_string());
                if let Some(policy) = retry_policy_for(&e)
                    && *retry_attempt <= policy.max_attempts
                {
                    let delay_ms = backoff_delay_ms(*retry_attempt, policy.base_ms, policy.cap_ms);
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "fallback 非流式失败，{}ms 后重试 ({}/{})",
                            delay_ms, *retry_attempt, policy.max_attempts
                        ),
                    );
                    let _ = ctx.tx.send(StreamMsg::Retrying {
                        attempt: *retry_attempt,
                        max_attempts: policy.max_attempts,
                        delay_ms,
                        error: e.display_message(),
                    });
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {
                            *retry_attempt += 1;
                            continue;
                        }
                        _ = ctx.cancel_token.cancelled() => {
                            let _ = ctx.tx.send(StreamMsg::Cancelled);
                            return FallbackResult::Return;
                        }
                    }
                }
                let _ = ctx.tx.send(StreamMsg::Error(e));
                return FallbackResult::Return;
            }
        }
    };

    write_info_log(
        "agent_loop",
        &format!(
            "fallback 非流式结果: has_tool_calls={}, has_content={}, finish_reason={:?}",
            fallback_result.has_tool_calls(),
            fallback_result
                .content
                .as_ref()
                .map(|c| c.len())
                .unwrap_or(0),
            fallback_result.finish_reason
        ),
    );

    // 工具调用
    if fallback_result.has_tool_calls()
        && let Some(tool_items) = fallback_result.tool_calls
    {
        if tool_items.is_empty() {
            write_info_log("agent_loop", "fallback tool_calls 为空列表，break 'round");
            return FallbackResult::BreakRound;
        }
        let assistant_text: String = fallback_result.content.clone().unwrap_or_default();
        match process_tool_calls(
            tool_items,
            assistant_text,
            ctx.messages,
            ctx.tool_ctx,
            fallback_result.reasoning_content.clone(),
        ) {
            Ok(result) => {
                if result.compact_requested && ctx.config.compact_config.enabled {
                    let _ = ctx.tx.send(StreamMsg::Compacting);
                    if let Ok(compact_result) = compact::auto_compact(
                        ctx.messages,
                        &AutoCompactParams {
                            provider: &ctx.config.provider,
                            invoked_skills: ctx.invoked_skills,
                            session_id: ctx.session_id,
                            protected_context: None,
                        },
                    )
                    .await
                    {
                        clear_channels(ctx.display_messages, ctx.context_messages);
                        push_compact_tool_messages(
                            ctx.messages,
                            ctx.display_messages,
                            ctx.context_messages,
                            &compact_result,
                        );
                        let _ = ctx.tx.send(StreamMsg::Compacted {
                            messages_before: compact_result.messages_before,
                        });
                    }
                }
                if let Some(ref plan_content) = result.plan_with_context_clear {
                    write_info_log(
                        "agent_loop",
                        "Clearing context after plan approval (fallback path)",
                    );
                    ctx.messages.clear();
                    if let Ok(mut shared) = ctx.display_messages.lock() {
                        shared.clear();
                    }
                    if let Ok(mut shared) = ctx.context_messages.lock() {
                        shared.clear();
                    }
                    let plan_msg = ChatMessage::text(
                        MessageRole::User,
                        format!("以下计划已获批准，请按计划执行：\n\n{}", plan_content),
                    );
                    ctx.messages.push(plan_msg);
                }
                return FallbackResult::ContinueRound;
            }
            Err(e) => {
                write_error_log("agent_loop", &format!("process_tool_calls failed: {}", e));
                return FallbackResult::Return;
            }
        }
    }

    // 普通文本回复
    if let Some(ref content) = fallback_result.content
        && !content.is_empty()
    {
        write_info_log("Sprite 回复", content);
        let mut stream_buf = safe_lock(ctx.streaming_content, "agent::fallback_content");
        stream_buf.push_str(content);
        drop(stream_buf);
        let _ = ctx.tx.send(StreamMsg::Chunk);
    }
    if let Some(ref reason) = fallback_result.finish_reason
        && !matches!(
            reason.as_str(),
            "stop" | "length" | "tool_calls" | "content_filter" | "function_call"
        )
        && fallback_result
            .content
            .as_deref()
            .unwrap_or_default()
            .is_empty()
    {
        let error_msg = ChatError::AbnormalFinish(reason.clone());
        write_error_log("Sprite API fallback 非流式", &error_msg.to_string());
        let _ = ctx.tx.send(StreamMsg::Error(error_msg));
        return FallbackResult::Return;
    }

    let has_pending =
        !safe_lock(ctx.pending_user_messages, "agent::pending_check_fallback").is_empty();
    write_info_log(
        "agent_loop",
        &format!("fallback 正常结束，pending_user_messages={}", has_pending),
    );
    if has_pending {
        flush_streaming_as_message(
            ctx.streaming_content,
            ctx.streaming_reasoning_content,
            ctx.messages,
            ctx.display_messages,
            ctx.context_messages,
            fallback_result.reasoning_content.clone(),
        );
        write_info_log("agent_loop", "有用户增量消息，continue 'round");
        FallbackResult::ContinueRound
    } else {
        write_info_log("agent_loop", "无用户增量消息，break 'round (fallback 路径)");
        FallbackResult::BreakRound
    }
}
