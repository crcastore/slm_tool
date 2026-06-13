use crate::{
    chunking::{chunk_text, kind_for_path, ChunkKind},
    DocsError,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Schema, STRING, STORED, TEXT, Value},
    Index, IndexWriter, TantivyDocument,
};

/// A single docs search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsSearchResult {
    pub path: String,
    pub kind: String,
    pub start_line: u64,
    pub end_line: u64,
    pub snippet: String,
    pub score: f32,
}

/// Tantivy-backed documentation search index.
///
/// Maintains separate logical collections for docs, code, tests, examples,
/// architecture, and API files by including a `kind` field in the schema.
pub struct DocsIndex {
    index: Index,
    schema: Schema,
}

impl DocsIndex {
    pub fn open(index_dir: impl AsRef<Path>) -> Result<Self, DocsError> {
        let dir = index_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let (index, schema) = Self::build_index(
            tantivy::directory::MmapDirectory::open(dir)
                .map_err(|e| DocsError::Index(e.to_string()))?,
        )?;
        Ok(Self { index, schema })
    }

    pub fn open_in_memory() -> Result<Self, DocsError> {
        let (index, schema) = Self::build_index(tantivy::directory::RamDirectory::create())?;
        Ok(Self { index, schema })
    }

    fn build_index<D: tantivy::Directory>(dir: D) -> Result<(Index, Schema), DocsError> {
        let mut builder = Schema::builder();
        builder.add_text_field("path", STRING | STORED);
        builder.add_text_field("kind", STRING | STORED);
        builder.add_u64_field("start_line", tantivy::schema::INDEXED | STORED);
        builder.add_u64_field("end_line", tantivy::schema::INDEXED | STORED);
        builder.add_text_field("content", TEXT | STORED);
        let schema = builder.build();
        let index = Index::open_or_create(dir, schema.clone())
            .map_err(|e| DocsError::Index(e.to_string()))?;
        Ok((index, schema))
    }

    pub fn writer(&self) -> Result<IndexWriter, DocsError> {
        Ok(self.index.writer(50_000_000)?)
    }

    /// Index a workspace, processing only documentation-relevant files.
    pub fn index_workspace(
        &self,
        workspace_root: impl AsRef<Path>,
    ) -> Result<u64, DocsError> {
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
            let rel_path = abs_path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| abs_path.to_string_lossy().to_string());

            // Only index human-readable files.
            if !should_index_for_docs(&rel_path) {
                continue;
            }

            let content = match std::fs::read_to_string(abs_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let chunks = chunk_text(&rel_path, &content, 60, 10);
            for chunk in chunks {
                self.add_chunk_internal(&mut writer, &chunk)?;
                total += 1;
            }
        }

        writer.commit()?;
        Ok(total)
    }

    fn add_chunk_internal(
        &self,
        writer: &mut IndexWriter,
        chunk: &crate::chunking::Chunk,
    ) -> Result<(), DocsError> {
        let path_field = self.schema.get_field("path").unwrap();
        let kind_field = self.schema.get_field("kind").unwrap();
        let start_field = self.schema.get_field("start_line").unwrap();
        let end_field = self.schema.get_field("end_line").unwrap();
        let content_field = self.schema.get_field("content").unwrap();

        writer.add_document(doc!(
            path_field => chunk.path.clone(),
            kind_field => chunk.kind.to_string(),
            start_field => chunk.start_line,
            end_field => chunk.end_line,
            content_field => chunk.content.clone(),
        ))?;
        Ok(())
    }

    /// Add a single chunk to the index.
    pub fn add_chunk(
        &self,
        writer: &mut IndexWriter,
        path: &str,
        kind: &ChunkKind,
        start_line: u64,
        end_line: u64,
        content: &str,
    ) -> Result<(), DocsError> {
        let path_field = self.schema.get_field("path").unwrap();
        let kind_field = self.schema.get_field("kind").unwrap();
        let start_field = self.schema.get_field("start_line").unwrap();
        let end_field = self.schema.get_field("end_line").unwrap();
        let content_field = self.schema.get_field("content").unwrap();

        writer.add_document(doc!(
            path_field => path,
            kind_field => kind.to_string(),
            start_field => start_line,
            end_field => end_line,
            content_field => content,
        ))?;
        Ok(())
    }

    /// Search the docs index for `query`, optionally filtering by `kind`.
    pub fn search(
        &self,
        query: &str,
        kind_filter: Option<&ChunkKind>,
        limit: usize,
    ) -> Result<Vec<DocsSearchResult>, DocsError> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let content_field = self.schema.get_field("content").unwrap();
        let path_field = self.schema.get_field("path").unwrap();
        let kind_field = self.schema.get_field("kind").unwrap();
        let start_field = self.schema.get_field("start_line").unwrap();
        let end_field = self.schema.get_field("end_line").unwrap();

        let query_parser = QueryParser::for_index(&self.index, vec![content_field]);
        let parsed = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&parsed, &TopDocs::with_limit(limit * 2).order_by_score())?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            let kind_str = doc
                .get_first(kind_field)
                .and_then(|v| v.as_str())
                .unwrap_or("code")
                .to_string();

            // Apply kind filter if provided.
            if let Some(kf) = kind_filter {
                if kind_str != kf.to_string() {
                    continue;
                }
            }

            let path = doc
                .get_first(path_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let start_line = doc.get_first(start_field).and_then(|v| v.as_u64()).unwrap_or(0);
            let end_line = doc.get_first(end_field).and_then(|v| v.as_u64()).unwrap_or(0);
            let snippet = doc
                .get_first(content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(DocsSearchResult {
                path,
                kind: kind_str,
                start_line,
                end_line,
                snippet,
                score,
            });

            if results.len() >= limit {
                break;
            }
        }

        Ok(results)
    }
}

/// Returns true if this file should be indexed for documentation search.
fn should_index_for_docs(path: &str) -> bool {
    let lower = path.to_lowercase();
    // Always index markdown, text, and documentation files.
    if lower.ends_with(".md")
        || lower.ends_with(".rst")
        || lower.ends_with(".txt")
        || lower.ends_with(".adoc")
    {
        return true;
    }
    // Index YAML/JSON that look like API specs.
    if (lower.ends_with(".yaml") || lower.ends_with(".yml") || lower.ends_with(".json"))
        && (lower.contains("openapi")
            || lower.contains("swagger")
            || lower.contains("api")
            || lower.contains("schema"))
    {
        return true;
    }
    // Index source files for example and test search.
    if lower.ends_with(".rs")
        || lower.ends_with(".py")
        || lower.ends_with(".ts")
        || lower.ends_with(".js")
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_docs_search() {
        let idx = DocsIndex::open_in_memory().unwrap();
        let mut writer = idx.writer().unwrap();
        idx.add_chunk(
            &mut writer,
            "README.md",
            &ChunkKind::Docs,
            1,
            10,
            "This project uses JWT for authentication and RBAC for authorization.",
        )
        .unwrap();
        writer.commit().unwrap();

        let results = idx.search("authentication", None, 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "README.md");
    }
}
