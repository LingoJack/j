//! 日报文件读写辅助函数。

use crate::config::YamlConfig;
use crate::constants::REPORT_DATE_FORMAT;
use crate::error;
use crate::info;
use chrono::NaiveDate;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(super) const DATE_FORMAT: &str = REPORT_DATE_FORMAT;

/// 获取日报文件路径（统一入口，自动创建目录和文件）
pub fn get_report_path(config: &YamlConfig) -> Option<String> {
    let report_path = config.report_file_path();

    // 确保父目录存在
    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // 如果文件不存在则自动创建空文件
    if !report_path.exists() {
        if let Err(e) = fs::write(&report_path, "") {
            error!("创建日报文件失败: {}", e);
            return None;
        }
        info!("已自动创建日报文件: {:?}", report_path);
    }

    Some(report_path.to_string_lossy().to_string())
}

/// 获取日报文件路径（静默版本，不输出 info）
pub fn get_report_path_silent(config: &YamlConfig) -> Option<String> {
    let report_path = config.report_file_path();

    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if !report_path.exists() && fs::write(&report_path, "").is_err() {
        return None;
    }

    Some(report_path.to_string_lossy().to_string())
}

/// 获取日报工作目录下的 settings.json 路径
pub fn get_settings_json_path(report_path: &str) -> Option<std::path::PathBuf> {
    Path::new(report_path)
        .parent()
        .map(|p| p.join("settings.json"))
}

/// 获取日报目录（report 文件所在的目录）
pub fn get_report_dir(config: &YamlConfig) -> Option<String> {
    let report_path = config.report_file_path();
    report_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
}

/// 解析日期字符串
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, DATE_FORMAT).ok()
}

/// 追加内容到文件末尾（确保文件末尾有换行符）
pub fn append_to_file(path: &Path, content: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;

    match OpenOptions::new().read(true).append(true).open(path) {
        Ok(mut f) => {
            // 确保文件末尾有换行符，防止内容拼到上一行末尾
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            if len > 0 {
                let _ = f.seek(SeekFrom::Start(len - 1));
                let mut last_byte = [0u8; 1];
                if f.read_exact(&mut last_byte).is_ok() && last_byte[0] != b'\n' {
                    let _ = f.write_all(b"\n");
                }
                // seek 回末尾继续追加
                let _ = f.seek(SeekFrom::End(0));
            }
            if let Err(e) = f.write_all(content.as_bytes()) {
                error!("写入文件失败: {}", e);
            }
        }
        Err(e) => error!("打开文件失败: {}", e),
    }
}

/// 替换文件最后 N 行为新内容
pub fn replace_last_n_lines(path: &Path, n: usize, new_content: &str) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            error!("读取文件失败: {}", e);
            return;
        }
    };

    let all_lines: Vec<&str> = content.lines().collect();

    // 保留前面的行（去掉最后 n 行）
    let keep_count = if all_lines.len() > n {
        all_lines.len() - n
    } else {
        0
    };

    let mut result = String::new();

    // 写入保留的行
    for line in &all_lines[..keep_count] {
        result.push_str(line);
        result.push('\n');
    }

    // 追加编辑器的内容
    result.push_str(new_content);

    // 确保文件以换行结尾
    if !result.ends_with('\n') {
        result.push('\n');
    }

    if let Err(e) = fs::write(path, &result) {
        error!("写入文件失败: {}", e);
    }
}

/// 从文件尾部读取最后 N 个"条目"对应的行。
///
/// 日报/周报通常是 Markdown 列表，一个 block 动辄包含数十个条目。
/// 按顶层 block 截取粒度太粗（`c 1` 会输出整个列表），因此这里按
/// **列表项**（以 `- `、`* `、`+ `、`数字.` 开头的行）计数。
///
/// 非列表区域（标题、段落、代码块等）退化为按顶层 block 计数。
pub fn read_last_n_blocks(path: &Path, n: usize) -> Vec<String> {
    use crate::markdown::parser::parse_markdown;

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            error!("读取文件时发生错误: {}", e);
            return Vec::new();
        }
    };

    let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    if all_lines.is_empty() {
        return Vec::new();
    }

    let doc = parse_markdown(&content, usize::MAX);

    // 最后一个 block 是列表时，按列表项粒度截取
    if let Some(last_block) = doc.blocks.last()
        && let crate::markdown::ir::BlockKind::List(data) = &last_block.kind
    {
        return read_last_n_list_items(&all_lines, &doc.blocks, data, n);
    }

    // 非列表区域：退化为顶层 block 计数
    if n == usize::MAX || n >= doc.blocks.len() {
        return all_lines;
    }

    let start_block = &doc.blocks[doc.blocks.len() - n];
    let mut actual_start = start_block.source.start_line;
    while actual_start > 0 && all_lines[actual_start - 1].trim().is_empty() {
        actual_start -= 1;
    }

    all_lines[actual_start..].to_vec()
}

