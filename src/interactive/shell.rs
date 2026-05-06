use crate::config::YamlConfig;
use crate::constants::shell;
use crate::{error, info};
use colored::Colorize;

/// 进入交互式 shell 子进程
pub fn enter_interactive_shell(config: &YamlConfig) {
    let os = std::env::consts::OS;

    let shell_path = if os == shell::WINDOWS_OS {
        shell::WINDOWS_CMD.to_string()
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| shell::BASH_PATH.to_string())
    };

    info!(
        "进入 shell 模式 ({}), 输入 exit (或按 Ctrl+D) 返回 copilot",
        shell_path
    );

    let mut command = std::process::Command::new(&shell_path);

    for (key, value) in config.collect_alias_envs() {
        command.env(&key, &value);
    }

    let mut cleanup_path: Option<std::path::PathBuf> = None;

    if os != shell::WINDOWS_OS {
        let is_zsh = shell_path.contains("zsh");
        let is_bash = shell_path.contains("bash");

        if is_zsh {
            let pid = std::process::id();
            let tmp_dir = std::env::temp_dir().join(format!("j_shell_zsh_{}", pid));
            let _ = std::fs::create_dir_all(&tmp_dir);

            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            let zshrc_content = format!(
                "# j shell 临时配置 - 自动生成，退出后自动清理\n\
                 export ZDOTDIR=\"{home}\"\n\
                 if [ -f \"{home}/.zshenv\" ]; then\n\
                   source \"{home}/.zshenv\"\n\
                 fi\n\
                 if [ -f \"{home}/.zprofile\" ]; then\n\
                   source \"{home}/.zprofile\"\n\
                 fi\n\
                 if [ -f \"{home}/.zshrc\" ]; then\n\
                   source \"{home}/.zshrc\"\n\
                 fi\n\
                 PROMPT='%F{{green}}shell%f (%F{{cyan}}%~%f) %F{{green}}>%f '\n",
                home = home,
            );

            let zshrc_path = tmp_dir.join(".zshrc");
            if let Err(e) = std::fs::write(&zshrc_path, &zshrc_content) {
                error!("创建临时 .zshrc 失败: {}", e);
                command.env("PROMPT", "%F{green}shell%f (%F{cyan}%~%f) %F{green}>%f ");
            } else {
                command.env("ZDOTDIR", tmp_dir.to_str().unwrap_or("/tmp"));
                cleanup_path = Some(tmp_dir);
            }
        } else if is_bash {
            let pid = std::process::id();
            let tmp_rc = std::env::temp_dir().join(format!("j_shell_bashrc_{}", pid));

            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            let bashrc_content = format!(
                "# j shell 临时配置 - 自动生成，退出后自动清理\n\
                 if [ -f \"{home}/.bashrc\" ]; then\n\
                   source \"{home}/.bashrc\"\n\
                 fi\n\
                 PS1='\\[\\033[32m\\]shell\\[\\033[0m\\] (\\[\\033[36m\\]\\w\\[\\033[0m\\]) \\[\\033[32m\\]>\\[\\033[0m\\] '\n",
                home = home,
            );

            if let Err(e) = std::fs::write(&tmp_rc, &bashrc_content) {
                error!("创建临时 bashrc 失败: {}", e);
                command.env("PS1", "\\[\\033[32m\\]shell\\[\\033[0m\\] (\\[\\033[36m\\]\\w\\[\\033[0m\\]) \\[\\033[32m\\]>\\[\\033[0m\\] ");
            } else {
                command.arg("--rcfile");
                command.arg(tmp_rc.to_str().unwrap_or("/tmp/j_shell_bashrc"));
                cleanup_path = Some(tmp_rc);
            }
        } else {
            command.env(
                "PS1",
                "\x1b[32mshell\x1b[0m (\x1b[36m\\w\x1b[0m) \x1b[32m>\x1b[0m ",
            );
            command.env(
                "PROMPT",
                "\x1b[32mshell\x1b[0m (\x1b[36m%~\x1b[0m) \x1b[32m>\x1b[0m ",
            );
        }
    }

    command
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    match command.status() {
        Ok(status) => {
            if !status.success()
                && let Some(code) = status.code()
            {
                error!("shell 退出码: {}", code);
            }
        }
        Err(e) => {
            error!("启动 shell 失败: {}", e);
        }
    }

    if let Some(path) = cleanup_path {
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }

    info!("{}", "已返回 copilot 交互模式 🚀".green());
}

/// 执行 shell 命令（交互模式下 ! 前缀触发）
pub fn execute_shell_command(cmd: &str, config: &YamlConfig) {
    if cmd.is_empty() {
        return;
    }

    let os = std::env::consts::OS;
    let mut command = if os == shell::WINDOWS_OS {
        let mut c = std::process::Command::new(shell::WINDOWS_CMD);
        c.args([shell::WINDOWS_CMD_FLAG, cmd]);
        c
    } else {
        let shell_path = std::env::var("SHELL").unwrap_or_else(|_| shell::BASH_PATH.to_string());
        let mut c = std::process::Command::new(shell_path);
        c.args([shell::BASH_CMD_FLAG, cmd]);
        c
    };

    for (key, value) in config.collect_alias_envs() {
        command.env(&key, &value);
    }

    match command.status() {
        Ok(status) => {
            if !status.success()
                && let Some(code) = status.code()
            {
                error!("命令退出码: {}", code);
            }
        }
        Err(e) => {
            error!("执行命令失败: {}", e);
        }
    }
}

/// 将所有别名路径注入为当前进程的环境变量
pub fn inject_envs_to_process(config: &YamlConfig) {
    for (key, value) in config.collect_alias_envs() {
        // SAFETY: 交互模式为单线程，set_var 不会引起数据竞争
        unsafe {
            std::env::set_var(&key, &value);
        }
    }
}

/// 展开字符串中的环境变量引用（支持 $VAR 和 ${VAR} 格式）
pub fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '$' && i + 1 < len {
            if chars[i + 1] == '{'
                && let Some(end) = chars[i + 2..].iter().position(|&c| c == '}')
            {
                let var_name: String = chars[i + 2..i + 2 + end].iter().collect();
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    result.push_str(&input[i..i + 3 + end]);
                }
                i = i + 3 + end;
                continue;
            }
            let start = i + 1;
            let mut end = start;
            while end < len && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            if end > start {
                let var_name: String = chars[start..end].iter().collect();
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    let original: String = chars[i..end].iter().collect();
                    result.push_str(&original);
                }
                i = end;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}
