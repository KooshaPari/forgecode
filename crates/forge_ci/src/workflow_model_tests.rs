use crate::workflow_model::{Event, Job, Level, Permissions, Push, Step, Workflow};

#[test]
fn serializes_an_ordered_workflow_with_github_actions_keys() {
    let fixture = Workflow::new("CI")
        .on(Event::default().push(Push::default().add_branch("main")))
        .permissions(Permissions::default().contents(Level::Read))
        .add_job(
            "check",
            Job::new("Check").add_step(Step::new("Checkout").uses(
                "actions",
                "checkout",
                "d23441a48e516b6c34aea4fa41551a30e30af803",
            )),
        );

    let actual = fixture.to_yaml().unwrap();
    let expected = "name: CI\non:\n  push:\n    branches:\n      - main\npermissions:\n  contents: read\njobs:\n  check:\n    name: Check\n    runs-on: ubuntu-latest\n    steps:\n      - name: Checkout\n        uses: actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803\n";

    let actual = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&actual).unwrap();
    let expected = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(expected).unwrap();
    assert_eq!(actual, expected);
}
