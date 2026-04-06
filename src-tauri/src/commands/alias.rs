use j_cli::config::YamlConfig;
use j_cli::core::open;

#[tauri::command]
pub fn open_alias(alias: String, args: Vec<String>) -> Result<String, String> {
    let config = YamlConfig::load();
    let mut full_args = vec![alias.clone()];
    full_args.extend(args);
    open::open_alias_silent(&alias, &full_args, &config)
}
