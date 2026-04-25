use crate::command::chat::infra::skill::{self, skills_dir};
use crate::command::chat::storage::{load_memory, load_soul, load_style, load_system_prompt};
use crate::command::chat::tools::ToolRegistry;
use std::sync::Arc;

pub struct StaticPlaceholderValues<'a> {
    pub skills_summary: &'a str,
    pub tools_summary: &'a str,
    pub style_text: &'a str,
    pub memory_text: &'a str,
    pub soul_text: &'a str,
    pub agent_md_text: &'a str,
    pub current_dir: &'a str,
    pub skill_dir: &'a str,
    pub project_skill_dir: &'a str,
}

pub fn apply_static_placeholders(template: &str, values: &StaticPlaceholderValues<'_>) -> String {
    template
        .replace("{{.current_dir}}", values.current_dir)
        .replace("{{.skills}}", values.skills_summary)
        .replace("{{.skill_dir}}", values.skill_dir)
        .replace("{{.project_skill_dir}}", values.project_skill_dir)
        .replace("{{.tools}}", values.tools_summary)
        .replace("{{.style}}", values.style_text)
        .replace("{{.memory}}", values.memory_text)
        .replace("{{.soul}}", values.soul_text)
        .replace("{{.agent_md}}", values.agent_md_text)
}

/// 构建每轮调用的 system_prompt_fn 闭包（TUI 和 oneshot 共用）
pub fn build_system_prompt_fn(
    loaded_skills: Vec<skill::Skill>,
    disabled_skills: Vec<String>,
    disabled_tools: Vec<String>,
    tool_registry: Arc<ToolRegistry>,
) -> Arc<dyn Fn() -> Option<String> + Send + Sync> {
    Arc::new(move || {
        use crate::command::chat::agent_md;
        let template = load_system_prompt()?;
        let skills_summary = skill::build_skills_summary(&loaded_skills, &disabled_skills);
        let tools_summary = tool_registry.build_tools_summary(&disabled_tools);
        let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
        let memory_text = load_memory().unwrap_or_default();
        let soul_text = load_soul().unwrap_or_default();
        let agent_md_text = agent_md::load_agent_md();
        let current_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let skill_dir = skills_dir().to_string_lossy().to_string();
        let project_skill_dir = skill::project_skills_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Some(apply_static_placeholders(
            &template,
            &StaticPlaceholderValues {
                skills_summary: &skills_summary,
                tools_summary: &tools_summary,
                style_text: &style_text,
                memory_text: &memory_text,
                soul_text: &soul_text,
                agent_md_text: &agent_md_text,
                current_dir: &current_dir,
                skill_dir: &skill_dir,
                project_skill_dir: &project_skill_dir,
            },
        ))
    })
}

use super::chat_app::ChatApp;
use crate::command::chat::storage::ChatMessage;
use crate::command::chat::tools::background::build_running_summary;
use crate::command::chat::tools::task::build_tasks_summary;

impl ChatApp {
    pub fn build_current_system_prompt(&self) -> Option<String> {
        let template = load_system_prompt()?;
        let skills_summary = skill::build_skills_summary(
            &self.state.loaded_skills,
            &self.state.agent_config.disabled_skills,
        );
        let tools_summary = self
            .tool_registry
            .build_tools_summary(&self.state.agent_config.disabled_tools);
        let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
        let memory_text = load_memory().unwrap_or_default();
        let soul_text = load_soul().unwrap_or_default();
        let agent_md_text = crate::command::chat::agent_md::load_agent_md();
        let current_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let skill_dir = skills_dir().to_string_lossy().to_string();
        let project_skill_dir = skill::project_skills_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let tasks_summary = build_tasks_summary(&self.task_manager);
        let background_summary = build_running_summary(&self.background_manager);
        let session_state_summary = self.tool_registry.build_session_state_summary();
        let teammates_summary = self
            .teammate_manager
            .lock()
            .map(|m| m.team_summary())
            .unwrap_or_default();

        let resolved = apply_static_placeholders(
            &template,
            &StaticPlaceholderValues {
                skills_summary: &skills_summary,
                tools_summary: &tools_summary,
                style_text: &style_text,
                memory_text: &memory_text,
                soul_text: &soul_text,
                agent_md_text: &agent_md_text,
                current_dir: &current_dir,
                skill_dir: &skill_dir,
                project_skill_dir: &project_skill_dir,
            },
        )
        .replace("{{.tasks}}", &tasks_summary)
        .replace("{{.background_tasks}}", &background_summary)
        .replace("{{.session_state}}", &session_state_summary)
        .replace("{{.teammates}}", &teammates_summary);
        Some(resolved)
    }

    /// 构建 LLM API 调用所需的消息列表。
    ///
    /// 从 `context_messages` 读取（带 XML 前缀，如 `<Teammate@Frontend>text</Teammate@Frontend>`），
    /// LLM 据此区分消息来源。UI 渲染则从 `display_messages` 读取（干净文本），两者完全独立。
    pub fn build_api_messages(&self) -> Vec<ChatMessage> {
        let compact = &self.state.agent_config.compact;
        let context_msgs = crate::util::safe_lock(&self.context_messages, "build_api_messages");
        crate::command::chat::context::window::select_messages(
            &context_msgs,
            self.state.agent_config.max_history_messages,
            self.state.agent_config.max_context_tokens,
            compact.keep_recent,
            &compact.micro_compact_exempt_tools,
        )
    }
}
