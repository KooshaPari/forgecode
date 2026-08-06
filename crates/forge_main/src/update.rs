use colored::Colorize;
use forge_config::{Update, UpdateFrequency};
use forge_select::ForgeWidget;
use forge_tracker::VERSION;
use futures::future::BoxFuture;
use update_informer::{Check, Version, registry};

const DEFAULT_UPDATE_REPO: &str = "KooshaPari/forgecode";

fn validate_update_repo(raw: &str) -> Option<&str> {
    let (owner, name) = raw.split_once('/')?;
    if owner.is_empty() || name.is_empty() || raw.len() > 100 || name.ends_with(".git") {
        return None;
    }
    let valid = |part: &str| {
        part.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            && !part.starts_with('.')
            && !part.ends_with('.')
    };
    (valid(owner) && valid(name)).then_some(raw)
}

/// Boundary for replacing the current executable with the notified release.
trait NativeUpdateExecutor {
    fn execute<'a>(
        &'a self,
        repository: &'a str,
        version: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<()>>;
}

struct CurrentExecutableNativeUpdateExecutor;

impl NativeUpdateExecutor for CurrentExecutableNativeUpdateExecutor {
    fn execute<'a>(
        &'a self,
        repository: &'a str,
        version: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            crate::native_update::update_current_executable(repository, version)
                .await
                .map_err(Into::into)
        })
    }
}

/// Applies the notified release with the native updater, failing silently.
/// When `auto_update` is true, exits immediately after a successful update
/// without prompting the user.
async fn execute_update_command(
    executor: &impl NativeUpdateExecutor,
    repository: &str,
    notified_version: &str,
    auto_update: bool,
) {
    match executor.execute(repository, notified_version).await {
        Err(err) => {
            let _ = send_update_failure_event(&format!("Auto update failed {err}")).await;
        }
        Ok(()) => {
            let should_exit = if auto_update {
                true
            } else {
                let answer = forge_select::ForgeWidget::confirm(
                    "You need to close forge to complete update. Do you want to close it now?",
                )
                .with_default(true)
                .prompt();
                answer.unwrap_or_default().unwrap_or_default()
            };
            if should_exit {
                std::process::exit(0);
            }
        }
    }
}

async fn confirm_update(version: Version) -> bool {
    let answer = ForgeWidget::confirm(format!(
        "Confirm upgrade from {} -> {} (latest)?",
        VERSION.to_string().bold().white(),
        version.to_string().bold().white()
    ))
    .with_default(true)
    .prompt();

    match answer {
        Ok(Some(result)) => result,
        Ok(None) => false, // User canceled
        Err(_) => false,   // Error occurred
    }
}

fn should_check_for_updates(frequency: &UpdateFrequency) -> bool {
    !matches!(frequency, UpdateFrequency::Never)
}

fn choose_update_source<'a, T>(
    primary: Option<T>,
    primary_repo: &'a str,
    legacy: Option<T>,
    legacy_repo: &'a str,
) -> Option<(T, &'a str)> {
    primary
        .map(|version| (version, primary_repo))
        .or_else(|| legacy.map(|version| (version, legacy_repo)))
}

// Phenotype-org: detect non-interactive (agent/CI) invocations to skip the
// update check entirely.  Avoids a ~220ms GitHub API round-trip on every
// agent spawn; see profiling notes in perf/profile-zig-hotpath-2026-06-30.
fn is_non_interactive() -> bool {
    use std::io::IsTerminal;
    // CI env vars (standard subset)
    if std::env::var_os("CI").is_some()
        || std::env::var_os("FORGE_NON_INTERACTIVE").is_some()
        || std::env::var_os("FORGE_AGENT_MODE").is_some()
    {
        return true;
    }
    // stdin is not a TTY — running in a pipe or scripted context
    !std::io::stdin().is_terminal()
}

