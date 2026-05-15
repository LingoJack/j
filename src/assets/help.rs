use std::collections::{BTreeSet, HashSet};

use super::Assets;

// ========== 数据类型 ==========

/// 帮助文档条目（树形结构）
#[derive(Debug, Clone)]
pub struct HelpEntry {
    /// 条目类型
    pub kind: HelpEntryKind,
    /// 树形缩进引导线
    pub guide: String,
}

/// 帮助文档条目类型
#[derive(Debug, Clone)]
pub enum HelpEntryKind {
    /// 目录条目
    Dir {
        /// 目录相对路径，如 "chat"
        dir_path: String,
        /// 显示名
        name: String,
        /// 子文件数量
        file_count: usize,
    },
    /// 文件条目
    File {
        /// 文件相对路径（不含 .md），如 "quickstart" 或 "chat/commands"
        path: String,
        /// 显示名
        name: String,
        /// Markdown 内容
        content: String,
    },
}

/// 目录展开状态
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HelpExpandedDirs(pub HashSet<String>);

impl HelpExpandedDirs {
    /// 创建空的展开目录集合
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    /// 判断指定目录是否已展开
    pub fn is_expanded(&self, dir_path: &str) -> bool {
        self.0.contains(dir_path)
    }

    /// 创建展开所有目录的集合
    pub fn all_expanded() -> Self {
        let files = collect_help_files();
        let dirs = collect_dirs(&files);
        Self(dirs.into_iter().collect())
    }

    /// 切换指定目录的展开/折叠状态
    pub fn toggle(&mut self, dir_path: &str) {
        if self.0.contains(dir_path) {
            self.0.remove(dir_path);
        } else {
            self.0.insert(dir_path.to_string());
        }
    }
}

impl Default for HelpExpandedDirs {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 内部数据结构 ==========

/// 帮助文件元信息（从 assets 读取后的中间表示）
#[derive(Debug, Clone)]
struct HelpFile {
    /// 文件相对路径（不含 .md），如 "quickstart" 或 "chat/commands"
    path: String,
    /// Markdown 内容
    content: String,
}

impl HelpFile {
    /// 获取显示名称（路径最后一部分）
    fn display_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// 获取所在目录（相对路径），如 "chat"，根目录返回 None
    fn parent_dir(&self) -> Option<&str> {
        self.path.rsplit_once('/').map(|(dir, _)| dir)
    }
}

// ========== 公开 API ==========

/// 从嵌入资源加载帮助文档，构建树形扁平化列表
///
/// 遍历 `assets/help/` 目录下的所有 `.md` 文件，按文件路径构建树形结构，
/// 根据展开状态展开目录，返回扁平化的 `HelpEntry` 列表。
pub fn load_help_entries(expanded_dirs: &HelpExpandedDirs) -> Vec<HelpEntry> {
    let files = collect_help_files();
    let dir_set = collect_dirs(&files);
    let file_count = files.len();

    let mut flat = Vec::new();
    build_flat_entries(&files, &dir_set, expanded_dirs, "", 0, &mut flat);

    // 如果没有任何条目被选中，默认选中第一个文件条目
    let _ = file_count; // 仅用于统计
    flat
}

/// 获取帮助文件总数
pub fn help_file_count() -> usize {
    collect_help_files().len()
}

// ========== 内部实现 ==========

/// 从嵌入资源收集所有帮助文件
fn collect_help_files() -> Vec<HelpFile> {
    let mut files: Vec<HelpFile> = Vec::new();

    for filename in Assets::iter() {
        let filename = filename.as_ref();

        if !filename.starts_with("help/") || !filename.ends_with(".md") {
            continue;
        }

        let asset = match Assets::get(filename) {
            Some(a) => a,
            None => continue,
        };

        let content = String::from_utf8_lossy(&asset.data);

        // 路径：去掉 "help/" 前缀和 ".md" 后缀
        let relative = &filename[5..]; // 去掉 "help/"
        let path = relative.trim_end_matches(".md");

        files.push(HelpFile {
            path: path.to_string(),
            content: strip_frontmatter(&content).to_string(),
        });
    }

    // 按路径排序，确保确定性顺序
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

/// 收集所有涉及的目录路径
fn collect_dirs(files: &[HelpFile]) -> BTreeSet<String> {
    let mut dir_set: BTreeSet<String> = BTreeSet::new();
    for file in files {
        if let Some(parent) = file.parent_dir() {
            // 添加所有祖先目录
            let parts: Vec<&str> = parent.split('/').collect();
            let mut acc = String::new();
            for part in &parts {
                if !acc.is_empty() {
                    acc.push('/');
                }
                acc.push_str(part);
                dir_set.insert(acc.clone());
            }
        }
    }
    dir_set
}

/// 递归构建扁平化条目列表
fn build_flat_entries(
    files: &[HelpFile],
    dir_set: &BTreeSet<String>,
    expanded_dirs: &HelpExpandedDirs,
    prefix: &str,
    depth: usize,
    flat: &mut Vec<HelpEntry>,
) {
    // 1. 收集当前前缀下的直接子目录
    let mut child_dirs: Vec<String> = Vec::new();
    for dir_path in dir_set.iter() {
        if prefix.is_empty() {
            if !dir_path.contains('/') {
                child_dirs.push(dir_path.clone());
            }
        } else if dir_path.starts_with(&format!("{}/", prefix)) {
            let rest = &dir_path[prefix.len() + 1..];
            if !rest.contains('/') {
                child_dirs.push(dir_path.clone());
            }
        }
    }

    // 2. 收集当前前缀下的直接文件
    let mut child_files: Vec<usize> = Vec::new();
    for (idx, file) in files.iter().enumerate() {
        let parent = file.parent_dir().unwrap_or("");
        if parent == prefix {
            child_files.push(idx);
        }
    }

    // 3. 先渲染子目录，再渲染文件
    for dir_path in &child_dirs {
        let name = dir_path.rsplit('/').next().unwrap_or(dir_path);
        let expanded = expanded_dirs.is_expanded(dir_path);
        let file_count = files
            .iter()
            .filter(|f| {
                f.parent_dir()
                    .is_some_and(|p| p == *dir_path || p.starts_with(&format!("{}/", dir_path)))
            })
            .count();

        let guide = "  ".repeat(depth);
        flat.push(HelpEntry {
            kind: HelpEntryKind::Dir {
                dir_path: dir_path.clone(),
                name: name.to_string(),
                file_count,
            },
            guide,
        });

        if expanded {
            build_flat_entries(files, dir_set, expanded_dirs, dir_path, depth + 1, flat);
        }
    }

    for &idx in &child_files {
        let file = &files[idx];
        let guide = "  ".repeat(depth);
        flat.push(HelpEntry {
            kind: HelpEntryKind::File {
                path: file.path.clone(),
                name: file.display_name().to_string(),
                content: file.content.clone(),
            },
            guide,
        });
    }
}

/// 去除 YAML frontmatter（如果存在）
fn strip_frontmatter(content: &str) -> &str {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return trimmed;
    }
    let after_first = &trimmed[3..];
    let Some(end) = after_first.find("\n---") else {
        return trimmed;
    };
    after_first[end + 4..].trim_start()
}
