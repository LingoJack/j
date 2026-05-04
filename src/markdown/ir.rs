//! Markdown 中间表示（IR）类型定义
//!
//! Parser 输出 `ParsedDocument`，Render 基于 IR 生成 `Vec<Line>`。
//! IR 与终端宽度、主题无关，可被多次渲染（不同宽度/主题）。

/// 源码位置范围
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SourceRange {
    /// 源码起始行号（0-based）
    pub start_line: usize,
    /// 源码结束行号（0-based, inclusive）
    pub end_line: usize,
}

/// 解析后的文档
#[derive(Debug, Clone, Default)]
pub struct ParsedDocument {
    /// 文档中的 block 级元素
    pub blocks: Vec<Block>,
    /// 源码行号 -> block 索引的映射（用于 editor 侧快速定位）
    #[allow(dead_code)]
    pub line_to_block: Vec<Option<usize>>,
}

/// Block 级元素
#[derive(Debug, Clone)]
pub struct Block {
    /// 源码位置（后续 Step 实现精确映射）
    #[allow(dead_code)]
    pub source: SourceRange,
    /// Block 类型
    pub kind: BlockKind,
}

/// Block 类型枚举
#[derive(Debug, Clone)]
pub enum BlockKind {
    /// 普通段落
    Paragraph(Vec<Inline>),
    /// 标题（level: 1-6）
    Heading { level: u8, content: Vec<Inline> },
    /// 围栏代码块
    CodeBlock { lang: String, code: String },
    /// 表格
    Table(TableData),
    /// 列表
    List(ListData),
    /// 引用块（可嵌套）
    BlockQuote(Vec<Block>),
    /// 水平分隔线
    Rule,
}

/// 表格数据
#[derive(Debug, Clone)]
pub struct TableData {
    /// 每列对齐方式
    pub alignments: Vec<pulldown_cmark::Alignment>,
    /// 行数据：rows[row_idx][col_idx] = 单元格内的 inline 元素
    pub rows: Vec<Vec<Vec<Inline>>>,
}

/// 列表数据
#[derive(Debug, Clone)]
pub struct ListData {
    /// 是否为有序列表
    pub ordered: bool,
    /// 有序列表的起始序号
    pub start_index: Option<u64>,
    /// 列表项
    pub items: Vec<ListItem>,
}

/// 列表项
#[derive(Debug, Clone)]
pub struct ListItem {
    /// Task list 标记：`Some(true)` = `[x]`，`Some(false)` = `[ ]`，`None` = 非 task list
    pub checked: Option<bool>,
    /// 列表项的 inline 内容（item 自身的文本）
    pub content: Vec<Inline>,
    /// 嵌套 block（子列表、代码块、引用等），用于支持多级嵌套列表
    pub children: Vec<Block>,
}

/// Inline 级元素
#[derive(Debug, Clone)]
pub enum Inline {
    /// 普通文本
    Text(String),
    /// 加粗
    Strong(Vec<Inline>),
    /// 斜体
    Emphasis(Vec<Inline>),
    /// 删除线
    Strikethrough(Vec<Inline>),
    /// 行内代码
    Code(String),
    /// 链接
    Link { text: Vec<Inline>, url: String },
    /// 软换行（同段落内换行）
    SoftBreak,
    /// 硬换行（显式 `<br>` 或行尾 `\`）
    HardBreak,
}
