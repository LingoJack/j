use crate::assets::version_template;
use crate::config::YamlConfig;
use crate::constants::{self, CONTAIN_SEARCH_SECTIONS, config_key, section};
use crate::{error, info, md, usage};
use colored::Colorize;

/// 处理 version 命令: j version
pub fn handle_version() {
    let text = version_template()
        .replace("{version}", constants::VERSION)
        .replace("{os}", std::env::consts::OS)
        .replace("{arch}", std::env::consts::ARCH);
    md!("{}", text);
}

/// 处理 exit 命令
pub fn handle_exit() {
    info!("Bye~ See you again 😭");
    std::process::exit(0);
}

/// 处理 log 命令: j log mode <verbose|concise>
pub fn handle_log(key: &str, value: &str, config: &mut YamlConfig) {
    if key == config_key::MODE {
        let mode = if value == config_key::VERBOSE {
            config_key::VERBOSE
        } else {
            config_key::CONCISE
        };
        config.set_property(section::LOG, config_key::MODE, mode);
        info!("✅ 日志模式已切换为: {}", mode);
    } else {
        usage!("j log mode <verbose|concise>");
    }
}

/// 处理 clear 命令: j clear
pub fn handle_clear() {
    // 使用 ANSI 转义序列清屏
    print!("\x1B[2J\x1B[1;1H");
}

/// 处理 contain 命令: j contain <alias> [containers]
/// 在指定分类中查找别名
pub fn handle_contain(alias: &str, containers: Option<&str>, config: &YamlConfig) {
    let sections: Vec<&str> = match containers {
        Some(c) => c.split(',').collect(),
        None => CONTAIN_SEARCH_SECTIONS.to_vec(),
    };

    let mut found = Vec::new();

    for section in &sections {
        if config.contains(section, alias)
            && let Some(value) = config.get_property(section, alias)
        {
            found.push(format!(
                "{} {}: {}",
                format!("[{}]", section).green(),
                alias,
                value
            ));
        }
    }

    if found.is_empty() {
        info!("nothing found 😢");
    } else {
        info!("找到 {} 条结果 😊", found.len().to_string().green());
        for line in &found {
            info!("{}", line);
        }
    }
}

/// 处理 change 命令: j change <part> <field> <value>
/// 直接修改配置文件中的某个字段（如果字段不存在则新增）
pub fn handle_change(part: &str, field: &str, value: &str, config: &mut YamlConfig) {
    if config.get_section(part).is_none() {
        error!("❌ 在配置文件中未找到该 section：{}", part);
        return;
    }

    let old_value = config.get_property(part, field).cloned();
    config.set_property(part, field, value);

    match old_value {
        Some(old) => {
            info!(
                "✅ 已修改 {}.{} 的值为 {}，旧值为 {}",
                part, field, value, old
            );
        }
        None => {
            info!("✅ 已新增 {}.{} = {}", part, field, value);
        }
    }
    info!(
        "🚧 此命令可能会导致配置文件属性错乱而使 Copilot 无法正常使用，请确保在您清楚在做什么的情况下使用"
    );
}

// ========== completion 命令 ==========

/// 处理 completion 命令: j completion [shell]
/// 生成 shell 补全脚本，支持 zsh / bash
pub fn handle_completion(shell_type: Option<&str>, config: &YamlConfig) {
    let shell = shell_type.unwrap_or("zsh");

    match shell {
        "zsh" => generate_zsh_completion(config),
        "bash" => generate_bash_completion(config),
        _ => {
            error!("❌ 不支持的 shell 类型: {}，可选: zsh, bash", shell);
            usage!("j completion [zsh|bash]");
        }
    }
}

