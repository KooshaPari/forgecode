pub mod agents_md;
pub mod claude_md;
pub mod settings_json;
pub mod config_toml;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    AgentsMd,
    ClaudeMd,
    SettingsJson,
    ConfigToml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDoc {
    pub source: Source,
    pub version: u32,
    pub raw: String,
}