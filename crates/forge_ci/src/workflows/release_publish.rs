use gh_workflow::*;

use crate::jobs::ReleaseBuilderJob;

const RELEASE_ASSET_ATTESTATION_JOB: &str = r#"
  attest_release_assets:
    name: Attest release assets
    needs: build_release
    runs-on: ubuntu-latest
    permissions:
      contents: read
      id-token: write
      attestations: write
    steps:
      - name: Download release assets
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          mkdir -p release-assets
          gh release download "${{ github.event.release.tag_name }}" \
            --repo "${{ github.repository }}" \
            --dir release-assets \
            --pattern "forge-*"
          test "$(find release-assets -maxdepth 1 -type f | wc -l | tr -d ' ')" -eq 18
      - name: Attest release assets
        uses: actions/attest-build-provenance@0f67c3f4856b2e3261c31976d6725780e5e4c373
        with:
          subject-path: release-assets/*
"#;

/// Generate the release build workflow.
///
/// Third-party npm and Homebrew publication jobs are intentionally omitted
/// until fork-owned destinations and credentials are configured. Keeping the
/// destinations in generated CI would allow a release in this repository to
/// publish into the upstream project's channels.
pub fn release_publish() {
    let release_build_job = ReleaseBuilderJob::new("${{ github.event.release.tag_name }}")
        .release_id("${{ github.event.release.id }}");
    let npm_workflow = Workflow::default()
        .name("Multi Channel Release")
        .on(Event {
            release: Some(Release { types: vec![ReleaseType::Published] }),
            ..Event::default()
        })
        .permissions(Permissions::default().contents(Level::Read))
        .add_job("build_release", release_build_job.into_job());

    super::generate_workflow_with_suffix(
        npm_workflow,
        "release.yml",
        RELEASE_ASSET_ATTESTATION_JOB,
    );
}
