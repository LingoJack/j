use super::ChatApp;
use crate::command::chat::app::action::CursorDirection;
use crate::command::chat::app::ui_state::ConfigTab;
use crate::command::chat::infra::command;
use crate::command::chat::storage::ModelProvider;
use crate::constants::CONFIG_FIELDS;

impl ChatApp {
    pub(super) fn update_config_navigate(&mut self, dir: CursorDirection) {
        // Model tab 使用层级导航
        if self.ui.config_tab == ConfigTab::Model {
            if self.ui.model_in_fields {
                // 字段层级：上下切换配置字段
                let total = CONFIG_FIELDS.len();
                if total == 0 {
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.model_field_idx > 0 {
                            self.ui.model_field_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        if self.ui.model_field_idx < total - 1 {
                            self.ui.model_field_idx += 1;
                        }
                    }
                }
            } else {
                // Provider 列表层级：上下切换 Provider
                let count = self.state.agent_config.providers.len();
                if count == 0 {
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.config_provider_idx == 0 {
                            self.ui.config_provider_idx = count - 1;
                        } else {
                            self.ui.config_provider_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        self.ui.config_provider_idx = (self.ui.config_provider_idx + 1) % count;
                    }
                }
            }
            return;
        }

        // 其他 Tab 保持原有逻辑
        let total_fields = super::config_tab_field_count(self);
        if total_fields == 0 {
            return;
        }
        match dir {
            CursorDirection::Up => {
                if self.ui.config_field_idx > 0 {
                    self.ui.config_field_idx -= 1;
                }
            }
            CursorDirection::Down => {
                if self.ui.config_field_idx < total_fields - 1 {
                    self.ui.config_field_idx += 1;
                }
            }
        }
    }

    pub(super) fn update_config_enter(&mut self) {
        use crate::command::chat::render::helpers::{
            config_field_raw_value_global, config_field_raw_value_model,
        };
        use crate::constants::CONFIG_GLOBAL_FIELDS_TAB;
        match self.ui.config_tab {
            ConfigTab::Model => {
                if self.state.agent_config.providers.is_empty() {
                    self.show_toast("还没有 Provider，按 a 新增", true);
                    return;
                }
                // 必须在字段层级才能编辑
                if !self.ui.model_in_fields {
                    return;
                }
                // supports_vision 是布尔开关，直接 toggle
                if self.ui.model_field_idx < CONFIG_FIELDS.len()
                    && CONFIG_FIELDS[self.ui.model_field_idx] == "supports_vision"
                    && let Some(p) = self
                        .state
                        .agent_config
                        .providers
                        .get_mut(self.ui.config_provider_idx)
                {
                    p.supports_vision = !p.supports_vision;
                    let status = if p.supports_vision {
                        "开启"
                    } else {
                        "关闭"
                    };
                    self.show_toast(format!("当前 Provider 支持视觉已{}", status), false);
                    return;
                }
                self.ui.config_edit_buf =
                    config_field_raw_value_model(self, self.ui.model_field_idx);
                self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
                self.ui.config_editing = true;
            }
            ConfigTab::Global => {
                let idx = self.ui.config_field_idx;
                if idx < CONFIG_GLOBAL_FIELDS_TAB.len() {
                    let field = CONFIG_GLOBAL_FIELDS_TAB[idx];
                    if field == "auto_restore_session" {
                        self.state.agent_config.auto_restore_session =
                            !self.state.agent_config.auto_restore_session;
                        let status = if self.state.agent_config.auto_restore_session {
                            "开启"
                        } else {
                            "关闭"
                        };
                        self.show_toast(format!("自动恢复会话已{}", status), false);
                        return;
                    }
                    if field == "compact_enabled" {
                        self.state.agent_config.compact.enabled =
                            !self.state.agent_config.compact.enabled;
                        let status = if self.state.agent_config.compact.enabled {
                            "开启"
                        } else {
                            "关闭"
                        };
                        self.show_toast(format!("上下文压缩已{}", status), false);
                        return;
                    }
                    if field == "theme" {
                        self.switch_theme();
                        return;
                    }
                    if field == "flat_bubble" {
                        self.state.agent_config.flat_bubble = !self.state.agent_config.flat_bubble;
                        let status = if self.state.agent_config.flat_bubble {
                            "开启"
                        } else {
                            "关闭"
                        };
                        self.show_toast(format!("扁平气泡已{}", status), false);
                        self.ui.msg_lines_cache = None;
                        return;
                    }
                    if field == "welcome_quote" {
                        self.state.agent_config.welcome_quote =
                            !self.state.agent_config.welcome_quote;
                        let status = if self.state.agent_config.welcome_quote {
                            "开启"
                        } else {
                            "关闭"
                        };
                        self.show_toast(format!("欢迎诗句已{}", status), false);
                        return;
                    }
                    if field == "thinking_style" {
                        let next = self.state.agent_config.thinking_style.next();
                        self.state.agent_config.thinking_style = next;
                        self.show_toast(format!("思考动画: {}", next.display_name()), false);
                        return;
                    }
                    if field == "system_prompt" {
                        self.ui.pending_system_prompt_edit = true;
                        return;
                    }
                    if field == "agent_md" {
                        self.ui.pending_agent_md_edit = true;
                        return;
                    }
                    if field == "compact_exempt_tools" {
                        self.ui.compact_exempt_sublist = true;
                        self.ui.compact_exempt_idx = 0;
                        self.ui.config_scroll_offset = 0;
                        return;
                    }
                    if field == "style" {
                        self.ui.pending_style_edit = true;
                        return;
                    }
                    self.ui.config_edit_buf = config_field_raw_value_global(self, idx);
                    self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
                    self.ui.config_editing = true;
                }
            }
            // Toggle 开关类 Tab
            ConfigTab::Tools | ConfigTab::Skills | ConfigTab::Commands => {
                self.update_toggle_menu_toggle();
            }
            // 这些 Tab 的 Enter 键无特殊操作
            ConfigTab::Session | ConfigTab::Hooks | ConfigTab::Teammates | ConfigTab::Archive => {}
        }
    }

