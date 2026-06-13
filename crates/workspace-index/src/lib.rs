pub mod crawler;
pub mod metadata;
pub mod watcher;

pub use metadata::{FileMetadata, MetadataDb};
pub use crawler::crawl_workspace;
pub use watcher::WorkspaceWatcher;

use thiserror::Error;

/// Errors from workspace indexing.
#[derive(Debug, Error)]
pub enum IndexError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("workspace root does not exist: {0}")]
    NoWorkspace(String),
}
