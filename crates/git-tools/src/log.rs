use crate::GitError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Information about a single commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub message: String,
}

/// Return recent commits for the repository (up to `limit`).
pub fn recent_commits(
    repo_path: impl AsRef<Path>,
    limit: usize,
) -> Result<Vec<CommitInfo>, GitError> {
    let repo = crate::open_repo(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commits = Vec::new();
    for oid in revwalk.take(limit) {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let author = commit.author();
        let message = commit.message().unwrap_or("").trim().to_string();
        commits.push(CommitInfo {
            hash: oid.to_string(),
            short_hash: oid.to_string()[..7].to_string(),
            author: author.name().unwrap_or("unknown").to_string(),
            email: author.email().unwrap_or("").to_string(),
            timestamp: commit.time().seconds(),
            message,
        });
    }

    Ok(commits)
}

/// Return commits that touched a specific file (up to `limit`).
pub fn file_log(
    repo_path: impl AsRef<Path>,
    file_path: impl AsRef<Path>,
    limit: usize,
) -> Result<Vec<CommitInfo>, GitError> {
    let repo = crate::open_repo(repo_path)?;
    let file_str = file_path.as_ref().to_string_lossy().to_string();

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commits = Vec::new();
    for oid in revwalk {
        if commits.len() >= limit {
            break;
        }
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        // Check if this commit touches the file.
        let touches_file = touches_file_path(&repo, &commit, &file_str);
        if touches_file {
            let author = commit.author();
            let message = commit.message().unwrap_or("").trim().to_string();
            commits.push(CommitInfo {
                hash: oid.to_string(),
                short_hash: oid.to_string()[..7].to_string(),
                author: author.name().unwrap_or("unknown").to_string(),
                email: author.email().unwrap_or("").to_string(),
                timestamp: commit.time().seconds(),
                message,
            });
        }
    }

    Ok(commits)
}

fn touches_file_path(repo: &git2::Repository, commit: &git2::Commit, file_path: &str) -> bool {
    let tree = match commit.tree() {
        Ok(t) => t,
        Err(_) => return false,
    };

    if commit.parent_count() == 0 {
        // Initial commit: check if file exists in the tree.
        return tree.get_path(Path::new(file_path)).is_ok();
    }

    for i in 0..commit.parent_count() {
        let parent = match commit.parent(i) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let parent_tree = match parent.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(file_path);

        if let Ok(diff) =
            repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diff_opts))
        {
            if diff.deltas().len() > 0 {
                return true;
            }
        }
    }

    false
}