    pub(super) fn update_config_edit_char(&mut self, c: char) {
        let byte_idx = self
            .ui
            .config_edit_buf
            .char_indices()
            .nth(self.ui.config_edit_cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.ui.config_edit_buf.len());
        self.ui.config_edit_buf.insert(byte_idx, c);
        self.ui.config_edit_cursor += 1;
    }

    pub(super) fn update_config_edit_delete(&mut self) {
        if self.ui.config_edit_cursor > 0 {
            let idx = self
                .ui
                .config_edit_buf
                .char_indices()
                .nth(self.ui.config_edit_cursor - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end_idx = self
                .ui
                .config_edit_buf
                .char_indices()
                .nth(self.ui.config_edit_cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.ui.config_edit_buf.len());
            self.ui.config_edit_buf = format!(
                "{}{}",
                &self.ui.config_edit_buf[..idx],
                &self.ui.config_edit_buf[end_idx..]
            );
            self.ui.config_edit_cursor -= 1;
        }
    }

    pub(super) fn update_config_edit_delete_forward(&mut self) {
        let char_count = self.ui.config_edit_buf.chars().count();
        if self.ui.config_edit_cursor < char_count {
            let idx = self
                .ui
                .config_edit_buf
                .char_indices()
                .nth(self.ui.config_edit_cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.ui.config_edit_buf.len());
            let end_idx = self
                .ui
                .config_edit_buf
                .char_indices()
                .nth(self.ui.config_edit_cursor + 1)
                .map(|(i, _)| i)
                .unwrap_or(self.ui.config_edit_buf.len());
            self.ui.config_edit_buf = format!(
                "{}{}",
                &self.ui.config_edit_buf[..idx],
                &self.ui.config_edit_buf[end_idx..]
            );
        }
    }

    pub(super) fn update_config_edit_move_cursor(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => {
                self.ui.config_edit_cursor = self.ui.config_edit_cursor.saturating_sub(1);
            }
            CursorDirection::Down => {
                let char_count = self.ui.config_edit_buf.chars().count();
                if self.ui.config_edit_cursor < char_count {
                    self.ui.config_edit_cursor += 1;
                }
            }
        }
    }

