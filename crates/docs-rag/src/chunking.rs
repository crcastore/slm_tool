use serde::{Deserialize, Serialize};

/// The category / collection that a chunk belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkKind {
    /// README and general documentation.
    Docs,
    /// Source code examples.
    Examples,
    /// Architecture decision records, diagrams, design docs.
    Architecture,
    /// API contracts (OpenAPI, GraphQL, etc.).
    Api,
    /// Test files.
    Tests,
    /// General code.
    Code,
}

impl std::fmt::Display for ChunkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ChunkKind::Docs => "docs",
            ChunkKind::Examples => "examples",
            ChunkKind::Architecture => "architecture",
            ChunkKind::Api => "api",
            ChunkKind::Tests => "tests",
            ChunkKind::Code => "code",
        };
        write!(f, "{s}")
    }
}

/// A single text chunk with its origin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub path: String,
    pub kind: ChunkKind,
    pub start_line: u64,
    pub end_line: u64,
    pub content: String,
}

/// Detect the appropriate `ChunkKind` from a file path.
pub fn kind_for_path(path: &str) -> ChunkKind {
    let lower = path.to_lowercase();
    if lower.contains("test") || lower.contains("spec") {
        return ChunkKind::Tests;
    }
    if lower.ends_with(".md") || lower.ends_with(".rst") || lower.ends_with(".txt") {
        if lower.contains("adr")
            || lower.contains("architecture")
            || lower.contains("design")
        {
            return ChunkKind::Architecture;
        }
        if lower.contains("readme")
            || lower.contains("contributing")
            || lower.contains("docs/")
            || lower.contains("doc/")
        {
            return ChunkKind::Docs;
        }
        return ChunkKind::Docs;
    }
    if lower.ends_with(".yaml")
        || lower.ends_with(".yml")
        || lower.ends_with(".json")
        || lower.ends_with(".graphql")
        || lower.ends_with(".gql")
    {
        if lower.contains("openapi")
            || lower.contains("swagger")
            || lower.contains("api")
            || lower.contains("schema")
        {
            return ChunkKind::Api;
        }
    }
    if lower.contains("example") || lower.contains("sample") || lower.contains("demo") {
        return ChunkKind::Examples;
    }
    ChunkKind::Code
}

/// Split `content` into overlapping chunks of approximately `chunk_size` lines
/// with `overlap` lines of overlap between adjacent chunks.
pub fn chunk_text(
    path: &str,
    content: &str,
    chunk_size: usize,
    overlap: usize,
) -> Vec<Chunk> {
    let kind = kind_for_path(path);
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return vec![];
    }

    let step = if chunk_size > overlap {
        chunk_size - overlap
    } else {
        1
    };

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < lines.len() {
        let end = (start + chunk_size).min(lines.len());
        let chunk_content = lines[start..end].join("\n");
        chunks.push(Chunk {
            path: path.to_string(),
            kind: kind.clone(),
            start_line: (start + 1) as u64,
            end_line: end as u64,
            content: chunk_content,
        });
        if end == lines.len() {
            break;
        }
        start += step;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text() {
        let content = (0..20)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_text("README.md", &content, 10, 2);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[0].end_line, 10);
        // Second chunk overlaps.
        assert_eq!(chunks[1].start_line, 9);
    }

    #[test]
    fn test_kind_detection() {
        assert_eq!(kind_for_path("README.md"), ChunkKind::Docs);
        assert_eq!(kind_for_path("docs/architecture/adr-01.md"), ChunkKind::Architecture);
        assert_eq!(kind_for_path("tests/auth_test.rs"), ChunkKind::Tests);
        assert_eq!(kind_for_path("openapi.yaml"), ChunkKind::Api);
        assert_eq!(kind_for_path("src/main.rs"), ChunkKind::Code);
    }
}
