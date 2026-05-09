use crate::command::chat::agent_md;
use crate::command::chat::context::window;
use crate::command::chat::infra::skill::{self, skills_dir};
use crate::command::chat::storage::{load_memory, load_soul, load_style, load_system_prompt};
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::{BackgroundManager, build_running_summary};
use crate::command::chat::tools::task::{TaskManager, build_tasks_summary};
use crate::util;
use std::sync::{Arc, Mutex};

pub struct StaticPlaceholderValues<'a> {
    pub skills_summary: &'a str,
    pub tools_summary: &'a str,
    pub style_text: &'a str,
    pub memory_text: &'a str,
    pub soul_text: &'a str,
    pub project_instructions: &'a str,
    pub current_dir: &'a str,
    pub skill_dir: &'a str,
    pub project_skill_dir: &'a str,
}

/// 将静态占位符值应用到系统提示模板中，返回替换后的完整字符串。
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
        .replace("{{.project_instructions}}", values.project_instructions)
}

/// 构建每轮调用的 system_prompt_fn 闭包（TUI 和 oneshot 共用）
///
/// 每轮 agent loop 调用此闭包时，动态读取最新的 teammate 状态、task 列表等信息，
/// 确保 main agent 的 system prompt 始终反映当前团队状态。
#[allow(clippy::too_many_arguments)]
pub fn build_system_prompt_fn(
    loaded_skills: Vec<skill::Skill>,
    disabled_skills: Vec<String>,
    disabled_tools: Vec<String>,
    deferred_tools: Arc<Mutex<Vec<String>>>,
    tool_registry: Arc<ToolRegistry>,
    teammate_manager: Arc<Mutex<TeammateManager>>,
    task_manager: Arc<TaskManager>,
    background_manager: Arc<BackgroundManager>,
) -> Arc<dyn Fn() -> Option<String> + Send + Sync> {
    Arc::new(move || {
        use crate::command::chat::agent_md;
        let template = load_system_prompt()?;
        let skills_summary = skill::build_skills_summary(&loaded_skills, &disabled_skills);
        // 排除 deferred 工具，只将非 deferred 的工具摘要拼入 system prompt
        let deferred = match deferred_tools.lock() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };
        let tools_summary =
            tool_registry.build_tools_summary_non_deferred(&disabled_tools, &deferred);
        let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
        let memory_text = load_memory().unwrap_or_default();
        let soul_text = load_soul().unwrap_or_default();
        let project_instructions = agent_md::load_agent_md();
        let current_dir = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let skill_dir = skills_dir().to_string_lossy().to_string();
        let project_skill_dir = skill::project_skills_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // 动态占位符（每轮更新）
        let tasks_summary = build_tasks_summary(&task_manager);
        let background_summary = build_running_summary(&background_manager);
        let session_state_summary = tool_registry.build_session_state_summary();
        let teammates_summary = teammate_manager
            .lock()
            .map(|m| m.team_summary())
            .unwrap_or_default();

        Some(
            apply_static_placeholders(
                &template,
                &StaticPlaceholderValues {
                    skills_summary: &skills_summary,
                    tools_summary: &tools_summary,
                    style_text: &style_text,
                    memory_text: &memory_text,
                    soul_text: &soul_text,
                    project_instructions: &project_instructions,
                    current_dir: &current_dir,
                    skill_dir: &skill_dir,
                    project_skill_dir: &project_skill_dir,
                },
            )
            .replace("{{.tasks}}", &tasks_summary)
            .replace("{{.background_tasks}}", &background_summary)
            .replace("{{.session_state}}", &session_state_summary)
            .replace("{{.teammates}}", &teammates_summary),
        )
    })
}

use super::chat_app::ChatApp;
use crate::command::chat::storage::ChatMessage;

impl ChatApp {
    pub fn build_current_system_prompt(&self) -> Option<String> {
        let template = load_system_prompt()?;
        let skills_summary = skill::build_skills_summary(
            &self.state.loaded_skills,
            &self.state.agent_config.disabled_skills,
        );
        let deferred = match self.deferred_tools.lock() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        };
        let tools_summary = self
            .tool_registry
            .build_tools_summary_non_deferred(&self.state.agent_config.disabled_tools, &deferred);
        let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
        let memory_text = load_memory().unwrap_or_default();
        let soul_text = load_soul().unwrap_or_default();
        let project_instructions = agent_md::load_agent_md();
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
                project_instructions: &project_instructions,
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
        let context_msgs = util::safe_lock(&self.context_messages, "build_api_messages");
        window::select_messages(
            &context_msgs,
            self.state.agent_config.max_history_messages,
            self.state.agent_config.max_context_tokens,
            compact.keep_recent,
            &compact.micro_compact_exempt_tools,
        )
    }
}