    pub(super) fn update_config_edit_clear_line(&mut self) {
        self.ui.config_edit_buf.clear();
        self.ui.config_edit_cursor = 0;
    }

    pub(super) fn update_config_edit_submit(&mut self) {
        use crate::command::chat::render::helpers::{
            config_field_set_global, config_field_set_model,
        };
        let val = self.ui.config_edit_buf.clone();
        match self.ui.config_tab {
            ConfigTab::Model => {
                config_field_set_model(self, self.ui.model_field_idx, &val);
            }
            ConfigTab::Global => {
                config_field_set_global(self, self.ui.config_field_idx, &val);
            }
            // 这些 Tab 不支持字段编辑提交
            ConfigTab::Session
            | ConfigTab::Tools
            | ConfigTab::Skills
            | ConfigTab::Hooks
            | ConfigTab::Commands
            | ConfigTab::Teammates
            | ConfigTab::Archive => {}
        }
        self.ui.config_editing = false;
    }

    pub(super) fn update_config_add_provider(&mut self) {
        let new_provider = ModelProvider {
            name: format!("Provider-{}", self.state.agent_config.providers.len() + 1),
            api_base: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: String::new(),
            supports_vision: false,
        };
        self.state.agent_config.providers.push(new_provider);
        self.ui.config_provider_idx = self.state.agent_config.providers.len() - 1;
        self.ui.config_field_idx = 0;
        self.show_toast("已新增 Provider，请填写配置", false);
    }

    pub(super) fn update_config_delete_provider(&mut self) {
        let count = self.state.agent_config.providers.len();
        if count == 0 {
            self.show_toast("没有可删除的 Provider", true);
        } else {
            let removed_name = self.state.agent_config.providers[self.ui.config_provider_idx]
                .name
                .clone();
            self.state
                .agent_config
                .providers
                .remove(self.ui.config_provider_idx);
            if self.ui.config_provider_idx >= self.state.agent_config.providers.len()
                && self.ui.config_provider_idx > 0
            {
                self.ui.config_provider_idx -= 1;
            }
            if self.state.agent_config.active_index >= self.state.agent_config.providers.len()
                && self.state.agent_config.active_index > 0
            {
                self.state.agent_config.active_index -= 1;
            }
            self.show_toast(format!("已删除 Provider: {}", removed_name), false);
        }
    }

    pub(super) fn update_config_set_active_provider(&mut self) {
        if !self.state.agent_config.providers.is_empty() {
            self.state.agent_config.active_index = self.ui.config_provider_idx;
            let name = self.state.agent_config.providers[self.ui.config_provider_idx]
                .name
                .clone();
            self.show_toast(format!("已设为活跃模型: {}", name), false);
        }
    }

    pub(super) fn update_config_switch_tab(&mut self, dir: CursorDirection) {
        use crate::command::chat::infra::archive;
        self.ui.config_tab = match dir {
            CursorDirection::Down => self.ui.config_tab.next(),
            CursorDirection::Up => self.ui.config_tab.prev(),
        };
        self.ui.config_field_idx = 0;
        self.ui.config_scroll_offset = 0;
        self.ui.config_editing = false;
        // 切换到 Session tab 时自动加载列表
        if self.ui.config_tab == ConfigTab::Session {
            self.update_load_session_list();
        }
        // 切换到 Archive tab 时自动加载归档列表
        if self.ui.config_tab == ConfigTab::Archive {
            self.ui.archives = archive::list_archives();
            self.ui.archive_list_index = 0;
            self.ui.restore_confirm_needed = false;
        }
    }

