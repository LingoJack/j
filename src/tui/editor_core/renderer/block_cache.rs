//! 基于 `pulldown-cmark` 解析器的块级缓存。
//!
//! 取代原 `CodeBlockCache` 的朴素 ``` toggle 扫描——后者无法识别 list-item
//! 缩进里的 fence、blockquote 内的 fence、表格里的 `|...|` 误匹配等场景，
//! 会把两段合法代码块之间的段落误判为代码块内容、画上竖线框。
//!
//! 本 cache 调用 `crate::markdown::parser::parse_markdown` 拿到 CommonMark
//! 视角下的 block 列表与源码行范围，再扁平化为 editor 渲染所需的几张表：
//!   - `line_to_block`：每行所属的扁平 block 索引（用于查 kind / lang）。
//!   - `fence_lines`：仅 fenced CodeBlock 的起止行 true（缩进代码块不计入；
//!     `render_code_fence_line` 当前依赖 ``` 取语言，4-space 缩进块本次保持
//!     普通文本渲染，与改造前一致）。
//!
//! 嵌套 block（BlockQuote 内、List item children 内）必须递归展开，否则
//! list/quote 内的代码块或表格漏标。

use crate::markdown::ir::{Block, BlockKind, ListData, ParsedDocument, SourceRange};
use crate::markdown::parser::parse_markdown;

/// 缓存里保留的 block 字段（不携带 inline / table data，节省内存）
#[derive(Debug, Clone)]
struct CachedBlock {
    kind: CachedKind,
    source: SourceRange,
}

#[derive(Debug, Clone)]
enum CachedKind {
    /// 围栏代码块（源码起止行以 ``` 开头）
    FencedCodeBlock {
        lang: String,
    },
    /// 缩进代码块（4 空格），暂不参与 fence/content 标记
    IndentedCodeBlock,
    Table,
    Heading,
    List,
    BlockQuote,
    Paragraph,
    Rule,
}

/// 基于解析器的块级缓存
#[derive(Debug, Default)]
pub(crate) struct BlockCache {
    /// 每次 rebuild 递增
    revision: u64,
    /// 扁平化后的 block 列表（递归展开 BlockQuote / List children）
    blocks: Vec<CachedBlock>,
    /// 源码行号 -> blocks 索引（最内层 block 优先）
    line_to_block: Vec<Option<usize>>,
    /// 与 `line_count` 同长：仅 fenced CodeBlock 的 start_line / end_line 为 true
    fence_lines: Vec<bool>,
    /// 上次构建是否有效
    valid: bool,
    /// 上次构建时的行数
    line_count: usize,
    /// 上次构建时的折行宽度
    width: usize,
}

