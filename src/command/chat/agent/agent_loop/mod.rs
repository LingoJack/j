//! 后台 Agent 循环：支持多轮工具调用
//!
//! 主函数 `run_main_agent_loop` 是一个精简的调度器，
//! 将 compact/stream-reading/tool-dispatch 委托给子模块。

mod compact_phase;
mod stream_reader;
mod tool_dispatch;

use super::api::{build_request_with_tools, create_llm_client};
use super::config::{AgentLoopConfig, AgentLoopSharedState};
use super::tool_processor::{ToolCallContext, drain_pending_user_messages};
use crate::command::chat::app::types::{StreamMsg, ToolResultMsg};
use crate::command::chat::context::compact;
use crate::command::chat::context::compact::InvokedSkillsMap;
use crate::command::chat::error::ChatError;
use crate::command::chat::infra::hook::{HookContext, HookEvent};
use crate::command::chat::storage::{ChatMessage, MessageRole};
use crate::llm::{ChatRequest, LlmClient};
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use std::env::current_dir;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

/// `run_main_agent_loop` 的参数集合（将 6 个独立参数封装为单一结构体）
pub struct MainAgentLoopParams {
    /// Agent loop 的静态配置
    pub config: AgentLoopConfig,
    /// Agent loop 的共享状态（Arc 引用，跨线程共享）
    pub shared: AgentLoopSharedState,
    /// 初始消息列表
    pub messages: Vec<ChatMessage>,
    /// 动态 system prompt 构建函数
    pub system_prompt_fn: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    /// 流式消息发送通道
    pub tx: mpsc::Sender<StreamMsg>,
    /// 工具执行结果接收通道
    pub tool_result_rx: mpsc::Receiver<ToolResultMsg>,
}

/// 重试循环中的共享可变状态（所有引用由主函数持有）
struct LoopState<'a> {
    agent_config: &'a AgentLoopConfig,
    messages: &'a mut Vec<ChatMessage>,
    system_prompt: &'a mut Option<String>,
    streaming_content: &'a Arc<Mutex<String>>,
    streaming_reasoning_content: &'a Arc<Mutex<String>>,
    invoked_skills: &'a InvokedSkillsMap,
    session_id: &'a str,
    pending_user_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    tool_ctx: &'a ToolCallContext<'a>,
}