    /// 配置界面：鼠标直接点击跳转到指定 Tab
    pub(super) fn update_config_switch_tab_to(&mut self, tab: ConfigTab) {
        use crate::command::chat::infra::archive;
        self.ui.config_tab = tab;
        self.ui.config_field_idx = 0;
        self.ui.config_scroll_offset = 0;
        self.ui.config_editing = false;
        if self.ui.config_tab == ConfigTab::Session {
            self.update_load_session_list();
        }
        if self.ui.config_tab == ConfigTab::Archive {
            self.ui.archives = archive::list_archives();
            self.ui.archive_list_index = 0;
            self.ui.restore_confirm_needed = false;
        }
    }

    /// 配置界面：鼠标点击选中指定字段索引
    pub(super) fn update_config_field_select(&mut self, idx: usize) {
        let total = super::config_tab_field_count(self);
        if total > 0 && idx < total {
            self.ui.config_field_idx = idx;
        }
    }

    pub(super) fn update_toggle_menu_navigate(&mut self, dir: CursorDirection) {
        // Tools tab 使用层级导航
        if self.ui.config_tab == ConfigTab::Tools {
            if self.ui.tools_in_options {
                // 选项层级：上下切换启用/defer
                match dir {
                    CursorDirection::Up => {
                        if self.ui.tools_option_idx > 0 {
                            self.ui.tools_option_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        if self.ui.tools_option_idx < 1 {
                            self.ui.tools_option_idx += 1;
                        }
                    }
                }
            } else {
                // 工具列表层级：上下切换工具
                let total = self.tool_registry.tool_names().len();
                if total == 0 {
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.config_field_idx == 0 {
                            self.ui.config_field_idx = total - 1;
                        } else {
                            self.ui.config_field_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        self.ui.config_field_idx = (self.ui.config_field_idx + 1) % total;
                    }
                }
            }
            return;
        }

        // 其他 Tab 保持原有逻辑
        let total = super::config_tab_field_count(self);
        if total == 0 {
            return;
        }
        match dir {
            CursorDirection::Up => {
                if self.ui.config_field_idx == 0 {
                    self.ui.config_field_idx = total - 1;
                } else {
                    self.ui.config_field_idx -= 1;
                }
            }
            CursorDirection::Down => {
                self.ui.config_field_idx = (self.ui.config_field_idx + 1) % total;
            }
        }
    }

    /// Tools tab：Tab 键切换层级（工具列表 ↔ 选项区）。
    /// 选项区包含 2 个选项：索引 0 = 启用/禁用，索引 1 = defer 开关。
    /// 详见 `config/tools.rs` 中的渲染逻辑。
    pub(super) fn update_tools_toggle_level(&mut self) {
        if self.ui.config_tab != ConfigTab::Tools {
            return;
        }
        if self.ui.tools_in_options {
            // 从选项区返回工具列表
            self.ui.tools_in_options = false;
        } else {
            // 进入选中工具的选项区
            self.ui.tools_in_options = true;
            self.ui.tools_option_idx = 0; // 默认焦点在"启用"
        }
    }

    /// Model tab：Tab 键切换层级（Provider 列表 ↔ 配置字段）。
    /// 在 Provider 列表层按 Tab 进入右侧配置字段区，
    /// 在字段区按 Tab 返回 Provider 列表。
    pub(super) fn update_model_toggle_level(&mut self) {
        if self.ui.config_tab != ConfigTab::Model {
            return;
        }
        if self.ui.config_editing {
            // 编辑模式下 Tab 不切换层级
            return;
        }
        if self.ui.model_in_fields {
            // 从字段区返回 Provider 列表
            self.ui.model_in_fields = false;
        } else if !self.state.agent_config.providers.is_empty() {
            // 进入选中 Provider 的配置字段区
            self.ui.model_in_fields = true;
            self.ui.model_field_idx = 0;
        }
    }

    pub(super) fn update_toggle_menu_toggle(&mut self) {
        if self.ui.config_tab == ConfigTab::Tools {
            // 根据当前层级和焦点决定 toggle 哪个选项
            if self.ui.tools_in_options {
                let tool_names = self.tool_registry.tool_names();
                if let Some(name) = tool_names.get(self.ui.config_field_idx) {
                    let name = name.to_string();
                    if self.ui.tools_option_idx == 0 {
                        // toggle 启用状态
                        if let Some(pos) = self
                            .state
                            .agent_config
                            .disabled_tools
                            .iter()
                            .position(|d| d == &name)
                        {
                            self.state.agent_config.disabled_tools.remove(pos);
                        } else {
                            self.state.agent_config.disabled_tools.push(name);
                        }
                    } else {
                        // toggle defer 状态（仅对启用的工具有效）
                        let is_enabled = !self
                            .state
                            .agent_config
                            .disabled_tools
                            .iter()
                            .any(|d| d == &name);
                        if is_enabled {
                            // 双写：同时修改 agent_config（持久化）和 deferred_tools（运行时）
                            let mut deferred = match self.deferred_tools.lock() {
                                Ok(guard) => guard,
                                Err(e) => e.into_inner(),
                            };
                            if let Some(pos) = deferred.iter().position(|d| d == &name) {
                                deferred.remove(pos);
                                // 同步移除 agent_config 中的对应项
                                if let Some(pos2) = self
                                    .state
                                    .agent_config
                                    .deferred_tools
                                    .iter()
                                    .position(|d| d == &name)
                                {
                                    self.state.agent_config.deferred_tools.remove(pos2);
                                }
                                // 同步移除 session_loaded_deferred 中的记录
                                if let Ok(mut loaded) = self.session_loaded_deferred.lock() {
                                    loaded.retain(|n| n != &name);
                                }
                            } else {
                                deferred.push(name.clone());
                                self.state.agent_config.deferred_tools.push(name);
                            }
                        }
                    }
                }
            } else {
                // 工具列表层级，Enter 也 toggle 启用状态
                let tool_names = self.tool_registry.tool_names();
                if let Some(name) = tool_names.get(self.ui.config_field_idx) {
                    let name = name.to_string();
                    if let Some(pos) = self
                        .state
                        .agent_config
                        .disabled_tools
                        .iter()
                        .position(|d| d == &name)
                    {
                        self.state.agent_config.disabled_tools.remove(pos);
                    } else {
                        self.state.agent_config.disabled_tools.push(name);
                    }
                }
            }
            return;
        }

        // 其他 Tab 保持原有逻辑
        if self.ui.config_tab == ConfigTab::Skills
            && let Some(skill) = self.state.loaded_skills.get(self.ui.config_field_idx)
        {
            let name = skill.frontmatter.name.clone();
            if let Some(pos) = self
                .state
                .agent_config
                .disabled_skills
                .iter()
                .position(|d| d == &name)
            {
                self.state.agent_config.disabled_skills.remove(pos);
            } else {
                self.state.agent_config.disabled_skills.push(name);
            }
        } else if self.ui.config_tab == ConfigTab::Commands
            && let Some(cmd) = self.state.loaded_commands.get(self.ui.config_field_idx)
        {
            let name = cmd.frontmatter.name.clone();
            if let Some(pos) = self
                .state
                .agent_config
                .disabled_commands
                .iter()
                .position(|d| d == &name)
            {
                self.state.agent_config.disabled_commands.remove(pos);
            } else {
                self.state.agent_config.disabled_commands.push(name);
            }
        } else if self.ui.config_tab == ConfigTab::Hooks
            && let Ok(manager) = self.hook_manager.lock()
        {
            let hooks = manager.list_hooks();
            if let Some(entry) = hooks.get(self.ui.config_field_idx) {
                let uid = entry.unique_id.clone();
                if let Some(pos) = self
                    .state
                    .agent_config
                    .disabled_hooks
                    .iter()
                    .position(|d| d == &uid)
                {
                    self.state.agent_config.disabled_hooks.remove(pos);
                } else {
                    self.state.agent_config.disabled_hooks.push(uid);
                }
            }
        }
    }

    pub(super) fn update_toggle_menu_enable_all(&mut self) {
        if self.ui.config_tab == ConfigTab::Tools {
            self.state.agent_config.disabled_tools.clear();
            // 启用全部时同时清除 deferred 状态（双写）
            self.state.agent_config.deferred_tools.clear();
            let mut deferred = match self.deferred_tools.lock() {
                Ok(guard) => guard,
                Err(e) => e.into_inner(),
            };
            deferred.clear();
            drop(deferred);
            if let Ok(mut loaded) = self.session_loaded_deferred.lock() {
                loaded.clear();
            }
            self.show_toast("已启用全部工具", false);
        } else if self.ui.config_tab == ConfigTab::Skills {
            self.state.agent_config.disabled_skills.clear();
            self.show_toast("已启用全部 Skills", false);
        } else if self.ui.config_tab == ConfigTab::Commands {
            self.state.agent_config.disabled_commands.clear();
            self.show_toast("已启用全部命令", false);
        } else if self.ui.config_tab == ConfigTab::Hooks {
            self.state.agent_config.disabled_hooks.clear();
            self.show_toast("已启用全部 Hooks", false);
        }
    }

    pub(super) fn update_toggle_menu_disable_all(&mut self) {
        if self.ui.config_tab == ConfigTab::Tools {
            self.state.agent_config.disabled_tools = self
                .tool_registry
                .tool_names()
                .iter()
                .map(|n| n.to_string())
                .collect();
            // 禁用全部时清除 deferred 状态（双写）
            self.state.agent_config.deferred_tools.clear();
            let mut deferred = match self.deferred_tools.lock() {
                Ok(guard) => guard,
                Err(e) => e.into_inner(),
            };
            deferred.clear();
            drop(deferred);
            if let Ok(mut loaded) = self.session_loaded_deferred.lock() {
                loaded.clear();
            }
            self.show_toast("已禁用全部工具", false);
        } else if self.ui.config_tab == ConfigTab::Skills {
            self.state.agent_config.disabled_skills = self
                .state
                .loaded_skills
                .iter()
                .map(|s| s.frontmatter.name.clone())
                .collect();
            self.show_toast("已禁用全部 Skills", false);
        } else if self.ui.config_tab == ConfigTab::Commands {
            self.state.agent_config.disabled_commands = self
                .state
                .loaded_commands
                .iter()
                .map(|c| c.frontmatter.name.clone())
                .collect();
            self.show_toast("已禁用全部命令", false);
        } else if self.ui.config_tab == ConfigTab::Hooks {
            if let Ok(manager) = self.hook_manager.lock() {
                self.state.agent_config.disabled_hooks = manager
                    .list_hooks()
                    .iter()
                    .map(|h| h.unique_id.clone())
                    .collect();
            }
            self.show_toast("已禁用全部 Hooks", false);
        }
    }

    pub(super) fn update_compact_exempt_toggle(&mut self) {
        let tool_names = self.tool_registry.tool_names();
        if let Some(name) = tool_names.get(self.ui.compact_exempt_idx) {
            let name_str = name.to_string();
            let exempt = &mut self.state.agent_config.compact.micro_compact_exempt_tools;
            if let Some(pos) = exempt.iter().position(|t| t == &name_str) {
                exempt.remove(pos);
            } else {
                exempt.push(name_str);
            }
        }
    }

    /// 豁免压缩工具子列表：鼠标点击选中指定索引
    pub(super) fn update_compact_exempt_select(&mut self, idx: usize) {
        let total = self.tool_registry.tool_names().len();
        if total > 0 && idx < total {
            self.ui.compact_exempt_idx = idx;
        }
    }

    // ========== 模型选择 ==========

    pub(super) fn update_model_select_navigate(&mut self, dir: CursorDirection) {
        let count = self.state.agent_config.providers.len();
        if count > 0 {
            match dir {
                CursorDirection::Up => {
                    let i = self
                        .ui
                        .model_list_state
                        .selected()
                        .map(|i| if i == 0 { count - 1 } else { i - 1 })
                        .unwrap_or(0);
                    self.ui.model_list_state.select(Some(i));
                }
                CursorDirection::Down => {
                    let i = self
                        .ui
                        .model_list_state
                        .selected()
                        .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                        .unwrap_or(0);
                    self.ui.model_list_state.select(Some(i));
                }
            }
        }
    }

    // ========== 主题选择 ==========

    pub(super) fn update_theme_select_navigate(&mut self, dir: CursorDirection) {
        let count = crate::theme::ThemeName::all().len();
        if count > 0 {
            match dir {
                CursorDirection::Up => {
                    let i = self
                        .ui
                        .theme_list_state
                        .selected()
                        .map(|i| if i == 0 { count - 1 } else { i - 1 })
                        .unwrap_or(0);
                    self.ui.theme_list_state.select(Some(i));
                }
                CursorDirection::Down => {
                    let i = self
                        .ui
                        .theme_list_state
                        .selected()
                        .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                        .unwrap_or(0);
                    self.ui.theme_list_state.select(Some(i));
                }
            }
        }
    }

    pub(super) fn update_theme_select_confirm(&mut self) {
        use crate::command::chat::storage::save_agent_config;
        use crate::theme::{Theme, ThemeName};
        if let Some(sel) = self.ui.theme_list_state.selected() {
            let all = ThemeName::all();
            if sel < all.len() {
                self.state.agent_config.theme = all[sel].clone();
                self.ui.theme = Theme::from_name(&all[sel]);
                self.ui.msg_lines_cache = None;
                let _ = save_agent_config(&self.state.agent_config);
                let name = all[sel].display_name();
                self.show_toast(format!("已切换主题: {}", name), false);
            }
        }
        self.ui.mode = crate::command::chat::app::ui_state::ChatMode::Chat;
    }

    /// Teammates Tab：导航上下移动选中指针
    pub(super) fn update_teammates_navigate(&mut self, dir: CursorDirection) {
        let count = self
            .teammate_manager
            .lock()
            .map(|m| m.teammates.len())
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        match dir {
            CursorDirection::Up => {
                if self.ui.teammate_list_index > 0 {
                    self.ui.teammate_list_index -= 1;
                }
            }
            CursorDirection::Down => {
                if self.ui.teammate_list_index < count - 1 {
                    self.ui.teammate_list_index += 1;
                }
            }
        }
    }

    /// Teammates Tab：鼠标点击选中指定索引
    pub(super) fn update_teammates_select(&mut self, idx: usize) {
        let count = self
            .teammate_manager
            .lock()
            .map(|m| m.teammates.len())
            .unwrap_or(0);
        if count > 0 && idx < count {
            self.ui.teammate_list_index = idx;
        }
    }

    // ========== Commands 创建 ==========

    /// 进入选择命令保存级别模式
    pub(super) fn update_config_command_select_source(&mut self) {
        use crate::command::chat::app::ui_state::CommandsMode;
        use crate::command::chat::infra::command::CommandSource;

        if command::project_commands_dir().is_some() {
            self.ui.commands_mode = CommandsMode::SelectSource;
            self.ui.commands_source_idx = 0;
        } else {
            // 没有项目级目录，直接使用用户级
            self.ui.command_create_source = CommandSource::User;
            self.ui.pending_command_create = true;
        }
    }

    /// 选择命令保存级别导航
    pub(super) fn update_config_command_navigate_source(&mut self, dir: CursorDirection) {
        match dir {
            CursorDirection::Up => {
                if self.ui.commands_source_idx > 0 {
                    self.ui.commands_source_idx -= 1;
                }
            }
            CursorDirection::Down => {
                if self.ui.commands_source_idx < 1 {
                    self.ui.commands_source_idx += 1;
                }
            }
        }
    }

    /// 确认命令保存级别选择
    pub(super) fn update_config_command_confirm_source(&mut self) {
        use crate::command::chat::app::ui_state::CommandsMode;
        use crate::command::chat::infra::command::CommandSource;

        self.ui.command_create_source = if self.ui.commands_source_idx == 0 {
            CommandSource::User
        } else {
            CommandSource::Project
        };
        self.ui.commands_mode = CommandsMode::Normal;
        self.ui.pending_command_create = true;
    }

    /// 取消命令创建
    pub(super) fn update_config_command_cancel(&mut self) {
        use crate::command::chat::app::ui_state::CommandsMode;
        self.ui.commands_mode = CommandsMode::Normal;
    }
}
