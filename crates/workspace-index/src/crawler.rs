use crate::{
    metadata::{count_lines, detect_language, hash_file, relative_path, FileMetadata},
    IndexError, MetadataDb,
};
use ignore::WalkBuilder;
use std::path::Path;

/// Crawl `workspace_root` and index every non-ignored file into `db`.
///
/// Files that already have a matching hash are skipped (incremental update).
/// Returns the number of files indexed (new or updated).
pub fn crawl_workspace(
    workspace_root: impl AsRef<Path>,
    db: &MetadataDb,
) -> Result<u64, IndexError> {
    let root = workspace_root.as_ref();
    if !root.exists() {
        return Err(IndexError::NoWorkspace(root.to_string_lossy().to_string()));
    }

    let mut indexed = 0u64;
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Skip directories.
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        let abs_path = entry.path();
        let rel = match relative_path(root, abs_path) {
            Some(p) => p,
            None => continue,
        };
        let rel_str = rel.to_string_lossy().to_string();

        // Stat the file.
        let meta = match std::fs::metadata(abs_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size_bytes = meta.len();
        let last_modified = meta
            .modified()
            .ok()
            .and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs() as i64)
            })
            .unwrap_or(0);

        // Skip very large files (> 10 MB).
        if size_bytes > 10 * 1024 * 1024 {
            continue;
        }

        // Check if we already have an up-to-date record (by timestamp).
        if let Ok(Some(existing)) = db.get(&rel_str) {
            if existing.last_modified == last_modified && existing.size_bytes == size_bytes {
                continue;
            }
        }

        let hash = match hash_file(abs_path) {
            Ok(h) => h,
            Err(_) => continue,
        };

        // If hash matches, only update the timestamp.
        if let Ok(Some(existing)) = db.get(&rel_str) {
            if existing.hash == hash {
                continue;
            }
        }

        let language = detect_language(abs_path);
        let line_count = count_lines(abs_path).unwrap_or(0);

        let file_meta = FileMetadata {
            path: rel_str,
            language,
            size_bytes,
            last_modified,
            hash,
            line_count,
        };

        db.upsert(&file_meta)?;
        indexed += 1;
    }

    Ok(indexed)
}
