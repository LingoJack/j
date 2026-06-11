//! oneshot Agent 主循环：有工具/无工具模式

use crate::command::chat::agent::config::{AgentLoopConfig, AgentLoopSharedState};
use crate::command::chat::app::AskRequest;
use crate::command::chat::app::types::StreamMsg;
use crate::command::chat::app::{MainAgentHandle, build_system_prompt_fn};
use crate::command::chat::context::compact::new_invoked_skills_map;
use crate::command::chat::context::window::select_messages;
use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::infra::skill;
use crate::command::chat::oneshot::animation::{start_thinking_animation, stop_thinking_animation};
use crate::command::chat::oneshot::ask_ui::spawn_ask_handler;
use crate::command::chat::oneshot::display::{redraw_markdown, term_width};
use crate::command::chat::oneshot::session::{fire_session_end, persist_messages};
use crate::command::chat::oneshot::tool_exec::handle_tool_call;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{AgentConfig, ChatMessage, MessageRole, ModelProvider};
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::task::TaskManager;
use crate::command::chat::tools::todo::TodoManager;
use crate::error;
use crate::theme::Theme;
use colored::Colorize;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Agent 结果轮询间隔（毫秒）。
const ONESHOT_POLL_MS: u64 = 30;

/// 无工具模式：流式输出 + Markdown 重绘
pub(crate) fn run_oneshot_no_tools(
    provider: &ModelProvider,
    agent_config: &AgentConfig,
    message: String,
    prior_messages: Vec<ChatMessage>,
    session_id: &str,
    no_render: bool,
) {
    use crate::command::chat::agent::api::call_llm_stream;
    use crate::command::chat::oneshot::animation::start_thinking_animation;
    use crate::command::chat::oneshot::display::redraw_markdown;
    use crate::command::chat::oneshot::session::persist_messages;

    let user_msg = ChatMessage::text(MessageRole::User, message.clone());
    let mut messages = prior_messages.clone();
    messages.push(user_msg.clone());

    let thinking_style = agent_config.thinking_style;
    let _stop_anim = if no_render {
        // no_render 模式下跳过动画（避免 stdout 噪音）
        Arc::new(AtomicBool::new(true))
    } else {
        start_thinking_animation(thinking_style)
    };

    let tw = term_width();
    let mut cur_col: usize = 0;
    let mut raw_lines: usize = 0;
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_for_handler = Arc::clone(&interrupted);
    let _ = ctrlc::set_handler(move || {
        let _ = crossterm::terminal::disable_raw_mode();
        interrupted_for_handler.store(true, Ordering::Relaxed);
    });

    let send_messages = select_messages(
        &messages,
        agent_config.max_history_messages,
        agent_config.max_context_tokens,
        agent_config.compact.keep_recent,
        &agent_config.compact.micro_compact_exempt_tools,
    );

    match call_llm_stream(
        provider,
        &send_messages,
        crate::command::chat::storage::load_system_prompt().as_deref(),
        &mut |chunk| {
            if interrupted.load(Ordering::Relaxed) {
                return;
            }
            print!("{}", chunk);
            let _ = io::stdout().flush();
            for ch in chunk.chars() {
                use unicode_width::UnicodeWidthChar;
                if ch == '\n' {
                    raw_lines += 1;
                    cur_col = 0;
                } else {
                    cur_col += ch.width().unwrap_or(0);
                    if cur_col >= tw {
                        raw_lines += 1;
                        cur_col = 0;
                    }
                }
            }
        },
    ) {
        Ok(full_text) => {
            // ★ 无论回复是否为空，都持久化 user message，避免产生空会话记录
            persist_messages(session_id, &[user_msg], 0);

            if !full_text.is_empty() {
                if !no_render {
                    redraw_markdown(raw_lines, cur_col, &full_text);
                } else {
                    // no_render 模式下，流式文本已经 print 到 stdout，补一个换行即可
                    println!();
                }
                persist_messages(
                    session_id,
                    &[ChatMessage::text(MessageRole::Assistant, &full_text)],
                    0,
                );
            }
            eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
        }
        Err(e) => {
            // ★ LLM 调用失败时也持久化 user message，保留用户输入记录
            persist_messages(session_id, &[user_msg], 0);
            error!("\n{}", e.display_message());
        }
    }
}

