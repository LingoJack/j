//! Compact 子管线：micro_compact + auto_compact，含 hook 调度
//!
//! 对应原 `run_main_agent_loop` 中 LLM 请求前的上下文压缩阶段。

use super::super::config::AgentLoopConfig;
use super::super::tool_processor::{clear_channels, push_both, sync_context_full};
use crate::command::chat::app::types::StreamMsg;
use crate::command::chat::context::compact::{self, AutoCompactParams, InvokedSkillsMap};
use crate::command::chat::error::ChatError;
use crate::command::chat::infra::hook::{HookContext, HookEvent};
use crate::command::chat::storage::{ChatMessage, MessageRole, ToolCallItem};
use crate::util::log::{write_error_log, write_info_log};
use std::sync::{Arc, Mutex, mpsc};

/// compact 子管线的运行时上下文
pub(super) struct CompactContext<'a> {
    pub config: &'a AgentLoopConfig,
    pub messages: &'a mut Vec<ChatMessage>,
    pub display_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub context_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub invoked_skills: &'a InvokedSkillsMap,
    pub session_id: &'a str,
    pub system_prompt: &'a Option<String>,
    pub tx: &'a mpsc::Sender<StreamMsg>,
}

/// 执行 compact 子管线（micro_compact → auto_compact），含全部 hook 调度。
///
/// 返回 `true` 表示 compact 子管线整体被 hook 中止。
pub(super) async fn run_compact_phase(ctx: &mut CompactContext<'_>) -> bool {
    if !ctx.config.compact_config.enabled {
        return false;
    }

    let mut compact_aborted = false;

    // ★ PreMicroCompact hook
    if ctx
        .config
        .hook_manager
        .has_hooks_for(HookEvent::PreMicroCompact)
    {
        let hook_ctx = HookContext {
            event: HookEvent::PreMicroCompact,
            messages: Some(ctx.messages.clone()),
            model: Some(ctx.config.provider.model.clone()),
            session_id: Some(ctx.session_id.to_string()),
            ..Default::default()
        };
        if let Some(result) = ctx.config.hook_manager.execute(
            HookEvent::PreMicroCompact,
            hook_ctx,
            &ctx.config.disabled_hooks,
        ) && result.is_stop()
        {
            write_info_log(
                "PreMicroCompact hook",
                "compact 子管线被 hook 中止（跳过 micro + auto）",
            );
            compact_aborted = true;
        }
    }

    if compact_aborted {
        return true;
    }

    compact::micro_compact(
        ctx.messages,
        ctx.config.compact_config.keep_recent,
        &ctx.config.compact_config.micro_compact_exempt_tools,
    );

    // ★ PostMicroCompact hook
    if ctx
        .config
        .hook_manager
        .has_hooks_for(HookEvent::PostMicroCompact)
    {
        let hook_ctx = HookContext {
            event: HookEvent::PostMicroCompact,
            messages: Some(ctx.messages.clone()),
            session_id: Some(ctx.session_id.to_string()),
            ..Default::default()
        };
        if let Some(result) = ctx.config.hook_manager.execute(
            HookEvent::PostMicroCompact,
            hook_ctx,
            &ctx.config.disabled_hooks,
        ) && let Some(new_msgs) = result.messages
        {
            *ctx.messages = new_msgs;
        }
    }

    if compact::estimate_tokens(ctx.messages)
        > ctx.config.compact_config.effective_token_threshold()
    {
        write_info_log(
            "agent_loop",
            "auto_compact triggered (token threshold exceeded)",
        );

        // ★ PreAutoCompact hook
        let mut protected_context: Option<String> = None;
        if ctx
            .config
            .hook_manager
            .has_hooks_for(HookEvent::PreAutoCompact)
        {
            let hook_ctx = HookContext {
                event: HookEvent::PreAutoCompact,
                messages: Some(ctx.messages.clone()),
                system_prompt: ctx.system_prompt.clone(),
                model: Some(ctx.config.provider.model.clone()),
                session_id: Some(ctx.session_id.to_string()),
                ..Default::default()
            };
            if let Some(result) = ctx.config.hook_manager.execute(
                HookEvent::PreAutoCompact,
                hook_ctx,
                &ctx.config.disabled_hooks,
            ) {
                if result.is_stop() {
                    write_info_log("PreAutoCompact hook", "auto_compact 被 hook 中止");
                    compact_aborted = true;
                }
                if let Some(ac) = result.additional_context {
                    protected_context = Some(ac);
                }
            }
        }

        if !compact_aborted {
            let _ = ctx.tx.send(StreamMsg::Compacting);
            match compact::auto_compact(
                ctx.messages,
                &AutoCompactParams {
                    provider: &ctx.config.provider,
                    invoked_skills: ctx.invoked_skills,
                    session_id: ctx.session_id,
                    protected_context: protected_context.as_deref(),
                },
            )
            .await
            {
                Err(e) => {
                    write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
                }
                Ok(result) => {
                    clear_channels(ctx.display_messages, ctx.context_messages);
                    push_compact_tool_messages(
                        ctx.messages,
                        ctx.display_messages,
                        ctx.context_messages,
                        &result,
                    );
                    let _ = ctx.tx.send(StreamMsg::Compacted {
                        messages_before: result.messages_before,
                    });
                    // ★ PostAutoCompact hook
                    if ctx
                        .config
                        .hook_manager
                        .has_hooks_for(HookEvent::PostAutoCompact)
                    {
                        let hook_ctx = HookContext {
                            event: HookEvent::PostAutoCompact,
                            messages: Some(ctx.messages.clone()),
                            session_id: Some(ctx.session_id.to_string()),
                            ..Default::default()
                        };
                        if let Some(hook_result) = ctx.config.hook_manager.execute(
                            HookEvent::PostAutoCompact,
                            hook_ctx,
                            &ctx.config.disabled_hooks,
                        ) && let Some(new_msgs) = hook_result.messages
                        {
                            *ctx.messages = new_msgs;
                            sync_context_full(
                                ctx.display_messages,
                                ctx.context_messages,
                                ctx.messages,
                            );
                        }
                    }
                }
            }
        }
    }

    compact_aborted
}

