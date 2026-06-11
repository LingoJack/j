use crate::infra::hook::types::*;
use crate::permission::JcliConfig;
use crate::util::log::{write_error_log, write_info_log};
use crate::util::shell_runtime::ShellRuntime;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ========== HookKind 枚举 ==========

/// Hook 种类：Shell 命令（子进程）、LLM（prompt 模板调 LLM）、内置 Rust 闭包（进程内）
#[derive(Clone)]
pub enum HookKind {
    /// Shell 命令，通过 `sh -c` 子进程执行（现有行为）
    Shell(ShellHook),
    /// LLM hook，通过 prompt 模板调用 LLM API，返回 HookResult JSON
    Llm(LlmHook),
    /// 内置 Rust 闭包，进程内零开销执行
    Builtin(BuiltinHook),
}

/// Shell hook：一条命令 + 超时 + 失败策略 + 条件过滤
#[derive(Debug, Clone)]
pub struct ShellHook {
    /// Hook 目录名（目录布局下有值，session hook 为 None）
    pub name: Option<String>,
    pub command: String,
    pub timeout: u64,
    pub retry: u32,
    pub on_error: OnError,
    pub filter: HookFilter,
    pub runtime: Option<ShellRuntime>,
    /// Hook 目录路径（目录布局下有值，session hook 为 None）
    pub dir_path: Option<PathBuf>,
}

/// LLM hook：prompt 模板 + 模型覆盖 + 超时 + 重试 + 失败策略 + 条件过滤
#[derive(Debug, Clone)]
pub struct LlmHook {
    /// Hook 目录名（目录布局下有值，session hook 为 None）
    pub name: Option<String>,
    /// Prompt 模板，支持 {{variable}} 模板变量
    pub prompt: String,
    /// 模型名覆盖（空则使用当前活跃 provider 的模型）
    pub model: Option<String>,
    /// 超时秒数
    pub timeout: u64,
    /// 重试次数（仅 Err 路径生效）
    pub retry: u32,
    /// 失败策略
    pub on_error: OnError,
    /// 条件过滤
    pub filter: HookFilter,
    /// Hook 目录路径（目录布局下有值，session hook 为 None）
    #[allow(dead_code)]
    pub dir_path: Option<PathBuf>,
}

/// 内置 hook 的处理函数类型
pub type BuiltinHookFn = Arc<dyn Fn(&HookContext) -> Option<HookResult> + Send + Sync>;

/// 内置 hook：一个命名的 Rust 闭包
pub struct BuiltinHook {
    /// 唯一名称，用于列出/调试（如 "tasks_status"、"todo_nag"）
    pub name: String,
    /// 实际执行的 Rust 闭包
    pub handler: BuiltinHookFn,
}

impl Clone for BuiltinHook {
    fn clone(&self) -> Self {
        BuiltinHook {
            name: self.name.clone(),
            handler: Arc::clone(&self.handler),
        }
    }
}

impl fmt::Debug for HookKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookKind::Shell(shell) => f
                .debug_struct("HookKind::Shell")
                .field("name", &shell.name)
                .field("command", &shell.command)
                .field("timeout", &shell.timeout)
                .field("on_error", &shell.on_error)
                .finish(),
            HookKind::Llm(llm) => f
                .debug_struct("HookKind::Llm")
                .field("name", &llm.name)
                .field("prompt", &llm.prompt.len())
                .field("model", &llm.model)
                .field("timeout", &llm.timeout)
                .field("retry", &llm.retry)
                .finish(),
            HookKind::Builtin(builtin) => f
                .debug_struct("HookKind::Builtin")
                .field("name", &builtin.name)
                .finish(),
        }
    }
}

// ========== HookDef（YAML 兼容格式）==========

