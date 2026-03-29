pub mod agent;
pub mod api;
pub mod app;
pub mod archive;
pub mod autocomplete;
pub mod compact;
pub mod constants;
pub mod handler;
pub mod hook;
pub mod input_thread;
pub mod markdown;
pub mod permission;
pub mod remote;
pub mod render_cache;
pub mod sandbox;
pub mod skill;
pub mod storage;
pub mod theme;
pub mod tools;
pub mod ui;
pub mod ui_helpers;

use crate::config::YamlConfig;
use crate::{error, info};
use handler::run_chat_tui;
use std::io::{self, Write};
use storage::{
    ChatMessage, SessionEvent, append_session_event, load_agent_config, load_session,
    load_system_prompt,
};

/// 生成 oneshot 会话 ID（时间戳微秒 + 进程 ID）
fn generate_oneshot_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let pid = std::process::id();
    format!("{:x}-{:x}", ts, pid)
}

/// 将消息列表持久化到 session 文件（仅保存从 start_idx 开始的新消息）
fn persist_messages(session_id: &str, messages: &[ChatMessage], start_idx: usize) {
    for msg in messages.iter().skip(start_idx) {
        append_session_event(session_id, &SessionEvent::Msg(msg.clone()));
    }
}

pub fn handle_chat(
    content: &[String],
    cont: bool,
    session_id_opt: Option<&str>,
    remote: bool,
    port: u16,
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
        storage::find_latest_session_id().unwrap_or_else(generate_oneshot_session_id)
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
        );
    } else {
        // 无工具模式：流式输出 + 结束后 markdown 重绘
        use crossterm::{cursor, execute, terminal};

        let user_msg = ChatMessage::text("user", message.clone());
        let mut messages = prior_messages.clone();
        messages.push(user_msg.clone());

        let term_width = crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80);
        let mut cur_col: usize = 0;
        let mut raw_lines: usize = 0;

        // 发送给 API 时截取历史窗口
        let history_limit = agent_config.max_history_messages;
        let send_messages = if messages.len() > history_limit {
            messages[messages.len() - history_limit..].to_vec()
        } else {
            messages.clone()
        };

        match api::call_openai_stream(
            provider,
            &send_messages,
            load_system_prompt().as_deref(),
            &mut |chunk| {
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
                        &[ChatMessage::text("assistant", &full_text)],
                        0,
                    );
                    use colored::Colorize;
                    eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
                }
            }
            Err(e) => {
                error!("\n✖️ {}", e);
            }
        }
    }
}

