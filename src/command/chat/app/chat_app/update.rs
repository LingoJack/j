use super::ChatApp;
use crate::command::chat::app::action::Action;

impl ChatApp {
    /// Redux-like reducer：集中处理所有 Action，分发到具体方法
    ///
    /// 该方法是 unidirectional data flow 的核心：
    /// 1. 接收 Action（用户输入或系统事件）
    /// 2. 根据 Action 类型和当前状态执行相应操作
    /// 3. 修改 self.state、self.ui、self.tool_executor 等
    /// 4. 不再直接在 handler 中修改状态
    pub fn update(&mut self, action: Action) {
        match action {
            // ========== Chat 输入和文本编辑 ==========
            Action::SendMessage => self.send_message(),
            Action::InsertChar(ch) => self.update_insert_char(ch),
            Action::DeleteChar => self.ui.input_buffer.backspace(),
            Action::DeleteForward => self.ui.input_buffer.delete_char(),
            Action::MoveCursor(dir) => self.update_move_cursor(dir),
            Action::ClearInput => self.ui.clear_input(),

            // ========== 弹窗交互 ==========
            Action::AtPopupActivate => self.update_at_popup_activate(),
            Action::AtPopupClose => self.ui.at_popup_active = false,
            Action::AtPopupFilter(text) => self.update_at_popup_filter(text),
            Action::AtPopupNavigate(dir) => self.update_at_popup_navigate(dir),
            Action::AtPopupConfirm => {}
            Action::FilePopupActivate => self.update_file_popup_activate(),
            Action::FilePopupClose => self.ui.file_popup_active = false,
            Action::FilePopupFilter(text) => self.update_file_popup_filter(text),
            Action::FilePopupNavigate(_dir) => {}
            Action::FilePopupConfirm => {}
            Action::SkillPopupActivate => self.update_skill_popup_activate(),
            Action::SkillPopupClose => self.ui.skill_popup_active = false,
            Action::SkillPopupFilter(text) => self.update_skill_popup_filter(text),
            Action::SkillPopupNavigate(dir) => self.update_skill_popup_navigate(dir),
            Action::SkillPopupConfirm => {}

            // ========== 流式生命周期 ==========
            Action::StreamChunk => self.update_stream_chunk(),
            Action::ToolCallRequest(_tool_calls) => {}
            Action::StreamDone => self.update_stream_done(),
            Action::StreamError(ref e) => self.update_stream_error(e),
            Action::StreamCancelled => self.update_stream_cancelled(),
            Action::StreamRetrying {
                attempt,
                max_attempts,
                delay_ms,
                error,
            } => {
                self.update_stream_retrying(attempt, max_attempts, delay_ms, error);
            }
            Action::StreamCompacting => self.update_stream_compacting(),
            Action::StreamCompacted { messages_before: _ } => self.update_stream_compacted(),

            // ========== 工具执行 ==========
            Action::ExecutePendingTool => self.execute_pending_tool(),
            Action::RejectPendingTool => self.reject_pending_tool(""),
            Action::RejectPendingToolWithReason(ref reason) => self.reject_pending_tool(reason),
            Action::AllowAndExecutePendingTool => self.allow_and_execute_pending_tool(),

            // ========== Ask 工具交互 ==========
            Action::AskNavigate(dir) => self.update_ask_navigate(dir),
            Action::AskOptionNavigate(dir) => self.update_ask_option_navigate(dir),
            Action::AskSingleSelect => self.update_ask_single_select(),
            Action::AskToggleMultiSelect => self.update_ask_toggle_multi_select(),
            Action::AskInputChar(c) => self.update_ask_input_char(c),
            Action::AskDeleteChar => self.update_ask_delete_char(),
            Action::AskSubmitAnswer => self.update_ask_submit_answer(),
            Action::AskCancel => self.update_ask_cancel(),

            // ========== 工具交互区 ==========
            Action::ToolInteractNavigate(dir) => self.update_tool_interact_navigate(dir),
            Action::ToolInteractInputChar(c) => self.update_tool_interact_input_char(c),
            Action::ToolInteractDeleteChar => self.update_tool_interact_delete_char(),
            Action::ToolInteractConfirm => self.update_tool_interact_confirm(),

            // ========== 模式切换和导航 ==========
            Action::EnterMode(mode) => self.update_enter_mode(mode),
            Action::ExitToChat => self.update_exit_to_chat(),
            Action::Scroll(dir) => self.update_scroll(dir),
            Action::PageScroll(dir) => self.update_page_scroll(dir),
            Action::BrowseNavigate(dir) => self.update_browse_navigate(dir),
            Action::BrowseFineScroll(dir) => self.update_browse_fine_scroll(dir),
            Action::BrowseCopyMessage => self.update_browse_copy_message(),
            Action::BrowseInputChar(c) => self.update_browse_input_char(c),
            Action::BrowseDeleteChar => self.update_browse_delete_char(),
            Action::BrowseClearFilter => self.update_browse_clear_filter(),
            Action::BrowseToggleRole => self.update_browse_toggle_role(),

            // ========== 配置编辑 ==========
            Action::ConfigNavigate(dir) => self.update_config_navigate(dir),
            Action::ConfigEnter => self.update_config_enter(),
            Action::ConfigEditChar(c) => self.update_config_edit_char(c),
            Action::ConfigEditDelete => self.update_config_edit_delete(),
            Action::ConfigEditDeleteForward => self.update_config_edit_delete_forward(),
            Action::ConfigEditMoveCursor(dir) => self.update_config_edit_move_cursor(dir),
            Action::ConfigEditMoveHome => self.ui.config_edit_cursor = 0,
            Action::ConfigEditMoveEnd => {
                self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
            }
            Action::ConfigEditClearLine => self.update_config_edit_clear_line(),
            Action::ConfigEditSubmit => self.update_config_edit_submit(),
            Action::ConfigAddProvider => self.update_config_add_provider(),
            Action::ConfigDeleteProvider => self.update_config_delete_provider(),
            Action::ConfigSetActiveProvider => self.update_config_set_active_provider(),
            Action::ConfigSwitchTab(dir) => self.update_config_switch_tab(dir),
            Action::ConfigSwitchTabTo(tab) => self.update_config_switch_tab_to(tab),
            Action::ConfigFieldSelect(idx) => self.update_config_field_select(idx),
            Action::ConfigProviderSelect(idx) => self.update_config_provider_select(idx),
            Action::ToggleMenuNavigate(dir) => self.update_toggle_menu_navigate(dir),
            Action::ToggleMenuToggle => self.update_toggle_menu_toggle(),
            Action::ToggleMenuEnableAll => self.update_toggle_menu_enable_all(),
            Action::ToggleMenuDisableAll => self.update_toggle_menu_disable_all(),
            Action::ToolsToggleLevel => self.update_tools_toggle_level(),
            Action::ModelToggleLevel => self.update_model_toggle_level(),
            Action::TeammatesNavigate(dir) => self.update_teammates_navigate(dir),
            Action::TeammatesSelect(idx) => self.update_teammates_select(idx),
            Action::CompactExemptToggle => self.update_compact_exempt_toggle(),
            Action::CompactExemptSelect(idx) => self.update_compact_exempt_select(idx),

            // ========== 模型选择 ==========
            Action::ModelSelectNavigate(dir) => self.update_model_select_navigate(dir),
            Action::ModelSelectConfirm => self.switch_model(),

            // ========== 主题选择 ==========
            Action::ThemeSelectNavigate(dir) => self.update_theme_select_navigate(dir),
            Action::ThemeSelectConfirm => self.update_theme_select_confirm(),

            // ========== 归档管理 ==========
            Action::StartArchiveConfirm => self.start_archive_confirm(),
            Action::ArchiveConfirmEditName => self.ui.archive_editing_name = true,
            Action::ArchiveConfirmMoveCursor(dir) => self.update_archive_confirm_move_cursor(dir),
            Action::ArchiveConfirmInputChar(c) => self.update_archive_confirm_input_char(c),
            Action::ArchiveConfirmDeleteChar => self.update_archive_confirm_delete_char(),
            Action::ArchiveWithDefault => self.do_archive(&self.ui.archive_default_name.clone()),
            Action::ArchiveWithCustom => self.do_archive(&self.ui.archive_custom_name.clone()),
            Action::ClearSession => self.clear_session(),
            Action::ListSessions => self.update_list_sessions(),
            Action::SwitchSession { session_id } => self.update_switch_session(session_id),
            Action::NewSession => self.update_new_session(),
            Action::LoadSessionList => self.update_load_session_list(),
            Action::SessionListNavigate(dir) => self.update_session_list_navigate(dir),
            Action::SessionListSelect(idx) => self.update_session_list_select(idx),
            Action::RestoreSession => self.update_restore_session(),
            Action::DeleteSession => self.update_delete_session(),
            Action::NewSessionFromList => self.update_new_session_from_list(),
            Action::RespawnTeammate { name } => self.update_respawn_teammate(name),
            Action::SessionStateRestored => self.show_toast("会话状态已恢复".to_string(), false),
            Action::StartArchiveList => self.start_archive_list(),
            Action::ArchiveListNavigate(dir) => self.update_archive_list_navigate(dir),
            Action::ArchiveListSelect(idx) => self.update_archive_list_select(idx),
            Action::RestoreArchive => self.do_restore(),
            Action::DeleteArchive => self.do_delete_archive(),

            // ========== 模型和主题 ==========
            Action::SwitchModel => {
                self.ui.mode = crate::command::chat::app::ui_state::ChatMode::SelectModel
            }
            Action::SwitchTheme => self.switch_theme(),

            // ========== 流式控制 ==========
            Action::CancelStream => self.update_cancel_stream(),
            Action::CancelToolsOnly => self.cancel_tools_only(),

            // ========== UI 管理 ==========
            Action::ShowToast(msg, is_error) => self.show_toast(msg, is_error),
            Action::TickToast => self.tick_toast(),
            Action::SaveConfig => self.update_save_config(),

            // ========== 快速操作 ==========
            Action::CopyLastAiReply => self.update_copy_last_ai_reply(),
            Action::ShowHelp => self.ui.mode = crate::command::chat::app::ui_state::ChatMode::Help,
            Action::OpenLogWindows => self.update_open_log_windows(),

            // ========== 应用控制 ==========
            Action::Quit => {}
            Action::ToggleExpandTools => self.update_toggle_expand_tools(),
            Action::ToggleAutoApprove => self.update_toggle_auto_approve(),

            // ========== Commands 创建 ==========
            Action::ConfigCreateCommandSelectSource => self.update_config_command_select_source(),
            Action::ConfigCreateCommandNavigateSource(dir) => {
                self.update_config_command_navigate_source(dir)
            }
            Action::ConfigCreateCommandConfirmSource => self.update_config_command_confirm_source(),
            Action::ConfigCreateCommandCancel => self.update_config_command_cancel(),
        }
    }
}
