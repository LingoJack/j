use crate::command::chat::app::{Action, ChatApp};
use crate::command::chat::infra::hook::{HookContext, HookEvent};
use crate::command::chat::storage::ChatMessage;
use crate::command::chat::storage::agent_data_dir;
use crate::util::safe_lock;

/// 将当前 session 的 system prompt + messages 导出到
/// ~/.jdata/agent/dumps/<subdir>_<ts>/ 目录，分文件存放。
/// 同时导出所有当前注册的 Teammate 和运行中子 Agent 的数据。
///
/// `processed=false`（`/dump`）：导出原始 session messages，不经过任何处理管线。
/// `processed=true`（`/dump-processed`）：对 messages 应用与 agent loop 一致的处理管线：
/// rolling window → micro_compact → PreLlmRequest hooks → sanitize，
/// 产出与最终发给 LLM 一致的数据。
pub(super) fn dump_current_request(app: &mut ChatApp, processed: bool) {
    let mut system_prompt = app.build_current_system_prompt();
    let mut messages = if processed {
        app.build_api_messages()
    } else {
        safe_lock(&app.context_messages, "dump::ctx_msgs").clone()
    };

    if processed {
        apply_processed_pipeline(app, &mut system_prompt, &mut messages);
    }

    let timestamp = chrono::Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
    let subdir = if processed { "processed" } else { "dump" };
    let dump_dir = agent_data_dir()
        .join("dumps")
        .join(format!("{}_{}", subdir, timestamp));
    if let Err(e) = std::fs::create_dir_all(&dump_dir) {
        app.update(Action::ShowToast(
            format!("创建 dump 目录失败: {}", e),
            true,
        ));
        return;
    }

    if processed {
        write_pipeline_info(&dump_dir);
    }

    if let Err(e) = write_agent_dump(&dump_dir, system_prompt.as_deref(), &messages) {
        app.update(Action::ShowToast(e, true));
        return;
    }

    let teammate_count = dump_teammates(app, &dump_dir);
    let sub_agent_count = dump_sub_agents(app, &dump_dir);

    let mut toast = if processed {
        format!("已导出 processed 数据到 {}", dump_dir.display())
    } else {
        format!("已导出到 {}", dump_dir.display())
    };
    if teammate_count > 0 || sub_agent_count > 0 {
        toast.push_str(&format!(
            "（含 {} 个 teammate, {} 个 sub-agent）",
            teammate_count, sub_agent_count
        ));
    }
    app.update(Action::ShowToast(toast, false));
}

