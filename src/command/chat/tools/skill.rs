use crate::command::chat::compact::{InvokedSkillsMap, record_skill_invocation};
use crate::command::chat::skill::Skill;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

/// LoadSkillTool 参数
#[derive(Deserialize, JsonSchema)]
struct LoadSkillParams {
    /// Name of the skill to load
    name: String,
    /// Arguments to pass to the skill (optional)
    #[serde(default)]
    arguments: Option<String>,
}

// ========== LoadSkillTool ==========

#[derive(Debug)]
pub struct LoadSkillTool {
    pub skills: Vec<Skill>,
    /// 已调用技能追踪（执行时记录，供 auto_compact 后恢复）
    pub invoked_skills: InvokedSkillsMap,
}

impl LoadSkillTool {
    pub const NAME: &'static str = "LoadSkill";
}

impl Tool for LoadSkillTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        "Load the full content of a specified skill into context for more information, helping you better complete the task. Check the skills list for available skill names and directory paths."
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<LoadSkillParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: LoadSkillParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        if params.name.is_empty() {
            return ToolResult {
                output: "参数缺少 name 字段".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        let args_str = params.arguments.as_deref().unwrap_or("");

        match self
            .skills
            .iter()
            .find(|s| s.frontmatter.name == params.name)
        {
            Some(skill) => {
                let content = crate::command::chat::skill::resolve_skill_content(skill);
                let resolved = content.replace("$ARGUMENTS", args_str);
                // 记录技能调用（供 auto_compact 后恢复技能指令）
                record_skill_invocation(
                    &self.invoked_skills,
                    skill.frontmatter.name.clone(),
                    skill.dir_path.display().to_string(),
                    resolved.clone(),
                );
                ToolResult {
                    output: resolved,
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            None => {
                let available: Vec<&str> = self
                    .skills
                    .iter()
                    .map(|s| s.frontmatter.name.as_str())
                    .collect();
                ToolResult {
                    output: format!(
                        "未找到技能 '{}'。可用技能: {}",
                        params.name,
                        available.join(", ")
                    ),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
