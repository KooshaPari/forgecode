pub mod config;

use std::sync::Arc;

use thiserror::Error;

use crate::config::Tier;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Pluggable similarity provider.
///
/// - T2  uses local ONNX models (via `fastembed` or similar).
/// - T3  uses a hosted service (e.g. `forgeservices`).
/// - T0/T1 providers return `Ok(None)` to signal "not my tier".
#[async_trait::async_trait]
pub trait SimilarityProvider: Send + Sync {
    /// Compare `new_prompt` against the last indexed prompt for `agent_id`.
    /// Returns a score 0.0–1.0, or `None` if the provider cannot handle this
    /// tier (caller will fall back to T1).
    async fn compare(
        &self,
        agent_id: &str,
        new_prompt: &str,
    ) -> Result<Option<f64>, SimilarityError>;
}

// ---------------------------------------------------------------------------
// Concrete: Hash-only (T0/T1)
// ---------------------------------------------------------------------------

/// A no-op provider that immediately returns `None` so the caller falls back
/// to Jaccard / SHA-256 (`forge_drift` tier 0–1).
pub struct HashOnlyProvider;

impl HashOnlyProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HashOnlyProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SimilarityProvider for HashOnlyProvider {
    async fn compare(
        &self,
        _agent_id: &str,
        _new_prompt: &str,
    ) -> Result<Option<f64>, SimilarityError> {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Concrete: Local fastembed (T2)
// ---------------------------------------------------------------------------

/// Wraps `fastembed-rs` for local ONNX embedding + cosine similarity.
pub struct LocalFastembedProvider;

impl LocalFastembedProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalFastembedProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SimilarityProvider for LocalFastembedProvider {
    async fn compare(
        &self,
        _agent_id: &str,
        _new_prompt: &str,
    ) -> Result<Option<f64>, SimilarityError> {
        // Stub — real `fastembed` integration is deferred to a follow-up PR.
        // The architecture is correct: return None → caller falls back to Jaccard.
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Error)]
pub enum SimilarityError {
    #[error("embedding provider error: {0}")]
    Provider(String),
    #[error("hosted service unreachable")]
    ServiceUnavailable,
}

// ---------------------------------------------------------------------------
// Provider selection
// ---------------------------------------------------------------------------

/// Select the appropriate provider based on configuration.
pub fn select_provider(
    tier: &Tier,
    forgeservices_url: Option<&str>,
) -> Arc<dyn SimilarityProvider> {
    let _ = forgeservices_url; // Used in T3 implementation (follow-up PR).

    match tier {
        Tier::T0 | Tier::T1 => Arc::new(HashOnlyProvider::new()),
        Tier::T2 => {
            // T2: local ONNX — for now, returns None (Jaccard fallback)
            Arc::new(LocalFastembedProvider::new())
        }
        Tier::T3 => {
            // T3: hosted — for now, same fallback
            Arc::new(LocalFastembedProvider::new())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_only_returns_none() {
        let p = HashOnlyProvider::new();
        let r = p.compare("alice", "test prompt").await;
        assert!(r.is_ok());
        assert!(r.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_select_provider_t0() {
        let p = select_provider(&Tier::T0, None);
        let r = p.compare("bob", "another prompt").await;
        assert!(r.is_ok());
        assert!(r.unwrap().is_none());
    }
}
