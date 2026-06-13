use anyhow::Context;
use mcp_server::CodeAssistantServer;
use rmcp::{ServiceExt, transport::stdio};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise tracing; use RUST_LOG env var to control verbosity.
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr) // Write logs to stderr; keep stdout for MCP.
                .with_ansi(false),
        )
        .init();

    let workspace_root = resolve_workspace_root()?;
    tracing::info!("Starting local-code-agent for workspace: {workspace_root:?}");

    // Build the MCP server.  Skip indexing at startup to keep startup fast;
    // the client can call `reindex_workspace` explicitly.
    let server = CodeAssistantServer::new(workspace_root, false)
        .context("failed to initialise code assistant server")?;

    tracing::info!("Server initialised. Listening on stdio.");

    // Connect to the MCP client over stdin/stdout.
    server
        .serve(stdio())
        .await
        .context("MCP server error")?
        .waiting()
        .await
        .context("waiting for server shutdown")?;

    Ok(())
}

/// Determine the workspace root from CLI arguments or the current directory.
fn resolve_workspace_root() -> anyhow::Result<PathBuf> {
    let args: Vec<String> = std::env::args().collect();

    // Support `--workspace <path>` flag.
    let mut workspace_arg = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--workspace" {
            workspace_arg = iter.next().map(|s| PathBuf::from(s));
        }
    }

    let root = workspace_arg
        .or_else(|| std::env::var("WORKSPACE_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    Ok(root)
}
