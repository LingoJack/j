use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::assets::Assets;

/// 主题名称枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ThemeName {
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "midnight")]
    #[default]
    Midnight,
    #[serde(rename = "nord")]
    Nord,
    #[serde(rename = "monokai")]
    Monokai,
    #[serde(rename = "anthropic_light")]
    AnthropicLight,
    #[serde(rename = "anthropic_dark")]
    AnthropicDark,
}

impl std::str::FromStr for ThemeName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dark" => Ok(ThemeName::Dark),
            "light" => Ok(ThemeName::Light),
            "midnight" => Ok(ThemeName::Midnight),
            "nord" => Ok(ThemeName::Nord),
            "monokai" => Ok(ThemeName::Monokai),
            "anthropic_light" => Ok(ThemeName::AnthropicLight),
            "anthropic_dark" => Ok(ThemeName::AnthropicDark),
            _ => Ok(ThemeName::default()),
        }
    }
}

#[allow(dead_code)]
impl ThemeName {
    /// 获取所有主题名称列表（用于配置界面循环切换）
    pub fn all() -> &'static [ThemeName] {
        &[
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::Midnight,
            ThemeName::Nord,
            ThemeName::Monokai,
            ThemeName::AnthropicLight,
            ThemeName::AnthropicDark,
        ]
    }

    /// 切换到下一个主题
    pub fn next(&self) -> ThemeName {
        match self {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::Midnight,
            ThemeName::Midnight => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Monokai,
            ThemeName::Monokai => ThemeName::AnthropicLight,
            ThemeName::AnthropicLight => ThemeName::AnthropicDark,
            ThemeName::AnthropicDark => ThemeName::Dark,
        }
    }

    /// 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeName::Dark => "Dark",
            ThemeName::Light => "Light",
            ThemeName::Midnight => "Midnight（默认）",
            ThemeName::Nord => "Nord",
            ThemeName::Monokai => "Monokai",
            ThemeName::AnthropicLight => "Anthropic Light（米白赭陶）",
            ThemeName::AnthropicDark => "Anthropic Dark（深夜月蓝）",
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> ThemeName {
        s.parse().unwrap_or_default()
    }

    /// 转为字符串
    pub fn to_str(&self) -> &'static str {
        match self {
            ThemeName::Dark => "dark",
            ThemeName::Light => "light",
            ThemeName::Midnight => "midnight",
            ThemeName::Nord => "nord",
            ThemeName::Monokai => "monokai",
            ThemeName::AnthropicLight => "anthropic_light",
            ThemeName::AnthropicDark => "anthropic_dark",
        }
    }

    /// 对应的 assets 主题文件名（不含扩展名）
    fn asset_filename(&self) -> &'static str {
        self.to_str()
    }
}

