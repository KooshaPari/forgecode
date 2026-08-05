#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::path::Path;
    use std::sync::Mutex;

    use futures::future::BoxFuture;
    use sha2::{Digest, Sha256};
    use url::Url;

    use super::{NativeUpdateResponse, NativeUpdateTransport, install_release_at};
    use crate::native_update::RELEASE_REPOSITORY;

    struct QueueTransport {
        responses: Mutex<VecDeque<Result<NativeUpdateResponse, String>>>,
    }

    impl QueueTransport {
        fn new(responses: impl IntoIterator<Item = Result<NativeUpdateResponse, String>>) -> Self {
            Self { responses: Mutex::new(responses.into_iter().collect()) }
        }
    }

    impl NativeUpdateTransport for QueueTransport {
        fn get<'a>(
            &'a self,
            _url: &'a Url,
        ) -> BoxFuture<'a, Result<NativeUpdateResponse, String>> {
            Box::pin(async move { self.responses.lock().unwrap().pop_front().unwrap() })
        }
    }

    fn response(status: u16, body: impl AsRef<[u8]>) -> NativeUpdateResponse {
        NativeUpdateResponse::new(status, None, body.as_ref().to_vec())
    }

    #[tokio::test]
    async fn checksum_mismatch_preserves_the_existing_executable() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("forge");
        std::fs::write(&destination, b"old executable").unwrap();
        let payload = b"new executable";
        let incorrect_checksum = format!(
            "{}  forge-aarch64-apple-darwin\n",
            Sha256::digest(b"other")
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let transport = QueueTransport::new([
            Ok(response(200, incorrect_checksum)),
            Ok(response(200, payload)),
        ]);

        let result = install_release_at(
            RELEASE_REPOSITORY,
            "2.10.2",
            "aarch64-apple-darwin",
            Path::new(&destination),
            &transport,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(std::fs::read(destination).unwrap(), b"old executable");
    }
}
use std::fs;
use std::io::Write;
use std::path::Path;

use futures::future::BoxFuture;
use reqwest::header::LOCATION;
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

use super::{NativeUpdatePlan, NativeUpdatePlanError};

/// A release binary or checksum is capped at 64 MiB before it is buffered.
/// Forge release binaries are materially smaller; the cap limits a compromised or malformed
/// response without needlessly rejecting a normal debug-symbol-free release.
const MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;
const MAX_REDIRECTS: usize = 3;
const REDIRECT_HOSTS: &[&str] = &[
    "github.com",
    "objects.githubusercontent.com",
    "github-releases.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

/// A fully-buffered response supplied by an updater transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeUpdateResponse {
    status: u16,
    location: Option<String>,
    body: Vec<u8>,
}

impl NativeUpdateResponse {
    #[cfg(test)]
    pub(crate) fn new(status: u16, location: Option<String>, body: Vec<u8>) -> Self {
        Self { status, location, body }
    }
}

/// HTTP boundary for the native updater. Tests provide deterministic queued responses.
pub(crate) trait NativeUpdateTransport: Send + Sync {
    fn get<'a>(&'a self, url: &'a Url) -> BoxFuture<'a, Result<NativeUpdateResponse, String>>;
}

/// Reqwest transport with redirect following disabled so every redirect is validated by the
/// updater rather than implicitly trusted by the client.
pub(crate) struct ReqwestNativeUpdateTransport {
    client: reqwest::Client,
}

impl ReqwestNativeUpdateTransport {
    pub(crate) fn new() -> Result<Self, NativeUpdateError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| NativeUpdateError::Transport)?;
        Ok(Self { client })
    }
}

impl NativeUpdateTransport for ReqwestNativeUpdateTransport {
    fn get<'a>(&'a self, url: &'a Url) -> BoxFuture<'a, Result<NativeUpdateResponse, String>> {
        Box::pin(async move {
            let response = self
                .client
                .get(url.clone())
                .send()
                .await
                .map_err(|_| "request failed".to_string())?;
            let status = response.status().as_u16();
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            if response
                .content_length()
                .is_some_and(|length| length > MAX_DOWNLOAD_BYTES as u64)
            {
                return Err("response too large".to_string());
            }

            let mut response = response;
            let mut body = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "response read failed".to_string())?
            {
                if body.len().saturating_add(chunk.len()) > MAX_DOWNLOAD_BYTES {
                    return Err("response too large".to_string());
                }
                body.extend_from_slice(&chunk);
            }
            Ok(NativeUpdateResponse { status, location, body })
        })
    }
}

#[derive(Debug, Error)]
pub(crate) enum NativeUpdateError {
    #[error("native updates are not supported on this target")]
    UnsupportedTarget,
    #[error("native update plan is invalid")]
    InvalidPlan,
    #[error("native update request failed")]
    Transport,
    #[error("native update response had unexpected status {0}")]
    UnexpectedStatus(u16),
    #[error("native update redirect is invalid")]
    InvalidRedirect,
    #[error("native update exceeded the redirect limit")]
    TooManyRedirects,
    #[error("native update response exceeds the 64 MiB limit")]
    ResponseTooLarge,
    #[error("native update checksum sidecar is not UTF-8")]
    SidecarNotUtf8,
    #[error("native update checksum sidecar is malformed")]
    MalformedSidecar,
    #[error("native update checksum did not match the downloaded binary")]
    ChecksumMismatch,
    #[error("native update could not replace the executable")]
    Install,
}

