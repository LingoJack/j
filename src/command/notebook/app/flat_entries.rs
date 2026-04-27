//! 扁平化条目构建逻辑。

use std::collections::{BTreeSet, HashSet};

use super::types::{ExpandedDirs, FlatEntry, FlatEntryKind, NoteItem};

/// 扁平化构建的上下文参数，封装 8 参数中的不变部分。
pub struct FlatEntriesContext<'a> {
    notes: &'a [NoteItem],
    filtered_set: &'a HashSet<usize>,
    dir_set: &'a BTreeSet<String>,
    expanded_dirs: &'a ExpandedDirs,
}

impl<'a> FlatEntriesContext<'a> {
    /// 从笔记列表、过滤集、目录集、展开目录创建上下文
    pub fn new(
        notes: &'a [NoteItem],
        filtered_set: &'a HashSet<usize>,
        dir_set: &'a BTreeSet<String>,
        expanded_dirs: &'a ExpandedDirs,
    ) -> Self {
        Self {
            notes,
            filtered_set,
            dir_set,
            expanded_dirs,
        }
    }
}

/// 递归构建扁平化条目列表
pub fn build_flat_entries_recursive(
    ctx: FlatEntriesContext<'_>,
    prefix: &str,
    depth: usize,
    flat: &mut Vec<FlatEntry>,
) {
    // 1. 收集当前前缀下的直接子目录（已排序，BTreeSet 保证）
    let mut child_dirs: Vec<String> = Vec::new();
    for dir_path in ctx.dir_set.iter() {
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

    // 2. 收集当前前缀下的直接笔记文件
    let mut child_files: Vec<usize> = Vec::new();
    for &idx in ctx.filtered_set {
        let note = &ctx.notes[idx];
        let parent = note.parent_dir().unwrap_or("");
        if parent == prefix {
            child_files.push(idx);
        }
    }

    // 3. 先渲染子目录，再渲染文件
    for dir_path in &child_dirs {
        let name = dir_path.rsplit('/').next().unwrap_or(dir_path);
        let expanded = ctx.expanded_dirs.is_expanded(dir_path);
        let file_count = ctx
            .filtered_set
            .iter()
            .filter(|&&idx| {
                ctx.notes[idx]
                    .parent_dir()
                    .is_some_and(|p| p == *dir_path || p.starts_with(&format!("{}/", dir_path)))
            })
            .count();

        let guide = "  ".repeat(depth);
        flat.push(FlatEntry {
            kind: FlatEntryKind::Dir {
                dir_path: dir_path.clone(),
                name: name.to_string(),
                file_count,
            },
            guide,
        });

        // 如果展开，递归
        if expanded {
            build_flat_entries_recursive(
                FlatEntriesContext::new(
                    ctx.notes,
                    ctx.filtered_set,
                    ctx.dir_set,
                    ctx.expanded_dirs,
                ),
                dir_path,
                depth + 1,
                flat,
            );
        }
    }

    for &idx in &child_files {
        let guide = "  ".repeat(depth);
        flat.push(FlatEntry {
            kind: FlatEntryKind::File { note_index: idx },
            guide,
        });
    }
}