/// 生成 zsh 补全脚本
fn generate_zsh_completion(config: &YamlConfig) {
    // 收集所有别名
    let mut all_aliases = Vec::new();
    for s in constants::ALIAS_EXISTS_SECTIONS {
        if let Some(map) = config.get_section(s) {
            for key in map.keys() {
                if !all_aliases.contains(key) {
                    all_aliases.push(key.clone());
                }
            }
        }
    }
    all_aliases.sort();

    // 收集编辑器别名（后续参数需要文件路径补全）
    let editor_aliases: Vec<String> = config
        .get_section(section::EDITOR)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    // 收集浏览器别名
    let browser_aliases: Vec<String> = config
        .get_section(section::BROWSER)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    // 收集内置命令关键字
    let keywords = constants::cmd::all_keywords();

    // 子命令列表
    let subcmds = keywords.to_vec();
    let subcmds_str = subcmds.join(" ");

    // 别名列表
    let aliases_str = all_aliases.join(" ");

    // 编辑器别名模式匹配
    let editor_pattern = if editor_aliases.is_empty() {
        String::new()
    } else {
        editor_aliases.join("|")
    };

    // 浏览器别名模式匹配
    let browser_pattern = if browser_aliases.is_empty() {
        String::new()
    } else {
        browser_aliases.join("|")
    };

    // 生成 zsh 补全脚本
    let mut script = String::new();
    script.push_str("#compdef j\n");
    script.push_str("# Zsh completion for j (work-copilot)\n");
    script.push_str("# 生成方式: eval \"$(j completion zsh)\"\n");
    script.push_str(
        "# 或: j completion zsh > ~/.zsh/completions/_j && fpath=(~/.zsh/completions $fpath)\n\n",
    );
    script.push_str("_j() {\n");
    script.push_str("    local curcontext=\"$curcontext\" state line\n");
    script.push_str("    typeset -A opt_args\n\n");

    // 子命令和别名合并列表
    script.push_str(&format!("    local -a subcmds=({})\n", subcmds_str));
    script.push_str(&format!("    local -a aliases=({})\n", aliases_str));

    // 编辑器/浏览器别名列表（用于判断是否需要文件补全）
    if !editor_pattern.is_empty() {
        script.push_str(&format!(
            "    local -a editor_aliases=({})\n",
            editor_aliases.join(" ")
        ));
    }

    script.push_str("\n    _arguments -C \\\n");
    script.push_str("        '1: :->cmd' \\\n");
    script.push_str("        '*: :->args'\n\n");

    script.push_str("    case $state in\n");
    script.push_str("        cmd)\n");
    script.push_str("            _describe 'command' subcmds\n");
    script.push_str("            _describe 'alias' aliases\n");
    script.push_str("            ;;\n");
    script.push_str("        args)\n");
    script.push_str("            case $words[2] in\n");

    // set / modify 命令：第二个参数是别名，第三个参数是文件路径
    script.push_str("                set|s|modify|mf)\n");
    script.push_str("                    if (( CURRENT == 3 )); then\n");
    script.push_str("                        _describe 'alias' aliases\n");
    script.push_str("                    else\n");
    script.push_str("                        _files\n");
    script.push_str("                    fi\n");
    script.push_str("                    ;;\n");

    // remove / rename 命令：补全别名
    script.push_str("                rm|remove|rename|rn|note|nt|denote|dnt|contain|find)\n");
    script.push_str("                    _describe 'alias' aliases\n");
    script.push_str("                    ;;\n");

    // list 命令：补全 section 名
    let sections_str = constants::ALL_SECTIONS.join(" ");
    script.push_str("                ls|list)\n");
    script.push_str(&format!(
        "                    local -a sections=(all {})\n",
        sections_str
    ));
    script.push_str("                    _describe 'section' sections\n");
    script.push_str("                    ;;\n");

    // reportctl 命令：补全子操作
    script.push_str("                reportctl|rctl)\n");
    script
        .push_str("                    local -a rctl_actions=(new sync push pull set-url open)\n");
    script.push_str("                    _describe 'action' rctl_actions\n");
    script.push_str("                    ;;\n");

    // log 命令
    script.push_str("                log)\n");
    script.push_str("                    if (( CURRENT == 3 )); then\n");
    script.push_str("                        local -a log_keys=(mode)\n");
    script.push_str("                        _describe 'key' log_keys\n");
    script.push_str("                    else\n");
    script.push_str("                        local -a log_values=(verbose concise)\n");
    script.push_str("                        _describe 'value' log_values\n");
    script.push_str("                    fi\n");
    script.push_str("                    ;;\n");

    // change 命令：补全 section
    script.push_str("                change|chg)\n");
    script.push_str(&format!(
        "                    local -a sections=({})\n",
        sections_str
    ));
    script.push_str("                    _describe 'section' sections\n");
    script.push_str("                    ;;\n");

    // time 命令
    script.push_str("                time)\n");
    script.push_str("                    local -a time_funcs=(countdown)\n");
    script.push_str("                    _describe 'function' time_funcs\n");
    script.push_str("                    ;;\n");

    // completion 命令
    script.push_str("                completion)\n");
    script.push_str("                    local -a shells=(zsh bash)\n");
    script.push_str("                    _describe 'shell' shells\n");
    script.push_str("                    ;;\n");

    // 编辑器类别名：文件路径补全
    if !editor_pattern.is_empty() {
        script.push_str(&format!("                {})\n", editor_pattern));
        script.push_str("                    _files\n");
        script.push_str("                    ;;\n");
    }

    // 浏览器类别名：别名 + 文件路径补全
    if !browser_pattern.is_empty() {
        script.push_str(&format!("                {})\n", browser_pattern));
        script.push_str("                    _describe 'alias' aliases\n");
        script.push_str("                    _files\n");
        script.push_str("                    ;;\n");
    }

    // 其他别名：文件路径补全 + 别名补全（CLI 工具可能接受文件参数）
    script.push_str("                *)\n");
    script.push_str("                    _files\n");
    script.push_str("                    _describe 'alias' aliases\n");
    script.push_str("                    ;;\n");

    script.push_str("            esac\n");
    script.push_str("            ;;\n");
    script.push_str("    esac\n");
    script.push_str("}\n\n");
    script.push_str("_j \"$@\"\n");

    print!("{}", script);
}

