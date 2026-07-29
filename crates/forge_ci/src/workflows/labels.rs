use gh_workflow::*;

use crate::jobs::label_sync_job;

/// Generate labels workflow
pub fn generate_labels_workflow() {
    let labels_workflow = Workflow::default()
        .name("Github Label Sync")
        .on(Event {
            push: Some(Push { branches: vec!["main".to_string()], ..Push::default() }),
            ..Event::default()
        })
        .permissions(
            Permissions::default()
                .contents(Level::Read)
                .issues(Level::Read),
        )
        .add_job("label-sync", label_sync_job());

    super::generate_workflow(labels_workflow, "labels.yml");
}
