use std::sync::Arc;

use colored::Colorize;
use forge_api::API;
use forge_config::{Update, UpdateFrequency};
use forge_select::ForgeWidget;
use forge_tracker::VERSION;
use update_informer::{Check, Version, registry};

/// Runs the official installation script to update Forge, failing silently.
/// When `auto_update` is true, exits immediately after a successful update
/// without prompting the user.
async fn execute_update_command(api: Arc<impl API>, auto_update: bool) {
    // The POSIX `curl … | sh` pipe cannot work on native Windows: there is no
    // `sh`, and cmd.exe stops resolving commands once PATH exceeds its ~2047
    // char batch limit. Use a native Windows updater there instead.
    let command = update_command();

    // Spawn a new task that won't block the main application
    let output = api.execute_shell_command_raw(&command).await;

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

/// Returns the update command for the current platform.
///
/// On Windows this returns a PowerShell invocation that downloads the release
/// binary and stages an atomic swap; on other platforms it returns the official
/// `curl … | sh` one-liner.
fn update_command() -> String {
    #[cfg(windows)]
    {
        windows_update_command().unwrap_or_else(|| "exit 1".to_string())
    }
    #[cfg(not(windows))]
    {
        "curl -fsSL https://forgecode.dev/cli | sh".to_string()
    }
}

/// Builds a native Windows update command.
///
/// The running `forge.exe` is locked while the process is alive, so the new
/// binary cannot be replaced in place. Instead we:
///
/// 1. Download `forge-{arch}-pc-windows-msvc.exe` to `forge.exe.new` next to
///    the current binary using PowerShell (absolute paths only, immune to the
///    length-capped PATH that breaks `curl` resolution in cmd.exe).
/// 2. Stage a small `.cmd` helper that waits for `forge.exe` to exit, swaps
///    `forge.exe.new` over `forge.exe`, cleans up, and relaunches forge.
/// 3. Launch that helper detached, so it survives forge exiting.
#[cfg(windows)]
fn windows_update_command() -> Option<String> {
    use std::io::Write;

    let local_app_data = std::env::var("LOCALAPPDATA").ok()?;
    let install_dir = format!(r"{local_app_data}\Programs\Forge");
    let new_exe = format!(r"{install_dir}\forge.exe.new");
    let exe = format!(r"{install_dir}\forge.exe");
    let swap_bat = format!(r"{install_dir}\_forge_swap.cmd");

    // The wait loop uses full paths to tasklist/find/timeout so it keeps
    // working even when the inherited PATH is polluted beyond cmd.exe's
    // ~2047-char batch limit. move/del/start are cmd built-ins.
    let swap_content = format!(
        "@echo off\r\n\
         set /a count=0\r\n\
         :wait\r\n\
         set /a count+=1\r\n\
         if %count% gtr 900 goto abort\r\n\
         %SystemRoot%\\System32\\tasklist.exe /FI \"IMAGENAME eq forge.exe\" 2>nul | %SystemRoot%\\System32\\find.exe /I \"forge.exe\" >nul\r\n\
         if not errorlevel 1 (\r\n\
           %SystemRoot%\\System32\\timeout.exe /t 1 /nobreak >nul\r\n\
           goto wait\r\n\
         )\r\n\
         move /Y \"{new_exe}\" \"{exe}\"\r\n\
         del \"%~f0\"\r\n\
         start \"\" \"{exe}\"\r\n\
         exit /b 0\r\n\
         :abort\r\n\
         del \"%~f0\"\r\n\
         del \"{new_exe}\"\r\n\
         exit /b 1\r\n"
    );

    std::fs::create_dir_all(&install_dir).ok()?;
    let mut swap_file = std::fs::File::create(&swap_bat).ok()?;
    swap_file.write_all(swap_content.as_bytes()).ok()?;

    let ps_script = format!(
        r#"$ErrorActionPreference = 'Stop'
$dir = Join-Path $env:LOCALAPPDATA 'Programs\Forge'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {{ 'aarch64' }} else {{ 'x86_64' }}
$url = 'https://github.com/tailcallhq/forgecode/releases/latest/download/forge-' + $arch + '-pc-windows-msvc.exe'
$new = Join-Path $dir 'forge.exe.new'
Invoke-WebRequest -Uri $url -OutFile $new -UseBasicParsing
$swap = Join-Path $dir '_forge_swap.cmd'
Start-Process -FilePath $swap -WindowStyle Hidden
"#
    );

    let ps_path = format!(r"{install_dir}\forge-update.ps1");
    let mut ps_file = std::fs::File::create(&ps_path).ok()?;
    ps_file.write_all(ps_script.as_bytes()).ok()?;

    Some(format!(
        r#"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass -File "{ps_path}""#
    ))
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

/// Checks if there is an update available
pub async fn on_update(api: Arc<impl API>, update: Option<&Update>) {
    let update = update.cloned().unwrap_or_default();
    let frequency = update.frequency.unwrap_or_default();

    if !should_check_for_updates(&frequency) {
        return;
    }

    let auto_update = update.auto_update.unwrap_or_default();

    // Check if version is development version, in which case we skip the update
    // check
    if VERSION.contains("dev") || VERSION == "0.1.0" {
        // Skip update for development version 0.1.0
        return;
    }

    let informer = update_informer::new(registry::GitHub, "tailcallhq/forgecode", VERSION)
        .interval(frequency.into());

    if let Some(version) = informer.check_version().ok().flatten()
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
}
