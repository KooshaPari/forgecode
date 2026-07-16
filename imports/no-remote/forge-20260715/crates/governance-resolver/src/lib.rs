pub mod sources;
pub mod resolver;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub use sources::{GovernanceDoc, Source};

pub fn load_all(definition_path: &Path) -> Vec<GovernanceDoc> {
    let mut docs = Vec::new();
    if let Some(doc) = sources::agents_md::load(definition_path) {
        docs.push(doc);
    }
    if let Some(doc) = sources::claude_md::load(definition_path) {
        docs.push(doc);
    }
    if let Some(doc) = sources::settings_json::load(definition_path) {
        docs.push(doc);
    }
    if let Some(doc) = sources::config_toml::load(definition_path) {
        docs.push(doc);
    }
    docs
}

pub fn resolve(definition_path: &Path) -> Option<GovernanceDoc> {
    let docs = load_all(definition_path);
    resolver::resolve(&docs)
}