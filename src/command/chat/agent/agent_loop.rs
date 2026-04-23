use super::super::app::types::{StreamMsg, ToolResultMsg};
use super::super::error::ChatError;
use super::super::hook::{HookContext, HookEvent};
use super::api::{build_request_with_tools, call_openai_non_stream_lenient, create_openai_client};
use super::config::{AgentLoopConfig, AgentLoopSharedState};
use super::retry::{backoff_delay_ms, retry_policy_for};
use super::tool_processor::{
    ToolCallContext, clear_channels, drain_pending_user_messages, flush_streaming_as_message,
    process_tool_calls, push_both, sync_context_full,
};
use crate::command::chat::context::compact;
use crate::command::chat::context::message_compress::{
    DEFAULT_OTHER_AGENT_TOOLCALL_THRESHOLD, compress_other_agent_toolcalls,
};
use crate::command::chat::storage::{ChatMessage, MessageRole, ToolCallItem};
use crate::util::log::{write_error_log, write_info_log};
use crate::util::safe_lock;
use async_openai::types::chat::ChatCompletionTools;
use futures::StreamExt;
use rand::Rng;
use std::sync::{Arc, mpsc};

/// auto_compact 成功后，向 messages 和双通道注入 Compact 工具调用 + 结果消息，
/// 等同于 LLM 手动调用 CompactTool 的效果。
///
/// UI 显示顺序（从上到下）：
/// 1. recent_user_messages（用户最近的消息）
/// 2. assistant tool_call (Compact)
/// 3. tool result（压缩摘要）
///
/// LLM 上下文顺序（messages）：
/// 1. recent_user_messages
/// 2. assistant tool_call
/// 3. tool result
fn push_compact_tool_messages(
    messages: &mut Vec<ChatMessage>,
    display: &Arc<std::sync::Mutex<Vec<ChatMessage>>>,
    context: &Arc<std::sync::Mutex<Vec<ChatMessage>>>,
    compact_result: &compact::CompactResult,
) {
    let tool_call_id = format!("compact_auto_{}", compact_result.messages_before);

    // 1. 先推送 recent_user_messages（UI 中用户消息在 compact 摘要上方）
    //    这些消息已在 messages 中（由 auto_compact 添加），只需同步到双通道
    for msg in &compact_result.recent_user_messages {
        push_both(display, context, msg.clone());
    }

    // 2. 创建 Compact 工具调用消息（模拟 LLM 调用 Compact 工具）
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
    };
    messages.push(tool_call_msg.clone());
    push_both(display, context, tool_call_msg);

    // 3. tool result 消息：包含摘要内容，UI 以边框形式展示
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
    };
    messages.push(tool_msg.clone());
    push_both(display, context, tool_msg);
}

/// 流式响应中逐步聚合的工具调用片段（按 chunk index 聚合 id/name/arguments）
struct StreamingToolCallPart {
    call_id: String,
    function_name: String,
    function_arguments: String,
}

