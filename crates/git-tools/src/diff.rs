use crate::GitError;
use git2::{DiffOptions, Repository};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Diff of a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub hunks: Vec<String>,
    pub additions: usize,
    pub deletions: usize,
}

/// Diff of the entire workspace (working tree vs HEAD).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDiff {
    pub files: Vec<FileDiff>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

/// Return the diff between the working tree and HEAD.
pub fn workspace_diff(repo_path: impl AsRef<Path>) -> Result<WorkspaceDiff, GitError> {
    let repo = crate::open_repo(repo_path)?;
    git_diff(&repo, None)
}

/// Return the diff for a single file between working tree and HEAD.
pub fn file_diff(
    repo_path: impl AsRef<Path>,
    file_path: impl AsRef<Path>,
) -> Result<Option<FileDiff>, GitError> {
    let repo = crate::open_repo(repo_path)?;
    let file_str = file_path.as_ref().to_string_lossy().to_string();
    let diff = git_diff(&repo, Some(&file_str))?;
    Ok(diff.files.into_iter().find(|f| f.path == file_str))
}

pub fn git_diff(repo: &Repository, path_filter: Option<&str>) -> Result<WorkspaceDiff, GitError> {
    let mut opts = DiffOptions::new();
    if let Some(p) = path_filter {
        opts.pathspec(p);
    }

    let head_tree = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_tree().ok());

    let diff = repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))?;

    let mut files: Vec<FileDiff> = Vec::new();
    let mut current_file: Option<FileDiff> = None;

    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        use git2::DiffLineType::*;

        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // Flush previous file if we moved to a new one.
        if current_file
            .as_ref()
            .map(|f| f.path != path)
            .unwrap_or(true)
        {
            if let Some(f) = current_file.take() {
                files.push(f);
            }
            current_file = Some(FileDiff {
                path: path.clone(),
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
            });
        }

        if let Some(ref mut f) = current_file {
            let content = String::from_utf8_lossy(line.content()).to_string();
            match line.origin_value() {
                Addition => {
                    f.additions += 1;
                    f.hunks.push(format!("+{content}"));
                }
                Deletion => {
                    f.deletions += 1;
                    f.hunks.push(format!("-{content}"));
                }
                Context => {
                    f.hunks.push(format!(" {content}"));
                }
                _ => {}
            }
        }

        true
    })?;

    if let Some(f) = current_file {
        files.push(f);
    }

    let total_additions = files.iter().map(|f| f.additions).sum();
    let total_deletions = files.iter().map(|f| f.deletions).sum();

    Ok(WorkspaceDiff {
        files,
        total_additions,
        total_deletions,
    })
}
