// ========== 数据结构与常量 ==========

use serde::{Deserialize, Serialize};

// ========== 命令面板 ==========

/// 命令面板选项列表 (key, 中文标签)
pub const CMD_POPUP_ITEMS: &[(&str, &str)] = &[
    ("toggle", "切换完成"),
    ("edit", "编辑"),
    ("add", "添加"),
    ("delete", "删除"),
    ("copy", "复制"),
    ("filter", "切换过滤"),
    ("moveup", "上移排序"),
    ("movedown", "下移排序"),
    ("save", "保存"),
    ("quit", "退出"),
    ("help", "帮助"),
];

// ========== 数据结构 ==========

/// 单条待办事项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoItem {
    /// 待办内容
    pub content: String,
    /// 是否已完成
    pub done: bool,
    /// 创建时间
    pub created_at: String,
    /// 完成时间（可选）
    pub done_at: Option<String>,
}

/// 待办列表（序列化到 JSON）
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
}

#[derive(PartialEq, Clone)]
/// Todo 应用模式枚举
pub enum AppMode {
    /// 正常浏览模式
    Normal,
    /// 输入添加模式
    Adding,
    /// 编辑模式
    Editing,
    /// 确认删除
    ConfirmDelete,
    /// 确认写入日报
    ConfirmReport,
    /// 确认取消输入（有内容变化时）
    ConfirmCancelInput,
    /// 显示帮助
    Help,
    /// 命令面板
    CommandPopup,
}
