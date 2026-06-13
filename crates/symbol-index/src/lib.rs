pub mod languages;
pub mod parser;
pub mod references;
pub mod symbols;

pub use parser::SymbolParser;
pub use symbols::{Symbol, SymbolKind, SymbolIndex};
pub use references::ReferenceIndex;

use thiserror::Error;

/// Errors from symbol indexing.
#[derive(Debug, Error)]
pub enum SymbolError {
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("parse error in {0}")]
    ParseError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
