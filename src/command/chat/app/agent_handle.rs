use super::types::{StreamMsg, ToolResultMsg};
use crate::command::chat::agent::config::{AgentLoopConfig, AgentLoopSharedState};
use crate::command::chat::agent::run_main_agent_loop;
use crate::command::chat::error::ChatError;
use crate::command::chat::storage::ChatMessage;
use crate::llm::ToolDefinition;
use std::sync::{Arc, mpsc};
use tokio_util::sync::CancellationToken;

// ========== Agent 生命周期句柄 ==========

/// 主 Agent 生命周期管理：封装 stream channel、取消令牌等
#[derive(Debug)]
pub struct MainAgentHandle {
    /// 用于接收后台流式回复的 channel
    pub stream_rx: mpsc::Receiver<StreamMsg>,
    /// 流式请求取消令牌
    pub cancel_token: CancellationToken,
}

impl MainAgentHandle {
    /// 启动一个主 agent loop，返回 (MainAgentHandle, tool_result_tx)
    pub fn spawn(
        config: AgentLoopConfig,
        shared: AgentLoopSharedState,
        api_messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        system_prompt_fn: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    ) -> (Self, mpsc::SyncSender<ToolResultMsg>) {
        let (stream_tx, stream_rx) = mpsc::channel::<StreamMsg>();
        let (tool_result_tx, tool_result_rx) = mpsc::sync_channel::<ToolResultMsg>(16);

        let cancel_token = config.cancel_token.clone();

        std::thread::spawn(move || {
            // 保留一个 stream_tx 副本，用于 panic 后向主线程发送错误消息
            let stream_tx_panic = stream_tx.clone();

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = stream_tx
                            .send(StreamMsg::Error(ChatError::RuntimeFailed(e.to_string())));
                        return;
                    }
                };

                runtime.block_on(run_main_agent_loop(
                    config,
                    shared,
                    api_messages,
                    tools,
                    system_prompt_fn,
                    stream_tx,
                    tool_result_rx,
                ));
            }));

            if let Err(panic_info) = result {
                // 尝试提取 panic 信息
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    format!("Agent 线程 panic: {}", s)
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    format!("Agent 线程 panic: {}", s)
                } else {
                    "Agent 线程发生未知 panic".to_string()
                };
                crate::util::log::write_error_log("MainAgentHandle::spawn", &panic_msg);
                // 通知主线程，避免 loading 状态永久卡住
                let _ = stream_tx_panic.send(StreamMsg::Error(ChatError::AgentPanic(panic_msg)));
            }
        });

        (
            MainAgentHandle {
                stream_rx,
                cancel_token,
            },
            tool_result_tx,
        )
    }

    /// 取消当前流式请求
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 非阻塞地获取所有可用的流式消息
    pub fn poll(&self) -> Vec<StreamMsg> {
        let mut msgs = Vec::new();
        loop {
            match self.stream_rx.try_recv() {
                Ok(msg) => msgs.push(msg),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // channel 断开意味着 agent 线程已退出（可能是 panic 或异常），
                    // 不应伪造 StreamMsg::Done，否则会误报"回复完成"。
                    // 发送 Error 让主循环走错误处理路径。
                    msgs.push(StreamMsg::Error(ChatError::Other(
                        "Agent 通道已断开（agent 线程异常退出）".to_string(),
                    )));
                    break;
                }
            }
        }
        msgs
    }
}
