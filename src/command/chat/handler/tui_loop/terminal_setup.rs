//! 终端生命周期管理（RAII guard + 恢复）

use crossterm::{
    event::{
        self, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute, terminal,
};
use std::io;

/// RAII guard：确保 TUI 退出时（含 panic / `?` 传播）恢复终端到正常状态。
///
/// 正常退出路径调用 [`TerminalGuard::disarm`] 后，`Drop` 不再重复恢复。
/// 异常路径（panic、loop 内 `?` 提前返回）由 `Drop` 兜底执行完整恢复序列。
pub(super) struct TerminalGuard {
    /// keyboard enhancement 协议是否已 push
    keyboard_enhancement_active: bool,
    /// 是否已手动恢复（disarm），避免 Drop 重复恢复
    disarmed: bool,
}

impl TerminalGuard {
    pub(super) fn new() -> Self {
        Self {
            keyboard_enhancement_active: false,
            disarmed: false,
        }
    }

    /// 标记 `PushKeyboardEnhancementFlags` 已执行成功
    pub(super) fn set_keyboard_active(&mut self) {
        self.keyboard_enhancement_active = true;
    }

    /// 正常退出路径：手动完成恢复后调用，阻止 `Drop` 再次恢复。
    pub(super) fn disarm(&mut self) {
        self.disarmed = true;
    }

    /// keyboard enhancement 是否已激活
    pub(super) fn is_keyboard_active(&self) -> bool {
        self.keyboard_enhancement_active
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        let _ = terminal::disable_raw_mode();
        // 使用 io::stdout() 而非 terminal.backend_mut()，
        // 因为 Drop 发生时 terminal 可能已经被 move 或 drop。
        let mut stdout = io::stdout();
        let _ = restore_terminal_state(&mut stdout, self.keyboard_enhancement_active);
    }
}

/// 尝试启用 keyboard enhancement。
///
/// 部分终端会直接忽略该协议，但 legacy WindowsAPI 会显式返回错误。
/// 这里将其视为可选能力：失败时继续运行，只是少了更细粒度的按键区分。
pub(super) fn try_enable_keyboard_enhancement<W: io::Write>(writer: &mut W) -> bool {
    execute!(
        writer,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )
    .is_ok()
}

/// 恢复终端状态。
///
/// keyboard enhancement 的 `Pop` 必须单独处理，避免其失败时短路后续恢复步骤。
pub(super) fn restore_terminal_state<W: io::Write>(
    writer: &mut W,
    keyboard_enhancement_active: bool,
) -> io::Result<()> {
    if keyboard_enhancement_active {
        let _ = execute!(writer, PopKeyboardEnhancementFlags);
    }

    execute!(
        writer,
        event::DisableMouseCapture,
        event::DisableBracketedPaste,
        terminal::LeaveAlternateScreen
    )
}

/// 恢复终端状态（仅用于 panic hook）。
/// panic 发生时 `TerminalGuard` 也会 Drop 恢复，此处作为双重保险。
pub(super) fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = restore_terminal_state(&mut stdout, true);
}
