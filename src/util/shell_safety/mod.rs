/// 检查命令是否属于危险操作（rm -rf /、mkfs、dd 等），用于 Shell 安全审核
pub fn is_dangerous_command(cmd: &str) -> bool {
    let cmd_lower = cmd.to_lowercase();
    let tokens = shell_words(&cmd_lower);

    if tokens.is_empty() {
        return false;
    }

    let first = &tokens[0];

    if first.starts_with("mkfs") || first.starts_with("mkfs.") {
        return true;
    }

    if first == "dd"
        && tokens
            .iter()
            .any(|t| t.starts_with("of=/dev/") && !t.starts_with("of=/dev/null"))
    {
        return true;
    }

    if cmd_lower.contains(":(){:|:&};:") || cmd_lower.contains(":(){ :|:& };:") {
        return true;
    }

    if first == "chmod" {
        let has_recursive = tokens.iter().any(|t| t == "-r" || t == "-R");
        if has_recursive && cmd_lower.contains("777") && tokens.last().is_some_and(|t| t == "/") {
            return true;
        }
    }

    if first == "chown" && cmd_lower.contains("-r") && tokens.last().is_some_and(|t| t == "/") {
        return true;
    }

    if tokens.iter().any(|t| t == ">" || t == ">>")
        && tokens.iter().any(|t| {
            t.starts_with("/dev/sd") || t.starts_with("/dev/nvme") || t.starts_with("/dev/disk")
        })
    {
        return true;
    }

    if (first == "curl" || first == "wget")
        && (cmd_lower.contains("| sh")
            || cmd_lower.contains("| bash")
            || cmd_lower.contains("| zsh"))
    {
        return true;
    }

    if first == "alias" && tokens.len() == 1 {
        return true;
    }

    if first == "rm" {
        let has_recursive = tokens.iter().any(|t| {
            t == "-r" || t == "-rf" || t == "-fr" || t.starts_with("-r") || t.starts_with("-f")
        });
        let targets_root = tokens.iter().any(|t| t == "/" || t == "/*");
        if has_recursive && targets_root {
            return true;
        }
    }

    false
}

/// 检查命令是否为阻塞式交互命令（vim、top、less 等），返回匹配到的命令名
pub fn check_blocking_command(cmd: &str) -> Option<&'static str> {
    let cmd_trimmed = cmd.trim();
    let segments = split_command_segments(cmd_trimmed);

    for segment in &segments {
        if let Some(msg) = check_single_segment(segment) {
            return Some(msg);
        }
    }
    None
}

fn split_command_segments(cmd: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;

    for (i, c) in cmd.char_indices() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ';' if !in_single && !in_double => {
                let seg = cmd[start..i].trim();
                if !seg.is_empty() {
                    segments.push(seg);
                }
                start = i + ';'.len_utf8();
            }
            '&' if !in_single && !in_double => {
                let rest = &cmd[i + '&'.len_utf8()..];
                if rest.starts_with('&') {
                    let seg = cmd[start..i].trim();
                    if !seg.is_empty() {
                        segments.push(seg);
                    }
                    start = i + "&&".len();
                }
            }
            '|' if !in_single && !in_double => {
                let rest = &cmd[i + '|'.len_utf8()..];
                if rest.starts_with('|') {
                    let seg = cmd[start..i].trim();
                    if !seg.is_empty() {
                        segments.push(seg);
                    }
                    start = i + "||".len();
                }
            }
            _ => {}
        }
    }
    let last = cmd[start..].trim();
    if !last.is_empty() {
        segments.push(last);
    }
    if segments.is_empty() {
        segments.push(cmd);
    }
    segments
}