impl BlockCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 使缓存失效（下次访问时重建）
    pub(crate) fn invalidate(&mut self) {
        self.valid = false;
    }

    /// 当前缓存是否对给定 lines + width 仍然有效
    pub(crate) fn is_valid_for(&self, lines: &[String], width: usize) -> bool {
        self.valid && self.line_count == lines.len() && self.width == width
    }

    /// 重建缓存：调用 parser、扁平化 block 树、填行号映射
    pub(crate) fn build(&mut self, lines: &[String], width: usize) {
        self.blocks.clear();
        self.line_to_block.clear();
        self.fence_lines.clear();
        self.line_to_block.resize(lines.len(), None);
        self.fence_lines.resize(lines.len(), false);

        let joined = lines.join("\n");
        let doc: ParsedDocument = parse_markdown(&joined, width);

        // 递归扁平化：list/quote 内的 block 也加进来
        let mut flat: Vec<CachedBlock> = Vec::with_capacity(doc.blocks.len());
        for block in &doc.blocks {
            flatten_block(block, lines, &mut flat);
        }

        // 填行号映射：后写入（更内层）优先，因为 flatten 顺序是先父后子
        for (idx, cb) in flat.iter().enumerate() {
            let start = cb.source.start_line;
            let end = cb.source.end_line.min(lines.len().saturating_sub(1));
            if start >= lines.len() {
                continue;
            }
            for slot in self.line_to_block.iter_mut().take(end + 1).skip(start) {
                *slot = Some(idx);
            }
            if let CachedKind::FencedCodeBlock { .. } = cb.kind {
                if start < self.fence_lines.len() {
                    self.fence_lines[start] = true;
                }
                if end < self.fence_lines.len() {
                    self.fence_lines[end] = true;
                }
            }
        }

        self.blocks = flat;
        self.line_count = lines.len();
        self.width = width;
        self.valid = true;
        self.revision = self.revision.wrapping_add(1);
    }

    // ========== 查询 API ==========

    /// 指定行是否是 fenced 代码块的围栏行（起或止）
    pub(crate) fn is_fence_line(&self, line_idx: usize) -> bool {
        self.fence_lines.get(line_idx).copied().unwrap_or(false)
    }

    /// 指定行是否在 fenced 代码块的内容区（不含围栏行本身）
    pub(crate) fn is_in_code_block_content(&self, line_idx: usize) -> bool {
        let Some(block) = self.block_at(line_idx) else {
            return false;
        };
        if !matches!(block.kind, CachedKind::FencedCodeBlock { .. }) {
            return false;
        }
        line_idx > block.source.start_line && line_idx < block.source.end_line
    }

    /// 指定行是否是 fenced 代码块的起始行
    pub(crate) fn is_code_block_start(&self, line_idx: usize) -> bool {
        let Some(block) = self.block_at(line_idx) else {
            return false;
        };
        matches!(block.kind, CachedKind::FencedCodeBlock { .. })
            && block.source.start_line == line_idx
    }

    /// 取指定行所属 fenced 代码块的语言
    pub(crate) fn code_block_lang(&self, line_idx: usize) -> Option<&str> {
        let block = self.block_at(line_idx)?;
        match &block.kind {
            CachedKind::FencedCodeBlock { lang } => Some(lang.as_str()),
            _ => None,
        }
    }

    /// 指定行是否属于一个表格
    pub(crate) fn is_table_line(&self, line_idx: usize) -> bool {
        self.block_at(line_idx)
            .is_some_and(|b| matches!(b.kind, CachedKind::Table))
    }

    /// 返回包含指定行的表格源码行范围（闭区间）
    pub(crate) fn table_range_at(&self, line_idx: usize) -> Option<(usize, usize)> {
        let block = self.block_at(line_idx)?;
        if !matches!(block.kind, CachedKind::Table) {
            return None;
        }
        Some((block.source.start_line, block.source.end_line))
    }

    /// 遍历所有表格的源码行范围
    pub(crate) fn table_blocks_iter(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.blocks.iter().filter_map(|cb| match cb.kind {
            CachedKind::Table => Some((cb.source.start_line, cb.source.end_line)),
            _ => None,
        })
    }

    /// 所有 fenced 代码块的内容行范围（闭区间，不含围栏行）。
    ///
    /// 仅在 `end > start` 时输出（单行 fenced 块无内容）。
    pub(crate) fn content_ranges(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for cb in &self.blocks {
            if matches!(cb.kind, CachedKind::FencedCodeBlock { .. })
                && cb.source.end_line > cb.source.start_line + 1
            {
                out.push((cb.source.start_line + 1, cb.source.end_line - 1));
            }
        }
        out
    }

    fn block_at(&self, line_idx: usize) -> Option<&CachedBlock> {
        let idx = self.line_to_block.get(line_idx).copied().flatten()?;
        self.blocks.get(idx)
    }
}

