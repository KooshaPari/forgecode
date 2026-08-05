use crate::workflow_model::{Event, Job, Level, Permissions, Push, Step, Workflow};

/// Generate release drafter workflow
pub fn generate_release_drafter_workflow() {
    let release_drafter = Workflow::new("Release Drafter")
        .on(Event::default()
            .push(Push::default().add_branch("main"))
            .pull_request_target(
                [
                    "opened",
                    "reopened",
                    "synchronize",
                    "labeled",
                    "unlabeled",
                    "closed",
                ],
                ["main"],
            ))
        .permissions(
            Permissions::default()
                .contents(Level::Read)
                .pull_requests(Level::Read),
        )
        .add_job(
            "update_release_draft",
            Job::new("update_release_draft")
                .permissions(
                    Permissions::default()
                        .contents(Level::Write)
                        .pull_requests(Level::Read),
                )
                .add_step(
                    Step::new("Release Drafter")
                        .uses(
                            "release-drafter",
                            "release-drafter",
                            "5a60cd8ddda6dc14fce77159675b8fd2cdca4007",
                        )
                        .input("config-name", "release-drafter.yml")
                        .env("GITHUB_TOKEN", "${{ secrets.GITHUB_TOKEN }}"),
                ),
        );

    super::generate_private_workflow(release_drafter, "release-drafter.yml");
}
