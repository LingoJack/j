use std::sync::OnceLock;

use crate::assets::quotes_text;

/// 全局缓存解析后的诗句列表
static QUOTES: OnceLock<Vec<String>> = OnceLock::new();

/// 解析 `assets/quotes.txt`，返回非空行列表
fn get_quotes() -> &'static Vec<String> {
    QUOTES.get_or_init(|| {
        let text = quotes_text();
        text.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    })
}

/// 返回诗句总数（用于外部随机取模）
pub fn quotes_count() -> usize {
    let q = get_quotes();
    if q.is_empty() { 1 } else { q.len() }
}

/// 按索引取一句诗句
pub fn get_quote(idx: usize) -> &'static str {
    let quotes = get_quotes();
    if quotes.is_empty() {
        return "";
    }
    &quotes[idx % quotes.len()]
}
