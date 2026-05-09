//! 主题名称定义（不含 ratatui 依赖）
//!
//! Theme 结构体（含 ratatui Color）留在 j-cli 的 theme.rs 中。

use serde::{Deserialize, Serialize};

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

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
