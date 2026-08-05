use thiserror::Error;
use url::Url;

mod runtime;

pub(crate) use runtime::update_current_executable;

const RELEASE_REPOSITORY: &str = "KooshaPari/forgecode";

/// A deterministic pair of release URLs for a native Forge update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUpdatePlan {
    asset_url: Url,
    checksum_url: Url,
}

/// Errors returned when a native update plan cannot be safely constructed.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NativeUpdatePlanError {
    /// The supplied version cannot be represented as a release tag.
    #[error("invalid native update version")]
    InvalidVersion,
    /// Windows self-replacement is deliberately not supported.
    #[error("native updates are not supported on Windows")]
    WindowsTargetUnsupported,
    /// The target is not a release asset published by this updater.
    #[error("unsupported native update target")]
    UnsupportedTarget,
}

impl NativeUpdatePlan {
    /// Construct immutable GitHub release and checksum URLs for a supported target and repository.
    ///
    /// # Errors
    ///
    /// Returns an error when the version is unsafe, the target is unsupported, or the target is
    /// Windows, where native replacement is intentionally disabled.
    pub fn new(
        repository: &str,
        version: &str,
        target: &str,
    ) -> Result<Self, NativeUpdatePlanError> {
        let tag = release_tag(version)?;
        let asset = release_asset(target)?;
        let asset_url = Url::parse(&format!(
            "https://github.com/{repository}/releases/download/{tag}/{asset}"
        ))
        .expect("fixed GitHub release URL is valid");
        let checksum_url =
            Url::parse(&format!("{asset_url}.sha256")).expect("fixed GitHub checksum URL is valid");

        Ok(Self { asset_url, checksum_url })
    }

    /// Return the immutable URL for the target-specific release asset.
    pub fn asset_url(&self) -> &Url {
        &self.asset_url
    }

    /// Return the immutable URL for the release asset's SHA-256 sidecar.
    pub fn checksum_url(&self) -> &Url {
        &self.checksum_url
    }
}

fn release_tag(version: &str) -> Result<String, NativeUpdatePlanError> {
    let version = version.strip_prefix('v').unwrap_or(version);
    if version.is_empty()
        || version.starts_with('v')
        || version.len() > 64
        || !version.as_bytes()[0].is_ascii_digit()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err(NativeUpdatePlanError::InvalidVersion);
    }

    Ok(format!("v{version}"))
}

fn release_asset(target: &str) -> Result<&'static str, NativeUpdatePlanError> {
    match target {
        "aarch64-apple-darwin" => Ok("forge-aarch64-apple-darwin"),
        "x86_64-apple-darwin" => Ok("forge-x86_64-apple-darwin"),
        "aarch64-unknown-linux-gnu" => Ok("forge-aarch64-unknown-linux-gnu"),
        "x86_64-unknown-linux-gnu" => Ok("forge-x86_64-unknown-linux-gnu"),
        "aarch64-unknown-linux-musl" => Ok("forge-aarch64-unknown-linux-musl"),
        "x86_64-unknown-linux-musl" => Ok("forge-x86_64-unknown-linux-musl"),
        "aarch64-pc-windows-msvc" | "x86_64-pc-windows-msvc" => {
            Err(NativeUpdatePlanError::WindowsTargetUnsupported)
        }
        _ => Err(NativeUpdatePlanError::UnsupportedTarget),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{NativeUpdatePlan, NativeUpdatePlanError, RELEASE_REPOSITORY};

    #[test]
    fn native_update_plan_normalizes_leading_v_in_versions() {
        let with_prefix =
            NativeUpdatePlan::new(RELEASE_REPOSITORY, "v2.10.2", "aarch64-apple-darwin").unwrap();
        let without_prefix =
            NativeUpdatePlan::new(RELEASE_REPOSITORY, "2.10.2", "aarch64-apple-darwin").unwrap();

        assert_eq!(with_prefix, without_prefix);
    }

    #[test]
    fn native_update_plan_uses_the_override_repository_for_both_release_urls() {
        let fixture =
            NativeUpdatePlan::new("nightly-org/forgecode", "v2.10.2", "aarch64-apple-darwin")
                .unwrap();

        let actual = (
            fixture.asset_url().as_str(),
            fixture.checksum_url().as_str(),
        );
        let release_base = "https://github.com/nightly-org/forgecode/releases/download/v2.10.2";
        let expected = (
            format!("{release_base}/forge-aarch64-apple-darwin"),
            format!("{release_base}/forge-aarch64-apple-darwin.sha256"),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn native_update_plan_builds_immutable_urls_for_supported_targets() {
        for (target, asset) in [
            ("aarch64-apple-darwin", "forge-aarch64-apple-darwin"),
            ("x86_64-apple-darwin", "forge-x86_64-apple-darwin"),
            (
                "aarch64-unknown-linux-gnu",
                "forge-aarch64-unknown-linux-gnu",
            ),
            ("x86_64-unknown-linux-gnu", "forge-x86_64-unknown-linux-gnu"),
            (
                "aarch64-unknown-linux-musl",
                "forge-aarch64-unknown-linux-musl",
            ),
            (
                "x86_64-unknown-linux-musl",
                "forge-x86_64-unknown-linux-musl",
            ),
        ] {
            let plan = NativeUpdatePlan::new(RELEASE_REPOSITORY, "2.10.2", target).unwrap();
            let release_base = "https://github.com/KooshaPari/forgecode/releases/download/v2.10.2";

            assert_eq!(plan.asset_url().as_str(), format!("{release_base}/{asset}"));
            assert_eq!(
                plan.checksum_url().as_str(),
                format!("{release_base}/{asset}.sha256")
            );
        }
    }

    #[test]
    fn native_update_plan_rejects_unsafe_versions() {
        for fixture in ["", "v", "vv2.10.2", "2.10.2/next", "2.10.2?draft=true"] {
            let actual = NativeUpdatePlan::new(RELEASE_REPOSITORY, fixture, "aarch64-apple-darwin");
            let expected = Err(NativeUpdatePlanError::InvalidVersion);

            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn native_update_plan_rejects_windows_targets() {
        for target in ["aarch64-pc-windows-msvc", "x86_64-pc-windows-msvc"] {
            let actual = NativeUpdatePlan::new(RELEASE_REPOSITORY, "v2.10.2", target);
            let expected = Err(NativeUpdatePlanError::WindowsTargetUnsupported);

            assert_eq!(actual, expected, "accepted {target:?}");
        }
    }

    #[test]
    fn native_update_plan_rejects_unknown_targets() {
        let actual = NativeUpdatePlan::new(RELEASE_REPOSITORY, "v2.10.2", "x86_64-unknown-freebsd");
        let expected = Err(NativeUpdatePlanError::UnsupportedTarget);

        assert_eq!(actual, expected);
    }
}