/// 按列表项粒度截取最后 N 个条目。
///
/// 从文件尾部向前扫描，找到第 N 个列表项的起始行作为起点。
/// 如果 N >= 列表项总数，则回退到列表 block 的 start_line（包含整个列表）。
fn read_last_n_list_items(
    all_lines: &[String],
    blocks: &[crate::markdown::ir::Block],
    list_data: &crate::markdown::ir::ListData,
    n: usize,
) -> Vec<String> {
    let total_items = list_data.items.len();
    if n >= total_items {
        // 要的比列表项还多，返回整个文档尾部（从列表 block 起始）
        let list_block_start = blocks.last().map(|b| b.source.start_line).unwrap_or(0);
        let mut start = list_block_start;
        while start > 0 && all_lines[start - 1].trim().is_empty() {
            start -= 1;
        }
        return all_lines[start..].to_vec();
    }

    // 找到列表所在区域，从尾部向前数第 n 个列表项
    // 列表项标记行：以 `- `、`* `、`+ `、`<数字>. ` 开头（允许前导空格用于嵌套）
    let item_start_lines = find_list_item_starts(all_lines);
    if item_start_lines.is_empty() || n > item_start_lines.len() {
        return all_lines.to_vec();
    }

    let start_idx = item_start_lines.len() - n;
    let start_line = item_start_lines[start_idx];

    all_lines[start_line..].to_vec()
}

/// 扫描所有行，返回列表项标记行的行号（0-based）。
///
/// 匹配 `- `、`* `、`+ `、`1. ` 等 Markdown 无序/有序列表标记。
fn find_list_item_starts(lines: &[String]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim_start();
            trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ ")
                || (trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
                    && trimmed.find('.').is_some_and(|dot| {
                        dot > 0 && trimmed.len() > dot + 1 && trimmed.as_bytes()[dot + 1] == b' '
                    }))
        })
        .map(|(i, _)| i)
        .collect()
}

/// 从文件尾部读取最后 N 行（高效实现，不需要读取整个文件）
pub fn read_last_n_lines(path: &Path, n: usize) -> Vec<String> {
    let buffer_size: usize = crate::constants::REPORT_READ_BUFFER_SIZE; // 16KB
    let mut lines = Vec::new();

    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            error!("读取文件时发生错误: {}", e);
            return lines;
        }
    };

    let file_len = match file.metadata() {
        Ok(m) => m.len() as usize,
        Err(_) => return lines,
    };

    if file_len == 0 {
        return lines;
    }

    // 对于较小的文件或者需要读取全部内容的情况，直接全部读取
    if n == usize::MAX || file_len <= buffer_size * 2 {
        let mut content = String::new();
        let _ = file.seek(SeekFrom::Start(0));
        if file.read_to_string(&mut content).is_ok() {
            let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            if n >= all_lines.len() {
                return all_lines;
            }
            return all_lines[all_lines.len() - n..].to_vec();
        }
        return lines;
    }

    // 从文件尾部逐块读取
    let mut pointer = file_len;
    let mut remainder = Vec::new();

    while pointer > 0 && lines.len() < n {
        let bytes_to_read = pointer.min(buffer_size);
        pointer -= bytes_to_read;

        let _ = file.seek(SeekFrom::Start(pointer as u64));
        let mut buffer = vec![0u8; bytes_to_read];
        if file.read_exact(&mut buffer).is_err() {
            break;
        }

        // 将 remainder（上次剩余的不完整行）追加到这个块的末尾
        buffer.append(&mut remainder);

        // 从后向前按行分割
        let text = String::from_utf8_lossy(&buffer).to_string();
        let mut block_lines: Vec<&str> = text.split('\n').collect();

        // 第一行可能是不完整的（跨块）
        if pointer > 0 {
            remainder = block_lines.remove(0).as_bytes().to_vec();
        }

        for line in block_lines.into_iter().rev() {
            if !line.is_empty() {
                lines.push(line.to_string());
                if lines.len() >= n {
                    break;
                }
            }
        }
    }

    // 处理文件最开头的那行
    if !remainder.is_empty() && lines.len() < n {
        let line = String::from_utf8_lossy(&remainder).to_string();
        if !line.is_empty() {
            lines.push(line);
        }
    }

    lines.reverse();
    lines
}