/// 主题配色方案
/// 将所有 UI 颜色归类为语义化字段，方便统一管理
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct Theme {
    // ===== 全局背景 =====
    /// 主背景色
    pub bg_primary: Color,
    /// 标题栏背景
    pub bg_title: Color,
    /// 输入区背景
    pub bg_input: Color,
    /// 帮助/配置界面背景
    pub bg_panel: Color,

    // ===== 边框 =====
    /// 标题栏边框
    pub border_title: Color,
    /// 消息区边框
    pub border_message: Color,
    /// 输入区边框（正常）
    pub border_input: Color,
    /// 输入区边框（加载中）
    pub border_input_loading: Color,
    /// 配置界面边框
    pub border_config: Color,
    /// 分隔线
    pub separator: Color,

    // ===== 气泡 =====
    /// AI 气泡背景
    pub bubble_ai: Color,
    /// AI 气泡背景（选中时）
    pub bubble_ai_selected: Color,
    /// 用户气泡背景
    pub bubble_user: Color,
    /// 用户气泡背景（选中时）
    pub bubble_user_selected: Color,

    // ===== 标签 =====
    /// AI 标签颜色
    pub label_ai: Color,
    /// 用户标签颜色
    pub label_user: Color,
    /// 选中标签颜色
    pub label_selected: Color,

    // ===== 文字 =====
    /// 正文颜色
    pub text_normal: Color,
    /// 强调色（加粗文本）
    pub text_bold: Color,
    /// 弱化文字
    pub text_dim: Color,
    /// 非常弱化的文字
    pub text_very_dim: Color,
    /// 白色文字（用于输入区等）
    pub text_white: Color,
    /// 系统消息颜色
    pub text_system: Color,

    // ===== 标题栏元素 =====
    /// 标题栏图标色
    pub title_icon: Color,
    /// 标题栏分隔符
    pub title_separator: Color,
    /// 模型名称颜色
    pub title_model: Color,
    /// 消息计数颜色
    pub title_count: Color,
    /// 加载中文字颜色
    pub title_loading: Color,

    // ===== 输入区 =====
    /// 输入提示符颜色
    pub input_prompt: Color,
    /// 输入提示符（加载中）颜色
    pub input_prompt_loading: Color,
    /// 光标前景
    pub cursor_fg: Color,
    /// 光标背景
    pub cursor_bg: Color,

    // ===== 提示栏 =====
    /// 键位标签前景
    pub hint_key_fg: Color,
    /// 键位标签背景
    pub hint_key_bg: Color,
    /// 键位描述文字
    pub hint_desc: Color,
    /// 提示栏分隔符
    pub hint_separator: Color,

    // ===== Toast =====
    /// 成功 Toast 边框
    pub toast_success_border: Color,
    /// 成功 Toast 背景
    pub toast_success_bg: Color,
    /// 成功 Toast 文字
    pub toast_success_text: Color,
    /// 错误 Toast 边框
    pub toast_error_border: Color,
    /// 错误 Toast 背景
    pub toast_error_bg: Color,
    /// 错误 Toast 文字
    pub toast_error_text: Color,

    // ===== 工具确认区 =====
    /// 工具确认区边框
    pub tool_confirm_border: Color,
    /// 工具确认区背景
    pub tool_confirm_bg: Color,
    /// 工具确认区标题颜色
    pub tool_confirm_title: Color,
    /// 工具确认区工具名颜色
    pub tool_confirm_name: Color,
    /// 工具确认区消息文字颜色
    pub tool_confirm_text: Color,
    /// 工具确认区标签颜色（如"工具:"）
    pub tool_confirm_label: Color,
    /// 工具确认区提示文字颜色
    pub tool_confirm_hint: Color,

    // ===== 欢迎界面 =====
    /// 欢迎框边框
    pub welcome_border: Color,
    /// 欢迎文字
    pub welcome_text: Color,
    /// 欢迎提示文字
    pub welcome_hint: Color,
    /// 欢迎框诗句颜色
    pub welcome_quote: Color,
    /// 欢迎框渐变调色板索引（0-7），对应不同的渐变色组集合
    pub welcome_palette: u8,

    // ===== 模型选择 =====
    /// 模型选择框边框
    pub model_sel_border: Color,
    /// 模型选择框标题
    pub model_sel_title: Color,
    /// 活跃模型颜色
    pub model_sel_active: Color,
    /// 非活跃模型颜色
    pub model_sel_inactive: Color,
    /// 选中高亮背景
    pub model_sel_highlight_bg: Color,
    /// 选中高亮前景
    pub model_sel_highlight_fg: Color,

    // ===== 配置界面 =====
    /// 配置标题颜色
    pub config_title: Color,
    /// 配置分类标题颜色
    pub config_section: Color,
    /// 配置选中指针颜色
    pub config_pointer: Color,
    /// 配置选中标签颜色
    pub config_label_selected: Color,
    /// 配置普通标签颜色
    pub config_label: Color,
    /// 配置值颜色
    pub config_value: Color,
    /// 配置编辑背景
    pub config_edit_bg: Color,
    /// 配置 tab 选中背景
    pub config_tab_active_bg: Color,
    /// 配置 tab 选中前景
    pub config_tab_active_fg: Color,
    /// 配置 tab 非选中颜色
    pub config_tab_inactive: Color,
    /// 配置键位说明颜色
    pub config_hint_key: Color,
    /// 配置描述颜色
    pub config_hint_desc: Color,
    /// 配置 toggle 开启颜色
    pub config_toggle_on: Color,
    /// 配置 toggle 关闭颜色
    pub config_toggle_off: Color,
    /// 配置弱化文字
    pub config_dim: Color,
    /// API Key 隐藏颜色
    pub config_api_key: Color,

    // ===== Markdown 渲染 =====
    /// 标题 h1 颜色
    pub md_h1: Color,
    /// 标题 h2 颜色
    pub md_h2: Color,
    /// 标题 h3 颜色
    pub md_h3: Color,
    /// 标题 h4+ 颜色
    pub md_h4: Color,
    /// 标题分隔线
    pub md_heading_sep: Color,
    /// 行内代码前景
    pub md_inline_code_fg: Color,
    /// 行内代码背景
    pub md_inline_code_bg: Color,
    /// 列表符号颜色
    pub md_list_bullet: Color,
    /// 引用块竖线颜色
    pub md_blockquote_bar: Color,
    /// 引用块文字颜色
    pub md_blockquote_text: Color,
    /// 引用块背景颜色
    pub md_blockquote_bg: Color,
    /// 分隔线颜色
    pub md_rule: Color,
    /// 链接/URL 颜色
    pub md_link: Color,

    // ===== 代码块 =====
    /// 代码块边框颜色
    pub code_border: Color,
    /// 代码块背景
    pub code_bg: Color,
    /// 代码默认文字颜色
    pub code_default: Color,
    /// 关键字颜色
    pub code_keyword: Color,
    /// 字符串颜色
    pub code_string: Color,
    /// 注释颜色
    pub code_comment: Color,
    /// 数字颜色
    pub code_number: Color,
    /// 类型名颜色
    pub code_type: Color,
    /// 原始类型颜色
    pub code_primitive: Color,
    /// 宏调用颜色
    pub code_macro: Color,
    /// 属性/装饰器颜色
    pub code_attribute: Color,
    /// 生命周期颜色
    pub code_lifetime: Color,
    /// Shell 变量颜色
    pub code_shell_var: Color,

    // ===== 表格 =====
    /// 表格边框颜色
    pub table_border: Color,
    /// 表格表头颜色
    pub table_header: Color,
    /// 表格内容颜色
    pub table_body: Color,

    // ===== 帮助界面 =====
    /// 帮助标题颜色
    pub help_title: Color,
    /// 帮助按键颜色
    pub help_key: Color,
    /// 帮助描述颜色
    pub help_desc: Color,
    /// 帮助文件路径颜色
    pub help_path: Color,
    /// 帮助背景颜色
    pub help_bg: Color,

    // ===== Diff 显示 =====
    /// 新增行颜色（绿色）
    pub diff_add: Color,
    /// 删除行颜色（红色）
    pub diff_del: Color,
    /// diff header 颜色（青色）
    pub diff_header: Color,
}