/// Download, validate, and atomically install the release at `destination`.
///
/// This accepts an explicit destination for normal composition and deterministic tests; the
/// production entrypoint below always passes `current_exe()` for the compile-time target.
pub(crate) async fn install_release_at(
    repository: &str,
    version: &str,
    target: &str,
    destination: &Path,
    transport: &impl NativeUpdateTransport,
) -> Result<(), NativeUpdateError> {
    let plan = NativeUpdatePlan::new(repository, version, target).map_err(plan_error)?;
    let sidecar = fetch_release_response(plan.checksum_url(), transport).await?;
    let expected = parse_sha256_sidecar(&sidecar)?;
    let payload = fetch_release_response(plan.asset_url(), transport).await?;
    if !verify_sha256(&payload, &expected) {
        return Err(NativeUpdateError::ChecksumMismatch);
    }
    install_verified_binary(&payload, destination).map_err(|_| NativeUpdateError::Install)
}

/// Apply a release to the running native executable on a supported compile-time target.
pub(crate) async fn update_current_executable(
    repository: &str,
    version: &str,
) -> Result<(), NativeUpdateError> {
    let target = compile_time_target()?;
    let destination = std::env::current_exe().map_err(|_| NativeUpdateError::Install)?;
    let transport = ReqwestNativeUpdateTransport::new()?;
    install_release_at(repository, version, target, &destination, &transport).await
}

async fn fetch_release_response(
    initial_url: &Url,
    transport: &impl NativeUpdateTransport,
) -> Result<Vec<u8>, NativeUpdateError> {
    if !is_initial_release_url(initial_url) {
        return Err(NativeUpdateError::InvalidPlan);
    }
    let mut current = initial_url.clone();
    let mut redirects = 0;
    loop {
        let response = transport.get(&current).await.map_err(|_| NativeUpdateError::Transport)?;
        if response.status == 200 {
            if response.body.len() > MAX_DOWNLOAD_BYTES {
                return Err(NativeUpdateError::ResponseTooLarge);
            }
            return Ok(response.body);
        }
        if !is_redirect_status(response.status) {
            return Err(NativeUpdateError::UnexpectedStatus(response.status));
        }
        if redirects == MAX_REDIRECTS {
            return Err(NativeUpdateError::TooManyRedirects);
        }
        current = resolve_redirect(&current, response.location.as_deref())?;
        redirects += 1;
    }
}

fn is_initial_release_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

fn resolve_redirect(current: &Url, location: Option<&str>) -> Result<Url, NativeUpdateError> {
    let location = location.filter(|value| !value.is_empty()).ok_or(NativeUpdateError::InvalidRedirect)?;
    let redirect = current.join(location).map_err(|_| NativeUpdateError::InvalidRedirect)?;
    if redirect.scheme() != "https"
        || !redirect.username().is_empty()
        || redirect.password().is_some()
        || redirect.port().is_some()
        || !redirect
            .host_str()
            .is_some_and(|host| REDIRECT_HOSTS.contains(&host))
    {
        return Err(NativeUpdateError::InvalidRedirect);
    }
    Ok(redirect)
}

fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn parse_sha256_sidecar(contents: &[u8]) -> Result<[u8; 32], NativeUpdateError> {
    let contents = std::str::from_utf8(contents).map_err(|_| NativeUpdateError::SidecarNotUtf8)?;
    let line = contents.strip_suffix('\n').unwrap_or(contents);
    if line.is_empty() || line.contains('\n') {
        return Err(NativeUpdateError::MalformedSidecar);
    }
    let mut parts = line.split_ascii_whitespace();
    let hash = parts.next().ok_or(NativeUpdateError::MalformedSidecar)?;
    let filename = parts.next().ok_or(NativeUpdateError::MalformedSidecar)?;
    if parts.next().is_some() || filename.is_empty() || hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(NativeUpdateError::MalformedSidecar);
    }
    let mut digest = [0u8; 32];
    for (index, chunk) in hash.as_bytes().chunks_exact(2).enumerate() {
        digest[index] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap_or_default(), 16)
            .map_err(|_| NativeUpdateError::MalformedSidecar)?;
    }
    Ok(digest)
}

fn verify_sha256(payload: &[u8], expected: &[u8; 32]) -> bool {
    Sha256::digest(payload).as_slice() == expected
}

fn install_verified_binary(payload: &[u8], destination: &Path) -> std::io::Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "executable has no parent")
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(payload)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary.as_file().set_permissions(fs::Permissions::from_mode(0o755))?;
    }
    temporary.as_file().sync_all()?;
    temporary.persist(destination).map_err(|error| error.error)?;
    Ok(())
}

fn plan_error(error: NativeUpdatePlanError) -> NativeUpdateError {
    match error {
        NativeUpdatePlanError::WindowsTargetUnsupported | NativeUpdatePlanError::UnsupportedTarget => {
            NativeUpdateError::UnsupportedTarget
        }
        NativeUpdatePlanError::InvalidVersion => NativeUpdateError::InvalidPlan,
    }
}

fn compile_time_target() -> Result<&'static str, NativeUpdateError> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok("aarch64-apple-darwin");
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok("x86_64-apple-darwin");
    #[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "gnu"))]
    return Ok("aarch64-unknown-linux-gnu");
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    return Ok("x86_64-unknown-linux-gnu");
    #[cfg(all(target_os = "linux", target_arch = "aarch64", target_env = "musl"))]
    return Ok("aarch64-unknown-linux-musl");
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
    return Ok("x86_64-unknown-linux-musl");
    #[allow(unreachable_code)]
    Err(NativeUpdateError::UnsupportedTarget)
}
