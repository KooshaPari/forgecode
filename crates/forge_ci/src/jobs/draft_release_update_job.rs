use gh_workflow::*;

/// Create a job to update the release draft
pub fn draft_release_update_job() -> Job {
    Job::new("update_release_draft")
        .add_step(
            Step::new("Auto Labeler")
                .uses("release-drafter", "release-drafter/autolabeler", "eada3c96a64734dd381cfbda23511034e328ddb0")
                .if_condition(Expression::new(
                    "github.event_name == 'pull_request_target'",
                ))
                .env(("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}"))
                .add_with(("config-name", "release-drafter.yml")),
        )
        .add_step(
            Step::new("Release Drafter")
                .uses("release-drafter", "release-drafter", "5a60cd8ddda6dc14fce77159675b8fd2cdca4007")
                .env(("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}"))
                .add_with(("config-name", "release-drafter.yml")),
        )
}
