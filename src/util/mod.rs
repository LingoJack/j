pub mod fuzzy;
pub mod html_extract;
pub mod log;
pub mod md_render;
pub mod text;

// Re-export commonly used functions for convenience
pub use text::remove_quotes;
