use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// 主题名称枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThemeName {
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "midnight")]
    Midnight,
    #[serde(rename = "nord")]
    Nord,
    #[serde(rename = "monokai")]
    Monokai,
}

impl Default for ThemeName {
    fn default() -> Self {
        ThemeName::Midnight
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
        ]
    }

    /// 切换到下一个主题
    pub fn next(&self) -> ThemeName {
        match self {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::Midnight,
            ThemeName::Midnight => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Monokai,
            ThemeName::Monokai => ThemeName::Dark,
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
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> ThemeName {
        match s.to_lowercase().as_str() {
            "dark" => ThemeName::Dark,
            "light" => ThemeName::Light,
            "midnight" => ThemeName::Midnight,
            "nord" => ThemeName::Nord,
            "monokai" => ThemeName::Monokai,
            _ => ThemeName::default(),
        }
    }

    /// 转为字符串
    pub fn to_str(&self) -> &'static str {
        match self {
            ThemeName::Dark => "dark",
            ThemeName::Light => "light",
            ThemeName::Midnight => "midnight",
            ThemeName::Nord => "nord",
            ThemeName::Monokai => "monokai",
        }
    }
}

/// 主题配色方案
/// 将所有 UI 颜色归类为语义化字段，方便统一管理
#[derive(Debug, Clone)]
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
    /// 分隔线颜色
    pub md_rule: Color,

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
}

impl Theme {
    /// 根据主题名称创建对应的主题
    pub fn from_name(name: &ThemeName) -> Self {
        match name {
            ThemeName::Dark => Self::dark(),
            ThemeName::Light => Self::light(),
            ThemeName::Midnight => Self::midnight(),
            ThemeName::Nord => Self::nord(),
            ThemeName::Monokai => Self::monokai(),
        }
    }

