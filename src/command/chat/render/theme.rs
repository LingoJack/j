//! 向后兼容的 re-export
//!
//! Theme 已上移到 crate::theme，此文件仅做桥接。
// SAFETY: 仅 re-export，无实际逻辑
#![allow(clippy::single_component_path_imports)]
pub use crate::theme::{Theme, ThemeName};
