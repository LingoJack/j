use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShellRuntime {
    #[default]
    Auto,
    Pwsh,
    PowerShell,
    GitBash,
    Cmd,
    Sh,
    Bash,
}

impl ShellRuntime {
    pub const ALL: &[ShellRuntime] = &[
        ShellRuntime::Auto,
        ShellRuntime::Pwsh,
        ShellRuntime::PowerShell,
        ShellRuntime::GitBash,
        ShellRuntime::Cmd,
        ShellRuntime::Sh,
        ShellRuntime::Bash,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Pwsh => "pwsh",
            Self::PowerShell => "powershell",
            Self::GitBash => "git_bash",
            Self::Cmd => "cmd",
            Self::Sh => "sh",
            Self::Bash => "bash",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Auto => "自动",
            Self::Pwsh => "PowerShell 7",
            Self::PowerShell => "Windows PowerShell",
            Self::GitBash => "Git Bash",
            Self::Cmd => "CMD",
            Self::Sh => "sh",
            Self::Bash => "bash",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "pwsh" => Some(Self::Pwsh),
            "powershell" | "powershell.exe" => Some(Self::PowerShell),
            "git_bash" | "git-bash" | "gitbash" => Some(Self::GitBash),
            "cmd" | "cmd.exe" => Some(Self::Cmd),
            "sh" => Some(Self::Sh),
            "bash" => Some(Self::Bash),
            _ => None,
        }
    }

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|r| r == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedShellRuntime {
    pub runtime: ShellRuntime,
    pub program: String,
    pub args: Vec<String>,
}

impl ResolvedShellRuntime {
    pub fn build_command(&self, shell_command: &str) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args).arg(shell_command);
        cmd
    }
}

pub fn parse_runtime_override(runtime: Option<&str>) -> Result<Option<ShellRuntime>, String> {
    match runtime {
        None => Ok(None),
        Some(value) => ShellRuntime::parse(value)
            .map(Some)
            .ok_or_else(|| invalid_runtime_message(value)),
    }
}

fn default_runtime_lock() -> &'static RwLock<ShellRuntime> {
    static DEFAULT_RUNTIME: OnceLock<RwLock<ShellRuntime>> = OnceLock::new();
    DEFAULT_RUNTIME.get_or_init(|| RwLock::new(ShellRuntime::Auto))
}

pub fn set_default_shell_runtime(runtime: ShellRuntime) {
    let mut guard = default_runtime_lock()
        .write()
        .unwrap_or_else(|e| e.into_inner());
    *guard = runtime;
}

pub fn get_default_shell_runtime() -> ShellRuntime {
    *default_runtime_lock()
        .read()
        .unwrap_or_else(|e| e.into_inner())
}

fn find_program_in_path(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.is_absolute() || program.contains(std::path::MAIN_SEPARATOR) {
        return path.exists().then(|| path.to_path_buf());
    }

    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var).find_map(|dir| {
        candidate_paths(&dir, program)
            .into_iter()
            .find(|candidate| candidate.exists())
    })
}

fn candidate_paths(dir: &Path, program: &str) -> Vec<PathBuf> {
    let mut candidates = vec![dir.join(program)];
    #[cfg(windows)]
    {
        let has_ext = Path::new(program).extension().is_some();
        if !has_ext {
            candidates.push(dir.join(format!("{program}.exe")));
            candidates.push(dir.join(format!("{program}.cmd")));
            candidates.push(dir.join(format!("{program}.bat")));
        }
    }
    candidates
}

fn invalid_runtime_message(value: &str) -> String {
    let allowed = ShellRuntime::ALL
        .iter()
        .map(ShellRuntime::as_str)
        .collect::<Vec<_>>()
        .join(" / ");
    format!("未知 shell runtime: {value}；允许值: {allowed}")
}

fn is_git_bash_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let file_name = file_name.to_ascii_lowercase();
    if file_name != "bash.exe" && file_name != "bash" {
        return false;
    }

    let normalized = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    if normalized.contains("\\windows\\system32\\bash.exe") {
        return false;
    }

    normalized.contains("\\git\\")
}

