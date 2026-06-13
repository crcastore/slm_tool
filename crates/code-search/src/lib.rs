pub mod grep;
pub mod ranking;
pub mod tantivy_index;

pub use grep::{grep_workspace, GrepMatch};
pub use tantivy_index::{CodeIndex, SearchResult};

use thiserror::Error;

/// Errors from search operations.
#[derive(Debug, Error)]
pub enum SearchError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    #[error("Tantivy query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("index directory error: {0}")]
    OpenDirectory(String),
}