// ===== JSON 反序列化中间结构 =====

/// JSON 主题文件的中间表示（所有字段为可反序列化类型）
#[derive(Deserialize)]
struct ThemeJson {
    bg_primary: ColorValue,
    bg_title: ColorValue,
    bg_input: ColorValue,
    bg_panel: ColorValue,
    border_title: ColorValue,
    border_message: ColorValue,
    border_input: ColorValue,
    border_input_loading: ColorValue,
    border_config: ColorValue,
    separator: ColorValue,
    bubble_ai: ColorValue,
    bubble_ai_selected: ColorValue,
    bubble_user: ColorValue,
    bubble_user_selected: ColorValue,
    label_ai: ColorValue,
    label_user: ColorValue,
    label_selected: ColorValue,
    text_normal: ColorValue,
    text_bold: ColorValue,
    text_dim: ColorValue,
    text_very_dim: ColorValue,
    text_white: ColorValue,
    text_system: ColorValue,
    title_icon: ColorValue,
    title_separator: ColorValue,
    title_model: ColorValue,
    title_count: ColorValue,
    title_loading: ColorValue,
    input_prompt: ColorValue,
    input_prompt_loading: ColorValue,
    cursor_fg: ColorValue,
    cursor_bg: ColorValue,
    hint_key_fg: ColorValue,
    hint_key_bg: ColorValue,
    hint_desc: ColorValue,
    hint_separator: ColorValue,
    toast_success_border: ColorValue,
    toast_success_bg: ColorValue,
    toast_success_text: ColorValue,
    toast_error_border: ColorValue,
    toast_error_bg: ColorValue,
    toast_error_text: ColorValue,
    tool_confirm_border: ColorValue,
    tool_confirm_bg: ColorValue,
    tool_confirm_title: ColorValue,
    tool_confirm_name: ColorValue,
    tool_confirm_text: ColorValue,
    tool_confirm_label: ColorValue,
    tool_confirm_hint: ColorValue,
    welcome_border: ColorValue,
    welcome_text: ColorValue,
    welcome_hint: ColorValue,
    welcome_quote: ColorValue,
    welcome_palette: u8,
    model_sel_border: ColorValue,
    model_sel_title: ColorValue,
    model_sel_active: ColorValue,
    model_sel_inactive: ColorValue,
    model_sel_highlight_bg: ColorValue,
    model_sel_highlight_fg: ColorValue,
    config_title: ColorValue,
    config_section: ColorValue,
    config_pointer: ColorValue,
    config_label_selected: ColorValue,
    config_label: ColorValue,
    config_value: ColorValue,
    config_edit_bg: ColorValue,
    config_tab_active_bg: ColorValue,
    config_tab_active_fg: ColorValue,
    config_tab_inactive: ColorValue,
    config_hint_key: ColorValue,
    config_hint_desc: ColorValue,
    config_toggle_on: ColorValue,
    config_toggle_off: ColorValue,
    config_dim: ColorValue,
    config_api_key: ColorValue,
    md_h1: ColorValue,
    md_h2: ColorValue,
    md_h3: ColorValue,
    md_h4: ColorValue,
    md_heading_sep: ColorValue,
    md_inline_code_fg: ColorValue,
    md_inline_code_bg: ColorValue,
    md_list_bullet: ColorValue,
    md_blockquote_bar: ColorValue,
    md_blockquote_text: ColorValue,
    md_blockquote_bg: ColorValue,
    md_rule: ColorValue,
    md_link: ColorValue,
    code_border: ColorValue,
    code_bg: ColorValue,
    code_default: ColorValue,
    code_keyword: ColorValue,
    code_string: ColorValue,
    code_comment: ColorValue,
    code_number: ColorValue,
    code_type: ColorValue,
    code_primitive: ColorValue,
    code_macro: ColorValue,
    code_attribute: ColorValue,
    code_lifetime: ColorValue,
    code_shell_var: ColorValue,
    table_border: ColorValue,
    table_header: ColorValue,
    table_body: ColorValue,
    help_title: ColorValue,
    help_key: ColorValue,
    help_desc: ColorValue,
    help_path: ColorValue,
    help_bg: ColorValue,
    diff_add: ColorValue,
    diff_del: ColorValue,
    diff_header: ColorValue,
}

