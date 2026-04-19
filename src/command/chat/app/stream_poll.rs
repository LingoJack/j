use super::action::Action;
use super::chat_app::ChatApp;
use super::types::{
    AskAnswer, AskRequest, PlanDecision, StreamMsg, ToolCallStatus, ToolExecStatus, ToolResultMsg,
};
use super::ui_state::ChatMode;
use crate::command::chat::constants::ROLE_ASSISTANT;
use crate::command::chat::infra::hook::{HookContext, HookEvent};
use crate::command::chat::remote::protocol::{AskOptionInfo, AskQuestionInfo, WsOutbound};
use crate::command::chat::storage::ChatMessage;
use crate::util::log::write_info_log;
use crate::util::safe_lock;

impl ChatApp {
    /// 处理后台流式消息（在主循环中每帧调用）
    /// 返回需要通过 update() 分发的 Actions 列表
    pub fn poll_stream_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();

        // ★ 从共享消息列表中检测新消息，增量追加到 session.messages
        {
            let shared = safe_lock(&self.ui_messages, "poll::shared_msgs");
            let new_count = shared.len();
            if new_count > self.ui_messages_read_offset {
                for msg in &shared[self.ui_messages_read_offset..] {
                    self.state.session.messages.push(msg.clone());
                }
                self.ui_messages_read_offset = new_count;
                self.ui.msg_lines_cache = None;
                if self.ui.auto_scroll && self.state.is_loading {
                    self.ui.scroll_offset = u16::MAX;
                }
            }
        }

        if self.main_agent.is_none() {
            return actions;
        }

        // 如果在 ToolConfirm 模式，仍然需要轮询工具执行结果
        if self.ui.mode == ChatMode::ToolConfirm {
            let completed = self.tool_executor.poll_results();
            for (name, output, is_error) in completed {
                self.broadcast_ws(WsOutbound::ToolResult {
                    name,
                    output,
                    is_error,
                });
            }
            if let Some(ref rx) = self.ask_request_rx
                && let Ok(ask_req) = rx.try_recv()
            {
                self.init_ask_mode(ask_req);
                self.ui.msg_lines_cache = None;
            }
            return actions;
        }

        // 如果上一帧设置了 pending_tool_execution，本帧才真正执行
        if self.tool_executor.pending_tool_execution {
            self.tool_executor.pending_tool_execution = false;

            if self.ws_bridge.is_some() {
                for tc in &self.tool_executor.active_tool_calls {
                    self.broadcast_ws(WsOutbound::ToolCall {
                        name: tc.tool_name.clone(),
                        arguments: tc.arguments.clone(),
                    });
                }
            }

            for tc in &mut self.tool_executor.active_tool_calls {
                if let ToolExecStatus::Failed(ref msg) = tc.status
                    && let Some(ref tx) = self.tool_executor.tool_result_tx
                {
                    let final_msg = {
                        let has_hooks = self
                            .hook_manager
                            .lock()
                            .map(|m| m.has_hooks_for(HookEvent::PostToolExecutionFailure))
                            .unwrap_or(false);
                        if has_hooks {
                            let ctx = HookContext {
                                event: HookEvent::PostToolExecutionFailure,
                                tool_name: Some(tc.tool_name.clone()),
                                tool_error: Some(msg.clone()),
                                session_id: Some(self.session_id.clone()),
                                cwd: std::env::current_dir()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|_| ".".to_string()),
                                ..Default::default()
                            };
                            if let Ok(manager) = self.hook_manager.lock() {
                                if let Some(result) =
                                    manager.execute(HookEvent::PostToolExecutionFailure, ctx)
                                {
                                    result.tool_error.unwrap_or_else(|| msg.clone())
                                } else {
                                    msg.clone()
                                }
                            } else {
                                msg.clone()
                            }
                        } else {
                            msg.clone()
                        }
                    };
                    let _ = tx.send(ToolResultMsg {
                        tool_call_id: tc.tool_call_id.clone(),
                        result: final_msg,
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    });
                }
            }

