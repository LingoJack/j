use super::super::app::types::{PlanDecision, StreamMsg, ToolResultMsg};
use super::super::error::ChatError;
use super::super::hook::{HookContext, HookEvent, HookManager};
use super::super::storage::{ChatMessage, ToolCallItem};
use super::api::{build_request_with_tools, call_openai_non_stream_lenient, create_openai_client};
use super::compact;
use super::config::{AgentLoopConfig, AgentSharedState};
use crate::command::chat::constants::{ROLE_ASSISTANT, ROLE_TOOL, ROLE_USER};
use crate::command::chat::tools::Tool;
use crate::command::chat::tools::compact::CompactTool;
use crate::util::log::{write_error_log, write_info_log};
use crate::util::safe_lock;
use async_openai::types::chat::ChatCompletionTools;
use futures::StreamExt;
use rand::Rng;
use std::sync::{Arc, Mutex, mpsc};

/// process_tool_calls 所需的通道和共享状态
struct ToolCallContext<'a> {
    tx: &'a mpsc::Sender<StreamMsg>,
    tool_result_rx: &'a mpsc::Receiver<ToolResultMsg>,
    pending_user_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    hook_manager: &'a HookManager,
    supports_vision: bool,
    shared_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    streaming_content: &'a Arc<Mutex<String>>,
    #[allow(dead_code)]
    invoked_skills: &'a compact::InvokedSkillsMap,
}

