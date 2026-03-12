use crate::config::YamlConfig;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

// ========== 数据结构 ==========

#[derive(Debug, Clone, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,

    #[allow(dead_code)]
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
        if skill_md.exists()
            && let Some(skill) = parse_skill_md(&skill_md, &path)
        {
            skills.push(skill);
        }
    }

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

/// 拼合 body + references/ 下的参考文件
pub fn resolve_skill_content(skill: &Skill) -> String {
    let mut result = skill.body.clone();

    // 读取 references/ 目录
    let refs_dir = skill.dir_path.join("references");
    if refs_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&refs_dir)
    {
        let mut ref_files: Vec<_> = entries.flatten().collect();
        ref_files.sort_by_key(|e| e.file_name());
        for entry in ref_files {
            let path = entry.path();
            if path.is_file()
                && let Ok(content) = fs::read_to_string(&path)
            {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                result.push_str(&format!("\n\n--- 参考文件: {} ---\n{}", filename, content));
            }
        }
    }

    result
}

// ========== build_skills_summary ==========

/// 构建 skills 摘要列表（JSON 数组格式），用于系统提示词的 {{.skills}} 占位符
/// disabled_skills 中的 skill 会被过滤掉
pub fn build_skills_summary(skills: &[Skill], disabled_skills: &[String]) -> String {
    let filtered: Vec<&Skill> = skills
        .iter()
        .filter(|s| !disabled_skills.iter().any(|d| d == &s.frontmatter.name))
        .collect();
    if filtered.is_empty() {
        return "[]".to_string();
    }
    let items: Vec<serde_json::Value> = filtered
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.frontmatter.name,
                "description": s.frontmatter.description,
                "dir": s.dir_path.to_string_lossy()
            })
        })
        .collect();
    serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".to_string())
}
