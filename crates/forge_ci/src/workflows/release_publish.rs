use gh_workflow::*;

use crate::jobs::ReleaseBuilderJob;

/// Generate the release build workflow.
///
/// Third-party npm and Homebrew publication jobs are intentionally omitted
/// until fork-owned destinations and credentials are configured. Keeping the
/// destinations in generated CI would allow a release in this repository to
/// publish into the upstream project's channels.
pub fn release_publish() {
    let release_build_job = ReleaseBuilderJob::new("${{ github.event.release.tag_name }}")
        .release_id("${{ github.event.release.id }}");
    let checksum_job = Job::new("Aggregate Release Checksums")
        .add_needs("build_release")
        .permissions(Permissions::default().contents(Level::Write))
        .add_step(
            Step::new("Download release binaries")
                .run("mkdir -p release-assets && gh release download \"${{ github.event.release.tag_name }}\" --repo \"${{ github.repository }}\" --pattern 'forge-*' --dir release-assets --clobber"),
        )
        .add_step(
            Step::new("Generate aggregate SHA-256 checksums")
                .run("find release-assets -maxdepth 1 -type f ! -name '*.sha256' -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS"),
        )
        .add_step(
            Step::new("Upload aggregate checksums")
                .uses(
                    "xresloader",
                    "upload-to-github-release",
                    "7c5757a90c0bcf0c0e1741da8f2abd7b85e675d0",
                )
                .add_with(("release_id", "${{ github.event.release.id }}"))
                .add_with(("file", "SHA256SUMS"))
                .add_with(("overwrite", "true")),
        );

    let npm_workflow = Workflow::default()
        .name("Multi Channel Release")
        .on(Event {
            release: Some(Release { types: vec![ReleaseType::Published] }),
            ..Event::default()
        })
        .permissions(Permissions::default().contents(Level::Read))
        .add_job("build_release", release_build_job.into_job())
        .add_job("aggregate_checksums", checksum_job);

    super::generate_workflow(npm_workflow, "release.yml");
}