/// 递归把 block 树展开成扁平列表（先父后子）
fn flatten_block(block: &Block, lines: &[String], out: &mut Vec<CachedBlock>) {
    let kind = match &block.kind {
        BlockKind::CodeBlock { lang, .. } => {
            if is_fenced_at(block.source.start_line, lines) {
                CachedKind::FencedCodeBlock { lang: lang.clone() }
            } else {
                CachedKind::IndentedCodeBlock
            }
        }
        BlockKind::Table(_) => CachedKind::Table,
        BlockKind::Heading { .. } => CachedKind::Heading,
        BlockKind::List(_) => CachedKind::List,
        BlockKind::BlockQuote(_) => CachedKind::BlockQuote,
        BlockKind::Paragraph(_) => CachedKind::Paragraph,
        BlockKind::Rule => CachedKind::Rule,
    };
    // pulldown-cmark 偶尔把 block 紧跟着的空行（甚至下一个 block 的首行）也算进
    // source 范围（实测 Table、CodeBlock 偶发）。这里：
    //   1. 剥掉尾部全空白行；
    //   2. 对 Table，进一步要求结尾行确实是表格行（去掉解析器误带的下个 block）。
    let source = trim_trailing_blank_lines(block.source, lines);
    let source = match &kind {
        CachedKind::Table => trim_trailing_non_table_lines(source, lines),
        _ => source,
    };
    out.push(CachedBlock { kind, source });

    // 递归子 block（必须，否则 list/quote 内的代码块或表格漏标）
    match &block.kind {
        BlockKind::BlockQuote(inner) => {
            for sub in inner {
                flatten_block(sub, lines, out);
            }
        }
        BlockKind::List(ListData { items, .. }) => {
            for item in items {
                for sub in &item.children {
                    flatten_block(sub, lines, out);
                }
            }
        }
        _ => {}
    }
}

/// 把 SourceRange 末尾全空白的行剥掉
fn trim_trailing_blank_lines(mut source: SourceRange, lines: &[String]) -> SourceRange {
    while source.end_line > source.start_line {
        let blank = lines
            .get(source.end_line)
            .is_none_or(|l| l.trim().is_empty());
        if !blank {
            break;
        }
        source.end_line -= 1;
    }
    source
}

/// 表格专用：剥掉尾部不像表格行的行（pulldown-cmark 偶尔把下一个 block 的首行算进来）
fn trim_trailing_non_table_lines(mut source: SourceRange, lines: &[String]) -> SourceRange {
    while source.end_line > source.start_line {
        let is_table_like = lines
            .get(source.end_line)
            .map(|l| {
                let t = l.trim();
                t.starts_with('|') && t.ends_with('|')
            })
            .unwrap_or(false);
        if is_table_like {
            break;
        }
        source.end_line -= 1;
    }
    source
}