/// 一句话模式的 agent 循环：支持工具调用
fn run_oneshot_agent(
    provider: &storage::ModelProvider,
    agent_config: &storage::AgentConfig,
    message: String,
    prior_messages: Vec<ChatMessage>,
    session_id: &str,
) {
    use api::{build_request_with_tools, create_openai_client};
    use colored::Colorize;
    use crossterm::event::{self, Event, KeyCode};
    use crossterm::{cursor, execute, terminal};
    use futures::StreamExt;
    use permission::JcliConfig;
    use std::sync::{Arc, Mutex, atomic::AtomicBool};
    use storage::ToolCallItem;
    use tools::background::BackgroundManager;
    use tools::task::TaskManager;

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("创建异步运行时失败: {}", e);
            return;
        }
    };

    // 构建工具注册表
    let (ask_tx, ask_rx) = std::sync::mpsc::channel::<crate::command::chat::app::AskRequest>();
    let background_manager = Arc::new(BackgroundManager::new());
    let task_manager = Arc::new(TaskManager::new());
    let hook_manager = Arc::new(Mutex::new(hook::HookManager::default()));
    let tool_registry = tools::ToolRegistry::new(
        vec![],
        ask_tx,
        background_manager,
        task_manager,
        hook_manager,
    );

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
                    answers.insert(q.question.clone(), serde_json::Value::String(answer));
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
                    answers.insert(q.question.clone(), serde_json::Value::String(answer));
                }
            }

            let response = serde_json::to_string(&serde_json::json!({ "answers": answers }))
                .unwrap_or_default();
            let _ = req.response_tx.send(response);
        }
    });

    let openai_tools = tool_registry.to_openai_tools_filtered(&agent_config.disabled_tools);
    let system_prompt = resolve_oneshot_system_prompt(&tool_registry, &agent_config.disabled_tools);
    let mut jcli_config = JcliConfig::load();
    let cancelled = Arc::new(AtomicBool::new(false));
    let max_rounds = agent_config.max_tool_rounds;

    // 构建初始消息列表：先写入历史，再追加本次用户消息
    let user_msg = ChatMessage::text("user", message);
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

    let client = create_openai_client(provider);

    // 流式调用结果
    struct StreamResult {
        assistant_text: String,
        tool_items: Vec<ToolCallItem>,
        raw_lines: usize,
        cur_col: usize,
    }

    for _round in 0..max_rounds {
        // ── 异步部分：API 流式调用 ──
        let stream_result: Result<StreamResult, String> = rt.block_on(async {
            let request = build_request_with_tools(
                provider,
                &messages,
                openai_tools.clone(),
                system_prompt.as_deref(),
            )?;

            let mut stream = client
                .chat()
                .create_stream(request)
                .await
                .map_err(|e| format!("API 请求失败: {}", e))?;

            let mut assistant_text = String::new();
            let mut raw_tool_calls: std::collections::BTreeMap<u32, (String, String, String)> =
                std::collections::BTreeMap::new();
            let mut finish_reason: Option<async_openai::types::chat::FinishReason> = None;
            let term_width = crossterm::terminal::size()
                .map(|(w, _)| w as usize)
                .unwrap_or(80);
            let mut cur_col: usize = 0;
            let mut raw_lines: usize = 0;

            while let Some(result) = stream.next().await {
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
                                finish_reason = Some(*fr);
                            }
                        }
                    }
                    Err(e) => return Err(format!("流式响应错误: {}", e)),
                }
            }

            let is_tool_calls = matches!(
                finish_reason,
                Some(async_openai::types::chat::FinishReason::ToolCalls)
            );
            let tool_items: Vec<ToolCallItem> = if is_tool_calls {
                raw_tool_calls
                    .into_values()
                    .map(|(id, name, arguments)| ToolCallItem {
                        id,
                        name,
                        arguments,
                    })
                    .collect()
            } else {
                vec![]
            };

            Ok(StreamResult {
                assistant_text,
                tool_items,
                raw_lines,
                cur_col,
            })
        });

        let sr = match stream_result {
            Ok(sr) => sr,
            Err(e) => {
                error!("\n{}", e);
                return;
            }
        };

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
            // 无工具调用，正常结束：持久化本轮新增消息并打印会话 ID
            persist_messages(session_id, &messages, prior_len);
            eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
            return;
        }

        // 添加 assistant 消息（含 tool_calls）
        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: sr.assistant_text,
            tool_calls: Some(sr.tool_items.clone()),
            tool_call_id: None,
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
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: "工具调用被拒绝（deny 规则匹配）".to_string(),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
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

            if needs_confirm {
                let tool_desc = format!("{}  {}", "🔧", confirm_msg.yellow());
                let allow_rule = permission::generate_allow_rule(&item.name, &item.arguments);
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
                            role: "tool".to_string(),
                            content: "用户拒绝执行该工具".to_string(),
                            tool_calls: None,
                            tool_call_id: Some(item.id.clone()),
                        });
                        continue;
                    }
                }
            }

            println!("🔧 {} ...", confirm_msg.cyan());
            let result = tool_registry.execute(&item.name, &item.arguments, &cancelled);
            if result.is_error {
                println!("{} {}", " ✖ ".red(), "执行出错".red());
            } else {
                println!("{} {}", " ✔ ".green(), "完成".green());
            }

            messages.push(ChatMessage {
                role: "tool".to_string(),
                content: result.output,
                tool_calls: None,
                tool_call_id: Some(item.id.clone()),
            });
        }

        // 继续下一轮
    }

    // 达到最大轮数：持久化已有消息并提示
    persist_messages(session_id, &messages, prior_len);
    eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
    eprintln!("\n⚠️ 达到最大工具调用轮数 ({})", max_rounds);
}

/// 为一句话模式解析系统提示词模板
fn resolve_oneshot_system_prompt(
    tool_registry: &tools::ToolRegistry,
    disabled_tools: &[String],
) -> Option<String> {
    use storage::{load_memory, load_soul, load_style};

    let template = load_system_prompt()?;
    let tools_summary = tool_registry.build_tools_summary(disabled_tools);
    let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
    let memory_text = load_memory().unwrap_or_default();
    let soul_text = load_soul().unwrap_or_default();
    let current_dir = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let skill_dir = skill::skills_dir().to_string_lossy().to_string();
    let resolved = template
        .replace("{{.current_dir}}", &current_dir)
        .replace("{{.skills}}", "")
        .replace("{{.skill_dir}}", &skill_dir)
        .replace("{{.tools}}", &tools_summary)
        .replace("{{.style}}", &style_text)
        .replace("{{.memory}}", &memory_text)
        .replace("{{.soul}}", &soul_text);
    Some(resolved)
}
