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

    // 优先检测"长运行服务 + &"模式：在 shell 中用 & 后台化的服务命令
    // 应该使用 run_in_background: true 而非 shell 内部 &
    if let Some(msg) = check_background_service(cmd_trimmed) {
        return Some(msg);
    }

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
                    // && 逻辑 AND 分隔符
                    let seg = cmd[start..i].trim();
                    if !seg.is_empty() {
                        segments.push(seg);
                    }
                    start = i + "&&".len();
                } else if !cmd[..i].ends_with('&') && !cmd[..i].ends_with('>') {
                    // 单个 & 后台运行分隔符（排除 && 的第二个 &，以及重定向 >&）
                    let seg = cmd[start..i].trim();
                    if !seg.is_empty() {
                        segments.push(seg);
                    }
                    start = i + '&'.len_utf8();
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

/// 检测"长运行服务命令 + shell & 后台化"模式。
/// 当用户用 `cmd &` 在 shell 中后台化一个服务进程时，
/// 应该使用 `run_in_background: true` 让工具层面管理，而非依赖 shell 的 &。
fn check_background_service(cmd: &str) -> Option<&'static str> {
    // 需要命令中包含独立的 &（非 &&）后台运行符号
    if !contains_background_ampersand(cmd) {
        return None;
    }

    // 将命令按 & 分割，检查每个后台段是否包含长运行服务命令
    let bg_segments = split_at_background(cmd);
    for segment in &bg_segments {
        // 后台段可能包含 && 或 ; 连接的多条命令，需进一步拆分
        let sub_segments = split_command_segments(segment);
        for sub in &sub_segments {
            let first_cmd = split_at_pipe(sub);
            let tokens = shell_words(first_cmd);
            if tokens.is_empty() {
                continue;
            }
            let first = tokens[0].as_str();
            if is_long_running_server(first, &tokens) {
                return Some(
                    "检测到后台启动长运行服务（shell &）。请设置 run_in_background: true \
                     来启动服务，然后通过单独的 Bash 调用执行健康检查等操作",
                );
            }
        }
    }

    None
}

/// 判断命令是否可能是长运行的服务/服务器进程
fn is_long_running_server(first: &str, tokens: &[String]) -> bool {
    // 直接的 server 命令
    if matches!(
        first,
        "nginx"
            | "apache2"
            | "httpd"
            | "redis-server"
            | "redis-cli"
            | "mongod"
            | "mysqld"
            | "postgres"
            | "pg_ctl"
            | "elasticsearch"
            | "rabbitmq-server"
            | "consul"
            | "etcd"
            | "vault"
            | "minio"
    ) {
        return true;
    }

    // go run / go serve / air（Go 热重载）
    if first == "go" && tokens.iter().skip(1).any(|t| t == "run" || t == "serve") {
        return true;
    }

    // air（Go 热重载工具）
    if first == "air" {
        return true;
    }

    // node/npx 运行服务器
    if first == "node" || first == "npx" {
        // node server.js / node app.js 等
        if tokens.iter().skip(1).any(|t| !t.starts_with('-')) {
            return true;
        }
    }

    // npm/yarn/pnpm/bun run dev/start/serve
    if matches!(first, "npm" | "yarn" | "pnpm" | "bun")
        && tokens
            .iter()
            .skip(1)
            .any(|t| t == "run" || t == "start" || t == "serve" || t == "dev")
    {
        return true;
    }

    // python -m http.server / python manage.py runserver / uvicorn / gunicorn
    if matches!(first, "python" | "python3")
        && tokens
            .iter()
            .skip(1)
            .any(|t| t == "runserver" || t == "http.server" || t == "uvicorn" || t == "gunicorn")
    {
        return true;
    }
    if matches!(
        first,
        "uvicorn" | "gunicorn" | "flask" | "django-admin" | "celery"
    ) {
        return true;
    }

    // java -jar xxx.jar / mvn spring-boot:run / gradle bootRun
    if first == "java" {
        // java -jar app.jar 基本就是启动服务
        if tokens.iter().any(|t| t == "-jar") {
            return true;
        }
    }
    // mvn spring-boot:run / gradle bootRun
    if (first == "mvn" || first == "./mvnw")
        && tokens
            .iter()
            .skip(1)
            .any(|t| t.contains("spring-boot") || t == "tomcat:run" || t == "jetty:run")
    {
        return true;
    }
    if (first == "gradle" || first == "./gradlew")
        && tokens
            .iter()
            .skip(1)
            .any(|t| t == "bootRun" || t.contains("tomcat") || t.contains("jetty"))
    {
        return true;
    }

    // dotnet run/watch
    if first == "dotnet"
        && tokens
            .iter()
            .skip(1)
            .any(|t| t == "run" || t == "watch" || t == "serve")
    {
        return true;
    }

    // cargo watch（Rust 热重载）
    if first == "cargo" && tokens.iter().skip(1).any(|t| t == "watch") {
        return true;
    }

    // docker compose up
    if first == "docker" && tokens.iter().skip(1).any(|t| t == "compose" || t == "up") {
        return true;
    }
    if first == "docker-compose" {
        return true;
    }

    // podman compose up
    if first == "podman" && tokens.iter().skip(1).any(|t| t == "compose") {
        return true;
    }
    if first == "podman-compose" {
        return true;
    }

    // ruby rails server
    if first == "rails" && tokens.iter().skip(1).any(|t| t == "server" || t == "s") {
        return true;
    }
    if first == "bundle"
        && tokens
            .iter()
            .skip(1)
            .any(|t| t == "exec" && tokens.iter().any(|t2| t2 == "rails"))
    {
        return true;
    }

    // php artisan serve
    if first == "php"
        && tokens
            .iter()
            .skip(1)
            .any(|t| t == "artisan" || t == "serve")
    {
        return true;
    }

    false
}

/// 检查命令中是否包含独立的后台运行符号 &（非 &&，非重定向 >&）
fn contains_background_ampersand(cmd: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<char> = cmd.chars().collect();

    for i in 0..chars.len() {
        match chars[i] {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '\\' if !in_single && !in_double => {
                // 跳过转义字符
                continue;
            }
            '&' if !in_single && !in_double => {
                // 检查后面不是 &（排除 &&）
                let next_is_amp = chars.get(i + 1) == Some(&'&');
                // 检查前面不是 &（排除被 && 的第二个 &）
                let prev_is_amp = i > 0 && chars[i - 1] == '&';
                // 检查前面不是 >（排除重定向 >& 或 2>&1）
                let prev_is_redirect = i > 0 && chars[i - 1] == '>';
                if !next_is_amp && !prev_is_amp && !prev_is_redirect {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// 按独立 &（非 &&，非重定向 >&）分割命令，返回被后台化的命令段
fn split_at_background(cmd: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<char> = cmd.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '\\' if !in_single && !in_double => {
                // 跳过转义字符
                i += 1;
                continue;
            }
            '&' if !in_single && !in_double => {
                let next_is_amp = chars.get(i + 1) == Some(&'&');
                let prev_is_amp = i > 0 && chars[i - 1] == '&';
                let prev_is_redirect = i > 0 && chars[i - 1] == '>';
                if !next_is_amp && !prev_is_amp && !prev_is_redirect {
                    // 独立的 &，取前面部分作为一个后台段
                    let seg = cmd[start..i].trim();
                    if !seg.is_empty() {
                        segments.push(seg);
                    }
                    start = i + 1;
                }
            }
            _ => {}
        }
        i += 1;
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