/// 后台 Agent 循环：支持多轮工具调用
pub async fn run_main_agent_loop(params: MainAgentLoopParams) {
    let MainAgentLoopParams {
        config,
        shared,
        mut messages,
        system_prompt_fn,
        tx,
        tool_result_rx,
    } = params;
    let AgentLoopConfig {
        provider,
        max_llm_rounds,
        compact_config,
        hook_manager,
        disabled_hooks,
        cancel_token,
    } = config;
    let AgentLoopSharedState {
        streaming_content,
        streaming_reasoning_content,
        pending_user_messages,
        background_manager: _,
        todo_manager,
        display_messages,
        context_messages,
        estimated_context_tokens,
        invoked_skills,
        session_id,
        derived_system_prompt,
        tool_registry,
        disabled_tools,
        tools_enabled,
    } = shared;

    let client = create_llm_client(&provider);

    let tool_ctx = ToolCallContext {
        stream_msg_sender: &tx,
        tool_result_receiver: &tool_result_rx,
        pending_user_messages: &pending_user_messages,
        hook_manager: &hook_manager,
        disabled_hooks: &disabled_hooks,
        supports_vision: provider.supports_vision,
        display_messages: &display_messages,
        context_messages: &context_messages,
        streaming_content: &streaming_content,
        session_id: &session_id,
    };

    let agent_config = AgentLoopConfig {
        provider: provider.clone(),
        max_llm_rounds,
        compact_config: compact_config.clone(),
        hook_manager: hook_manager.clone(),
        disabled_hooks: disabled_hooks.clone(),
        cancel_token: cancel_token.clone(),
    };

    let mut final_round_idx: usize = 0;
    'round: for round_idx in 0..max_llm_rounds {
        final_round_idx = round_idx;

        let _tools = if tools_enabled {
            tool_registry.to_llm_tools_filtered(&disabled_tools)
        } else {
            vec![]
        };
        write_info_log(
            "agent_loop",
            &format!(
                "========== 第 {} 轮开始 (max={}) ==========",
                round_idx, max_llm_rounds
            ),
        );

        let mut system_prompt = system_prompt_fn();
        if let Ok(mut sp) = derived_system_prompt.lock() {
            *sp = system_prompt.clone();
        }

        let pending_count = safe_lock(&pending_user_messages, "agent::pending_count").len();
        drain_pending_user_messages(&mut messages, &pending_user_messages);
        if pending_count > 0 {
            write_info_log(
                "agent_loop",
                &format!("drain 了 {} 条用户增量消息", pending_count),
            );
        }

        // ── Compact 子管线 ──
        let mut compact_ctx = compact_phase::CompactContext {
            config: &agent_config,
            messages: &mut messages,
            display_messages: &display_messages,
            context_messages: &context_messages,
            invoked_skills: &invoked_skills,
            session_id: &session_id,
            system_prompt: &system_prompt,
            tx: &tx,
        };
        compact_phase::run_compact_phase(&mut compact_ctx).await;

        todo_manager.increment_turn();
        {
            safe_lock(&streaming_content, "agent::streaming_content_clear").clear();
        }
        {
            safe_lock(
                &streaming_reasoning_content,
                "agent::streaming_reasoning_clear",
            )
            .clear();
        }

        // PreLlmRequest hook + 构建请求
        let mut prepare_ctx = PrepareRequestContext {
            hook_manager: &hook_manager,
            disabled_hooks: &disabled_hooks,
            messages: &mut messages,
            system_prompt: &mut system_prompt,
            provider: &provider,
            session_id: &session_id,
            estimated_context_tokens: &estimated_context_tokens,
            tx: &tx,
        };
        let request = prepare_round_request(&mut prepare_ctx);
        let Some(request) = request else {
            return;
        };

        log_request_input(&messages, &system_prompt);
        log_request_stats(&messages, round_idx, provider.supports_vision);
        for (i, m) in messages.iter().enumerate() {
            if let Some(rc) = &m.reasoning_content {
                write_info_log(
                    "agent_loop",
                    &format!(
                        "messages[{}] role={:?} has reasoning_content len={}",
                        i,
                        m.role,
                        rc.len()
                    ),
                );
            }
        }

        // ── 指数退避重试循环 ──
        let mut loop_state = LoopState {
            agent_config: &agent_config,
            messages: &mut messages,
            system_prompt: &mut system_prompt,
            streaming_content: &streaming_content,
            streaming_reasoning_content: &streaming_reasoning_content,
            invoked_skills: &invoked_skills,
            session_id: &session_id,
            pending_user_messages: &pending_user_messages,
            tool_ctx: &tool_ctx,
        };
        let round_result = execute_api_retry_loop(
            &client,
            &request,
            &cancel_token,
            &tx,
            &display_messages,
            &context_messages,
            &mut loop_state,
        )
        .await;

        match round_result {
            RoundResult::Continue => continue 'round,
            RoundResult::Break => break 'round,
            RoundResult::Return => return,
        }
    }

    write_info_log(
        "agent_loop",
        &format!(
            "agent loop 结束，发送 Done (共执行 {} 轮后退出 'round)",
            final_round_idx + 1
        ),
    );
    let _ = tx.send(StreamMsg::Done);
}

// ── 轮次准备 ──

/// PreLlmRequest hook 的运行时上下文
struct PrepareRequestContext<'a> {
    hook_manager: &'a crate::command::chat::infra::hook::HookManager,
    disabled_hooks: &'a [String],
    messages: &'a mut Vec<ChatMessage>,
    system_prompt: &'a mut Option<String>,
    provider: &'a crate::command::chat::storage::ModelProvider,
    session_id: &'a str,
    estimated_context_tokens: &'a Arc<Mutex<usize>>,
    tx: &'a mpsc::Sender<StreamMsg>,
}

