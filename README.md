# slm_tool — Local Rust MCP/RAG Coding Assistant

A local Rust-based [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server that
surrounds an [Ollama](https://ollama.com/) coding model with better context, tools, verification
loops, and coding-specific retrieval so it performs closer to a stronger agent on real repository
tasks.

## Goal

Help a local LLM reliably:

- Identify relevant files without manual help
- Explain repository code using real context
- Make small, correct edits
- Find references before changing APIs
- Run relevant checks and repair errors from test/typecheck output
- Follow project conventions
- Avoid hallucinating files, functions, and dependencies

## Architecture

```text
VS Code / MCP Client
        │
        ▼
local-code-agent  (Rust MCP server)
        │
        ├── workspace-index   SQLite metadata + incremental crawler
        ├── code-search       Tantivy full-text + grep + ranking
        ├── symbol-index      Tree-sitter symbol/reference extraction
        ├── docs-rag          Documentation chunking + search
        ├── verifier          Allowlisted command runner (cargo/npm/pytest…)
        ├── git-tools         status / diff / blame / log
        └── safety            Path sandbox + command policy + secret denylist
```

## Crates

| Crate | Purpose |
|---|---|
| `mcp-server` | MCP stdio server; exposes all tools to VS Code / Copilot |
| `workspace-index` | SQLite file-metadata store + inotify watcher |
| `code-search` | Tantivy full-text index, grep search, result re-ranking |
| `symbol-index` | Tree-sitter symbol extraction for Rust/Python/JS/TS |
| `docs-rag` | Chunking, category detection, Tantivy docs index |
| `verifier` | Safe command runner with allowlist, timeouts, size limits |
| `git-tools` | Git status, diff, blame, log wrappers via libgit2 |
| `safety` | Path-traversal prevention, sensitive-file denylist, command policy |
| `evals` | Benchmark task definitions, metrics, runner framework |

## Tools Exposed via MCP

| Tool | Description |
|---|---|
| `read_file` | Read a workspace file with optional line range |
| `list_files` | List workspace files with optional glob filter |
| `grep_code` | Full-text grep across the workspace |
| `workspace_tree` | Print the directory tree up to a given depth |
| `git_status` | Show current git working-tree status |
| `git_diff` | Show the full working-tree diff |
| `git_diff_file` | Show diff for a single file |
| `git_log_file` | Show recent commits for a file |
| `git_blame` | Show blame for a file range |
| `find_symbol` | Find a symbol by name across the index |
| `list_symbols` | List all symbols in a file |
| `search_code` | Lexical + symbol hybrid search |
| `run_tests` | Run allowlisted test/check command |
| `search_docs` | Search project documentation chunks |
| `propose_patch` | Validate and preview a unified diff patch |
| `reindex_workspace` | Trigger incremental workspace reindex |

## Prerequisites

- **Rust** ≥ 1.75 (`rustup update stable`)
- **Ollama** running locally with a coding model, e.g. `ollama pull qwen2.5-coder`
- A VS Code extension that supports MCP servers (e.g. GitHub Copilot, Continue, or Claude for VS Code)

## Build

```bash
cargo build --release
```

The binary is produced at `target/release/local-code-agent`.

## Usage

### Run standalone

```bash
./target/release/local-code-agent --workspace /path/to/your/repo
```

Logs go to stderr; MCP protocol runs on stdin/stdout.

### VS Code configuration

Add to your MCP settings (e.g. `.vscode/mcp.json` or user settings):

```json
{
  "servers": {
    "local-code-agent": {
      "type": "stdio",
      "command": "/path/to/local-code-agent",
      "args": ["--workspace", "${workspaceFolder}"]
    }
  }
}
```

## Security Model

- **Workspace sandbox** — all file access is restricted to the workspace root; path-traversal
  attempts (`../`) are rejected.
- **Sensitive-file denylist** — `.env`, `*.pem`, `*.key`, `.ssh/`, `id_rsa`, etc. are never read.
- **Command allowlist** — only pre-approved commands (`cargo`, `npm`, `pytest`, `go test`, …) can
  be executed; no arbitrary shell access.
- **Timeout + size limits** — every command has a hard timeout and output is capped.

## Running Tests

```bash
cargo test
```

## Project Layout

```text
local-code-agent/
  Cargo.toml               workspace manifest
  crates/
    mcp-server/            MCP stdio server + tool dispatch
    workspace-index/       SQLite metadata + file watcher
    code-search/           Tantivy + grep + ranking
    symbol-index/          Tree-sitter parser + symbol store
    docs-rag/              Docs chunking + Tantivy docs index
    verifier/              Safe command runner
    git-tools/             libgit2 wrappers
    safety/                Path sandbox + policy + secret scanner
    evals/                 Evaluation harness
```

## Phase Roadmap

| Phase | Status |
|---|---|
| 1 – Basic MCP server (file tools, git tools) | ✅ |
| 2 – Workspace indexing (SQLite + incremental) | ✅ |
| 3 – Tree-sitter symbol index | ✅ |
| 4 – Hybrid retrieval (lexical + symbol) | ✅ |
| 5 – Docs/convention RAG | ✅ |
| 6 – Test & verification tools | ✅ |
| 7 – Agent loop design (structured prompts) | ✅ |
| 8 – Patch / edit system | ✅ |
| 9 – Git context tools | ✅ |
| 10 – Model & prompt layer | planned |
| 11 – Evaluation harness (framework) | ✅ |
| 12 – Security model | ✅ |
| 13 – VS Code integration | ✅ |
| 14 – Implementation milestones | ✅ |
