use crate::cli::SubCmd;
use crate::config::YamlConfig;

/// 统一的命令处理 trait
pub trait CommandHandler {
    fn execute(&self, config: &mut YamlConfig);
}

/// 声明式宏：批量生成 handler struct + CommandHandler impl
///
/// 用法：
/// ```ignore
/// command_handlers! {
///     SetCmd { alias: String, path: Vec<String> } => |self, config| {
///         crate::command::alias::handle_set(&self.alias, &self.path, config);
///     },
/// }
/// ```
macro_rules! command_handlers {
    (
        $(
            $name:ident { $( $field:ident : $ty:ty ),* $(,)? } => |$self_:ident, $cfg:ident| $body:block
        ),* $(,)?
    ) => {
        $(
            pub struct $name {
                $( pub $field: $ty, )*
            }

            impl CommandHandler for $name {
                fn execute(&$self_, $cfg: &mut YamlConfig) $body
            }
        )*
    };
}

// ========== 别名管理 ==========
command_handlers! {
    SetCmd { alias: String, path: Vec<String> } => |self, config| {
        crate::command::alias::handle_set(&self.alias, &self.path, config);
    },
    RemoveCmd { alias: String } => |self, config| {
        crate::command::alias::handle_remove(&self.alias, config);
    },
    RenameCmd { alias: String, new_alias: String } => |self, config| {
        crate::command::alias::handle_rename(&self.alias, &self.new_alias, config);
    },
    ModifyCmd { alias: String, path: Vec<String> } => |self, config| {
        crate::command::alias::handle_modify(&self.alias, &self.path, config);
    },

    // ========== 分类标记 ==========
    NoteCmd { alias: String, category: String } => |self, config| {
        crate::command::category::handle_note(&self.alias, &self.category, config);
    },
    DenoteCmd { alias: String, category: String } => |self, config| {
        crate::command::category::handle_denote(&self.alias, &self.category, config);
    },

    // ========== 列表 & 查找 ==========
    ListCmd { part: Option<String> } => |self, config| {
        crate::command::list::handle_list(self.part.as_deref(), config);
    },
    ContainCmd { alias: String, containers: Option<String> } => |self, config| {
        crate::command::system::handle_contain(&self.alias, self.containers.as_deref(), config);
    },

    // ========== 日报系统 ==========
    ReportCmd { content: Vec<String> } => |self, config| {
        crate::command::report::handle_report("report", &self.content, config);
    },
    ReportCtlCmd { action: String, arg: Option<String> } => |self, config| {
        let mut args = vec![self.action.clone()];
        if let Some(ref a) = self.arg {
            args.push(a.clone());
        }
        crate::command::report::handle_report("reportctl", &args, config);
    },
    CheckCmd { line_count: Option<String> } => |self, config| {
        crate::command::report::handle_check(self.line_count.as_deref(), config);
    },
    SearchCmd { line_count: String, target: String, fuzzy: Option<String> } => |self, config| {
        crate::command::report::handle_search(&self.line_count, &self.target, self.fuzzy.as_deref(), config);
    },

    // ========== 待办备忘录 ==========
    TodoCmd { content: Vec<String> } => |self, config| {
        crate::command::todo::handle_todo(&self.content, config);
    },

    // ========== AI 对话 ==========
    ChatCmd { content: Vec<String> } => |self, config| {
        crate::command::chat::handle_chat(&self.content, config);
    },

    // ========== 脚本 ==========
    ConcatCmd { name: String, content: Vec<String> } => |self, config| {
        crate::command::script::handle_concat(&self.name, &self.content, config);
    },

    // ========== 计时器 ==========
    TimeCmd { function: String, arg: String } => |self, _config| {
        crate::command::time::handle_time(&self.function, &self.arg);
    },

    // ========== 系统设置 ==========
    LogCmd { key: String, value: String } => |self, config| {
        crate::command::system::handle_log(&self.key, &self.value, config);
    },
    ChangeCmd { part: String, field: String, value: String } => |self, config| {
        crate::command::system::handle_change(&self.part, &self.field, &self.value, config);
    },
    ClearCmd {} => |self, _config| {
        crate::command::system::handle_clear();
    },

    // ========== 系统信息 ==========
    VersionCmd {} => |self, _config| {
        crate::command::system::handle_version();
    },
    HelpCmd {} => |self, _config| {
        crate::command::help::handle_help();
    },
    ExitCmd {} => |self, _config| {
        crate::command::system::handle_exit();
    },
    CompletionCmd { shell: Option<String> } => |self, config| {
        crate::command::system::handle_completion(self.shell.as_deref(), config);
    },

    // ========== 语音转文字 ==========
    VoiceCmd { action: String, copy: bool, model: Option<String> } => |self, config| {
        crate::command::voice::handle_voice(&self.action, self.copy, self.model.as_deref(), config);
    },
}

/// 将 SubCmd 枚举变体转换为 Box<dyn CommandHandler>
impl SubCmd {
    pub fn into_handler(self) -> Box<dyn CommandHandler> {
        match self {
            // 别名管理
            SubCmd::Set { alias, path } => Box::new(SetCmd { alias, path }),
            SubCmd::Remove { alias } => Box::new(RemoveCmd { alias }),
            SubCmd::Rename { alias, new_alias } => Box::new(RenameCmd { alias, new_alias }),
            SubCmd::Modify { alias, path } => Box::new(ModifyCmd { alias, path }),

            // 分类标记
            SubCmd::Note { alias, category } => Box::new(NoteCmd { alias, category }),
            SubCmd::Denote { alias, category } => Box::new(DenoteCmd { alias, category }),

            // 列表 & 查找
            SubCmd::List { part } => Box::new(ListCmd { part }),
            SubCmd::Contain { alias, containers } => Box::new(ContainCmd { alias, containers }),

            // 日报系统
            SubCmd::Report { content } => Box::new(ReportCmd { content }),
            SubCmd::Reportctl { action, arg } => Box::new(ReportCtlCmd { action, arg }),
            SubCmd::Check { line_count } => Box::new(CheckCmd { line_count }),
            SubCmd::Search {
                line_count,
                target,
                fuzzy,
            } => Box::new(SearchCmd {
                line_count,
                target,
                fuzzy,
            }),

            // 待办 & AI
            SubCmd::Todo { content } => Box::new(TodoCmd { content }),
            SubCmd::Chat { content } => Box::new(ChatCmd { content }),

            // 脚本 & 计时
            SubCmd::Concat { name, content } => Box::new(ConcatCmd { name, content }),
            SubCmd::Time { function, arg } => Box::new(TimeCmd { function, arg }),

            // 系统设置
            SubCmd::Log { key, value } => Box::new(LogCmd { key, value }),
            SubCmd::Change { part, field, value } => Box::new(ChangeCmd { part, field, value }),
            SubCmd::Clear => Box::new(ClearCmd {}),

            // 系统信息
            SubCmd::Version => Box::new(VersionCmd {}),
            SubCmd::Help => Box::new(HelpCmd {}),
            SubCmd::Exit => Box::new(ExitCmd {}),
            SubCmd::Completion { shell } => Box::new(CompletionCmd { shell }),

            // 语音转文字
            SubCmd::Voice {
                action,
                copy,
                model,
            } => Box::new(VoiceCmd {
                action,
                copy,
                model,
            }),
        }
    }
}