    /// Midnight 主题（原始深色主题 - 默认）
    pub fn midnight() -> Self {
        Self {
            // 全局背景
            bg_primary: Color::Rgb(22, 22, 30),
            bg_title: Color::Rgb(28, 28, 40),
            bg_input: Color::Rgb(26, 26, 38),
            bg_panel: Color::Rgb(24, 24, 34),

            // 边框
            border_title: Color::Rgb(80, 100, 140),
            border_message: Color::Rgb(50, 55, 70),
            border_input: Color::Rgb(60, 100, 80),
            border_input_loading: Color::Rgb(120, 100, 50),
            border_config: Color::Rgb(80, 80, 110),
            separator: Color::Rgb(50, 55, 70),

            // 气泡
            bubble_ai: Color::Rgb(28, 28, 38),
            bubble_ai_selected: Color::Rgb(255, 255, 255),
            bubble_user: Color::Rgb(40, 70, 120),
            bubble_user_selected: Color::Rgb(255, 255, 255),

            // 标签
            label_ai: Color::Rgb(120, 220, 160),
            label_user: Color::Rgb(100, 160, 255),
            label_selected: Color::Rgb(255, 200, 80),

            // 文字
            text_normal: Color::Rgb(220, 220, 230),
            text_bold: Color::Rgb(220, 245, 230),
            text_dim: Color::Rgb(140, 140, 170),
            text_very_dim: Color::Rgb(80, 80, 100),
            text_white: Color::White,
            text_system: Color::Rgb(100, 100, 120),

            // 标题栏
            title_icon: Color::Rgb(120, 180, 255),
            title_separator: Color::Rgb(60, 60, 80),
            title_model: Color::Rgb(160, 220, 160),
            title_count: Color::Rgb(180, 180, 200),
            title_loading: Color::Rgb(255, 200, 80),

            // 输入区
            input_prompt: Color::Rgb(100, 200, 130),
            input_prompt_loading: Color::Rgb(255, 200, 80),
            cursor_fg: Color::Rgb(22, 22, 30),
            cursor_bg: Color::Rgb(200, 210, 240),

            // 提示栏
            hint_key_fg: Color::Rgb(22, 22, 30),
            hint_key_bg: Color::Rgb(100, 110, 140),
            hint_desc: Color::Rgb(120, 120, 150),
            hint_separator: Color::Rgb(50, 50, 65),

            // Toast
            toast_success_border: Color::Rgb(60, 160, 80),
            toast_success_bg: Color::Rgb(18, 40, 25),
            toast_success_text: Color::Rgb(140, 230, 160),
            toast_error_border: Color::Rgb(200, 70, 70),
            toast_error_bg: Color::Rgb(50, 18, 18),
            toast_error_text: Color::Rgb(255, 130, 130),

            // 工具确认区
            tool_confirm_border: Color::Rgb(200, 180, 80),
            tool_confirm_bg: Color::Rgb(35, 30, 15),
            tool_confirm_title: Color::Rgb(255, 220, 100),
            tool_confirm_name: Color::Rgb(255, 220, 100),
            tool_confirm_text: Color::Rgb(220, 220, 230),
            tool_confirm_label: Color::Rgb(140, 140, 160),
            tool_confirm_hint: Color::Rgb(180, 180, 200),

            // 欢迎界面
            welcome_border: Color::Rgb(60, 70, 90),
            welcome_text: Color::Rgb(120, 140, 180),
            welcome_hint: Color::Rgb(80, 90, 110),

            // 模型选择
            model_sel_border: Color::Rgb(180, 160, 80),
            model_sel_title: Color::Rgb(230, 210, 120),
            model_sel_active: Color::Rgb(120, 220, 160),
            model_sel_inactive: Color::Rgb(180, 180, 200),
            model_sel_highlight_bg: Color::Rgb(50, 55, 80),

            // 配置界面
            config_title: Color::Rgb(120, 180, 255),
            config_section: Color::Rgb(160, 220, 160),
            config_pointer: Color::Rgb(255, 200, 80),
            config_label_selected: Color::Rgb(230, 210, 120),
            config_label: Color::Rgb(140, 140, 160),
            config_value: Color::Rgb(180, 180, 200),
            config_edit_bg: Color::Rgb(50, 55, 80),
            config_tab_active_bg: Color::Rgb(120, 180, 255),
            config_tab_active_fg: Color::Rgb(22, 22, 30),
            config_tab_inactive: Color::Rgb(150, 150, 170),
            config_hint_key: Color::Rgb(230, 210, 120),
            config_hint_desc: Color::Rgb(120, 120, 150),
            config_toggle_on: Color::Rgb(120, 220, 160),
            config_toggle_off: Color::Rgb(200, 100, 100),
            config_dim: Color::Rgb(80, 80, 100),
            config_api_key: Color::Rgb(100, 100, 120),

            // Markdown
            md_h1: Color::Rgb(100, 180, 255),
            md_h2: Color::Rgb(130, 190, 255),
            md_h3: Color::Rgb(160, 200, 255),
            md_h4: Color::Rgb(180, 210, 255),
            md_heading_sep: Color::Rgb(60, 70, 100),
            md_inline_code_fg: Color::Rgb(230, 190, 120),
            md_inline_code_bg: Color::Rgb(45, 45, 60),
            md_list_bullet: Color::Rgb(100, 160, 255),
            md_blockquote_bar: Color::Rgb(80, 100, 140),
            md_blockquote_text: Color::Rgb(150, 160, 180),
            md_rule: Color::Rgb(70, 75, 90),

            // 代码块
            code_border: Color::Rgb(80, 90, 110),
            code_bg: Color::Rgb(30, 30, 42),
            code_default: Color::Rgb(171, 178, 191),
            code_keyword: Color::Rgb(198, 120, 221),
            code_string: Color::Rgb(152, 195, 121),
            code_comment: Color::Rgb(92, 99, 112),
            code_number: Color::Rgb(209, 154, 102),
            code_type: Color::Rgb(229, 192, 123),
            code_primitive: Color::Rgb(86, 182, 194),
            code_macro: Color::Rgb(97, 175, 239),
            code_attribute: Color::Rgb(86, 182, 194),
            code_lifetime: Color::Rgb(229, 192, 123),
            code_shell_var: Color::Rgb(86, 182, 194),

            // 表格
            table_border: Color::Rgb(60, 70, 100),
            table_header: Color::Rgb(120, 180, 255),
            table_body: Color::Rgb(180, 180, 200),

            // 帮助界面
            help_title: Color::Rgb(120, 180, 255),
            help_key: Color::Rgb(230, 210, 120),
            help_desc: Color::Rgb(200, 200, 220),
            help_path: Color::Rgb(100, 100, 130),
            help_bg: Color::Rgb(24, 24, 34),
        }
    }

