use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashMap;

/// 图片渲染状态
pub enum ImageState {
    /// 等待加载
    #[allow(dead_code)]
    Pending,
    /// 加载中
    Loading,
    /// 已加载，持有 protocol 对象
    Ready(StatefulProtocol),
    /// 加载失败
    Failed(String),
}

/// 全局图片缓存（按 URL/路径 去重）
pub struct ImageCache {
    pub picker: Option<Picker>,
    pub images: HashMap<String, ImageState>,
}

/// 检测当前终端是否可能支持终端图形协议（Kitty / iTerm2 / Sixel）。
/// 对于已知不支持的终端（macOS Terminal.app 等）直接返回 false，
/// 避免 `from_query_stdio` 向备用屏幕写入查询序列导致界面乱码。
fn terminal_supports_graphics() -> bool {
    // Kitty 终端：环境变量 KITTY_WINDOW_ID 存在
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }
    match std::env::var("TERM_PROGRAM").as_deref() {
        // macOS Terminal.app 明确不支持
        Ok("Apple_Terminal") => false,
        // iTerm2 / WezTerm / Ghostty 支持
        Ok("iTerm.app") | Ok("WezTerm") | Ok("ghostty") => true,
        // 其他：凭 COLORTERM 判断（truecolor 终端通常也支持图形协议）
        _ => matches!(
            std::env::var("COLORTERM").as_deref(),
            Ok("truecolor") | Ok("24bit")
        ),
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCache {
    pub fn new() -> Self {
        let picker = if terminal_supports_graphics() {
            Picker::from_query_stdio().ok()
        } else {
            None
        };
        Self {
            picker,
            images: HashMap::new(),
        }
    }
}
