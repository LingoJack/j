//! 向后兼容的 re-export + UI 扩展方法
//!
//! Theme 已上移到 crate::theme，此文件仅做桥接。
//! ToolCategory/ToolStatus 的 color() 方法通过扩展 trait 实现（依赖 ratatui Theme）。

#![allow(clippy::single_component_path_imports)]
pub use crate::theme::{Theme, ThemeName};
use j_agent::tools::classification::{ToolCategory, ToolStatus};
use ratatui::style::Color;

/// 工具分类颜色扩展 trait
pub trait ToolCategoryColor {
    fn color(&self, theme: &Theme) -> Color;
}

impl ToolCategoryColor for ToolCategory {
    fn color(&self, theme: &Theme) -> Color {
        match self {
            Self::File => theme.label_user,
            Self::Search => theme.label_ai,
            Self::Execute => theme.title_loading,
            Self::Network => theme.config_title,
            Self::Plan => theme.label_ai,
            Self::Agent => theme.title_loading,
            Self::Teammate => theme.config_title,
            Self::Compact => theme.config_title,
            Self::SendMessage => theme.config_title,
            Self::IgnoreMessage => theme.text_dim,
            Self::WorkDone => theme.label_ai,
            Self::Other => theme.text_dim,
        }
    }
}

/// 工具状态颜色扩展 trait
pub trait ToolStatusColor {
    fn color(&self, theme: &Theme) -> Color;
}

impl ToolStatusColor for ToolStatus {
    fn color(&self, theme: &Theme) -> Color {
        match self {
            Self::Success => theme.label_ai,
            Self::Failed => theme.toast_error_border,
        }
    }
}
