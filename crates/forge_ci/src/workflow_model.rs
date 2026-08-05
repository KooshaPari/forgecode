//! Minimal ordered GitHub Actions model used by the Forge CI workflow generator.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Default, Serialize)]
pub(crate) struct Workflow {
    name: String,
    #[serde(rename = "on")]
    event: Event,
    #[serde(skip_serializing_if = "Permissions::is_empty")]
    permissions: Permissions,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    jobs: IndexMap<String, Job>,
}

impl Workflow {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Self::default() }
    }

    pub(crate) fn on(mut self, event: Event) -> Self {
        self.event = event;
        self
    }

    pub(crate) fn permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = permissions;
        self
    }

    pub(crate) fn add_job(mut self, id: impl Into<String>, job: Job) -> Self {
        self.jobs.insert(id.into(), job);
        self
    }

    pub(crate) fn to_yaml(&self) -> Result<String, serde_yaml_ng::Error> {
        serde_yaml_ng::to_string(self)
    }
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct Event {
    #[serde(skip_serializing_if = "Option::is_none")]
    push: Option<Push>,
}

impl Event {
    pub(crate) fn push(mut self, push: Push) -> Self {
        self.push = Some(push);
        self
    }
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct Push {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    branches: Vec<String>,
}

impl Push {
    pub(crate) fn add_branch(mut self, branch: impl Into<String>) -> Self {
        self.branches.push(branch.into());
        self
    }
}

#[derive(Clone, Default, Serialize)]
#[serde(transparent)]
pub(crate) struct Permissions(IndexMap<String, Level>);

impl Permissions {
    pub(crate) fn contents(mut self, level: Level) -> Self {
        self.0.insert("contents".to_string(), level);
        self
    }

    pub(crate) fn issues(mut self, level: Level) -> Self {
        self.0.insert("issues".to_string(), level);
        self
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Level {
    Read,
    Write,
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct Job {
    name: String,
    #[serde(rename = "runs-on")]
    runs_on: String,
    #[serde(skip_serializing_if = "Permissions::is_empty")]
    permissions: Permissions,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    steps: Vec<Step>,
}

impl Job {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            runs_on: "ubuntu-latest".to_string(),
            ..Self::default()
        }
    }

    pub(crate) fn add_step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    pub(crate) fn permissions(mut self, permissions: Permissions) -> Self {
        self.permissions = permissions;
        self
    }
}

#[derive(Clone, Default, Serialize)]
pub(crate) struct Step {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    uses: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    value: Option<Value>,
}

impl Step {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Self::default() }
    }

    pub(crate) fn uses(
        mut self,
        owner: impl AsRef<str>,
        action: impl AsRef<str>,
        revision: impl AsRef<str>,
    ) -> Self {
        self.uses = Some(format!(
            "{}/{}@{}",
            owner.as_ref(),
            action.as_ref(),
            revision.as_ref()
        ));
        self
    }

    #[allow(dead_code)]
    pub(crate) fn run(mut self, command: impl Into<String>) -> Self {
        self.run = Some(command.into());
        self
    }
}
