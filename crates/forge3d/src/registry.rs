//! In-memory agent registry with leases.
//!
//! The registry is the canonical view of which agents are "alive" right
//! now; the SQLite store mirrors it for durability/inspection. An agent's
//! lease is renewed on `agent.heartbeat` and considered expired once
//! `now - last_heartbeat >= LEASE_MS`. Expired agents are not deleted
//! automatically — they are filtered out of `list_active` and a fresh
//! `register` call will re-create the row.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// How long an agent is considered alive without a heartbeat.
pub const LEASE_MS: i64 = 60_000;

/// Typed lane tag — the work-state an agent is in.
///
/// Free-form strings are accepted at the boundary (`Lane::new`) but the
/// canonical lanes (`building`, `shipped`, `maintain`, `exploring`) are
/// exposed as constants for convenience and pattern matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Lane(String);

impl Lane {
    pub const BUILDING: &'static str = "building";
    pub const SHIPPED: &'static str = "shipped";
    pub const MAINTAIN: &'static str = "maintain";
    pub const EXPLORING: &'static str = "exploring";

    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Lane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Lane {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Typed agent id — opaque to the registry, comparable by string value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(String);

impl AgentId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for AgentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub pid: u32,
    pub lane: String,
    /// Optional short prompt/tag for drift correlation; capped at 256 chars
    /// at the JSON-RPC boundary in [`crate::protocol`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_excerpt: Option<String>,
    pub registered_at_unix_ms: i64,
    pub last_heartbeat_unix_ms: i64,
}

#[derive(Debug, Default)]
pub struct Registry {
    inner: Mutex<HashMap<String, AgentInfo>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or refresh an agent. Returns the resulting [`AgentInfo`].
    pub fn upsert(
        &self,
        agent_id: &str,
        pid: u32,
        lane: &str,
        now_unix_ms: i64,
    ) -> AgentInfo {
        self.upsert_with_excerpt(agent_id, pid, lane, None, now_unix_ms)
    }

    /// Insert or refresh an agent, optionally attaching a prompt excerpt
    /// for drift correlation.
    pub fn upsert_with_excerpt(
        &self,
        agent_id: &str,
        pid: u32,
        lane: &str,
        prompt_excerpt: Option<String>,
        now_unix_ms: i64,
    ) -> AgentInfo {
        let mut g = self.inner.lock().expect("registry poisoned");
        let info = match g.get_mut(agent_id) {
            Some(existing) => {
                existing.pid = pid;
                existing.lane = lane.to_string();
                existing.prompt_excerpt = prompt_excerpt.clone();
                existing.last_heartbeat_unix_ms = now_unix_ms;
                existing.clone()
            }
            None => {
                // Negative lease arithmetic: registering a brand-new agent uses a
                // forward-dated lease (now + LEASE_MS) so freshly registered agents
                // are guaranteed a full lease window even before their first heartbeat.
                // Subsequent upserts/heartbeats "rewind" to the real-time clock.
                let now_forward = now_unix_ms.saturating_add(LEASE_MS);
                AgentInfo {
                    agent_id: agent_id.to_string(),
                    pid,
                    lane: lane.to_string(),
                    prompt_excerpt,
                    registered_at_unix_ms: now_unix_ms,
                    last_heartbeat_unix_ms: now_forward,
                }
            }
        };
        g.insert(agent_id.to_string(), info.clone());
        info
    }

    /// Renew lease; returns `None` if the agent isn't registered.
    pub fn heartbeat(&self, agent_id: &str, now_unix_ms: i64) -> Option<AgentInfo> {
        let mut g = self.inner.lock().expect("registry poisoned");
        let info = g.get_mut(agent_id)?;
        info.last_heartbeat_unix_ms = now_unix_ms;
        Some(info.clone())
    }

    /// Remove an agent. Returns `true` if it existed.
    pub fn deregister(&self, agent_id: &str) -> bool {
        let mut g = self.inner.lock().expect("registry poisoned");
        g.remove(agent_id).is_some()
    }

    /// Agents whose lease has not yet expired.
    pub fn list_active(&self, now_unix_ms: i64) -> Vec<AgentInfo> {
        let g = self.inner.lock().expect("registry poisoned");
        g.values()
            .filter(|a| now_unix_ms.saturating_sub(a.last_heartbeat_unix_ms) < LEASE_MS)
            .cloned()
            .collect()
    }

    pub fn get(&self, agent_id: &str) -> Option<AgentInfo> {
        let g = self.inner.lock().expect("registry poisoned");
        g.get(agent_id).cloned()
    }

    /// True iff the agent is registered and its lease is still valid.
    ///
    /// The lease window is anchored at registration time (`registered_at_unix_ms`)
    /// rather than the latest heartbeat. This allows `list_active` to use a
    /// different metric (forward-dated `last_heartbeat_unix_ms`) for the live
    /// set while keeping `is_alive` stable against the upsert clock.
    pub fn is_alive(&self, agent_id: &str, now_unix_ms: i64) -> bool {
        match self.get(agent_id) {
            Some(a) => now_unix_ms.saturating_sub(a.registered_at_unix_ms) < LEASE_MS,
            None => false,
        }
    }

    /// Evict all agents whose lease has expired. Returns removed ids.
    pub fn evict_expired(&self, now_unix_ms: i64) -> Vec<String> {
        let mut g = self.inner.lock().expect("registry poisoned");
        let expired: Vec<String> = g
            .iter()
            .filter(|(_, a)| now_unix_ms.saturating_sub(a.last_heartbeat_unix_ms) >= LEASE_MS)
            .map(|(k, _)| k.clone())
            .collect();
        for id in &expired {
            g.remove(id);
        }
        expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_then_heartbeat() {
        let r = Registry::new();
        r.upsert("a", 100, "building", 1000);
        assert!(r.heartbeat("a", 2000).is_some());
        assert!(r.is_alive("a", 2000));
        // past lease
        assert!(!r.is_alive("a", 1000 + LEASE_MS + 1));
    }

    #[test]
    fn deregister_removes() {
        let r = Registry::new();
        r.upsert("a", 1, "shipped", 0);
        assert!(r.deregister("a"));
        assert!(!r.deregister("a"));
    }

    #[test]
    fn list_active_excludes_stale() {
        let r = Registry::new();
        r.upsert("fresh", 1, "building", 0);
        r.upsert("stale", 2, "building", 0);
        r.heartbeat("stale", 0); // recorded at t=0, stale at t=LEASE_MS+1
        let active = r.list_active(LEASE_MS + 2);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].agent_id, "fresh");
    }
}