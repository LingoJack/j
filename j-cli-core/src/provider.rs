//! 供 j-cli-core 使用的 trait 抽象
//!
//! j-cli-core 不直接依赖 j-cli 的具体类型（YamlConfig、Assets、Theme），
//! 而是通过 trait 由调用方注入。Tauri 和 CLI 各自提供自己的实现。

use std::path::PathBuf;

/// 配置提供者 trait — 抽象 YamlConfig
pub trait ConfigProvider: Send + Sync {
    /// 获取数据根目录 (~/.jdata/)
    fn data_dir(&self) -> PathBuf;

    /// 获取指定 section 的配置项
    fn get(&self, section: &str, key: &str) -> Option<String>;

    /// 获取 VPN 配置项
    fn vpn_config(&self) -> Option<String>;
}

/// 资源提供者 trait — 抽象 Assets (rust-embed)
pub trait AssetProvider: Send + Sync {
    /// 获取内置资源文件内容
    fn get_asset(&self, path: &str) -> Option<Vec<u8>>;
}
