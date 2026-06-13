use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use safety::{PathValidator, SecretScanner};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Parameters for `read_file`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileParams {
    /// Path to the file, relative to the workspace root.
    pub path: String,
    /// First line to read (1-based, inclusive). Defaults to 1.
    pub start_line: Option<u64>,
    /// Last line to read (1-based, inclusive). Defaults to 500.
    pub end_line: Option<u64>,
}

/// Parameters for `list_files`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesParams {
    /// Glob pattern relative to the workspace root.
    pub pattern: Option<String>,
    /// Maximum number of files to return. Defaults to 100.
    pub limit: Option<usize>,
}

/// Parameters for `grep_code`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GrepCodeParams {
    /// Regular expression to search for.
    pub query: String,
    /// Maximum number of matches to return. Defaults to 50.
    pub limit: Option<usize>,
}

/// Parameters for `workspace_tree`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceTreeParams {
    /// Maximum directory depth to traverse. Defaults to 3.
    pub depth: Option<usize>,
}

/// Parameters for `find_symbol`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindSymbolParams {
    /// Symbol name to search for (case-insensitive).
    pub name: String,
    /// Optional symbol kind filter (e.g. "function", "struct", "class").
    pub kind: Option<String>,
}

/// Parameters for `find_references`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindReferencesParams {
    /// Symbol name to find references for.
    pub name: String,
}

/// Parameters for `list_symbols`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListSymbolsParams {
    /// File path (relative to workspace root) to list symbols for.
    pub path: String,
}

/// Parameters for `search_code`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchCodeParams {
    /// Query string.
    pub query: String,
    /// Maximum results to return. Defaults to 10.
    pub limit: Option<usize>,
}

/// Parameters for `git_diff_file`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GitDiffFileParams {
    /// File path (relative to workspace root).
    pub path: String,
}

/// Parameters for `git_log_file`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GitLogFileParams {
    /// File path (relative to workspace root).
    pub path: String,
    /// Maximum number of commits to return. Defaults to 10.
    pub limit: Option<usize>,
}

/// Parameters for `git_blame`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GitBlameParams {
    /// File path (relative to workspace root).
    pub path: String,
    /// Start line (1-based). Defaults to 1.
    pub start_line: Option<u64>,
    /// End line (1-based). Defaults to 30.
    pub end_line: Option<u64>,
}

/// Parameters for `run_tests`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunTestsParams {
    /// Test target or filter (e.g. "auth", "tests/unit").
    pub target: Option<String>,
    /// Command override, must be in the allowlist (e.g. "cargo test", "pytest").
    pub command: Option<String>,
}

/// Parameters for `search_docs`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchDocsParams {
    /// Query string.
    pub query: String,
    /// Kind filter: "docs", "examples", "architecture", "api", "tests", "code".
    pub kind: Option<String>,
    /// Maximum results to return. Defaults to 5.
    pub limit: Option<usize>,
}

/// Parameters for `propose_patch`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProposePatchParams {
    /// Target file path (relative to workspace root).
    pub path: String,
    /// Unified diff to apply.
    pub diff: String,
}

/// Parameters for `apply_patch`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ApplyPatchParams {
    /// Target file path (relative to workspace root).
    pub path: String,
    /// Unified diff to apply.
    pub diff: String,
    /// Validate and preview without writing. Defaults to false.
    pub dry_run: Option<bool>,
}

/// Parameters for `replace_text`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReplaceTextParams {
    /// Target file path (relative to workspace root).
    pub path: String,
    /// Exact text to replace.
    pub old_text: String,
    /// Replacement text.
    pub new_text: String,
    /// Expected number of replacements. If omitted, all matches are replaced.
    pub expected_replacements: Option<usize>,
}

/// Parameters for `replace_range`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReplaceRangeParams {
    /// Target file path (relative to workspace root).
    pub path: String,
    /// First line to replace (1-based, inclusive).
    pub start_line: u64,
    /// Last line to replace (1-based, inclusive).
    pub end_line: u64,
    /// New content for the range.
    pub new_content: String,
}

/// Parameters for `create_file`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateFileParams {
    /// File path (relative to workspace root).
    pub path: String,
    /// File content.
    pub content: String,
    /// Overwrite an existing file. Defaults to false.
    pub overwrite: Option<bool>,
}

/// Parameters for `delete_file`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteFileParams {
    /// File path (relative to workspace root).
    pub path: String,
}

/// Parameters for `rename_file`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenameFileParams {
    /// Current path (relative to workspace root).
    pub from: String,
    /// New path (relative to workspace root).
    pub to: String,
    /// Overwrite the destination if it exists. Defaults to false.
    pub overwrite: Option<bool>,
}

/// Parameters for `scan_file_for_secrets`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScanFileForSecretsParams {
    /// File path (relative to workspace root).
    pub path: String,
}

/// Parameters for `scan_patch_for_secrets`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScanPatchForSecretsParams {
    /// Path associated with the patch.
    pub path: String,
    /// Unified diff to scan.
    pub diff: String,
}

/// Parameters for `validate_path`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidatePathParams {
    /// Path to validate relative to the workspace root.
    pub path: String,
}

/// Parameters for `validate_command`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ValidateCommandParams {
    /// Command string to validate against the verifier allowlist.
    pub command: String,
}

/// Parameters for `run_command`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RunCommandParams {
    /// Allowlisted verification command to run.
    pub command: String,
    /// Timeout in seconds. Defaults to 60.
    pub timeout_secs: Option<u64>,
}

/// Parameters for `embed_text`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EmbedTextParams {
    /// Text to embed.
    pub text: String,
    /// Ollama embedding model. Defaults to "nomic-embed-text".
    pub model: Option<String>,
    /// Ollama base URL. Defaults to http://localhost:11434.
    pub base_url: Option<String>,
    /// Include the raw vector in the response. Defaults to false.
    pub include_vector: Option<bool>,
}

/// Parameters for `index_semantic_docs`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct IndexSemanticDocsParams {
    /// Ollama embedding model. Defaults to "nomic-embed-text".
    pub model: Option<String>,
    /// Ollama base URL. Defaults to http://localhost:11434.
    pub base_url: Option<String>,
    /// Maximum documentation-relevant files to embed. Defaults to 200.
    pub max_files: Option<usize>,
}

