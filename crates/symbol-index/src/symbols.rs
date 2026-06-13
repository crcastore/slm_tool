use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The kind of a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Trait,
    Interface,
    Enum,
    EnumVariant,
    Module,
    Import,
    Export,
    TypeAlias,
    Constant,
    Variable,
    Test,
    Unknown,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Class => "class",
            SymbolKind::Struct => "struct",
            SymbolKind::Trait => "trait",
            SymbolKind::Interface => "interface",
            SymbolKind::Enum => "enum",
            SymbolKind::EnumVariant => "enum_variant",
            SymbolKind::Module => "module",
            SymbolKind::Import => "import",
            SymbolKind::Export => "export",
            SymbolKind::TypeAlias => "type_alias",
            SymbolKind::Constant => "constant",
            SymbolKind::Variable => "variable",
            SymbolKind::Test => "test",
            SymbolKind::Unknown => "unknown",
        };
        write!(f, "{s}")
    }
}

/// A symbol extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub path: String,
    pub line: u64,
    pub end_line: u64,
}

/// An in-memory index of symbols, keyed by name.
#[derive(Debug, Default)]
pub struct SymbolIndex {
    /// Map from lower-cased symbol name to list of matching symbols.
    symbols: HashMap<String, Vec<Symbol>>,
    /// All symbols stored by file path.
    by_path: HashMap<String, Vec<Symbol>>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a symbol to the index.
    pub fn insert(&mut self, symbol: Symbol) {
        self.symbols
            .entry(symbol.name.to_lowercase())
            .or_default()
            .push(symbol.clone());
        self.by_path
            .entry(symbol.path.clone())
            .or_default()
            .push(symbol);
    }

    /// Add all symbols from a file.
    pub fn insert_many(&mut self, symbols: Vec<Symbol>) {
        for s in symbols {
            self.insert(s);
        }
    }

    /// Remove all symbols from a specific file path.
    pub fn remove_path(&mut self, path: &str) {
        if let Some(file_symbols) = self.by_path.remove(path) {
            for sym in &file_symbols {
                let key = sym.name.to_lowercase();
                if let Some(list) = self.symbols.get_mut(&key) {
                    list.retain(|s| s.path != path);
                    if list.is_empty() {
                        self.symbols.remove(&key);
                    }
                }
            }
        }
    }

    /// Find symbols by exact name (case-insensitive).
    pub fn find_by_name(&self, name: &str) -> Vec<&Symbol> {
        self.symbols
            .get(&name.to_lowercase())
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Find symbols by name prefix (case-insensitive).
    pub fn find_by_prefix(&self, prefix: &str) -> Vec<&Symbol> {
        let lower = prefix.to_lowercase();
        self.symbols
            .iter()
            .filter(|(k, _)| k.starts_with(&lower))
            .flat_map(|(_, v)| v.iter())
            .collect()
    }

    /// Return all symbols in a file.
    pub fn symbols_in_file(&self, path: &str) -> Vec<&Symbol> {
        self.by_path
            .get(path)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Return the file outline: all symbols sorted by line number.
    pub fn file_outline(&self, path: &str) -> Vec<&Symbol> {
        let mut syms = self.symbols_in_file(path);
        syms.sort_by_key(|s| s.line);
        syms
    }

    /// Return all symbols, across all files.
    pub fn all_symbols(&self) -> Vec<&Symbol> {
        self.by_path.values().flatten().collect()
    }

    /// Index a workspace directory.
    pub fn index_workspace(
        &mut self,
        workspace_root: impl AsRef<Path>,
    ) -> Result<u64, crate::SymbolError> {
        use ignore::WalkBuilder;

        let root = workspace_root.as_ref();
        let mut total = 0u64;
        let parser = crate::parser::SymbolParser::new();

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

            if let Ok(symbols) = parser.parse_file(abs_path, &rel_path) {
                total += symbols.len() as u64;
                self.remove_path(&rel_path);
                self.insert_many(symbols);
            }
        }

        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_index_round_trip() {
        let mut idx = SymbolIndex::new();
        idx.insert(Symbol {
            name: "create_session".to_string(),
            kind: SymbolKind::Function,
            path: "src/auth.rs".to_string(),
            line: 42,
            end_line: 55,
        });

        let found = idx.find_by_name("create_session");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].line, 42);
    }

    #[test]
    fn test_remove_path() {
        let mut idx = SymbolIndex::new();
        idx.insert(Symbol {
            name: "foo".to_string(),
            kind: SymbolKind::Function,
            path: "src/foo.rs".to_string(),
            line: 1,
            end_line: 5,
        });
        idx.remove_path("src/foo.rs");
        assert!(idx.find_by_name("foo").is_empty());
    }
}