/// 对 messages 应用与 agent loop 一致的处理管线（micro_compact → hooks → sanitize）
fn apply_processed_pipeline(
    app: &ChatApp,
    system_prompt: &mut Option<String>,
    messages: &mut Vec<ChatMessage>,
) {
    {
        use crate::command::chat::context::compact;
        let compact_config = &app.state.agent_config.compact;
        if compact_config.enabled {
            compact::micro_compact(
                messages,
                compact_config.keep_recent,
                &compact_config.micro_compact_exempt_tools,
            );
        }
    }
    {
        let hook_manager = app.hook_manager.lock();
        if let Ok(mgr) = hook_manager
            && mgr.has_hooks_for(HookEvent::PreLlmRequest)
        {
            let model_name = app.active_model_name();
            let ctx = HookContext {
                event: HookEvent::PreLlmRequest,
                messages: Some(messages.clone()),
                system_prompt: system_prompt.clone(),
                model: Some(model_name),
                session_id: Some(app.session_id.clone()),
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            if let Some(result) = mgr.execute(
                HookEvent::PreLlmRequest,
                ctx,
                &app.state.agent_config.disabled_hooks,
            ) {
                if let Some(new_msgs) = result.messages {
                    *messages = new_msgs;
                }
                if let Some(new_prompt) = result.system_prompt {
                    *system_prompt = Some(new_prompt);
                }
                if let Some(inject) = result.inject_messages {
                    messages.extend(inject);
                }
            }
        }
    }
    {
        use crate::command::chat::agent::api::sanitize_messages;
        *messages = sanitize_messages(messages);
    }
}

fn write_pipeline_info(dump_dir: &std::path::Path) {
    let pipeline_info = "处理管线: window.select_messages (三阶段: 时间保底 → 豁免保底 → 比例配额+溢出) → micro_compact → PreLlmRequest hooks → sanitize_messages\n\
         - window: 最近 keep_recent*2 个 unit 无条件保留；含豁免工具 (LoadSkill/Task/Todo/Ask/...) 的 ToolGroup 优先保留；剩余预算按 User:AsstText:ToolGroup = 35:25:40 分配，tier 间有溢出。\n\
         - micro_compact: 将旧 tool result (>800 chars) 替换为 [Previous: used X]，最近 keep_recent 个保留原样，豁免工具不压缩。\n\
         - PreLlmRequest hooks 已执行（含内置 hooks: tasks_status, background_status, session_state, teammates_status, todo_nag, broadcast_compress）。\n\
         - broadcast_compress (末位内置 hook): 折叠来自 SubAgent/Teammate 的 tool call 广播消息 (<Name> [调用工具 X])，保留最近 N 条，较早的合并为 [早期工具调用摘要]。\n\
         - sanitize_messages: 移除孤立的 tool_call / tool result。\n\
         注意: auto_compact (LLM 摘要) 需要 API 调用，未在此处执行。\n";
    let _ = std::fs::write(dump_dir.join("pipeline.txt"), pipeline_info);
}

/// 写入单个 agent 的 system_prompt.md + messages.json 到指定目录
fn write_agent_dump(
    dir: &std::path::Path,
    system_prompt: Option<&str>,
    messages: &[ChatMessage],
) -> Result<(), String> {
    let sp_content = system_prompt
        .map(|s| s.to_string())
        .unwrap_or_else(|| "（未设置 system prompt）".to_string());
    std::fs::write(dir.join("system_prompt.md"), sp_content)
        .map_err(|e| format!("写入 {}/system_prompt.md 失败: {}", dir.display(), e))?;

    let msgs_content = serde_json::to_string_pretty(messages)
        .map_err(|e| format!("序列化 messages 失败: {}", e))?;
    std::fs::write(dir.join("messages.json"), msgs_content)
        .map_err(|e| format!("写入 {}/messages.json 失败: {}", dir.display(), e))?;
    Ok(())
}

/// 将所有 teammate 写入 dump_dir/teammates/<name>/，返回成功导出的数量
fn dump_teammates(app: &ChatApp, dump_dir: &std::path::Path) -> usize {
    let manager = match app.teammate_manager.lock() {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if manager.teammates.is_empty() {
        return 0;
    }
    let teammates_root = dump_dir.join("teammates");
    if std::fs::create_dir_all(&teammates_root).is_err() {
        return 0;
    }

    let mut count = 0;
    for (name, handle) in &manager.teammates {
        let safe_name = sanitize_dir_name(name);
        let tm_dir = teammates_root.join(&safe_name);
        if std::fs::create_dir_all(&tm_dir).is_err() {
            continue;
        }
        let sp_snapshot = handle
            .system_prompt_snapshot
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let msgs_snapshot = handle
            .messages_snapshot
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default();
        if write_agent_dump(&tm_dir, Some(&sp_snapshot), &msgs_snapshot).is_ok() {
            count += 1;
        }
    }
    count
}

/// 将正在运行的子 Agent 写入 dump_dir/subagents/<id>/，返回成功导出的数量
fn dump_sub_agents(app: &ChatApp, dump_dir: &std::path::Path) -> usize {
    let snapshots = app.sub_agent_tracker.snapshot_running();
    if snapshots.is_empty() {
        return 0;
    }
    let sub_root = dump_dir.join("subagents");
    if std::fs::create_dir_all(&sub_root).is_err() {
        return 0;
    }
    let mut count = 0;
    for (id, description, mode, sp, msgs) in snapshots {
        let safe_id = sanitize_dir_name(&id);
        let agent_dir = sub_root.join(&safe_id);
        if std::fs::create_dir_all(&agent_dir).is_err() {
            continue;
        }
        let meta = format!("id: {}\nmode: {}\ndescription: {}\n", id, mode, description);
        if std::fs::write(agent_dir.join("meta.txt"), meta).is_err() {
            continue;
        }
        if write_agent_dump(&agent_dir, Some(&sp), &msgs).is_ok() {
            count += 1;
        }
    }
    count
}

/// 将不适合做目录名的字符替换为 `_`
fn sanitize_dir_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
