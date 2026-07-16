use super::{GovernanceDoc, Source};

/// Unified governance resolution: agents-md > claude-md > settings-json > config-toml
/// Then by lowest (most stable) version.
pub fn resolve(sources: &[GovernanceDoc]) -> Option<GovernanceDoc> {
    let mut best: Option<&GovernanceDoc> = None;
    for doc in sources {
        best = Some(best.map_or(doc, |b| if priority(doc) > priority(b) { doc } else { b }));
    }
    best.cloned()
}

fn priority(doc: &GovernanceDoc) -> u32 {
    let source_weight = match doc.source {
        Source::AgentsMd => 100,
        Source::ClaudeMd => 80,
        Source::SettingsJson => 60,
        Source::ConfigToml => 40,
    };
    // Lower version = more stable; higher weight wins
    source_weight + (u32::MAX - doc.version)
}
