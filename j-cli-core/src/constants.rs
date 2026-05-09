//! j-cli-core 核心常量
//!
//! 与 j-cli 的 constants.rs 共享的常量定义。

/// 用户数据根目录名
pub const DATA_DIR: &str = ".jdata";

/// Agent 子目录
pub const AGENT_DIR: &str = "agent";

/// Agent 日志子目录
pub const AGENT_LOG_DIR: &str = "logs";

/// 信息日志文件名
pub const AGENT_LOG_INFO: &str = "info.log";

/// 错误日志文件名
pub const AGENT_LOG_ERROR: &str = "error.log";

/// Agent 数据子目录
pub const AGENT_DATA_DIR: &str = "data";

/// 默认数据路径环境变量
pub const DATA_PATH_ENV: &str = "J_DATA_PATH";

/// 获取数据根目录路径
pub fn data_root() -> std::path::PathBuf {
    if let Ok(p) = std::env::var(DATA_PATH_ENV) {
        std::path::PathBuf::from(p)
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(DATA_DIR)
    }
}