/// Hook 定义（YAML 兼容）：支持 bash 和 llm 两种类型
///
/// YAML 示例（bash）：
/// ```yaml
/// - command: "echo '{\"user_input\": \"hooked\"}'"
///   timeout: 10
///   on_error: skip
/// ```
///
/// YAML 示例（llm）：
/// ```yaml
/// - type: llm
///   prompt: |
///     检查以下用户输入是否包含敏感信息：
///     {{user_input}}
///     如果包含，返回 action=stop + retry_feedback。
///   timeout: 30
///   retry: 1
///   on_error: skip
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    /// Hook 类型：bash（默认）或 llm
    #[serde(default)]
    pub r#type: HookType,
    /// Shell 命令（type=bash 时必填，通过 `sh -c` 执行）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// LLM prompt 模板（type=llm 时必填，支持 {{variable}} 模板变量）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// LLM 模型名覆盖（type=llm 时可选，空则使用当前活跃 provider 的模型）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 超时秒数（bash 默认 10，llm 默认 30）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// 重试次数（仅 Err 路径生效，默认 0 即不重试）
    #[serde(default)]
    pub retry: u32,
    /// 脚本/LLM 失败时的处理策略（默认 skip）
    #[serde(default)]
    pub on_error: OnError,
    /// 条件过滤：仅当条件匹配时执行（默认无过滤）
    #[serde(default, skip_serializing_if = "HookFilter::is_empty")]
    pub filter: HookFilter,
    /// shell runtime 覆盖；为空时使用全局默认值
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<ShellRuntime>,
}

impl HookDef {
    /// 转换为 HookKind（根据 type 字段分派）
    pub fn into_hook_kind(self) -> Result<HookKind, String> {
        match self.r#type {
            HookType::Bash => {
                let command = self.command.unwrap_or_default();
                if command.is_empty() {
                    return Err("bash hook 缺少 command 字段".to_string());
                }
                Ok(HookKind::Shell(ShellHook {
                    name: None,
                    command,
                    timeout: self.timeout,
                    retry: self.retry,
                    on_error: self.on_error,
                    filter: self.filter,
                    runtime: self.runtime,
                    dir_path: None,
                }))
            }
            HookType::Llm => {
                let prompt = self.prompt.unwrap_or_default();
                if prompt.is_empty() {
                    return Err("llm hook 缺少 prompt 字段".to_string());
                }
                Ok(HookKind::Llm(LlmHook {
                    name: None,
                    prompt,
                    model: self.model,
                    timeout: if self.timeout == default_timeout() {
                        default_llm_timeout()
                    } else {
                        self.timeout
                    },
                    retry: if self.retry == 0 { 1 } else { self.retry },
                    on_error: self.on_error,
                    filter: self.filter,
                    dir_path: None,
                }))
            }
        }
    }
}

impl From<HookDef> for HookKind {
    fn from(def: HookDef) -> Self {
        def.into_hook_kind().unwrap_or_else(|e| {
            write_error_log("HookDef::into_hook_kind", &e);
            // 回退到空 Shell hook（不会执行有效操作，但不会 panic）
            HookKind::Shell(ShellHook {
                name: None,
                command: String::new(),
                timeout: 0,
                retry: 0,
                on_error: OnError::Skip,
                filter: HookFilter::default(),
                runtime: None,
                dir_path: None,
            })
        })
    }
}

// ========== HookDirDef（目录布局下的 HOOK.yaml / HOOK.yml 格式）==========

/// HOOK.yaml / HOOK.yml 定义（目录布局下的格式）
///
/// 与 `HookDef` 的区别：`events` 为列表（一个 hook 可绑定多个事件），无 `command`/`prompt` 以外的不必要字段。
/// 目录布局下 `command` 中的相对路径以 hook 目录为 cwd 解析。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDirDef {
    /// 绑定的事件列表
    pub events: Vec<HookEvent>,
    /// Hook 类型
    #[serde(default)]
    pub r#type: HookType,
    /// Shell 命令（type=bash 时必填，通过 `sh -c` 执行，cwd 为 hook 目录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// LLM prompt 模板（type=llm 时必填，支持 {{variable}} 模板变量）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// LLM 模型名覆盖
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 超时秒数（bash 默认 10，llm 默认 30）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    /// 重试次数（仅 Err 路径生效）
    #[serde(default)]
    pub retry: u32,
    /// 失败策略
    #[serde(default)]
    pub on_error: OnError,
    /// 条件过滤
    #[serde(default, skip_serializing_if = "HookFilter::is_empty")]
    pub filter: HookFilter,
    /// shell runtime 覆盖；为空时使用全局默认值
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<ShellRuntime>,
}