/// Parameters for `search_semantic_docs`.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchSemanticDocsParams {
    /// Query string to embed and search with.
    pub query: String,
    /// Kind filter: "docs", "examples", "architecture", "api", "tests", "code".
    pub kind: Option<String>,
    /// Maximum results to return. Defaults to 5.
    pub limit: Option<usize>,
    /// Ollama embedding model. Defaults to "nomic-embed-text".
    pub model: Option<String>,
    /// Ollama base URL. Defaults to http://localhost:11434.
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct IndexState {
    symbols_indexed: u64,
    references_indexed: u64,
    code_chunks_indexed: u64,
    docs_chunks_indexed: u64,
    semantic_docs_chunks_indexed: u64,
    last_reindexed_unix: Option<u64>,
    in_memory_indexes: bool,
}

/// The main MCP server handler.
///
/// This is the central struct that exposes all coding-assistant tools to the
/// MCP client.  It is constructed with a workspace root path and optional
/// pre-built indexes.
#[derive(Clone)]
pub struct CodeAssistantServer {
    workspace_root: Arc<PathBuf>,
    validator: Arc<PathValidator>,
    tool_router: ToolRouter<Self>,
    symbol_index: Arc<tokio::sync::Mutex<symbol_index::symbols::SymbolIndex>>,
    reference_index: Arc<tokio::sync::Mutex<symbol_index::references::ReferenceIndex>>,
    docs_index: Arc<docs_rag::search::DocsIndex>,
    code_index: Arc<code_search::tantivy_index::CodeIndex>,
    semantic_docs_index: Arc<tokio::sync::Mutex<docs_rag::embeddings::SemanticDocsIndex>>,
    index_state: Arc<tokio::sync::Mutex<IndexState>>,
}

impl CodeAssistantServer {
    /// Create a new server for `workspace_root`.
    ///
    /// If `index` is true, all sub-indexes are built synchronously during
    /// construction.  In production, prefer setting `index = false` and
    /// triggering indexing asynchronously after startup.
    pub fn new(workspace_root: PathBuf, index: bool) -> anyhow::Result<Self> {
        let validator = PathValidator::new(&workspace_root)?;
        let mut sym_idx = symbol_index::symbols::SymbolIndex::new();
        let mut ref_idx = symbol_index::references::ReferenceIndex::new();
        let docs_idx = docs_rag::search::DocsIndex::open_in_memory()?;
        let code_idx = code_search::tantivy_index::CodeIndex::open_in_memory()?;
        let semantic_docs_idx = docs_rag::embeddings::SemanticDocsIndex::new();
        let mut index_state = IndexState {
            in_memory_indexes: true,
            ..IndexState::default()
        };

        if index {
            index_state.symbols_indexed = sym_idx.index_workspace(&workspace_root).unwrap_or(0);
            let references_indexed =
                rebuild_reference_index(&workspace_root, &sym_idx, &mut ref_idx);
            index_state.references_indexed = references_indexed;
            index_state.docs_chunks_indexed =
                docs_idx.index_workspace(&workspace_root).unwrap_or(0);
            index_state.code_chunks_indexed =
                code_idx.index_workspace(&workspace_root, 50).unwrap_or(0);
            index_state.last_reindexed_unix = Some(now_unix());
        }

        Ok(Self {
            workspace_root: Arc::new(workspace_root),
            validator: Arc::new(validator),
            tool_router: Self::tool_router(),
            symbol_index: Arc::new(tokio::sync::Mutex::new(sym_idx)),
            reference_index: Arc::new(tokio::sync::Mutex::new(ref_idx)),
            docs_index: Arc::new(docs_idx),
            code_index: Arc::new(code_idx),
            semantic_docs_index: Arc::new(tokio::sync::Mutex::new(semantic_docs_idx)),
            index_state: Arc::new(tokio::sync::Mutex::new(index_state)),
        })
    }

    fn ok_json(value: impl Serialize) -> CallToolResult {
        let text = serde_json::to_string_pretty(&value)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {e}\"}}"));
        CallToolResult::success(vec![Content::text(text)])
    }

    fn err_text(msg: impl std::fmt::Display) -> CallToolResult {
        CallToolResult::success(vec![Content::text(format!("Error: {msg}"))])
    }

    fn safe_path(&self, path: &str) -> Result<PathBuf, String> {
        self.validator.validate(path).map_err(|e| e.to_string())
    }

    fn ollama_embedding_client(
        model: Option<String>,
        base_url: Option<String>,
    ) -> docs_rag::embeddings::OllamaEmbeddingClient {
        docs_rag::embeddings::OllamaEmbeddingClient::new(
            base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model.unwrap_or_else(|| "nomic-embed-text".to_string()),
        )
    }

    async fn run_verified_command(&self, command: String, timeout_secs: u64) -> CallToolResult {
        match verifier::commands::run_command(&command, self.workspace_root.as_ref(), timeout_secs)
            .await
        {
            Ok(result) => {
                let summary = verifier::output::summarize(&result);
                Self::ok_json(serde_json::json!({
                    "summary": summary,
                    "result": result,
                }))
            }
            Err(e) => Self::err_text(e),
        }
    }
}

