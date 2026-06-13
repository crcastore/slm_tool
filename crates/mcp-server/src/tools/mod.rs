use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use safety::PathValidator;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

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
    docs_index: Arc<docs_rag::search::DocsIndex>,
    code_index: Arc<code_search::tantivy_index::CodeIndex>,
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
        let docs_idx = docs_rag::search::DocsIndex::open_in_memory()?;
        let code_idx = code_search::tantivy_index::CodeIndex::open_in_memory()?;

        if index {
            let _ = sym_idx.index_workspace(&workspace_root);
            let _ = docs_idx.index_workspace(&workspace_root);
            let _ = code_idx.index_workspace(&workspace_root, 50);
        }

        Ok(Self {
            workspace_root: Arc::new(workspace_root),
            validator: Arc::new(validator),
            tool_router: Self::tool_router(),
            symbol_index: Arc::new(tokio::sync::Mutex::new(sym_idx)),
            docs_index: Arc::new(docs_idx),
            code_index: Arc::new(code_idx),
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
        self.validator
            .validate(path)
            .map_err(|e| e.to_string())
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

        match verifier::commands::run_command(
            &command,
            self.workspace_root.as_ref(),
            60,
        )
        .await
        {
            Ok(result) => Self::ok_json(result),
            Err(e) => Self::err_text(e),
        }
    }

    /// Search documentation and conventions in the workspace.
    #[tool(
        name = "search_docs",
        description = "Search project documentation, READMEs, architecture notes, and API specs. Optionally filter by kind: docs, examples, architecture, api, tests, code."
    )]
    async fn search_docs(&self, params: Parameters<SearchDocsParams>) -> CallToolResult {
        let p = params.0;
        let limit = p.limit.unwrap_or(5);
        let kind_filter = p.kind.as_deref().and_then(|k| {
            match k {
                "docs" => Some(docs_rag::chunking::ChunkKind::Docs),
                "examples" => Some(docs_rag::chunking::ChunkKind::Examples),
                "architecture" => Some(docs_rag::chunking::ChunkKind::Architecture),
                "api" => Some(docs_rag::chunking::ChunkKind::Api),
                "tests" => Some(docs_rag::chunking::ChunkKind::Tests),
                "code" => Some(docs_rag::chunking::ChunkKind::Code),
                _ => None,
            }
        });

        match self.docs_index.search(&p.query, kind_filter.as_ref(), limit) {
            Ok(results) => Self::ok_json(serde_json::json!({
                "query": p.query,
                "results": results,
            })),
            Err(e) => Self::err_text(format!("docs search error: {e}")),
        }
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

    /// Trigger a full workspace re-index (symbols, code, docs).
    #[tool(
        name = "reindex_workspace",
        description = "Rebuild the workspace symbol, code, and docs indexes. Run this after significant file changes."
    )]
    async fn reindex_workspace(&self) -> CallToolResult {
        let root = self.workspace_root.clone();

        // Rebuild symbol index.
        let mut sym_idx = self.symbol_index.lock().await;
        let sym_count = sym_idx.index_workspace(root.as_ref()).unwrap_or(0);

        // Rebuild code index.
        let code_count = self
            .code_index
            .index_workspace(root.as_ref(), 50)
            .unwrap_or(0);

        // Rebuild docs index.
        let docs_count = self
            .docs_index
            .index_workspace(root.as_ref())
            .unwrap_or(0);

        Self::ok_json(serde_json::json!({
            "status": "ok",
            "symbols_indexed": sym_count,
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
fn build_tree(root: &std::path::Path, current: &std::path::Path, max_depth: usize, depth: usize) -> Vec<String> {
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
