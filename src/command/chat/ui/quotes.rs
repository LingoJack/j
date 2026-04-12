use std::borrow::Cow;
use std::sync::OnceLock;

use crate::assets::quotes_text;

/// 全局缓存解析后的诗句列表
static QUOTES: OnceLock<Vec<String>> = OnceLock::new();

/// 解析 `assets/quotes.txt`，返回非空行列表
fn get_quotes() -> &'static Vec<String> {
    QUOTES.get_or_init(|| {
        let text = quotes_text();
        match text {
            Cow::Borrowed(s) => parse_lines(s),
            Cow::Owned(ref s) => parse_lines(s),
        }
    })
}

fn parse_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// 按索引取一句诗句（自动取模循环）
pub fn get_quote(index: usize) -> &'static str {
    let quotes = get_quotes();
    if quotes.is_empty() {
        return "";
    }
    &quotes[index % quotes.len()]
}

/// 诗句总数
#[allow(dead_code)]
pub fn quote_count() -> usize {
    get_quotes().len()
}
