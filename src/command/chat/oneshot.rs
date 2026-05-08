//! oneshot 模式：非 TUI 终端交互入口
//!
//! 子模块职责：
//! - `session`      → 会话 ID 生成、消息持久化、Hook 触发
//! - `display`      → 终端打印（工具参数预览、调用/结果行、Markdown 重绘）
//! - `animation`    → 思考动画线程
//! - `confirm`      → 交互确认 UI + 公共边框绘制
//! - `ask_ui`       → Ask 请求处理（单选/多选 UI）
//! - `tool_exec`    → 工具调用确认 + 执行
//! - `agent_loop`   → Agent 主循环（有工具/无工具模式）

pub(crate) mod agent_loop;
pub(crate) mod animation;
pub(crate) mod ask_ui;
pub(crate) mod confirm;
pub(crate) mod display;
pub(crate) mod session;
pub(crate) mod tool_exec;

use crate::command::chat::handler::run_chat_tui;
use crate::command::chat::oneshot::agent_loop::{run_oneshot_agent, run_oneshot_no_tools};
use crate::command::chat::oneshot::session::generate_oneshot_session_id;
use crate::command::chat::storage::{find_latest_session_id, load_agent_config, load_session};
use crate::config::YamlConfig;
use crate::{error, info};

/// oneshot chat 启动参数
#[derive(Debug)]
pub struct ChatArgs<'a> {
    pub content: &'a [String],
    pub cont: bool,
    pub session_id: Option<&'a str>,
    pub remote: bool,
    pub port: u16,
    pub bypass: bool,
    pub no_render: bool,
    pub config: &'a YamlConfig,
}

/// 处理 chat 子命令入口
pub fn handle_chat(args: ChatArgs<'_>) {
    let ChatArgs {
        content,
        cont,
        session_id: session_id_opt,
        remote,
        port,
        bypass,
        no_render,
        config: _config,
    } = args;
    let agent_config = load_agent_config();

    if remote
        || content.is_empty() && !cont && session_id_opt.is_none()
        || agent_config.providers.is_empty()
    {
        run_chat_tui(remote, port);
        return;
    }

    let message = content.join(" ");
    let message = message.trim().to_string();
    if message.is_empty() && !cont && session_id_opt.is_none() {
        error!("消息内容为空");
        return;
    }
    if message.is_empty() {
        error!("消息内容为空（--continue / --session 需要附带消息内容）");
        return;
    }

    let session_id = if let Some(id) = session_id_opt {
        id.to_string()
    } else if cont {
        find_latest_session_id().unwrap_or_else(generate_oneshot_session_id)
    } else {
        generate_oneshot_session_id()
    };

    let prior_messages = if cont || session_id_opt.is_some() {
        let loaded = load_session(&session_id);
        if !loaded.is_empty() {
            info!("延续会话 {} （{} 条历史消息）", session_id, loaded.len());
        } else if session_id_opt.is_some() {
            info!("会话 {} 不存在或为空，开始新对话", session_id);
        }
        loaded
    } else {
        vec![]
    };

    let idx = agent_config
        .active_index
        .min(agent_config.providers.len() - 1);
    let provider = &agent_config.providers[idx];

    if agent_config.tools_enabled {
        run_oneshot_agent(
            provider,
            &agent_config,
            message,
            prior_messages,
            &session_id,
            bypass,
            no_render,
        );
    } else {
        run_oneshot_no_tools(
            provider,
            &agent_config,
            message,
            prior_messages,
            &session_id,
            no_render,
        );
    }
}