#[cfg(windows)]
fn candidate_git_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(output) = Command::new("git").arg("--exec-path").output()
        && output.status.success()
    {
        let exec_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !exec_path.is_empty() {
            let exec_path = PathBuf::from(exec_path);
            for depth in 0..=4 {
                if let Some(root) = exec_path.ancestors().nth(depth) {
                    roots.push(root.to_path_buf());
                }
            }
        }
    }

    for program in ["git.exe", "git", "git.cmd", "git.bat"] {
        if let Some(path) = find_program_in_path(program)
            && let Some(parent) = path.parent()
        {
            let root = if parent
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("cmd"))
            {
                parent.parent().unwrap_or(parent).to_path_buf()
            } else {
                parent.to_path_buf()
            };
            roots.push(root);
        }
    }

    for key in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
        if let Some(value) = std::env::var_os(key) {
            roots.push(PathBuf::from(value).join("Git"));
        }
    }
    if let Some(value) = std::env::var_os("LocalAppData") {
        roots.push(PathBuf::from(value).join("Programs").join("Git"));
    }

    roots
}

#[cfg(windows)]
fn find_git_bash_path() -> Option<PathBuf> {
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path_var| std::env::split_paths(&path_var).collect())
        .unwrap_or_default();

    for dir in &path_dirs {
        if let Some(candidate) = candidate_paths(dir, "bash.exe")
            .into_iter()
            .find(|candidate| candidate.exists() && is_git_bash_path(candidate))
        {
            return Some(candidate);
        }
    }

    for root in candidate_git_roots() {
        for rel in ["bin\\bash.exe", "usr\\bin\\bash.exe"] {
            let candidate = root.join(rel);
            if candidate.exists() && is_git_bash_path(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn find_git_bash_path() -> Option<PathBuf> {
    None
}

fn resolve_with_checker_impl<F>(
    preferred: ShellRuntime,
    mut exists: F,
) -> Result<ResolvedShellRuntime, String>
where
    F: FnMut(&str) -> bool,
{
    let is_windows = cfg!(windows);

    let mut try_candidates = |runtime: ShellRuntime, names: &[&str], args: &[&str]| {
        names.iter().find_map(|name| {
            exists(name).then(|| ResolvedShellRuntime {
                runtime,
                program: (*name).to_string(),
                args: args.iter().map(|s| (*s).to_string()).collect(),
            })
        })
    };

    let windows_auto = [
        (
            ShellRuntime::Pwsh,
            vec!["pwsh"],
            vec!["-NoProfile", "-NonInteractive", "-Command"],
        ),
        (
            ShellRuntime::PowerShell,
            vec!["powershell.exe", "powershell"],
            vec!["-NoProfile", "-NonInteractive", "-Command"],
        ),
        (ShellRuntime::GitBash, vec!["bash.exe", "bash"], vec!["-lc"]),
        (
            ShellRuntime::Cmd,
            vec!["cmd.exe", "cmd"],
            vec!["/d", "/s", "/c"],
        ),
    ];
    let unix_auto = [
        (ShellRuntime::Bash, vec!["bash"], vec!["-lc"]),
        (ShellRuntime::Sh, vec!["sh"], vec!["-lc"]),
    ];

    if preferred == ShellRuntime::Auto {
        let resolved = if is_windows {
            windows_auto
                .iter()
                .find_map(|(rt, names, args)| try_candidates(*rt, names, args))
        } else {
            unix_auto
                .iter()
                .find_map(|(rt, names, args)| try_candidates(*rt, names, args))
        };
        return resolved.ok_or_else(|| {
            if is_windows {
                "未找到可用 shell：按顺序尝试了 pwsh / powershell / git bash / cmd".to_string()
            } else {
                "未找到可用 shell：按顺序尝试了 bash / sh".to_string()
            }
        });
    }

    let resolved = match preferred {
        ShellRuntime::Pwsh => {
            if !is_windows {
                return Err("pwsh 仅支持在 Windows 运行时策略中使用".to_string());
            }
            try_candidates(
                preferred,
                &["pwsh"],
                &["-NoProfile", "-NonInteractive", "-Command"],
            )
        }
        ShellRuntime::PowerShell => {
            if !is_windows {
                return Err("powershell 仅支持在 Windows 运行时策略中使用".to_string());
            }
            try_candidates(
                preferred,
                &["powershell.exe", "powershell"],
                &["-NoProfile", "-NonInteractive", "-Command"],
            )
        }
        ShellRuntime::GitBash => {
            if !is_windows {
                return Err("git_bash 仅支持在 Windows 运行时策略中使用".to_string());
            }
            try_candidates(preferred, &["bash.exe", "bash"], &["-lc"])
        }
        ShellRuntime::Cmd => {
            if !is_windows {
                return Err("cmd 仅支持在 Windows 运行时策略中使用".to_string());
            }
            try_candidates(preferred, &["cmd.exe", "cmd"], &["/d", "/s", "/c"])
        }
        ShellRuntime::Sh => try_candidates(preferred, &["sh"], &["-lc"]),
        ShellRuntime::Bash => try_candidates(preferred, &["bash"], &["-lc"]),
        ShellRuntime::Auto => None,
    };

    resolved.ok_or_else(|| format!("未找到可用 shell: {}", preferred.as_str()))
}

pub fn resolve_shell_runtime(
    preferred: Option<ShellRuntime>,
) -> Result<ResolvedShellRuntime, String> {
    let preferred = preferred.unwrap_or_else(get_default_shell_runtime);

    let build = |runtime: ShellRuntime, program: PathBuf, args: &[&str]| ResolvedShellRuntime {
        runtime,
        program: program.display().to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
    };

    let find_named = |runtime: ShellRuntime, names: &[&str], args: &[&str]| {
        names.iter().find_map(|name| {
            find_program_in_path(name).map(|program| build(runtime, program, args))
        })
    };

    if preferred == ShellRuntime::Auto {
        let resolved = if cfg!(windows) {
            find_named(
                ShellRuntime::Pwsh,
                &["pwsh"],
                &["-NoProfile", "-NonInteractive", "-Command"],
            )
            .or_else(|| {
                find_named(
                    ShellRuntime::PowerShell,
                    &["powershell.exe", "powershell"],
                    &["-NoProfile", "-NonInteractive", "-Command"],
                )
            })
            .or_else(|| {
                find_git_bash_path().map(|path| build(ShellRuntime::GitBash, path, &["-lc"]))
            })
            .or_else(|| find_named(ShellRuntime::Cmd, &["cmd.exe", "cmd"], &["/d", "/s", "/c"]))
        } else {
            find_named(ShellRuntime::Bash, &["bash"], &["-lc"])
                .or_else(|| find_named(ShellRuntime::Sh, &["sh"], &["-lc"]))
        };

        return resolved.ok_or_else(|| {
            if cfg!(windows) {
                "未找到可用 shell：按顺序尝试了 pwsh / powershell / git bash / cmd".to_string()
            } else {
                "未找到可用 shell：按顺序尝试了 bash / sh".to_string()
            }
        });
    }

    match preferred {
        ShellRuntime::Pwsh => {
            if !cfg!(windows) {
                return Err("pwsh 仅支持在 Windows 运行时策略中使用".to_string());
            }
            find_named(
                ShellRuntime::Pwsh,
                &["pwsh"],
                &["-NoProfile", "-NonInteractive", "-Command"],
            )
        }
        ShellRuntime::PowerShell => {
            if !cfg!(windows) {
                return Err("powershell 仅支持在 Windows 运行时策略中使用".to_string());
            }
            find_named(
                ShellRuntime::PowerShell,
                &["powershell.exe", "powershell"],
                &["-NoProfile", "-NonInteractive", "-Command"],
            )
        }
        ShellRuntime::GitBash => {
            if !cfg!(windows) {
                return Err("git_bash 仅支持在 Windows 运行时策略中使用".to_string());
            }
            find_git_bash_path().map(|path| build(ShellRuntime::GitBash, path, &["-lc"]))
        }
        ShellRuntime::Cmd => {
            if !cfg!(windows) {
                return Err("cmd 仅支持在 Windows 运行时策略中使用".to_string());
            }
            find_named(ShellRuntime::Cmd, &["cmd.exe", "cmd"], &["/d", "/s", "/c"])
        }
        ShellRuntime::Sh => find_named(ShellRuntime::Sh, &["sh"], &["-lc"]),
        ShellRuntime::Bash => find_named(ShellRuntime::Bash, &["bash"], &["-lc"]),
        ShellRuntime::Auto => None,
    }
    .ok_or_else(|| format!("未找到可用 shell: {}", preferred.as_str()))
}

pub fn resolve_hook_shell_runtime(
    preferred: Option<ShellRuntime>,
) -> Result<ResolvedShellRuntime, String> {
    match preferred {
        Some(runtime) => resolve_shell_runtime(Some(runtime)),
        None => resolve_default_hook_shell_runtime(),
    }
}

fn resolve_default_hook_shell_runtime() -> Result<ResolvedShellRuntime, String> {
    if cfg!(windows) {
        if let Some(path) = find_git_bash_path() {
            return Ok(ResolvedShellRuntime {
                runtime: ShellRuntime::GitBash,
                program: path.display().to_string(),
                args: vec!["-lc".to_string()],
            });
        }
        if let Some(path) = find_program_in_path("sh") {
            return Ok(ResolvedShellRuntime {
                runtime: ShellRuntime::Sh,
                program: path.display().to_string(),
                args: vec!["-lc".to_string()],
            });
        }
        if let Some(path) =
            find_program_in_path("bash.exe").or_else(|| find_program_in_path("bash"))
            && !path
                .to_string_lossy()
                .replace('/', "\\")
                .to_ascii_lowercase()
                .contains("\\windows\\system32\\bash.exe")
        {
            return Ok(ResolvedShellRuntime {
                runtime: ShellRuntime::Bash,
                program: path.display().to_string(),
                args: vec!["-lc".to_string()],
            });
        }
        return Err(
            "未找到可用的 POSIX shell hook runtime：按顺序尝试了 git bash / bash / sh".to_string(),
        );
    }

    resolve_shell_runtime(Some(ShellRuntime::Bash))
        .or_else(|_| resolve_shell_runtime(Some(ShellRuntime::Sh)))
        .map_err(|_| "未找到可用的 POSIX shell hook runtime：按顺序尝试了 bash / sh".to_string())
}

pub fn resolve_shell_runtime_for_tests<F>(
    preferred: ShellRuntime,
    exists: F,
) -> Result<ResolvedShellRuntime, String>
where
    F: FnMut(&str) -> bool,
{
    resolve_with_checker_impl(preferred, exists)
}

#[cfg(test)]
pub fn resolve_hook_shell_runtime_for_tests<F>(
    preferred: Option<ShellRuntime>,
    mut exists: F,
) -> Result<ResolvedShellRuntime, String>
where
    F: FnMut(&str) -> bool,
{
    match preferred {
        Some(runtime) => resolve_with_checker_impl(runtime, exists),
        None => {
            if cfg!(windows) {
                if exists("git-bash.exe") {
                    return Ok(ResolvedShellRuntime {
                        runtime: ShellRuntime::GitBash,
                        program: "git-bash.exe".to_string(),
                        args: vec!["-lc".to_string()],
                    });
                }
                if exists("sh") {
                    return Ok(ResolvedShellRuntime {
                        runtime: ShellRuntime::Sh,
                        program: "sh".to_string(),
                        args: vec!["-lc".to_string()],
                    });
                }
                if exists("bash.exe") || exists("bash") {
                    return Ok(ResolvedShellRuntime {
                        runtime: ShellRuntime::Bash,
                        program: if exists("bash.exe") {
                            "bash.exe".to_string()
                        } else {
                            "bash".to_string()
                        },
                        args: vec!["-lc".to_string()],
                    });
                }
                Err(
                    "未找到可用的 POSIX shell hook runtime：按顺序尝试了 git bash / bash / sh"
                        .to_string(),
                )
            } else {
                if exists("bash") {
                    return Ok(ResolvedShellRuntime {
                        runtime: ShellRuntime::Bash,
                        program: "bash".to_string(),
                        args: vec!["-lc".to_string()],
                    });
                }
                if exists("sh") {
                    return Ok(ResolvedShellRuntime {
                        runtime: ShellRuntime::Sh,
                        program: "sh".to_string(),
                        args: vec!["-lc".to_string()],
                    });
                }
                Err("未找到可用的 POSIX shell hook runtime：按顺序尝试了 bash / sh".to_string())
            }
        }
    }
}

pub fn kill_process_tree(pid: u32) {
    #[cfg(unix)]
    {
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid as i32),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ShellRuntime, is_git_bash_path, parse_runtime_override,
        resolve_hook_shell_runtime_for_tests, resolve_shell_runtime_for_tests,
    };
    use std::path::Path;

    #[test]
    fn auto_prefers_windows_runtime_order() {
        if cfg!(windows) {
            let resolved = resolve_shell_runtime_for_tests(ShellRuntime::Auto, |name| {
                name == "powershell.exe"
            })
            .unwrap();
            assert_eq!(resolved.runtime, ShellRuntime::PowerShell);
        } else {
            let resolved =
                resolve_shell_runtime_for_tests(ShellRuntime::Auto, |name| name == "bash").unwrap();
            assert_eq!(resolved.runtime, ShellRuntime::Bash);
        }
    }

    #[test]
    fn explicit_runtime_does_not_fallback_to_other_candidates() {
        let result =
            resolve_shell_runtime_for_tests(ShellRuntime::Pwsh, |name| name == "powershell.exe");
        assert!(result.is_err());
    }

    #[test]
    fn parse_roundtrip() {
        for value in [
            "auto",
            "pwsh",
            "powershell",
            "git_bash",
            "cmd",
            "sh",
            "bash",
        ] {
            let parsed = ShellRuntime::parse(value).unwrap();
            assert_eq!(ShellRuntime::parse(parsed.as_str()), Some(parsed));
        }
    }

    #[test]
    fn invalid_runtime_override_is_rejected() {
        let err = parse_runtime_override(Some("powerhsell")).unwrap_err();
        assert!(err.contains("powerhsell"));
    }

    #[test]
    fn git_bash_path_detection_skips_wsl_launcher() {
        assert!(is_git_bash_path(Path::new(
            r"C:\Program Files\Git\bin\bash.exe"
        )));
        assert!(!is_git_bash_path(Path::new(
            r"C:\Windows\System32\bash.exe"
        )));
    }

    #[test]
    fn hook_default_prefers_posix_shells_over_windows_auto_order() {
        let resolved = if cfg!(windows) {
            resolve_hook_shell_runtime_for_tests(None, |name| matches!(name, "pwsh" | "bash.exe"))
                .unwrap()
        } else {
            resolve_hook_shell_runtime_for_tests(None, |name| matches!(name, "pwsh" | "bash"))
                .unwrap()
        };
        assert_eq!(resolved.runtime, ShellRuntime::Bash);
    }

    #[test]
    fn hook_default_prefers_sh_over_plain_bash_on_windows() {
        if !cfg!(windows) {
            return;
        }

        let resolved =
            resolve_hook_shell_runtime_for_tests(None, |name| matches!(name, "sh" | "bash.exe"))
                .unwrap();
        assert_eq!(resolved.runtime, ShellRuntime::Sh);
        assert_eq!(resolved.program, "sh");
    }

    #[test]
    fn hook_default_resolves_to_existing_program() {
        let resolved = super::resolve_hook_shell_runtime(None).unwrap();
        assert!(
            Path::new(&resolved.program).exists(),
            "resolved to {}",
            resolved.program
        );
    }
}
