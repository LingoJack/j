use super::tools::{Tool, ToolResult};
use crate::config::YamlConfig;
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

// ========== 数据结构 ==========

#[derive(Debug, Clone, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(rename = "argument-hint")]
    pub argument_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub frontmatter: SkillFrontmatter,
    /// frontmatter 之后的 Markdown 正文
    pub body: String,
    /// skill 目录路径
    pub dir_path: PathBuf,
}

// ========== 加载与解析 ==========

/// 返回 skills 目录: ~/.jdata/agent/skills/
pub fn skills_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("agent").join("skills");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 扫描 skills 目录，加载所有 skill
pub fn load_all_skills() -> Vec<Skill> {
    let dir = skills_dir();
    let mut skills = Vec::new();

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return skills,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if skill_md.exists() {
            if let Some(skill) = parse_skill_md(&skill_md, &path) {
                skills.push(skill);
            }
        }
    }

    skills.sort_by(|a, b| a.frontmatter.name.cmp(&b.frontmatter.name));
    skills
}

/// 解析 SKILL.md: YAML frontmatter + body
fn parse_skill_md(path: &PathBuf, dir: &PathBuf) -> Option<Skill> {
    let content = fs::read_to_string(path).ok()?;
    let (fm_str, body) = split_frontmatter(&content)?;
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(&fm_str).ok()?;

    if frontmatter.name.is_empty() {
        return None;
    }

    Some(Skill {
        frontmatter,
        body: body.trim().to_string(),
        dir_path: dir.clone(),
    })
}

/// 按 `---` 分隔 frontmatter 和 body
fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    // 跳过第一个 ---
    let rest = &trimmed[3..];
    let end_idx = rest.find("\n---")?;
    let fm = rest[..end_idx].trim().to_string();
    let body = rest[end_idx + 4..].to_string();
    Some((fm, body))
}

/// 拼合 body + references/ 下的参考文件，截断到 MAX_BYTES
pub fn resolve_skill_content(skill: &Skill) -> String {
    const MAX_BYTES: usize = 12000;
    let mut result = skill.body.clone();

    // 读取 references/ 目录
    let refs_dir = skill.dir_path.join("references");
    if refs_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&refs_dir) {
            let mut ref_files: Vec<_> = entries.flatten().collect();
            ref_files.sort_by_key(|e| e.file_name());
            for entry in ref_files {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let filename = path.file_name().unwrap_or_default().to_string_lossy();
                        result
                            .push_str(&format!("\n\n--- 参考文件: {} ---\n{}", filename, content));
                    }
                }
                if result.len() > MAX_BYTES {
                    break;
                }
            }
        }
    }

    // 截断到 MAX_BYTES
    if result.len() > MAX_BYTES {
        let mut end = MAX_BYTES;
        while !result.is_char_boundary(end) {
            end -= 1;
        }
        result.truncate(end);
        result.push_str("\n...(内容已截断)");
    }

    result
}

// ========== build_skills_summary ==========

/// 构建 skills 摘要列表（name + description），用于系统提示词的 {{.skills}} 占位符
pub fn build_skills_summary(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return "（暂无已安装的技能）".to_string();
    }
    let mut result = String::new();
    for skill in skills {
        result.push_str(&format!(
            "- **{}**: {}\n",
            skill.frontmatter.name, skill.frontmatter.description
        ));
    }
    result.trim_end().to_string()
}

// ========== LoadSkillTool: 统一的技能加载工具 ==========

pub struct LoadSkillTool {
    pub skills: Vec<Skill>,
}

impl Tool for LoadSkillTool {
    fn name(&self) -> &str {
        "load_skill"
    }

    fn description(&self) -> &str {
        "加载指定技能的完整内容到上下文。当你判断需要某个技能时调用此工具。"
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

    fn execute(&self, arguments: &str) -> ToolResult {
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
                let content = resolve_skill_content(skill);
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