impl From<ThemeJson> for Theme {
    fn from(j: ThemeJson) -> Self {
        Self {
            bg_primary: j.bg_primary.0,
            bg_title: j.bg_title.0,
            bg_input: j.bg_input.0,
            bg_panel: j.bg_panel.0,
            border_title: j.border_title.0,
            border_message: j.border_message.0,
            border_input: j.border_input.0,
            border_input_loading: j.border_input_loading.0,
            border_config: j.border_config.0,
            separator: j.separator.0,
            bubble_ai: j.bubble_ai.0,
            bubble_ai_selected: j.bubble_ai_selected.0,
            bubble_user: j.bubble_user.0,
            bubble_user_selected: j.bubble_user_selected.0,
            label_ai: j.label_ai.0,
            label_user: j.label_user.0,
            label_selected: j.label_selected.0,
            text_normal: j.text_normal.0,
            text_bold: j.text_bold.0,
            text_dim: j.text_dim.0,
            text_very_dim: j.text_very_dim.0,
            text_white: j.text_white.0,
            text_system: j.text_system.0,
            title_icon: j.title_icon.0,
            title_separator: j.title_separator.0,
            title_model: j.title_model.0,
            title_count: j.title_count.0,
            title_loading: j.title_loading.0,
            input_prompt: j.input_prompt.0,
            input_prompt_loading: j.input_prompt_loading.0,
            cursor_fg: j.cursor_fg.0,
            cursor_bg: j.cursor_bg.0,
            hint_key_fg: j.hint_key_fg.0,
            hint_key_bg: j.hint_key_bg.0,
            hint_desc: j.hint_desc.0,
            hint_separator: j.hint_separator.0,
            toast_success_border: j.toast_success_border.0,
            toast_success_bg: j.toast_success_bg.0,
            toast_success_text: j.toast_success_text.0,
            toast_error_border: j.toast_error_border.0,
            toast_error_bg: j.toast_error_bg.0,
            toast_error_text: j.toast_error_text.0,
            tool_confirm_border: j.tool_confirm_border.0,
            tool_confirm_bg: j.tool_confirm_bg.0,
            tool_confirm_title: j.tool_confirm_title.0,
            tool_confirm_name: j.tool_confirm_name.0,
            tool_confirm_text: j.tool_confirm_text.0,
            tool_confirm_label: j.tool_confirm_label.0,
            tool_confirm_hint: j.tool_confirm_hint.0,
            welcome_border: j.welcome_border.0,
            welcome_text: j.welcome_text.0,
            welcome_hint: j.welcome_hint.0,
            welcome_quote: j.welcome_quote.0,
            welcome_palette: j.welcome_palette,
            model_sel_border: j.model_sel_border.0,
            model_sel_title: j.model_sel_title.0,
            model_sel_active: j.model_sel_active.0,
            model_sel_inactive: j.model_sel_inactive.0,
            model_sel_highlight_bg: j.model_sel_highlight_bg.0,
            model_sel_highlight_fg: j.model_sel_highlight_fg.0,
            config_title: j.config_title.0,
            config_section: j.config_section.0,
            config_pointer: j.config_pointer.0,
            config_label_selected: j.config_label_selected.0,
            config_label: j.config_label.0,
            config_value: j.config_value.0,
            config_edit_bg: j.config_edit_bg.0,
            config_tab_active_bg: j.config_tab_active_bg.0,
            config_tab_active_fg: j.config_tab_active_fg.0,
            config_tab_inactive: j.config_tab_inactive.0,
            config_hint_key: j.config_hint_key.0,
            config_hint_desc: j.config_hint_desc.0,
            config_toggle_on: j.config_toggle_on.0,
            config_toggle_off: j.config_toggle_off.0,
            config_dim: j.config_dim.0,
            config_api_key: j.config_api_key.0,
            md_h1: j.md_h1.0,
            md_h2: j.md_h2.0,
            md_h3: j.md_h3.0,
            md_h4: j.md_h4.0,
            md_heading_sep: j.md_heading_sep.0,
            md_inline_code_fg: j.md_inline_code_fg.0,
            md_inline_code_bg: j.md_inline_code_bg.0,
            md_list_bullet: j.md_list_bullet.0,
            md_blockquote_bar: j.md_blockquote_bar.0,
            md_blockquote_text: j.md_blockquote_text.0,
            md_blockquote_bg: j.md_blockquote_bg.0,
            md_rule: j.md_rule.0,
            md_link: j.md_link.0,
            code_border: j.code_border.0,
            code_bg: j.code_bg.0,
            code_default: j.code_default.0,
            code_keyword: j.code_keyword.0,
            code_string: j.code_string.0,
            code_comment: j.code_comment.0,
            code_number: j.code_number.0,
            code_type: j.code_type.0,
            code_primitive: j.code_primitive.0,
            code_macro: j.code_macro.0,
            code_attribute: j.code_attribute.0,
            code_lifetime: j.code_lifetime.0,
            code_shell_var: j.code_shell_var.0,
            table_border: j.table_border.0,
            table_header: j.table_header.0,
            table_body: j.table_body.0,
            help_title: j.help_title.0,
            help_key: j.help_key.0,
            help_desc: j.help_desc.0,
            help_path: j.help_path.0,
            help_bg: j.help_bg.0,
            diff_add: j.diff_add.0,
            diff_del: j.diff_del.0,
            diff_header: j.diff_header.0,
        }
    }
}

