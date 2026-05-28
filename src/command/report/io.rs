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

/// 从文件尾部读取最后 N 行，且不切断 Markdown 块（代码块、表格、列表等）。
///
/// 在 `j check` 等需要把结果再喂给 Markdown 渲染器的场景下使用。
/// 朴素的按行截取会让起点落在围栏代码块内部，导致 ``` 配对被破坏，
/// 渲染时把两个真实代码块之间的内容也当成一段代码块包裹。
///
/// 实现：复用 `crate::markdown::parser::parse_markdown` 的 IR 与 `SourceRange`，
/// 找到任何跨越 `total - n` 边界的 block，把起点回退到该 block 的起始行，
/// 保证返回的切片里所有 block 都是完整的。返回的行数可能略大于 `n`。
pub fn read_last_n_lines_block_aware(path: &Path, n: usize) -> Vec<String> {
    use crate::markdown::ir::{Block, BlockKind, ParsedDocument};
    use crate::markdown::parser::parse_markdown;

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            error!("读取文件时发生错误: {}", e);
            return Vec::new();
        }
    };

    let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let total = all_lines.len();
    if total == 0 {
        return Vec::new();
    }
    if n == usize::MAX || n >= total {
        return all_lines;
    }

    let desired_start = total - n;

    // 解析 IR，递归收集所有 block 的源码行范围（含 BlockQuote/List 子 block）
    let doc: ParsedDocument = parse_markdown(&content, usize::MAX);

    fn walk(blocks: &[Block], out: &mut Vec<(usize, usize)>) {
        for b in blocks {
            out.push((b.source.start_line, b.source.end_line));
            match &b.kind {
                BlockKind::BlockQuote(inner) => walk(inner, out),
                BlockKind::List(data) => {
                    for item in &data.items {
                        walk(&item.children, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut ranges = Vec::new();
    walk(&doc.blocks, &mut ranges);

    // 找到任何跨越 desired_start 的 block，把起点回退到它的 start_line。
    // 反复迭代直到稳定，处理「回退后又触发另一个 block 的跨越」的级联情况。
    let mut start = desired_start;
    loop {
        let mut new_start = start;
        for (s, e) in &ranges {
            if *s < new_start && *e >= new_start {
                new_start = *s;
            }
        }
        if new_start == start {
            break;
        }
        start = new_start;
    }

    all_lines[start..].to_vec()
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
