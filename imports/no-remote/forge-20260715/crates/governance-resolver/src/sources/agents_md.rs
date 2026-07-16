use super::{GovernanceDoc, Source};
use std::path::Path;

pub fn load(dir: &Path) -> Option<GovernanceDoc> {
    let p = dir.join("AGENTS.md");
    let raw = std::fs::read_to_string(&p).ok()?;
    let version = parse_version(&raw)?;
    Some(GovernanceDoc { source: Source::AgentsMd, version, raw })
}

fn parse_version(raw: &str) -> Option<u32> {
    // YAML front-matter on first 20 lines: `governance_version: <n>`
    for line in raw.lines().take(20) {
        if let Some(rest) = line.strip_prefix("governance_version:") {
            return rest.trim().parse().ok();
        }
    }
    None
}