/// 有工具模式：Agent 主循环
pub(crate) fn run_oneshot_agent(
    provider: &ModelProvider,
    agent_config: &AgentConfig,
    message: String,
    prior_messages: Vec<ChatMessage>,
    session_id: &str,
    bypass: bool,
    no_render: bool,
) {
    let thinking_style = agent_config.thinking_style;

    let hook_manager_loaded = HookManager::load();
    let hook_manager_for_end = hook_manager_loaded.clone();
    let disabled_hooks: Vec<String> = vec![];

    // ★ SessionStart hook
    {
        if hook_manager_loaded.has_hooks_for(HookEvent::SessionStart) {
            let ctx = HookContext {
                event: HookEvent::SessionStart,
                messages: Some(prior_messages.clone()),
                model: Some(provider.model.clone()),
                session_id: Some(session_id.to_string()),
                ..Default::default()
            };
            hook_manager_loaded.execute(HookEvent::SessionStart, ctx, &disabled_hooks);
        }
    }

    let (ask_tx, ask_rx) = std::sync::mpsc::channel::<AskRequest>();
    let background_manager = Arc::new(BackgroundManager::new());
    let task_manager = Arc::new(TaskManager::new_with_session(session_id));
    let todo_manager = Arc::new(TodoManager::new());
    let hook_manager_for_registry = hook_manager_loaded.clone();
    let invoked_skills = new_invoked_skills_map();

    let mut tool_registry = ToolRegistry::new(
        vec![],
        ask_tx,
        Arc::clone(&background_manager),
        Arc::clone(&task_manager),
        Arc::new(Mutex::new(hook_manager_for_registry)),
        invoked_skills.clone(),
        crate::command::chat::storage::SessionPaths::new(session_id).todos_file(),
    );
    // 注册 LoadTool
    let deferred_tools_for_load = Arc::new(Mutex::new(agent_config.deferred_tools.clone()));
    let session_loaded_for_load: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    tool_registry.register(Box::new(
        crate::command::chat::tools::load_tool::LoadTool::new(
            Arc::clone(&deferred_tools_for_load),
            Arc::clone(&session_loaded_for_load),
        ),
    ));
    let tool_registry = Arc::new(tool_registry);

    // 启动 Ask 请求处理线程
    spawn_ask_handler(ask_rx);

    // 构建消息
    let user_msg = ChatMessage::text(MessageRole::User, &message);
    let prior_len = prior_messages.len();
    let mut messages = prior_messages.clone();
    messages.push(user_msg.clone());

    let loaded_skills = skill::load_all_skills();
    let teammate_manager: Arc<Mutex<TeammateManager>> = Arc::new(Mutex::new(TeammateManager::new(
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
    )));
    let system_prompt_fn = build_system_prompt_fn(
        loaded_skills,
        agent_config.disabled_skills.clone(),
        agent_config.disabled_tools.clone(),
        provider.tools_allowed(agent_config.tools_enabled),
        Arc::clone(&deferred_tools_for_load),
        Arc::clone(&tool_registry),
        teammate_manager,
        Arc::clone(&task_manager),
        Arc::clone(&background_manager),
    );

    let api_messages = select_messages(
        &messages,
        agent_config.max_history_messages,
        agent_config.max_context_tokens,
        agent_config.compact.keep_recent,
        &agent_config.compact.micro_compact_exempt_tools,
    );

    // 构造 AgentLoopConfig + AgentLoopSharedState
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let streaming_content: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let streaming_reasoning_content: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let pending_user_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let display_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let context_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));

    // ★ 将 prior_messages + user_msg 同步到 context_messages，
    //   这样 agent loop 中的 push_both 会在此基础上追加 assistant/tool 消息，
    //   最终 persist_messages 能拿到完整的消息序列。
    {
        let mut ctx = context_messages.lock().unwrap();
        ctx.extend(prior_messages.iter().cloned());
        ctx.push(user_msg.clone());
    }
    let estimated_context_tokens: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let derived_system_prompt: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let agent_config_struct = AgentLoopConfig {
        provider: provider.clone(),
        max_llm_rounds: agent_config.max_tool_rounds,
        compact_config: agent_config.compact.clone(),
        hook_manager: hook_manager_loaded,
        disabled_hooks: agent_config.disabled_hooks.clone(),
        cancel_token: cancel_token.clone(),
    };
    let agent_shared = AgentLoopSharedState {
        streaming_content: Arc::clone(&streaming_content),
        streaming_reasoning_content: Arc::clone(&streaming_reasoning_content),
        pending_user_messages,
        background_manager,
        todo_manager,
        display_messages: Arc::clone(&display_messages),
        context_messages: Arc::clone(&context_messages),
        estimated_context_tokens,
        invoked_skills,
        session_id: session_id.to_string(),
        derived_system_prompt,
        tool_registry: Arc::clone(&tool_registry),
        disabled_tools: agent_config.disabled_tools.clone(),
        deferred_tools: Arc::clone(&deferred_tools_for_load),
        session_loaded_deferred: Arc::clone(&session_loaded_for_load),
        tools_enabled: agent_config.tools_enabled,
        sub_agent_metrics: Arc::new(Mutex::new(
            crate::command::chat::tools::derived_shared::SubAgentMetrics::default(),
        )),
    };

    // Ctrl+C → 设标志，让主 loop 优雅退回（不杀进程，REPL 仍可继续）
    let cancel_for_ctrlc = cancel_token.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_for_ctrlc = Arc::clone(&cancelled);
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_for_handler = Arc::clone(&interrupted);
    let _ = ctrlc::set_handler(move || {
        let _ = crossterm::terminal::disable_raw_mode();
        cancel_for_ctrlc.cancel();
        cancelled_for_ctrlc.store(true, Ordering::Relaxed);
        interrupted_for_handler.store(true, Ordering::Relaxed);
    });

    // spawn agent loop
    let (handle, tool_result_tx) = MainAgentHandle::spawn(
        agent_config_struct,
        agent_shared,
        api_messages,
        system_prompt_fn,
    );
    let tool_result_tx: std::sync::mpsc::SyncSender<
        crate::command::chat::app::types::ToolResultMsg,
    > = tool_result_tx;

    // 启动思考动画
    let anim_stop = start_thinking_animation(thinking_style);
    let mut anim_running = true;

    // 消费循环
    let mut last_streaming_len: usize = 0;
    let jcli_config = JcliConfig::load();
    let mut round: usize = 0;
    let mut first_content = true;
    // 流式文本行数跟踪（用于 Markdown 重绘时基于行数回退，不依赖 DSR）
    let tw = term_width();
    let mut raw_lines: usize = 0;
    let mut cur_col: usize = 0;
    // 本地保存完整文本（j-agent 的 flush_streaming_as_message 会在 Done 前清空 streaming_content）
    let mut full_text_for_redraw: String = String::new();

    loop {
        // 优先检查中断标志：用户按 Ctrl+C 后立即退出，回到 REPL
        if interrupted.load(Ordering::Relaxed) {
            if anim_running {
                stop_thinking_animation(&anim_stop);
            }
            eprintln!("\n  {}", "⏹ 已中断".dimmed());
            return;
        }
        let msgs = handle.poll();
        if msgs.is_empty() {
            std::thread::sleep(Duration::from_millis(ONESHOT_POLL_MS));
            continue;
        }
        for msg in msgs {
            match msg {
                StreamMsg::Chunk => {
                    let content = streaming_content.lock().unwrap();
                    if content.len() > last_streaming_len {
                        // 停止思考动画（首次文本到来时）
                        if anim_running {
                            stop_thinking_animation(&anim_stop);
                            anim_running = false;
                        }
                        // 打印 AI 标签（首次文本到来时）
                        if first_content {
                            if !no_render {
                                let theme = Theme::terminal();
                                eprintln!();
                                eprintln!(
                                    "  {}",
                                    crate::util::color_adapt::apply_fg("Sprite", theme.label_ai)
                                        .bold()
                                );
                                // 首次输出前先打印缩进
                                print!("  ");
                                let _ = io::stdout().flush();
                            }
                            first_content = false;
                        }

                        let delta = &content[last_streaming_len..];
                        if no_render {
                            // no_render 模式：原样输出，避免污染重定向到文件的内容
                            print!("{}", delta);
                        } else {
                            // 缩进输出：每个换行后加 "  " 缩进（仅终端显示）
                            // 同时跟踪行数用于 markdown 重绘（使用 Unicode 宽度）
                            use unicode_width::UnicodeWidthChar;
                            for ch in delta.chars() {
                                if ch == '\n' {
                                    print!("\n  ");
                                    raw_lines += 1;
                                    cur_col = 2; // "  " 缩进占 2 列
                                } else {
                                    print!("{}", ch);
                                    cur_col += ch.width().unwrap_or(0);
                                    if cur_col >= tw {
                                        raw_lines += 1;
                                        cur_col = 0;
                                    }
                                }
                            }
                        }
                        let _ = io::stdout().flush();
                        full_text_for_redraw = content.to_string();
                        last_streaming_len = full_text_for_redraw.len();
                    }
                }
                StreamMsg::ToolCallRequest(items) => {
                    // 停止思考动画
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                        anim_running = false;
                    }
                    // 先重绘已输出的流式文本
                    if last_streaming_len > 0 && !no_render {
                        redraw_markdown(raw_lines, cur_col, &full_text_for_redraw);
                        last_streaming_len = 0;
                        raw_lines = 0;
                        cur_col = 0;
                        full_text_for_redraw.clear(); // 清空准备下一轮
                    } else if last_streaming_len > 0 {
                        // no_render 模式下补一个换行
                        println!();
                        last_streaming_len = 0;
                        full_text_for_redraw.clear();
                    }

                    round += 1;

                    // 轮次标题
                    eprintln!();
                    eprintln!("  {} R{} · {} 工具", "⚙".dimmed(), round, items.len());

                    // 逐个确认 + 执行 + 发送结果
                    for item in items.iter() {
                        let tool_result = handle_tool_call(
                            item,
                            Arc::clone(&tool_registry),
                            &jcli_config,
                            &cancelled,
                            &interrupted,
                            bypass,
                        );
                        match tool_result {
                            Some(r) => {
                                let _ = tool_result_tx.send(r);
                            }
                            None => {
                                // 用户中断：放弃后续工具，退回 REPL
                                eprintln!("\n  {}", "⏹ 已中断".dimmed());
                                return;
                            }
                        }
                    }

                    // 下一轮工具调用结束后，重置 first_content 标记
                    first_content = true;
                }
                StreamMsg::Done => {
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                    }
                    if last_streaming_len > 0 && !no_render {
                        redraw_markdown(raw_lines, cur_col, &full_text_for_redraw);
                    } else if last_streaming_len > 0 {
                        // no_render 模式下补一个换行
                        println!();
                    }
                    let ctx_msgs = context_messages.lock().unwrap();
                    let persist_from = if prior_len < ctx_msgs.len() {
                        prior_len
                    } else {
                        0
                    };
                    persist_messages(session_id, &ctx_msgs, persist_from);
                    if round > 0 {
                        eprintln!();
                    }
                    eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
                    fire_session_end(
                        &hook_manager_for_end,
                        &disabled_hooks,
                        &ctx_msgs,
                        session_id,
                        &provider.model,
                    );
                    return;
                }
                StreamMsg::Error(e) => {
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                    }
                    error!("\n{}", e.display_message());
                    let ctx_msgs = context_messages.lock().unwrap();
                    let persist_from = if prior_len < ctx_msgs.len() {
                        prior_len
                    } else {
                        0
                    };
                    persist_messages(session_id, &ctx_msgs, persist_from);
                    fire_session_end(
                        &hook_manager_for_end,
                        &disabled_hooks,
                        &ctx_msgs,
                        session_id,
                        &provider.model,
                    );
                    return;
                }
                StreamMsg::Cancelled => {
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                    }
                    println!();
                    let ctx_msgs = context_messages.lock().unwrap();
                    let persist_from = if prior_len < ctx_msgs.len() {
                        prior_len
                    } else {
                        0
                    };
                    persist_messages(session_id, &ctx_msgs, persist_from);
                    eprintln!("\n  {}", "⏹ 已中断".dimmed());
                    eprintln!("  {} {}", "会话 ID:".dimmed(), session_id.dimmed());
                    fire_session_end(
                        &hook_manager_for_end,
                        &disabled_hooks,
                        &ctx_msgs,
                        session_id,
                        &provider.model,
                    );
                    return;
                }
                StreamMsg::Retrying {
                    attempt,
                    max_attempts,
                    delay_ms,
                    error,
                } => {
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                        anim_running = false;
                    }
                    eprintln!(
                        "  {} 重试中 ({}/{}, {}ms) — {}",
                        "⟳".yellow(),
                        attempt,
                        max_attempts,
                        delay_ms,
                        error.dimmed()
                    );
                }
                StreamMsg::Compacting => {
                    eprintln!("  {} 压缩上下文中...", "📦".dimmed());
                }
                StreamMsg::Compacted { messages_before } => {
                    eprintln!("  {} 已压缩 {} 条消息", "📦".dimmed(), messages_before);
                }
            }
        }
    }
}