/// 颜色值的 JSON 反序列化包装
///
/// 支持格式：
/// - `"#rrggbb"` → `Color::Rgb(r, g, b)`
/// - `"reset"` / `"white"` / `"dark_gray"` 等 → 对应 ANSI 命名色
#[derive(Deserialize)]
#[serde(try_from = "String")]
struct ColorValue(Color);

impl TryFrom<String> for ColorValue {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let color = parse_color(&s)?;
        Ok(ColorValue(color))
    }
}

/// 解析颜色字符串
fn parse_color(s: &str) -> Result<Color, String> {
    // #rrggbb 十六进制
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
            return Ok(Color::Rgb(r, g, b));
        }
        return Err(format!("invalid hex color: {s}"));
    }

    // ANSI 命名色
    Ok(match s {
        "reset" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" => Color::DarkGray,
        "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_yellow" => Color::LightYellow,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => return Err(format!("unknown color name: {s}")),
    })
}

impl Theme {
    /// 根据主题名称从嵌入资源加载主题
    pub fn from_name(name: &ThemeName) -> Self {
        let filename = format!("themes/{}.json", name.asset_filename());
        match Self::load_from_assets(&filename) {
            Ok(theme) => theme,
            Err(_) => {
                // 主题加载失败时静默回退，不输出日志（无 config 上下文）
                // 回退到 midnight（如果连默认都加载失败则用 terminal）
                if *name != ThemeName::Midnight {
                    match Self::load_from_assets("themes/midnight.json") {
                        Ok(t) => t,
                        Err(_) => Self::terminal_fallback(),
                    }
                } else {
                    Self::terminal_fallback()
                }
            }
        }
    }

