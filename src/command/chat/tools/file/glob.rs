use crate::command::chat::tools::{Tool, ToolResult, expand_tilde};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        r###"
        - Fast file pattern matching tool, works with any codebase size
        - Supports glob patterns like "**/*.js" or "src/**/*.tsx"
        - Returns matching file paths sorted by modification time
        - Use this tool when you need to find files by name pattern, e.g. "src/components/**/*.tsx"
        - For open-ended searches that may require multiple rounds of glob and grep, use the Agent tool instead
        - Multiple tools can be called in a single response. If multiple file patterns may be useful, run glob searches in parallel
        - Important: if no path is needed, omit the field entirely — do not enter "undefined", "null", or empty string
        - Important: always prefer editing existing files in the codebase; do not create new files unless explicitly required
        - Important: only use emojis if the user explicitly requests it
        "###
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "properties": {
                "limit": {
                    "default": 100,
                    "description": "Maximum number of results to return, default 100",
                    "type": "integer"
                },
                "offset": {
                    "default": 0,
                    "description": "Skip the first N results, for pagination with limit",
                    "type": "integer"
                },
                "path": {
                    "description": "Directory path to search. Defaults to current working directory if not specified. Important: omit this field if not needed — do not enter undefined, null, or empty string",
                    "type": "string"
                },
                "pattern": {
                    "description": "File glob pattern to match (e.g. **/*.js, *.{ts,tsx}, src/**/*.py)",
                    "type": "string"
                },
                "excludePattern": {
                    "description": "File glob pattern to exclude (e.g. **/node_modules/**, **/.git/**)",
                    "type": "string"
                }
            },
            "required": ["pattern"],
            "type": "object"
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let v = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let pattern = match v.get("pattern").and_then(|p| p.as_str()) {
            Some(p) => p,
            None => {
                return ToolResult {
                    output: "参数缺少 pattern 字段".to_string(),
                    is_error: true,
                };
            }
        };

        // 获取搜索路径，默认为当前目录
        let base_path = v
            .get("path")
            .and_then(|p| p.as_str())
            .filter(|s| !s.is_empty())
            .map(expand_tilde)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|d| d.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            });

        // 获取限制数量
        let limit = v
            .get("limit")
            .and_then(|l| l.as_u64())
            .map(|l| (l as usize).clamp(1, 1000))
            .unwrap_or(100);

        // 获取偏移量
        let offset = v
            .get("offset")
            .and_then(|o| o.as_u64())
            .map(|o| o as usize)
            .unwrap_or(0);

        // 解析排除模式
        let exclude_pattern = v
            .get("excludePattern")
            .and_then(|p| p.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|p| glob::Pattern::new(p).ok());

        // 构建完整的 glob 模式
        let full_pattern = if pattern.starts_with('/') {
            pattern.to_string()
        } else {
            format!("{}/{}", base_path.trim_end_matches('/'), pattern)
        };

        // 执行 glob 搜索
        let mut matches: Vec<std::path::PathBuf> = match glob::glob(&full_pattern) {
            Ok(paths) => paths
                .filter_map(Result::ok)
                .filter(|path| {
                    // 应用排除模式过滤
                    if let Some(ref exclude) = exclude_pattern {
                        let path_str = path.to_string_lossy();
                        !exclude.matches(&path_str)
                    } else {
                        true
                    }
                })
                .collect(),
            Err(e) => {
                return ToolResult {
                    output: format!("glob 模式无效: {}", e),
                    is_error: true,
                };
            }
        };

        // 按修改时间排序（最新的在前）
        matches.sort_by(|a, b| {
            let meta_a = std::fs::metadata(a);
            let meta_b = std::fs::metadata(b);
            let time_a = meta_a.ok().and_then(|m| m.modified().ok());
            let time_b = meta_b.ok().and_then(|m| m.modified().ok());
            time_b.cmp(&time_a)
        });

        let total = matches.len();

        if total == 0 {
            return ToolResult {
                output: format!("未找到匹配 '{}' 的文件", pattern),
                is_error: false,
            };
        }

        // 应用分页
        let paginated: Vec<_> = matches.into_iter().skip(offset).take(limit).collect();

        let displayed = paginated.len();

        // 格式化输出
        let mut result = String::new();
        result.push_str(&format!("找到 {} 个匹配文件", total));
        if offset > 0 || offset + displayed < total {
            result.push_str(&format!(
                "（显示 {}-{} 项，共 {} 项）",
                offset + 1,
                offset + displayed,
                total
            ));
        }
        result.push_str(":\n\n");

        for path in paginated {
            result.push_str(&format!("{}\n", path.display()));
        }

        if offset + displayed < total {
            result.push_str(&format!(
                "\n... 还有 {} 个结果未显示（使用 offset={} 继续查看）",
                total - offset - displayed,
                offset + displayed
            ));
        }

        ToolResult {
            output: result,
            is_error: false,
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
