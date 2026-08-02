//! `brain-mcp` — run the AgenticBox Brain as an MCP server over stdio.
//!
//! Point any MCP client (Hermes, Claude Code, sandboxed agents) at this
//! binary; it exposes `brain_search`, `brain_who_knows`, `brain_recent_prs`,
//! and `brain_audit`.
//!
//! Usage: `brain-mcp [path-to-brain.toml]` (default: `$BRAIN_CONFIG` or
//! `~/.agenticbox/brain.toml`).

use std::path::PathBuf;
use std::sync::Arc;

use brain::mcp::{serve_stdio, BrainConfig, BrainRuntime};
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mut args = std::env::args();
    let _bin = args.next();
    let config_path: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(BrainConfig::default_path);

    let runtime = tokio::runtime::Runtime::new()?;
    let result: anyhow::Result<()> = runtime.block_on(async move {
        let rt = BrainRuntime::from_config(&config_path).await?;
        serve_stdio(Arc::new(rt)).await
    });
    result?;
    Ok(())
}
