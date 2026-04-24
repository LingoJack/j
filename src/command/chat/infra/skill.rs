use crate::config::YamlConfig;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ========== 数据结构 ==========

/// Skill 来源层级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillSource {
    /// 用户级: ~/.jdata/agent/skills/
    User,
    /// 项目级: .jcli/skills/
    Project,
}

impl SkillSource {
    /// 返回当前来源的中文标签
    pub fn label(&self) -> &'static str {
        match self {
            SkillSource::User => "用户",
            SkillSource::Project => "项目",
        }
    }
}

/// 技能的 YAML frontmatter 元数据
#[derive(Debug, Clone, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
}

/// 技能定义，包含 frontmatter 元数据、正文内容、目录路径和来源层级
#[derive(Debug, Clone)]
pub struct Skill {
    pub frontmatter: SkillFrontmatter,
    /// frontmatter 之后的 Markdown 正文
    pub body: String,
    /// skill 目录路径
    pub dir_path: PathBuf,
    /// 来源层级
    pub source: SkillSource,
}

// ========== 加载与解析 ==========

/// 返回用户级 skills 目录: ~/.jdata/agent/skills/
pub fn skills_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("agent").join("skills");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 返回项目级 skills 目录: .jcli/skills/（如果存在）
pub fn project_skills_dir() -> Option<PathBuf> {
    use super::super::permission::JcliConfig;
    let config_dir = JcliConfig::find_config_dir()?;
    let dir = config_dir.join("skills");
    if dir.is_dir() { Some(dir) } else { None }
}

/// 从指定目录加载 skills
fn load_skills_from_dir(dir: &Path, source: SkillSource) -> Vec<Skill> {
    let mut skills = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return skills,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if skill_md.exists()
            && let Some(mut skill) = parse_skill_md(&skill_md, &path)
        {
            skill.source = source;
            skills.push(skill);
        }
    }
    skills
}

/// 扫描 skills 目录，加载所有 skill（用户级 + 项目级，同名时项目级覆盖）
pub fn load_all_skills() -> Vec<Skill> {
    let mut map: HashMap<String, Skill> = HashMap::new();

    // 1. 用户级
    for skill in load_skills_from_dir(&skills_dir(), SkillSource::User) {
        map.insert(skill.frontmatter.name.clone(), skill);
    }

    // 2. 项目级（覆盖同名）
    if let Some(dir) = project_skills_dir() {
        for skill in load_skills_from_dir(&dir, SkillSource::Project) {
            map.insert(skill.frontmatter.name.clone(), skill);
        }
    }

    let mut skills: Vec<Skill> = map.into_values().collect();
    skills.sort_by(|a, b| a.frontmatter.name.cmp(&b.frontmatter.name));
    skills
}

/// 解析 SKILL.md: YAML frontmatter + body
fn parse_skill_md(path: &PathBuf, dir: &Path) -> Option<Skill> {
    let content = fs::read_to_string(path).ok()?;
    let (fm_str, body) = split_frontmatter(&content)?;
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(&fm_str).ok()?;

    if frontmatter.name.is_empty() {
        return None;
    }

    Some(Skill {
        frontmatter,
        body: body.trim().to_string(),
        dir_path: dir.to_path_buf(),
        source: SkillSource::User, // 由调用方覆盖
    })
}

/// 按 `---` 分隔 frontmatter 和 body
pub(super) fn split_frontmatter(content: &str) -> Option<(String, String)> {
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

/// 拼合 body + references/ 和 scripts/ 的文件列表（不内联内容，由模型按需 Read/Bash）
pub fn resolve_skill_content(skill: &Skill) -> String {
    let mut result = skill.body.clone();

    // 列出 references/ 目录中的文件路径，供模型按需读取
    if let Some(paths) = list_dir_files(&skill.dir_path.join("references")) {
        result.push_str("\n\n## 参考文件\n\n以下参考文件可按需使用 Read 工具读取：\n");
        for p in &paths {
            result.push_str(&format!("- `{}`\n", p));
        }
    }

    // 列出 scripts/ 目录中的脚本路径，供模型按需执行
    if let Some(paths) = list_dir_files(&skill.dir_path.join("scripts")) {
        result.push_str("\n\n## 脚本\n\n以下脚本可按需使用 Bash/BackgroundRun 工具执行：\n");
        for p in &paths {
            result.push_str(&format!("- `{}`\n", p));
        }
    }

    result
}

/// 列出目录下的文件路径（排序），目录不存在或为空时返回 None
fn list_dir_files(dir: &Path) -> Option<Vec<String>> {
    if !dir.is_dir() {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    let mut files: Vec<_> = entries.flatten().collect();
    files.sort_by_key(|e| e.file_name());
    let paths: Vec<String> = files
        .iter()
        .filter(|e| e.path().is_file())
        .map(|e| e.path().display().to_string())
        .collect();
    if paths.is_empty() { None } else { Some(paths) }
}

// ========== build_skills_summary ==========

/// 构建 skills 摘要列表（Markdown 格式），用于系统提示词的 {{.skills}} 占位符
/// disabled_skills 中的 skill 会被过滤掉
pub fn build_skills_summary(skills: &[Skill], disabled_skills: &[String]) -> String {
    let filtered: Vec<&Skill> = skills
        .iter()
        .filter(|s| !disabled_skills.iter().any(|d| d == &s.frontmatter.name))
        .collect();
    if filtered.is_empty() {
        return "（暂无可用技能）".to_string();
    }
    let mut md = String::new();
    for s in &filtered {
        md.push_str(&format!(
            "- {}: {}\n",
            s.frontmatter.name, s.frontmatter.description
        ));
    }
    md.trim_end().to_string()
}