/// Checks if there is an update available
pub async fn on_update(update: Option<&Update>) {
    let update = update.cloned().unwrap_or_default();
    let frequency = update.frequency.unwrap_or_default();

    if !should_check_for_updates(&frequency) {
        return;
    }

    // Phenotype-org: skip update check in CI / non-TTY / agent-batch mode.
    // Each forge process pays ~220ms for a GitHub API call when `frequency`
    // is `Always`; agent fleets spawn many short-lived processes and this
    // dominates per-invocation overhead.
    if is_non_interactive() {
        return;
    }

    let auto_update = update.auto_update.unwrap_or_default();

    // Check if version is development version, in which case we skip the update
    // check
    if VERSION.contains("dev") || VERSION == "0.1.0" {
        // Skip update for development version 0.1.0
        return;
    }

    // Phenotype rename: prefer the renamed-binary GitHub repo
    // (`KooshaPari/forgecode`). The fork's releases are served from the canonical
    // are kept as the canonical source for both name chains; `HELIOSLITE_REPO`
    // overrides the lookup so nightlies can target a third-party fork without
    // recompiling.
    //
    // Tombstone: until the rename is pushed to remote (Gate 4b),
    // the canonical repository lookup can fail. We swallow
    // that case and try the legacy `KooshaPari/forgecode` releases so users on
    // pre-rename builds keep getting notified. This branch will be removed once
    // the rename is permanent.
    let primary_repo = std::env::var("HELIOSLITE_REPO")
        .ok()
        .and_then(|raw| validate_update_repo(&raw).map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_UPDATE_REPO.to_string());
    let legacy_repo = DEFAULT_UPDATE_REPO;
    let informer_primary = update_informer::new(registry::GitHub, primary_repo.as_str(), VERSION)
        .interval(frequency.clone().into());
    let informer_legacy =
        update_informer::new(registry::GitHub, legacy_repo, VERSION).interval(frequency.into());

    let primary_version = informer_primary.check_version().ok().flatten();
    let legacy_version = informer_legacy.check_version().ok().flatten();
    if let Some((version, source_repo)) = choose_update_source(
        primary_version,
        primary_repo.as_str(),
        legacy_version,
        legacy_repo,
    ) {
        let notified_version = version.to_string();
        if auto_update || confirm_update(version).await {
            execute_update_command(
                &CurrentExecutableNativeUpdateExecutor,
                source_repo,
                &notified_version,
                auto_update,
            )
            .await;
        }
    }
}

/// Sends an event to the tracker when an update fails
async fn send_update_failure_event(error_msg: &str) -> anyhow::Result<()> {
    tracing::error!(error = error_msg, "Update failed");
    // Always return Ok since we want to fail silently
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use futures::future::BoxFuture;
    use pretty_assertions::assert_eq;

    use super::*;

    struct RecordingNativeUpdateExecutor {
        notified_versions: Mutex<Vec<String>>,
    }

    impl RecordingNativeUpdateExecutor {
        fn new() -> Self {
            Self { notified_versions: Mutex::new(Vec::new()) }
        }

        fn notified_versions(&self) -> Vec<String> {
            self.notified_versions.lock().unwrap().clone()
        }
    }

    impl NativeUpdateExecutor for RecordingNativeUpdateExecutor {
        fn execute<'a>(
            &'a self,
            _repository: &'a str,
            version: &'a str,
        ) -> BoxFuture<'a, anyhow::Result<()>> {
            Box::pin(async move {
                self.notified_versions
                    .lock()
                    .unwrap()
                    .push(version.to_string());
                Err(anyhow::anyhow!("native update deliberately failed"))
            })
        }
    }

    #[tokio::test]
    async fn update_execution_invokes_native_executor_with_notified_version() {
        let fixture = RecordingNativeUpdateExecutor::new();

        execute_update_command(&fixture, DEFAULT_UPDATE_REPO, "v2.10.2", false).await;

        let actual = fixture.notified_versions();
        let expected = vec!["v2.10.2".to_string()];
        assert_eq!(actual, expected);
    }

    #[test]
    fn native_update_plan_builds_immutable_release_urls() {
        let fixture = crate::native_update::NativeUpdatePlan::new(
            DEFAULT_UPDATE_REPO,
            "v2.10.2",
            "aarch64-apple-darwin",
        )
        .unwrap();

        let actual = (
            fixture.asset_url().as_str().to_string(),
            fixture.checksum_url().as_str().to_string(),
        );
        let release_base = "https://github.com/KooshaPari/forgecode/releases/download/v2.10.2";
        let expected = (
            format!("{release_base}/forge-aarch64-apple-darwin"),
            format!("{release_base}/forge-aarch64-apple-darwin.sha256"),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn fallback_update_uses_the_repository_that_produced_the_version() {
        let selected = choose_update_source(
            None::<u8>,
            "nightly-org/forgecode",
            Some(7),
            DEFAULT_UPDATE_REPO,
        );
        assert_eq!(selected, Some((7, DEFAULT_UPDATE_REPO)));
    }

    #[test]
    fn test_should_skip_update_check_when_frequency_is_never() {
        let fixture = UpdateFrequency::Never;

        let actual = should_check_for_updates(&fixture);

        let expected = false;
        assert_eq!(actual, expected);
    }

    #[test]
    fn update_repo_accepts_github_style_owner_and_name() {
        assert_eq!(
            validate_update_repo("KooshaPari/forgecode"),
            Some("KooshaPari/forgecode")
        );
        assert_eq!(
            validate_update_repo("org_name/tool.v2"),
            Some("org_name/tool.v2")
        );
    }

    #[test]
    fn update_repo_rejects_injection_and_malformed_values() {
        for value in [
            "",
            "forgecode",
            "/forgecode",
            "KooshaPari/",
            "KooshaPari/forge code",
            "KooshaPari/forgecode?x=1",
            "KooshaPari/forgecode.git",
            "../owner/repo",
            "owner/.repo",
            "owner/repo/extra",
        ] {
            assert!(validate_update_repo(value).is_none(), "accepted {value:?}");
        }
    }
}
