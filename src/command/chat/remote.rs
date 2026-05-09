pub mod bridge;
pub mod server;
pub mod setup;

// Re-export protocol and crypto from j-cli-core
pub use j_agent::crypto;
pub use j_agent::protocol;

pub use setup::start_remote_and_wait;