    /// Dark 主题（偏灰暗的深色主题，类似 VS Code Dark+）
    pub fn dark() -> Self {
        Self {
            // 全局背景
            bg_primary: Color::Rgb(30, 30, 30),
            bg_title: Color::Rgb(37, 37, 38),
            bg_input: Color::Rgb(37, 37, 38),
            bg_panel: Color::Rgb(37, 37, 38),

            // 边框
            border_title: Color::Rgb(70, 70, 70),
            border_message: Color::Rgb(55, 55, 55),
            border_input: Color::Rgb(55, 80, 55),
            border_input_loading: Color::Rgb(120, 100, 50),
            border_config: Color::Rgb(70, 70, 70),
            separator: Color::Rgb(55, 55, 55),

            // 气泡
            bubble_ai: Color::Rgb(34, 34, 34),
            bubble_ai_selected: Color::Rgb(255, 255, 255),
            bubble_user: Color::Rgb(38, 65, 110),
            bubble_user_selected: Color::Rgb(255, 255, 255),

            // 标签
            label_ai: Color::Rgb(80, 200, 120),
            label_user: Color::Rgb(80, 150, 240),
            label_selected: Color::Rgb(255, 200, 80),

            // 文字
            text_normal: Color::Rgb(212, 212, 212),
            text_bold: Color::Rgb(210, 240, 220),
            text_dim: Color::Rgb(128, 128, 128),
            text_very_dim: Color::Rgb(80, 80, 80),
            text_white: Color::White,
            text_system: Color::Rgb(100, 100, 100),

            // 标题栏
            title_icon: Color::Rgb(100, 160, 240),
            title_separator: Color::Rgb(60, 60, 60),
            title_model: Color::Rgb(140, 200, 140),
            title_count: Color::Rgb(170, 170, 170),
            title_loading: Color::Rgb(255, 200, 80),

            // 输入区
            input_prompt: Color::Rgb(80, 180, 100),
            input_prompt_loading: Color::Rgb(255, 200, 80),
            cursor_fg: Color::Rgb(30, 30, 30),
            cursor_bg: Color::Rgb(200, 200, 200),

            // 提示栏
            hint_key_fg: Color::Rgb(30, 30, 30),
            hint_key_bg: Color::Rgb(100, 100, 100),
            hint_desc: Color::Rgb(128, 128, 128),
            hint_separator: Color::Rgb(50, 50, 50),

            // Toast
            toast_success_border: Color::Rgb(60, 160, 80),
            toast_success_bg: Color::Rgb(20, 40, 25),
            toast_success_text: Color::Rgb(140, 230, 160),
            toast_error_border: Color::Rgb(200, 70, 70),
            toast_error_bg: Color::Rgb(50, 20, 20),
            toast_error_text: Color::Rgb(255, 130, 130),

            // 工具确认区
            tool_confirm_border: Color::Rgb(200, 180, 80),
            tool_confirm_bg: Color::Rgb(40, 35, 20),
            tool_confirm_title: Color::Rgb(255, 220, 100),
            tool_confirm_name: Color::Rgb(255, 220, 100),
            tool_confirm_text: Color::Rgb(212, 212, 212),
            tool_confirm_label: Color::Rgb(128, 128, 128),
            tool_confirm_hint: Color::Rgb(170, 170, 170),

            // 欢迎界面
            welcome_border: Color::Rgb(60, 60, 60),
            welcome_text: Color::Rgb(120, 140, 180),
            welcome_hint: Color::Rgb(80, 80, 80),

            // 模型选择
            model_sel_border: Color::Rgb(180, 160, 80),
            model_sel_title: Color::Rgb(230, 210, 120),
            model_sel_active: Color::Rgb(80, 200, 120),
            model_sel_inactive: Color::Rgb(170, 170, 170),
            model_sel_highlight_bg: Color::Rgb(50, 50, 60),

            // 配置界面
            config_title: Color::Rgb(100, 160, 240),
            config_section: Color::Rgb(140, 200, 140),
            config_pointer: Color::Rgb(255, 200, 80),
            config_label_selected: Color::Rgb(230, 210, 120),
            config_label: Color::Rgb(128, 128, 128),
            config_value: Color::Rgb(170, 170, 170),
            config_edit_bg: Color::Rgb(50, 50, 60),
            config_tab_active_bg: Color::Rgb(100, 160, 240),
            config_tab_active_fg: Color::Rgb(30, 30, 30),
            config_tab_inactive: Color::Rgb(140, 140, 140),
            config_hint_key: Color::Rgb(230, 210, 120),
            config_hint_desc: Color::Rgb(128, 128, 128),
            config_toggle_on: Color::Rgb(80, 200, 120),
            config_toggle_off: Color::Rgb(200, 100, 100),
            config_dim: Color::Rgb(80, 80, 80),
            config_api_key: Color::Rgb(100, 100, 100),

            // Markdown
            md_h1: Color::Rgb(80, 160, 240),
            md_h2: Color::Rgb(100, 170, 240),
            md_h3: Color::Rgb(120, 180, 240),
            md_h4: Color::Rgb(140, 190, 240),
            md_heading_sep: Color::Rgb(60, 60, 80),
            md_inline_code_fg: Color::Rgb(220, 180, 110),
            md_inline_code_bg: Color::Rgb(50, 50, 60),
            md_list_bullet: Color::Rgb(80, 150, 240),
            md_blockquote_bar: Color::Rgb(70, 90, 130),
            md_blockquote_text: Color::Rgb(150, 150, 170),
            md_rule: Color::Rgb(70, 70, 80),

            // 代码块
            code_border: Color::Rgb(70, 70, 80),
            code_bg: Color::Rgb(35, 35, 38),
            code_default: Color::Rgb(212, 212, 212),
            code_keyword: Color::Rgb(198, 120, 221),
            code_string: Color::Rgb(152, 195, 121),
            code_comment: Color::Rgb(106, 115, 125),
            code_number: Color::Rgb(209, 154, 102),
            code_type: Color::Rgb(229, 192, 123),
            code_primitive: Color::Rgb(86, 182, 194),
            code_macro: Color::Rgb(97, 175, 239),
            code_attribute: Color::Rgb(86, 182, 194),
            code_lifetime: Color::Rgb(229, 192, 123),
            code_shell_var: Color::Rgb(86, 182, 194),

            // 表格
            table_border: Color::Rgb(60, 60, 80),
            table_header: Color::Rgb(80, 160, 240),
            table_body: Color::Rgb(170, 170, 170),

            // 帮助界面
            help_title: Color::Rgb(100, 160, 240),
            help_key: Color::Rgb(230, 210, 120),
            help_desc: Color::Rgb(200, 200, 200),
            help_path: Color::Rgb(100, 100, 100),
            help_bg: Color::Rgb(37, 37, 38),
        }
    }

