use crate::IndexError;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Metadata stored for each indexed file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub language: String,
    pub size_bytes: u64,
    pub last_modified: i64,
    pub hash: String,
    pub line_count: u64,
}

/// SQLite-backed file metadata database.
pub struct MetadataDb {
    conn: Connection,
}

impl MetadataDb {
    /// Open (or create) the metadata database at `db_path`.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, IndexError> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Open an in-memory database (useful for tests).
    pub fn open_in_memory() -> Result<Self, IndexError> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> Result<(), IndexError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                path          TEXT PRIMARY KEY,
                language      TEXT NOT NULL,
                size_bytes    INTEGER NOT NULL,
                last_modified INTEGER NOT NULL,
                hash          TEXT NOT NULL,
                line_count    INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_files_language ON files(language);
            CREATE INDEX IF NOT EXISTS idx_files_last_modified ON files(last_modified);",
        )?;
        Ok(())
    }

    /// Upsert file metadata.
    pub fn upsert(&self, meta: &FileMetadata) -> Result<(), IndexError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO files
                (path, language, size_bytes, last_modified, hash, line_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                meta.path,
                meta.language,
                meta.size_bytes as i64,
                meta.last_modified,
                meta.hash,
                meta.line_count as i64,
            ],
        )?;
        Ok(())
    }

    /// Get metadata for a specific path.
    pub fn get(&self, path: &str) -> Result<Option<FileMetadata>, IndexError> {
        let result = self.conn.query_row(
            "SELECT path, language, size_bytes, last_modified, hash, line_count
             FROM files WHERE path = ?1",
            params![path],
            |row| {
                Ok(FileMetadata {
                    path: row.get(0)?,
                    language: row.get(1)?,
                    size_bytes: row.get::<_, i64>(2)? as u64,
                    last_modified: row.get(3)?,
                    hash: row.get(4)?,
                    line_count: row.get::<_, i64>(5)? as u64,
                })
            },
        );
        match result {
            Ok(m) => Ok(Some(m)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Return all files, optionally filtered by language.
    pub fn list(&self, language: Option<&str>) -> Result<Vec<FileMetadata>, IndexError> {
        let (query, param): (&str, Option<&str>) = match language {
            Some(lang) => (
                "SELECT path, language, size_bytes, last_modified, hash, line_count
                 FROM files WHERE language = ?1 ORDER BY path",
                Some(lang),
            ),
            None => (
                "SELECT path, language, size_bytes, last_modified, hash, line_count
                 FROM files ORDER BY path",
                None,
            ),
        };

        let mut stmt = self.conn.prepare(query)?;
        let rows = if let Some(p) = param {
            stmt.query_map(params![p], row_to_meta)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], row_to_meta)?
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    /// Remove a file record.
    pub fn remove(&self, path: &str) -> Result<(), IndexError> {
        self.conn
            .execute("DELETE FROM files WHERE path = ?1", params![path])?;
        Ok(())
    }

    /// Return the total number of indexed files.
    pub fn count(&self) -> Result<u64, IndexError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        Ok(n as u64)
    }
}

fn row_to_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileMetadata> {
    Ok(FileMetadata {
        path: row.get(0)?,
        language: row.get(1)?,
        size_bytes: row.get::<_, i64>(2)? as u64,
        last_modified: row.get(3)?,
        hash: row.get(4)?,
        line_count: row.get::<_, i64>(5)? as u64,
    })
}

/// Detect the programming language from the file extension.
pub fn detect_language(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("js") => "javascript",
        Some("ts") => "typescript",
        Some("tsx") => "typescript",
        Some("jsx") => "javascript",
        Some("go") => "go",
        Some("md") => "markdown",
        Some("toml") => "toml",
        Some("yaml" | "yml") => "yaml",
        Some("json") => "json",
        Some("sh" | "bash") => "shell",
        Some("c") => "c",
        Some("cpp" | "cc" | "cxx") => "cpp",
        Some("h" | "hpp") => "c_header",
        Some("java") => "java",
        Some("rb") => "ruby",
        Some("sql") => "sql",
        Some("html" | "htm") => "html",
        Some("css") => "css",
        Some("lock") => "lockfile",
        _ => "unknown",
    }
    .to_string()
}

/// Hash file contents using BLAKE3.
pub fn hash_file(path: impl AsRef<Path>) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

/// Count the number of lines in a file.
pub fn count_lines(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let content = std::fs::read(path)?;
    let count = content.iter().filter(|&&b| b == b'\n').count() as u64;
    Ok(count.max(1))
}

/// Build a `PathBuf` relative to `workspace_root` for storing in the DB.
pub fn relative_path(workspace_root: &Path, abs_path: &Path) -> Option<PathBuf> {
    abs_path.strip_prefix(workspace_root).ok().map(|p| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_db_round_trip() {
        let db = MetadataDb::open_in_memory().unwrap();
        let meta = FileMetadata {
            path: "src/main.rs".to_string(),
            language: "rust".to_string(),
            size_bytes: 1024,
            last_modified: 1700000000,
            hash: "abc123".to_string(),
            line_count: 42,
        };
        db.upsert(&meta).unwrap();
        let retrieved = db.get("src/main.rs").unwrap().unwrap();
        assert_eq!(retrieved.language, "rust");
        assert_eq!(retrieved.line_count, 42);
    }

    #[test]
    fn test_count() {
        let db = MetadataDb::open_in_memory().unwrap();
        assert_eq!(db.count().unwrap(), 0);
        db.upsert(&FileMetadata {
            path: "a.rs".into(),
            language: "rust".into(),
            size_bytes: 100,
            last_modified: 0,
            hash: "h1".into(),
            line_count: 5,
        })
        .unwrap();
        assert_eq!(db.count().unwrap(), 1);
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("src/main.rs"), "rust");
        assert_eq!(detect_language("app.py"), "python");
        assert_eq!(detect_language("index.ts"), "typescript");
        assert_eq!(detect_language("readme.md"), "markdown");
    }
}
