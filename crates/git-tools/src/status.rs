use crate::GitError;
use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Status of a single file in the working tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
}

/// Summary of the entire workspace status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    pub branch: Option<String>,
    pub files: Vec<FileStatus>,
    pub is_clean: bool,
}

/// Return the current working tree status.
pub fn workspace_status(repo_path: impl AsRef<Path>) -> Result<WorkspaceStatus, GitError> {
    let repo = crate::open_repo(repo_path)?;
    git_status(&repo)
}

pub fn git_status(repo: &Repository) -> Result<WorkspaceStatus, GitError> {
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().ok().map(|s| s.to_string()));

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut files = Vec::new();

    for entry in statuses.iter() {
        let path = entry
            .path()
            .unwrap_or("<invalid utf-8>")
            .to_string();
        let status = format_status(entry.status());
        files.push(FileStatus { path, status });
    }

    let is_clean = files.is_empty();
    Ok(WorkspaceStatus {
        branch,
        files,
        is_clean,
    })
}

fn format_status(s: Status) -> String {
    let mut parts = Vec::new();
    if s.contains(Status::INDEX_NEW) {
        parts.push("new file (staged)");
    }
    if s.contains(Status::INDEX_MODIFIED) {
        parts.push("modified (staged)");
    }
    if s.contains(Status::INDEX_DELETED) {
        parts.push("deleted (staged)");
    }
    if s.contains(Status::INDEX_RENAMED) {
        parts.push("renamed (staged)");
    }
    if s.contains(Status::WT_NEW) {
        parts.push("untracked");
    }
    if s.contains(Status::WT_MODIFIED) {
        parts.push("modified");
    }
    if s.contains(Status::WT_DELETED) {
        parts.push("deleted");
    }
    if s.contains(Status::CONFLICTED) {
        parts.push("conflicted");
    }
    if parts.is_empty() {
        "unknown".to_string()
    } else {
        parts.join(", ")
    }
}
