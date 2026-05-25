//! WebSocket 远程消息处理

use crate::command::chat::app::{Action, ChatApp, ChatMode, ConfigTab};
use crate::command::chat::remote::protocol::{WsInbound, WsOutbound};

/// 处理一批 WebSocket 入站消息。
///
/// 返回 true 表示需要重绘。
pub(super) fn handle_ws_inbound(app: &mut ChatApp, msgs: Vec<WsInbound>) -> bool {
    let mut needs_redraw = false;

    for msg in msgs {
        needs_redraw = true;
        match msg {
            WsInbound::SendMessage { content } => {
                app.inject_remote_message(&content);
            }
            WsInbound::ToolConfirm { action, reason } => match action.as_str() {
                "allow" => app.update(Action::ExecutePendingTool),
                "allow_always" => app.update(Action::AllowAndExecutePendingTool),
                "reject_with_reason" => {
                    let r = reason.unwrap_or_default();
                    app.update(Action::RejectPendingToolWithReason(r));
                }
                _ => app.update(Action::RejectPendingTool),
            },
            WsInbound::AskResponse { answers } => {
                if app.ui.tool_ask_mode {
                    // 将远程回答直接构建为 JSON 响应发送给 Ask 工具
                    let response = serde_json::json!({ "answers": answers }).to_string();
                    if let Some(tx) = app.ask_response_tx.take() {
                        let _ = tx.send(response);
                    }
                    // 清理 ask 状态
                    app.ui.tool_ask_mode = false;
                    app.ui.tool_ask_questions.clear();
                    app.ui.tool_ask_current_idx = 0;
                    app.ui.tool_ask_answers.clear();
                    app.ui.tool_ask_selections.clear();
                    app.ui.tool_ask_cursor = 0;
                    if !app.tool_executor.has_pending_confirm() {
                        app.ui.mode = ChatMode::Chat;
                    }
                    app.broadcast_ws(WsOutbound::Status {
                        state: "loading".to_string(),
                    });
                }
            }
            WsInbound::Cancel => {
                app.update(Action::CancelStream);
            }
            WsInbound::Sync => {
                let sync = app.build_sync_outbound();
                app.broadcast_ws(sync);
            }
            WsInbound::Ping => {
                app.broadcast_ws(WsOutbound::Pong);
            }
            WsInbound::ListSessions => {
                app.update(Action::ListSessions);
            }
            WsInbound::SwitchSession { session_id } => {
                app.update(Action::SwitchSession { session_id });
            }
            WsInbound::NewSession => {
                app.update(Action::NewSession);
            }
            // KeyExchange 在 server.rs 层处理，不会到达 TUI 层
            WsInbound::KeyExchange { .. } => {}
            WsInbound::SelectModel { index } => {
                app.ui.model_list_state.select(Some(index));
                app.update(Action::ModelSelectConfirm);
            }
            WsInbound::SelectTheme { index } => {
                app.ui.theme_list_state.select(Some(index));
                app.update(Action::ThemeSelectConfirm);
            }
            WsInbound::RequestConfig { tab } => {
                let config_tab = match tab.as_str() {
                    "session" => ConfigTab::Session,
                    "global" => ConfigTab::Global,
                    "tools" => ConfigTab::Tools,
                    "skills" => ConfigTab::Skills,
                    "hooks" => ConfigTab::Hooks,
                    "commands" => ConfigTab::Commands,
                    "teammates" => ConfigTab::Teammates,
                    "archive" => ConfigTab::Archive,
                    _ => ConfigTab::Model,
                };
                app.update(Action::ConfigSwitchTabTo(config_tab));
                app.broadcast_config_state();
            }
            WsInbound::ConfigEditSubmit { value } => {
                // 直接写入当前编辑字段并提交
                app.ui.config_edit_buf = value.clone();
                app.ui.config_edit_cursor = value.chars().count();
                app.update(Action::ConfigEditSubmit);
                app.broadcast_config_state();
            }
            WsInbound::ConfigToggle { index } => {
                // 远程 toggle：根据当前 tab 直接操作配置
                match app.ui.config_tab {
                    ConfigTab::Tools => {
                        let all_tools: Vec<String> = app
                            .tool_registry
                            .tool_names()
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect();
                        if let Some(name) = all_tools.get(index) {
                            if app.state.agent_config.disabled_tools.contains(name) {
                                app.state.agent_config.disabled_tools.retain(|n| n != name);
                            } else {
                                app.state.agent_config.disabled_tools.push(name.clone());
                            }
                            let _ = crate::command::chat::storage::save_agent_config(
                                &app.state.agent_config,
                            );
                        }
                        app.broadcast_config_state();
                    }
                    ConfigTab::Skills => {
                        let names: Vec<String> = app
                            .state
                            .loaded_skills
                            .iter()
                            .map(|s| s.frontmatter.name.clone())
                            .collect();
                        if let Some(name) = names.get(index) {
                            if app.state.agent_config.disabled_skills.contains(name) {
                                app.state.agent_config.disabled_skills.retain(|n| n != name);
                            } else {
                                app.state.agent_config.disabled_skills.push(name.clone());
                            }
                            let _ = crate::command::chat::storage::save_agent_config(
                                &app.state.agent_config,
                            );
                        }
                        app.broadcast_config_state();
                    }
                    ConfigTab::Global => {
                        // 远程切换全局布尔设置
                        let fields = ["tools_enabled", "auto_restore_session", "flat_bubble"];
                        if index < fields.len() {
                            match fields[index] {
                                "tools_enabled" => {
                                    app.state.agent_config.tools_enabled =
                                        !app.state.agent_config.tools_enabled
                                }
                                "auto_restore_session" => {
                                    app.state.agent_config.auto_restore_session =
                                        !app.state.agent_config.auto_restore_session
                                }
                                "flat_bubble" => {
                                    app.state.agent_config.flat_bubble =
                                        !app.state.agent_config.flat_bubble
                                }
                                _ => {}
                            }
                            let _ = crate::command::chat::storage::save_agent_config(
                                &app.state.agent_config,
                            );
                        }
                        app.broadcast_config_state();
                    }
                    _ => {}
                }
            }
            WsInbound::StartArchive => {
                app.start_archive_confirm();
                app.broadcast_archive_confirm_state();
            }
            WsInbound::ArchiveWithDefault => {
                app.do_archive(&app.ui.archive_default_name.clone());
                let sync = app.build_sync_outbound();
                app.broadcast_ws(sync);
            }
            WsInbound::ArchiveWithCustom { name } => {
                app.do_archive(&name);
                let sync = app.build_sync_outbound();
                app.broadcast_ws(sync);
            }
            WsInbound::ClearSession => {
                app.clear_session();
                let sync = app.build_sync_outbound();
                app.broadcast_ws(sync);
            }
            WsInbound::StartArchiveList => {
                app.start_archive_list();
                app.broadcast_archive_list_state();
            }
            WsInbound::RestoreArchive { index } => {
                app.ui.archive_list_index = index;
                app.do_restore();
                let sync = app.build_sync_outbound();
                app.broadcast_ws(sync);
            }
            WsInbound::DeleteArchive { index } => {
                app.ui.archive_list_index = index;
                app.do_delete_archive();
                app.broadcast_archive_list_state();
            }
            WsInbound::DeleteSession { index } => {
                if index < app.ui.session_list.len() {
                    app.ui.session_list_index = index;
                    app.update(Action::DeleteSession);
                    app.broadcast_session_list_state();
                }
            }
            WsInbound::AgentPermConfirm { approve } => {
                if let Some(req) = app.ui.pending_agent_perm.take() {
                    req.resolve(approve);
                }
                app.ui.mode = ChatMode::Chat;
                app.ui.msg_lines_cache = None;
            }
            WsInbound::PlanApproval { approve, content } => {
                use crate::command::chat::app::types::PlanDecision;
                if let Some(req) = app.ui.pending_plan_approval.take() {
                    let decision = if approve {
                        match content.as_deref() {
                            Some("clear") => PlanDecision::ApproveAndClearContext,
                            _ => PlanDecision::Approve,
                        }
                    } else {
                        PlanDecision::Reject
                    };
                    req.resolve(decision);
                }
                app.ui.mode = ChatMode::Chat;
                app.ui.msg_lines_cache = None;
            }
            WsInbound::ToggleAutoApprove => {
                app.update(Action::ToggleAutoApprove);
            }
            // ── 文件操作 ──
            WsInbound::FileList { path } => {
                let entries = ChatApp::handle_file_list(&path);
                app.broadcast_ws(WsOutbound::FileListResult { path, entries });
            }
            WsInbound::FileRead { path } => {
                let (content, error) = ChatApp::handle_file_read(&path);
                app.broadcast_ws(WsOutbound::FileReadResult {
                    path,
                    content,
                    error,
                });
            }
            WsInbound::FileWrite { path, content } => {
                let (success, error) = ChatApp::handle_file_write(&path, &content);
                app.broadcast_ws(WsOutbound::FileWriteResult {
                    path,
                    success,
                    error,
                });
            }
            // ── 终端操作 ──
            WsInbound::TerminalExec { command } => {
                let (output, exit_code) = ChatApp::handle_terminal_exec(&command);
                app.broadcast_ws(WsOutbound::TerminalOutput { output, exit_code });
            }
            WsInbound::TerminalInterrupt => {
                // 终端中断暂不实现（需要进程管理）
            }
        }
    }

    needs_redraw
}
