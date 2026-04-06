use j_cli::config::YamlConfig;
use j_cli::core::search;

#[tauri::command]
pub fn search_aliases(query: String) -> Vec<search::SearchResult> {
    let config = YamlConfig::load();
    search::search_aliases(&query, &config)
}
