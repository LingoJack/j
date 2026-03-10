use crate::command::chat::skill::Skill;
use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

// ========== LoadSkillTool ==========

pub struct LoadSkillTool {
    pub skills: Vec<Skill>,
}

impl Tool for LoadSkillTool {
    fn name(&self) -> &str {
        "load_skill"
    }

    fn description(&self) -> &str {
        "加载指定 skill 的完整内容到上下文以了解更多信息，帮助你进行更好地完成任务。可在 skills 列表中查看各技能的目录路径。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "要加载的技能名称"
                },
                "arguments": {
                    "type": "string",
                    "description": "传递给技能的参数（可选）"
                }
            },
            "required": ["name"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed = serde_json::from_str::<Value>(arguments).ok();

        let skill_name = parsed
            .as_ref()
            .and_then(|v| v.get("name").and_then(|n| n.as_str()))
            .unwrap_or("");

        let args_str = parsed
            .as_ref()
            .and_then(|v| v.get("arguments").and_then(|a| a.as_str()))
            .unwrap_or("");

        if skill_name.is_empty() {
            return ToolResult {
                output: "参数缺少 name 字段".to_string(),
                is_error: true,
            };
        }

        match self
            .skills
            .iter()
            .find(|s| s.frontmatter.name == skill_name)
        {
            Some(skill) => {
                let content = crate::command::chat::skill::resolve_skill_content(skill);
                let resolved = content.replace("$ARGUMENTS", args_str);
                ToolResult {
                    output: resolved,
                    is_error: false,
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
                        skill_name,
                        available.join(", ")
                    ),
                    is_error: true,
                }
            }
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
