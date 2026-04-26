use serde::Deserialize;

use super::Assets;

/// 帮助 Tab 数据
#[derive(Debug, Clone)]
pub struct HelpTab {
    /// Tab 显示名称
    pub name: String,
    /// Tab Markdown 内容
    pub content: String,
}

/// 帮助 Tab frontmatter
#[derive(Deserialize)]
struct HelpFrontmatter {
    name: String,
    order: u32,
}

/// 从嵌入资源加载帮助 Tab 列表
///
/// 遍历 `assets/help/` 目录下的 `.md` 文件，解析 YAML frontmatter
/// 中的 `name` 和 `order` 字段，按 `order` 排序后返回。
pub fn load_help_tabs() -> Vec<HelpTab> {
    let mut tabs: Vec<(u32, HelpTab)> = Vec::new();

    for filename in Assets::iter() {
        let filename = filename.as_ref();

        // 只处理 help/ 前缀的 .md 文件
        if !filename.starts_with("help/") || !filename.ends_with(".md") {
            continue;
        }

        let asset = match Assets::get(filename) {
            Some(a) => a,
            None => continue,
        };

        let content = String::from_utf8_lossy(&asset.data);

        // 解析 frontmatter
        if let Some((fm_str, body)) = split_help_frontmatter(&content)
            && let Ok(fm) = serde_yaml::from_str::<HelpFrontmatter>(&fm_str)
        {
            tabs.push((
                fm.order,
                HelpTab {
                    name: fm.name,
                    content: body,
                },
            ));
        }
    }

    // 按 order 排序
    tabs.sort_by_key(|(order, _)| *order);
    tabs.into_iter().map(|(_, tab)| tab).collect()
}

/// 按 `---` 分隔 frontmatter 和 body
///
/// 格式与 SKILL.md 一致：首行 `---`，YAML frontmatter，`---` 结束，剩余为 body。
fn split_help_frontmatter(content: &str) -> Option<(String, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    let fm = after_first[..end].trim().to_string();
    let body = after_first[end + 4..].trim_start().to_string();
    Some((fm, body))
}
