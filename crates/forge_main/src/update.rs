use std::sync::Arc;
use std::{fs, io, path::Path};

use colored::Colorize;
use forge_api::API;
use forge_config::{Update, UpdateFrequency};
use forge_select::ForgeWidget;
use forge_tracker::VERSION;
use update_informer::{Check, Version, registry};
use url::Url;
use sha2::{Digest, Sha256};

const ALLOWED_UPDATE_HOSTS: &[&str] = &["helioslite.dev", "forgecode.dev"];
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

fn validate_update_url(raw: &str) -> Option<Url> {
    let url = Url::parse(raw).ok()?;
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || url.host_str().is_none()
        || !ALLOWED_UPDATE_HOSTS.contains(&url.host_str().unwrap())
        || url.port().is_some()
        || url.path() != "/cli"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url)
}

/// Return the release asset name produced for a Rust target triple.
///
/// Release artifacts are intentionally named after their complete target
/// triple, matching the matrix in `.github/workflows/release.yml`. Keeping
/// this mapping explicit prevents an updater from guessing an asset for an
/// unsupported platform.
fn release_asset_for_target(target: &str) -> Option<&'static str> {
    match target {
        "aarch64-apple-darwin" => Some("forge-aarch64-apple-darwin"),
        "x86_64-apple-darwin" => Some("forge-x86_64-apple-darwin"),
        "aarch64-unknown-linux-gnu" => Some("forge-aarch64-unknown-linux-gnu"),
        "x86_64-unknown-linux-gnu" => Some("forge-x86_64-unknown-linux-gnu"),
        "aarch64-unknown-linux-musl" => Some("forge-aarch64-unknown-linux-musl"),
        "x86_64-unknown-linux-musl" => Some("forge-x86_64-unknown-linux-musl"),
        "aarch64-pc-windows-msvc" => Some("forge-aarch64-pc-windows-msvc.exe"),
        "x86_64-pc-windows-msvc" => Some("forge-x86_64-pc-windows-msvc.exe"),
        _ => None,
    }
}

/// Build the canonical GitHub release URL for a supported target asset.
///
/// Versions may be supplied with or without the conventional leading `v`.
/// The returned URL is only constructed for non-empty, release-safe version
/// strings and targets present in the release matrix.
fn release_asset_url(version: &str, target: &str) -> Option<Url> {
    let version = version.strip_prefix('v').unwrap_or(version);
    if version.is_empty()
        || version.starts_with('v')
        || version.len() > 64
        || !version.as_bytes()[0].is_ascii_digit()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return None;
    }
    let asset = release_asset_for_target(target)?;
    Url::parse(&format!(
        "https://github.com/{DEFAULT_UPDATE_REPO}/releases/download/v{version}/{asset}"
    ))
    .ok()
}

/// Parse the first SHA-256 token from a sidecar checksum document.
fn parse_sha256_sidecar(contents: &str) -> Option<[u8; 32]> {
    let token = contents.split_whitespace().next()?;
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut digest = [0u8; 32];
    for (index, chunk) in token.as_bytes().chunks_exact(2).enumerate() {
        digest[index] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
    }
    Some(digest)
}

fn verify_sha256(payload: &[u8], expected: &[u8; 32]) -> bool {
    Sha256::digest(payload).as_slice() == expected
}

/// Stage a verified release beside the current executable and atomically replace it.
/// The caller must verify the downloaded bytes before invoking this function.
fn install_verified_binary(payload: &[u8], destination: &Path) -> io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent")
    })?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    use std::io::Write;
    temp.write_all(payload)?;
    temp.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file().set_permissions(fs::Permissions::from_mode(0o755))?;
    }
    temp.persist(destination).map_err(|error| error.error)?;
    Ok(())
}

fn verified_update_command(url: &Url) -> String {
    let raw = url.as_str();
    format!(
        "tmp_dir=\"$(mktemp -d)\" && trap 'rm -rf \"$tmp_dir\"' EXIT && curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --output \"$tmp_dir/update.sh\" '{raw}' && curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --output \"$tmp_dir/update.sh.sha256\" '{raw}.sha256' && expected=\"$(awk 'NF {{ print $1; exit }}' \"$tmp_dir/update.sh.sha256\")\" && case \"$expected\" in (''|*[!0123456789abcdefABCDEF]*) exit 1;; esac && [ \"${{#expected}}\" -eq 64 ] && actual=\"$(if command -v sha256sum >/dev/null 2>&1; then sha256sum \"$tmp_dir/update.sh\"; else shasum -a 256 \"$tmp_dir/update.sh\"; fi | awk '{{print $1}}')\" && [ \"$(printf '%s' \"$expected\" | tr '[:upper:]' '[:lower:]')\" = \"$(printf '%s' \"$actual\" | tr '[:upper:]' '[:lower:]')\" ] && sh \"$tmp_dir/update.sh\""
    )
}

