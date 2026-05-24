mod dicts;

use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use crate::tui::editor_core::EditorTheme;

/// 语法高亮样式集合
pub struct SyntaxStyles {
    pub default_style: Style,
    pub kw_style: Style,
    pub str_style: Style,
    pub num_style: Style,
    pub type_style: Style,
    pub primitive_style: Style,
    pub macro_style: Style,
}

/// 语法关键字字典
pub struct SyntaxDicts<'a> {
    pub keywords: &'a [&'a str],
    pub primitive_types: &'a [&'a str],
    pub go_type_names: &'a [&'a str],
}

/// 解析过程中的共享上下文，减少 handle_* 函数的参数数量。
struct ParseContext<'a, 'b> {
    chars: &'a mut std::iter::Peekable<std::str::Chars<'b>>,
    buf: &'a mut String,
    spans: &'a mut Vec<Span<'static>>,
    dicts: &'a SyntaxDicts<'a>,
    styles: &'a SyntaxStyles,
    lang: &'a str,
    theme: &'a EditorTheme,
}

/// 简单的代码语法高亮（无需外部依赖）
///
/// 根据语言类型对常见关键字、字符串、注释、数字进行着色。
pub fn highlight_code_line(line: &str, lang: &str, theme: &EditorTheme) -> Vec<Span<'static>> {
    let lang_lower = lang.to_lowercase();
    let lang_str = lang_lower.as_str();

    let keywords = dicts::keywords_for_lang(lang_str);
    let primitive_types = dicts::primitive_types_for_lang(lang_str);
    let go_type_names = dicts::go_type_names_for_lang(lang_str);
    let comment_prefix = dicts::comment_prefix_for_lang(lang_str);

    // ===== 代码高亮配色方案（基于主题）=====
    let styles = SyntaxStyles {
        default_style: Style::default().fg(theme.code_default),
        kw_style: Style::default().fg(theme.code_keyword),
        str_style: Style::default().fg(theme.code_string),
        num_style: Style::default().fg(theme.code_number),
        type_style: Style::default().fg(theme.code_type),
        primitive_style: Style::default().fg(theme.code_primitive),
        macro_style: Style::default().fg(theme.code_macro),
    };
    let comment_style = Style::default()
        .fg(theme.code_comment)
        .add_modifier(Modifier::ITALIC);

    let dicts = SyntaxDicts {
        keywords,
        primitive_types,
        go_type_names,
    };

    let trimmed = line.trim_start();

    // 注释行
    if trimmed.starts_with(comment_prefix) {
        return vec![Span::styled(line.to_string(), comment_style)];
    }

    // 逐词解析
    let mut spans = Vec::new();
    let mut chars = line.chars().peekable();
    let mut buf = String::new();

    let mut ctx = ParseContext {
        chars: &mut chars,
        buf: &mut buf,
        spans: &mut spans,
        dicts: &dicts,
        styles: &styles,
        lang: &lang_lower,
        theme,
    };

    while let Some(&ch) = ctx.chars.peek() {
        // --- YAML 专用高亮（优先级最高）---
        if handle_yaml_document_marker(ch, &mut ctx) {
            continue;
        }
        if handle_yaml_tag(ch, &mut ctx) {
            continue;
        }
        if handle_yaml_anchor(ch, &mut ctx) {
            continue;
        }
        if handle_yaml_block_scalar(ch, &mut ctx) {
            continue;
        }
        if handle_yaml_list_indicator(ch, &mut ctx) {
            continue;
        }
        if handle_yaml_key(ch, &mut ctx) {
            continue;
        }
        // --- 通用高亮 ---
        if handle_double_quote(ch, &mut ctx) {
            continue;
        }
        if handle_backtick(ch, &mut ctx) {
            continue;
        }
        if handle_rust_lifetime(ch, &mut ctx) {
            continue;
        }
        if handle_single_quote(ch, &mut ctx) {
            continue;
        }
        if handle_rust_attribute(ch, &mut ctx) {
            continue;
        }
        if handle_shell_variable(ch, &mut ctx) {
            continue;
        }
        if handle_inline_comment(ch, &mut ctx, comment_prefix, comment_style) {
            continue;
        }
        ctx.buf.push(ch);
        ctx.chars.next();
    }

    if !ctx.buf.is_empty() {
        ctx.spans
            .extend(colorize_tokens(ctx.buf, ctx.dicts, ctx.styles, ctx.lang));
    }

    if ctx.spans.is_empty() {
        ctx.spans
            .push(Span::styled(line.to_string(), ctx.styles.default_style));
    }

    spans
}