/// 生成 bash 补全脚本
fn generate_bash_completion(config: &YamlConfig) {
    // 收集所有别名
    let mut all_aliases = Vec::new();
    for s in constants::ALIAS_EXISTS_SECTIONS {
        if let Some(map) = config.get_section(s) {
            for key in map.keys() {
                if !all_aliases.contains(key) {
                    all_aliases.push(key.clone());
                }
            }
        }
    }
    all_aliases.sort();

    let keywords = constants::cmd::all_keywords();
    let all_completions: Vec<String> = keywords
        .iter()
        .map(|s| s.to_string())
        .chain(all_aliases.iter().cloned())
        .collect();

    // 收集编辑器别名
    let editor_aliases: Vec<String> = config
        .get_section(section::EDITOR)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    let mut script = String::new();
    script.push_str("# Bash completion for j (work-copilot)\n");
    script.push_str("# 生成方式: eval \"$(j completion bash)\"\n");
    script.push_str("# 或: j completion bash > /etc/bash_completion.d/j\n\n");
    script.push_str("_j_completion() {\n");
    script.push_str("    local cur prev words cword\n");
    script.push_str("    _init_completion || return\n\n");

    script.push_str(&format!(
        "    local commands=\"{}\"\n",
        all_completions.join(" ")
    ));
    script.push_str(&format!(
        "    local aliases=\"{}\"\n",
        all_aliases.join(" ")
    ));

    if !editor_aliases.is_empty() {
        script.push_str(&format!(
            "    local editor_aliases=\"{}\"\n",
            editor_aliases.join(" ")
        ));
    }

    script.push_str("\n    if [[ $cword -eq 1 ]]; then\n");
    script.push_str("        COMPREPLY=( $(compgen -W \"$commands\" -- \"$cur\") )\n");
    script.push_str("        return\n");
    script.push_str("    fi\n\n");

    script.push_str("    case \"${words[1]}\" in\n");
    script.push_str("        set|s|modify|mf)\n");
    script.push_str("            if [[ $cword -eq 2 ]]; then\n");
    script.push_str("                COMPREPLY=( $(compgen -W \"$aliases\" -- \"$cur\") )\n");
    script.push_str("            else\n");
    script.push_str("                _filedir\n");
    script.push_str("            fi\n");
    script.push_str("            ;;\n");
    script.push_str("        rm|remove|rename|rn|note|nt|denote|dnt|contain|find)\n");
    script.push_str("            COMPREPLY=( $(compgen -W \"$aliases\" -- \"$cur\") )\n");
    script.push_str("            ;;\n");
    script.push_str("        reportctl|rctl)\n");
    script.push_str(
        "            COMPREPLY=( $(compgen -W \"new sync push pull set-url open\" -- \"$cur\") )\n",
    );
    script.push_str("            ;;\n");

    // 编辑器别名：文件路径补全
    if !editor_aliases.is_empty() {
        for alias in &editor_aliases {
            script.push_str(&format!("        {})\n", alias));
            script.push_str("            _filedir\n");
            script.push_str("            ;;\n");
        }
    }

    script.push_str("        *)\n");
    script.push_str("            _filedir\n");
    script.push_str("            COMPREPLY+=( $(compgen -W \"$aliases\" -- \"$cur\") )\n");
    script.push_str("            ;;\n");
    script.push_str("    esac\n");
    script.push_str("}\n\n");
    script.push_str("complete -F _j_completion j\n");

    print!("{}", script);
}
