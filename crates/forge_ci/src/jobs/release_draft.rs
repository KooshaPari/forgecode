use crate::workflow_model::{Job, Level, Permissions, Step};

/// Create a draft release job for GitHub Actions
pub fn create_draft_release_job(build_job_id: &str) -> Job {
    Job::new("draft_release")
        .needs(build_job_id)
        .if_condition(
            "github.event_name == 'push' && github.ref == 'refs/heads/main'",
        )
        // This job only runs on push to main, not needed for release events
        .permissions(
            Permissions::default()
                .contents(Level::Write)
                .pull_requests(Level::Write),
        )
        .add_step(Step::new("Checkout Code").uses("actions", "checkout", "d23441a48e516b6c34aea4fa41551a30e30af803"))
        .add_step(
            Step::new("Draft Release").uses("release-drafter", "release-drafter", "5a60cd8ddda6dc14fce77159675b8fd2cdca4007")
                .id("create_release")
                .env("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}")
                .input("config-name", "release-drafter.yml"),
        )
        .add_step(
            Step::new("Export Outputs").run("echo \"crate_release_id=${{ steps.create_release.outputs.id }}\" >> \"$GITHUB_OUTPUT\" && echo \"crate_release_name=${{ steps.create_release.outputs.tag_name }}\" >> \"$GITHUB_OUTPUT\"")
                .id("set_output"),
        )
        .output("crate_release_name", "${{ steps.set_output.outputs.crate_release_name }}")
        .output("crate_release_id", "${{ steps.set_output.outputs.crate_release_id }}")
}