fn check_single_segment(segment: &str) -> Option<&'static str> {
    let first_cmd = split_at_pipe(segment);
    let tokens = shell_words(first_cmd);
    if tokens.is_empty() {
        return None;
    }

    let first = tokens[0].as_str();

    if first == "ssh" {
        let non_flag_args: Vec<&String> = tokens
            .iter()
            .skip(1)
            .filter(|t| !t.starts_with('-'))
            .collect();
        if non_flag_args.len() >= 2 {
            return None;
        }
        return Some(
            "SSH 是交互式会话，不支持前台运行。如需远程执行命令，请用 ssh host 'command' 形式并设置 run_in_background: true",
        );
    }
    if first == "telnet" || first == "mosh" {
        return Some(
            "telnet/mosh 是交互式会话，不支持前台运行。如需远程执行命令，请用 ssh host 'command' 形式并设置 run_in_background: true",
        );
    }

    if matches!(first, "vim" | "vi" | "nano" | "emacs" | "micro" | "pico") {
        return Some(
            "交互式编辑器不支持前台运行。请使用 Edit/Write 工具编辑文件，或使用 sed 进行文本替换",
        );
    }
    if first == "code" {
        let has_non_interactive_flag = tokens.iter().skip(1).any(|t| {
            t.starts_with("--diff")
                || t.starts_with("--version")
                || t.starts_with("--list-extensions")
                || t.starts_with("--install-extension")
                || t.starts_with("--uninstall-extension")
        });
        if !has_non_interactive_flag {
            return Some(
                "交互式编辑器不支持前台运行。请使用 Edit/Write 工具编辑文件，或使用 sed 进行文本替换",
            );
        }
        return None;
    }

    if matches!(first, "less" | "more" | "most") {
        return Some(
            "分页器不支持前台运行。请直接运行命令（输出会自动捕获），或使用 Read 工具查看文件",
        );
    }

    if matches!(first, "ipython" | "pry" | "groovysh") {
        return Some(
            "交互式 REPL 不支持前台运行。请用 -c 参数执行单条命令，或设置 run_in_background: true",
        );
    }
    if matches!(first, "python" | "python3" | "python2") {
        let has_script = tokens
            .iter()
            .skip(1)
            .any(|t| t == "-c" || t == "-m" || !t.starts_with('-'));
        if !has_script {
            return Some(
                "交互式 Python REPL 不支持前台运行。请用 -c 参数执行单条命令（如 python3 -c 'code'），或设置 run_in_background: true",
            );
        }
        return None;
    }
    if first == "node" {
        let has_script = tokens
            .iter()
            .skip(1)
            .any(|t| t == "-e" || t == "--eval" || !t.starts_with('-'));
        if !has_script {
            return Some(
                "交互式 Node REPL 不支持前台运行。请用 -e 参数执行单条命令（如 node -e 'code'），或设置 run_in_background: true",
            );
        }
        return None;
    }
    if first == "irb" {
        return Some(
            "交互式 Ruby REPL 不支持前台运行。请用 ruby -e 'code' 执行单条命令，或设置 run_in_background: true",
        );
    }
    if first == "lua" {
        let has_script = tokens
            .iter()
            .skip(1)
            .any(|t| t == "-e" || !t.starts_with('-'));
        if !has_script {
            return Some(
                "交互式 Lua REPL 不支持前台运行。请用 -e 参数执行单条命令，或设置 run_in_background: true",
            );
        }
        return None;
    }
    if first == "php" {
        if tokens
            .iter()
            .skip(1)
            .any(|t| t == "-a" || t == "--interactive")
        {
            return Some(
                "交互式 PHP REPL 不支持前台运行。请用 -r 参数执行单条命令，或设置 run_in_background: true",
            );
        }
        return None;
    }
    if first == "r" || first == "R" {
        if tokens.len() > 1 && (tokens[1] == "CMD" || tokens[1] == "cmd") {
            return None;
        }
        return Some(
            "交互式 R 不支持前台运行。请用 R CMD batch 或 Rscript 运行脚本，或设置 run_in_background: true",
        );
    }
    if first == "scala" {
        let has_script = tokens
            .iter()
            .skip(1)
            .any(|t| t == "-e" || !t.starts_with('-'));
        if !has_script {
            return Some(
                "交互式 Scala REPL 不支持前台运行。请用 -e 参数执行单条命令，或设置 run_in_background: true",
            );
        }
        return None;
    }

    if matches!(first, "top" | "htop" | "btop" | "glances") {
        return Some(
            "持续监控命令不支持前台运行。请用单次快照方式执行（如 ps aux），或设置 run_in_background: true",
        );
    }
    if first == "watch" {
        return Some(
            "watch 持续刷新不支持前台运行。请直接执行命令获取单次输出，或设置 run_in_background: true",
        );
    }

    if matches!(first, "gdb" | "lldb" | "pdb") {
        if first == "gdb" && tokens.iter().any(|t| t == "--batch" || t == "-batch") {
            return None;
        }
        if first == "lldb"
            && tokens
                .iter()
                .any(|t| t == "--batch" || t == "-batch" || t == "-o")
        {
            return None;
        }
        return Some(
            "调试器不支持前台运行。请使用 --batch 非交互模式，或设置 run_in_background: true",
        );
    }
    if matches!(first, "strace" | "ltrace") {
        return None;
    }

    if matches!(first, "apt" | "apt-get" | "yum" | "dnf" | "pacman") {
        let has_yes = tokens
            .iter()
            .any(|t| t == "-y" || t == "--yes" || t == "--assumeyes" || t == "--noconfirm");
        if !has_yes {
            return Some(
                "包管理器通常需要交互确认。请加 -y/--yes 标志（如 apt-get install -y pkg），或设置 run_in_background: true",
            );
        }
        return None;
    }
    if first == "brew" {
        return None;
    }

    if first == "docker" {
        let has_it = tokens
            .iter()
            .any(|t| t == "-it" || t == "-ti" || t == "-i" || t == "--interactive");
        if has_it {
            let subcmd = tokens.get(1).map(|s| s.as_str()).unwrap_or("");
            if matches!(subcmd, "run" | "exec") {
                return Some(
                    "交互式 Docker 命令不支持前台运行。请去掉 -i/-t 标志，或设置 run_in_background: true",
                );
            }
        }
        return None;
    }

    None
}

fn split_at_pipe(segment: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    for (i, c) in segment.char_indices() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '|' if !in_single && !in_double => return segment[..i].trim(),
            _ => {}
        }
    }
    segment.trim()
}

/// 类 sh 单词拆分：处理单引号、双引号和反斜杠转义，返回拆分后的参数列表
pub fn shell_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for c in input.chars() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

#[cfg(test)]
mod tests;
