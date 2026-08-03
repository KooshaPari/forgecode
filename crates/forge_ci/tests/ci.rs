use forge_ci::workflows as workflow;

#[test]
fn generate() {
    workflow::generate_ci_workflow();
}

#[test]
fn test_release_drafter() {
    workflow::generate_release_drafter_workflow();

    let workflow_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/release-drafter.yml");
    let workflow = std::fs::read_to_string(workflow_path)
        .expect("release drafter workflow should be generated");
    assert!(
        !workflow.contains("Auto Labeler"),
        "pull_request_target must not execute label writes"
    );
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
    assert!(generated.contains("shell: bash"));
    assert!(generated.contains("target: x86_64-unknown-linux-gnu"));
    assert!(generated.contains("target: x86_64-pc-windows-msvc"));
    assert!(generated.contains("matrix.binary_name }}.sha256"));
    assert!(generated.contains("attest_release_assets:"));
    assert!(generated.contains("needs: build_release"));
    assert!(generated.contains("attestations: write"));
    assert!(generated.contains("id-token: write"));
    let release_download = r#"gh release download "${{ github.event.release.tag_name }}" \
            --repo "${{ github.repository }}" \
            --dir release-assets \
            --pattern "forge-*""#;
    assert!(generated.contains(release_download));
    assert!(
        !generated.contains(": \n"),
        "release workflow must not contain trailing whitespace"
    );
    assert!(
        !generated.contains("\\\\\n"),
        "shell continuations must have exactly one trailing backslash"
    );
    assert!(generated.contains("actions/attest-build-provenance@"));
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

    let generated = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/bounty.yml"),
    )
    .expect("generated bounty workflow");
    assert!(generated.contains(
        "if: github.event_name == 'pull_request' || github.event_name == 'pull_request_target'",
    ));
}
