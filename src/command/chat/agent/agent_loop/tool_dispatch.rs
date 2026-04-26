//! 流式路径下的工具调用处理 + Stop hook
//!
//! 对应原 `run_main_agent_loop` 中流式读取完成后的工具调用调度、
//! compact tool 触发、plan 上下文清空、Stop hook 等逻辑。

use super::super::config::AgentLoopConfig;
use super::super::tool_processor::{
    ToolCallContext, flush_streaming_as_message, process_tool_calls, push_both,
};
use super::compact_phase::push_compact_tool_messages;
use super::stream_reader::StreamingToolCallPart;
use crate::command::chat::app::types::StreamMsg;
use crate::command::chat::context::compact::{self, AutoCompactParams, InvokedSkillsMap};
use crate::command::chat::error::ChatError;
use crate::command::chat::infra::hook::{HookContext, HookEvent};
use crate::command::chat::storage::{ChatMessage, MessageRole, ToolCallItem};
use crate::util::log::{write_error_log, write_info_log};
use crate::util::safe_lock;
use rand::Rng;
use std::collections::BTreeMap;
use std::mem::take;
use std::sync::{Arc, Mutex, mpsc};

/// 流式路径决策结果
pub(super) enum ToolDispatchResult {
    ContinueRound,
    BreakRound,
    Return,
}

/// 流式路径工具调用的运行时上下文
pub(super) struct ToolDispatchContext<'a> {
    pub config: &'a AgentLoopConfig,
    pub messages: &'a mut Vec<ChatMessage>,
    pub display_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub context_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub streaming_content: &'a Arc<Mutex<String>>,
    pub streaming_reasoning_content: &'a Arc<Mutex<String>>,
    pub invoked_skills: &'a InvokedSkillsMap,
    pub session_id: &'a str,
    pub system_prompt: &'a mut Option<String>,
    pub tx: &'a mpsc::Sender<StreamMsg>,
    pub tool_ctx: &'a ToolCallContext<'a>,
    pub pending_user_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
}

/// 处理流式路径下的工具调用 / 正常结束决策。
pub(super) async fn handle_streaming_tool_calls(
    ctx: &mut ToolDispatchContext<'_>,
    active_tool_call_parts: BTreeMap<u32, StreamingToolCallPart>,
    assistant_text: String,
    mut assistant_reasoning: String,
    finish_reason: &Option<String>,
) -> ToolDispatchResult {
    let has_tool_calls = !active_tool_call_parts.is_empty();
    write_info_log(
        "agent_loop",
        &format!(
            "流式路径决策: has_tool_calls={}, finish_reason={:?}",
            has_tool_calls, finish_reason
        ),
    );

    if has_tool_calls {
        dispatch_tool_calls(
            ctx,
            active_tool_call_parts,
            assistant_text,
            &mut assistant_reasoning,
            finish_reason,
        )
        .await
    } else {
        handle_no_tool_calls(ctx, assistant_text, &mut assistant_reasoning).await
    }
}

