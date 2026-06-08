use crate::config::YamlConfig;
use crate::constants::shell;
use crate::markdown::highlight::highlight_code_line;
use crate::markdown::theme::MdStyle;
use crate::theme::Theme;
use crate::{error, info};
use colored::Colorize;
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use ratatui::style::{Color, Modifier, Style};
use std::io::{BufRead, BufReader, Read, Write};

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
            let zshrc_content = build_zshrc_content(&home);

            let zshrc_path = tmp_dir.join(".zshrc");
            if let Err(e) = std::fs::write(&zshrc_path, &zshrc_content) {
                error!("创建临时 .zshrc 失败: {}", e);
                command.env(
                    "PROMPT",
                    "\n%F{green}(%F{cyan}shell%F{green})%f %F{cyan}%~%f\n%F{cyan}❯%f ",
                );
            } else {
                command.env("ZDOTDIR", tmp_dir.to_str().unwrap_or("/tmp"));
                cleanup_path = Some(tmp_dir);
            }
        } else if is_bash {
            let pid = std::process::id();
            let tmp_rc = std::env::temp_dir().join(format!("j_shell_bashrc_{}", pid));

            let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
            let bashrc_content = build_bashrc_content(&home);

            if let Err(e) = std::fs::write(&tmp_rc, &bashrc_content) {
                error!("创建临时 bashrc 失败: {}", e);
                command.env(
                    "PS1",
                    "\n\\[\\033[32m\\](\\[\\033[36m\\]shell\\[\\033[32m\\])\\[\\033[0m\\] \\[\\033[36m\\]\\w\\[\\033[0m\\]\n\\[\\033[36m\\]❯\\[\\033[0m\\] ",
                );
            } else {
                command.arg("--rcfile");
                command.arg(tmp_rc.to_str().unwrap_or("/tmp/j_shell_bashrc"));
                cleanup_path = Some(tmp_rc);
            }
        } else {
            command.env(
                "PS1",
                "\n\x1b[32m(\x1b[36mshell\x1b[32m)\x1b[0m \x1b[36m\\w\x1b[0m\n\x1b[36m❯\x1b[0m ",
            );
            command.env(
                "PROMPT",
                "\n\x1b[32m(\x1b[36mshell\x1b[32m)\x1b[0m \x1b[36m%~\x1b[0m\n\x1b[36m❯\x1b[0m ",
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

fn build_zshrc_content(home: &str) -> String {
    format!(
        r#"# j shell 临时配置 - 自动生成，退出后自动清理
export ZDOTDIR="{home}"
if [ -f "{home}/.zshenv" ]; then
  source "{home}/.zshenv"
fi
if [ -f "{home}/.zprofile" ]; then
  source "{home}/.zprofile"
fi
if [ -f "{home}/.zshrc" ]; then
  source "{home}/.zshrc"
fi

autoload -Uz vcs_info
zstyle ':vcs_info:*' enable git
zstyle ':vcs_info:git:*' formats '%F{{magenta}} %b%f'
zstyle ':vcs_info:git:*' actionformats '%F{{magenta}} %b|%a%f'
setopt prompt_subst

_j_shell_git_dirty() {{
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || return
  if ! git diff --quiet --ignore-submodules --cached 2>/dev/null || \
     ! git diff --quiet --ignore-submodules 2>/dev/null || \
     [ -n "$(git ls-files --others --exclude-standard 2>/dev/null)" ]; then
    print -r -- ' %F{{yellow}}✱%f'
  fi
}}

_j_shell_prompt_symbol() {{
  print -r -- '%F{{cyan}}❯%f'
}}

_j_shell_virtualenv() {{
  if [ -n "$VIRTUAL_ENV" ]; then
    print -r -- " %F{{blue}}($(basename "$VIRTUAL_ENV"))%f"
  fi
}}

precmd() {{
  local prompt_symbol git_dirty_segment virtualenv_segment
  vcs_info
  prompt_symbol=$(_j_shell_prompt_symbol)
  git_dirty_segment=$(_j_shell_git_dirty)
  virtualenv_segment=$(_j_shell_virtualenv)
  PROMPT="
%F{{green}}(%F{{cyan}}shell%F{{green}})%f${{virtualenv_segment}} %F{{cyan}}%~%f ${{vcs_info_msg_0_}}${{git_dirty_segment}}
${{prompt_symbol}} "
}}
"#,
        home = home,
    )
}

fn build_bashrc_content(home: &str) -> String {
    format!(
        r#"# j shell 临时配置 - 自动生成，退出后自动清理
if [ -f "{home}/.bashrc" ]; then
  source "{home}/.bashrc"
fi

__j_shell_git_info() {{
  local branch dirty
  branch=$(git symbolic-ref --short HEAD 2>/dev/null || git rev-parse --short HEAD 2>/dev/null) || return
  dirty=""
  if ! git diff --quiet --ignore-submodules --cached 2>/dev/null || \
     ! git diff --quiet --ignore-submodules 2>/dev/null || \
     [ -n "$(git ls-files --others --exclude-standard 2>/dev/null)" ]; then
    dirty=" dirty"
  fi
  printf ' \\[\033[35m\\] %s\\[\033[0m\\]\\[\033[33m\\]%s\\[\033[0m\\]' "$branch" "$dirty"
}}

__j_shell_virtualenv() {{
  if [ -n "$VIRTUAL_ENV" ]; then
    printf ' \\[\033[34m\\](%s)\\[\033[0m\\]' "$(basename "$VIRTUAL_ENV")"
  fi
}}

__j_shell_set_prompt() {{
  PS1="\n\\[\033[32m\\](\\[\033[36m\\]shell\\[\033[32m\\])\\[\033[0m\\]$(__j_shell_virtualenv) \\[\033[36m\\]\\w\\[\033[0m\\]$(__j_shell_git_info)\n\\[\033[36m\\]❯\\[\033[0m\\] "
}}

PROMPT_COMMAND=__j_shell_set_prompt
"#,
        home = home,
    )
}

/// 高亮展示 shell 命令，复用 Chat Markdown 代码块的 bash 语法高亮。
pub fn highlight_shell_command(cmd: &str) -> String {
    let theme = Theme::terminal().code_syntax_theme();
    highlight_code_line(cmd, "bash", &theme)
        .into_iter()
        .map(|span| span_to_ansi(&span.content, span.style))
        .collect()
}

fn span_to_ansi(content: &str, style: Style) -> String {
    let mut ansi = String::new();

    if style.add_modifier.contains(Modifier::BOLD) {
        ansi.push_str("\x1b[1m");
    }
    if style.add_modifier.contains(Modifier::ITALIC) {
        ansi.push_str("\x1b[3m");
    }
    if let Some(fg) = style.fg {
        ansi.push_str(&fg_to_ansi(fg));
    }

    if ansi.is_empty() {
        content.to_string()
    } else {
        format!("{}{}\x1b[0m", ansi, content)
    }
}

fn fg_to_ansi(color: Color) -> String {
    match color {
        Color::Black => "\x1b[30m".to_string(),
        Color::Red => "\x1b[31m".to_string(),
        Color::Green => "\x1b[32m".to_string(),
        Color::Yellow => "\x1b[33m".to_string(),
        Color::Blue => "\x1b[34m".to_string(),
        Color::Magenta => "\x1b[35m".to_string(),
        Color::Cyan => "\x1b[36m".to_string(),
        Color::Gray | Color::White => "\x1b[37m".to_string(),
        Color::DarkGray => "\x1b[90m".to_string(),
        Color::LightRed => "\x1b[91m".to_string(),
        Color::LightGreen => "\x1b[92m".to_string(),
        Color::LightYellow => "\x1b[93m".to_string(),
        Color::LightBlue => "\x1b[94m".to_string(),
        Color::LightMagenta => "\x1b[95m".to_string(),
        Color::LightCyan => "\x1b[96m".to_string(),
        Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
        Color::Indexed(i) => format!("\x1b[38;5;{}m", i),
        Color::Reset => "\x1b[39m".to_string(),
    }
}

pub struct ShellSession {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    reader: BufReader<Box<dyn Read + Send>>,
    marker: String,
}

impl ShellSession {
    const MARKER_PREFIX: &'static str = "__JCLI_SHELL_DONE_";

    pub fn new(config: &YamlConfig) -> Option<Self> {
        let os = std::env::consts::OS;
        if os == shell::WINDOWS_OS {
            return None;
        }

        let shell_path = std::env::var("SHELL").unwrap_or_else(|_| shell::BASH_PATH.to_string());
        let marker = format!("{}{}", Self::MARKER_PREFIX, std::process::id());
        let pty_system = NativePtySystem::default();
        let pair = match pty_system.openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(pair) => pair,
            Err(err) => {
                error!("启动 shell PTY 失败: {}", err);
                return None;
            }
        };

        let mut command = CommandBuilder::new(&shell_path);
        command.arg(shell::BASH_CMD_FLAG);
        command.arg(build_persistent_shell_bootstrap(&shell_path));
        command.env("JCLI_SHELL_MARKER", &marker);
        command.env("TERM", "xterm-256color");
        command.env("CLICOLOR_FORCE", "1");
        command.env("GIT_PAGER", "cat");
        if let Ok(current_dir) = std::env::current_dir() {
            command.cwd(current_dir.as_os_str());
            command.env("PWD", current_dir.as_os_str());
        }

        for (key, value) in config.collect_alias_envs() {
            command.env(&key, &value);
        }

        let child = match pair.slave.spawn_command(command) {
            Ok(child) => child,
            Err(err) => {
                error!("启动 shell 会话失败: {}", err);
                return None;
            }
        };

        let reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(err) => {
                error!("启动 shell 会话失败: 无法打开 reader: {}", err);
                return None;
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(err) => {
                error!("启动 shell 会话失败: 无法打开 writer: {}", err);
                return None;
            }
        };

        let mut session = Self {
            child,
            _master: pair.master,
            writer,
            reader: BufReader::new(reader),
            marker,
        };
        if !session.wait_until_ready() {
            return None;
        }
        Some(session)
    }

    fn wait_until_ready(&mut self) -> bool {
        let marker_prefix = format!("{}:ready:", self.marker);
        let mut line = String::new();
        loop {
            line.clear();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    error!("shell 会话启动后立即退出");
                    return false;
                }
                Ok(_) => {
                    let normalized = line.trim_matches(['\r', '\n']);
                    if normalized.starts_with(&marker_prefix) {
                        return true;
                    }
                }
                Err(err) => {
                    error!("读取 shell 启动输出失败: {}", err);
                    return false;
                }
            }
        }
    }

    pub fn execute(&mut self, cmd: &str) -> bool {
        if cmd.trim().is_empty() {
            return true;
        }

        let marker_command = format!("\nprintf '%s:%s:%s\\n' '{}' \"$?\" \"$PWD\"\n", self.marker);
        if writeln!(self.writer, "{}", cmd)
            .and_then(|()| write!(self.writer, "{}", marker_command))
            .and_then(|()| self.writer.flush())
            .is_err()
        {
            error!("shell 会话已不可用");
            return false;
        }

        self.read_until_marker()
    }

    fn read_until_marker(&mut self) -> bool {
        let marker_prefix = format!("{}:", self.marker);
        let mut line = String::new();

        loop {
            line.clear();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    error!("shell 会话已退出");
                    return false;
                }
                Ok(_) => {
                    let normalized = line.trim_matches(['\r', '\n']);
                    if let Some(metadata) = normalized.strip_prefix(&marker_prefix) {
                        self.sync_cwd_from_marker(metadata);
                        return true;
                    }
                    print!("{}", line);
                    let _ = std::io::stdout().flush();
                }
                Err(err) => {
                    error!("读取 shell 输出失败: {}", err);
                    return false;
                }
            }
        }
    }

    fn sync_cwd_from_marker(&self, metadata: &str) {
        let Some((_, pwd)) = metadata.split_once(':') else {
            return;
        };
        if pwd.is_empty() {
            return;
        }

        if std::env::set_current_dir(pwd).is_err() {
            return;
        }

        // SAFETY: 普通交互 REPL 单线程串行执行命令；这里同步 shell 会话 cwd 到
        // 当前进程环境，供 prompt 和后续子进程使用。
        unsafe {
            std::env::set_var("PWD", pwd);
        }
    }
}

impl Drop for ShellSession {
    fn drop(&mut self) {
        let _ = writeln!(self.writer, "exit");
        let _ = self.writer.flush();
        let _ = self.child.wait();
    }
}

fn build_persistent_shell_bootstrap(shell_path: &str) -> String {
    let rc_source = if shell_path.contains("zsh") {
        "source ~/.zshrc 2>/dev/null"
    } else if shell_path.contains("bash") {
        "shopt -s expand_aliases; source ~/.bashrc 2>/dev/null"
    } else {
        ""
    };
    let loop_script = "stty -echo 2>/dev/null; printf '%s:ready:%s\\n' \"$JCLI_SHELL_MARKER\" \"$PWD\"; while IFS= read -r __jcli_cmd; do eval \"$__jcli_cmd\"; done";
    if rc_source.is_empty() {
        loop_script.to_string()
    } else {
        format!("{}; {}", rc_source, loop_script)
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
