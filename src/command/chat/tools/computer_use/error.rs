use std::fmt;

/// 计算机使用模块错误类型
#[derive(Debug)]
pub enum AicError {
    UnknownKey(String),
    UnknownModifier(String),
    EventCreationFailed(String),
    ScreenshotFailed(String),
    ImageEncodingFailed(String),
    IoError(std::io::Error),
    AxHelperNotFound,
    AxQueryFailed(String),
    AxParseFailed(String),
}

impl fmt::Display for AicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AicError::UnknownKey(k) => write!(f, "Unknown key: '{k}'"),
            AicError::UnknownModifier(m) => write!(f, "Unknown modifier: '{m}'"),
            AicError::EventCreationFailed(msg) => write!(f, "Failed to create event: {msg}"),
            AicError::ScreenshotFailed(msg) => write!(f, "Screenshot failed: {msg}"),
            AicError::ImageEncodingFailed(msg) => write!(f, "Image encoding failed: {msg}"),
            AicError::IoError(e) => write!(f, "IO error: {e}"),
            AicError::AxHelperNotFound => write!(
                f,
                "j-ax helper not found — make sure it is next to the j binary or in PATH"
            ),
            AicError::AxQueryFailed(msg) => write!(f, "Accessibility query failed: {msg}"),
            AicError::AxParseFailed(msg) => {
                write!(f, "Failed to parse accessibility data: {msg}")
            }
        }
    }
}

impl std::error::Error for AicError {}

impl From<std::io::Error> for AicError {
    fn from(e: std::io::Error) -> Self {
        AicError::IoError(e)
    }
}
