use crate::{
    symbols::{Symbol, SymbolKind},
    SymbolError,
};
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

/// Multi-language symbol parser using Tree-sitter.
pub struct SymbolParser {
    // Parsers are not Clone/Sync, so we create one per call.
}

impl SymbolParser {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse a source file and extract all top-level symbols.
    pub fn parse_file(
        &self,
        path: impl AsRef<Path>,
        rel_path: &str,
    ) -> Result<Vec<Symbol>, SymbolError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let lang = detect_tree_sitter_language(path);
        let Some(ts_lang) = lang else {
            return Ok(vec![]);
        };
        self.parse_content(&content, rel_path, ts_lang, path)
    }

    /// Parse `content` with the given tree-sitter language.
    fn parse_content(
        &self,
        content: &str,
        rel_path: &str,
        language: Language,
        path: &Path,
    ) -> Result<Vec<Symbol>, SymbolError> {
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|_| SymbolError::ParseError(rel_path.to_string()))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| SymbolError::ParseError(rel_path.to_string()))?;

        let root = tree.root_node();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let symbols = match ext {
            "rs" => extract_rust_symbols(root, content, rel_path),
            "py" => extract_python_symbols(root, content, rel_path),
            "js" | "jsx" => extract_js_symbols(root, content, rel_path),
            "ts" | "tsx" => extract_ts_symbols(root, content, rel_path),
            _ => vec![],
        };

        Ok(symbols)
    }
}

impl Default for SymbolParser {
    fn default() -> Self {
        Self::new()
    }
}

fn detect_tree_sitter_language(path: &Path) -> Option<Language> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Some(tree_sitter_rust::LANGUAGE.into()),
        Some("py") => Some(tree_sitter_python::LANGUAGE.into()),
        Some("js" | "jsx") => Some(tree_sitter_javascript::LANGUAGE.into()),
        Some("ts") => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        Some("tsx") => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => None,
    }
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

fn node_start_line(node: Node) -> u64 {
    node.start_position().row as u64 + 1
}

fn node_end_line(node: Node) -> u64 {
    node.end_position().row as u64 + 1
}

fn is_test_fn_rust(node: Node, source: &str) -> bool {
    // Check previous siblings for #[test] attribute.
    let parent = match node.parent() {
        Some(p) => p,
        None => return false,
    };
    let mut cursor = parent.walk();
    for child in parent.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            let text = node_text(child, source);
            if text.contains("#[test]") || text.contains("#[tokio::test]") {
                return true;
            }
        }
    }
    false
}

fn extract_rust_symbols(root: Node, source: &str, path: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    let mut cursor = root.walk();

    fn visit(node: Node, source: &str, path: &str, symbols: &mut Vec<Symbol>) {
        match node.kind() {
            "function_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(name_node, source).to_string();
                    let kind = if is_test_fn_rust(node, source) {
                        SymbolKind::Test
                    } else {
                        SymbolKind::Function
                    };
                    symbols.push(Symbol {
                        name,
                        kind,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "impl_item" => {
                // Recurse into impl blocks to find methods.
                let mut c = node.walk();
                for child in node.children(&mut c) {
                    if child.kind() == "declaration_list" {
                        let mut c2 = child.walk();
                        for item in child.children(&mut c2) {
                            if item.kind() == "function_item" {
                                if let Some(nn) = item.child_by_field_name("name") {
                                    let name = node_text(nn, source).to_string();
                                    let kind = if is_test_fn_rust(item, source) {
                                        SymbolKind::Test
                                    } else {
                                        SymbolKind::Method
                                    };
                                    symbols.push(Symbol {
                                        name,
                                        kind,
                                        path: path.to_string(),
                                        line: node_start_line(item),
                                        end_line: node_end_line(item),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "struct_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Struct,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "enum_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Enum,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "trait_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Trait,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "mod_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Module,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "type_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::TypeAlias,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            _ => {
                let mut c = node.walk();
                for child in node.children(&mut c) {
                    visit(child, source, path, symbols);
                }
            }
        }
    }

    visit(root, source, path, &mut symbols);
    symbols
}

fn extract_python_symbols(root: Node, source: &str, path: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    fn visit(node: Node, source: &str, path: &str, symbols: &mut Vec<Symbol>) {
        match node.kind() {
            "function_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(name_node, source).to_string();
                    let kind = if name.starts_with("test_") {
                        SymbolKind::Test
                    } else {
                        SymbolKind::Function
                    };
                    symbols.push(Symbol {
                        name,
                        kind,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "class_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Class,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                    // Recurse into class body for methods.
                    let mut c = node.walk();
                    for child in node.children(&mut c) {
                        visit(child, source, path, symbols);
                    }
                    return;
                }
            }
            _ => {}
        }
        let mut c = node.walk();
        for child in node.children(&mut c) {
            visit(child, source, path, symbols);
        }
    }

    visit(root, source, path, &mut symbols);
    symbols
}

fn extract_js_symbols(root: Node, source: &str, path: &str) -> Vec<Symbol> {
    extract_js_ts_symbols(root, source, path)
}

fn extract_ts_symbols(root: Node, source: &str, path: &str) -> Vec<Symbol> {
    extract_js_ts_symbols(root, source, path)
}

fn extract_js_ts_symbols(root: Node, source: &str, path: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    fn visit(node: Node, source: &str, path: &str, symbols: &mut Vec<Symbol>) {
        match node.kind() {
            "function_declaration" | "function" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Function,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "class_declaration" | "class" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Class,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "method_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Method,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "interface_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Interface,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "enum_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::Enum,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            "type_alias_declaration" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    symbols.push(Symbol {
                        name: node_text(name_node, source).to_string(),
                        kind: SymbolKind::TypeAlias,
                        path: path.to_string(),
                        line: node_start_line(node),
                        end_line: node_end_line(node),
                    });
                }
            }
            _ => {}
        }
        let mut c = node.walk();
        for child in node.children(&mut c) {
            visit(child, source, path, symbols);
        }
    }

    visit(root, source, path, &mut symbols);
    symbols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_symbols() {
        let source = r#"
pub fn create_session(user: &str) -> Session {
    todo!()
}

pub struct User {
    name: String,
}

pub enum Status { Active, Inactive }

pub trait Handler {
    fn handle(&self);
}
"#;
        let parser = SymbolParser::new();
        let path = std::path::Path::new("src/auth.rs");
        let lang = tree_sitter_rust::LANGUAGE.into();
        let symbols = parser
            .parse_content(source, "src/auth.rs", lang, path)
            .unwrap();
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"create_session"));
        assert!(names.contains(&"User"));
        assert!(names.contains(&"Status"));
        assert!(names.contains(&"Handler"));
    }

    #[test]
    fn test_parse_python_symbols() {
        let source = r#"
def create_user(name: str) -> User:
    pass

class UserService:
    def get_user(self, id: int):
        pass

def test_create_user():
    pass
"#;
        let parser = SymbolParser::new();
        let path = std::path::Path::new("service.py");
        let lang = tree_sitter_python::LANGUAGE.into();
        let symbols = parser
            .parse_content(source, "service.py", lang, path)
            .unwrap();
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"create_user"));
        assert!(names.contains(&"UserService"));
        // test_create_user should be marked as Test
        let test_sym = symbols.iter().find(|s| s.name == "test_create_user");
        assert!(test_sym.is_some());
        assert_eq!(test_sym.unwrap().kind, SymbolKind::Test);
    }
}