    /// Light 主题（浅色主题，类似 VS Code Light+）
    pub fn light() -> Self {
        Self {
            // 全局背景
            bg_primary: Color::Rgb(255, 255, 255),
            bg_title: Color::Rgb(243, 243, 243),
            bg_input: Color::Rgb(248, 248, 248),
            bg_panel: Color::Rgb(248, 248, 248),

            // 边框
            border_title: Color::Rgb(190, 190, 200),
            border_message: Color::Rgb(210, 210, 220),
            border_input: Color::Rgb(160, 200, 160),
            border_input_loading: Color::Rgb(180, 150, 50),
            border_config: Color::Rgb(190, 190, 200),
            separator: Color::Rgb(210, 210, 220),

            // 气泡
            bubble_ai: Color::Rgb(244, 244, 248),
            bubble_ai_selected: Color::Rgb(255, 255, 255),
            bubble_user: Color::Rgb(210, 230, 255),
            bubble_user_selected: Color::Rgb(255, 255, 255),

            // 标签
            label_ai: Color::Rgb(40, 140, 80),
            label_user: Color::Rgb(30, 100, 200),
            label_selected: Color::Rgb(180, 130, 20),

            // 文字
            text_normal: Color::Rgb(40, 40, 50),
            text_bold: Color::Rgb(30, 100, 60),
            text_dim: Color::Rgb(120, 120, 140),
            text_very_dim: Color::Rgb(170, 170, 180),
            text_white: Color::Rgb(40, 40, 50),
            text_system: Color::Rgb(140, 140, 160),

            // 标题栏
            title_icon: Color::Rgb(40, 100, 200),
            title_separator: Color::Rgb(200, 200, 210),
            title_model: Color::Rgb(40, 140, 80),
            title_count: Color::Rgb(100, 100, 120),
            title_loading: Color::Rgb(180, 130, 20),

            // 输入区
            input_prompt: Color::Rgb(40, 140, 80),
            input_prompt_loading: Color::Rgb(180, 130, 20),
            cursor_fg: Color::Rgb(255, 255, 255),
            cursor_bg: Color::Rgb(50, 100, 200),

            // 提示栏
            hint_key_fg: Color::Rgb(255, 255, 255),
            hint_key_bg: Color::Rgb(100, 110, 130),
            hint_desc: Color::Rgb(120, 120, 140),
            hint_separator: Color::Rgb(210, 210, 220),

            // Toast
            toast_success_border: Color::Rgb(60, 160, 80),
            toast_success_bg: Color::Rgb(230, 250, 235),
            toast_success_text: Color::Rgb(30, 100, 50),
            toast_error_border: Color::Rgb(200, 70, 70),
            toast_error_bg: Color::Rgb(255, 235, 235),
            toast_error_text: Color::Rgb(160, 30, 30),

            // 工具确认区
            tool_confirm_border: Color::Rgb(180, 140, 40),
            tool_confirm_bg: Color::Rgb(255, 250, 235),
            tool_confirm_title: Color::Rgb(160, 120, 20),
            tool_confirm_name: Color::Rgb(160, 120, 20),
            tool_confirm_text: Color::Rgb(40, 40, 50),
            tool_confirm_label: Color::Rgb(120, 120, 140),
            tool_confirm_hint: Color::Rgb(80, 80, 100),

            // 欢迎界面
            welcome_border: Color::Rgb(180, 190, 210),
            welcome_text: Color::Rgb(60, 80, 130),
            welcome_hint: Color::Rgb(140, 150, 170),

            // 模型选择
            model_sel_border: Color::Rgb(180, 160, 80),
            model_sel_title: Color::Rgb(140, 110, 30),
            model_sel_active: Color::Rgb(40, 140, 80),
            model_sel_inactive: Color::Rgb(100, 100, 120),
            model_sel_highlight_bg: Color::Rgb(225, 230, 245),

            // 配置界面
            config_title: Color::Rgb(40, 100, 200),
            config_section: Color::Rgb(40, 140, 80),
            config_pointer: Color::Rgb(180, 130, 20),
            config_label_selected: Color::Rgb(140, 110, 30),
            config_label: Color::Rgb(120, 120, 140),
            config_value: Color::Rgb(60, 60, 80),
            config_edit_bg: Color::Rgb(225, 230, 245),
            config_tab_active_bg: Color::Rgb(40, 100, 200),
            config_tab_active_fg: Color::Rgb(255, 255, 255),
            config_tab_inactive: Color::Rgb(120, 120, 140),
            config_hint_key: Color::Rgb(140, 110, 30),
            config_hint_desc: Color::Rgb(120, 120, 140),
            config_toggle_on: Color::Rgb(40, 140, 80),
            config_toggle_off: Color::Rgb(200, 80, 80),
            config_dim: Color::Rgb(170, 170, 180),
            config_api_key: Color::Rgb(160, 160, 170),

            // Markdown
            md_h1: Color::Rgb(30, 80, 180),
            md_h2: Color::Rgb(40, 100, 200),
            md_h3: Color::Rgb(50, 110, 210),
            md_h4: Color::Rgb(60, 120, 220),
            md_heading_sep: Color::Rgb(180, 190, 210),
            md_inline_code_fg: Color::Rgb(160, 80, 30),
            md_inline_code_bg: Color::Rgb(240, 235, 225),
            md_list_bullet: Color::Rgb(30, 100, 200),
            md_blockquote_bar: Color::Rgb(100, 130, 180),
            md_blockquote_text: Color::Rgb(80, 90, 110),
            md_rule: Color::Rgb(190, 195, 210),

            // 代码块（VS Code Light+ 风格）
            code_border: Color::Rgb(190, 195, 210),
            code_bg: Color::Rgb(245, 245, 248),
            code_default: Color::Rgb(40, 40, 50),
            code_keyword: Color::Rgb(175, 0, 219),
            code_string: Color::Rgb(163, 21, 21),
            code_comment: Color::Rgb(0, 128, 0),
            code_number: Color::Rgb(9, 134, 88),
            code_type: Color::Rgb(38, 127, 153),
            code_primitive: Color::Rgb(0, 112, 193),
            code_macro: Color::Rgb(121, 94, 38),
            code_attribute: Color::Rgb(0, 112, 193),
            code_lifetime: Color::Rgb(38, 127, 153),
            code_shell_var: Color::Rgb(0, 112, 193),

            // 表格
            table_border: Color::Rgb(180, 190, 210),
            table_header: Color::Rgb(30, 80, 180),
            table_body: Color::Rgb(60, 60, 80),

            // 帮助界面
            help_title: Color::Rgb(40, 100, 200),
            help_key: Color::Rgb(140, 110, 30),
            help_desc: Color::Rgb(50, 50, 60),
            help_path: Color::Rgb(120, 120, 140),
            help_bg: Color::Rgb(248, 248, 248),
        }
    }

