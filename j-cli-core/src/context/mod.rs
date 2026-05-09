//! 上下文管理模块：统一管理消息压缩、窗口选择、Plan 状态等
//!
//! 包含以下子模块：
//! - `policy`：工具上下文优先级策略（统一源头，compact/window 都查询此处）
//! - `compact`：核心压缩引擎（micro_compact / auto_compact）
//! - `window`：消息窗口选择（三阶段优先级策略）
//! - `message_compress`：其他 agent tool call 压缩
//! - `plan_state`：Plan mode 状态管理

pub mod compact;
pub mod message_compress;
pub mod plan_state;
pub mod policy;
pub mod window;

#[cfg(test)]
mod regression_tests;
