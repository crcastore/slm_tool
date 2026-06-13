pub mod chunking;
pub mod embeddings;
pub mod search;

pub use chunking::{chunk_text, Chunk, ChunkKind};
pub use search::{DocsIndex, DocsSearchResult};

use thiserror::Error;

/// Errors from the docs RAG pipeline.
#[derive(Debug, Error)]
pub enum DocsError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    #[error("Tantivy query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("index error: {0}")]
    Index(String),
}