/// 后台 Agent 循环：支持多轮工具调用
pub async fn run_main_agent_loop(
    config: AgentLoopConfig,
    shared: AgentLoopSharedState,
    mut messages: Vec<ChatMessage>,
    tools: Vec<ChatCompletionTools>,
    system_prompt_fn: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    tx: mpsc::Sender<StreamMsg>,
    tool_result_rx: mpsc::Receiver<ToolResultMsg>,
) {
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
        pending_user_messages,
        background_manager: _,
        todo_manager,
        display_messages,
        context_messages,
        estimated_context_tokens,
        invoked_skills,
        session_id,
        derived_system_prompt,
    } = shared;

    let client = create_openai_client(&provider);

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
        invoked_skills: &invoked_skills,
        session_id: &session_id,
    };

    write_info_log(
        "agent_loop",
        &format!(
            "agent loop 启动: max_llm_rounds={}, model={}, tools_count={}",
            max_llm_rounds,
            provider.model,
            tools.len()
        ),
    );
    if !tools.is_empty() {
        let tool_names: Vec<&str> = tools
            .iter()
            .filter_map(|t| {
                if let async_openai::types::chat::ChatCompletionTools::Function(f) = t {
                    Some(f.function.name.as_str())
                } else {
                    None
                }
            })
            .collect();
        write_info_log(
            "agent_loop",
            &format!("可用工具列表: [{}]", tool_names.join(", ")),
        );
    } else {
        write_info_log("agent_loop", "警告: tools 列表为空，LLM 将无法调用任何工具");
    }

    let mut final_round_idx: usize = 0;
    'round: for round_idx in 0..max_llm_rounds {
        final_round_idx = round_idx;
        write_info_log(
            "agent_loop",
            &format!(
                "========== 第 {} 轮开始 (max={}) ==========",
                round_idx, max_llm_rounds
            ),
        );

        // 每轮重新构建 system prompt（从磁盘读取最新配置）
        let mut system_prompt = system_prompt_fn();

        // 同步到共享槽，供子 Agent（AgentTool / CreateTeammateTool / AgentTeamTool）读取
        {
            if let Ok(mut sp) = derived_system_prompt.lock() {
                *sp = system_prompt.clone();
            }
        }

        // 每轮开始时从待处理队列中 drain 用户在 agent loop 期间输入的新消息
        let pending_count_before = safe_lock(&pending_user_messages, "agent::pending_count").len();
        drain_pending_user_messages(&mut messages, &pending_user_messages);
        if pending_count_before > 0 {
            write_info_log(
                "agent_loop",
                &format!("drain 了 {} 条用户增量消息", pending_count_before),
            );
        }

        // ── Layer 1: micro_compact（替换旧 tool results）──
        // ── Layer 2: if tokens > threshold → auto_compact（LLM 摘要）──
        // abort 语义统一：abort PreMicroCompact = 中止整个 compact 子管线（包括 auto_compact）
        if compact_config.enabled {
            let mut compact_aborted = false;

            // ★ PreMicroCompact hook
            if hook_manager.has_hooks_for(HookEvent::PreMicroCompact) {
                let ctx = HookContext {
                    event: HookEvent::PreMicroCompact,
                    messages: Some(messages.clone()),
                    model: Some(provider.model.clone()),
                    session_id: Some(session_id.clone()),
                    ..Default::default()
                };
                if let Some(result) =
                    hook_manager.execute(HookEvent::PreMicroCompact, ctx, &disabled_hooks)
                    && result.is_stop()
                {
                    write_info_log(
                        "PreMicroCompact hook",
                        "compact 子管线被 hook 中止（跳过 micro + auto）",
                    );
                    compact_aborted = true;
                }
            }

            if !compact_aborted {
                compact::micro_compact(
                    &mut messages,
                    compact_config.keep_recent,
                    &compact_config.micro_compact_exempt_tools,
                );

                // ★ PostMicroCompact hook
                if hook_manager.has_hooks_for(HookEvent::PostMicroCompact) {
                    let ctx = HookContext {
                        event: HookEvent::PostMicroCompact,
                        messages: Some(messages.clone()),
                        session_id: Some(session_id.clone()),
                        ..Default::default()
                    };
                    if let Some(result) =
                        hook_manager.execute(HookEvent::PostMicroCompact, ctx, &disabled_hooks)
                        && let Some(new_msgs) = result.messages
                    {
                        messages = new_msgs;
                    }
                }

                if compact::estimate_tokens(&messages) > compact_config.effective_token_threshold()
                {
                    write_info_log(
                        "agent_loop",
                        "auto_compact triggered (token threshold exceeded)",
                    );

                    // ★ PreAutoCompact hook
                    let mut protected_context: Option<String> = None;
                    if hook_manager.has_hooks_for(HookEvent::PreAutoCompact) {
                        let ctx = HookContext {
                            event: HookEvent::PreAutoCompact,
                            messages: Some(messages.clone()),
                            system_prompt: system_prompt.clone(),
                            model: Some(provider.model.clone()),
                            session_id: Some(session_id.clone()),
                            ..Default::default()
                        };
                        if let Some(result) =
                            hook_manager.execute(HookEvent::PreAutoCompact, ctx, &disabled_hooks)
                        {
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
                        let _ = tx.send(StreamMsg::Compacting);
                        match compact::auto_compact(
                            &mut messages,
                            &provider,
                            &invoked_skills,
                            &session_id,
                            protected_context.as_deref(),
                        )
                        .await
                        {
                            Err(e) => {
                                write_error_log(
                                    "agent_loop",
                                    &format!("auto_compact failed: {}", e),
                                );
                            }
                            Ok(result) => {
                                clear_channels(&display_messages, &context_messages);
                                push_compact_tool_messages(
                                    &mut messages,
                                    &display_messages,
                                    &context_messages,
                                    &result,
                                );
                                let _ = tx.send(StreamMsg::Compacted {
                                    messages_before: result.messages_before,
                                });
                                // ★ PostAutoCompact hook
                                if hook_manager.has_hooks_for(HookEvent::PostAutoCompact) {
                                    let ctx = HookContext {
                                        event: HookEvent::PostAutoCompact,
                                        messages: Some(messages.clone()),
                                        session_id: Some(session_id.clone()),
                                        ..Default::default()
                                    };
                                    if let Some(hook_result) = hook_manager.execute(
                                        HookEvent::PostAutoCompact,
                                        ctx,
                                        &disabled_hooks,
                                    ) && let Some(new_msgs) = hook_result.messages
                                    {
                                        messages = new_msgs;
                                        // hook 可能修改了消息，重新全量同步
                                        sync_context_full(
                                            &display_messages,
                                            &context_messages,
                                            &messages,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 检查是否有待办事项（递增轮数计数器，供内置 todo_nag hook 判断）
        todo_manager.increment_turn();

        // 清空流式内容缓冲（每轮开始时）
        {
            let mut stream_buf = safe_lock(&streaming_content, "agent::streaming_content_clear");
            stream_buf.clear();
        }

        // 记录请求输入日志
        {
            let mut log_content = String::new();
            if let Some(ref system_prompt) = system_prompt {
                log_content.push_str(&format!("[System] {}\n", system_prompt));
            }
            for msg in &messages {
                match msg.role {
                    MessageRole::Assistant => {
                        if !msg.content.is_empty() {
                            log_content.push_str(&format!("[Assistant] {}\n", msg.content));
                        }
                        if let Some(ref tool_calls) = msg.tool_calls {
                            for tool_call in tool_calls {
                                log_content.push_str(&format!(
                                    "[Assistant/ToolCall] {}: {}\n",
                                    tool_call.name, tool_call.arguments
                                ));
                            }
                        }
                    }
                    MessageRole::Tool => {
                        let id = msg.tool_call_id.as_deref().unwrap_or("?");
                        let tool_name = msg
                            .tool_calls
                            .as_ref()
                            .and_then(|tool_calls| tool_calls.first())
                            .map(|tool_call| tool_call.name.as_str())
                            .unwrap_or("unknown");
                        log_content.push_str(&format!(
                            "[Tool/Result({} with id `{}`)] result:\n{}\n",
                            tool_name, id, msg.content
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

        // ★ PreLlmRequest hook（可修改 messages 和 system_prompt）
        if hook_manager.has_hooks_for(HookEvent::PreLlmRequest) {
            let ctx = HookContext {
                event: HookEvent::PreLlmRequest,
                messages: Some(messages.clone()),
                system_prompt: system_prompt.clone(),
                model: Some(provider.model.clone()),
                session_id: Some(session_id.clone()),
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            if let Some(result) =
                hook_manager.execute(HookEvent::PreLlmRequest, ctx, &disabled_hooks)
            {
                if result.is_stop() {
                    let _ = tx.send(StreamMsg::Error(ChatError::HookAborted));
                    return;
                }
                if let Some(new_msgs) = result.messages {
                    messages = new_msgs;
                }
                if let Some(new_prompt) = result.system_prompt {
                    system_prompt = Some(new_prompt);
                }
                if let Some(inject) = result.inject_messages {
                    messages.extend(inject);
                }
            }
        }

        // 更新实际上下文 token 估算值（供 UI 显示）
        {
            let tokens = compact::estimate_tokens(&messages);
            if let Ok(mut ct) = estimated_context_tokens.lock() {
                *ct = tokens;
            }
        }

        // 记录本轮请求的消息统计
        {
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
                    provider.supports_vision
                ),
            );
        }

        // 压缩来自其他 agent (SubAgent/Teammate) 的工具调用广播消息，
        // 保留最近 N 条完整消息，将早期消息压缩为摘要
        let compressed_messages = compress_other_agent_toolcalls(
            &messages,
            "Main",
            DEFAULT_OTHER_AGENT_TOOLCALL_THRESHOLD,
        );

        let request = match build_request_with_tools(
            &provider,
            &compressed_messages,
            tools.clone(),
            system_prompt.as_deref(),
        ) {
            Ok(req) => {
                write_info_log("agent_loop", "build_request_with_tools 成功");
                req
            }
            Err(e) => {
                let _ = tx.send(StreamMsg::Error(e));
                return;
            }
        };

        // ── 指数退避重试循环：包裹整个流式请求+读取过程 ──
        // retry_attempt 从 1 开始，每次创建流或读流失败后自增并重试
        let mut retry_attempt: u32 = 0;

        'api_retry: loop {
            retry_attempt += 1;

            // ── 创建流式请求（可重试）──
            write_info_log(
                "agent_loop",
                &format!("开始创建流式请求 (attempt={})...", retry_attempt),
            );
            let mut stream = match client.chat().create_stream(request.clone()).await {
                Ok(s) => {
                    write_info_log("agent_loop", "流式请求创建成功");
                    s
                }
                Err(e) => {
                    let err = ChatError::from(e);
                    write_error_log("Chat API 流式请求创建", &err.to_string());
                    if let Some(policy) = retry_policy_for(&err)
                        && retry_attempt <= policy.max_attempts
                    {
                        let delay_ms =
                            backoff_delay_ms(retry_attempt, policy.base_ms, policy.cap_ms);
                        write_info_log(
                            "agent_loop",
                            &format!(
                                "流式创建失败，{}ms 后重试 ({}/{})",
                                delay_ms, retry_attempt, policy.max_attempts
                            ),
                        );
                        let _ = tx.send(StreamMsg::Retrying {
                            attempt: retry_attempt,
                            max_attempts: policy.max_attempts,
                            delay_ms,
                            error: err.display_message(),
                        });
                        tokio::select! {
                            _ = tokio::time::sleep(std::time::Duration::from_millis(delay_ms)) => {
                                continue 'api_retry;
                            }
                            _ = cancel_token.cancelled() => {
                                let _ = tx.send(StreamMsg::Cancelled);
                                return;
                            }
                        }
                    }
                    let _ = tx.send(StreamMsg::Error(err));
                    return;
                }
            };

            // ── 读取流式响应 ──
            let mut finish_reason: Option<async_openai::types::chat::FinishReason> = None;
            let mut assistant_text = String::new();
            // 手动收集 tool_calls：按 index 聚合 (id, name, arguments)
            let mut active_tool_call_parts: std::collections::BTreeMap<u32, StreamingToolCallPart> =
                std::collections::BTreeMap::new();
            let mut deserialize_failed = false;
            // 流式读取中途遇到 tool_call_id 不一致的请求错误
            let mut needs_compact_for_tool_id_mismatch = false;
            // 流式读取中途遇到的可重试错误
            let mut stream_retriable_error: Option<ChatError> = None;

            let mut received_chunks: u32 = 0;

            'stream: loop {
                tokio::select! {
                    result = stream.next() => {
                        match result {
                            Some(Ok(response)) => {
                                received_chunks += 1;
                                // 记录前几个 chunk 的原始信息，便于调试
                                if received_chunks <= 3 {
                                    let choices_debug: Vec<String> = response.choices.iter().map(|choice| {
                                        format!(
                                            "idx={}, finish_reason={:?}, has_content={}, has_tool_calls={}",
                                            choice.index,
                                            choice.finish_reason,
                                            choice.delta.content.is_some(),
                                            choice.delta.tool_calls.is_some(),
                                        )
                                    }).collect();
                                    write_info_log(
                                        "stream_chunk",
                                        &format!("chunk #{}: choices=[{}]", received_chunks, choices_debug.join("; ")),
                                    );
                                }
                                for choice in &response.choices {
                                    if let Some(ref content) = choice.delta.content {
                                        assistant_text.push_str(content);
                                        let mut stream_buf = safe_lock(&streaming_content, "agent::stream_chunk");
                                        stream_buf.push_str(content);
                                        drop(stream_buf);
                                        let _ = tx.send(StreamMsg::Chunk);
                                    }
                                    // 尝试直接读取 tool_calls（若 async-openai 能反序列化）
                                    if let Some(ref toolcall_chunks) = choice.delta.tool_calls {
                                        for chunk in toolcall_chunks {
                                            let entry =
                                                active_tool_call_parts.entry(chunk.index).or_insert_with(|| {
                                                    StreamingToolCallPart {
                                                        call_id: chunk.id.clone().unwrap_or_default(),
                                                        function_name: String::new(),
                                                        function_arguments: String::new(),
                                                    }
                                                });
                                            if entry.call_id.is_empty()
                                                && let Some(ref id) = chunk.id {
                                                    entry.call_id = id.clone();
                                                }
                                            if let Some(ref tool_function) = chunk.function {
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
                                        finish_reason = Some(*finish_reason_val);
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                let error_str = format!("{}", e);
                                write_error_log("Chat API 流式响应 error", &error_str);
                                let err = ChatError::from(e);
                                // 反序列化错误优先走非流式 fallback（宽松 schema 能绕过大多数格式问题）；
                                // 保留字符串匹配兜底，防止外层包装错误未能转成 StreamDeserialize。
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
                                // 检测 tool_call_id 不一致错误（API 返回 "tool_call_id ... not found"）
                                // 这通常是消息历史损坏导致的，通过压缩上下文并重试可恢复
                                if matches!(&err, ChatError::ApiBadRequest(msg) if msg.contains("tool_call_id")) {
                                    write_error_log(
                                        "Chat API 流式响应",
                                        &format!("检测到 tool_call_id 不一致错误，将压缩上下文后重试: {}", err),
                                    );
                                    needs_compact_for_tool_id_mismatch = true;
                                    break 'stream;
                                }
                                // 可重试错误：记录后跳出流式循环，由外层决策是否重试
                                if retry_policy_for(&err).is_some() {
                                    stream_retriable_error = Some(err);
                                    break 'stream;
                                }
                                // 不可重试：直接报错退出
                                write_error_log("Chat API 流式响应（不可重试）", &err.to_string());
                                let _ = tx.send(StreamMsg::Error(err));
                                return;
                            }
                            None => {
                                write_info_log("agent_loop", "流式结束 (stream returned None)");
                                break 'stream;
                            }
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        let _ = tx.send(StreamMsg::Cancelled);
                        return;
                    }
                }
            }

            // ── 处理 tool_call_id 不一致错误：压缩上下文后重试本轮 ──
            if needs_compact_for_tool_id_mismatch {
                write_info_log(
                    "agent_loop",
                    "tool_call_id 不一致错误：将执行 auto_compact 压缩上下文后重试",
                );
                // 清空已积累的部分内容
                {
                    let mut stream_buf =
                        safe_lock(&streaming_content, "agent::tool_id_error_clear");
                    stream_buf.clear();
                }
                // 通过 auto_compact 重建干净的上下文（摘要 + 全新消息结构，无孤立引用）
                if compact_config.enabled {
                    let _ = tx.send(StreamMsg::Compacting);
                    match compact::auto_compact(
                        &mut messages,
                        &provider,
                        &invoked_skills,
                        &session_id,
                        None,
                    )
                    .await
                    {
                        Err(e) => {
                            write_error_log(
                                "agent_loop",
                                &format!("tool_call_id 恢复时 auto_compact 失败: {}", e),
                            );
                            let _ = tx.send(StreamMsg::Error(ChatError::Other(format!(
                                "消息历史损坏且自动修复失败: {}",
                                e
                            ))));
                            return;
                        }
                        Ok(result) => {
                            clear_channels(&display_messages, &context_messages);
                            push_compact_tool_messages(
                                &mut messages,
                                &display_messages,
                                &context_messages,
                                &result,
                            );
                            let _ = tx.send(StreamMsg::Compacted {
                                messages_before: result.messages_before,
                            });
                        }
                    }
                    continue 'round;
                } else {
                    // compact 未启用，无法恢复
                    let _ = tx.send(StreamMsg::Error(ChatError::Other(
                        "消息历史中 tool_call_id 不一致，且 compact 未启用，无法自动恢复"
                            .to_string(),
                    )));
                    return;
                }
            }

            // ── 处理流式读取中途的可重试错误 ──
            if let Some(err) = stream_retriable_error {
                write_error_log("Chat API 流式响应（将重试）", &err.to_string());
                if let Some(policy) = retry_policy_for(&err)
                    && retry_attempt <= policy.max_attempts
                {
                    // 清空已积累的部分内容，重新开始本轮请求
                    {
                        let mut stream_buf =
                            safe_lock(&streaming_content, "agent::stream_retry_clear");
                        stream_buf.clear();
                    }
                    let delay_ms = backoff_delay_ms(retry_attempt, policy.base_ms, policy.cap_ms);
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "流式中断，{}ms 后重试 ({}/{})",
                            delay_ms, retry_attempt, policy.max_attempts
                        ),
                    );
                    let _ = tx.send(StreamMsg::Retrying {
                        attempt: retry_attempt,
                        max_attempts: policy.max_attempts,
                        delay_ms,
                        error: err.display_message(),
                    });
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_millis(delay_ms)) => {
                            continue 'api_retry;
                        }
                        _ = cancel_token.cancelled() => {
                            let _ = tx.send(StreamMsg::Cancelled);
                            return;
                        }
                    }
                }
                // 重试次数耗尽
                let _ = tx.send(StreamMsg::Error(err));
                return;
            }

            // 记录流式回复日志
            if !assistant_text.is_empty() {
                write_info_log("Sprite 回复", &assistant_text);
            }

            write_info_log(
                "agent_loop",
                &format!(
                    "流式循环结束: finish_reason={:?}, assistant_text_len={}, active_tool_call_parts={}, deserialize_failed={}",
                    finish_reason,
                    assistant_text.len(),
                    active_tool_call_parts.len(),
                    deserialize_failed
                ),
            );

            // 如果流式遇到 tool_calls 反序列化错误，或者流式返回空响应（finish_reason=None 且无有效内容），
            // fallback 到非流式获取完整响应。
            // 常见场景：某些 API 对多模态+流式组合返回空 choices，需要非流式重试。
            let stream_empty = finish_reason.is_none()
                && assistant_text.is_empty()
                && active_tool_call_parts.is_empty();
            write_info_log(
                "agent_loop",
                &format!(
                    "流式结果分析: stream_empty={}, deserialize_failed={}, received_chunks={}",
                    stream_empty, deserialize_failed, received_chunks
                ),
            );
            if deserialize_failed || stream_empty {
                if stream_empty {
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "流式返回空响应 (chunks={}, finish_reason=None, 无内容)，fallback 到非流式重试",
                            received_chunks
                        ),
                    );
                }
                // 清空流式内容（切换到非流式）
                {
                    let mut stream_buf = safe_lock(&streaming_content, "agent::fallback_clear");
                    stream_buf.clear();
                }
                // 使用宽松反序列化的非流式调用（兼容非标准 finish_reason），同样支持重试
                let fallback_result = loop {
                    let create_fut = call_openai_non_stream_lenient(&provider, &request);
                    let result = tokio::select! {
                        result = create_fut => result,
                        _ = cancel_token.cancelled() => {
                            let _ = tx.send(StreamMsg::Cancelled);
                            return;
                        }
                    };
                    match result {
                        Ok(r) => break r,
                        Err(e) => {
                            write_error_log("Sprite API fallback 非流式", &e.to_string());
                            if let Some(policy) = retry_policy_for(&e)
                                && retry_attempt <= policy.max_attempts
                            {
                                let delay_ms =
                                    backoff_delay_ms(retry_attempt, policy.base_ms, policy.cap_ms);
                                write_info_log(
                                    "agent_loop",
                                    &format!(
                                        "fallback 非流式失败，{}ms 后重试 ({}/{})",
                                        delay_ms, retry_attempt, policy.max_attempts
                                    ),
                                );
                                let _ = tx.send(StreamMsg::Retrying {
                                    attempt: retry_attempt,
                                    max_attempts: policy.max_attempts,
                                    delay_ms,
                                    error: e.display_message(),
                                });
                                tokio::select! {
                                    _ = tokio::time::sleep(std::time::Duration::from_millis(delay_ms)) => {
                                        retry_attempt += 1;
                                        continue;
                                    }
                                    _ = cancel_token.cancelled() => {
                                        let _ = tx.send(StreamMsg::Cancelled);
                                        return;
                                    }
                                }
                            }
                            let _ = tx.send(StreamMsg::Error(e));
                            return;
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
                            .map(|content| content.len())
                            .unwrap_or(0),
                        fallback_result.finish_reason
                    ),
                );

                if fallback_result.has_tool_calls()
                    && let Some(tool_items) = fallback_result.tool_calls
                {
                    if tool_items.is_empty() {
                        write_info_log("agent_loop", "fallback tool_calls 为空列表，break 'round");
                        break 'round;
                    }
                    let assistant_text: String =
                        fallback_result.content.clone().unwrap_or_default();
                    match process_tool_calls(tool_items, assistant_text, &mut messages, &tool_ctx) {
                        Ok(result) => {
                            // ── Layer 3: compact tool 触发 ──
                            if result.compact_requested && compact_config.enabled {
                                let _ = tx.send(StreamMsg::Compacting);
                                if let Ok(compact_result) = compact::auto_compact(
                                    &mut messages,
                                    &provider,
                                    &invoked_skills,
                                    &session_id,
                                    None,
                                )
                                .await
                                {
                                    clear_channels(&display_messages, &context_messages);
                                    push_compact_tool_messages(
                                        &mut messages,
                                        &display_messages,
                                        &context_messages,
                                        &compact_result,
                                    );
                                    let _ = tx.send(StreamMsg::Compacted {
                                        messages_before: compact_result.messages_before,
                                    });
                                }
                            }
                            // ── Plan 被批准且清空上下文 ──
                            if let Some(ref plan_content) = result.plan_with_context_clear {
                                write_info_log(
                                    "agent_loop",
                                    "Clearing context after plan approval (fallback path)",
                                );
                                // 清空 messages 和双通道
                                messages.clear();
                                if let Ok(mut shared) = display_messages.lock() {
                                    shared.clear();
                                }
                                if let Ok(mut shared) = context_messages.lock() {
                                    shared.clear();
                                }
                                // 以 User 角色注入计划指令（给 LLM 上下文使用），
                                // 但不 push_both — UI 中不应出现用户未发送的消息
                                let plan_msg = ChatMessage::text(
                                    MessageRole::User,
                                    format!("以下计划已获批准，请按计划执行：\n\n{}", plan_content),
                                );
                                messages.push(plan_msg);
                            }
                            continue 'round;
                        }
                        Err(e) => {
                            write_error_log(
                                "agent_loop",
                                &format!("process_tool_calls failed: {}", e),
                            );
                            return;
                        }
                    }
                }

                // 普通文本回复（或非标准 finish_reason 如 network_error）
                if let Some(ref content) = fallback_result.content
                    && !content.is_empty()
                {
                    write_info_log("Sprite 回复", content);
                    let mut stream_buf = safe_lock(&streaming_content, "agent::fallback_content");
                    stream_buf.push_str(content);
                    drop(stream_buf);
                    let _ = tx.send(StreamMsg::Chunk);
                }
                // 非标准 finish_reason 且无内容时，报告错误
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
                    let _ = tx.send(StreamMsg::Error(error_msg));
                    return;
                }

                // fallback 非流式正常结束，但如果有用户增量消息则继续循环
                let has_pending =
                    !safe_lock(&pending_user_messages, "agent::pending_check_fallback").is_empty();
                write_info_log(
                    "agent_loop",
                    &format!("fallback 正常结束，pending_user_messages={}", has_pending),
                );
                if has_pending {
                    flush_streaming_as_message(
                        &streaming_content,
                        &mut messages,
                        &display_messages,
                        &context_messages,
                    );
                    write_info_log("agent_loop", "有用户增量消息，continue 'round");
                    continue 'round;
                }
                write_info_log("agent_loop", "无用户增量消息，break 'round (fallback 路径)");
                break 'round;
            }

            // ── 检查流式模式下是否有 tool_calls ──
            // 优先检查 active_tool_call_parts 是否非空，而非仅依赖 finish_reason。
            // 某些 API（非 OpenAI）流式返回的 finish_reason 不是 ToolCodes 枚举值，
            // 但 chunk 中确实包含 tool_calls 数据。此时如果只看 finish_reason 会直接
            // break 'round，导致工具调用被丢弃，agent 提前结束。
            let has_tool_calls = !active_tool_call_parts.is_empty();
            write_info_log(
                "agent_loop",
                &format!(
                    "流式路径决策: has_tool_calls={}, finish_reason={:?}",
                    has_tool_calls, finish_reason
                ),
            );

            if has_tool_calls {
                // 日志：检测 finish_reason 与实际 tool_calls 是否一致
                let finish_reason_is_tool_calls = matches!(
                    finish_reason,
                    Some(async_openai::types::chat::FinishReason::ToolCalls)
                );
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
                        // 某些 API 在流式 chunk 中不返回 tool_call id，
                        // 导致 id 为空字符串；发送给 API 时会报 tool_call_id not found。
                        // 此处为空 id 生成随机唯一 id。
                        let id = if part.call_id.is_empty() {
                            let rand_id =
                                format!("call_{:016x}", rand::thread_rng().r#gen::<u64>());
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
                        ToolCallItem { id, name: part.function_name, arguments: part.function_arguments }
                    })
                    .collect();

                if tool_items.is_empty() {
                    write_info_log("agent_loop", "流式 tool_items 转换后为空，break 'round");
                    break 'round;
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
                match process_tool_calls(tool_items, assistant_text, &mut messages, &tool_ctx) {
                    Ok(result) => {
                        // ── Layer 3: compact tool 触发 ──
                        if result.compact_requested && compact_config.enabled {
                            let _ = tx.send(StreamMsg::Compacting);
                            if let Ok(compact_result) = compact::auto_compact(
                                &mut messages,
                                &provider,
                                &invoked_skills,
                                &session_id,
                                None,
                            )
                            .await
                            {
                                clear_channels(&display_messages, &context_messages);
                                push_compact_tool_messages(
                                    &mut messages,
                                    &display_messages,
                                    &context_messages,
                                    &compact_result,
                                );
                                let _ = tx.send(StreamMsg::Compacted {
                                    messages_before: compact_result.messages_before,
                                });
                            }
                        }
                        // ── Plan 被批准且清空上下文 ──
                        if let Some(ref plan_content) = result.plan_with_context_clear {
                            write_info_log(
                                "agent_loop",
                                "Clearing context after plan approval (stream path)",
                            );
                            // 清空 messages 和双通道
                            messages.clear();
                            if let Ok(mut shared) = display_messages.lock() {
                                shared.clear();
                            }
                            if let Ok(mut shared) = context_messages.lock() {
                                shared.clear();
                            }
                            // 以 User 角色注入计划指令（给 LLM 上下文使用），
                            // 但不 push_both — UI 中不应出现用户未发送的消息
                            let plan_msg = ChatMessage::text(
                                MessageRole::User,
                                format!("以下计划已获批准，请按计划执行：\n\n{}", plan_content),
                            );
                            messages.push(plan_msg);
                        }
                        continue 'round;
                    }
                    Err(e) => {
                        write_error_log("agent_loop", &format!("process_tool_calls failed: {}", e));
                        return;
                    }
                }
            } else {
                // 正常结束，但如果有用户增量消息则继续循环
                let has_pending =
                    !safe_lock(&pending_user_messages, "agent::pending_check_stream").is_empty();
                write_info_log(
                    "agent_loop",
                    &format!(
                        "LLM 未调用工具 (finish_reason={:?}, text_len={})，pending_user_messages={}",
                        finish_reason,
                        assistant_text.len(),
                        has_pending
                    ),
                );
                if has_pending {
                    flush_streaming_as_message(
                        &streaming_content,
                        &mut messages,
                        &display_messages,
                        &context_messages,
                    );
                    write_info_log("agent_loop", "有用户增量消息，continue 'round");
                    continue 'round;
                }

                // ★ Stop hook：LLM 即将结束回复（无工具调用且无待处理消息），纠查官可阻止并注入反馈
                if hook_manager.has_hooks_for(HookEvent::Stop) {
                    flush_streaming_as_message(
                        &streaming_content,
                        &mut messages,
                        &display_messages,
                        &context_messages,
                    );
                    let stop_ctx = HookContext {
                        event: HookEvent::Stop,
                        messages: Some(messages.clone()),
                        system_prompt: system_prompt.clone(),
                        model: Some(provider.model.clone()),
                        user_input: Some(assistant_text.clone()),
                        session_id: Some(session_id.clone()),
                        ..Default::default()
                    };
                    if let Some(result) =
                        hook_manager.execute(HookEvent::Stop, stop_ctx, &disabled_hooks)
                    {
                        // 注入额外上下文（追加到 system_prompt）
                        if let Some(ref ctx_text) = result.additional_context {
                            let current = system_prompt.unwrap_or_default();
                            system_prompt = Some(format!("{}\n\n{}", current, ctx_text));
                        }
                        // retry_feedback → 注入为 user message，LLM 带反馈继续
                        if let Some(ref feedback) = result.retry_feedback {
                            write_info_log("Stop hook", &format!("纠查官反馈: {}", feedback));
                            let feedback_msg =
                                ChatMessage::text(MessageRole::User, feedback.clone());
                            messages.push(feedback_msg.clone());
                            push_both(&display_messages, &context_messages, feedback_msg);
                            continue 'round;
                        }
                        // stop → 直接中止
                        if result.is_stop() {
                            let _ = tx.send(StreamMsg::Error(ChatError::HookAborted));
                            return;
                        }
                    }
                }

                write_info_log(
                    "agent_loop",
                    &format!(
                        "break 'round: LLM 返回 Stop 且无工具调用，无待处理消息 (round={}, text_len={})",
                        round_idx,
                        assistant_text.len()
                    ),
                );
                break 'round;
            }

            // 流式请求成功完成，退出重试循环
            #[allow(unreachable_code)]
            {
                break 'api_retry;
            }
        } // end 'api_retry
    } // end 'round

    write_info_log(
        "agent_loop",
        &format!(
            "agent loop 结束，发送 Done (共执行 {} 轮后退出 'round)",
            final_round_idx + 1
        ),
    );
    let _ = tx.send(StreamMsg::Done);
}
