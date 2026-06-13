pub mod allowlist;
pub mod commands;
pub mod output;

pub use allowlist::ALLOWED_COMMANDS;
pub use commands::{run_command, CommandResult};

use thiserror::Error;

/// Errors from the verifier.
#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("command not allowed: {0}")]
    NotAllowed(String),

    #[error("command timed out after {0}s")]
    Timeout(u64),

    #[error("output exceeded {0} bytes")]
    OutputTooLarge(usize),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tokio error: {0}")]
    Tokio(String),
}