/// 后台 Agent 循环：支持多轮工具调用
pub async fn run_agent_loop(
    config: AgentLoopConfig,
    shared: AgentSharedState,
    mut messages: Vec<ChatMessage>,
    tools: Vec<ChatCompletionTools>,
    system_prompt_fn: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    tx: mpsc::Sender<StreamMsg>,
    tool_result_rx: mpsc::Receiver<ToolResultMsg>,
) {
    let AgentLoopConfig {
        provider,
        max_tool_rounds,
        compact_config,
        hook_manager,
        cancel_token,
    } = config;
    let AgentSharedState {
        streaming_content,
        pending_user_messages,
        background_manager: _,
        todo_manager,
        shared_messages,
        context_tokens,
        invoked_skills,
    } = shared;

    let client = create_openai_client(&provider);

    let tool_ctx = ToolCallContext {
        tx: &tx,
        tool_result_rx: &tool_result_rx,
        pending_user_messages: &pending_user_messages,
        hook_manager: &hook_manager,
        supports_vision: provider.supports_vision,
        shared_messages: &shared_messages,
        streaming_content: &streaming_content,
        invoked_skills: &invoked_skills,
    };

    write_info_log(
        "agent_loop",
        &format!(
            "agent loop 启动: max_tool_rounds={}, model={}, tools_count={}",
            max_tool_rounds,
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

    let mut last_round: usize = 0;
    'round: for _round in 0..max_tool_rounds {
        last_round = _round;
        write_info_log(
            "agent_loop",
            &format!(
                "========== 第 {} 轮开始 (max={}) ==========",
                _round, max_tool_rounds
            ),
        );

        // 每轮重新构建 system prompt（从磁盘读取最新配置）
        let mut system_prompt = system_prompt_fn();

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
        if compact_config.enabled {
            compact::micro_compact(
                &mut messages,
                compact_config.keep_recent,
                &compact_config.micro_compact_exempt_tools,
            );
            if compact::estimate_tokens(&messages) > compact_config.token_threshold {
                write_info_log(
                    "agent_loop",
                    "auto_compact triggered (token threshold exceeded)",
                );
                if let Err(e) =
                    compact::auto_compact(&mut messages, &provider, &invoked_skills).await
                {
                    write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
                }
            }
        }

        // 检查是否有待办事项（递增轮数计数器，供内置 todo_nag hook 判断）
        todo_manager.increment_turn();

        // 清空流式内容缓冲（每轮开始时）
        {
            let mut sc = safe_lock(&streaming_content, "agent::streaming_content_clear");
            sc.clear();
        }

        // 记录请求输入日志
        {
            let mut log_content = String::new();
            if let Some(ref sp) = system_prompt {
                log_content.push_str(&format!("[System] {}\n", sp));
            }
            for msg in &messages {
                match msg.role.as_str() {
                    ROLE_ASSISTANT => {
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
                    ROLE_TOOL => {
                        let id = msg.tool_call_id.as_deref().unwrap_or("?");
                        let tool_name = msg
                            .tool_calls
                            .as_ref()
                            .and_then(|tc| tc.first())
                            .map(|tc| tc.name.as_str())
                            .unwrap_or("unknown");
                        log_content.push_str(&format!(
                            "[Tool/Result({} with id `{}`)] result:\n{}\n",
                            tool_name, id, msg.content
                        ));
                    }
                    ROLE_USER => {
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
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            if let Some(result) = hook_manager.execute(HookEvent::PreLlmRequest, ctx) {
                if result.abort {
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
            if let Ok(mut ct) = context_tokens.lock() {
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
                    _round,
                    messages.len(),
                    has_images,
                    provider.supports_vision
                ),
            );
        }

        let request = match build_request_with_tools(
            &provider,
            &messages,
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
        // api_attempt 从 1 开始，每次创建流或读流失败后自增并重试
        let mut api_attempt: u32 = 0;

        'api_retry: loop {
            api_attempt += 1;

            // ── 创建流式请求（可重试）──
            write_info_log(
                "agent_loop",
                &format!("开始创建流式请求 (attempt={})...", api_attempt),
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
                        && api_attempt <= policy.max_attempts
                    {
                        let delay_ms = backoff_delay_ms(api_attempt, policy.base_ms, policy.cap_ms);
                        write_info_log(
                            "agent_loop",
                            &format!(
                                "流式创建失败，{}ms 后重试 ({}/{})",
                                delay_ms, api_attempt, policy.max_attempts
                            ),
                        );
                        let _ = tx.send(StreamMsg::Retrying {
                            attempt: api_attempt,
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
            let mut raw_tool_calls: std::collections::BTreeMap<u32, (String, String, String)> =
                std::collections::BTreeMap::new();
            let mut stream_had_deserialize_error = false;
            // 流式读取中途遇到 tool_call_id 不一致的请求错误
            let mut stream_tool_id_error = false;
            // 流式读取中途遇到的可重试错误
            let mut stream_retry_error: Option<ChatError> = None;

            let mut stream_chunk_count: u32 = 0;

            'stream: loop {
                tokio::select! {
                    result = stream.next() => {
                        match result {
                            Some(Ok(response)) => {
                                stream_chunk_count += 1;
                                // 记录前几个 chunk 的原始信息，便于调试
                                if stream_chunk_count <= 3 {
                                    let choices_debug: Vec<String> = response.choices.iter().map(|c| {
                                        format!(
                                            "idx={}, finish_reason={:?}, has_content={}, has_tool_calls={}",
                                            c.index,
                                            c.finish_reason,
                                            c.delta.content.is_some(),
                                            c.delta.tool_calls.is_some(),
                                        )
                                    }).collect();
                                    write_info_log(
                                        "stream_chunk",
                                        &format!("chunk #{}: choices=[{}]", stream_chunk_count, choices_debug.join("; ")),
                                    );
                                }
                                for choice in &response.choices {
                                    if let Some(ref content) = choice.delta.content {
                                        assistant_text.push_str(content);
                                        let mut sc = safe_lock(&streaming_content, "agent::stream_chunk");
                                        sc.push_str(content);
                                        drop(sc);
                                        let _ = tx.send(StreamMsg::Chunk);
                                    }
                                    // 尝试直接读取 tool_calls（若 async-openai 能反序列化）
                                    if let Some(ref toolcall_chunks) = choice.delta.tool_calls {
                                        for chunk in toolcall_chunks {
                                            let entry =
                                                raw_tool_calls.entry(chunk.index).or_insert_with(|| {
                                                    (
                                                        chunk.id.clone().unwrap_or_default(),
                                                        String::new(),
                                                        String::new(),
                                                    )
                                                });
                                            if entry.0.is_empty()
                                                && let Some(ref id) = chunk.id {
                                                    entry.0 = id.clone();
                                                }
                                            if let Some(ref tool_function) = chunk.function {
                                                if let Some(ref name) = tool_function.name {
                                                    entry.1.push_str(name);
                                                }
                                                if let Some(ref args) = tool_function.arguments {
                                                    entry.2.push_str(args);
                                                }
                                            }
                                        }
                                    }
                                    if let Some(ref fr) = choice.finish_reason {
                                        finish_reason = Some(*fr);
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                let error_str = format!("{}", e);
                                write_error_log("Chat API 流式响应 error", &error_str);
                                // 检测是否是 tool_calls 反序列化错误（Gemini 等不返回 chunk index）
                                if error_str.contains("missing field `index`")
                                    || error_str.contains("tool_calls")
                                {
                                    // 标记需要用非流式重做，跳出流式循环
                                    stream_had_deserialize_error = true;
                                    break 'stream;
                                }
                                let err = ChatError::from(e);
                                // 检测 tool_call_id 不一致错误（API 返回 "tool_call_id ... not found"）
                                // 这通常是消息历史损坏导致的，通过压缩上下文并重试可恢复
                                if matches!(&err, ChatError::ApiBadRequest(msg) if msg.contains("tool_call_id")) {
                                    write_error_log(
                                        "Chat API 流式响应",
                                        &format!("检测到 tool_call_id 不一致错误，将压缩上下文后重试: {}", err),
                                    );
                                    stream_tool_id_error = true;
                                    break 'stream;
                                }
                                // 可重试错误：记录后跳出流式循环，由外层决策是否重试
                                if retry_policy_for(&err).is_some() {
                                    stream_retry_error = Some(err);
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
            if stream_tool_id_error {
                write_info_log(
                    "agent_loop",
                    "tool_call_id 不一致错误：将执行 auto_compact 压缩上下文后重试",
                );
                // 清空已积累的部分内容
                {
                    let mut sc = safe_lock(&streaming_content, "agent::tool_id_error_clear");
                    sc.clear();
                }
                // 通过 auto_compact 重建干净的上下文（摘要 + 全新消息结构，无孤立引用）
                if compact_config.enabled {
                    if let Err(e) =
                        compact::auto_compact(&mut messages, &provider, &invoked_skills).await
                    {
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
            if let Some(err) = stream_retry_error {
                write_error_log("Chat API 流式响应（将重试）", &err.to_string());
                if let Some(policy) = retry_policy_for(&err)
                    && api_attempt <= policy.max_attempts
                {
                    // 清空已积累的部分内容，重新开始本轮请求
                    {
                        let mut sc = safe_lock(&streaming_content, "agent::stream_retry_clear");
                        sc.clear();
                    }
                    let delay_ms = backoff_delay_ms(api_attempt, policy.base_ms, policy.cap_ms);
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "流式中断，{}ms 后重试 ({}/{})",
                            delay_ms, api_attempt, policy.max_attempts
                        ),
                    );
                    let _ = tx.send(StreamMsg::Retrying {
                        attempt: api_attempt,
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
                    "流式循环结束: finish_reason={:?}, assistant_text_len={}, raw_tool_calls={}, stream_had_deserialize_error={}",
                    finish_reason,
                    assistant_text.len(),
                    raw_tool_calls.len(),
                    stream_had_deserialize_error
                ),
            );

            // 如果流式遇到 tool_calls 反序列化错误，或者流式返回空响应（finish_reason=None 且无有效内容），
            // fallback 到非流式获取完整响应。
            // 常见场景：某些 API 对多模态+流式组合返回空 choices，需要非流式重试。
            let stream_empty =
                finish_reason.is_none() && assistant_text.is_empty() && raw_tool_calls.is_empty();
            write_info_log(
                "agent_loop",
                &format!(
                    "流式结果分析: stream_empty={}, stream_had_deserialize_error={}, stream_chunk_count={}",
                    stream_empty, stream_had_deserialize_error, stream_chunk_count
                ),
            );
            if stream_had_deserialize_error || stream_empty {
                if stream_empty {
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "流式返回空响应 (chunks={}, finish_reason=None, 无内容)，fallback 到非流式重试",
                            stream_chunk_count
                        ),
                    );
                }
                // 清空流式内容（切换到非流式）
                {
                    let mut sc = safe_lock(&streaming_content, "agent::fallback_clear");
                    sc.clear();
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
                                && api_attempt <= policy.max_attempts
                            {
                                let delay_ms =
                                    backoff_delay_ms(api_attempt, policy.base_ms, policy.cap_ms);
                                write_info_log(
                                    "agent_loop",
                                    &format!(
                                        "fallback 非流式失败，{}ms 后重试 ({}/{})",
                                        delay_ms, api_attempt, policy.max_attempts
                                    ),
                                );
                                let _ = tx.send(StreamMsg::Retrying {
                                    attempt: api_attempt,
                                    max_attempts: policy.max_attempts,
                                    delay_ms,
                                    error: e.display_message(),
                                });
                                tokio::select! {
                                    _ = tokio::time::sleep(std::time::Duration::from_millis(delay_ms)) => {
                                        api_attempt += 1;
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
                        "fallback 非流式结果: is_tool_calls={}, has_content={}, finish_reason={:?}",
                        fallback_result.is_tool_calls,
                        fallback_result
                            .content
                            .as_ref()
                            .map(|c| c.len())
                            .unwrap_or(0),
                        fallback_result.finish_reason
                    ),
                );

                if fallback_result.is_tool_calls
                    && let Some(tool_items) = fallback_result.tool_calls
                {
                    if tool_items.is_empty() {
                        write_info_log("agent_loop", "fallback tool_calls 为空列表，break 'round");
                        break 'round;
                    }
                    let assistant_text = fallback_result.content.clone().unwrap_or_default();
                    match process_tool_calls(tool_items, assistant_text, &mut messages, &tool_ctx) {
                        Ok(result) => {
                            // ── Layer 3: compact tool 触发 ──
                            if result.compact_requested && compact_config.enabled {
                                let _ = compact::auto_compact(
                                    &mut messages,
                                    &provider,
                                    &invoked_skills,
                                )
                                .await;
                            }
                            // ── Plan 被批准且清空上下文 ──
                            if let Some(ref plan_content) = result.plan_approved_clear_context {
                                write_info_log(
                                    "agent_loop",
                                    "Clearing context after plan approval",
                                );
                                messages.clear();
                                if let Ok(mut shared) = shared_messages.lock() {
                                    shared.clear();
                                }
                                let plan_msg = ChatMessage {
                                    role: ROLE_USER.to_string(),
                                    content: format!(
                                        "以下计划已获批准，请按计划执行：\n\n{}",
                                        plan_content
                                    ),
                                    tool_calls: None,
                                    tool_call_id: None,
                                    images: None,
                                };
                                messages.push(plan_msg.clone());
                                push_shared(&shared_messages, plan_msg);
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
                    let mut sc = safe_lock(&streaming_content, "agent::fallback_content");
                    sc.push_str(content);
                    drop(sc);
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
                    flush_streaming_as_message(&streaming_content, &mut messages, &shared_messages);
                    write_info_log("agent_loop", "有用户增量消息，continue 'round");
                    continue 'round;
                }
                write_info_log("agent_loop", "无用户增量消息，break 'round (fallback 路径)");
                break 'round;
            }

            // ── 检查流式模式下是否有 tool_calls ──
            // 优先检查 raw_tool_calls 是否非空，而非仅依赖 finish_reason。
            // 某些 API（非 OpenAI）流式返回的 finish_reason 不是 ToolCodes 枚举值，
            // 但 chunk 中确实包含 tool_calls 数据。此时如果只看 finish_reason 会直接
            // break 'round，导致工具调用被丢弃，agent 提前结束。
            let has_tool_calls = !raw_tool_calls.is_empty();
            write_info_log(
                "agent_loop",
                &format!(
                    "流式路径决策: has_tool_calls={}, finish_reason={:?}",
                    has_tool_calls, finish_reason
                ),
            );

            if has_tool_calls {
                // 日志：检测 finish_reason 与实际 tool_calls 是否一致
                let finish_is_tool_calls = matches!(
                    finish_reason,
                    Some(async_openai::types::chat::FinishReason::ToolCalls)
                );
                if !finish_is_tool_calls {
                    write_info_log(
                        "agent_loop",
                        &format!(
                            "finish_reason={:?} 不是 ToolCalls 但 raw_tool_calls 非空({})，仍处理工具调用",
                            finish_reason,
                            raw_tool_calls.len()
                        ),
                    );
                }

                let tool_items: Vec<ToolCallItem> = raw_tool_calls
                    .into_values()
                    .map(|(id, name, arguments)| {
                        // 某些 API 在流式 chunk 中不返回 tool_call id，
                        // 导致 id 为空字符串；发送给 API 时会报 tool_call_id not found。
                        // 此处为空 id 生成随机唯一 id。
                        let id = if id.is_empty() {
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
                            id
                        };
                        ToolCallItem { id, name, arguments }
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
                            let _ =
                                compact::auto_compact(&mut messages, &provider, &invoked_skills)
                                    .await;
                        }
                        // ── Plan 被批准且清空上下文 ──
                        if let Some(ref plan_content) = result.plan_approved_clear_context {
                            write_info_log("agent_loop", "Clearing context after plan approval");
                            messages.clear();
                            if let Ok(mut shared) = shared_messages.lock() {
                                shared.clear();
                            }
                            let plan_msg = ChatMessage {
                                role: ROLE_USER.to_string(),
                                content: format!(
                                    "以下计划已获批准，请按计划执行：\n\n{}",
                                    plan_content
                                ),
                                tool_calls: None,
                                tool_call_id: None,
                                images: None,
                            };
                            messages.push(plan_msg.clone());
                            push_shared(&shared_messages, plan_msg);
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
                    flush_streaming_as_message(&streaming_content, &mut messages, &shared_messages);
                    write_info_log("agent_loop", "有用户增量消息，continue 'round");
                    continue 'round;
                }
                write_info_log(
                    "agent_loop",
                    &format!(
                        "break 'round: LLM 返回 Stop 且无工具调用，无待处理消息 (round={}, text_len={})",
                        _round,
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
            last_round + 1
        ),
    );
    let _ = tx.send(StreamMsg::Done);
}

/// 从待处理队列中 drain 用户在 agent loop 期间发送的新消息，追加到 messages
fn drain_pending_user_messages(
    messages: &mut Vec<ChatMessage>,
    pending_user_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) {
    let mut pending = safe_lock(pending_user_messages, "agent::drain_pending");
    if !pending.is_empty() {
        // 给每条追加的用户消息添加 [User appended] 标记
        for msg in pending.iter_mut() {
            if msg.role == "user" {
                msg.content = format!("[User appended] {}", msg.content);
            }
        }
        messages.append(&mut *pending);
    }
}

/// 向共享消息列表中追加一条消息（agent 线程写入，UI 线程读取）
fn push_shared(shared: &Arc<Mutex<Vec<ChatMessage>>>, msg: ChatMessage) {
    if let Ok(mut msgs) = shared.lock() {
        msgs.push(msg);
    }
}

/// 将 streaming_content 中的文本保存为 assistant 消息（多轮 agent loop 中间轮的文本回复）
/// 调用后 streaming_content 被清空，避免 UI 侧 finish_loading 再次保存导致重复
fn flush_streaming_as_message(
    streaming_content: &Arc<Mutex<String>>,
    messages: &mut Vec<ChatMessage>,
    shared_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) {
    let mut sc = safe_lock(streaming_content, "agent::flush_streaming");
    if !sc.is_empty() {
        let text_msg = ChatMessage {
            role: ROLE_ASSISTANT.to_string(),
            content: sc.clone(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        };
        messages.push(text_msg.clone());
        push_shared(shared_messages, text_msg);
        sc.clear();
    }
}

/// 记录工具调用请求日志
fn log_tool_request(tool_items: &[ToolCallItem]) {
    let mut log_content = String::new();
    for item in tool_items {
        log_content.push_str(&format!("- {}: {}\n", item.name, item.arguments));
    }
    write_info_log("工具调用请求", &log_content);
}

/// 记录工具调用结果日志
fn log_tool_results(tool_items: &[ToolCallItem], tool_results: &[ToolResultMsg]) {
    let mut log_content = String::new();
    for (i, result) in tool_results.iter().enumerate() {
        let (tool_name, tool_args) = tool_items
            .get(i)
            .map(|t| (t.name.as_str(), t.arguments.as_str()))
            .unwrap_or(("unknown", ""));
        log_content.push_str(&format!(
            "- [{}] {}({}): {}\n",
            result.tool_call_id, tool_name, tool_args, result.result
        ));
    }
    write_info_log("工具调用结果", &log_content);
}

/// process_tool_calls 的返回结果
struct ToolCallResult {
    compact_requested: bool,
    /// Plan 被批准且用户选择清空上下文，值为 plan 文件内容
    plan_approved_clear_context: Option<String>,
}

/// 处理工具调用的公共逻辑：发送请求、等待结果、更新 messages
/// 返回 Ok(ToolCallResult) 表示成功（应 continue 循环）
/// Err(ChatError) 表示 channel 断开或执行失败
fn process_tool_calls(
    tool_items: Vec<ToolCallItem>,
    assistant_text: String,
    messages: &mut Vec<ChatMessage>,
    ctx: &ToolCallContext<'_>,
) -> Result<ToolCallResult, ChatError> {
    log_tool_request(&tool_items);

    if !assistant_text.is_empty() {
        write_info_log("Sprite 回复", &assistant_text);
    }

    // 检查是否有 compact tool 被调用
    let compact_requested = tool_items.iter().any(|t| t.name == CompactTool {}.name());

    // ★ 如果 LLM 同时返回了文本和 tool_calls，拆成两条消息：
    //   1. 纯文本 assistant 消息（让 UI 先渲染文字）
    //   2. tool_call assistant 消息（content 为空，只带 tool_calls）
    //   这样渲染时文字在上面，tool_call 在下面
    if !assistant_text.is_empty() {
        let text_msg = ChatMessage {
            role: ROLE_ASSISTANT.to_string(),
            content: assistant_text,
            tool_calls: None,
            tool_call_id: None,
            images: None,
        };
        messages.push(text_msg.clone());
        push_shared(ctx.shared_messages, text_msg);
        // 清空 streaming_content，文本已保存，避免 UI 继续显示流式内容
        if let Ok(mut sc) = ctx.streaming_content.lock() {
            sc.clear();
        }
    }

    let tool_call_msg = ChatMessage {
        role: ROLE_ASSISTANT.to_string(),
        content: String::new(),
        tool_calls: Some(tool_items.clone()),
        tool_call_id: None,
        images: None,
    };
    messages.push(tool_call_msg.clone());
    push_shared(ctx.shared_messages, tool_call_msg);

    if ctx
        .tx
        .send(StreamMsg::ToolCallRequest(tool_items.clone()))
        .is_err()
    {
        return Err(ChatError::Other("工具调用通道已断开".to_string()));
    }

    let mut tool_results: Vec<ToolResultMsg> = Vec::new();
    let mut plan_clear_context: Option<String> = None;
    for _ in &tool_items {
        match ctx.tool_result_rx.recv() {
            Ok(result) => {
                // 检测 ExitPlanMode 返回清空上下文信号
                if result.plan_decision == PlanDecision::ApproveAndClearContext {
                    plan_clear_context = Some(result.result.clone());
                }
                tool_results.push(result);
            }
            Err(_) => return Err(ChatError::Other("工具执行结果通道已断开".to_string())),
        }
    }

    log_tool_results(&tool_items, &tool_results);

    // 收集需要延迟注入的图片消息（在所有 tool results 之后统一注入，
    // 避免在 tool results 中间插入 user 消息导致 API 报错）
    let mut deferred_image_msgs: Vec<ChatMessage> = Vec::new();

    for result in tool_results {
        let mut result_content = result.result;
        let result_images = result.images;

        // 查找工具名
        let tool_name = tool_items
            .iter()
            .find(|t| t.id == result.tool_call_id)
            .map(|t| t.name.clone());

        // ★ PostToolExecution hook
        if ctx.hook_manager.has_hooks_for(HookEvent::PostToolExecution) {
            let hook_ctx = HookContext {
                event: HookEvent::PostToolExecution,
                tool_name: tool_name.clone(),
                tool_result: Some(result_content.clone()),
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            if let Some(hook_result) = ctx
                .hook_manager
                .execute(HookEvent::PostToolExecution, hook_ctx)
                && let Some(new_result) = hook_result.tool_result
            {
                result_content = new_result;
            }
        }

        let tool_msg = ChatMessage {
            role: ROLE_TOOL.to_string(),
            content: result_content,
            tool_calls: None,
            tool_call_id: Some(result.tool_call_id.clone()),
            images: None,
        };
        messages.push(tool_msg.clone());
        push_shared(ctx.shared_messages, tool_msg);

        // 如果模型支持视觉且工具返回了图片，先收集，稍后统一注入
        if !result_images.is_empty() {
            let tool_label = tool_name.as_deref().unwrap_or("unknown");
            let img_count = result_images.len();
            write_info_log(
                "ImageInjection",
                &format!(
                    "工具 {} 返回了 {} 张图片, supports_vision={}",
                    tool_label, img_count, ctx.supports_vision
                ),
            );
            if ctx.supports_vision {
                let img_msg = ChatMessage {
                    role: ROLE_USER.to_string(),
                    content: format!(
                        "[{tool_label} 返回了 {img_count} 张图片，请查看图片内容并继续帮助完成任务]"
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                    images: Some(
                        result_images
                            .into_iter()
                            .map(|img| super::super::storage::ImageData {
                                base64: img.base64,
                                media_type: img.media_type,
                            })
                            .collect(),
                    ),
                };
                deferred_image_msgs.push(img_msg);
            } else {
                write_info_log(
                    "ImageInjection",
                    &format!(
                        "supports_vision=false，丢弃 {} 返回的 {} 张图片",
                        tool_label, img_count
                    ),
                );
            }
        }
    }

    // ★ 所有 tool results 处理完毕后，统一注入图片 user messages
    if !deferred_image_msgs.is_empty() {
        write_info_log(
            "ImageInjection",
            &format!(
                "在所有 tool results 之后注入 {} 条图片消息",
                deferred_image_msgs.len()
            ),
        );
        for img_msg in deferred_image_msgs {
            // 只加入 LLM 上下文，不推送到 shared_messages（避免 UI 渲染这条内部消息）
            messages.push(img_msg);
        }
    }

    drain_pending_user_messages(messages, ctx.pending_user_messages);

    Ok(ToolCallResult {
        compact_requested,
        plan_approved_clear_context: plan_clear_context,
    })
}

// ==================== 指数退避重试 ====================

/// 每种可重试错误的重试策略
struct RetryPolicy {
    /// 最大重试次数（不含首次请求）
    max_attempts: u32,
    /// 首次退避基础延迟（毫秒）
    base_ms: u64,
    /// 延迟上限（毫秒）
    cap_ms: u64,
}

/// 根据错误类型确定重试策略
///
/// 策略设计原则：
/// - 网络瞬断（超时/断连）：快速重试，基础 1s，最多 5 次
/// - 5xx 服务端过载（503/504/529）：稍慢重试，基础 2s，最多 4 次
/// - 5xx 服务端错误（500/502）：再慢一些，基础 3s，最多 3 次
/// - 429 有 retry_after：精确等待（上限 120s），只重试 1 次
/// - 429 无 retry_after：保守重试，基础 5s，最多 3 次
/// - 非标准 finish_reason（如 network_error）：中等重试
fn retry_policy_for(error: &ChatError) -> Option<RetryPolicy> {
    match error {
        ChatError::NetworkTimeout(_) | ChatError::StreamInterrupted(_) => Some(RetryPolicy {
            max_attempts: 5,
            base_ms: 1_000,
            cap_ms: 30_000,
        }),
        ChatError::NetworkError(_) => Some(RetryPolicy {
            max_attempts: 5,
            base_ms: 2_000,
            cap_ms: 30_000,
        }),
        ChatError::ApiServerError { status, .. } => match status {
            503 | 504 | 529 => Some(RetryPolicy {
                max_attempts: 4,
                base_ms: 2_000,
                cap_ms: 30_000,
            }),
            500 | 502 => Some(RetryPolicy {
                max_attempts: 3,
                base_ms: 3_000,
                cap_ms: 30_000,
            }),
            _ => None,
        },
        ChatError::ApiRateLimit {
            retry_after_secs: Some(secs),
            ..
        } => {
            // 有明确的等待时间：等待指定时长（上限 120s），只重试一次
            let wait = (*secs).min(120);
            Some(RetryPolicy {
                max_attempts: 1,
                base_ms: wait * 1_000,
                cap_ms: 120_000,
            })
        }
        ChatError::ApiRateLimit {
            retry_after_secs: None,
            ..
        } => Some(RetryPolicy {
            max_attempts: 3,
            base_ms: 5_000,
            cap_ms: 60_000,
        }),
        ChatError::AbnormalFinish(reason)
            if matches!(reason.as_str(), "network_error" | "timeout" | "overloaded") =>
        {
            Some(RetryPolicy {
                max_attempts: 3,
                base_ms: 2_000,
                cap_ms: 20_000,
            })
        }
        // 兜底：Other 中包含过载/访问量过大关键词（部分 API 错误未被正确分类时）
        ChatError::Other(msg)
            if msg.contains("访问量过大")
                || msg.contains("过载")
                || msg.contains("overloaded")
                || msg.contains("too busy")
                || msg.contains("1305") =>
        {
            Some(RetryPolicy {
                max_attempts: 3,
                base_ms: 3_000,
                cap_ms: 30_000,
            })
        }
        _ => None,
    }
}

/// 计算第 `attempt`（从 1 开始）次重试的退避延迟（毫秒）
///
/// 公式：`clamp(base * 2^(attempt-1), 0, cap) + jitter(0..20%)`
fn backoff_delay_ms(attempt: u32, base_ms: u64, cap_ms: u64) -> u64 {
    // 最多移位 10 次，避免溢出
    let shift = (attempt - 1).min(10) as u64;
    let exp = base_ms.saturating_mul(1u64 << shift).min(cap_ms);
    // 加 0–20% 随机抖动，分散并发重试
    let jitter = rand::thread_rng().gen_range(0..=(exp / 5));
    exp + jitter
}