/// PreLlmRequest hook 处理 + 请求构建。返回 None 表示应直接 return。
fn prepare_round_request(ctx: &mut PrepareRequestContext<'_>) -> Option<ChatRequest> {
    if ctx.hook_manager.has_hooks_for(HookEvent::PreLlmRequest) {
        let hook_ctx = HookContext {
            event: HookEvent::PreLlmRequest,
            messages: Some(ctx.messages.clone()),
            system_prompt: ctx.system_prompt.clone(),
            model: Some(ctx.provider.model.clone()),
            session_id: Some(ctx.session_id.to_string()),
            cwd: current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            ..Default::default()
        };
        if let Some(result) =
            ctx.hook_manager
                .execute(HookEvent::PreLlmRequest, hook_ctx, ctx.disabled_hooks)
        {
            if result.is_stop() {
                let _ = ctx.tx.send(StreamMsg::Error(ChatError::HookAborted));
                return None;
            }
            if let Some(new_msgs) = result.messages {
                *ctx.messages = new_msgs;
            }
            if let Some(new_prompt) = result.system_prompt {
                *ctx.system_prompt = Some(new_prompt);
            }
            if let Some(inject) = result.inject_messages {
                ctx.messages.extend(inject);
            }
        }
    }

    {
        let tokens = compact::estimate_tokens(ctx.messages);
        if let Ok(mut ct) = ctx.estimated_context_tokens.lock() {
            *ct = tokens;
        }
    }

    build_request_with_tools(
        ctx.provider,
        ctx.messages,
        vec![],
        ctx.system_prompt.as_deref(),
    )
    .ok()
}

// ── 重试循环 ──

enum RoundResult {
    Continue,
    Break,
    Return,
}

/// 执行指数退避重试循环，返回本轮的决策结果。
async fn execute_api_retry_loop(
    client: &LlmClient,
    request: &ChatRequest,
    cancel_token: &tokio_util::sync::CancellationToken,
    tx: &mpsc::Sender<StreamMsg>,
    display_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    context_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    state: &mut LoopState<'_>,
) -> RoundResult {
    let mut retry_attempt: u32 = 0;
    'api_retry: loop {
        retry_attempt += 1;

        let stream_create_ctx = stream_reader::StreamCreateContext {
            client,
            request,
            cancel_token,
            tx,
            retry_attempt,
        };
        let mut stream = match stream_reader::create_stream_with_retry(&stream_create_ctx).await {
            stream_reader::StreamCreateResult::Ok(s) => s,
            stream_reader::StreamCreateResult::Retry {
                retry_attempt: new_attempt,
            } => {
                retry_attempt = new_attempt;
                continue 'api_retry;
            }
            stream_reader::StreamCreateResult::Return => return RoundResult::Return,
        };

        let read_result = stream_reader::read_stream(
            &mut stream,
            state.streaming_content,
            state.streaming_reasoning_content,
            cancel_token,
            tx,
        )
        .await;
        if read_result.should_return {
            return RoundResult::Return;
        }

        if read_result.needs_compact_for_tool_id_mismatch {
            let mut recover_ctx = compact_phase::RecoverContext {
                config: state.agent_config,
                messages: state.messages,
                display_messages,
                context_messages,
                streaming_content: state.streaming_content,
                streaming_reasoning_content: state.streaming_reasoning_content,
                invoked_skills: state.invoked_skills,
                session_id: state.session_id,
                tx,
            };
            return if compact_phase::recover_tool_id_mismatch(&mut recover_ctx).await {
                RoundResult::Continue
            } else {
                RoundResult::Return
            };
        }

        if let Some(err) = read_result.stream_retriable_error {
            match stream_reader::handle_stream_retriable_error(
                err,
                retry_attempt,
                state.streaming_content,
                state.streaming_reasoning_content,
            ) {
                stream_reader::RetryDecision::Retry {
                    delay_ms,
                    max_attempts,
                    error_message,
                } => {
                    let _ = tx.send(StreamMsg::Retrying {
                        attempt: retry_attempt,
                        max_attempts,
                        delay_ms,
                        error: error_message,
                    });
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => continue 'api_retry,
                        _ = cancel_token.cancelled() => { let _ = tx.send(StreamMsg::Cancelled); return RoundResult::Return; }
                    }
                }
                stream_reader::RetryDecision::Return { err } => {
                    let _ = tx.send(StreamMsg::Error(err));
                    return RoundResult::Return;
                }
            }
        }

        if !read_result.assistant_text.is_empty() {
            write_info_log("Sprite 回复", &read_result.assistant_text);
        }
        write_info_log(
            "agent_loop",
            &format!(
                "流式循环结束: finish_reason={:?}, text_len={}, tool_calls={}, deserialize_failed={}",
                read_result.finish_reason,
                read_result.assistant_text.len(),
                read_result.active_tool_call_parts.len(),
                read_result.deserialize_failed
            ),
        );

        let stream_empty = read_result.finish_reason.is_none()
            && read_result.assistant_text.is_empty()
            && read_result.active_tool_call_parts.is_empty();
        write_info_log(
            "agent_loop",
            &format!(
                "流式结果分析: stream_empty={}, deserialize_failed={}, chunks={}",
                stream_empty, read_result.deserialize_failed, read_result.received_chunks
            ),
        );

        if read_result.deserialize_failed || stream_empty {
            if stream_empty {
                write_info_log(
                    "agent_loop",
                    &format!(
                        "流式返回空响应 (chunks={}, finish_reason=None)，fallback",
                        read_result.received_chunks
                    ),
                );
            }
            let mut fallback_ctx = stream_reader::FallbackContext {
                config: state.agent_config,
                messages: state.messages,
                display_messages,
                context_messages,
                streaming_content: state.streaming_content,
                streaming_reasoning_content: state.streaming_reasoning_content,
                invoked_skills: state.invoked_skills,
                session_id: state.session_id,
                tx,
                tool_ctx: state.tool_ctx,
                cancel_token,
                pending_user_messages: state.pending_user_messages,
            };
            return match stream_reader::handle_fallback(
                &mut fallback_ctx,
                request,
                &mut retry_attempt,
            )
            .await
            {
                stream_reader::FallbackResult::ContinueRound => RoundResult::Continue,
                stream_reader::FallbackResult::BreakRound => RoundResult::Break,
                stream_reader::FallbackResult::Return => RoundResult::Return,
            };
        }

        let mut dispatch_ctx = tool_dispatch::ToolDispatchContext {
            config: state.agent_config,
            messages: state.messages,
            display_messages,
            context_messages,
            streaming_content: state.streaming_content,
            streaming_reasoning_content: state.streaming_reasoning_content,
            invoked_skills: state.invoked_skills,
            session_id: state.session_id,
            system_prompt: state.system_prompt,
            tx,
            tool_ctx: state.tool_ctx,
            pending_user_messages: state.pending_user_messages,
        };
        return match tool_dispatch::handle_streaming_tool_calls(
            &mut dispatch_ctx,
            read_result.active_tool_call_parts,
            read_result.assistant_text,
            read_result.assistant_reasoning,
            &read_result.finish_reason,
        )
        .await
        {
            tool_dispatch::ToolDispatchResult::ContinueRound => RoundResult::Continue,
            tool_dispatch::ToolDispatchResult::BreakRound => RoundResult::Break,
            tool_dispatch::ToolDispatchResult::Return => RoundResult::Return,
        };

        #[allow(unreachable_code)]
        {
            break 'api_retry;
        }
    }

    RoundResult::Break
}