#[tool_router]
impl CodeAssistantServer {
    /// Read a file from the workspace, optionally limiting to a line range.
    #[tool(
        name = "read_file",
        description = "Read a file from the workspace. Optionally specify start_line and end_line for a slice. Returns file content with line numbers."
    )]
    async fn read_file(&self, params: Parameters<ReadFileParams>) -> CallToolResult {
        let p = params.0;
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };

        let content = match tokio::fs::read_to_string(&safe_path).await {
            Ok(c) => c,
            Err(e) => return Self::err_text(format!("cannot read {}: {e}", p.path)),
        };

        let start = p.start_line.unwrap_or(1).max(1) as usize;
        let end = p.end_line.unwrap_or(500) as usize;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let slice_start = (start - 1).min(total_lines);
        let slice_end = end.min(total_lines);
        let truncated = slice_end < total_lines;

        let numbered: Vec<String> = lines[slice_start..slice_end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>6} | {line}", i + slice_start + 1))
            .collect();

        let body = numbered.join("\n");
        let result = serde_json::json!({
            "path": p.path,
            "start_line": slice_start + 1,
            "end_line": slice_end,
            "total_lines": total_lines,
            "truncated": truncated,
            "content": body,
        });
        Self::ok_json(result)
    }

    /// List files in the workspace, optionally filtered by a glob pattern.
    #[tool(
        name = "list_files",
        description = "List files in the workspace. Optionally filter with a glob pattern like '**/*.rs'. Respects .gitignore."
    )]
    async fn list_files(&self, params: Parameters<ListFilesParams>) -> CallToolResult {
        let p = params.0;
        let limit = p.limit.unwrap_or(100);
        let root = self.workspace_root.as_ref();

        let pattern = p.pattern.as_deref().unwrap_or("**/*");
        let glob_pat = format!("{}/{}", root.to_string_lossy(), pattern);

        let mut files: Vec<String> = Vec::new();

        use ignore::WalkBuilder;
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let abs = entry.path();
            // Apply glob filter if provided.
            let matches = if p.pattern.is_some() {
                let glob = glob::Pattern::new(&glob_pat).ok();
                glob.map(|g| g.matches_path(abs)).unwrap_or(true)
            } else {
                true
            };
            if matches {
                if let Ok(rel) = abs.strip_prefix(root) {
                    files.push(rel.to_string_lossy().to_string());
                }
            }
            if files.len() >= limit {
                break;
            }
        }

        Self::ok_json(serde_json::json!({
            "files": files,
            "count": files.len(),
        }))
    }

    /// Search the workspace for code matching a regular expression.
    #[tool(
        name = "grep_code",
        description = "Search the workspace for lines matching a regular expression. Returns file paths, line numbers, and matching lines."
    )]
    async fn grep_code(&self, params: Parameters<GrepCodeParams>) -> CallToolResult {
        let p = params.0;
        let limit = p.limit.unwrap_or(50);
        let root = self.workspace_root.as_ref();

        match code_search::grep::grep_workspace(root, &p.query, limit) {
            Ok(matches) => Self::ok_json(serde_json::json!({
                "query": p.query,
                "matches": matches,
                "count": matches.len(),
            })),
            Err(e) => Self::err_text(format!("regex error: {e}")),
        }
    }

    /// Return a directory tree of the workspace up to a given depth.
    #[tool(
        name = "workspace_tree",
        description = "Return a directory tree of the workspace. Depth defaults to 3. Respects .gitignore."
    )]
    async fn workspace_tree(&self, params: Parameters<WorkspaceTreeParams>) -> CallToolResult {
        let depth = params.0.depth.unwrap_or(3);
        let root = self.workspace_root.as_ref();
        let tree = build_tree(root, root, depth, 0);
        Self::ok_json(serde_json::json!({ "tree": tree }))
    }

    /// Return the current git status of the workspace.
    #[tool(
        name = "git_status",
        description = "Return the current git status of the workspace: branch name and list of changed files."
    )]
    async fn git_status(&self) -> CallToolResult {
        match git_tools::status::workspace_status(self.workspace_root.as_ref()) {
            Ok(s) => Self::ok_json(s),
            Err(e) => Self::err_text(e),
        }
    }

    /// Return the git diff (working tree vs HEAD).
    #[tool(
        name = "git_diff",
        description = "Return the git diff of the entire workspace (working tree vs HEAD)."
    )]
    async fn git_diff(&self) -> CallToolResult {
        match git_tools::diff::workspace_diff(self.workspace_root.as_ref()) {
            Ok(d) => Self::ok_json(d),
            Err(e) => Self::err_text(e),
        }
    }

    /// Return the git diff for a specific file.
    #[tool(
        name = "git_diff_file",
        description = "Return the git diff for a specific file (working tree vs HEAD)."
    )]
    async fn git_diff_file(&self, params: Parameters<GitDiffFileParams>) -> CallToolResult {
        let safe_path = match self.safe_path(&params.0.path) {
            Ok(p) => p,
            Err(e) => return Self::err_text(e),
        };
        let rel = safe_path
            .strip_prefix(self.workspace_root.as_ref())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| params.0.path.clone());

        match git_tools::diff::file_diff(self.workspace_root.as_ref(), &rel) {
            Ok(Some(d)) => Self::ok_json(d),
            Ok(None) => Self::ok_json(serde_json::json!({ "message": "no diff for file" })),
            Err(e) => Self::err_text(e),
        }
    }

    /// Return the git log for a specific file.
    #[tool(
        name = "git_log_file",
        description = "Return the git commit history for a specific file (most recent commits first)."
    )]
    async fn git_log_file(&self, params: Parameters<GitLogFileParams>) -> CallToolResult {
        let safe_path = match self.safe_path(&params.0.path) {
            Ok(p) => p,
            Err(e) => return Self::err_text(e),
        };
        let rel = safe_path
            .strip_prefix(self.workspace_root.as_ref())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| params.0.path.clone());
        let limit = params.0.limit.unwrap_or(10);

        match git_tools::log::file_log(self.workspace_root.as_ref(), &rel, limit) {
            Ok(commits) => Self::ok_json(serde_json::json!({ "commits": commits })),
            Err(e) => Self::err_text(e),
        }
    }

    /// Return git blame for a file or portion of a file.
    #[tool(
        name = "git_blame",
        description = "Return git blame annotations for a file, showing which commit last modified each line."
    )]
    async fn git_blame(&self, params: Parameters<GitBlameParams>) -> CallToolResult {
        let p = params.0;
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        let rel = safe_path
            .strip_prefix(self.workspace_root.as_ref())
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|_| p.path.clone());

        match git_tools::blame::file_blame(self.workspace_root.as_ref(), &rel) {
            Ok(blame) => {
                // Slice to requested line range.
                let start = p.start_line.unwrap_or(1).max(1) as usize;
                let end = p.end_line.unwrap_or(30) as usize;
                let lines: Vec<_> = blame
                    .lines
                    .into_iter()
                    .filter(|l| l.line >= start && l.line <= end)
                    .collect();
                Self::ok_json(serde_json::json!({
                    "path": blame.path,
                    "lines": lines,
                }))
            }
            Err(e) => Self::err_text(e),
        }
    }

    /// Find symbol definitions by name across the workspace.
    #[tool(
        name = "find_symbol",
        description = "Find where a symbol (function, struct, class, etc.) is defined across the workspace. Returns file paths and line numbers."
    )]
    async fn find_symbol(&self, params: Parameters<FindSymbolParams>) -> CallToolResult {
        let p = params.0;
        let idx = self.symbol_index.lock().await;
        let mut matches = idx.find_by_name(&p.name);

        // Apply kind filter.
        if let Some(ref kind_filter) = p.kind {
            let kf = kind_filter.to_lowercase();
            matches.retain(|s| s.kind.to_string().contains(&kf));
        }

        Self::ok_json(serde_json::json!({
            "name": p.name,
            "matches": matches.iter().map(|s| serde_json::json!({
                "name": s.name,
                "kind": s.kind.to_string(),
                "path": s.path,
                "line": s.line,
                "end_line": s.end_line,
            })).collect::<Vec<_>>(),
            "count": matches.len(),
        }))
    }

    /// Find references to a symbol across the workspace.
    #[tool(
        name = "find_references",
        description = "Find textual references to a symbol name across indexed workspace files. Rebuilds the reference index lazily if needed."
    )]
    async fn find_references(&self, params: Parameters<FindReferencesParams>) -> CallToolResult {
        let p = params.0;
        let mut ref_idx = self.reference_index.lock().await;

        if ref_idx.is_empty() {
            let mut sym_idx = self.symbol_index.lock().await;
            if sym_idx.all_symbols().is_empty() {
                match sym_idx.index_workspace(self.workspace_root.as_ref()) {
                    Ok(count) => {
                        let mut state = self.index_state.lock().await;
                        state.symbols_indexed = count;
                    }
                    Err(e) => return Self::err_text(format!("symbol index error: {e}")),
                }
            }

            let count =
                rebuild_reference_index(self.workspace_root.as_ref(), &sym_idx, &mut ref_idx);
            let mut state = self.index_state.lock().await;
            state.references_indexed = count;
            state.last_reindexed_unix = Some(now_unix());
        }

        let refs = ref_idx.find_references(&p.name);
        Self::ok_json(serde_json::json!({
            "name": p.name,
            "references": refs.iter().map(|r| serde_json::json!({
                "path": r.path,
                "line": r.line,
                "context": r.context,
            })).collect::<Vec<_>>(),
            "count": refs.len(),
        }))
    }

    /// List all symbols defined in a file.
    #[tool(
        name = "list_symbols",
        description = "List all symbols (functions, structs, classes, etc.) defined in a specific file."
    )]
    async fn list_symbols(&self, params: Parameters<ListSymbolsParams>) -> CallToolResult {
        let safe_path = match self.safe_path(&params.0.path) {
            Ok(p) => p,
            Err(e) => return Self::err_text(e),
        };
        let rel = safe_path
            .strip_prefix(self.workspace_root.as_ref())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| params.0.path.clone());

        // Parse on-demand for freshness.
        let parser = symbol_index::parser::SymbolParser::new();
        let symbols = match parser.parse_file(&safe_path, &rel) {
            Ok(s) => s,
            Err(e) => return Self::err_text(format!("parse error: {e}")),
        };

        Self::ok_json(serde_json::json!({
            "path": rel,
            "symbols": symbols.iter().map(|s| serde_json::json!({
                "name": s.name,
                "kind": s.kind.to_string(),
                "line": s.line,
                "end_line": s.end_line,
            })).collect::<Vec<_>>(),
            "count": symbols.len(),
        }))
    }

    /// Search workspace code using full-text search.
    #[tool(
        name = "search_code",
        description = "Full-text search across indexed workspace code. Returns matching code chunks with file paths and line ranges."
    )]
    async fn search_code(&self, params: Parameters<SearchCodeParams>) -> CallToolResult {
        let p = params.0;
        let limit = p.limit.unwrap_or(10);
        match self.code_index.search(&p.query, limit) {
            Ok(results) => Self::ok_json(serde_json::json!({
                "query": p.query,
                "results": results,
            })),
            Err(e) => Self::err_text(format!("search error: {e}")),
        }
    }

    /// Run tests or other allowlisted verification commands.
    #[tool(
        name = "run_tests",
        description = "Run an allowlisted verification command (cargo test, pytest, npm test, etc.). Returns exit code, stdout, and stderr."
    )]
    async fn run_tests(&self, params: Parameters<RunTestsParams>) -> CallToolResult {
        let p = params.0;
        let command = if let Some(cmd) = p.command {
            cmd
        } else if let Some(target) = p.target {
            format!("cargo test {target}")
        } else {
            "cargo test".to_string()
        };

        self.run_verified_command(command, 60).await
    }

    /// Run any allowlisted verification command.
    #[tool(
        name = "run_command",
        description = "Run an allowlisted verification command with a timeout. Rejects arbitrary or destructive commands."
    )]
    async fn run_command(&self, params: Parameters<RunCommandParams>) -> CallToolResult {
        let p = params.0;
        self.run_verified_command(p.command, p.timeout_secs.unwrap_or(60))
            .await
    }

    /// Run `cargo check`.
    #[tool(name = "run_check", description = "Run cargo check in the workspace.")]
    async fn run_check(&self) -> CallToolResult {
        self.run_verified_command("cargo check".to_string(), 120)
            .await
    }

    /// Run `cargo clippy`.
    #[tool(
        name = "run_clippy",
        description = "Run cargo clippy in the workspace."
    )]
    async fn run_clippy(&self) -> CallToolResult {
        self.run_verified_command("cargo clippy".to_string(), 120)
            .await
    }

    /// Run `cargo fmt -- --check`.
    #[tool(
        name = "run_fmt_check",
        description = "Run cargo fmt -- --check in the workspace."
    )]
    async fn run_fmt_check(&self) -> CallToolResult {
        self.run_verified_command("cargo fmt -- --check".to_string(), 60)
            .await
    }

    /// Search documentation and conventions in the workspace.
    #[tool(
        name = "search_docs",
        description = "Search project documentation, READMEs, architecture notes, and API specs. Optionally filter by kind: docs, examples, architecture, api, tests, code."
    )]
    async fn search_docs(&self, params: Parameters<SearchDocsParams>) -> CallToolResult {
        let p = params.0;
        let limit = p.limit.unwrap_or(5);
        let kind_filter = p.kind.as_deref().and_then(parse_chunk_kind);

        match self
            .docs_index
            .search(&p.query, kind_filter.as_ref(), limit)
        {
            Ok(results) => Self::ok_json(serde_json::json!({
                "query": p.query,
                "results": results,
            })),
            Err(e) => Self::err_text(format!("docs search error: {e}")),
        }
    }

    /// Embed a short text string with an Ollama embedding model.
    #[tool(
        name = "embed_text",
        description = "Generate an embedding for text using Ollama. Returns dimensions and optionally the raw vector."
    )]
    async fn embed_text(&self, params: Parameters<EmbedTextParams>) -> CallToolResult {
        let p = params.0;
        let client = Self::ollama_embedding_client(p.model, p.base_url);
        match client.embed_text(&p.text).await {
            Ok(embedding) => {
                let dimensions = embedding.len();
                let vector = if p.include_vector.unwrap_or(false) {
                    Some(embedding)
                } else {
                    None
                };
                Self::ok_json(serde_json::json!({
                    "model": client.model(),
                    "base_url": client.base_url(),
                    "dimensions": dimensions,
                    "embedding": vector,
                }))
            }
            Err(e) => Self::err_text(format!("embedding error: {e}")),
        }
    }

    /// Build the semantic documentation index using Ollama embeddings.
    #[tool(
        name = "index_semantic_docs",
        description = "Embed documentation-relevant workspace chunks with Ollama and build an in-memory semantic docs index."
    )]
    async fn index_semantic_docs(
        &self,
        params: Parameters<IndexSemanticDocsParams>,
    ) -> CallToolResult {
        let p = params.0;
        let client = Self::ollama_embedding_client(p.model, p.base_url);
        let mut next_index = docs_rag::embeddings::SemanticDocsIndex::new();
        let max_files = Some(p.max_files.unwrap_or(200));

        match next_index
            .index_workspace(self.workspace_root.as_ref(), &client, max_files)
            .await
        {
            Ok(count) => {
                let mut semantic_idx = self.semantic_docs_index.lock().await;
                *semantic_idx = next_index;
                let mut state = self.index_state.lock().await;
                state.semantic_docs_chunks_indexed = count;
                Self::ok_json(serde_json::json!({
                    "status": "ok",
                    "model": client.model(),
                    "base_url": client.base_url(),
                    "semantic_docs_chunks_indexed": count,
                }))
            }
            Err(e) => Self::err_text(format!("semantic index error: {e}")),
        }
    }

    /// Search documentation using an Ollama embedding query and cosine similarity.
    #[tool(
        name = "search_semantic_docs",
        description = "Search the in-memory semantic docs index using an Ollama embedding query. Call index_semantic_docs first."
    )]
    async fn search_semantic_docs(
        &self,
        params: Parameters<SearchSemanticDocsParams>,
    ) -> CallToolResult {
        let p = params.0;
        let client = Self::ollama_embedding_client(p.model, p.base_url);
        let query_embedding = match client.embed_text(&p.query).await {
            Ok(embedding) => embedding,
            Err(e) => return Self::err_text(format!("embedding error: {e}")),
        };
        let kind_filter = p.kind.as_deref().and_then(parse_chunk_kind);
        let limit = p.limit.unwrap_or(5);
        let semantic_idx = self.semantic_docs_index.lock().await;
        if semantic_idx.is_empty() {
            return Self::err_text("semantic docs index is empty; call index_semantic_docs first");
        }

        let results = semantic_idx.search(&query_embedding, kind_filter.as_ref(), limit);
        Self::ok_json(serde_json::json!({
            "query": p.query,
            "model": client.model(),
            "results": results,
        }))
    }

    /// Propose a unified-diff patch to a file.
    ///
    /// The patch is validated but NOT applied automatically.  Use
    /// `apply_patch` to apply a previously proposed patch.
    #[tool(
        name = "propose_patch",
        description = "Propose a unified-diff patch to a workspace file. The patch is validated for safety but not applied — call apply_patch to apply it."
    )]
    async fn propose_patch(&self, params: Parameters<ProposePatchParams>) -> CallToolResult {
        let p = params.0;
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };

        // Basic validation: confirm the file exists and the diff is non-empty.
        if !safe_path.exists() {
            return Self::err_text(format!("file not found: {}", p.path));
        }
        if p.diff.trim().is_empty() {
            return Self::err_text("diff is empty");
        }

        // Reject patches that are unreasonably large (> 500 lines).
        let diff_lines = p.diff.lines().count();
        if diff_lines > 500 {
            return Self::err_text(format!(
                "diff is too large ({diff_lines} lines, max 500). Split into smaller patches."
            ));
        }

        Self::ok_json(serde_json::json!({
            "status": "proposed",
            "path": p.path,
            "diff_lines": diff_lines,
            "message": "Patch proposed. Review the diff and call apply_patch to apply it.",
            "diff_preview": p.diff.lines().take(20).collect::<Vec<_>>().join("\n"),
        }))
    }

    /// Apply a unified-diff patch to a workspace file.
    #[tool(
        name = "apply_patch",
        description = "Apply a unified-diff patch to a workspace file after path validation, context validation, size checks, and added-line secret scanning."
    )]
    async fn apply_patch(&self, params: Parameters<ApplyPatchParams>) -> CallToolResult {
        let p = params.0;
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        if !safe_path.is_file() {
            return Self::err_text(format!("file not found: {}", p.path));
        }
        if let Err(e) = validate_patch_size(&p.diff) {
            return Self::err_text(e);
        }
        if let Err(result) = scan_diff_added_lines(&p.path, &p.diff) {
            return Self::ok_json(result);
        }

        let original = match tokio::fs::read_to_string(&safe_path).await {
            Ok(content) => content,
            Err(e) => return Self::err_text(format!("cannot read {}: {e}", p.path)),
        };
        let (updated, stats) = match apply_unified_diff(&original, &p.diff) {
            Ok(result) => result,
            Err(e) => return Self::err_text(e),
        };

        if !p.dry_run.unwrap_or(false) {
            if let Err(e) = tokio::fs::write(&safe_path, updated).await {
                return Self::err_text(format!("cannot write {}: {e}", p.path));
            }
        }

        Self::ok_json(serde_json::json!({
            "status": if p.dry_run.unwrap_or(false) { "dry_run" } else { "applied" },
            "path": p.path,
            "hunks": stats.hunks,
            "additions": stats.additions,
            "deletions": stats.deletions,
        }))
    }

    /// Replace exact text in a workspace file.
    #[tool(
        name = "replace_text",
        description = "Replace exact text in a workspace file after path validation and replacement-content secret scanning."
    )]
    async fn replace_text(&self, params: Parameters<ReplaceTextParams>) -> CallToolResult {
        let p = params.0;
        if p.old_text.is_empty() {
            return Self::err_text("old_text must not be empty");
        }
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        if let Err(result) = scan_content_for_secrets(&p.path, &p.new_text) {
            return Self::ok_json(result);
        }

        let content = match tokio::fs::read_to_string(&safe_path).await {
            Ok(content) => content,
            Err(e) => return Self::err_text(format!("cannot read {}: {e}", p.path)),
        };
        let replacements = content.matches(&p.old_text).count();
        if replacements == 0 {
            return Self::err_text("old_text was not found");
        }
        if let Some(expected) = p.expected_replacements {
            if replacements != expected {
                return Self::err_text(format!(
                    "replacement count mismatch: expected {expected}, found {replacements}"
                ));
            }
        }

        let updated = content.replace(&p.old_text, &p.new_text);
        if let Err(e) = tokio::fs::write(&safe_path, updated).await {
            return Self::err_text(format!("cannot write {}: {e}", p.path));
        }

        Self::ok_json(serde_json::json!({
            "status": "ok",
            "path": p.path,
            "replacements": replacements,
        }))
    }

    /// Replace a line range in a workspace file.
    #[tool(
        name = "replace_range",
        description = "Replace a 1-based inclusive line range in a workspace file after path validation and new-content secret scanning."
    )]
    async fn replace_range(&self, params: Parameters<ReplaceRangeParams>) -> CallToolResult {
        let p = params.0;
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        if let Err(result) = scan_content_for_secrets(&p.path, &p.new_content) {
            return Self::ok_json(result);
        }

        let content = match tokio::fs::read_to_string(&safe_path).await {
            Ok(content) => content,
            Err(e) => return Self::err_text(format!("cannot read {}: {e}", p.path)),
        };
        let updated = match replace_line_range(&content, p.start_line, p.end_line, &p.new_content) {
            Ok(content) => content,
            Err(e) => return Self::err_text(e),
        };
        if let Err(e) = tokio::fs::write(&safe_path, updated).await {
            return Self::err_text(format!("cannot write {}: {e}", p.path));
        }

        Self::ok_json(serde_json::json!({
            "status": "ok",
            "path": p.path,
            "start_line": p.start_line,
            "end_line": p.end_line,
        }))
    }

    /// Create a workspace file.
    #[tool(
        name = "create_file",
        description = "Create a workspace file after path validation and content secret scanning. Refuses to overwrite unless overwrite is true."
    )]
    async fn create_file(&self, params: Parameters<CreateFileParams>) -> CallToolResult {
        let p = params.0;
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        if safe_path.exists() && !p.overwrite.unwrap_or(false) {
            return Self::err_text(format!("file already exists: {}", p.path));
        }
        if let Err(result) = scan_content_for_secrets(&p.path, &p.content) {
            return Self::ok_json(result);
        }
        if let Some(parent) = safe_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return Self::err_text(format!("cannot create parent directory: {e}"));
            }
        }
        if let Err(e) = tokio::fs::write(&safe_path, p.content).await {
            return Self::err_text(format!("cannot write {}: {e}", p.path));
        }

        Self::ok_json(serde_json::json!({
            "status": "ok",
            "path": p.path,
        }))
    }

    /// Delete a workspace file.
    #[tool(
        name = "delete_file",
        description = "Delete a workspace file after path validation. Refuses to delete directories."
    )]
    async fn delete_file(&self, params: Parameters<DeleteFileParams>) -> CallToolResult {
        let p = params.0;
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        if !safe_path.is_file() {
            return Self::err_text(format!("not a file: {}", p.path));
        }
        if let Err(e) = tokio::fs::remove_file(&safe_path).await {
            return Self::err_text(format!("cannot delete {}: {e}", p.path));
        }

        Self::ok_json(serde_json::json!({
            "status": "ok",
            "path": p.path,
        }))
    }

    /// Rename or move a workspace file.
    #[tool(
        name = "rename_file",
        description = "Rename or move a workspace file after validating both paths. Refuses to overwrite unless overwrite is true."
    )]
    async fn rename_file(&self, params: Parameters<RenameFileParams>) -> CallToolResult {
        let p = params.0;
        let from = match self.safe_path(&p.from) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        let to = match self.safe_path(&p.to) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        if !from.is_file() {
            return Self::err_text(format!("not a file: {}", p.from));
        }
        if to.exists() {
            if !p.overwrite.unwrap_or(false) {
                return Self::err_text(format!("destination exists: {}", p.to));
            }
            if !to.is_file() {
                return Self::err_text(format!("destination is not a file: {}", p.to));
            }
            if let Err(e) = tokio::fs::remove_file(&to).await {
                return Self::err_text(format!("cannot overwrite {}: {e}", p.to));
            }
        }
        if let Some(parent) = to.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return Self::err_text(format!("cannot create parent directory: {e}"));
            }
        }
        if let Err(e) = tokio::fs::rename(&from, &to).await {
            return Self::err_text(format!("cannot rename {} to {}: {e}", p.from, p.to));
        }

        Self::ok_json(serde_json::json!({
            "status": "ok",
            "from": p.from,
            "to": p.to,
        }))
    }

    /// Scan a workspace file for likely secrets.
    #[tool(
        name = "scan_file_for_secrets",
        description = "Scan a workspace file for common secret patterns without exposing full secret values."
    )]
    async fn scan_file_for_secrets(
        &self,
        params: Parameters<ScanFileForSecretsParams>,
    ) -> CallToolResult {
        let p = params.0;
        let safe_path = match self.safe_path(&p.path) {
            Ok(path) => path,
            Err(e) => return Self::err_text(e),
        };
        let content = match tokio::fs::read_to_string(&safe_path).await {
            Ok(content) => content,
            Err(e) => return Self::err_text(format!("cannot read {}: {e}", p.path)),
        };
        Self::ok_json(SecretScanner::new().scan_content(&p.path, &content))
    }

    /// Scan added lines in a unified diff for likely secrets.
    #[tool(
        name = "scan_patch_for_secrets",
        description = "Scan added lines in a unified diff for common secret patterns without exposing full secret values."
    )]
    async fn scan_patch_for_secrets(
        &self,
        params: Parameters<ScanPatchForSecretsParams>,
    ) -> CallToolResult {
        let p = params.0;
        if let Err(e) = self.safe_path(&p.path) {
            return Self::err_text(e);
        }
        match scan_diff_added_lines(&p.path, &p.diff) {
            Ok(result) | Err(result) => Self::ok_json(result),
        }
    }

    /// Validate a path against the workspace sandbox and sensitive-file denylist.
    #[tool(
        name = "validate_path",
        description = "Validate a path against the workspace sandbox and sensitive-file denylist."
    )]
    async fn validate_path(&self, params: Parameters<ValidatePathParams>) -> CallToolResult {
        let p = params.0;
        match self.safe_path(&p.path) {
            Ok(path) => Self::ok_json(serde_json::json!({
                "valid": true,
                "path": p.path,
                "normalized": path.to_string_lossy(),
            })),
            Err(e) => Self::ok_json(serde_json::json!({
                "valid": false,
                "path": p.path,
                "error": e,
            })),
        }
    }

    /// Validate a command against verifier and safety policies.
    #[tool(
        name = "validate_command",
        description = "Validate a command against the verifier allowlist and safety policy without running it."
    )]
    async fn validate_command(&self, params: Parameters<ValidateCommandParams>) -> CallToolResult {
        let p = params.0;
        let verifier = verifier::allowlist::is_allowed(&p.command).map_err(|e| e.to_string());
        let policy = safety::CommandPolicy::with_defaults()
            .validate_command(&p.command)
            .map_err(|e| e.to_string());
        Self::ok_json(serde_json::json!({
            "command": p.command,
            "verifier_allowed": verifier.is_ok(),
            "verifier_error": verifier.err(),
            "safety_policy_allowed": policy.is_ok(),
            "safety_policy_error": policy.err(),
        }))
    }

    /// Return current index counts and freshness metadata.
    #[tool(
        name = "index_status",
        description = "Return current symbol, reference, lexical, docs, and semantic index counts and freshness metadata."
    )]
    async fn index_status(&self) -> CallToolResult {
        let state = self.index_state.lock().await.clone();
        let live_symbols = self.symbol_index.lock().await.all_symbols().len();
        let live_references = self.reference_index.lock().await.len();
        let live_semantic_docs = self.semantic_docs_index.lock().await.len();
        Self::ok_json(serde_json::json!({
            "state": state,
            "live_counts": {
                "symbols": live_symbols,
                "references": live_references,
                "semantic_docs_chunks": live_semantic_docs,
            },
            "needs_reindex": state.code_chunks_indexed == 0 || state.docs_chunks_indexed == 0 || state.symbols_indexed == 0,
        }))
    }

    /// Trigger a full workspace re-index (symbols, references, code, docs).
    #[tool(
        name = "reindex_workspace",
        description = "Rebuild the workspace symbol, reference, code, and docs indexes. Run this after significant file changes."
    )]
    async fn reindex_workspace(&self) -> CallToolResult {
        let root = self.workspace_root.clone();

        let mut next_sym_idx = symbol_index::symbols::SymbolIndex::new();
        let sym_count = next_sym_idx.index_workspace(root.as_ref()).unwrap_or(0);

        let mut next_ref_idx = symbol_index::references::ReferenceIndex::new();
        let ref_count = rebuild_reference_index(root.as_ref(), &next_sym_idx, &mut next_ref_idx);

        let code_count = self
            .code_index
            .index_workspace(root.as_ref(), 50)
            .unwrap_or(0);

        let docs_count = self.docs_index.index_workspace(root.as_ref()).unwrap_or(0);

        {
            let mut sym_idx = self.symbol_index.lock().await;
            *sym_idx = next_sym_idx;
        }
        {
            let mut ref_idx = self.reference_index.lock().await;
            *ref_idx = next_ref_idx;
        }
        {
            let mut state = self.index_state.lock().await;
            state.symbols_indexed = sym_count;
            state.references_indexed = ref_count;
            state.code_chunks_indexed = code_count;
            state.docs_chunks_indexed = docs_count;
            state.last_reindexed_unix = Some(now_unix());
        }

        Self::ok_json(serde_json::json!({
            "status": "ok",
            "symbols_indexed": sym_count,
            "references_indexed": ref_count,
            "code_chunks_indexed": code_count,
            "docs_chunks_indexed": docs_count,
        }))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CodeAssistantServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::default())
            .with_server_info(Implementation::new(
                "local-code-agent",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Local coding agent. Use tools to inspect files, symbols, and git state \
                 before answering questions or making edits.",
            )
    }
}

