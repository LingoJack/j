mod cli;
mod command;
mod config;
mod constants;
mod interactive;
mod tui;
mod util;

use clap::Parser;
use cli::Cli;
use config::YamlConfig;

fn main() {
    // 加载配置
    let mut config = YamlConfig::load();

    let verbose = config.is_verbose();
    let start = if verbose {
        Some(std::time::Instant::now())
    } else {
        None
    };

    // 检查是否有命令行参数
    // 如果 argv 只有一个元素（程序名），进入交互模式
    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args.len() <= 1 {
        // 无参数：进入交互模式
        interactive::run_interactive(&mut config);
        return;
    }

    // 尝试用 clap 解析命令
    // 如果用户输入的是 `j <alias>` 这种非子命令形式，clap 会解析失败
    // 这时候我们 fallback 到别名打开逻辑
    let cli = Cli::try_parse();

    match cli {
        Ok(cli) => {
            match cli.command {
                Some(subcmd) => {
                    command::dispatch(subcmd, &mut config);
                }
                None => {
                    if cli.args.is_empty() {
                        // 不应该走到这里（已在上面处理了无参数情况）
                        interactive::run_interactive(&mut config);
                    } else {
                        // 带参数但没匹配到子命令 → 别名打开
                        command::open::handle_open(&cli.args, &config);
                    }
                }
            }
        }
        Err(_) => {
            // clap 解析失败，可能是用户输入了别名
            // 例如: j chrome, j vscode file.txt
            // 跳过 argv[0]（程序名），把剩余的作为别名参数
            let alias_args: Vec<String> = raw_args[1..].to_vec();
            command::open::handle_open(&alias_args, &config);
        }
    }

    if let Some(start) = start {
        let elapsed = start.elapsed();
        debug_log!(config, "duration: {} ms", elapsed.as_millis());
    }
}