    /// Nord 主题（基于 Nord 配色方案 - 极地冰蓝色调）
    pub fn nord() -> Self {
        Self {
            // 全局背景 — Polar Night
            bg_primary: Color::Rgb(46, 52, 64), // nord0
            bg_title: Color::Rgb(59, 66, 82),   // nord1
            bg_input: Color::Rgb(59, 66, 82),   // nord1
            bg_panel: Color::Rgb(59, 66, 82),   // nord1

            // 边框 — Polar Night / Snow Storm
            border_title: Color::Rgb(76, 86, 106),  // nord3
            border_message: Color::Rgb(67, 76, 94), // nord2
            border_input: Color::Rgb(76, 86, 106),  // nord3
            border_input_loading: Color::Rgb(235, 203, 139), // nord13
            border_config: Color::Rgb(76, 86, 106), // nord3
            separator: Color::Rgb(67, 76, 94),      // nord2

            // 气泡
            bubble_ai: Color::Rgb(50, 56, 68),
            bubble_ai_selected: Color::Rgb(255, 255, 255),
            bubble_user: Color::Rgb(52, 75, 110),
            bubble_user_selected: Color::Rgb(255, 255, 255),

            // 标签 — Frost / Aurora
            label_ai: Color::Rgb(163, 190, 140),       // nord14
            label_user: Color::Rgb(129, 161, 193),     // nord9
            label_selected: Color::Rgb(235, 203, 139), // nord13

            // 文字
            text_normal: Color::Rgb(216, 222, 233), // nord4
            text_bold: Color::Rgb(210, 235, 220),
            text_dim: Color::Rgb(128, 140, 160),
            text_very_dim: Color::Rgb(76, 86, 106), // nord3
            text_white: Color::Rgb(236, 239, 244),  // nord6
            text_system: Color::Rgb(100, 112, 130),

            // 标题栏
            title_icon: Color::Rgb(136, 192, 208),   // nord8
            title_separator: Color::Rgb(67, 76, 94), // nord2
            title_model: Color::Rgb(163, 190, 140),  // nord14
            title_count: Color::Rgb(178, 186, 202),
            title_loading: Color::Rgb(235, 203, 139), // nord13

            // 输入区
            input_prompt: Color::Rgb(163, 190, 140), // nord14
            input_prompt_loading: Color::Rgb(235, 203, 139), // nord13
            cursor_fg: Color::Rgb(46, 52, 64),       // nord0
            cursor_bg: Color::Rgb(216, 222, 233),    // nord4

            // 提示栏
            hint_key_fg: Color::Rgb(46, 52, 64),  // nord0
            hint_key_bg: Color::Rgb(76, 86, 106), // nord3
            hint_desc: Color::Rgb(128, 140, 160),
            hint_separator: Color::Rgb(59, 66, 82), // nord1

            // Toast
            toast_success_border: Color::Rgb(163, 190, 140), // nord14
            toast_success_bg: Color::Rgb(50, 60, 55),
            toast_success_text: Color::Rgb(163, 190, 140),
            toast_error_border: Color::Rgb(191, 97, 106), // nord11
            toast_error_bg: Color::Rgb(60, 50, 52),
            toast_error_text: Color::Rgb(191, 97, 106),

            // 工具确认区
            tool_confirm_border: Color::Rgb(235, 203, 139), // nord13
            tool_confirm_bg: Color::Rgb(52, 58, 70),
            tool_confirm_title: Color::Rgb(235, 203, 139),
            tool_confirm_name: Color::Rgb(235, 203, 139),
            tool_confirm_text: Color::Rgb(216, 222, 233), // nord4
            tool_confirm_label: Color::Rgb(128, 140, 160),
            tool_confirm_hint: Color::Rgb(178, 186, 202),

            // 欢迎界面
            welcome_border: Color::Rgb(76, 86, 106), // nord3
            welcome_text: Color::Rgb(136, 192, 208), // nord8
            welcome_hint: Color::Rgb(100, 112, 130),

            // 模型选择
            model_sel_border: Color::Rgb(235, 203, 139), // nord13
            model_sel_title: Color::Rgb(235, 203, 139),
            model_sel_active: Color::Rgb(163, 190, 140), // nord14
            model_sel_inactive: Color::Rgb(178, 186, 202),
            model_sel_highlight_bg: Color::Rgb(67, 76, 94), // nord2

            // 配置界面
            config_title: Color::Rgb(129, 161, 193), // nord9
            config_section: Color::Rgb(163, 190, 140), // nord14
            config_pointer: Color::Rgb(235, 203, 139), // nord13
            config_label_selected: Color::Rgb(235, 203, 139),
            config_label: Color::Rgb(128, 140, 160),
            config_value: Color::Rgb(178, 186, 202),
            config_edit_bg: Color::Rgb(67, 76, 94), // nord2
            config_tab_active_bg: Color::Rgb(129, 161, 193), // nord9
            config_tab_active_fg: Color::Rgb(46, 52, 64), // nord0
            config_tab_inactive: Color::Rgb(128, 140, 160),
            config_hint_key: Color::Rgb(235, 203, 139),
            config_hint_desc: Color::Rgb(128, 140, 160),
            config_toggle_on: Color::Rgb(163, 190, 140), // nord14
            config_toggle_off: Color::Rgb(191, 97, 106), // nord11
            config_dim: Color::Rgb(76, 86, 106),         // nord3
            config_api_key: Color::Rgb(100, 112, 130),

            // Markdown — Frost colors
            md_h1: Color::Rgb(136, 192, 208), // nord8
            md_h2: Color::Rgb(129, 161, 193), // nord9
            md_h3: Color::Rgb(143, 188, 187), // nord7
            md_h4: Color::Rgb(178, 186, 202),
            md_heading_sep: Color::Rgb(67, 76, 94), // nord2
            md_inline_code_fg: Color::Rgb(235, 203, 139), // nord13
            md_inline_code_bg: Color::Rgb(59, 66, 82), // nord1
            md_list_bullet: Color::Rgb(129, 161, 193), // nord9
            md_blockquote_bar: Color::Rgb(76, 86, 106), // nord3
            md_blockquote_text: Color::Rgb(160, 170, 185),
            md_rule: Color::Rgb(67, 76, 94), // nord2

            // 代码块 — Nord 风格语法高亮
            code_border: Color::Rgb(76, 86, 106),    // nord3
            code_bg: Color::Rgb(46, 52, 64),         // nord0
            code_default: Color::Rgb(216, 222, 233), // nord4
            code_keyword: Color::Rgb(180, 142, 173), // nord15
            code_string: Color::Rgb(163, 190, 140),  // nord14
            code_comment: Color::Rgb(97, 110, 128),
            code_number: Color::Rgb(208, 135, 112), // nord12
            code_type: Color::Rgb(235, 203, 139),   // nord13
            code_primitive: Color::Rgb(143, 188, 187), // nord7
            code_macro: Color::Rgb(136, 192, 208),  // nord8
            code_attribute: Color::Rgb(143, 188, 187), // nord7
            code_lifetime: Color::Rgb(235, 203, 139), // nord13
            code_shell_var: Color::Rgb(143, 188, 187), // nord7

            // 表格
            table_border: Color::Rgb(67, 76, 94),    // nord2
            table_header: Color::Rgb(136, 192, 208), // nord8
            table_body: Color::Rgb(178, 186, 202),

            // 帮助界面
            help_title: Color::Rgb(136, 192, 208), // nord8
            help_key: Color::Rgb(235, 203, 139),   // nord13
            help_desc: Color::Rgb(216, 222, 233),  // nord4
            help_path: Color::Rgb(100, 112, 130),
            help_bg: Color::Rgb(59, 66, 82), // nord1
        }
    }

