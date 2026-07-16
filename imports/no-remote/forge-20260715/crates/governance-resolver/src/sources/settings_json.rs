use super::{GovernanceDoc, Source};
use std::path::Path;

pub fn load(dir: &Path) -> Option<GovernanceDoc> {
    let p = dir.join("settings.json");
    let raw = std::fs::read_to_string(&p).ok()?;
    let version = parse_version(&raw)?;
    Some(GovernanceDoc { source: Source::SettingsJson, version, raw })
}

fn parse_version(raw: &str) -> Option<u32> {
    for line in raw.lines().take(50) {
        if let Some(rest) = line.strip_prefix("governance_version") {
            if let Some(val) = rest.split(':').nth(1) {
                return val.trim().trim_matches(|c| c == ',' || c == '"').parse().ok();
            }
        }
    }
    None
}
