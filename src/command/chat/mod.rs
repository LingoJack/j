pub mod agent;
pub mod api;
pub mod app;
pub mod archive;
pub mod autocomplete;
pub mod config;
pub mod constant;
pub mod handler;
pub mod markdown;
pub mod model;
pub mod permission;
pub mod render;
pub mod skill;
pub mod theme;
pub mod tools;
pub mod ui;

use crate::config::YamlConfig;
use crate::{error, info};
use api::call_openai_stream;
use handler::run_chat_tui;
use model::{ChatMessage, load_agent_config, load_system_prompt};
use std::io::{self, Write};

pub fn handle_chat(content: &[String], _config: &YamlConfig) {
    let agent_config = load_agent_config();

    if content.is_empty() || agent_config.providers.is_empty() {
        // 无参数，或尚未配置 provider：进入 TUI 对话界面
        // 若 providers 为空，TUI 会自动切换到配置界面引导用户完成配置
        run_chat_tui();
        return;
    }

    // 有参数：快速发送消息并打印回复
    let message = content.join(" ");
    let message = message.trim().to_string();
    if message.is_empty() {
        error!("⚠️ 消息内容为空");
        return;
    }

    let idx = agent_config
        .active_index
        .min(agent_config.providers.len() - 1);
    let provider = &agent_config.providers[idx];

    info!("💫 [{}] 思考中...", provider.name);

    let mut messages = Vec::new();
    messages.push(ChatMessage::text("user", message));

    match call_openai_stream(
        provider,
        &messages,
        load_system_prompt().as_deref(),
        &mut |chunk| {
            print!("{}", chunk);
            let _ = io::stdout().flush();
        },
    ) {
        Ok(_) => {
            println!(); // 换行
        }
        Err(e) => {
            error!("\n❌ {}", e);
        }
    }
}
