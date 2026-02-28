use super::super::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

/// 简单的代码语法高亮（无需外部依赖）
/// 根据语言类型对常见关键字、字符串、注释、数字进行着色
pub fn highlight_code_line<'a>(line: &'a str, lang: &str, theme: &Theme) -> Vec<Span<'static>> {
    let lang_lower = lang.to_lowercase();
    // Rust 使用多组词汇分别高亮
    // keywords: 控制流/定义关键字 → 紫色
    // primitive_types: 原始类型 → 青绿色
    // 其他类型名（大写开头）自动通过 type_style 高亮 → 暖黄色
    // 宏调用（word!）通过 macro_style 高亮 → 淡蓝色
    let keywords: &[&str] = match lang_lower.as_str() {
        "rust" | "rs" => &[
            // 控制流/定义关键字（紫色）
            "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "trait", "for",
            "while", "loop", "if", "else", "match", "return", "self", "Self", "where", "async",
            "await", "move", "ref", "type", "const", "static", "crate", "super", "as", "in",
            "true", "false", "unsafe", "extern", "dyn", "abstract", "become", "box", "do", "final",
            "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "union", "break",
            "continue",
        ],
        "python" | "py" => &[
            "def", "class", "return", "if", "elif", "else", "for", "while", "import", "from", "as",
            "with", "try", "except", "finally", "raise", "pass", "break", "continue", "yield",
            "lambda", "and", "or", "not", "in", "is", "True", "False", "None", "global",
            "nonlocal", "assert", "del", "async", "await", "self", "print",
        ],
        "javascript" | "js" | "typescript" | "ts" | "jsx" | "tsx" => &[
            "function",
            "const",
            "let",
            "var",
            "return",
            "if",
            "else",
            "for",
            "while",
            "class",
            "new",
            "this",
            "import",
            "export",
            "from",
            "default",
            "async",
            "await",
            "try",
            "catch",
            "finally",
            "throw",
            "typeof",
            "instanceof",
            "true",
            "false",
            "null",
            "undefined",
            "of",
            "in",
            "switch",
            "case",
        ],
        "go" | "golang" => &[
            "func",
            "package",
            "import",
            "return",
            "if",
            "else",
            "for",
            "range",
            "struct",
            "interface",
            "type",
            "var",
            "const",
            "defer",
            "go",
            "chan",
            "select",
            "case",
            "switch",
            "default",
            "break",
            "continue",
            "map",
            "true",
            "false",
            "nil",
            "make",
            "append",
            "len",
            "cap",
        ],
        "java" | "kotlin" | "kt" => &[
            "public",
            "private",
            "protected",
            "class",
            "interface",
            "extends",
            "implements",
            "return",
            "if",
            "else",
            "for",
            "while",
            "new",
            "this",
            "import",
            "package",
            "static",
            "final",
            "void",
            "int",
            "String",
            "boolean",
            "true",
            "false",
            "null",
            "try",
            "catch",
            "throw",
            "throws",
            "fun",
            "val",
            "var",
            "when",
            "object",
            "companion",
        ],
        "sh" | "bash" | "zsh" | "shell" => &[
            "if",
            "then",
            "else",
            "elif",
            "fi",
            "for",
            "while",
            "do",
            "done",
            "case",
            "esac",
            "function",
            "return",
            "exit",
            "echo",
            "export",
            "local",
            "readonly",
            "set",
            "unset",
            "shift",
            "source",
            "in",
            "true",
            "false",
            "read",
            "declare",
            "typeset",
            "trap",
            "eval",
            "exec",
            "test",
            "select",
            "until",
            "break",
            "continue",
            "printf",
            // Go 命令
            "go",
            "build",
            "run",
            "test",
            "fmt",
            "vet",
            "mod",
            "get",
            "install",
            "clean",
            "doc",
            "list",
            "version",
            "env",
            "generate",
            "tool",
            "proxy",
            "GOPATH",
            "GOROOT",
            "GOBIN",
            "GOMODCACHE",
            "GOPROXY",
            "GOSUMDB",
            // Cargo 命令
            "cargo",
            "new",
            "init",
            "add",
            "remove",
            "update",
            "check",
            "clippy",
            "rustfmt",
            "rustc",
            "rustup",
            "publish",
            "uninstall",
            "search",
            "tree",
            "locate_project",
            "metadata",
            "audit",
            "watch",
            "expand",
        ],
        "c" | "cpp" | "c++" | "h" | "hpp" => &[
            "int",
            "char",
            "float",
            "double",
            "void",
            "long",
            "short",
            "unsigned",
            "signed",
            "const",
            "static",
            "extern",
            "struct",
            "union",
            "enum",
            "typedef",
            "sizeof",
            "return",
            "if",
            "else",
            "for",
            "while",
            "do",
            "switch",
            "case",
            "break",
            "continue",
            "default",
            "goto",
            "auto",
            "register",
            "volatile",
            "class",
            "public",
            "private",
            "protected",
            "virtual",
            "override",
            "template",
            "namespace",
            "using",
            "new",
            "delete",
            "try",
            "catch",
            "throw",
            "nullptr",
            "true",
            "false",
            "this",
            "include",
            "define",
            "ifdef",
            "ifndef",
            "endif",
        ],
        "sql" => &[
            "SELECT",
            "FROM",
            "WHERE",
            "INSERT",
            "UPDATE",
            "DELETE",
            "CREATE",
            "DROP",
            "ALTER",
            "TABLE",
            "INDEX",
            "INTO",
            "VALUES",
            "SET",
            "AND",
            "OR",
            "NOT",
            "NULL",
            "JOIN",
            "LEFT",
            "RIGHT",
            "INNER",
            "OUTER",
            "ON",
            "GROUP",
            "BY",
            "ORDER",
            "ASC",
            "DESC",
            "HAVING",
            "LIMIT",
            "OFFSET",
            "UNION",
            "AS",
            "DISTINCT",
            "COUNT",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "LIKE",
            "IN",
            "BETWEEN",
            "EXISTS",
            "CASE",
            "WHEN",
            "THEN",
            "ELSE",
            "END",
            "BEGIN",
            "COMMIT",
            "ROLLBACK",
            "PRIMARY",
            "KEY",
            "FOREIGN",
            "REFERENCES",
            "select",
            "from",
            "where",
            "insert",
            "update",
            "delete",
            "create",
            "drop",
            "alter",
            "table",
            "index",
            "into",
            "values",
            "set",
            "and",
            "or",
            "not",
            "null",
            "join",
            "left",
            "right",
            "inner",
            "outer",
            "on",
            "group",
            "by",
            "order",
            "asc",
            "desc",
            "having",
            "limit",
            "offset",
            "union",
            "as",
            "distinct",
            "count",
            "sum",
            "avg",
            "min",
            "max",
            "like",
            "in",
            "between",
            "exists",
            "case",
            "when",
            "then",
            "else",
            "end",
            "begin",
            "commit",
            "rollback",
            "primary",
            "key",
            "foreign",
            "references",
        ],
        "yaml" | "yml" => &["true", "false", "null", "yes", "no", "on", "off"],
        "toml" => &[
            "true",
            "false",
            // Cargo.toml 常用
            "name",
            "version",
            "edition",
            "authors",
            "dependencies",
            "dev-dependencies",
            "build-dependencies",
            "features",
            "workspace",
            "members",
            "exclude",
            "include",
            "path",
            "git",
            "branch",
            "tag",
            "rev",
            "package",
            "lib",
            "bin",
            "example",
            "test",
            "bench",
            "doc",
            "profile",
            "release",
            "debug",
            "opt-level",
            "lto",
            "codegen-units",
            "panic",
            "strip",
            "default",
            "optional",
            // 常见配置项
            "repository",
            "homepage",
            "documentation",
            "license",
            "license-file",
            "keywords",
            "categories",
            "readme",
            "description",
            "resolver",
        ],
        "css" | "scss" | "less" => &[
            "color",
            "background",
            "border",
            "margin",
            "padding",
            "display",
            "position",
            "width",
            "height",
            "font",
            "text",
            "flex",
            "grid",
            "align",
            "justify",
            "important",
            "none",
            "auto",
            "inherit",
            "initial",
            "unset",
        ],
        "dockerfile" | "docker" => &[
            "FROM",
            "RUN",
            "CMD",
            "LABEL",
            "EXPOSE",
            "ENV",
            "ADD",
            "COPY",
            "ENTRYPOINT",
            "VOLUME",
            "USER",
            "WORKDIR",
            "ARG",
            "ONBUILD",
            "STOPSIGNAL",
            "HEALTHCHECK",
            "SHELL",
            "AS",
        ],
        "ruby" | "rb" => &[
            "def", "end", "class", "module", "if", "elsif", "else", "unless", "while", "until",
            "for", "do", "begin", "rescue", "ensure", "raise", "return", "yield", "require",
            "include", "attr", "self", "true", "false", "nil", "puts", "print",
        ],
        _ => &[
            "fn", "function", "def", "class", "return", "if", "else", "for", "while", "import",
            "export", "const", "let", "var", "true", "false", "null", "nil", "None", "self",
            "this",
        ],
    };

    // 原始/内建类型列表（青绿色）
    let primitive_types: &[&str] = match lang_lower.as_str() {
        "rust" | "rs" => &[
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64", "bool", "char", "str",
        ],
        "go" | "golang" => &[
            "int",
            "int8",
            "int16",
            "int32",
            "int64",
            "uint",
            "uint8",
            "uint16",
            "uint32",
            "uint64",
            "uintptr",
            "float32",
            "float64",
            "complex64",
            "complex128",
            "bool",
            "byte",
            "rune",
            "string",
            "error",
            "any",
        ],
        _ => &[],
    };

    // Go 语言显式类型名列表（暖黄色，因为 Go 的大写开头不代表类型）
    let go_type_names: &[&str] = match lang_lower.as_str() {
        "go" | "golang" => &[
            "Reader",
            "Writer",
            "Closer",
            "ReadWriter",
            "ReadCloser",
            "WriteCloser",
            "ReadWriteCloser",
            "Seeker",
            "Context",
            "Error",
            "Stringer",
            "Mutex",
            "RWMutex",
            "WaitGroup",
            "Once",
            "Pool",
            "Map",
            "Duration",
            "Time",
            "Timer",
            "Ticker",
            "Buffer",
            "Builder",
            "Request",
            "Response",
            "ResponseWriter",
            "Handler",
            "HandlerFunc",
            "Server",
            "Client",
            "Transport",
            "File",
            "FileInfo",
            "FileMode",
            "Decoder",
            "Encoder",
            "Marshaler",
            "Unmarshaler",
            "Logger",
            "Flag",
            "Regexp",
            "Conn",
            "Listener",
            "Addr",
            "Scanner",
            "Token",
            "Type",
            "Value",
            "Kind",
            "Cmd",
            "Signal",
        ],
        _ => &[],
    };

    let comment_prefix = match lang_lower.as_str() {
        "python" | "py" | "sh" | "bash" | "zsh" | "shell" | "ruby" | "rb" | "yaml" | "yml"
        | "toml" | "dockerfile" | "docker" => "#",
        "sql" => "--",
        "css" | "scss" | "less" => "/*",
        _ => "//",
    };

    // ===== 代码高亮配色方案（基于主题）=====
    let code_style = Style::default().fg(theme.code_default);
    let kw_style = Style::default().fg(theme.code_keyword);
    let str_style = Style::default().fg(theme.code_string);
    let comment_style = Style::default()
        .fg(theme.code_comment)
        .add_modifier(Modifier::ITALIC);
    let num_style = Style::default().fg(theme.code_number);
    let type_style = Style::default().fg(theme.code_type);
    let primitive_style = Style::default().fg(theme.code_primitive);
    let macro_style = Style::default().fg(theme.code_macro);

    let trimmed = line.trim_start();

    // 注释行
    if trimmed.starts_with(comment_prefix) {
        return vec![Span::styled(line.to_string(), comment_style)];
    }

    // 逐词解析
    let mut spans = Vec::new();
    let mut chars = line.chars().peekable();
    let mut buf = String::new();

    while let Some(&ch) = chars.peek() {
        // 双引号字符串（支持 \ 转义）
        if ch == '"' {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    macro_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let mut s = String::new();
            s.push(ch);
            chars.next();
            let mut escaped = false;
            while let Some(&c) = chars.peek() {
                s.push(c);
                chars.next();
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
            spans.push(Span::styled(s, str_style));
            continue;
        }
        // 反引号字符串（不支持转义，遇到配对反引号结束）
        if ch == '`' {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    macro_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let mut s = String::new();
            s.push(ch);
            chars.next();
            while let Some(&c) = chars.peek() {
                s.push(c);
                chars.next();
                if c == '`' {
                    break;
                }
            }
            spans.push(Span::styled(s, str_style));
            continue;
        }
        // Rust 生命周期参数 ('a, 'static 等) vs 字符字面量 ('x')
        if ch == '\'' && matches!(lang_lower.as_str(), "rust" | "rs") {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    macro_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let mut s = String::new();
            s.push(ch);
            chars.next();
            let mut is_lifetime = false;
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    s.push(c);
                    chars.next();
                } else if c == '\'' && s.len() == 2 {
                    s.push(c);
                    chars.next();
                    break;
                } else {
                    is_lifetime = true;
                    break;
                }
            }
            if is_lifetime || (s.len() > 1 && !s.ends_with('\'')) {
                let lifetime_style = Style::default().fg(theme.code_lifetime);
                spans.push(Span::styled(s, lifetime_style));
            } else {
                spans.push(Span::styled(s, str_style));
            }
            continue;
        }
        // 其他语言的字符串（包含单引号）
        if ch == '\'' && !matches!(lang_lower.as_str(), "rust" | "rs") {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    macro_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let mut s = String::new();
            s.push(ch);
            chars.next();
            let mut escaped = false;
            while let Some(&c) = chars.peek() {
                s.push(c);
                chars.next();
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
            spans.push(Span::styled(s, str_style));
            continue;
        }
        // Rust 属性 (#[...] 或 #![...])
        if ch == '#' && matches!(lang_lower.as_str(), "rust" | "rs") {
            let mut lookahead = chars.clone();
            if let Some(next) = lookahead.next() {
                if next == '[' {
                    if !buf.is_empty() {
                        spans.extend(colorize_tokens(
                            &buf,
                            keywords,
                            primitive_types,
                            go_type_names,
                            code_style,
                            kw_style,
                            num_style,
                            type_style,
                            primitive_style,
                            macro_style,
                            &lang_lower,
                        ));
                        buf.clear();
                    }
                    let mut attr = String::new();
                    attr.push(ch);
                    chars.next();
                    let mut depth = 0;
                    while let Some(&c) = chars.peek() {
                        attr.push(c);
                        chars.next();
                        if c == '[' {
                            depth += 1;
                        } else if c == ']' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                    let attr_style = Style::default().fg(theme.code_attribute);
                    spans.push(Span::styled(attr, attr_style));
                    continue;
                }
            }
        }
        // Shell 变量 ($VAR, ${VAR}, $1 等)
        if ch == '$'
            && matches!(
                lang_lower.as_str(),
                "sh" | "bash" | "zsh" | "shell" | "dockerfile" | "docker"
            )
        {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    macro_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let var_style = Style::default().fg(theme.code_shell_var);
            let mut var = String::new();
            var.push(ch);
            chars.next();
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '{' {
                    var.push(next_ch);
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        var.push(c);
                        chars.next();
                        if c == '}' {
                            break;
                        }
                    }
                } else if next_ch == '(' {
                    var.push(next_ch);
                    chars.next();
                    let mut depth = 1;
                    while let Some(&c) = chars.peek() {
                        var.push(c);
                        chars.next();
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
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            var.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
            }
            spans.push(Span::styled(var, var_style));
            continue;
        }
        // 行内注释检测
        if ch == '/' || ch == '#' || ch == '-' {
            let rest: String = chars.clone().collect();
            if rest.starts_with(comment_prefix) {
                if !buf.is_empty() {
                    spans.extend(colorize_tokens(
                        &buf,
                        keywords,
                        primitive_types,
                        go_type_names,
                        code_style,
                        kw_style,
                        num_style,
                        type_style,
                        primitive_style,
                        macro_style,
                        &lang_lower,
                    ));
                    buf.clear();
                }
                while chars.peek().is_some() {
                    chars.next();
                }
                spans.push(Span::styled(rest, comment_style));
                break;
            }
        }
        buf.push(ch);
        chars.next();
    }

    if !buf.is_empty() {
        spans.extend(colorize_tokens(
            &buf,
            keywords,
            primitive_types,
            go_type_names,
            code_style,
            kw_style,
            num_style,
            type_style,
            primitive_style,
            macro_style,
            &lang_lower,
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled(line.to_string(), code_style));
    }

    spans
}

/// 将文本按照 word boundary 拆分并对关键字、数字、类型名、原始类型着色
pub fn colorize_tokens<'a>(
    text: &str,
    keywords: &[&str],
    primitive_types: &[&str],
    go_type_names: &[&str],
    default_style: Style,
    kw_style: Style,
    num_style: Style,
    type_style: Style,
    primitive_style: Style,
    macro_style: Style,
    lang: &str,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_word = String::new();
    let mut current_non_word = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() || ch == '_' {
            if !current_non_word.is_empty() {
                spans.push(Span::styled(current_non_word.clone(), default_style));
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
                    spans.push(Span::styled(current_word.clone(), macro_style));
                    current_word.clear();
                    spans.push(Span::styled("!".to_string(), macro_style));
                    continue;
                }
            }
            if !current_word.is_empty() {
                let style = classify_word(
                    &current_word,
                    keywords,
                    primitive_types,
                    go_type_names,
                    kw_style,
                    primitive_style,
                    num_style,
                    type_style,
                    default_style,
                    lang,
                );
                spans.push(Span::styled(current_word.clone(), style));
                current_word.clear();
            }
            current_non_word.push(ch);
        }
    }

    // 刷新剩余
    if !current_non_word.is_empty() {
        spans.push(Span::styled(current_non_word, default_style));
    }
    if !current_word.is_empty() {
        let style = classify_word(
            &current_word,
            keywords,
            primitive_types,
            go_type_names,
            kw_style,
            primitive_style,
            num_style,
            type_style,
            default_style,
            lang,
        );
        spans.push(Span::styled(current_word, style));
    }

    spans
}

/// 根据语言规则判断一个 word 应该使用哪种颜色样式
pub fn classify_word(
    word: &str,
    keywords: &[&str],
    primitive_types: &[&str],
    go_type_names: &[&str],
    kw_style: Style,
    primitive_style: Style,
    num_style: Style,
    type_style: Style,
    default_style: Style,
    lang: &str,
) -> Style {
    if keywords.contains(&word) {
        kw_style
    } else if primitive_types.contains(&word) {
        primitive_style
    } else if word
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        num_style
    } else if matches!(lang, "go" | "golang") {
        if go_type_names.contains(&word) {
            type_style
        } else {
            default_style
        }
    } else if word
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        type_style
    } else {
        default_style
    }
}
