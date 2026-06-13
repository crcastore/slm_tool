pub mod blame;
pub mod diff;
pub mod log;
pub mod status;

pub use blame::{BlameLine, FileBlame};
pub use diff::{FileDiff, WorkspaceDiff};
pub use log::CommitInfo;
pub use status::{FileStatus, WorkspaceStatus};

use std::path::Path;
use thiserror::Error;

/// Errors from git operations.
#[derive(Debug, Error)]
pub enum GitError {
    #[error("git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("not a git repository: {0}")]
    NotARepository(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid UTF-8 in git output: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Open the repository at `path` (or any parent), returning a `git2::Repository`.
pub fn open_repo(path: impl AsRef<Path>) -> Result<git2::Repository, GitError> {
    let path = path.as_ref();
    git2::Repository::discover(path).map_err(|_| {
        GitError::NotARepository(path.to_string_lossy().to_string())
    })
}
