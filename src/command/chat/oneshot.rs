use crate::command::chat::agent::api::{
    build_request_with_tools, call_llm_stream, create_llm_client,
};
use crate::command::chat::agent_md::load_agent_md;
use crate::command::chat::app::AskRequest;
use crate::command::chat::context::compact::{self, new_invoked_skills_map};
use crate::command::chat::context::window::select_messages;
use crate::command::chat::error::ChatError;
use crate::command::chat::handler::run_chat_tui;
use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::infra::skill::{self, project_skills_dir, skills_dir};
use crate::command::chat::permission::{JcliConfig, generate_allow_rule};
use crate::command::chat::storage::{
    AgentConfig, ChatMessage, MessageRole, ModelProvider, SessionEvent, ToolCallItem,
    append_session_event, find_latest_session_id, load_agent_config, load_memory, load_session,
    load_soul, load_style, load_system_prompt,
};
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::{BackgroundManager, build_running_summary};
use crate::command::chat::tools::task::{TaskManager, build_tasks_summary};
use crate::config::YamlConfig;
use crate::util::log::write_info_log;
use crate::{error, info};
use std::io::{self, Write};
use std::sync::Arc;

fn generate_oneshot_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let pid = std::process::id();
    format!("{:x}-{:x}", ts, pid)
}

fn persist_messages(session_id: &str, messages: &[ChatMessage], start_idx: usize) {
    for msg in messages.iter().skip(start_idx) {
        append_session_event(session_id, &SessionEvent::msg(msg.clone()));
    }
}

/// 处理 chat 子命令入口
pub fn handle_chat(
    content: &[String],
    cont: bool,
    session_id_opt: Option<&str>,
    remote: bool,
    port: u16,
    bypass: bool,
    _config: &YamlConfig,
) {
    let agent_config = load_agent_config();

    // --remote 始终进入 TUI 模式（远程控制需要 TUI 事件循环）
    if remote
        || content.is_empty() && !cont && session_id_opt.is_none()
        || agent_config.providers.is_empty()
    {
        // 无参数 / remote / 尚未配置 provider：进入 TUI 对话界面
        // 若 providers 为空，TUI 会自动切换到配置界面引导用户完成配置
        run_chat_tui(remote, port);
        return;
    }

    // 有参数：快速发送消息并打印回复
    let message = content.join(" ");
    let message = message.trim().to_string();
    if message.is_empty() && !cont && session_id_opt.is_none() {
        error!("⚠️ 消息内容为空");
        return;
    }
    if message.is_empty() {
        error!("⚠️ 消息内容为空（--continue / --session 需要附带消息内容）");
        return;
    }

    // 解析会话 ID
    let session_id = if let Some(id) = session_id_opt {
        id.to_string()
    } else if cont {
        find_latest_session_id().unwrap_or_else(generate_oneshot_session_id)
    } else {
        generate_oneshot_session_id()
    };

    // 加载历史消息（--continue 或 --session 时）
    let prior_messages = if cont || session_id_opt.is_some() {
        let loaded = load_session(&session_id).messages;
        if !loaded.is_empty() {
            info!("📂 延续会话 {} （{} 条历史消息）", session_id, loaded.len());
        } else if session_id_opt.is_some() {
            info!("📂 会话 {} 不存在或为空，开始新对话", session_id);
        }
        loaded
    } else {
        vec![]
    };

    let idx = agent_config
        .active_index
        .min(agent_config.providers.len() - 1);
    let provider = &agent_config.providers[idx];

    info!("💫 [{}] 思考中...", provider.name);

    if agent_config.tools_enabled {
        run_oneshot_agent(
            provider,
            &agent_config,
            message,
            prior_messages,
            &session_id,
            bypass,
        );
    } else {
        // 无工具模式：流式输出 + 结束后 markdown 重绘
        use crossterm::{cursor, execute, terminal};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let user_msg = ChatMessage::text(MessageRole::User, message.clone());
        let mut messages = prior_messages.clone();
        messages.push(user_msg.clone());

        let term_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);
        let mut cur_col: usize = 0;
        let mut raw_lines: usize = 0;
        let interrupted = Arc::new(AtomicBool::new(false));
        let interrupted2 = Arc::clone(&interrupted);
        let _ = ctrlc::set_handler(move || {
            interrupted2.store(true, Ordering::Relaxed);
        });

        // 发送给 API 时使用优先级消息窗口选择（与 CompactConfig 对齐）
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
            load_system_prompt().as_deref(),
            &mut |chunk| {
                if interrupted.load(Ordering::Relaxed) {
                    return;
                }
                print!("{}", chunk);
                let _ = io::stdout().flush();
                for ch in chunk.chars() {
                    if ch == '\n' {
                        raw_lines += 1;
                        cur_col = 0;
                    } else {
                        cur_col += 1;
                        if cur_col >= term_width {
                            raw_lines += 1;
                            cur_col = 0;
                        }
                    }
                }
            },
        ) {
            Ok(full_text) => {
                if !full_text.is_empty() {
                    let total_raw_lines = if cur_col > 0 {
                        raw_lines + 1
                    } else {
                        raw_lines
                    };
                    let mut stdout = io::stdout();
                    if total_raw_lines > 0 {
                        let _ = execute!(stdout, cursor::MoveToColumn(0));
                        if total_raw_lines > 1 {
                            let _ = execute!(stdout, cursor::MoveUp((total_raw_lines - 1) as u16));
                        }
                        let _ =
                            execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
                    }
                    crate::util::md_render::render_md(&full_text);
                    // 持久化本轮新增的两条消息
                    persist_messages(&session_id, &[user_msg], 0);
                    persist_messages(
                        &session_id,
                        &[ChatMessage::text(MessageRole::Assistant, &full_text)],
                        0,
                    );
                    use colored::Colorize;
                    eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
                }
            }
            Err(e) => {
                error!("\n✖️ {}", e.display_message());
            }
        }
    }
}

