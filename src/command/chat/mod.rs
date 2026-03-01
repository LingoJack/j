pub mod api;
pub mod app;
pub mod archive;
pub mod handler;
pub mod markdown;
pub mod model;
pub mod render;
pub mod skill;
pub mod theme;
pub mod tools;
pub mod ui;

use crate::command::chat::theme::ThemeName;
use crate::config::YamlConfig;
use crate::{error, info};
use api::call_openai_stream;
use handler::run_chat_tui;
use model::{
    AgentConfig, ChatMessage, ModelProvider, agent_config_path, load_agent_config,
    load_system_prompt, save_agent_config, save_system_prompt,
};
use std::io::{self, Write};

pub fn handle_chat(content: &[String], _config: &YamlConfig) {
    let mut agent_config = load_agent_config();
    if let Some(file_prompt) = load_system_prompt() {
        agent_config.system_prompt = Some(file_prompt);
    } else if let Some(config_prompt) = agent_config.system_prompt.clone() {
        let _ = save_system_prompt(&config_prompt);
    }

    if agent_config.providers.is_empty() {
        info!("⚠️  尚未配置 LLM 模型提供方。");
        info!("📁 请编辑配置文件: {}", agent_config_path().display());
        info!("📝 配置示例:");
        let example = AgentConfig {
            providers: vec![ModelProvider {
                name: "GPT-4o".to_string(),
                api_base: "https://api.openai.com/v1".to_string(),
                api_key: "sk-your-api-key".to_string(),
                model: "gpt-4o".to_string(),
            }],
            active_index: 0,
            system_prompt: None,
            stream_mode: true,
            max_history_messages: 20,
            theme: ThemeName::default(),
            tools_enabled: false,
            max_tool_rounds: 10,
        };
        if let Ok(json) = serde_json::to_string_pretty(&example) {
            println!("{}", json);
        }
        let _ = save_system_prompt("你是一个有用的助手。");
        // 自动创建示例配置文件
        if !agent_config_path().exists() {
            let _ = save_agent_config(&example);
            info!(
                "✅ 已自动创建示例配置文件: {}",
                agent_config_path().display()
            );
            info!("📌 请修改其中的 api_key 和其他配置后重新运行 chat 命令");
        }
        return;
    }

    if content.is_empty() {
        // 无参数：进入 TUI 对话界面
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

    info!("🤖 [{}] 思考中...", provider.name);

    let mut messages = Vec::new();
    messages.push(ChatMessage::text("user", message));

    match call_openai_stream(
        provider,
        &messages,
        agent_config.system_prompt.as_deref(),
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
