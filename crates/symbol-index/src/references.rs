use crate::symbols::Symbol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A cross-reference entry: symbol name → files that reference it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub path: String,
    pub line: u64,
    pub context: String,
}

/// In-memory index of references (uses of a symbol name in source text).
#[derive(Debug, Default)]
pub struct ReferenceIndex {
    /// Map from symbol name to list of reference locations.
    refs: HashMap<String, Vec<Reference>>,
}

impl ReferenceIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.refs.clear();
    }

    pub fn len(&self) -> usize {
        self.refs.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    /// Index `content` for references to each symbol in `symbols`.
    ///
    /// This is a lightweight text-search approach: a reference is any line
    /// that contains the symbol name as a word (surrounded by non-word chars).
    pub fn index_file(&mut self, path: &str, content: &str, symbols: &[&Symbol]) {
        for symbol in symbols {
            let pattern = format!(r"\b{}\b", regex::escape(&symbol.name));
            if let Ok(re) = regex::Regex::new(&pattern) {
                for (line_idx, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        self.refs
                            .entry(symbol.name.to_lowercase())
                            .or_default()
                            .push(Reference {
                                path: path.to_string(),
                                line: (line_idx + 1) as u64,
                                context: line.trim().to_string(),
                            });
                    }
                }
            }
        }
    }

    /// Return all references to `name`.
    pub fn find_references(&self, name: &str) -> Vec<&Reference> {
        self.refs
            .get(&name.to_lowercase())
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Remove references from a specific file.
    pub fn remove_path(&mut self, path: &str) {
        for refs in self.refs.values_mut() {
            refs.retain(|r| r.path != path);
        }
        self.refs.retain(|_, v| !v.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{Symbol, SymbolKind};

    #[test]
    fn test_reference_indexing() {
        let mut idx = ReferenceIndex::new();
        let sym = Symbol {
            name: "create_session".to_string(),
            kind: SymbolKind::Function,
            path: "src/auth.rs".to_string(),
            line: 10,
            end_line: 20,
        };
        let content = "let s = create_session(user);\nfn other() {}";
        idx.index_file("src/handler.rs", content, &[&sym]);
        let refs = idx.find_references("create_session");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 1);
    }
}