    /// Monokai 主题（经典 Monokai 配色 - 暖色调高对比度）
    pub fn monokai() -> Self {
        Self {
            // 全局背景
            bg_primary: Color::Rgb(39, 40, 34),
            bg_title: Color::Rgb(49, 50, 44),
            bg_input: Color::Rgb(49, 50, 44),
            bg_panel: Color::Rgb(49, 50, 44),

            // 边框
            border_title: Color::Rgb(80, 80, 70),
            border_message: Color::Rgb(65, 65, 55),
            border_input: Color::Rgb(80, 80, 70),
            border_input_loading: Color::Rgb(230, 219, 116), // monokai yellow
            border_config: Color::Rgb(80, 80, 70),
            separator: Color::Rgb(65, 65, 55),

            // 气泡
            bubble_ai: Color::Rgb(43, 44, 38),
            bubble_ai_selected: Color::Rgb(255, 255, 255),
            bubble_user: Color::Rgb(55, 65, 90),
            bubble_user_selected: Color::Rgb(255, 255, 255),

            // 标签
            label_ai: Color::Rgb(166, 226, 46),    // monokai green
            label_user: Color::Rgb(102, 217, 239), // monokai cyan
            label_selected: Color::Rgb(230, 219, 116), // monokai yellow

            // 文字
            text_normal: Color::Rgb(248, 248, 242), // monokai foreground
            text_bold: Color::Rgb(215, 245, 225),
            text_dim: Color::Rgb(140, 140, 130),
            text_very_dim: Color::Rgb(90, 90, 80),
            text_white: Color::Rgb(248, 248, 242),
            text_system: Color::Rgb(117, 113, 94), // monokai comment color

            // 标题栏
            title_icon: Color::Rgb(102, 217, 239), // monokai cyan
            title_separator: Color::Rgb(65, 65, 55),
            title_model: Color::Rgb(166, 226, 46), // monokai green
            title_count: Color::Rgb(190, 190, 180),
            title_loading: Color::Rgb(230, 219, 116), // monokai yellow

            // 输入区
            input_prompt: Color::Rgb(166, 226, 46), // monokai green
            input_prompt_loading: Color::Rgb(230, 219, 116),
            cursor_fg: Color::Rgb(39, 40, 34),
            cursor_bg: Color::Rgb(248, 248, 242),

            // 提示栏
            hint_key_fg: Color::Rgb(39, 40, 34),
            hint_key_bg: Color::Rgb(117, 113, 94),
            hint_desc: Color::Rgb(140, 140, 130),
            hint_separator: Color::Rgb(55, 55, 45),

            // Toast
            toast_success_border: Color::Rgb(166, 226, 46),
            toast_success_bg: Color::Rgb(45, 55, 38),
            toast_success_text: Color::Rgb(166, 226, 46),
            toast_error_border: Color::Rgb(249, 38, 114), // monokai pink
            toast_error_bg: Color::Rgb(60, 38, 42),
            toast_error_text: Color::Rgb(249, 38, 114),

            // 工具确认区
            tool_confirm_border: Color::Rgb(230, 219, 116), // monokai yellow
            tool_confirm_bg: Color::Rgb(50, 48, 38),
            tool_confirm_title: Color::Rgb(230, 219, 116),
            tool_confirm_name: Color::Rgb(230, 219, 116),
            tool_confirm_text: Color::Rgb(248, 248, 242),
            tool_confirm_label: Color::Rgb(140, 140, 130),
            tool_confirm_hint: Color::Rgb(190, 190, 180),

            // 欢迎界面
            welcome_border: Color::Rgb(80, 80, 70),
            welcome_text: Color::Rgb(102, 217, 239), // monokai cyan
            welcome_hint: Color::Rgb(100, 100, 90),

            // 模型选择
            model_sel_border: Color::Rgb(230, 219, 116),
            model_sel_title: Color::Rgb(230, 219, 116),
            model_sel_active: Color::Rgb(166, 226, 46),
            model_sel_inactive: Color::Rgb(190, 190, 180),
            model_sel_highlight_bg: Color::Rgb(60, 62, 54),

            // 配置界面
            config_title: Color::Rgb(102, 217, 239),
            config_section: Color::Rgb(166, 226, 46),
            config_pointer: Color::Rgb(230, 219, 116),
            config_label_selected: Color::Rgb(230, 219, 116),
            config_label: Color::Rgb(140, 140, 130),
            config_value: Color::Rgb(190, 190, 180),
            config_edit_bg: Color::Rgb(60, 62, 54),
            config_tab_active_bg: Color::Rgb(102, 217, 239),
            config_tab_active_fg: Color::Rgb(39, 40, 34),
            config_tab_inactive: Color::Rgb(140, 140, 130),
            config_hint_key: Color::Rgb(230, 219, 116),
            config_hint_desc: Color::Rgb(140, 140, 130),
            config_toggle_on: Color::Rgb(166, 226, 46),
            config_toggle_off: Color::Rgb(249, 38, 114),
            config_dim: Color::Rgb(90, 90, 80),
            config_api_key: Color::Rgb(100, 100, 90),

            // Markdown
            md_h1: Color::Rgb(249, 38, 114),  // monokai pink
            md_h2: Color::Rgb(102, 217, 239), // monokai cyan
            md_h3: Color::Rgb(166, 226, 46),  // monokai green
            md_h4: Color::Rgb(230, 219, 116), // monokai yellow
            md_heading_sep: Color::Rgb(80, 80, 70),
            md_inline_code_fg: Color::Rgb(230, 219, 116), // monokai yellow
            md_inline_code_bg: Color::Rgb(55, 55, 45),
            md_list_bullet: Color::Rgb(249, 38, 114), // monokai pink
            md_blockquote_bar: Color::Rgb(117, 113, 94),
            md_blockquote_text: Color::Rgb(170, 170, 160),
            md_rule: Color::Rgb(80, 80, 70),

            // 代码块 — Monokai 经典语法高亮
            code_border: Color::Rgb(80, 80, 70),
            code_bg: Color::Rgb(39, 40, 34),
            code_default: Color::Rgb(248, 248, 242), // monokai foreground
            code_keyword: Color::Rgb(249, 38, 114),  // monokai pink
            code_string: Color::Rgb(230, 219, 116),  // monokai yellow
            code_comment: Color::Rgb(117, 113, 94),  // monokai comment
            code_number: Color::Rgb(174, 129, 255),  // monokai purple
            code_type: Color::Rgb(166, 226, 46),     // monokai green
            code_primitive: Color::Rgb(102, 217, 239), // monokai cyan
            code_macro: Color::Rgb(102, 217, 239),   // monokai cyan
            code_attribute: Color::Rgb(166, 226, 46), // monokai green
            code_lifetime: Color::Rgb(174, 129, 255), // monokai purple
            code_shell_var: Color::Rgb(102, 217, 239), // monokai cyan

            // 表格
            table_border: Color::Rgb(80, 80, 70),
            table_header: Color::Rgb(102, 217, 239),
            table_body: Color::Rgb(190, 190, 180),

            // 帮助界面
            help_title: Color::Rgb(102, 217, 239),
            help_key: Color::Rgb(230, 219, 116),
            help_desc: Color::Rgb(248, 248, 242),
            help_path: Color::Rgb(117, 113, 94),
            help_bg: Color::Rgb(49, 50, 44),
        }
    }
}
