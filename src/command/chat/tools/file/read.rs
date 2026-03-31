use crate::command::chat::tools::{ImageData, Tool, ToolResult, expand_tilde};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};

/// 图片文件扩展名
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tiff", "tif",
];

/// 检测文件是否为图片
fn is_image_file(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// 根据扩展名获取图片 MIME 类型
fn image_media_type(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "tiff" | "tif" => "image/tiff",
        _ => "application/octet-stream",
    }
}

/// 读取文件的工具
pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "Read"
    }

    fn description(&self) -> &str {
        "Read local file contents and return with line numbers. Supports reading by line range via offset and limit parameters. Can also read image files (png/jpg/gif/webp/bmp) and return them as visual content for multimodal models."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to read (absolute or relative to current working directory)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Starting line number (0-based, i.e. 0 = first line). Omit to start from the beginning"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of lines to read. Omit to read to end of file"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let v = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let path = match v.get("path").and_then(|c| c.as_str()) {
            Some(p) => expand_tilde(p),
            None => {
                return ToolResult {
                    output: "参数缺少 path 字段".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        // 图片文件：读取为 base64，返回图片数据
        if is_image_file(&path) {
            return read_image_file(&path);
        }

        let offset = v.get("offset").and_then(|o| o.as_u64()).map(|o| o as usize);
        let limit = v.get("limit").and_then(|l| l.as_u64()).map(|l| l as usize);

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let start = offset.unwrap_or(0).min(total);
                let count = limit.unwrap_or(total - start).min(total - start);
                let selected: Vec<String> = lines[start..start + count]
                    .iter()
                    .enumerate()
                    .map(|(i, line)| format!("{:>4}│ {}", start + i + 1, line))
                    .collect();
                let mut result = selected.join("\n");

                if start + count < total {
                    result.push_str(&format!("\n...(还有 {} 行未显示)", total - start - count));
                }

                ToolResult {
                    output: result,
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("读取文件失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

/// 读取图片文件，返回 base64 图片数据
fn read_image_file(path: &str) -> ToolResult {
    use base64::Engine;

    match std::fs::read(path) {
        Ok(bytes) => {
            let size_kb = bytes.len() as f64 / 1024.0;
            let media_type = image_media_type(path);
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

            let output = format!(
                "图片文件: {}\n大小: {:.1} KB\n类型: {}",
                path, size_kb, media_type
            );

            ToolResult {
                output,
                is_error: false,
                images: vec![ImageData {
                    base64: b64,
                    media_type: media_type.to_string(),
                }],
            }
        }
        Err(e) => ToolResult {
            output: format!("读取图片文件失败: {}", e),
            is_error: true,
            images: vec![],
        },
    }
}
