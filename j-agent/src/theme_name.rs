//! 主题名称定义（不含 ratatui 依赖）
//!
//! Theme 结构体（含 ratatui Color）留在 j-cli 的 theme.rs 中。

use serde::{Deserialize, Serialize};

/// 主题名称枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ThemeName {
    #[serde(rename = "terminal")]
    Terminal,
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
            "terminal" => Ok(ThemeName::Terminal),
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

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeName::Terminal => write!(f, "terminal"),
            ThemeName::Dark => write!(f, "dark"),
            ThemeName::Light => write!(f, "light"),
            ThemeName::Midnight => write!(f, "midnight"),
            ThemeName::Nord => write!(f, "nord"),
            ThemeName::Monokai => write!(f, "monokai"),
            ThemeName::AnthropicLight => write!(f, "anthropic_light"),
            ThemeName::AnthropicDark => write!(f, "anthropic_dark"),
        }
    }
}

impl ThemeName {
    /// 获取所有主题名称列表
    pub fn all() -> &'static [ThemeName] {
        &[
            ThemeName::Terminal,
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
            ThemeName::Terminal => ThemeName::Dark,
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::Midnight,
            ThemeName::Midnight => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Monokai,
            ThemeName::Monokai => ThemeName::AnthropicLight,
            ThemeName::AnthropicLight => ThemeName::AnthropicDark,
            ThemeName::AnthropicDark => ThemeName::Terminal,
        }
    }

    /// 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeName::Terminal => "Terminal（跟随终端背景）",
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
            ThemeName::Terminal => "terminal",
            ThemeName::Dark => "dark",
            ThemeName::Light => "light",
            ThemeName::Midnight => "midnight",
            ThemeName::Nord => "nord",
            ThemeName::Monokai => "monokai",
            ThemeName::AnthropicLight => "anthropic_light",
            ThemeName::AnthropicDark => "anthropic_dark",
        }
    }
}
