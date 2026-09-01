//! Bridge to the HeliosLite agent — runs the `forge` CLI binary.
//!
//! Spawns `forge --request "..." --output-format json` and captures its
//! output.  The `forge` binary handles model resolution, provider auth,
//! context compression, and the full agent loop.

use anyhow::Result;
use bstr::ByteSlice;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Outcome of running the agent on a `@helios` mention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResult {
    /// The agent's response text to post back as a comment.
    pub response: String,
    /// Whether the agent created a PR (vs just commenting).
    pub created_pr: bool,
    /// PR number if one was created.
    pub pr_number: Option<u64>,
}

/// Run the HeliosLite agent on a request.
///
/// `repo_dir` is the path to a checked-out working copy of the target repo.
/// `request` is the natural-language ask from the issue/PR comment.
#[allow(dead_code)]
pub async fn run_agent(repo_dir: &Path, request: &str, llm_api_key: &str) -> Result<AgentResult> {
    let child = Command::new("forge")
        .arg("--request")
        .arg(request)
        .arg("--output-format")
        .arg("json")
        .current_dir(repo_dir)
        .env("LLM_API_KEY", llm_api_key)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        let stderr = output.stderr.to_str_lossy();
        anyhow::bail!("forge exited {}: {}", output.status, stderr);
    }

    // Parse the JSON response.  We accept either the canonical schema
    // (`{"response": "...", "pr_number": ...}`) or just plain text on stdout.
    let stdout = output.stdout.to_str_lossy();
    if let Ok(parsed) = serde_json::from_str::<ForgeJsonOutput>(&stdout) {
        Ok(AgentResult {
            response: parsed.response,
            created_pr: parsed.pr_number.is_some(),
            pr_number: parsed.pr_number,
        })
    } else {
        // Plain text fallback.
        Ok(AgentResult {
            response: stdout.into_owned(),
            created_pr: false,
            pr_number: None,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
struct ForgeJsonOutput {
    response: String,
    #[serde(default)]
    pr_number: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stub that bypasses the binary and returns a canned response.
    /// Useful when the test environment doesn't have `forge` on PATH.
    pub async fn run_agent_stub(repo_dir: &Path, request: &str) -> Result<AgentResult> {
        let _ = repo_dir;
        Ok(AgentResult {
            response: format!("[stub] received request: {request}"),
            created_pr: false,
            pr_number: None,
        })
    }

    #[tokio::test]
    async fn stub_returns_input() {
        let r = run_agent_stub(Path::new("."), "hello world").await.unwrap();
        assert!(r.response.contains("hello world"));
        assert_eq!(r.pr_number, None);
    }
}
