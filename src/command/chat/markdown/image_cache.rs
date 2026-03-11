use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use std::collections::HashMap;

/// 图片渲染状态
#[allow(dead_code)]
pub enum ImageState {
    /// 等待加载
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

impl ImageCache {
    pub fn new() -> Self {
        let picker = Picker::from_query_stdio().ok();
        Self {
            picker,
            images: HashMap::new(),
        }
    }
}
