/// 匹配区间，表示 content 中匹配 target 的一段 [start, end)（左闭右开）
#[derive(Debug, Clone)]
pub struct Interval {
    pub start: usize,
    pub end: usize,
}

/// 大小写不敏感的子串匹配
pub fn fuzzy_match(content: &str, target: &str) -> bool {
    content.to_lowercase().contains(&target.to_lowercase())
}

/// 获取 content 中所有匹配 target 的区间（大小写不敏感）
/// 返回的区间为 [start, end)，基于原始 content 的字节索引（保证在 char boundary 上）
pub fn get_match_intervals(content: &str, target: &str) -> Vec<Interval> {
    let mut intervals = Vec::new();

    if target.is_empty() {
        return intervals;
    }

    let content_lower = content.to_lowercase();
    let target_lower = target.to_lowercase();

    // 建立 content_lower 字节索引 -> content 字节索引的映射
    // 因为 to_lowercase 可能改变字节长度，需要通过 char 对齐
    let content_chars: Vec<(usize, char)> = content.char_indices().collect();
    let lower_chars: Vec<(usize, char)> = content_lower.char_indices().collect();

    // 在 lowercase 版本中查找所有匹配
    let mut search_from = 0;
    while let Some(pos) = content_lower[search_from..].find(&target_lower) {
        let lower_start = search_from + pos;
        let lower_end = lower_start + target_lower.len();

        // 找到 lower_start 和 lower_end 对应的 char 索引
        let char_start_idx = lower_chars
            .iter()
            .position(|(byte_idx, _)| *byte_idx == lower_start);
        let char_end_idx = lower_chars
            .iter()
            .position(|(byte_idx, _)| *byte_idx == lower_end)
            .unwrap_or(lower_chars.len());

        if let Some(char_start_idx) = char_start_idx {
            // 映射回原始 content 的字节索引
            let orig_start = content_chars[char_start_idx].0;
            let orig_end = if char_end_idx < content_chars.len() {
                content_chars[char_end_idx].0
            } else {
                content.len()
            };

            intervals.push(Interval {
                start: orig_start,
                end: orig_end,
            });
        }

        // 按字符而非字节前进，避免在多字节字符中间切割
        if let Some(char_start_idx) = char_start_idx {
            if char_start_idx + 1 < lower_chars.len() {
                search_from = lower_chars[char_start_idx + 1].0;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    intervals
}

/// 将 content 中匹配 target 的部分用 ANSI 绿色高亮
/// fuzzy = true 时使用大小写不敏感匹配，否则精确匹配
pub fn highlight_matches(content: &str, target: &str, fuzzy: bool) -> String {
    if !fuzzy {
        // 精确匹配直接替换
        return content.replace(target, &format!("\x1b[32m{}\x1b[0m", target));
    }

    let intervals = get_match_intervals(content, target);
    if intervals.is_empty() {
        return content.to_string();
    }

    let mut result = String::new();
    let mut last_end = 0;

    for interval in &intervals {
        // 添加匹配前的内容
        result.push_str(&content[last_end..interval.start]);
        // 添加高亮的匹配内容（保留原始大小写）
        result.push_str(&format!(
            "\x1b[32m{}\x1b[0m",
            &content[interval.start..interval.end]
        ));
        last_end = interval.end;
    }

    // 添加最后一段
    result.push_str(&content[last_end..]);
    result
}
