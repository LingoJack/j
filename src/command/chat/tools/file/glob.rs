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
        - 快速文件模式匹配工具，适用于任意规模的代码库
        - 支持 glob 模式如 "**/*.js" 或 "src/**/*.tsx"
        - 返回匹配的文件路径，按修改时间排序
        - 当你需要按文件名模式查找文件时使用此工具，如 "src/components/**/*.tsx"
        - 如果是开放式搜索，可能需要多轮 glob 和 grep，请改用 Agent 工具
        - 可以在单次响应中调用多个工具。如果多个文件模式可能有用，建议并行执行多个 glob 搜索。例如，需要按模式 A、B、C 查找文件时，并行运行三个 glob 查询
        - 重要：如果不需要指定路径，请省略此字段，不要输入 "undefined"、"null" 或空字符串
        - 重要：始终优先编辑代码库中的现有文件，除非明确要求，否则不要创建新文件
        - 重要：仅当用户明确要求时才使用 emoji
        "###
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "properties": {
                "limit": {
                    "default": 100,
                    "description": "返回结果的最大数量，默认 100",
                    "type": "integer"
                },
                "offset": {
                    "default": 0,
                    "description": "跳过前 N 个结果，配合 limit 实现分页",
                    "type": "integer"
                },
                "path": {
                    "description": "搜索的目录路径。如果不指定，则使用当前工作目录。重要：如果不需要指定路径，请省略此字段，不要输入 \"undefined\"、\"null\" 或空字符串",
                    "type": "string"
                },
                "pattern": {
                    "description": "要匹配的文件 glob 模式（如 \"**/*.js\"、\"*.{ts,tsx}\"、\"src/**/*.py\"）",
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

        // 构建完整的 glob 模式
        let full_pattern = if pattern.starts_with('/') {
            pattern.to_string()
        } else {
            format!("{}/{}", base_path.trim_end_matches('/'), pattern)
        };

        // 执行 glob 搜索
        let mut matches: Vec<std::path::PathBuf> = match glob::glob(&full_pattern) {
            Ok(paths) => paths.filter_map(Result::ok).collect(),
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