// ── 辅助函数 ──

fn log_request_input(messages: &[ChatMessage], system_prompt: &Option<String>) {
    use crate::util::log::write_info_log;
    let mut log_content = String::new();
    if let Some(sp) = system_prompt {
        log_content.push_str(&format!("[System] {}\n", sp));
    }
    for msg in messages {
        match msg.role {
            MessageRole::Assistant => {
                if !msg.content.is_empty() {
                    log_content.push_str(&format!("[Assistant] {}\n", msg.content));
                }
                if let Some(ref tcs) = msg.tool_calls {
                    for tc in tcs {
                        log_content.push_str(&format!(
                            "[Assistant/ToolCall] {}: {}\n",
                            tc.name, tc.arguments
                        ));
                    }
                }
            }
            MessageRole::Tool => {
                let id = msg.tool_call_id.as_deref().unwrap_or("?");
                let name = msg
                    .tool_calls
                    .as_ref()
                    .and_then(|tcs| tcs.first())
                    .map(|tc| tc.name.as_str())
                    .unwrap_or("unknown");
                log_content.push_str(&format!(
                    "[Tool/Result({} with id `{}`)] result:\n{}\n",
                    name, id, msg.content
                ));
            }
            MessageRole::User => {
                log_content.push_str(&format!("[User] {}\n", msg.content));
            }
            other => {
                log_content.push_str(&format!("[{}] {}\n", other, msg.content));
            }
        }
    }
    write_info_log("Chat 请求", &log_content);
}

fn log_request_stats(messages: &[ChatMessage], round_idx: usize, supports_vision: bool) {
    use crate::util::log::write_info_log;
    let has_images = messages
        .iter()
        .any(|m| m.images.as_ref().is_some_and(|imgs| !imgs.is_empty()));
    write_info_log(
        "agent_loop",
        &format!(
            "第 {} 轮请求: messages={}, has_images={}, supports_vision={}",
            round_idx,
            messages.len(),
            has_images,
            supports_vision
        ),
    );
}