/// 刷新 buf 中累积的普通文本为着色 token。
fn flush_buf(ctx: &mut ParseContext<'_, '_>) {
    if !ctx.buf.is_empty() {
        ctx.spans
            .extend(colorize_tokens(ctx.buf, ctx.dicts, ctx.styles, ctx.lang));
        ctx.buf.clear();
    }
}

/// 处理双引号字符串（支持 `\` 转义）。
fn handle_double_quote(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '"' {
        return false;
    }
    flush_buf(ctx);
    let mut s = String::new();
    s.push(ch);
    ctx.chars.next();
    let mut escaped = false;
    while let Some(&c) = ctx.chars.peek() {
        s.push(c);
        ctx.chars.next();
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            break;
        }
    }
    ctx.spans.push(Span::styled(s, ctx.styles.str_style));
    true
}

/// 处理反引号字符串（不支持转义，遇到配对反引号结束）。
fn handle_backtick(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '`' {
        return false;
    }
    flush_buf(ctx);
    let mut s = String::new();
    s.push(ch);
    ctx.chars.next();
    while let Some(&c) = ctx.chars.peek() {
        s.push(c);
        ctx.chars.next();
        if c == '`' {
            break;
        }
    }
    ctx.spans.push(Span::styled(s, ctx.styles.str_style));
    true
}

/// 处理 Rust 生命周期参数 (`'a`, `'static` 等) vs 字符字面量 (`'x'`)。
fn handle_rust_lifetime(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '\'' || !matches!(ctx.lang, "rust" | "rs") {
        return false;
    }
    flush_buf(ctx);
    let mut s = String::new();
    s.push(ch);
    ctx.chars.next();
    let mut is_lifetime = false;
    while let Some(&c) = ctx.chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            s.push(c);
            ctx.chars.next();
        } else if c == '\'' && s.len() == 2 {
            s.push(c);
            ctx.chars.next();
            break;
        } else {
            is_lifetime = true;
            break;
        }
    }
    if is_lifetime || (s.len() > 1 && !s.ends_with('\'')) {
        let lifetime_style = Style::default().fg(ctx.theme.code_lifetime);
        ctx.spans.push(Span::styled(s, lifetime_style));
    } else {
        ctx.spans.push(Span::styled(s, ctx.styles.str_style));
    }
    true
}

/// 处理非 Rust 语言的字符串（包含单引号）。
fn handle_single_quote(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '\'' || matches!(ctx.lang, "rust" | "rs") {
        return false;
    }
    flush_buf(ctx);
    let mut s = String::new();
    s.push(ch);
    ctx.chars.next();
    let mut escaped = false;
    while let Some(&c) = ctx.chars.peek() {
        s.push(c);
        ctx.chars.next();
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '\'' {
            break;
        }
    }
    ctx.spans.push(Span::styled(s, ctx.styles.str_style));
    true
}

/// 处理 Rust 属性 (`#[...]` 或 `#![...]`)。
fn handle_rust_attribute(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '#' || !matches!(ctx.lang, "rust" | "rs") {
        return false;
    }
    let mut lookahead = ctx.chars.clone();
    if let Some(next) = lookahead.next() {
        if next != '[' {
            return false;
        }
    } else {
        return false;
    }
    flush_buf(ctx);
    let mut attr = String::new();
    attr.push(ch);
    ctx.chars.next();
    let mut depth = 0;
    while let Some(&c) = ctx.chars.peek() {
        attr.push(c);
        ctx.chars.next();
        if c == '[' {
            depth += 1;
        } else if c == ']' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
    }
    let attr_style = Style::default().fg(ctx.theme.code_attribute);
    ctx.spans.push(Span::styled(attr, attr_style));
    true
}

/// 处理 Shell 变量 (`$VAR`, `${VAR}`, `$1` 等)。
fn handle_shell_variable(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if ch != '$'
        || !matches!(
            ctx.lang,
            "sh" | "bash" | "zsh" | "shell" | "dockerfile" | "docker"
        )
    {
        return false;
    }
    flush_buf(ctx);
    let var_style = Style::default().fg(ctx.theme.code_shell_var);
    let mut var = String::new();
    var.push(ch);
    ctx.chars.next();
    if let Some(&next_ch) = ctx.chars.peek() {
        if next_ch == '{' {
            var.push(next_ch);
            ctx.chars.next();
            while let Some(&c) = ctx.chars.peek() {
                var.push(c);
                ctx.chars.next();
                if c == '}' {
                    break;
                }
            }
        } else if next_ch == '(' {
            var.push(next_ch);
            ctx.chars.next();
            let mut depth = 1;
            while let Some(&c) = ctx.chars.peek() {
                var.push(c);
                ctx.chars.next();
                if c == '(' {
                    depth += 1;
                }
                if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
        } else if next_ch.is_alphanumeric()
            || next_ch == '_'
            || next_ch == '@'
            || next_ch == '#'
            || next_ch == '?'
            || next_ch == '!'
        {
            while let Some(&c) = ctx.chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    var.push(c);
                    ctx.chars.next();
                } else {
                    break;
                }
            }
        }
    }
    ctx.spans.push(Span::styled(var, var_style));
    true
}

