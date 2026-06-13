// Language configuration for tree-sitter parsers.
//
// This module centralises the mapping from file extensions to tree-sitter
// `Language` objects so the rest of the crate can add new languages in one
// place.

use tree_sitter::Language;

/// Return the tree-sitter `Language` for a given file extension, or `None`
/// if the extension is not supported.
pub fn language_for_extension(ext: &str) -> Option<Language> {
    match ext {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "js" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => None,
    }
}

/// Return all supported file extensions.
pub fn supported_extensions() -> &'static [&'static str] {
    &["rs", "py", "js", "jsx", "ts", "tsx"]
}
