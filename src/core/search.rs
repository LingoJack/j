//! 搜索核心逻辑（GUI 友好，无终端依赖）

use crate::config::YamlConfig;
use crate::constants::section;
use crate::util::fuzzy::fuzzy_match;
use serde::Serialize;

/// 搜索结果项
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    /// 别名
    pub alias: String,
    /// 路径或 URL
    pub path: String,
    /// 类型标签（app / url / script / editor / browser / vpn）
    pub kind: String,
}

/// 搜索别名（模糊匹配），返回匹配结果列表
pub fn search_aliases(query: &str, config: &YamlConfig) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // 搜索 PATH section
    if let Some(section_map) = config.get_section(section::PATH) {
        for (alias, path) in section_map {
            if query.is_empty() || fuzzy_match(alias, query) || fuzzy_match(path, query) {
                let kind = determine_kind(alias, config);
                results.push(SearchResult {
                    alias: alias.clone(),
                    path: path.clone(),
                    kind,
                });
            }
        }
    }

    // 搜索 INNER_URL section
    if let Some(section_map) = config.get_section(section::INNER_URL) {
        for (alias, url) in section_map {
            if query.is_empty() || fuzzy_match(alias, query) || fuzzy_match(url, query) {
                results.push(SearchResult {
                    alias: alias.clone(),
                    path: url.clone(),
                    kind: "url".to_string(),
                });
            }
        }
    }

    // 搜索 OUTER_URL section
    if let Some(section_map) = config.get_section(section::OUTER_URL) {
        for (alias, url) in section_map {
            if query.is_empty() || fuzzy_match(alias, query) || fuzzy_match(url, query) {
                results.push(SearchResult {
                    alias: alias.clone(),
                    path: url.clone(),
                    kind: "outer_url".to_string(),
                });
            }
        }
    }

    results
}

/// 根据别名在配置中的分类确定类型
fn determine_kind(alias: &str, config: &YamlConfig) -> String {
    if config.contains(section::BROWSER, alias) {
        "browser".to_string()
    } else if config.contains(section::EDITOR, alias) {
        "editor".to_string()
    } else if config.contains(section::VPN, alias) {
        "vpn".to_string()
    } else if config.contains(section::SCRIPT, alias) {
        "script".to_string()
    } else {
        "app".to_string()
    }
}
