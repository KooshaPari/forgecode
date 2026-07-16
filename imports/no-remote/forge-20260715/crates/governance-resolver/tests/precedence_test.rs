use governance_resolver::resolve;
use std::path::PathBuf;

#[test]
fn test_precedence_agents_over_claude() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let doc = resolve(&fixtures).expect("should resolve a doc");
    assert_eq!(doc.source, governance_resolver::Source::AgentsMd);
    assert_eq!(doc.version, 1);
}

#[test]
fn test_precedence_with_only_claude() {
    let tmp = std::env::temp_dir().join("gr_test_only_claude");
    std::fs::create_dir_all(&tmp).ok();
    std::fs::write(tmp.join("CLAUDE.md"), "---\ngovernance_version: 2\n---\n").ok();
    let doc = resolve(&tmp).expect("should resolve from claude");
    assert_eq!(doc.source, governance_resolver::Source::ClaudeMd);
    assert_eq!(doc.version, 2);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_precedence_agents_v2_over_claude_v1() {
    let tmp = std::env::temp_dir().join("gr_test_agents_v2");
    std::fs::create_dir_all(&tmp).ok();
    std::fs::write(tmp.join("CLAUDE.md"), "---\ngovernance_version: 1\n---\n").ok();
    std::fs::write(tmp.join("AGENTS.md"), "---\ngovernance_version: 2\n---\n").ok();
    let doc = resolve(&tmp).expect("should resolve");
    assert_eq!(doc.source, governance_resolver::Source::AgentsMd);
    assert_eq!(doc.version, 2);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_precedence_agents_v1_over_agents_v2() {
    let tmp = std::env::temp_dir().join("gr_test_agents_v1");
    std::fs::create_dir_all(&tmp).ok();
    std::fs::write(tmp.join("AGENTS.md"), "---\ngovernance_version: 1\n---\n").ok();
    let doc = resolve(&tmp).expect("should resolve");
    assert_eq!(doc.source, governance_resolver::Source::AgentsMd);
    assert_eq!(doc.version, 1);
    std::fs::remove_dir_all(&tmp).ok();
}
