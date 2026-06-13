use std::path::{Path, PathBuf};

/// A lightweight workspace file watcher.
///
/// Uses a simple polling strategy: compares file modification times on each
/// `check()` call against an in-memory snapshot. For production use, consider
/// replacing with the `notify` crate for event-driven updates.
pub struct WorkspaceWatcher {
    workspace_root: PathBuf,
    snapshot: std::collections::HashMap<PathBuf, std::time::SystemTime>,
}

/// A file change event.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
}

impl WorkspaceWatcher {
    /// Create a watcher for `workspace_root` and take an initial snapshot.
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        let root = workspace_root.as_ref().to_path_buf();
        let snapshot = take_snapshot(&root);
        Self {
            workspace_root: root,
            snapshot,
        }
    }

    /// Check for changes since the last snapshot.
    ///
    /// Returns a list of changed files. Also refreshes the internal snapshot.
    pub fn check(&mut self) -> Vec<FileChange> {
        let current = take_snapshot(&self.workspace_root);
        let mut changes = Vec::new();

        // Find created and modified files.
        for (path, mtime) in &current {
            match self.snapshot.get(path) {
                None => changes.push(FileChange {
                    path: path.clone(),
                    kind: ChangeKind::Created,
                }),
                Some(old_mtime) if old_mtime != mtime => changes.push(FileChange {
                    path: path.clone(),
                    kind: ChangeKind::Modified,
                }),
                _ => {}
            }
        }

        // Find deleted files.
        for path in self.snapshot.keys() {
            if !current.contains_key(path) {
                changes.push(FileChange {
                    path: path.clone(),
                    kind: ChangeKind::Deleted,
                });
            }
        }

        self.snapshot = current;
        changes
    }
}

fn take_snapshot(
    root: &Path,
) -> std::collections::HashMap<PathBuf, std::time::SystemTime> {
    use ignore::WalkBuilder;
    let mut map = std::collections::HashMap::new();
    if !root.exists() {
        return map;
    }
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .build();
    for result in walker {
        if let Ok(entry) = result {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Ok(meta) = std::fs::metadata(entry.path()) {
                    if let Ok(mtime) = meta.modified() {
                        map.insert(entry.path().to_path_buf(), mtime);
                    }
                }
            }
        }
    }
    map
}