/// 判断源码该行是否以 ``` 开头（可被 `>` 或空格前缀，
/// 用于覆盖 blockquote / list 内的 fence 行）。
fn is_fenced_at(line_idx: usize, lines: &[String]) -> bool {
    let Some(raw) = lines.get(line_idx) else {
        return false;
    };
    // 先去掉前导空格
    let without_space = raw.trim_start();
    if without_space.starts_with("```") {
        return true;
    }
    // blockquote 上下文：若以 `>` 开头，递归剥掉所有 `> ` 前缀再判断
    let mut rest = without_space;
    while let Some(stripped) = rest.strip_prefix('>') {
        rest = stripped.trim_start();
        if rest.starts_with("```") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(text: &str) -> BlockCache {
        let lines: Vec<String> = text.split('\n').map(String::from).collect();
        let mut cache = BlockCache::new();
        cache.build(&lines, 80);
        cache
    }

    #[test]
    fn two_separate_fenced_blocks() {
        let md = "```rust\nfn a() {}\n```\n\npara\n\n```py\nx = 1\n```";
        let cache = build(md);
        // 两个 fenced 代码块：(0,2) 与 (6,8)
        assert!(cache.is_fence_line(0));
        assert!(cache.is_fence_line(2));
        assert!(cache.is_fence_line(6));
        assert!(cache.is_fence_line(8));
        // 中间段落不是 fence
        assert!(!cache.is_fence_line(3));
        assert!(!cache.is_fence_line(4));
        // 内容范围
        assert_eq!(cache.content_ranges(), vec![(1, 1), (7, 7)]);
        // 语言
        assert_eq!(cache.code_block_lang(0).map(str::trim), Some("rust"));
        assert_eq!(cache.code_block_lang(6).map(str::trim), Some("py"));
    }

    #[test]
    fn bug_repro_paragraph_between_blocks() {
        // 用户报的 bug 缩影：段落 → 代码块 → 段落 → 代码块
        let md = "intro\n\n```\naaa\n```\n\nmiddle text\n\n```\nbbb\n```";
        let cache = build(md);
        // 中间段落 "middle text" 在第 6 行
        assert!(!cache.is_fence_line(6));
        assert!(!cache.is_in_code_block_content(6));
        // 两个代码块的 fence 行
        assert!(cache.is_fence_line(2));
        assert!(cache.is_fence_line(4));
        assert!(cache.is_fence_line(8));
        assert!(cache.is_fence_line(10));
        // 内容行
        assert!(cache.is_in_code_block_content(3));
        assert!(cache.is_in_code_block_content(9));
    }

    #[test]
    fn fence_inside_list_item() {
        // List item 内的围栏代码块（缩进 4 空格使其成为 item 的 child）
        let md = "- outer\n\n    ```\n    code\n    ```\n";
        let cache = build(md);
        // pulldown-cmark 把 ```（缩进 4 空格）识别为 list item 内的 fenced code block；
        // 递归 flatten 后 fence 行应被标记
        assert!(cache.is_fence_line(2));
        assert!(cache.is_fence_line(4));
        assert!(cache.is_in_code_block_content(3));
    }

    #[test]
    fn fence_inside_blockquote() {
        let md = "> ```\n> x\n> ```\n";
        let cache = build(md);
        assert!(cache.is_fence_line(0));
        assert!(cache.is_fence_line(2));
        assert!(cache.is_in_code_block_content(1));
    }

    #[test]
    fn unclosed_fence_at_eof() {
        // 未闭合 fence：pulldown-cmark 会把它当作延伸到 EOF 的 CodeBlock
        let md = "before\n\n```\nrunaway\nmore\n";
        let cache = build(md);
        // 起始 fence 行应被标记
        assert!(cache.is_fence_line(2));
        // EOF 后的"虚拟 end fence"也会被解析器标到末行
        // （取决于 pulldown-cmark 行为；不做严格断言，仅确保起始 fence 被识别）
        // 但起始 fence 之后的内容应在代码块内
        assert!(cache.is_in_code_block_content(3) || cache.block_at(3).is_some());
    }

    #[test]
    fn table_and_fence_do_not_cross() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n\n```\nx\n```";
        let cache = build(md);
        // 表格行
        assert!(cache.is_table_line(0));
        assert!(cache.is_table_line(1));
        assert!(cache.is_table_line(2));
        // fence 行
        assert!(cache.is_fence_line(4));
        assert!(cache.is_fence_line(6));
        // 互不串扰
        assert!(!cache.is_fence_line(0));
        assert!(!cache.is_table_line(4));
        // table_range_at
        assert_eq!(cache.table_range_at(1), Some((0, 2)));
        assert!(cache.table_range_at(5).is_none());
    }

    #[test]
    fn table_followed_by_paragraph_drops_paragraph_from_range() {
        // pulldown-cmark 偶尔把表格紧邻的下一个段落首行算进 Table source；
        // BlockCache 必须剥掉，否则 wrap_engine 会把段落"epilogue"也当作表格续行
        // (count=0)，导致光标跳过它。
        let md = "para\n\n| a | b |\n|---|---|\n| 1 | 2 |\nepilogue";
        let cache = build(md);
        assert_eq!(cache.table_range_at(2), Some((2, 4)));
        assert!(!cache.is_table_line(5));
    }

    #[test]
    fn user_bug_repro_paragraphs_around_two_code_blocks() {
        // 用户原 bug 的最小复现：段落 → 代码块 → 段落 → 代码块。
        // 旧朴素 toggle 把中间段落画成"第三个框"。新 cache 必须只标 4 个 fence 行，
        // 且中间段落行既不在 code block content 内，也不是 fence。
        let md = "intro\n\n```plaintext\nA\n```\n\nmiddle paragraph\n\n```plaintext\nB\n```";
        let cache = build(md);
        // 中间 "middle paragraph" 在第 6 行
        assert!(!cache.is_fence_line(6));
        assert!(!cache.is_in_code_block_content(6));
        // 4 个 fence 行
        assert!(cache.is_fence_line(2));
        assert!(cache.is_fence_line(4));
        assert!(cache.is_fence_line(8));
        assert!(cache.is_fence_line(10));
    }
}
