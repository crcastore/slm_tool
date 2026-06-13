use crate::SearchError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Schema, TEXT, STORED, STRING, Value},
    Index, IndexWriter, TantivyDocument,
};

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub start_line: u64,
    pub end_line: u64,
    pub snippet: String,
    pub score: f32,
}

/// Manages a Tantivy full-text search index for workspace code.
pub struct CodeIndex {
    index: Index,
    schema: Schema,
}

impl CodeIndex {
    /// Open or create an index in `index_dir`.
    pub fn open(index_dir: impl AsRef<Path>) -> Result<Self, SearchError> {
        let dir = index_dir.as_ref();
        std::fs::create_dir_all(dir)?;

        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("path", STRING | STORED);
        schema_builder.add_u64_field("start_line", tantivy::schema::INDEXED | STORED);
        schema_builder.add_u64_field("end_line", tantivy::schema::INDEXED | STORED);
        schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();

        let index = if dir.join("meta.json").exists() {
            Index::open_in_dir(dir).map_err(|e| SearchError::OpenDirectory(e.to_string()))?
        } else {
            Index::create_in_dir(dir, schema.clone())
                .map_err(|e| SearchError::OpenDirectory(e.to_string()))?
        };

        Ok(Self { index, schema })
    }

    /// Create an in-memory index (useful for tests).
    pub fn open_in_memory() -> Result<Self, SearchError> {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("path", STRING | STORED);
        schema_builder.add_u64_field("start_line", tantivy::schema::INDEXED | STORED);
        schema_builder.add_u64_field("end_line", tantivy::schema::INDEXED | STORED);
        schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema.clone());
        Ok(Self { index, schema })
    }

    /// Index a block of code from a file.
    ///
    /// `start_line` and `end_line` are 1-based inclusive line numbers.
    pub fn add_chunk(
        &self,
        writer: &mut IndexWriter,
        path: &str,
        start_line: u64,
        end_line: u64,
        content: &str,
    ) -> Result<(), SearchError> {
        let path_field = self.schema.get_field("path").unwrap();
        let start_field = self.schema.get_field("start_line").unwrap();
        let end_field = self.schema.get_field("end_line").unwrap();
        let content_field = self.schema.get_field("content").unwrap();

        writer.add_document(doc!(
            path_field => path,
            start_field => start_line,
            end_field => end_line,
            content_field => content,
        ))?;
        Ok(())
    }

    /// Commit pending writes.
    pub fn commit(writer: &mut IndexWriter) -> Result<(), SearchError> {
        writer.commit()?;
        Ok(())
    }

    /// Create a new `IndexWriter` with a reasonable heap size.
    pub fn writer(&self) -> Result<IndexWriter, SearchError> {
        Ok(self.index.writer(50_000_000)?)
    }

    /// Search for `query` and return up to `limit` results.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let content_field = self.schema.get_field("content").unwrap();
        let path_field = self.schema.get_field("path").unwrap();
        let start_field = self.schema.get_field("start_line").unwrap();
        let end_field = self.schema.get_field("end_line").unwrap();

        let query_parser = QueryParser::for_index(&self.index, vec![content_field]);
        let parsed = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&parsed, &TopDocs::with_limit(limit).order_by_score())?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            let path = doc
                .get_first(path_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let start_line = doc
                .get_first(start_field)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let end_line = doc
                .get_first(end_field)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let snippet = doc
                .get_first(content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            results.push(SearchResult {
                path,
                start_line,
                end_line,
                snippet,
                score,
            });
        }

        Ok(results)
    }

    /// Index all files in a workspace directory, chunking each file into
    /// blocks of up to `chunk_lines` lines.
    pub fn index_workspace(
        &self,
        workspace_root: impl AsRef<Path>,
        chunk_lines: usize,
    ) -> Result<u64, SearchError> {
        use ignore::WalkBuilder;

        let root = workspace_root.as_ref();
        let mut writer = self.writer()?;
        let mut total = 0u64;

        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }

            let abs_path = entry.path();
            // Skip files larger than 1 MB.
            if std::fs::metadata(abs_path)
                .map(|m| m.len() > 1_024 * 1_024)
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

            let lines: Vec<&str> = content.lines().collect();
            let mut chunk_start = 0;
            while chunk_start < lines.len() {
                let chunk_end = (chunk_start + chunk_lines).min(lines.len());
                let chunk_content = lines[chunk_start..chunk_end].join("\n");
                self.add_chunk(
                    &mut writer,
                    &rel_path,
                    (chunk_start + 1) as u64,
                    chunk_end as u64,
                    &chunk_content,
                )?;
                chunk_start = chunk_end;
                total += 1;
            }
        }

        Self::commit(&mut writer)?;
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_index_and_search() {
        let index = CodeIndex::open_in_memory().unwrap();
        let mut writer = index.writer().unwrap();
        index
            .add_chunk(&mut writer, "src/auth.rs", 1, 10, "fn create_session(user: &str) -> Session { todo!() }")
            .unwrap();
        CodeIndex::commit(&mut writer).unwrap();

        let results = index.search("create_session", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "src/auth.rs");
    }
}