/// auto_compact 成功后，向 messages 和双通道注入 Compact 工具调用 + 结果消息。
pub(super) fn push_compact_tool_messages(
    messages: &mut Vec<ChatMessage>,
    display: &Arc<Mutex<Vec<ChatMessage>>>,
    context: &Arc<Mutex<Vec<ChatMessage>>>,
    compact_result: &compact::CompactResult,
) {
    let tool_call_id = format!("compact_auto_{}", compact_result.messages_before);

    for msg in &compact_result.recent_user_messages {
        push_both(display, context, msg.clone());
    }

    let tool_call_item = ToolCallItem {
        id: tool_call_id.clone(),
        name: "Compact".to_string(),
        arguments: r#"{"reason":"auto_compact"}"#.to_string(),
    };
    let tool_call_msg = ChatMessage {
        role: MessageRole::Assistant,
        content: String::new(),
        tool_calls: Some(vec![tool_call_item]),
        tool_call_id: None,
        images: None,
        reasoning_content: None,
        sender_name: None,
    };
    messages.push(tool_call_msg.clone());
    push_both(display, context, tool_call_msg);

    let result_content = format!(
        "📦 上下文已压缩 ({} 条消息 → 摘要, transcript: {})\n\n{}",
        compact_result.messages_before, compact_result.transcript_path, compact_result.summary,
    );
    let tool_msg = ChatMessage {
        role: MessageRole::Tool,
        content: result_content,
        tool_calls: None,
        tool_call_id: Some(tool_call_id),
        images: None,
        reasoning_content: None,
        sender_name: None,
    };
    messages.push(tool_msg.clone());
    push_both(display, context, tool_msg);
}

/// tool_call_id 不一致错误恢复的运行时上下文
pub(super) struct RecoverContext<'a> {
    pub config: &'a AgentLoopConfig,
    pub messages: &'a mut Vec<ChatMessage>,
    pub display_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub context_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub streaming_content: &'a Arc<Mutex<String>>,
    pub streaming_reasoning_content: &'a Arc<Mutex<String>>,
    pub invoked_skills: &'a InvokedSkillsMap,
    pub session_id: &'a str,
    pub tx: &'a mpsc::Sender<StreamMsg>,
}

/// tool_call_id 不一致错误恢复：通过 auto_compact 压缩上下文后重试本轮。
///
/// 返回 `true` 表示恢复成功（调用方应 `continue 'round`），
/// 返回 `false` 表示恢复失败或 compact 未启用（调用方应 return）。
pub(super) async fn recover_tool_id_mismatch(ctx: &mut RecoverContext<'_>) -> bool {
    use crate::util::safe_lock;

    write_info_log(
        "agent_loop",
        "tool_call_id 不一致错误：将执行 auto_compact 压缩上下文后重试",
    );
    {
        let mut stream_buf = safe_lock(ctx.streaming_content, "agent::tool_id_error_clear");
        stream_buf.clear();
    }
    {
        let mut reason_buf = safe_lock(
            ctx.streaming_reasoning_content,
            "agent::tool_id_error_reason_clear",
        );
        reason_buf.clear();
    }
    if ctx.config.compact_config.enabled {
        let _ = ctx.tx.send(StreamMsg::Compacting);
        match compact::auto_compact(
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
            Err(e) => {
                write_error_log(
                    "agent_loop",
                    &format!("tool_call_id 恢复时 auto_compact 失败: {}", e),
                );
                let _ = ctx.tx.send(StreamMsg::Error(ChatError::Other(format!(
                    "消息历史损坏且自动修复失败: {}",
                    e
                ))));
                false
            }
            Ok(result) => {
                clear_channels(ctx.display_messages, ctx.context_messages);
                push_compact_tool_messages(
                    ctx.messages,
                    ctx.display_messages,
                    ctx.context_messages,
                    &result,
                );
                let _ = ctx.tx.send(StreamMsg::Compacted {
                    messages_before: result.messages_before,
                });
                true
            }
        }
    } else {
        let _ = ctx.tx.send(StreamMsg::Error(ChatError::Other(
            "消息历史中 tool_call_id 不一致，且 compact 未启用，无法自动恢复".to_string(),
        )));
        false
    }
}