impl HookDirDef {
    /// 转换为 `Vec<(HookEvent, HookKind)>`（每个 event 一个条目）
    pub fn into_hook_kinds(
        self,
        name: &str,
        dir_path: &Path,
    ) -> Result<Vec<(HookEvent, HookKind)>, String> {
        if self.events.is_empty() {
            return Err(format!("hook '{}' 的 events 为空", name));
        }
        let kind = match self.r#type {
            HookType::Bash => {
                let command = self.command.unwrap_or_default();
                if command.is_empty() {
                    return Err(format!("bash hook '{}' 缺少 command 字段", name));
                }
                HookKind::Shell(ShellHook {
                    name: Some(name.to_string()),
                    command,
                    timeout: self.timeout,
                    retry: self.retry,
                    on_error: self.on_error,
                    filter: self.filter,
                    runtime: self.runtime,
                    dir_path: Some(dir_path.to_path_buf()),
                })
            }
            HookType::Llm => {
                let prompt = self.prompt.unwrap_or_default();
                if prompt.is_empty() {
                    return Err(format!("llm hook '{}' 缺少 prompt 字段", name));
                }
                HookKind::Llm(LlmHook {
                    name: Some(name.to_string()),
                    prompt,
                    model: self.model,
                    timeout: if self.timeout == default_timeout() {
                        default_llm_timeout()
                    } else {
                        self.timeout
                    },
                    retry: if self.retry == 0 { 1 } else { self.retry },
                    on_error: self.on_error,
                    filter: self.filter,
                    dir_path: Some(dir_path.to_path_buf()),
                })
            }
        };
        Ok(self.events.into_iter().map(|e| (e, kind.clone())).collect())
    }
}

// ========== 目录加载函数 ==========

/// 返回用户级 hooks 目录: ~/.jdata/agent/hooks/
pub fn hooks_dir() -> PathBuf {
    let dir = crate::constants::data_root().join("agent").join("hooks");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 返回项目级 hooks 目录: .jcli/hooks/（如果存在）
pub fn project_hooks_dir() -> Option<PathBuf> {
    let config_dir = JcliConfig::find_config_dir()?;
    let dir = config_dir.join("hooks");
    if dir.is_dir() { Some(dir) } else { None }
}

/// 从指定目录加载 hooks（遍历子目录，解析 HOOK.yaml 或 HOOK.yml）
pub(crate) fn load_hooks_from_dir(
    dir: &Path,
    source_name: &str,
) -> Vec<(String, HookDirDef, PathBuf)> {
    let mut hooks = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return hooks,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let hook_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // 跳过 example 目录（模板示例，不是实际可执行的 hook）
        if hook_name == "example" {
            continue;
        }

        // 优先 HOOK.yaml，其次 HOOK.yml；两者共存时取 HOOK.yaml
        let hook_yaml = if path.join("HOOK.yaml").exists() {
            path.join("HOOK.yaml")
        } else if path.join("HOOK.yml").exists() {
            path.join("HOOK.yml")
        } else {
            continue;
        };
        let yaml_file_name = hook_yaml
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        match fs::read_to_string(&hook_yaml) {
            Ok(content) => match serde_yaml::from_str::<HookDirDef>(&content) {
                Ok(def) => {
                    if def.events.is_empty() {
                        write_error_log(
                            "load_hooks_from_dir",
                            &format!("hook '{}' 的 events 为空，跳过", hook_name),
                        );
                        continue;
                    }
                    hooks.push((hook_name, def, path));
                }
                Err(e) => write_error_log(
                    "load_hooks_from_dir",
                    &format!("解析 {}/{} 失败: {}", hook_name, yaml_file_name, e),
                ),
            },
            Err(e) => write_error_log(
                "load_hooks_from_dir",
                &format!("读取 {}/{} 失败: {}", hook_name, yaml_file_name, e),
            ),
        }
    }
    write_info_log(
        "load_hooks_from_dir",
        &format!("从 {} 加载了 {} 个 hook", source_name, hooks.len()),
    );
    hooks
}
