use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single grep match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub path: String,
    pub line: u64,
    pub content: String,
}

/// Search all workspace files for `pattern` (a regex), respecting `.gitignore`.
///
/// Returns up to `limit` matches.
pub fn grep_workspace(
    workspace_root: impl AsRef<Path>,
    pattern: &str,
    limit: usize,
) -> Result<Vec<GrepMatch>, regex::Error> {
    let re = regex::Regex::new(pattern)?;
    let root = workspace_root.as_ref();
    let mut matches = Vec::new();

    use ignore::WalkBuilder;
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .build();

    'outer: for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        let abs_path = entry.path();
        // Skip large and binary files.
        if std::fs::metadata(abs_path)
            .map(|m| m.len() > 2 * 1024 * 1024)
            .unwrap_or(true)
        {
            continue;
        }

        let content = match std::fs::read_to_string(abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let rel_path = abs_path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| abs_path.to_string_lossy().to_string());

        for (line_idx, line) in content.lines().enumerate() {
            if re.is_match(line) {
                matches.push(GrepMatch {
                    path: rel_path.clone(),
                    line: (line_idx + 1) as u64,
                    content: line.to_string(),
                });
                if matches.len() >= limit {
                    break 'outer;
                }
            }
        }
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grep_current_dir() {
        // Should find this file itself when searching for "GrepMatch".
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let results = grep_workspace(root, "GrepMatch", 50).unwrap();
        assert!(!results.is_empty());
    }
}
