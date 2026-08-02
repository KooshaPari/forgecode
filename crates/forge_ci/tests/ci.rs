use forge_ci::workflows as workflow;

#[test]
fn generate() {
    workflow::generate_ci_workflow();
}

#[test]
fn test_release_drafter() {
    workflow::generate_release_drafter_workflow();
}

#[test]
fn test_release_workflow() {
    workflow::release_publish();

    let generated = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.github/workflows/release.yml"),
    )
    .expect("generated release workflow");
    assert!(!generated.contains("npm_release"));
    assert!(!generated.contains("homebrew_release"));
    assert!(generated.contains("Generate SHA-256 checksum"));
    assert!(generated.contains("matrix.binary_name }}.sha256"));
}

#[test]
fn test_labels_workflow() {
    workflow::generate_labels_workflow();
}

#[test]
fn test_stale_workflow() {
    workflow::generate_stale_workflow();
}

#[test]
fn test_autofix_workflow() {
    workflow::generate_autofix_workflow();
}

#[test]
fn test_bounty_workflow() {
    workflow::generate_bounty_workflow();
}