/// Recursively build a directory tree string.
fn build_tree(
    root: &std::path::Path,
    current: &std::path::Path,
    max_depth: usize,
    depth: usize,
) -> Vec<String> {
    if depth > max_depth {
        return vec![];
    }
    let mut lines = Vec::new();
    let indent = "  ".repeat(depth);

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden dirs and common noise.
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if ft.is_dir() {
            lines.push(format!("{indent}{name}/"));
            let sub = build_tree(root, &entry.path(), max_depth, depth + 1);
            lines.extend(sub);
        } else {
            lines.push(format!("{indent}{name}"));
        }
    }
    lines
}

fn parse_chunk_kind(kind: &str) -> Option<docs_rag::chunking::ChunkKind> {
    match kind {
        "docs" => Some(docs_rag::chunking::ChunkKind::Docs),
        "examples" => Some(docs_rag::chunking::ChunkKind::Examples),
        "architecture" => Some(docs_rag::chunking::ChunkKind::Architecture),
        "api" => Some(docs_rag::chunking::ChunkKind::Api),
        "tests" => Some(docs_rag::chunking::ChunkKind::Tests),
        "code" => Some(docs_rag::chunking::ChunkKind::Code),
        _ => None,
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn rebuild_reference_index(
    root: &Path,
    sym_idx: &symbol_index::symbols::SymbolIndex,
    ref_idx: &mut symbol_index::references::ReferenceIndex,
) -> u64 {
    use ignore::WalkBuilder;

    ref_idx.clear();
    let symbols = sym_idx.all_symbols();
    if symbols.is_empty() {
        return 0;
    }

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
        if std::fs::metadata(abs_path)
            .map(|meta| meta.len() > 1_024 * 1_024)
            .unwrap_or(true)
        {
            continue;
        }
        let content = match std::fs::read_to_string(abs_path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        let rel_path = abs_path
            .strip_prefix(root)
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|_| abs_path.to_string_lossy().to_string());
        ref_idx.index_file(&rel_path, &content, &symbols);
    }

    ref_idx.len() as u64
}

fn validate_patch_size(diff: &str) -> Result<(), String> {
    if diff.trim().is_empty() {
        return Err("diff is empty".to_string());
    }
    let diff_lines = diff.lines().count();
    if diff_lines > 500 {
        return Err(format!(
            "diff is too large ({diff_lines} lines, max 500). Split into smaller patches."
        ));
    }
    Ok(())
}

fn scan_content_for_secrets(
    path: &str,
    content: &str,
) -> Result<safety::SecretScanResult, safety::SecretScanResult> {
    let result = SecretScanner::new().scan_content(path, content);
    if result.is_clean {
        Ok(result)
    } else {
        Err(result)
    }
}

fn scan_diff_added_lines(
    path: &str,
    diff: &str,
) -> Result<safety::SecretScanResult, safety::SecretScanResult> {
    let added = collect_added_lines(diff).join("\n");
    scan_content_for_secrets(path, &added)
}

fn collect_added_lines(diff: &str) -> Vec<String> {
    diff.lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .map(|line| line[1..].to_string())
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct PatchStats {
    hunks: usize,
    additions: usize,
    deletions: usize,
}

fn apply_unified_diff(original: &str, diff: &str) -> Result<(String, PatchStats), String> {
    let original_lines: Vec<String> = original.lines().map(|line| line.to_string()).collect();
    let mut output = Vec::new();
    let mut original_idx = 0usize;
    let mut stats = PatchStats {
        hunks: 0,
        additions: 0,
        deletions: 0,
    };
    let mut in_hunk = false;

    for line in diff.lines() {
        if line.starts_with("@@") {
            let target_idx =
                parse_hunk_old_start(line).ok_or_else(|| format!("invalid hunk header: {line}"))?;
            if target_idx < original_idx {
                return Err("overlapping or out-of-order patch hunk".to_string());
            }
            while original_idx < target_idx {
                let original_line = original_lines
                    .get(original_idx)
                    .ok_or_else(|| "hunk starts beyond end of file".to_string())?;
                output.push(original_line.clone());
                original_idx += 1;
            }
            stats.hunks += 1;
            in_hunk = true;
            continue;
        }

        if !in_hunk {
            continue;
        }

        if line.starts_with('\\') {
            continue;
        }

        let Some(prefix) = line.as_bytes().first().copied() else {
            return Err("invalid empty patch line inside hunk".to_string());
        };
        let patch_line = &line[1..];
        match prefix {
            b' ' => {
                let original_line = original_lines
                    .get(original_idx)
                    .ok_or_else(|| "context extends beyond end of file".to_string())?;
                if original_line != patch_line {
                    return Err(format!(
                        "patch context mismatch at original line {}",
                        original_idx + 1
                    ));
                }
                output.push(original_line.clone());
                original_idx += 1;
            }
            b'-' => {
                let original_line = original_lines
                    .get(original_idx)
                    .ok_or_else(|| "deletion extends beyond end of file".to_string())?;
                if original_line != patch_line {
                    return Err(format!(
                        "patch deletion mismatch at original line {}",
                        original_idx + 1
                    ));
                }
                original_idx += 1;
                stats.deletions += 1;
            }
            b'+' => {
                output.push(patch_line.to_string());
                stats.additions += 1;
            }
            _ => return Err(format!("invalid patch line in hunk: {line}")),
        }
    }

    if stats.hunks == 0 {
        return Err("diff does not contain any hunks".to_string());
    }

    while original_idx < original_lines.len() {
        output.push(original_lines[original_idx].clone());
        original_idx += 1;
    }

    let mut updated = output.join("\n");
    if original.ends_with('\n') && !updated.is_empty() {
        updated.push('\n');
    }
    Ok((updated, stats))
}

fn parse_hunk_old_start(header: &str) -> Option<usize> {
    let old_range = header.split_once('-')?.1.split_whitespace().next()?;
    let old_start = old_range.split(',').next()?.parse::<usize>().ok()?;
    Some(old_start.saturating_sub(1))
}

fn replace_line_range(
    content: &str,
    start_line: u64,
    end_line: u64,
    new_content: &str,
) -> Result<String, String> {
    if start_line == 0 {
        return Err("start_line must be >= 1".to_string());
    }
    if end_line < start_line {
        return Err("end_line must be >= start_line".to_string());
    }

    let lines: Vec<&str> = content.lines().collect();
    let start_idx = (start_line - 1) as usize;
    let end_idx = end_line as usize;
    if start_idx > lines.len() || end_idx > lines.len() {
        return Err(format!(
            "line range {start_line}-{end_line} is outside file with {} lines",
            lines.len()
        ));
    }

    let mut output: Vec<String> = Vec::new();
    output.extend(lines[..start_idx].iter().map(|line| line.to_string()));
    output.extend(new_content.lines().map(|line| line.to_string()));
    output.extend(lines[end_idx..].iter().map(|line| line.to_string()));

    let mut updated = output.join("\n");
    if content.ends_with('\n') && !updated.is_empty() {
        updated.push('\n');
    }
    Ok(updated)
}
