use crate::GitError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Blame information for a single line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    pub line: usize,
    pub content: String,
    pub commit_hash: String,
    pub short_hash: String,
    pub author: String,
    pub timestamp: i64,
}

/// Blame information for a whole file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBlame {
    pub path: String,
    pub lines: Vec<BlameLine>,
}

/// Return git blame for `file_path` relative to `repo_path`.
pub fn file_blame(
    repo_path: impl AsRef<Path>,
    file_path: impl AsRef<Path>,
) -> Result<FileBlame, GitError> {
    let repo = crate::open_repo(&repo_path)?;
    let file_str = file_path.as_ref().to_string_lossy().to_string();

    let blame = repo.blame_file(file_path.as_ref(), None)?;

    // Read the actual file contents to pair with blame info.
    let abs_path = repo_path.as_ref().join(file_path.as_ref());
    let content = std::fs::read_to_string(&abs_path)?;
    let file_lines: Vec<&str> = content.lines().collect();

    let mut lines: Vec<BlameLine> = Vec::new();

    for (line_idx, file_line) in file_lines.iter().enumerate() {
        let line_number = line_idx + 1;
        if let Some(hunk) = blame.get_line(line_number) {
            let sig = hunk.orig_signature();
            let oid = hunk.orig_commit_id();
            let author = sig
                .as_ref()
                .and_then(|s| s.name().ok())
                .unwrap_or("unknown")
                .to_string();
            let timestamp = sig.as_ref().map(|s| s.when().seconds()).unwrap_or(0);
            lines.push(BlameLine {
                line: line_number,
                content: file_line.to_string(),
                commit_hash: oid.to_string(),
                short_hash: oid.to_string()[..7].to_string(),
                author,
                timestamp,
            });
        }
    }

    Ok(FileBlame {
        path: file_str,
        lines,
    })
}
