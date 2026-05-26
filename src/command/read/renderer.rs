//! 文件渲染器：把不同格式的源文件转换为 Web 前端可消费的 JSON payload。
//!
//! 当前实现：
//! - `MarkdownRenderer` — 复用 `crate::markdown::parser::parse_markdown` 产出 IR
//! - `PlainTextRenderer` — 其它格式的兜底，原文打包为字符串
//!
//! 未来扩展（接口已预留，实现待补）：
//! - `PptxRenderer` / `DocxRenderer` / `XlsxRenderer`
//!
//! 选型策略由 [`pick_renderer`] 根据文件扩展名决定。

use serde::Serialize;
use std::path::Path;

/// 文档类型 — 用于前端按类型分发渲染组件。
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocKind {
    Markdown,
    PlainText,
    // 以下为占位，本期不实现：
    #[allow(dead_code)]
    Pptx,
    #[allow(dead_code)]
    Docx,
    #[allow(dead_code)]
    Xlsx,
}

/// 渲染产物 — 直接序列化为 `/api/doc` 的响应体。
#[derive(Debug, Serialize)]
pub struct RenderedDoc {
    /// 文件名（不含路径，仅用于 UI 展示）
    pub filename: String,
    /// 文档类型，前端按此分发组件
    pub kind: DocKind,
    /// 类型特定的载荷：
    /// - Markdown → `ParsedDocument` JSON
    /// - PlainText → `{ "text": "..." }`
    pub payload: serde_json::Value,
}

/// 文件渲染器抽象。
pub trait Renderer {
    fn render(&self, bytes: &[u8], filename: &str) -> Result<RenderedDoc, String>;
}

/// Markdown 渲染器：复用项目内 `parse_markdown`，输出 IR JSON。
pub struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    fn render(&self, bytes: &[u8], filename: &str) -> Result<RenderedDoc, String> {
        let text =
            std::str::from_utf8(bytes).map_err(|e| format!("文件不是合法的 UTF-8 编码：{e}"))?;
        // max_width 仅影响表格分隔行预处理逻辑，传一个足够大的值即可
        let doc = crate::markdown::parser::parse_markdown(text, 120);
        let payload =
            serde_json::to_value(&doc).map_err(|e| format!("Markdown IR 序列化失败：{e}"))?;
        Ok(RenderedDoc {
            filename: filename.to_string(),
            kind: DocKind::Markdown,
            payload,
        })
    }
}

/// 纯文本兜底渲染器：原文打包为 `{ "text": "..." }`。
pub struct PlainTextRenderer;

impl Renderer for PlainTextRenderer {
    fn render(&self, bytes: &[u8], filename: &str) -> Result<RenderedDoc, String> {
        // 非 UTF-8 时使用 lossy 转换，避免直接报错（如二进制文件用户也可能误用）
        let text = String::from_utf8_lossy(bytes).into_owned();
        let payload = serde_json::json!({ "text": text });
        Ok(RenderedDoc {
            filename: filename.to_string(),
            kind: DocKind::PlainText,
            payload,
        })
    }
}

/// 根据文件扩展名选择渲染器。
pub fn pick_renderer(path: &Path) -> Box<dyn Renderer> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" => Box::new(MarkdownRenderer),
        _ => Box::new(PlainTextRenderer),
    }
}