/// 处理行内注释检测。
fn handle_inline_comment(
    ch: char,
    ctx: &mut ParseContext<'_, '_>,
    comment_prefix: &str,
    comment_style: Style,
) -> bool {
    if ch != '/' && ch != '#' && ch != '-' {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    if !rest.starts_with(comment_prefix) {
        return false;
    }
    flush_buf(ctx);
    while ctx.chars.peek().is_some() {
        ctx.chars.next();
    }
    ctx.spans.push(Span::styled(rest, comment_style));
    true
}

// ---------------------------------------------------------------------------
// YAML 专用高亮处理
// ---------------------------------------------------------------------------

/// 处理 YAML 文档分隔符 `---` 和 `...`
fn handle_yaml_document_marker(_ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    if rest.starts_with("---") || rest.starts_with("...") {
        flush_buf(ctx);
        let marker = if rest.starts_with("---") {
            "---"
        } else {
            "..."
        };
        for _ in 0..marker.len() {
            ctx.chars.next();
        }
        ctx.spans
            .push(Span::styled(marker.to_string(), ctx.styles.kw_style));
        return true;
    }
    false
}

/// 处理 YAML 类型标签 `!!str`, `!!int`, `!!bool` 等
fn handle_yaml_tag(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || ch != '!' {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    if !rest.starts_with("!!") {
        return false;
    }
    flush_buf(ctx);
    let mut tag = String::new();
    tag.push('!');
    ctx.chars.next();
    tag.push('!');
    ctx.chars.next();
    while let Some(&c) = ctx.chars.peek() {
        if c.is_alphanumeric() {
            tag.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    let tag_style = Style::default()
        .fg(ctx.theme.code_comment)
        .add_modifier(Modifier::ITALIC);
    ctx.spans.push(Span::styled(tag, tag_style));
    true
}

/// 处理 YAML 锚点 `&anchor`、别名 `*alias`、合并键 `<<`
fn handle_yaml_anchor(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") {
        return false;
    }
    // 处理 `<<` 合并键（后面可能有 `:`）
    let rest: String = ctx.chars.clone().collect();
    if rest.starts_with("<<") {
        flush_buf(ctx);
        ctx.chars.next();
        ctx.chars.next();
        let anchor_style = Style::default().fg(ctx.theme.code_attribute);
        ctx.spans.push(Span::styled("<<", anchor_style));
        return true;
    }
    // 处理 `&anchor` 或 `*alias`
    if ch != '&' && ch != '*' {
        return false;
    }
    flush_buf(ctx);
    let mut anchor = String::new();
    anchor.push(ch);
    ctx.chars.next();
    while let Some(&c) = ctx.chars.peek() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            anchor.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    let anchor_style = Style::default().fg(ctx.theme.code_attribute);
    ctx.spans.push(Span::styled(anchor, anchor_style));
    true
}

/// 处理 YAML 块标量指示符 `|` 或 `>` 及其修饰符
fn handle_yaml_block_scalar(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || (ch != '|' && ch != '>') {
        return false;
    }
    flush_buf(ctx);
    let mut indicator = String::new();
    indicator.push(ch);
    ctx.chars.next();
    // 可选修饰符：`-`（strip）、`+`（keep）、数字（缩进）
    while let Some(&c) = ctx.chars.peek() {
        if c == '-' || c == '+' || c.is_ascii_digit() {
            indicator.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    ctx.spans
        .push(Span::styled(indicator, ctx.styles.macro_style));
    true
}

/// 处理 YAML 键名（冒号前的标识符）
///
/// YAML 键名格式：`key:` 或 `key :`（冒号后通常是空格或换行）
/// 检测逻辑：标识符后紧跟冒号，且冒号后是空格/换行/结束
fn handle_yaml_key(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") {
        return false;
    }
    // 只处理标识符起始字符
    if !ch.is_alphanumeric() && ch != '_' {
        return false;
    }
    // 检查后续是否形成键名模式
    let rest: String = ctx.chars.clone().collect();
    // 检查是否后面有冒号（键名结束）
    if let Some(colon_pos) = rest.find(':') {
        let before_colon = &rest[..colon_pos];
        // 确保冒号前只有合法的键名字符（字母、数字、下划线、连字符、点）
        if !before_colon
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return false;
        }
        // 检查冒号后是否是有效的值分隔（空格、换行、结束）
        let after_colon = &rest[colon_pos + 1..];
        let is_key = after_colon.is_empty()
            || after_colon.starts_with(' ')
            || after_colon.starts_with('\n')
            || after_colon.starts_with('\t');
        if !is_key {
            return false;
        }
        // 确认是键名：刷新 buf，读取键名部分
        flush_buf(ctx);
        let mut key = String::new();
        while let Some(&c) = ctx.chars.peek() {
            if c == ':' {
                ctx.chars.next();
                break;
            }
            key.push(c);
            ctx.chars.next();
        }
        // 键名用 type 风格，冒号用 default
        ctx.spans.push(Span::styled(key, ctx.styles.type_style));
        ctx.spans.push(Span::styled(":", ctx.styles.default_style));
        return true;
    }
    false
}

/// 处理 YAML 列表指示符 `-`
///
/// 只在行首（或仅有缩进空格后）识别为列表指示符。
/// 检测条件：buf 为空（意味着刚处理完行首空格），且后续是空格或内容。
fn handle_yaml_list_indicator(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || ch != '-' {
        return false;
    }
    // 只在 buf 为空时识别（即行首或仅有缩进空格后）
    if !ctx.buf.is_empty() {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    // 列表指示符后面必须是空格或内容（不能是 `---` 文档分隔符）
    if rest.starts_with("---") {
        return false;
    }
    // 检查 `-` 后面是否是空格（标准列表格式）或直接是内容
    let mut lookahead = ctx.chars.clone();
    lookahead.next(); // skip '-'
    if let Some(&after_dash) = lookahead.peek() {
        // `- ` 或 `-word` 都是有效列表项
        if after_dash == ' '
            || after_dash.is_alphanumeric()
            || after_dash == '"'
            || after_dash == '\''
        {
            flush_buf(ctx);
            ctx.chars.next(); // consume '-'
            ctx.spans.push(Span::styled("-", ctx.styles.kw_style));
            return true;
        }
    }
    false
}

/// 将文本按照 word boundary 拆分并对关键字、数字、类型名、原始类型着色
pub fn colorize_tokens(
    text: &str,
    dicts: &SyntaxDicts<'_>,
    styles: &SyntaxStyles,
    lang: &str,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_word = String::new();
    let mut current_non_word = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() || ch == '_' {
            if !current_non_word.is_empty() {
                spans.push(Span::styled(current_non_word.clone(), styles.default_style));
                current_non_word.clear();
            }
            current_word.push(ch);
        } else {
            // Rust 宏调用高亮：word! 或 word!()
            if ch == '!' && matches!(lang, "rust" | "rs") && !current_word.is_empty() {
                let is_macro = chars
                    .peek()
                    .map(|&c| c == '(' || c == '{' || c == '[' || c.is_whitespace())
                    .unwrap_or(true);
                if is_macro {
                    spans.push(Span::styled(current_word.clone(), styles.macro_style));
                    current_word.clear();
                    spans.push(Span::styled("!".to_string(), styles.macro_style));
                    continue;
                }
            }
            if !current_word.is_empty() {
                let style = classify_word(&current_word, dicts, styles, lang);
                spans.push(Span::styled(current_word.clone(), style));
                current_word.clear();
            }
            current_non_word.push(ch);
        }
    }

    // 刷新剩余
    if !current_non_word.is_empty() {
        spans.push(Span::styled(current_non_word, styles.default_style));
    }
    if !current_word.is_empty() {
        let style = classify_word(&current_word, dicts, styles, lang);
        spans.push(Span::styled(current_word, style));
    }

    spans
}

/// 根据语言规则判断一个 word 应该使用哪种颜色样式
pub fn classify_word(
    word: &str,
    dicts: &SyntaxDicts<'_>,
    styles: &SyntaxStyles,
    lang: &str,
) -> Style {
    if dicts.keywords.contains(&word) {
        styles.kw_style
    } else if dicts.primitive_types.contains(&word) {
        styles.primitive_style
    } else if word
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        styles.num_style
    } else if matches!(lang, "go" | "golang") {
        if dicts.go_type_names.contains(&word) {
            styles.type_style
        } else {
            styles.default_style
        }
    } else if word
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        styles.type_style
    } else {
        styles.default_style
    }
}
