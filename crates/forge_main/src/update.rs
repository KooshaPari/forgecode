use std::sync::Arc;

use colored::Colorize;
use forge_api::API;
use forge_config::{Update, UpdateFrequency};
use forge_select::ForgeWidget;
use forge_tracker::VERSION;
use update_informer::{Check, Version, registry};

const DEFAULT_UPDATE_REPOSITORY: &str = "KooshaPari/forgecode";
const DEFAULT_UPDATE_INSTALL_URL: &str =
    "https://github.com/KooshaPari/forgecode/releases/latest/download/install.sh";

/// Runs the official ForgeCode GitHub Release installer, failing silently.
/// When `auto_update` is true, exits immediately after a successful update
/// without prompting the user.
///
async fn execute_update_command(api: Arc<impl API>, auto_update: bool) {
    let install_url = std::env::var("FORGE_DEV_UPDATE_URL")
        .unwrap_or_else(|_| DEFAULT_UPDATE_INSTALL_URL.to_string());

    let output = api
        .execute_shell_command_raw(&format!("curl -fsSL {install_url} | sh"))
        .await;

    match output {
        Err(err) => {
            // Send an event to the tracker on failure
            // We don't need to handle this result since we're failing silently
            let _ = send_update_failure_event(&format!("Auto update failed {err}")).await;
        }
        Ok(output) => {
            if output.success() {
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
            } else {
                let exit_output = match output.code() {
                    Some(code) => format!("Process exited with code: {code}"),
                    None => "Process exited without code".to_string(),
                };
                let _ =
                    send_update_failure_event(&format!("Auto update failed, {exit_output}",)).await;
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
pub async fn on_update(api: Arc<impl API>, update: Option<&Update>) {
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

    let repository = std::env::var("FORGE_DEV_UPDATE_REPOSITORY")
        .unwrap_or_else(|_| DEFAULT_UPDATE_REPOSITORY.to_string());
    let informer = update_informer::new(registry::GitHub, repository.as_str(), VERSION)
        .interval(frequency.into());

    if let Some(version) = informer
        .check_version()
        .ok()
        .flatten()
        && (auto_update || confirm_update(version).await)
    {
        execute_update_command(api, auto_update).await;
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
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_should_skip_update_check_when_frequency_is_never() {
        let fixture = UpdateFrequency::Never;

        let actual = should_check_for_updates(&fixture);

        let expected = false;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_release_update_defaults_target_the_owned_fork() {
        let actual = (DEFAULT_UPDATE_REPOSITORY, DEFAULT_UPDATE_INSTALL_URL);

        let expected = (
            "KooshaPari/forgecode",
            "https://github.com/KooshaPari/forgecode/releases/latest/download/install.sh",
        );
        assert_eq!(actual, expected);
    }
}