/// Runs the official installation script to update Forge, failing silently.
/// When `auto_update` is true, exits immediately after a successful update
/// without prompting the user.
///
/// Phenotype rename: by default we hit `helioslite.dev/cli`; if that
/// endpoint is unreachable we fall back to the upstream `forgecode.dev/cli`
/// URL so users running pre-rename builds keep working.
async fn execute_update_command(api: Arc<impl API>, auto_update: bool) {
    let primary = std::env::var("HELIOSLITE_UPDATE_URL")
        .ok()
        .and_then(|raw| validate_update_url(&raw))
        .unwrap_or_else(|| Url::parse("https://helioslite.dev/cli").expect("valid update URL"));
    let fallback = Url::parse("https://forgecode.dev/cli").expect("valid update URL");

    let output = match api
        .execute_shell_command_raw(&verified_update_command(&primary))
        .await
    {
        Ok(output) => Ok(output),
        Err(_) => {
            api.execute_shell_command_raw(&verified_update_command(&fallback))
                .await
        }
    };

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

    if let Some(version) = informer_primary
        .check_version()
        .ok()
        .flatten()
        .or_else(|| informer_legacy.check_version().ok().flatten())
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
    fn update_url_allows_only_trusted_https_hosts() {
        assert!(validate_update_url("https://helioslite.dev/cli").is_some());
        assert!(validate_update_url("https://forgecode.dev/cli").is_some());
        assert!(validate_update_url("http://helioslite.dev/cli").is_none());
        assert!(validate_update_url("https://example.com/cli").is_none());
        assert!(validate_update_url("https://helioslite.dev:8443/cli").is_none());
        assert!(validate_update_url("https://helioslite.dev/other").is_none());
        assert!(validate_update_url("https://helioslite.dev/cli?x=1").is_none());
        assert!(validate_update_url("https://helioslite.dev/cli#fragment").is_none());
        assert!(validate_update_url("https://helioslite.dev/cli; rm -rf /").is_none());
    }

    #[test]
    fn update_repo_accepts_github_style_owner_and_name() {
        assert_eq!(validate_update_repo("KooshaPari/forgecode"), Some("KooshaPari/forgecode"));
        assert_eq!(validate_update_repo("org_name/tool.v2"), Some("org_name/tool.v2"));
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

    #[test]
    fn release_asset_maps_supported_target_triples() {
        for (target, expected) in [
            ("aarch64-apple-darwin", "forge-aarch64-apple-darwin"),
            ("x86_64-apple-darwin", "forge-x86_64-apple-darwin"),
            ("aarch64-unknown-linux-gnu", "forge-aarch64-unknown-linux-gnu"),
            ("x86_64-unknown-linux-gnu", "forge-x86_64-unknown-linux-gnu"),
            ("aarch64-unknown-linux-musl", "forge-aarch64-unknown-linux-musl"),
            ("x86_64-unknown-linux-musl", "forge-x86_64-unknown-linux-musl"),
            ("aarch64-pc-windows-msvc", "forge-aarch64-pc-windows-msvc.exe"),
            ("x86_64-pc-windows-msvc", "forge-x86_64-pc-windows-msvc.exe"),
        ] {
            assert_eq!(release_asset_for_target(target), Some(expected));
        }
    }

    #[test]
    fn release_asset_rejects_unsupported_target_triples() {
        for target in [
            "aarch64-linux-android",
            "x86_64-pc-windows-gnu",
            "x86_64-unknown-freebsd",
            "wasm32-unknown-unknown",
            "",
        ] {
            assert_eq!(release_asset_for_target(target), None, "accepted {target:?}");
        }
    }

    #[test]
    fn release_asset_url_normalizes_version_and_target() {
        assert_eq!(
            release_asset_url("2.10.2", "aarch64-apple-darwin").unwrap().as_str(),
            "https://github.com/KooshaPari/forgecode/releases/download/v2.10.2/forge-aarch64-apple-darwin"
        );
        assert_eq!(
            release_asset_url("v2.10.2", "x86_64-pc-windows-msvc")
                .unwrap()
                .as_str(),
            "https://github.com/KooshaPari/forgecode/releases/download/v2.10.2/forge-x86_64-pc-windows-msvc.exe"
        );
    }

    #[test]
    fn release_asset_url_rejects_invalid_versions_and_targets() {
        for version in [
            "",
            "v",
            "vv2.10.2",
            "release-2.10.2",
            "2.10.2/../../x",
            "2.10.2?download=1",
            "2.10.2#x",
        ] {
            assert!(release_asset_url(version, "aarch64-apple-darwin").is_none());
        }
        assert!(release_asset_url("2.10.2", "x86_64-pc-windows-gnu").is_none());
    }

    #[test]
    fn verified_update_command_requires_checksum_before_execution() {
        let url = Url::parse("https://helioslite.dev/cli").unwrap();
        let command = verified_update_command(&url);
        assert!(command.contains("/cli.sha256"));
        assert!(command.contains("sha256sum"));
        assert!(command.contains("sh \"$tmp_dir/update.sh\""));
        assert!(command.contains("[ \"${#expected}\" -eq 64 ]"));
    }

    #[test]
    fn checksum_sidecar_parser_and_verifier_are_strict() {
        let payload = b"forge release";
        let digest = Sha256::digest(payload);
        let sidecar = format!("{:x}  forge-aarch64-apple-darwin\n", digest);
        let expected = parse_sha256_sidecar(&sidecar).unwrap();
        assert!(verify_sha256(payload, &expected));
        assert!(parse_sha256_sidecar("not-a-digest").is_none());
        assert!(parse_sha256_sidecar(&"a".repeat(64)).is_some());
        assert!(parse_sha256_sidecar(&"g".repeat(64)).is_none());
    }

    #[test]
    fn verified_binary_is_staged_and_replaced_atomically() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("bin/forge");
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::write(&destination, b"old binary").unwrap();
        install_verified_binary(b"new binary", &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"new binary");
    }
}