fn run_oneshot_agent(
    provider: &ModelProvider,
    agent_config: &AgentConfig,
    message: String,
    prior_messages: Vec<ChatMessage>,
    session_id: &str,
    bypass: bool,
) {
    use colored::Colorize;
    use crossterm::event::{self, Event, KeyCode};
    use crossterm::{cursor, execute, terminal};
    use futures::StreamExt;
    use std::sync::{Arc, Mutex, atomic::AtomicBool};

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("创建异步运行时失败: {}", e);
            return;
        }
    };

    // 加载 hooks（用户级 + 项目级）
    let hook_manager = Arc::new(Mutex::new(HookManager::load()));
    let disabled_hooks: Vec<String> = vec![];

    // 构建工具注册表
    let (ask_tx, ask_rx) = std::sync::mpsc::channel::<AskRequest>();
    let background_manager = Arc::new(BackgroundManager::new());
    let task_manager = Arc::new(TaskManager::new_with_session(session_id));
    let hook_manager_for_registry = Arc::clone(&hook_manager);
    let invoked_skills = new_invoked_skills_map();
    let background_for_prompt = Arc::clone(&background_manager);
    let task_for_prompt = Arc::clone(&task_manager);
    let tool_registry = ToolRegistry::new(
        vec![],
        ask_tx,
        background_manager,
        task_manager,
        hook_manager_for_registry,
        invoked_skills.clone(),
        crate::command::chat::storage::SessionPaths::new(session_id).todos_file(),
    );

    // ★ SessionStart hook
    {
        let hm = hook_manager.lock().unwrap();
        if hm.has_hooks_for(HookEvent::SessionStart) {
            let ctx = HookContext {
                event: HookEvent::SessionStart,
                messages: Some(prior_messages.clone()),
                model: Some(provider.model.clone()),
                session_id: Some(session_id.to_string()),
                ..Default::default()
            };
            hm.execute(HookEvent::SessionStart, ctx, &disabled_hooks);
        }
    }

    // 启动 Ask 请求处理线程：在终端交互式回答 AI 的提问
    std::thread::spawn(move || {
        use crossterm::event::{self, Event, KeyCode};
        use crossterm::{cursor, execute, terminal};

        while let Ok(req) = ask_rx.recv() {
            let mut answers = serde_json::Map::new();
            for q in &req.questions {
                // 显示问题
                println!("\n{}  {}", " ❓ ".cyan().bold(), q.question.cyan().bold());
                if !q.header.is_empty() {
                    println!("   {}", q.header.dimmed());
                }

                if q.multi_select {
                    // 多选模式：用空格切换，Enter 确认
                    let mut selected = vec![false; q.options.len()];
                    let mut cursor_pos: usize = 0;
                    let total_lines = (q.options.len() + 1) as u16;

                    let draw_multi = |stdout: &mut io::Stdout,
                                      cursor_pos: usize,
                                      selected: &[bool],
                                      first: bool|
                     -> io::Result<()> {
                        if !first {
                            let _ = execute!(stdout, cursor::MoveUp(total_lines));
                        }
                        let _ =
                            execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
                        for (i, opt) in q.options.iter().enumerate() {
                            let pointer = if cursor_pos == i { "❯" } else { " " };
                            let check = if selected[i] { "✔" } else { "○" };
                            let line = format!(
                                "{} {} {} — {}",
                                pointer, check, opt.label, opt.description
                            );
                            if cursor_pos == i {
                                write!(stdout, "{}\r\n", line.cyan().bold())?;
                            } else {
                                write!(stdout, "{}\r\n", line.dimmed())?;
                            }
                        }
                        write!(
                            stdout,
                            "{} ↑↓ 移动  {} 切换  {} 确认\r\n",
                            "•".dimmed(),
                            "Space".dimmed(),
                            "Enter".dimmed()
                        )?;
                        stdout.flush()?;
                        Ok(())
                    };

                    let _ = terminal::enable_raw_mode();
                    let mut stdout = io::stdout();
                    let _ = draw_multi(&mut stdout, cursor_pos, &selected, true);

                    loop {
                        if let Ok(Event::Key(key)) = event::read() {
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    cursor_pos = cursor_pos.saturating_sub(1);
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if cursor_pos + 1 < q.options.len() {
                                        cursor_pos += 1;
                                    }
                                }
                                KeyCode::Char(' ') => {
                                    selected[cursor_pos] = !selected[cursor_pos];
                                }
                                KeyCode::Enter => break,
                                KeyCode::Esc => break,
                                _ => continue,
                            }
                            let _ = draw_multi(&mut stdout, cursor_pos, &selected, false);
                        }
                    }
                    let _ = terminal::disable_raw_mode();
                    let _ = execute!(stdout, cursor::MoveUp(total_lines));
                    let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));

                    let result: Vec<String> = q
                        .options
                        .iter()
                        .zip(selected.iter())
                        .filter(|(_, s)| **s)
                        .map(|(o, _)| o.label.clone())
                        .collect();
                    let answer = if result.is_empty() {
                        "(无选择)".to_string()
                    } else {
                        result.join(", ")
                    };
                    println!("  → {}", answer.green());
                    answers.insert(q.header.clone(), serde_json::Value::String(answer));
                } else {
                    // 单选模式：复用 interactive_confirm 的模式
                    let options: Vec<String> = q
                        .options
                        .iter()
                        .map(|o| format!("{} — {}", o.label, o.description))
                        .collect();
                    let option_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
                    let mut cursor_pos: usize = 0;
                    let total_lines = (option_refs.len() + 1) as u16;

                    let draw_single = |stdout: &mut io::Stdout,
                                       cursor_pos: usize,
                                       opts: &[&str],
                                       first: bool|
                     -> io::Result<()> {
                        if !first {
                            let _ = execute!(stdout, cursor::MoveUp(total_lines));
                        }
                        let _ =
                            execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
                        for (i, opt) in opts.iter().enumerate() {
                            let pointer = if cursor_pos == i { "❯" } else { " " };
                            if cursor_pos == i {
                                write!(
                                    stdout,
                                    "{} {}\r\n",
                                    pointer.cyan().bold(),
                                    opt.cyan().bold()
                                )?;
                            } else {
                                write!(stdout, "{} {}\r\n", pointer, opt.dimmed())?;
                            }
                        }
                        write!(
                            stdout,
                            "{} ↑↓ 选择  {} 确认\r\n",
                            "•".dimmed(),
                            "Enter".dimmed()
                        )?;
                        stdout.flush()?;
                        Ok(())
                    };

                    let _ = terminal::enable_raw_mode();
                    let mut stdout = io::stdout();
                    let _ = draw_single(&mut stdout, cursor_pos, &option_refs, true);

                    loop {
                        if let Ok(Event::Key(key)) = event::read() {
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    cursor_pos = cursor_pos.saturating_sub(1);
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if cursor_pos + 1 < option_refs.len() {
                                        cursor_pos += 1;
                                    }
                                }
                                KeyCode::Enter => break,
                                KeyCode::Esc => break,
                                _ => continue,
                            }
                            let _ = draw_single(&mut stdout, cursor_pos, &option_refs, false);
                        }
                    }
                    let _ = terminal::disable_raw_mode();
                    let _ = execute!(stdout, cursor::MoveUp(total_lines));
                    let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));

                    let answer = q
                        .options
                        .get(cursor_pos)
                        .map(|o| o.label.clone())
                        .unwrap_or_default();
                    println!("  → {}", answer.green());
                    answers.insert(q.header.clone(), serde_json::Value::String(answer));
                }
            }

            let response = serde_json::to_string(&serde_json::json!({ "answers": answers }))
                .unwrap_or_default();
            let _ = req.response_tx.send(response);
        }
    });

    let llm_tools = tool_registry.to_llm_tools_filtered(&agent_config.disabled_tools);
    let mut jcli_config = JcliConfig::load();
    let cancelled = Arc::new(AtomicBool::new(false));
    let max_rounds = agent_config.max_tool_rounds;
    let compact_config = &agent_config.compact;

    // 构建初始消息列表：先写入历史，再追加本次用户消息
    let user_msg = ChatMessage::text(MessageRole::User, message);
    let prior_len = prior_messages.len();
    let mut messages = prior_messages;
    messages.push(user_msg);

    /// 交互式工具确认（crossterm raw mode，↑↓ 选择，Enter 确认）
    /// 返回选中的选项索引
    fn interactive_confirm(tool_msg: &str, options: &[&str], initial: usize) -> Option<usize> {
        let mut stdout = io::stdout();
        let mut cursor_pos = initial;
        let total_lines = (1 + options.len() + 1) as u16; // 工具描述 + 选项 + 提示行

        // 绘制一次
        let draw = |stdout: &mut io::Stdout,
                    cursor_pos: usize,
                    first: bool,
                    total_lines: u16|
         -> io::Result<()> {
            if !first {
                let _ = execute!(stdout, cursor::MoveUp(total_lines));
            }
            let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
            write!(stdout, "{}\r\n", tool_msg)?;
            for (i, opt) in options.iter().enumerate() {
                let pointer = if cursor_pos == i { "❯" } else { " " };
                if cursor_pos == i {
                    write!(
                        stdout,
                        "{} {}\r\n",
                        pointer.cyan().bold(),
                        opt.cyan().bold()
                    )?;
                } else {
                    write!(stdout, "{} {}\r\n", pointer, opt.dimmed())?;
                }
            }
            write!(
                stdout,
                "{} ↑↓ 选择  {} 确认\r\n",
                "•".dimmed(),
                "Enter".dimmed()
            )?;
            stdout.flush()?;
            Ok(())
        };

        if terminal::enable_raw_mode().is_err() {
            return None;
        }

        let _ = draw(&mut stdout, cursor_pos, true, total_lines);

        let result = loop {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        cursor_pos = cursor_pos.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if cursor_pos + 1 < options.len() {
                            cursor_pos += 1;
                        }
                    }
                    KeyCode::Enter => break Some(cursor_pos),
                    KeyCode::Esc | KeyCode::Char('q') => break None,
                    _ => continue,
                }
                let _ = draw(&mut stdout, cursor_pos, false, total_lines);
            }
        };

        let _ = terminal::disable_raw_mode();
        // 清除菜单内容
        {
            let _ = execute!(stdout, cursor::MoveUp(total_lines));
            let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
        }
        result
    }

    let client = create_llm_client(provider);

    // Ctrl+C 中断标志
    let ctrl_c = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // 流式调用结果
    struct StreamResult {
        assistant_text: String,
        assistant_reasoning: String,
        tool_items: Vec<ToolCallItem>,
        raw_lines: usize,
        cur_col: usize,
    }

    for _round in 0..max_rounds {
        // ── micro_compact（替换旧 tool results）──
        if compact_config.enabled {
            compact::micro_compact(
                &mut messages,
                compact_config.keep_recent,
                &compact_config.micro_compact_exempt_tools,
            );
        }

        // 每轮构建 system prompt（从磁盘读取最新配置）
        let mut system_prompt = resolve_oneshot_system_prompt(
            &tool_registry,
            &agent_config.disabled_tools,
            &background_for_prompt,
            &task_for_prompt,
        );

        // ★ PreLlmRequest hook（可修改 messages 和 system_prompt）
        {
            let hm = hook_manager.lock().unwrap();
            if hm.has_hooks_for(HookEvent::PreLlmRequest) {
                let ctx = HookContext {
                    event: HookEvent::PreLlmRequest,
                    messages: Some(messages.clone()),
                    system_prompt: system_prompt.clone(),
                    model: Some(provider.model.clone()),
                    session_id: Some(session_id.to_string()),
                    ..Default::default()
                };
                if let Some(result) = hm.execute(HookEvent::PreLlmRequest, ctx, &disabled_hooks) {
                    if result.is_stop() {
                        error!("PreLlmRequest hook 中止了请求");
                        persist_messages(session_id, &messages, prior_len);
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
        }

        // ── 消息窗口选择（与 TUI 对齐）──
        let send_messages = select_messages(
            &messages,
            agent_config.max_history_messages,
            agent_config.max_context_tokens,
            compact_config.keep_recent,
            &compact_config.micro_compact_exempt_tools,
        );

        // ── 异步部分：API 流式调用 ──
        let ctrl_c_stream = Arc::clone(&ctrl_c);
        let stream_result: Result<StreamResult, ChatError> = rt.block_on(async {
            let request = build_request_with_tools(
                provider,
                &send_messages,
                llm_tools.clone(),
                system_prompt.as_deref(),
            )?;

            let mut stream = client
                .chat_completion_stream(&request)
                .await
                .map_err(ChatError::from)?;

            let mut assistant_text = String::new();
            let mut assistant_reasoning = String::new();
            let mut raw_tool_calls: std::collections::BTreeMap<u32, (String, String, String)> =
                std::collections::BTreeMap::new();
            let mut finish_reason: Option<String> = None;
            let term_width = crossterm::terminal::size()
                .map(|(w, _)| w as usize)
                .unwrap_or(80);
            let mut cur_col: usize = 0;
            let mut raw_lines: usize = 0;

            loop {
                let chunk = tokio::select! {
                    biased;
                    _ = tokio::signal::ctrl_c() => {
                        ctrl_c_stream.store(true, std::sync::atomic::Ordering::Relaxed);
                        break;
                    }
                    chunk = stream.next() => chunk,
                };
                let Some(result) = chunk else { break };
                match result {
                    Ok(response) => {
                        for choice in &response.choices {
                            if let Some(ref content) = choice.delta.content {
                                assistant_text.push_str(content);
                                print!("{}", content);
                                let _ = io::stdout().flush();
                                for ch in content.chars() {
                                    if ch == '\n' {
                                        raw_lines += 1;
                                        cur_col = 0;
                                    } else {
                                        cur_col += 1;
                                        if cur_col >= term_width {
                                            raw_lines += 1;
                                            cur_col = 0;
                                        }
                                    }
                                }
                            }
                            if let Some(ref reasoning) = choice.delta.reasoning_content {
                                assistant_reasoning.push_str(reasoning);
                            }
                            if let Some(ref tc_chunks) = choice.delta.tool_calls {
                                for chunk in tc_chunks {
                                    let entry =
                                        raw_tool_calls.entry(chunk.index).or_insert_with(|| {
                                            (
                                                chunk.id.clone().unwrap_or_default(),
                                                String::new(),
                                                String::new(),
                                            )
                                        });
                                    if entry.0.is_empty()
                                        && let Some(ref id) = chunk.id
                                    {
                                        entry.0 = id.clone();
                                    }
                                    if let Some(ref f) = chunk.function {
                                        if let Some(ref name) = f.name {
                                            entry.1.push_str(name);
                                        }
                                        if let Some(ref args) = f.arguments {
                                            entry.2.push_str(args);
                                        }
                                    }
                                }
                            }
                            if let Some(ref fr) = choice.finish_reason {
                                finish_reason = Some(fr.clone());
                            }
                        }
                    }
                    Err(e) => return Err(ChatError::from(e)),
                }
            }

            let is_tool_calls = finish_reason.as_deref() == Some("tool_calls");
            let tool_items: Vec<ToolCallItem> = if is_tool_calls {
                raw_tool_calls
                    .into_values()
                    .map(|(id, name, arguments)| {
                        // 某些 API 在流式 chunk 中不返回 tool_call id，
                        // 导致 id 为空字符串；发送给 API 时会报 tool_call_id not found。
                        // 此处为空 id 生成随机唯一 id。
                        let id = if id.is_empty() {
                            use rand::Rng;
                            format!("call_{:016x}", rand::thread_rng().r#gen::<u64>())
                        } else {
                            id
                        };
                        ToolCallItem {
                            id,
                            name,
                            arguments,
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

            Ok(StreamResult {
                assistant_text,
                assistant_reasoning,
                tool_items,
                raw_lines,
                cur_col,
            })
        });

        let sr = match stream_result {
            Ok(sr) => sr,
            Err(e) => {
                error!("\n{}", e.display_message());
                // ★ SessionEnd hook（出错退出）
                fire_session_end(
                    &hook_manager,
                    &disabled_hooks,
                    &messages,
                    session_id,
                    &provider.model,
                );
                return;
            }
        };

        // Ctrl+C 打断：持久化已有内容后退出
        if ctrl_c.load(std::sync::atomic::Ordering::Relaxed) {
            println!();
            if !sr.assistant_text.is_empty() {
                messages.push(ChatMessage::text(
                    MessageRole::Assistant,
                    &sr.assistant_text,
                ));
            }
            persist_messages(session_id, &messages, prior_len);
            eprintln!("\n{}", "⏹ 已中断".dimmed());
            eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
            fire_session_end(
                &hook_manager,
                &disabled_hooks,
                &messages,
                session_id,
                &provider.model,
            );
            return;
        }

        // ── 同步部分：markdown 重绘 + 工具执行 ──

        // 回退清除 raw 文本，用 markdown 重绘
        if !sr.assistant_text.is_empty() {
            let total_raw_lines = if sr.cur_col > 0 {
                sr.raw_lines + 1
            } else {
                sr.raw_lines
            };
            let mut stdout = io::stdout();
            if total_raw_lines > 0 {
                let _ = execute!(stdout, cursor::MoveToColumn(0));
                if total_raw_lines > 1 {
                    let _ = execute!(stdout, cursor::MoveUp((total_raw_lines - 1) as u16));
                }
                let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
            }
            crate::util::md_render::render_md(&sr.assistant_text);
        }

        if sr.tool_items.is_empty() {
            // 无工具调用
            // ★ Stop hook：LLM 即将结束回复，hook 可阻止并注入反馈
            let mut stop_retry = false;
            {
                let hm = hook_manager.lock().unwrap();
                if hm.has_hooks_for(HookEvent::Stop) {
                    let stop_ctx = HookContext {
                        event: HookEvent::Stop,
                        messages: Some(messages.clone()),
                        system_prompt: system_prompt.clone(),
                        model: Some(provider.model.clone()),
                        user_input: Some(sr.assistant_text.clone()),
                        session_id: Some(session_id.to_string()),
                        ..Default::default()
                    };
                    if let Some(result) = hm.execute(HookEvent::Stop, stop_ctx, &disabled_hooks)
                        && let Some(ref feedback) = result.retry_feedback
                    {
                        write_info_log("Stop hook", &format!("纠查官反馈: {}", feedback));
                        let feedback_msg = ChatMessage::text(MessageRole::User, feedback.clone());
                        messages.push(feedback_msg);
                        stop_retry = true;
                    }
                }
            }
            if stop_retry {
                continue; // 带反馈继续下一轮
            }

            // 正常结束：持久化本轮新增消息并打印会话 ID
            persist_messages(session_id, &messages, prior_len);
            eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
            fire_session_end(
                &hook_manager,
                &disabled_hooks,
                &messages,
                session_id,
                &provider.model,
            );
            return;
        }

        // 添加 assistant 消息（含 tool_calls）
        messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: sr.assistant_text,
            tool_calls: Some(sr.tool_items.clone()),
            tool_call_id: None,
            images: None,
            reasoning_content: if sr.assistant_reasoning.is_empty() {
                None
            } else {
                Some(sr.assistant_reasoning)
            },
        });

        // 逐个执行工具（同步上下文，reqwest::blocking 安全）
        for item in &sr.tool_items {
            if jcli_config.is_denied(&item.name, &item.arguments) {
                println!(
                    "{} {} {}",
                    "⛔".red(),
                    item.name.red().bold(),
                    "被权限规则拒绝".red()
                );

                // ★ PostToolExecutionFailure hook
                fire_post_tool_failure(
                    &hook_manager,
                    &disabled_hooks,
                    &item.name,
                    &item.arguments,
                    "工具调用被拒绝（deny 规则匹配）",
                    session_id,
                );

                messages.push(ChatMessage {
                    role: MessageRole::Tool,
                    content: "工具调用被拒绝（deny 规则匹配）".to_string(),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                    reasoning_content: None,
                });
                continue;
            }

            let confirm_msg = tool_registry
                .get(&item.name)
                .map(|t| t.confirmation_message(&item.arguments))
                .unwrap_or_else(|| format!("调用工具 {} 参数: {}", item.name, item.arguments));

            let needs_confirm = tool_registry
                .get(&item.name)
                .map(|t| t.requires_confirmation())
                .unwrap_or(false)
                && !jcli_config.is_allowed(&item.name, &item.arguments);

            // ★ PreToolExecution hook（可修改参数或跳过）
            let mut effective_args = item.arguments.clone();
            let mut skip_tool = false;
            {
                let hm = hook_manager.lock().unwrap();
                if hm.has_hooks_for(HookEvent::PreToolExecution) {
                    let ctx = HookContext {
                        event: HookEvent::PreToolExecution,
                        tool_name: Some(item.name.clone()),
                        tool_arguments: Some(item.arguments.clone()),
                        session_id: Some(session_id.to_string()),
                        ..Default::default()
                    };
                    if let Some(result) =
                        hm.execute(HookEvent::PreToolExecution, ctx, &disabled_hooks)
                    {
                        if result.is_skip() {
                            skip_tool = true;
                        }
                        if result.is_stop() {
                            skip_tool = true;
                        }
                        if let Some(new_args) = result.tool_arguments {
                            effective_args = new_args;
                        }
                    }
                }
            }

            if skip_tool {
                println!(
                    "{} {} {}",
                    "⏭".dimmed(),
                    item.name.dimmed(),
                    "被 hook 跳过".dimmed()
                );
                messages.push(ChatMessage {
                    role: MessageRole::Tool,
                    content: "工具调用被 hook 跳过".to_string(),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                    reasoning_content: None,
                });
                continue;
            }

            if needs_confirm && !bypass {
                let tool_desc = format!("{}  {}", "🔧", confirm_msg.yellow());
                let allow_rule = generate_allow_rule(&item.name, &effective_args);
                let options = ["允许执行", "拒绝", &format!("始终允许 ({})", allow_rule)];
                let choice = interactive_confirm(&tool_desc, &options, 0);
                match choice {
                    Some(0) => {}
                    Some(2) => {
                        jcli_config.add_allow_rule(&allow_rule);
                    }
                    _ => {
                        println!("{} {}", "⏭".dimmed(), "已跳过".dimmed());
                        messages.push(ChatMessage {
                            role: MessageRole::Tool,
                            content: "用户拒绝执行该工具".to_string(),
                            tool_calls: None,
                            tool_call_id: Some(item.id.clone()),
                            images: None,
                            reasoning_content: None,
                        });
                        continue;
                    }
                }
            }

            println!("🔧 {} ...", confirm_msg.cyan());
            let result = tool_registry.execute(&item.name, &effective_args, &cancelled);
            if result.is_error {
                println!("{} {}", " ✖ ".red(), "执行出错".red());

                // ★ PostToolExecutionFailure hook
                fire_post_tool_failure(
                    &hook_manager,
                    &disabled_hooks,
                    &item.name,
                    &effective_args,
                    &result.output,
                    session_id,
                );
            } else {
                println!("{} {}", " ✔ ".green(), "完成".green());

                // ★ PostToolExecution hook
                {
                    let hm = hook_manager.lock().unwrap();
                    if hm.has_hooks_for(HookEvent::PostToolExecution) {
                        let ctx = HookContext {
                            event: HookEvent::PostToolExecution,
                            tool_name: Some(item.name.clone()),
                            tool_arguments: Some(effective_args.clone()),
                            tool_result: Some(result.output.clone()),
                            session_id: Some(session_id.to_string()),
                            ..Default::default()
                        };
                        if let Some(hook_result) =
                            hm.execute(HookEvent::PostToolExecution, ctx, &disabled_hooks)
                            && let Some(system_msg) = hook_result.system_message
                        {
                            eprintln!("{}", system_msg.dimmed());
                        }
                    }
                }
            }

            messages.push(ChatMessage {
                role: MessageRole::Tool,
                content: result.output,
                tool_calls: None,
                tool_call_id: Some(item.id.clone()),
                images: None,
                reasoning_content: None,
            });
        }

        // 继续下一轮
    }

    // 达到最大轮数：持久化已有消息并提示
    persist_messages(session_id, &messages, prior_len);
    eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
    eprintln!("\n⚠️ 达到最大工具调用轮数 ({})", max_rounds);
    fire_session_end(
        &hook_manager,
        &disabled_hooks,
        &messages,
        session_id,
        &provider.model,
    );
}

/// 触发 SessionEnd hook
fn fire_session_end(
    hook_manager: &Arc<std::sync::Mutex<HookManager>>,
    disabled_hooks: &[String],
    messages: &[ChatMessage],
    session_id: &str,
    model: &str,
) {
    let hm = hook_manager.lock().unwrap();
    if hm.has_hooks_for(HookEvent::SessionEnd) {
        let ctx = HookContext {
            event: HookEvent::SessionEnd,
            messages: Some(messages.to_vec()),
            model: Some(model.to_string()),
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };
        hm.execute(HookEvent::SessionEnd, ctx, disabled_hooks);
    }
}

/// 触发 PostToolExecutionFailure hook
fn fire_post_tool_failure(
    hook_manager: &Arc<std::sync::Mutex<HookManager>>,
    disabled_hooks: &[String],
    tool_name: &str,
    tool_arguments: &str,
    error_msg: &str,
    session_id: &str,
) {
    let hm = hook_manager.lock().unwrap();
    if hm.has_hooks_for(HookEvent::PostToolExecutionFailure) {
        let ctx = HookContext {
            event: HookEvent::PostToolExecutionFailure,
            tool_name: Some(tool_name.to_string()),
            tool_arguments: Some(tool_arguments.to_string()),
            tool_error: Some(error_msg.to_string()),
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };
        hm.execute(HookEvent::PostToolExecutionFailure, ctx, disabled_hooks);
    }
}

fn resolve_oneshot_system_prompt(
    tool_registry: &ToolRegistry,
    disabled_tools: &[String],
    background_manager: &Arc<BackgroundManager>,
    task_manager: &Arc<TaskManager>,
) -> Option<String> {
    let template = load_system_prompt()?;
    let tools_summary = tool_registry.build_tools_summary(disabled_tools);
    let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
    let memory_text = load_memory().unwrap_or_default();
    let soul_text = load_soul().unwrap_or_default();
    let agent_md_text = load_agent_md();
    let current_dir = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let skill_dir = skills_dir().to_string_lossy().to_string();
    let project_skill_dir = project_skills_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let session_state_summary = tool_registry.build_session_state_summary();
    let tasks_summary = build_tasks_summary(task_manager);
    let background_summary = build_running_summary(background_manager);
    // 加载 skills 摘要（与 TUI 对齐）
    let loaded_skills = skill::load_all_skills();
    let skills_summary = skill::build_skills_summary(&loaded_skills, &[]);
    let resolved = template
        .replace("{{.current_dir}}", &current_dir)
        .replace("{{.skills}}", &skills_summary)
        .replace("{{.skill_dir}}", &skill_dir)
        .replace("{{.project_skill_dir}}", &project_skill_dir)
        .replace("{{.tools}}", &tools_summary)
        .replace("{{.style}}", &style_text)
        .replace("{{.memory}}", &memory_text)
        .replace("{{.soul}}", &soul_text)
        .replace("{{.agent_md}}", &agent_md_text)
        .replace("{{.session_state}}", &session_state_summary)
        .replace("{{.tasks}}", &tasks_summary)
        .replace("{{.background_tasks}}", &background_summary)
        .replace("{{.teammates}}", "");
    Some(resolved)
}