    /// 从 Assets 加载并解析主题 JSON
    fn load_from_assets(path: &str) -> Result<Self, String> {
        let asset = Assets::get(path).ok_or_else(|| format!("asset not found: {path}"))?;
        let json_str = std::str::from_utf8(&asset.data).map_err(|e| e.to_string())?;
        let theme_json: ThemeJson =
            serde_json::from_str(json_str).map_err(|e| format!("parse theme {path}: {e}"))?;
        Ok(theme_json.into())
    }

    /// 终端原生主题：使用标准 ANSI 颜色，适合非 AI 模式下的终端输出
    /// 颜色跟随终端自身配色方案，不依赖 agent 配置
    pub fn terminal() -> Self {
        Self::terminal_fallback()
    }

    /// 终端回退主题：使用标准 ANSI 颜色，不依赖外部文件
    fn terminal_fallback() -> Self {
        Self {
            bg_primary: Color::Reset,
            bg_title: Color::Reset,
            bg_input: Color::Reset,
            bg_panel: Color::Reset,
            border_title: Color::DarkGray,
            border_message: Color::DarkGray,
            border_input: Color::DarkGray,
            border_input_loading: Color::DarkGray,
            border_config: Color::DarkGray,
            separator: Color::DarkGray,
            bubble_ai: Color::Reset,
            bubble_ai_selected: Color::Reset,
            bubble_user: Color::Reset,
            bubble_user_selected: Color::Reset,
            label_ai: Color::Reset,
            label_user: Color::Reset,
            label_selected: Color::Reset,
            text_normal: Color::Reset,
            text_bold: Color::White,
            text_dim: Color::DarkGray,
            text_very_dim: Color::DarkGray,
            text_white: Color::White,
            text_system: Color::DarkGray,
            title_icon: Color::Reset,
            title_separator: Color::DarkGray,
            title_model: Color::Reset,
            title_count: Color::Reset,
            title_loading: Color::Reset,
            input_prompt: Color::Reset,
            input_prompt_loading: Color::Reset,
            cursor_fg: Color::Reset,
            cursor_bg: Color::Reset,
            hint_key_fg: Color::Reset,
            hint_key_bg: Color::Reset,
            hint_desc: Color::Reset,
            hint_separator: Color::DarkGray,
            toast_success_border: Color::Green,
            toast_success_bg: Color::Reset,
            toast_success_text: Color::LightGreen,
            toast_error_border: Color::Red,
            toast_error_bg: Color::Reset,
            toast_error_text: Color::LightRed,
            tool_confirm_border: Color::Cyan,
            tool_confirm_bg: Color::Blue,
            tool_confirm_title: Color::Yellow,
            tool_confirm_name: Color::Yellow,
            tool_confirm_text: Color::White,
            tool_confirm_label: Color::White,
            tool_confirm_hint: Color::Yellow,
            welcome_border: Color::DarkGray,
            welcome_text: Color::Reset,
            welcome_hint: Color::DarkGray,
            welcome_quote: Color::DarkGray,
            welcome_palette: 5,
            model_sel_border: Color::DarkGray,
            model_sel_title: Color::Reset,
            model_sel_active: Color::LightGreen,
            model_sel_inactive: Color::Reset,
            model_sel_highlight_bg: Color::Reset,
            model_sel_highlight_fg: Color::Yellow,
            config_title: Color::LightCyan,
            config_section: Color::LightGreen,
            config_pointer: Color::Yellow,
            config_label_selected: Color::Yellow,
            config_label: Color::DarkGray,
            config_value: Color::Reset,
            config_edit_bg: Color::Reset,
            config_tab_active_bg: Color::LightCyan,
            config_tab_active_fg: Color::Reset,
            config_tab_inactive: Color::DarkGray,
            config_hint_key: Color::Yellow,
            config_hint_desc: Color::DarkGray,
            config_toggle_on: Color::LightGreen,
            config_toggle_off: Color::Red,
            config_dim: Color::DarkGray,
            config_api_key: Color::DarkGray,
            md_h1: Color::LightCyan,
            md_h2: Color::Cyan,
            md_h3: Color::LightBlue,
            md_h4: Color::Blue,
            md_heading_sep: Color::DarkGray,
            md_inline_code_fg: Color::LightYellow,
            md_inline_code_bg: Color::Reset,
            md_list_bullet: Color::LightGreen,
            md_blockquote_bar: Color::Cyan,
            md_blockquote_text: Color::Gray,
            md_blockquote_bg: Color::Reset,
            md_rule: Color::DarkGray,
            md_link: Color::LightBlue,
            code_border: Color::DarkGray,
            code_bg: Color::Reset,
            code_default: Color::Reset,
            code_keyword: Color::LightMagenta,
            code_string: Color::LightGreen,
            code_comment: Color::DarkGray,
            code_number: Color::LightYellow,
            code_type: Color::LightYellow,
            code_primitive: Color::LightCyan,
            code_macro: Color::LightBlue,
            code_attribute: Color::LightCyan,
            code_lifetime: Color::LightYellow,
            code_shell_var: Color::LightCyan,
            table_border: Color::DarkGray,
            table_header: Color::LightCyan,
            table_body: Color::Reset,
            help_title: Color::LightCyan,
            help_key: Color::Yellow,
            help_desc: Color::Reset,
            help_path: Color::DarkGray,
            help_bg: Color::Reset,
            diff_add: Color::LightGreen,
            diff_del: Color::LightRed,
            diff_header: Color::LightCyan,
        }
    }
}
