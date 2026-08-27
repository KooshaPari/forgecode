use crate::jobs::ReleaseBuilderJob;
use crate::workflow_model::{Event, Job, Level, Permissions, Step, Workflow};

/// Generate the release build and asset-attestation workflow.
///
/// Third-party npm and Homebrew publication jobs are intentionally omitted
/// until fork-owned destinations and credentials are configured.
pub fn release_publish() {
    let release_build_job = ReleaseBuilderJob::new("${{ github.event.release.tag_name }}")
        .release_id("${{ github.event.release.id }}");
    let sbom_job = Job::new("Generate release SBOM")
        .needs("build_release")
        .permissions(Permissions::default().contents(Level::Write))
        .add_step(
            Step::new("Download release assets")
                .env("GH_TOKEN", "${{ github.token }}")
                .run(
                    "set -euo pipefail\nmkdir -p release-assets\ngh release download \"${{ github.event.release.tag_name }}\" \\\n  --repo \"${{ github.repository }}\" \\\n  --dir release-assets \\\n  --pattern \"forge-*\" \\\n  --pattern \"helioslite-*\"",
                ),
        )
        .add_step(
            Step::new("Generate CycloneDX SBOM")
                .uses(
                    "anchore",
                    "sbom-action",
                    "6b92ff5b2cce1787a99198f282dd8a26d1991449",
                )
                .input("path", "release-assets")
                .input("format", "cyclonedx-json")
                .input(
                    "artifact-name",
                    "forgecode-${{ github.event.release.tag_name }}.cdx.json",
                )
                .input(
                    "output-file",
                    "forgecode-${{ github.event.release.tag_name }}.cdx.json",
                )
                .input("upload-artifact", "false")
                .input("upload-release-assets", "true"),
        );
    let attest_job = Job::new("Attest release assets")
        .needs("build_release")
        .permissions(
            Permissions::default()
                .contents(Level::Read)
                .id_token(Level::Write)
                .attestations(Level::Write),
        )
        .add_step(
            Step::new("Download release assets")
                .env("GH_TOKEN", "${{ github.token }}")
                .run(
                    "set -euo pipefail\nmkdir -p release-assets\ngh release download \"${{ github.event.release.tag_name }}\" \\\n  --repo \"${{ github.repository }}\" \\\n  --dir release-assets \\\n  --pattern \"forge-*\" \\\n  --pattern \"helioslite-*\" \\\n  --pattern \"forge_dbd-*\"",
                ),
        )
        .add_step(
            Step::new("Attest release assets")
                .uses(
                    "actions",
                    "attest-build-provenance",
                    "0f67c3f4856b2e3261c31976d6725780e5e4c373",
                )
                .input("subject-path", "release-assets/*"),
        );
    let release_workflow = Workflow::new("Multi Channel Release")
        .on(Event::default().release(["published"]))
        .permissions(Permissions::default().contents(Level::Read))
        .add_job("build_release", release_build_job.into_job())
        .add_job("sbom_release_assets", sbom_job)
        .add_job("attest_release_assets", attest_job);

    super::generate_private_workflow(release_workflow, "release.yml");
}