/// 有工具调用时的处理
async fn dispatch_tool_calls(
    ctx: &mut ToolDispatchContext<'_>,
    active_tool_call_parts: BTreeMap<u32, StreamingToolCallPart>,
    assistant_text: String,
    assistant_reasoning: &mut String,
    finish_reason: &Option<String>,
) -> ToolDispatchResult {
    let finish_reason_is_tool_calls = finish_reason.as_deref() == Some("tool_calls");
    if !finish_reason_is_tool_calls {
        write_info_log(
            "agent_loop",
            &format!(
                "finish_reason={:?} 不是 ToolCalls 但 active_tool_call_parts 非空({})，仍处理工具调用",
                finish_reason,
                active_tool_call_parts.len()
            ),
        );
    }

    let tool_items: Vec<ToolCallItem> = active_tool_call_parts
        .into_values()
        .map(|part| {
            let id = if part.call_id.is_empty() {
                let rand_id = format!("call_{:016x}", rand::thread_rng().r#gen::<u64>());
                write_info_log(
                    "agent_loop",
                    &format!(
                        "tool_call id 为空（API 未在流式 chunk 中返回），已生成随机 id: {}",
                        rand_id
                    ),
                );
                rand_id
            } else {
                part.call_id
            };
            ToolCallItem {
                id,
                name: part.function_name,
                arguments: part.function_arguments,
            }
        })
        .collect();

    if tool_items.is_empty() {
        write_info_log("agent_loop", "流式 tool_items 转换后为空，break 'round");
        return ToolDispatchResult::BreakRound;
    }

    write_info_log(
        "agent_loop",
        &format!(
            "开始处理 {} 个工具调用: [{}]",
            tool_items.len(),
            tool_items
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    let reasoning_opt = {
        let r: String = take(assistant_reasoning);
        if r.is_empty() { None } else { Some(r) }
    };
    match process_tool_calls(
        tool_items,
        assistant_text,
        ctx.messages,
        ctx.tool_ctx,
        reasoning_opt,
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
                    super::super::tool_processor::clear_channels(
                        ctx.display_messages,
                        ctx.context_messages,
                    );
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
                    "Clearing context after plan approval (stream path)",
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
            ToolDispatchResult::ContinueRound
        }
        Err(e) => {
            write_error_log("agent_loop", &format!("process_tool_calls failed: {}", e));
            ToolDispatchResult::Return
        }
    }
}

/// 无工具调用时的决策：有用户增量消息则继续，否则检查 Stop hook。
async fn handle_no_tool_calls(
    ctx: &mut ToolDispatchContext<'_>,
    assistant_text: String,
    assistant_reasoning: &mut String,
) -> ToolDispatchResult {
    let has_pending =
        !safe_lock(ctx.pending_user_messages, "agent::pending_check_stream").is_empty();
    write_info_log(
        "agent_loop",
        &format!(
            "LLM 未调用工具 (text_len={})，pending_user_messages={}",
            assistant_text.len(),
            has_pending
        ),
    );
    if has_pending {
        let reasoning_for_flush = take_reasoning(assistant_reasoning);
        flush_streaming_as_message(
            ctx.streaming_content,
            ctx.streaming_reasoning_content,
            ctx.messages,
            ctx.display_messages,
            ctx.context_messages,
            reasoning_for_flush,
        );
        write_info_log("agent_loop", "有用户增量消息，continue 'round");
        return ToolDispatchResult::ContinueRound;
    }

    // ★ Stop hook
    if ctx.config.hook_manager.has_hooks_for(HookEvent::Stop) {
        let reasoning_for_flush = take_reasoning(assistant_reasoning);
        flush_streaming_as_message(
            ctx.streaming_content,
            ctx.streaming_reasoning_content,
            ctx.messages,
            ctx.display_messages,
            ctx.context_messages,
            reasoning_for_flush,
        );
        let stop_ctx = HookContext {
            event: HookEvent::Stop,
            messages: Some(ctx.messages.clone()),
            system_prompt: ctx.system_prompt.clone(),
            model: Some(ctx.config.provider.model.clone()),
            user_input: Some(assistant_text.clone()),
            session_id: Some(ctx.session_id.to_string()),
            ..Default::default()
        };
        if let Some(result) =
            ctx.config
                .hook_manager
                .execute(HookEvent::Stop, stop_ctx, &ctx.config.disabled_hooks)
        {
            if let Some(ref ctx_text) = result.additional_context {
                let current = ctx.system_prompt.take().unwrap_or_default();
                *ctx.system_prompt = Some(format!("{}\n\n{}", current, ctx_text));
            }
            if let Some(ref feedback) = result.retry_feedback {
                write_info_log("Stop hook", &format!("纠查官反馈: {}", feedback));
                let feedback_msg = ChatMessage::text(MessageRole::User, feedback.clone());
                ctx.messages.push(feedback_msg.clone());
                push_both(ctx.display_messages, ctx.context_messages, feedback_msg);
                return ToolDispatchResult::ContinueRound;
            }
            if result.is_stop() {
                let _ = ctx.tx.send(StreamMsg::Error(ChatError::HookAborted));
                return ToolDispatchResult::Return;
            }
        }
    }

    ToolDispatchResult::BreakRound
}

fn take_reasoning(reasoning: &mut String) -> Option<String> {
    let r = take(reasoning);
    if r.is_empty() { None } else { Some(r) }
}
