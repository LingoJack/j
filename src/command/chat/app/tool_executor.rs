use super::types::{
    CompletedToolResult, PlanDecision, ToolCallStatus, ToolExecStatus, ToolResultMsg,
};
use super::ui_state::ChatMode;
use crate::command::chat::constants::TOOL_OUTPUT_SUMMARY_MAX_LEN;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::tools::ToolRegistry;
use crate::util::log::write_info_log;
use std::sync::{Arc, Mutex, mpsc};

// ========== 工具执行器 ==========

/// 工具执行器：管理工具调用的状态和执行
pub struct ToolExecutor {
    /// 当前活跃的工具调用状态列表
    pub active_tool_calls: Vec<ToolCallStatus>,
    /// ToolConfirm 模式中当前待处理工具的索引
    pub pending_tool_idx: usize,
    /// 进入 ToolConfirm 模式的时间（用于超时自动执行）
    pub tool_confirm_entered_at: std::time::Instant,
    /// 是否有待执行的工具（已设为 Executing 状态但尚未实际调用）
    pub pending_tool_execution: bool,
    /// 当前正在后台执行的工具数量
    pub tools_executing_count: usize,
    /// 工具执行取消标志
    pub tool_cancelled: Arc<std::sync::atomic::AtomicBool>,
    /// Worker 完成后写入的共享结果（UI poll 时 drain）
    pub completed_results: Arc<Mutex<Vec<CompletedToolResult>>>,
    /// 工具结果发送通道（Worker/UI → Agent 线程）
    pub tool_result_tx: Option<mpsc::SyncSender<ToolResultMsg>>,
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            active_tool_calls: Vec::new(),
            pending_tool_idx: 0,
            tool_confirm_entered_at: std::time::Instant::now(),
            pending_tool_execution: false,
            tools_executing_count: 0,
            tool_cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            completed_results: Arc::new(Mutex::new(Vec::new())),
            tool_result_tx: None,
        }
    }

    /// 轮询后台工具执行结果，更新 UI 状态。
    /// Worker 已直接发 ToolResultMsg 给 Agent，这里只更新 active_tool_calls 的显示状态。
    /// 返回新完成的工具信息 (tool_name, summary, is_error)（供 broadcast_ws 使用）。
    pub fn poll_results(&mut self) -> Vec<(String, String, bool)> {
        let done_items: Vec<CompletedToolResult> = {
            if let Ok(mut results) = self.completed_results.lock() {
                results.drain(..).collect()
            } else {
                return Vec::new();
            }
        };
        if !done_items.is_empty() {
            write_info_log(
                "poll_tool_exec_results",
                &format!(
                    "收到 {} 个工具结果, tools_executing_count={}",
                    done_items.len(),
                    self.tools_executing_count,
                ),
            );
        }
        let mut completed = Vec::new();
        for done in done_items {
            // 查找工具名
            let tool_name = self
                .active_tool_calls
                .iter()
                .find(|tc| tc.tool_call_id == done.tool_call_id)
                .map(|tc| tc.tool_name.clone())
                .unwrap_or_default();
            // 更新 UI 显示状态
            if let Some(tc) = self
                .active_tool_calls
                .iter_mut()
                .find(|tc| tc.tool_call_id == done.tool_call_id)
            {
                tc.status = if done.is_error {
                    ToolExecStatus::Failed(done.summary.clone())
                } else {
                    ToolExecStatus::Done(done.summary.clone())
                };
            }
            completed.push((tool_name, done.summary, done.is_error));
            self.tools_executing_count = self.tools_executing_count.saturating_sub(1);
            if self.tools_executing_count == 0 {
                // 本批工具全部完成，重置取消标志
                self.tool_cancelled
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
        completed
    }

    /// 把所有 Executing 状态的工具放到后台线程执行
    pub fn execute_batch(&mut self, registry: &Arc<ToolRegistry>) {
        let tasks: Vec<(String, String, String)> = self
            .active_tool_calls
            .iter()
            .filter(|tc| matches!(tc.status, ToolExecStatus::Executing))
            .map(|tc| {
                (
                    tc.tool_call_id.clone(),
                    tc.tool_name.clone(),
                    tc.arguments.clone(),
                )
            })
            .collect();

        if tasks.is_empty() {
            return;
        }

        // 新一批工具开始前，清除上一批的取消标志
        self.tool_cancelled
            .store(false, std::sync::atomic::Ordering::Relaxed);

        self.tools_executing_count += tasks.len();

        let result_tx = self.tool_result_tx.clone();
        let completed_results = Arc::clone(&self.completed_results);

        for (tool_call_id, tool_name, arguments) in tasks {
            let result_tx = result_tx.clone();
            let completed_results = Arc::clone(&completed_results);
            let registry = Arc::clone(registry);
            let cancelled = Arc::clone(&self.tool_cancelled);
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    registry.execute(&tool_name, &arguments, &cancelled)
                }));
                let (output, is_error, images, plan_decision) = match result {
                    Ok(exec_result) => (
                        exec_result.output,
                        exec_result.is_error,
                        exec_result.images,
                        exec_result.plan_decision,
                    ),
                    Err(panic_info) => {
                        let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic_info.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        (
                            format!("[Tool panic] {}", msg),
                            true,
                            vec![],
                            PlanDecision::None,
                        )
                    }
                };
                // 生成摘要供 UI 显示
                let summary = if output.len() > TOOL_OUTPUT_SUMMARY_MAX_LEN {
                    let mut end = TOOL_OUTPUT_SUMMARY_MAX_LEN;
                    while !output.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}...", &output[..end])
                } else {
                    output.clone()
                };
                // 写入共享状态（供 UI poll）
                if let Ok(mut results) = completed_results.lock() {
                    results.push(CompletedToolResult {
                        tool_call_id: tool_call_id.clone(),
                        summary,
                        is_error,
                    });
                }
                // 直接发给 Agent 线程
                if let Some(ref tx) = result_tx {
                    write_info_log(
                        "ToolExecutor",
                        &format!(
                            "发送工具结果: tool_call_id={}, is_error={}, images_count={}, output_len={}",
                            tool_call_id,
                            is_error,
                            images.len(),
                            output.len()
                        ),
                    );
                    let _ = tx.send(ToolResultMsg {
                        tool_call_id,
                        result: output,
                        is_error,
                        images,
                        plan_decision,
                    });
                }
            });
        }
    }

    /// 用户确认执行当前待处理工具 → 返回 Some(ChatMode) 表示需要切换模式
    pub fn execute_current(&mut self, registry: &Arc<ToolRegistry>) -> Option<ChatMode> {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            return Some(ChatMode::Chat);
        }

        write_info_log(
            "execute_pending_tool",
            &format!(
                "确认执行 idx={}, tool={}, tools_executing_count={}",
                idx, self.active_tool_calls[idx].tool_name, self.tools_executing_count,
            ),
        );

        self.active_tool_calls[idx].status = ToolExecStatus::Executing;

        let (tool_name, arguments, tool_call_id) = {
            let tc = &self.active_tool_calls[idx];
            (
                tc.tool_name.clone(),
                tc.arguments.clone(),
                tc.tool_call_id.clone(),
            )
        };

        self.tools_executing_count += 1;

        // 新工具开始前，清除上一批的取消标志
        self.tool_cancelled
            .store(false, std::sync::atomic::Ordering::Relaxed);

        let result_tx = self.tool_result_tx.clone();
        let completed_results = Arc::clone(&self.completed_results);
        let registry = Arc::clone(registry);
        let cancelled = Arc::clone(&self.tool_cancelled);
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                registry.execute(&tool_name, &arguments, &cancelled)
            }));
            let (output, is_error, images, plan_decision) = match result {
                Ok(exec_result) => (
                    exec_result.output,
                    exec_result.is_error,
                    exec_result.images,
                    exec_result.plan_decision,
                ),
                Err(panic_info) => {
                    let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    (
                        format!("[Tool panic] {}", msg),
                        true,
                        vec![],
                        PlanDecision::None,
                    )
                }
            };
            // 生成摘要供 UI 显示
            let summary = if output.len() > TOOL_OUTPUT_SUMMARY_MAX_LEN {
                let mut end = TOOL_OUTPUT_SUMMARY_MAX_LEN;
                while !output.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &output[..end])
            } else {
                output.clone()
            };
            // 写入共享状态（供 UI poll）
            if let Ok(mut results) = completed_results.lock() {
                results.push(CompletedToolResult {
                    tool_call_id: tool_call_id.clone(),
                    summary,
                    is_error,
                });
            }
            // 直接发给 Agent 线程
            if let Some(ref tx) = result_tx {
                let _ = tx.send(ToolResultMsg {
                    tool_call_id,
                    result: output,
                    is_error,
                    images,
                    plan_decision,
                });
            }
        });

        self.advance()
    }

    /// 用户拒绝执行当前待处理工具 → 返回 Some(ChatMode) 表示需要切换模式
    pub fn reject_current(&mut self, reason: &str) -> Option<ChatMode> {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            return Some(ChatMode::Chat);
        }

        let tool_call_id = self.active_tool_calls[idx].tool_call_id.clone();
        self.active_tool_calls[idx].status = ToolExecStatus::Rejected;

        let reject_msg = if reason.is_empty() {
            "用户拒绝执行该工具".to_string()
        } else {
            format!("用户拒绝执行该工具。用户说: {}", reason)
        };

        if let Some(ref tx) = self.tool_result_tx {
            let _ = tx.send(ToolResultMsg {
                tool_call_id,
                result: reject_msg,
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            });
        }

        self.advance()
    }

    /// 用户选择 "允许并记住" → 返回 Some(ChatMode) 表示需要切换模式
    pub fn allow_and_execute(
        &mut self,
        registry: &Arc<ToolRegistry>,
        jcli_config: &mut Arc<JcliConfig>,
    ) -> Option<ChatMode> {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            return Some(ChatMode::Chat);
        }

        let tool_name = self.active_tool_calls[idx].tool_name.clone();
        let arguments = self.active_tool_calls[idx].arguments.clone();

        // 生成 allow 规则并写入 .jcli/permissions.yaml
        let rule = crate::command::chat::permission::generate_allow_rule(&tool_name, &arguments);
        let mut jcli = (**jcli_config).clone();
        jcli.add_allow_rule(&rule);
        *jcli_config = Arc::new(jcli);

        // 执行工具
        self.execute_current(registry)
    }

    /// 是否还有待确认的工具
    pub fn has_pending_confirm(&self) -> bool {
        self.active_tool_calls
            .iter()
            .any(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
    }

    /// 推进到下一个待确认工具，或返回 Some(ChatMode::Chat) 退出确认模式
    pub fn advance(&mut self) -> Option<ChatMode> {
        let next = self
            .active_tool_calls
            .iter()
            .enumerate()
            .find(|(_, tc)| matches!(tc.status, ToolExecStatus::PendingConfirm))
            .map(|(i, _)| i);

        if let Some(next_idx) = next {
            self.pending_tool_idx = next_idx;
            self.tool_confirm_entered_at = std::time::Instant::now();
            write_info_log(
                "advance_tool_confirm",
                &format!("推进到 pending_tool_idx={}", next_idx),
            );
            None // 继续保持 ToolConfirm 模式
        } else {
            write_info_log(
                "advance_tool_confirm",
                &format!(
                    "所有工具已处理, 退出 ToolConfirm, tools_executing_count={}",
                    self.tools_executing_count,
                ),
            );
            Some(ChatMode::Chat)
        }
    }

    /// 只取消工具执行，不终止 agent loop
    pub fn cancel(&mut self) {
        self.tool_cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// 重置所有工具状态（新消息发送时调用）
    pub fn reset(&mut self) {
        self.active_tool_calls.clear();
        self.pending_tool_idx = 0;
        if let Ok(mut results) = self.completed_results.lock() {
            results.clear();
        }
        self.tools_executing_count = 0;
        self.pending_tool_execution = false;
        self.tool_cancelled
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}
