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
    let npm_workflow = Workflow::default()
        .name("Multi Channel Release")
        .on(Event {
            release: Some(Release { types: vec![ReleaseType::Published] }),
            ..Event::default()
        })
        .permissions(Permissions::default().contents(Level::Read))
        .add_job("build_release", release_build_job.into_job());

    super::generate_workflow(npm_workflow, "release.yml");
}