            let first_confirm_idx = self
                .tool_executor
                .active_tool_calls
                .iter()
                .position(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm));

            if let Some(idx) = first_confirm_idx {
                self.tool_executor.pending_tool_idx = idx;
                self.tool_executor.tool_confirm_entered_at = std::time::Instant::now();
                self.tool_executor.execute_batch(&self.tool_registry);
                self.ui.tool_interact_selected = 0;
                self.ui.tool_interact_typing = false;
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
                self.ui.tool_ask_mode = false;
                self.ui.tool_ask_questions.clear();
                self.ui.tool_ask_current_idx = 0;
                self.ui.tool_ask_answers.clear();
                self.ui.tool_ask_selections.clear();
                self.ui.tool_ask_cursor = 0;
                actions.push(Action::EnterMode(ChatMode::ToolConfirm));
                write_info_log(
                    "poll_stream",
                    &format!(
                        "进入 ToolConfirm 模式, pending_tool_idx={}, active_tool_calls={}, tools_executing_count={}",
                        self.tool_executor.pending_tool_idx,
                        self.tool_executor.active_tool_calls.len(),
                        self.tool_executor.tools_executing_count,
                    ),
                );
            } else {
                write_info_log(
                    "poll_stream",
                    &format!(
                        "无需确认的工具, 直接执行, active_tool_calls={}",
                        self.tool_executor.active_tool_calls.len(),
                    ),
                );
                self.tool_executor.execute_batch(&self.tool_registry);
            }
            return actions;
        }

        // 轮询后台工具执行结果
        let completed = self.tool_executor.poll_results();
        for (name, output, is_error) in completed {
            self.broadcast_ws(WsOutbound::ToolResult {
                name,
                output,
                is_error,
            });
        }

        // 轮询 ask 工具请求
        if let Some(ref rx) = self.ask_request_rx
            && let Ok(ask_req) = rx.try_recv()
        {
            self.init_ask_mode(ask_req);
            actions.push(Action::EnterMode(ChatMode::ToolConfirm));
            self.ui.msg_lines_cache = None;
            return actions;
        }

        if let Some(ref agent) = self.main_agent {
            let msgs = agent.poll();
            for msg in msgs {
                match msg {
                    StreamMsg::Chunk => {
                        actions.push(Action::StreamChunk);
                    }
                    StreamMsg::ToolCallRequest(tool_calls) => {
                        self.tool_executor.active_tool_calls.clear();
                        self.tool_executor.pending_tool_idx = 0;

                        for mut tc in tool_calls {
                            // ★ PreToolExecution hook（同步，需要返回值）
                            {
                                let has_hooks = self
                                    .hook_manager
                                    .lock()
                                    .map(|m| m.has_hooks_for(HookEvent::PreToolExecution))
                                    .unwrap_or(false);
                                if has_hooks {
                                    let ctx = HookContext {
                                        event: HookEvent::PreToolExecution,
                                        tool_name: Some(tc.name.clone()),
                                        tool_arguments: Some(tc.arguments.clone()),
                                        session_id: Some(self.session_id.clone()),
                                        cwd: std::env::current_dir()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|_| ".".to_string()),
                                        ..Default::default()
                                    };
                                    if let Ok(manager) = self.hook_manager.lock()
                                        && let Some(result) =
                                            manager.execute(HookEvent::PreToolExecution, ctx)
                                    {
                                        if result.is_skip() {
                                            self.tool_executor.active_tool_calls.push(
                                                ToolCallStatus {
                                                    tool_call_id: tc.id.clone(),
                                                    tool_name: tc.name.clone(),
                                                    arguments: tc.arguments.clone(),
                                                    confirm_message: format!(
                                                        "🚫 {} 被 hook 跳过",
                                                        tc.name
                                                    ),
                                                    status: ToolExecStatus::Failed(
                                                        "该工具调用被 hook 跳过".to_string(),
                                                    ),
                                                    tool_description: extract_tool_description(
                                                        &tc.name,
                                                        &tc.arguments,
                                                    ),
                                                },
                                            );
                                            continue;
                                        }
                                        if let Some(new_args) = result.tool_arguments {
                                            tc.arguments = new_args;
                                        }
                                    }
                                }
                            }

                            if self.jcli_config.is_denied(&tc.name, &tc.arguments) {
                                self.tool_executor.active_tool_calls.push(ToolCallStatus {
                                    tool_call_id: tc.id.clone(),
                                    tool_name: tc.name.clone(),
                                    arguments: tc.arguments.clone(),
                                    confirm_message: format!(
                                        "🚫 {} 被 .jcli/ 权限配置拒绝",
                                        tc.name
                                    ),
                                    status: ToolExecStatus::Failed(
                                        "该命令被 .jcli/ 权限配置拒绝".to_string(),
                                    ),
                                    tool_description: extract_tool_description(
                                        &tc.name,
                                        &tc.arguments,
                                    ),
                                });
                                continue;
                            }

                            let sandbox_outside = self.sandbox.is_outside(&tc.name, &tc.arguments);
                            let confirm_msg = if sandbox_outside {
                                self.sandbox.outside_message(&tc.name, &tc.arguments)
                            } else if let Some(tool) = self.tool_registry.get(&tc.name) {
                                tool.confirmation_message(&tc.arguments)
                            } else {
                                format!("调用工具 {} 参数: {}", tc.name, tc.arguments)
                            };
                            let tool_needs_confirm = self
                                .tool_registry
                                .get(&tc.name)
                                .map(|t| t.requires_confirmation())
                                .unwrap_or(false);
                            let needs_confirm = (tool_needs_confirm || sandbox_outside)
                                && !self.jcli_config.is_allowed(&tc.name, &tc.arguments);
                            self.tool_executor.active_tool_calls.push(ToolCallStatus {
                                tool_call_id: tc.id.clone(),
                                tool_name: tc.name.clone(),
                                arguments: tc.arguments.clone(),
                                confirm_message: confirm_msg,
                                status: if needs_confirm {
                                    ToolExecStatus::PendingConfirm
                                } else {
                                    ToolExecStatus::Executing
                                },
                                tool_description: extract_tool_description(&tc.name, &tc.arguments),
                            });
                        }

                        self.tool_executor.pending_tool_execution = true;
                        break;
                    }
                    StreamMsg::Done => {
                        actions.push(Action::StreamDone);
                        break;
                    }
                    StreamMsg::Error(e) => {
                        actions.push(Action::StreamError(e));
                        break;
                    }
                    StreamMsg::Cancelled => {
                        actions.push(Action::StreamCancelled);
                        break;
                    }
                    StreamMsg::Retrying {
                        attempt,
                        max_attempts,
                        delay_ms,
                        error,
                    } => {
                        actions.push(Action::StreamRetrying {
                            attempt,
                            max_attempts,
                            delay_ms,
                            error,
                        });
                    }
                }
            }
        }

        actions
    }

    pub(super) fn init_ask_mode(&mut self, ask_req: AskRequest) {
        if self.ws_bridge.is_some() {
            let questions: Vec<AskQuestionInfo> = ask_req
                .questions
                .iter()
                .map(|q| AskQuestionInfo {
                    question: q.question.clone(),
                    header: q.header.clone(),
                    options: q
                        .options
                        .iter()
                        .map(|o| AskOptionInfo {
                            label: o.label.clone(),
                            description: o.description.clone(),
                        })
                        .collect(),
                    multi_select: q.multi_select,
                })
                .collect();
            self.broadcast_ws(WsOutbound::AskRequest { questions });
            self.broadcast_ws(WsOutbound::Status {
                state: "ask".to_string(),
            });
        }

        self.ui.tool_ask_mode = true;
        self.ui.tool_ask_questions = ask_req.questions;
        self.ui.tool_ask_current_idx = 0;
        self.ui.tool_ask_answers = Vec::new();
        self.ask_response_tx = Some(ask_req.response_tx);
        self.init_ask_question_state();
        self.ui.tool_interact_selected = 0;
        self.ui.tool_interact_typing = false;
        self.ui.tool_interact_input.clear();
        self.ui.tool_interact_cursor = 0;
    }

    /// 初始化当前 ask 问题的选项状态
    pub fn init_ask_question_state(&mut self) {
        if let Some(q) = self.ui.tool_ask_questions.get(self.ui.tool_ask_current_idx) {
            self.ui.tool_ask_selections = vec![false; q.options.len() + 1];
            self.ui.tool_ask_cursor = 0;

            if q.options.is_empty() {
                self.ui.tool_interact_typing = true;
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
            } else {
                self.ui.tool_interact_typing = false;
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
            }
        }
    }

    /// 提交当前问题的答案，前进到下一题或完成全部
    pub fn ask_submit_answer(&mut self, answer: AskAnswer) {
        let total = self.ui.tool_ask_questions.len();

        if self.ui.tool_ask_current_idx < self.ui.tool_ask_answers.len() {
            self.ui.tool_ask_answers[self.ui.tool_ask_current_idx] = answer;
        } else {
            self.ui.tool_ask_answers.push(answer);
        }

        if self.ui.tool_ask_current_idx + 1 < total {
            self.ui.tool_ask_current_idx += 1;
            self.init_ask_question_state();
        } else {
            let mut answers_map = serde_json::Map::new();
            for (i, q) in self.ui.tool_ask_questions.iter().enumerate() {
                if let Some(ans) = self.ui.tool_ask_answers.get(i) {
                    let val = match ans {
                        AskAnswer::Selected(indices) => {
                            let labels: Vec<&str> = indices
                                .iter()
                                .filter_map(|&idx| q.options.get(idx).map(|o| o.label.as_str()))
                                .collect();
                            labels.join(", ")
                        }
                        AskAnswer::FreeText(text) => text.clone(),
                    };
                    answers_map.insert(q.question.clone(), serde_json::Value::String(val));
                }
            }

            let response = serde_json::json!({ "answers": answers_map }).to_string();
            if let Some(tx) = self.ask_response_tx.take() {
                let _ = tx.send(response);
            }

            self.ui.tool_ask_mode = false;
            self.ui.tool_ask_questions.clear();
            self.ui.tool_ask_current_idx = 0;
            self.ui.tool_ask_answers.clear();
            self.ui.tool_ask_selections.clear();
            self.ui.tool_ask_cursor = 0;
            if !self.tool_executor.has_pending_confirm() {
                self.ui.mode = ChatMode::Chat;
            }
        }
    }

    /// 结束加载状态（流式完成或错误）
    pub(super) fn finish_loading(&mut self, had_error: bool, was_cancelled: bool) {
        if let Some(ref agent) = self.main_agent {
            agent.cancel();
        }

        self.tool_executor.tool_result_tx = None;
        self.main_agent = None;
        self.tool_executor.tools_executing_count = 0;
        self.state.is_loading = false;
        self.ui.last_rendered_streaming_len = 0;
        self.ui.msg_lines_cache = None;
        self.tool_executor.active_tool_calls.clear();

        if was_cancelled {
            let content = {
                let sc = safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content",
                );
                sc.clone()
            };
            if !content.is_empty() {
                let cancelled_content = format!("{}\n\n*[已取消]*", content);
                self.state
                    .session
                    .messages
                    .push(ChatMessage::text(ROLE_ASSISTANT, cancelled_content));
            }
            safe_lock(
                &self.state.streaming_content,
                "finish_loading::streaming_content_clear",
            )
            .clear();
            if self.ui.auto_scroll {
                self.ui.scroll_offset = u16::MAX;
            }
            self.show_toast("已取消", false);
        } else if !had_error {
            let mut content = {
                let sc = safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content_done",
                );
                sc.clone()
            };
            if !content.is_empty() {
                // ★ PostLlmResponse hook（同步，需要返回值来修改 content / abort / retry）
                let hook_result = {
                    let has_hooks = self
                        .hook_manager
                        .lock()
                        .map(|m| m.has_hooks_for(HookEvent::PostLlmResponse))
                        .unwrap_or(false);
                    if has_hooks {
                        let ctx = HookContext {
                            event: HookEvent::PostLlmResponse,
                            assistant_output: Some(content.clone()),
                            messages: Some(self.state.session.messages.clone()),
                            model: self.active_provider().map(|p| p.model.clone()),
                            session_id: Some(self.session_id.clone()),
                            cwd: std::env::current_dir()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|_| ".".to_string()),
                            ..Default::default()
                        };
                        if let Ok(manager) = self.hook_manager.lock() {
                            manager.execute(HookEvent::PostLlmResponse, ctx)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some(result) = hook_result {
                    if let Some(ref sys_msg) = result.system_message {
                        self.show_toast(sys_msg, false);
                    }
                    if result.is_stop() {
                        safe_lock(
                            &self.state.streaming_content,
                            "finish_loading::hook_aborted",
                        )
                        .clear();
                        if let Some(feedback) = result.retry_feedback {
                            self.show_toast(format!("纠查官拦截: {}", feedback), true);
                            self.send_message_internal(feedback);
                        } else {
                            self.show_toast("回复被 hook 拦截", true);
                        }
                        return;
                    }
                    if let Some(new_msg) = result.assistant_output {
                        content = new_msg;
                    }
                }

                self.state
                    .session
                    .messages
                    .push(ChatMessage::text(ROLE_ASSISTANT, content));
                safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content_done_clear",
                )
                .clear();
                self.show_toast("回复完成 ✓", false);
            }
            if self.ui.auto_scroll {
                self.ui.scroll_offset = u16::MAX;
            }
        } else {
            safe_lock(
                &self.state.streaming_content,
                "finish_loading::streaming_content_error",
            )
            .clear();
        }

        self.persist_new_messages();

        // 检查排队的任务
        let next_task = {
            let mut tasks = safe_lock(&self.state.queued_tasks, "finish_loading::queued_tasks");
            if !tasks.is_empty() {
                Some(tasks.remove(0))
            } else {
                None
            }
        };
        if let Some(task_text) = next_task {
            self.send_message_internal(task_text);
        }
    }
}

pub(super) fn extract_tool_description(tool_name: &str, arguments: &str) -> Option<String> {
    if tool_name != "Bash" {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .and_then(|v| v.get("description")?.as_str().map(|s| s.to_string()))
}
