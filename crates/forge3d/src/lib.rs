//! `forge3d` — shared multi-agent coordination daemon for forgecode.
//!
//! PR-6 ships the minimum viable daemon: a Unix-domain-socket JSON-RPC server
//! with an in-memory agent registry (60s leases), PID-file + flock guard, and
//! no SQLite. Drift detection, similarity scoring, and persistent storage land
//! in PR-9+.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────┐  length-prefixed JSON-RPC  ┌────────────────────┐
//! │ ZSH glue / CLI │ ◀────────────────────────▶ │ forge3d daemon     │
//! │ (forge drift,  │   4-byte BE len + UTF-8    │  - agent registry  │
//! │  forge agent)  │   JSON                    │  - PID + flock     │
//! └────────────────┘                            │  - UDS listener    │
//!                                               └────────────────────┘
//! ```
//!
//! # Public surface
//!
//! - [`protocol`] — wire types: [`Request`], [`Response`]
//! - [`registry`] — [`AgentId`], [`Lane`], [`AgentInfo`], [`Registry`]
//! - [`pidfile`] — [`PidFile`] for single-instance enforcement
//! - [`server`] — UDS server + JSON-RPC dispatcher

pub mod error;
pub mod pidfile;
pub mod protocol;
pub mod registry;
pub mod server;

pub use error::{Forge3Error, Result};
pub use pidfile::PidFile;
pub use protocol::{Request, Response};
pub use registry::{AgentId, AgentInfo, Lane, Registry, LEASE_MS};
pub use server::{Sockets, Server, Clock, system_clock, fixed_clock};
