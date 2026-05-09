pub mod bridge;
pub mod server;
pub mod setup;

// Re-export protocol and crypto from j-cli-core
pub use j_cli_core::crypto;
pub use j_cli_core::protocol;

pub use setup::start_remote_and_wait;
