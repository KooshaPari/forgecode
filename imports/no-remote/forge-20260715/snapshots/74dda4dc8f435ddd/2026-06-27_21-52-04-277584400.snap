use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time;

#[derive(Parser)]
#[command(
    name = "anthropic-usage-poll",
    about = "Poll Anthropic Admin API for token usage"
)]
struct Cli {
    /// Run once and exit
    #[arg(long)]
    once: bool,
    /// Polling interval in seconds
    #[arg(long, default_value = "600")]
    interval: u64,
    /// Output path (default: ~/.claude/usage.json)
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct UsageSnapshot {
    daily_remaining: Option<u64>,
    monthly_remaining: Option<u64>,
    per_model_tokens_last_24h: serde_json::Value,
    updated_at: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = cli.output.unwrap_or_else(default_output_path);
    if cli.once {
        poll_and_write(&output).await?;
    } else {
        let mut ticker = time::interval(Duration::from_secs(cli.interval));
        loop {
            ticker.tick().await;
            if let Err(e) = poll_and_write(&output).await {
                eprintln!("poll error: {e}");
            }
        }
    }
    Ok(())
}

fn default_output_path() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".claude")
        .join("usage.json")
}

async fn poll_and_write(output: &Path) -> Result<()> {
    let snapshot = fetch_usage().await?;
    write_atomic(output, &snapshot)
}

async fn fetch_usage() -> Result<UsageSnapshot> {
    // TODO: read ANTHROPIC_ADMIN_KEY from env; skip poll if unset
    let _api_key = std::env::var("ANTHROPIC_ADMIN_KEY").unwrap_or_default();
    // TODO: add If-None-Match / ETag header to avoid redundant writes
    // TODO: GET /v1/organizations/usage_report/messages with bearer auth
    // TODO: map response body fields into UsageSnapshot
    let _client = reqwest::Client::new();
    Ok(UsageSnapshot {
        daily_remaining: None,
        monthly_remaining: None,
        per_model_tokens_last_24h: serde_json::json!({}),
        updated_at: Utc::now().to_rfc3339(),
    })
}

fn write_atomic(path: &Path, snapshot: &UsageSnapshot) -> Result<()> {
    // TODO: write to a sibling temp file then fs::rename for atomicity
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_snapshot_serializes() {
        let s = UsageSnapshot::default();
        let json = serde_json::to_string(&s).expect("serialize");
        assert!(json.contains("updated_at"));
    }

    #[test]
    fn default_output_path_contains_claude() {
        let p = default_output_path();
        assert!(p.to_string_lossy().contains(".claude"));
    }
}
