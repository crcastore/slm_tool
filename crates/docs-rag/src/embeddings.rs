use crate::{
    chunking::{chunk_text, Chunk, ChunkKind},
    search::should_index_for_docs,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// A text embedding vector.
pub type Embedding = Vec<f32>;

/// Errors from embedding providers and semantic indexing.
#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("embedding response did not contain a vector")]
    MissingEmbedding,
}

/// Client for Ollama's local embedding API.
#[derive(Debug, Clone)]
pub struct OllamaEmbeddingClient {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct OllamaEmbeddingRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

impl OllamaEmbeddingClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn local(model: impl Into<String>) -> Self {
        Self::new("http://localhost:11434", model)
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Request one embedding vector from Ollama.
    pub async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let value: serde_json::Value = self
            .client
            .post(url)
            .json(&OllamaEmbeddingRequest {
                model: &self.model,
                prompt: text,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        parse_ollama_embedding(value).ok_or(EmbeddingError::MissingEmbedding)
    }
}

fn parse_ollama_embedding(value: serde_json::Value) -> Option<Embedding> {
    if let Some(values) = value.get("embedding").and_then(|v| v.as_array()) {
        return values
            .iter()
            .map(|v| v.as_f64().map(|n| n as f32))
            .collect();
    }

    value
        .get("embeddings")
        .and_then(|v| v.as_array())
        .and_then(|embeddings| embeddings.first())
        .and_then(|first| first.as_array())
        .and_then(|values| {
            values
                .iter()
                .map(|v| v.as_f64().map(|n| n as f32))
                .collect()
        })
}

/// A semantic search hit produced from dense vectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub path: String,
    pub kind: String,
    pub start_line: u64,
    pub end_line: u64,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
struct EmbeddedChunk {
    chunk: Chunk,
    embedding: Embedding,
}

/// In-memory semantic index for documentation chunks.
#[derive(Debug, Default, Clone)]
pub struct SemanticDocsIndex {
    chunks: Vec<EmbeddedChunk>,
}

impl SemanticDocsIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn insert_chunk(&mut self, chunk: Chunk, embedding: Embedding) {
        self.chunks.push(EmbeddedChunk { chunk, embedding });
    }

    pub async fn index_workspace(
        &mut self,
        workspace_root: impl AsRef<Path>,
        client: &OllamaEmbeddingClient,
        max_files: Option<usize>,
    ) -> Result<u64, EmbeddingError> {
        use ignore::WalkBuilder;

        self.clear();
        let root = workspace_root.as_ref();
        let mut indexed = 0u64;
        let mut files_seen = 0usize;

        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for result in walker {
            let entry = match result {
                Ok(entry) => entry,
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

            if !should_index_for_docs(&rel_path) {
                continue;
            }

            if std::fs::metadata(abs_path)
                .map(|m| m.len() > 512 * 1024)
                .unwrap_or(true)
            {
                continue;
            }

            if let Some(limit) = max_files {
                if files_seen >= limit {
                    break;
                }
            }
            files_seen += 1;

            let content = match std::fs::read_to_string(abs_path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            for chunk in chunk_text(&rel_path, &content, 60, 10) {
                let embedding = client.embed_text(&chunk.content).await?;
                self.insert_chunk(chunk, embedding);
                indexed += 1;
            }
        }

        Ok(indexed)
    }

    pub fn search(
        &self,
        query_embedding: &[f32],
        kind_filter: Option<&ChunkKind>,
        limit: usize,
    ) -> Vec<SemanticSearchResult> {
        let mut results: Vec<SemanticSearchResult> = self
            .chunks
            .iter()
            .filter(|entry| {
                kind_filter
                    .map(|kind| entry.chunk.kind == *kind)
                    .unwrap_or(true)
            })
            .map(|entry| SemanticSearchResult {
                path: entry.chunk.path.clone(),
                kind: entry.chunk.kind.to_string(),
                start_line: entry.chunk.start_line,
                end_line: entry.chunk.end_line,
                snippet: entry.chunk.content.clone(),
                score: cosine_similarity(query_embedding, &entry.embedding),
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }
}

/// Compute the cosine similarity between two embedding vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn test_semantic_index_search() {
        let mut index = SemanticDocsIndex::new();
        index.insert_chunk(
            Chunk {
                path: "README.md".to_string(),
                kind: ChunkKind::Docs,
                start_line: 1,
                end_line: 1,
                content: "authentication docs".to_string(),
            },
            vec![1.0, 0.0],
        );
        index.insert_chunk(
            Chunk {
                path: "tests/auth.rs".to_string(),
                kind: ChunkKind::Tests,
                start_line: 1,
                end_line: 1,
                content: "test details".to_string(),
            },
            vec![0.0, 1.0],
        );

        let results = index.search(&[1.0, 0.0], Some(&ChunkKind::Docs), 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "README.md");
    }
}
