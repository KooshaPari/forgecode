//! `byteport` desktop app — Tauri 2.x entry point.
//!
//! Wires the [`byteport_transport::S3UploadTransport`] into a Tauri
//! `State` so the frontend can request upload instructions over the IPC
//! bridge. Tracing is initialised once per process via the
//! `tracing-subscriber` JSON layer (the `tauri-plugin-log` plugin
//! augments this with per-window log capture for the in-app console).

#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::all)]

use std::sync::Arc;

use byteport_transport::{S3UploadTransport, UploadTransport};
use tauri::Manager;

/// Shared application state — kept as an `Arc<dyn UploadTransport>` so
/// additional transports (SOCKS5, WebDAV, …) can be swapped in via
/// feature flags without changing the IPC contract.
pub struct AppState {
    /// The active upload transport.
    pub uploader: Arc<dyn UploadTransport>,
}

/// Read the upload endpoint from `BYTEPORT_UPLOAD_URL` or fall back to the
/// local dev default. Centralised so tests can stub the env.
fn upload_endpoint() -> String {
    std::env::var("BYTEPORT_UPLOAD_URL")
        .unwrap_or_else(|_| "https://uploads.byteport.local".to_string())
}

/// Read the upload bucket from `BYTEPORT_UPLOAD_BUCKET` or fall back to
/// the dev default.
fn upload_bucket() -> String {
    std::env::var("BYTEPORT_UPLOAD_BUCKET")
        .unwrap_or_else(|_| "byteport-uploads".to_string())
}

/// Tauri entry point.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise tracing once per process. `try_init` is a no-op if a
    // subscriber is already installed (e.g. by `cargo test`).
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .try_init();

    let uploader: Arc<dyn UploadTransport> = Arc::new(S3UploadTransport::new(
        upload_endpoint(),
        upload_bucket(),
        Some("desktop"),
    ));

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build())
        .setup(move |app| {
            // Register shared state for IPC handlers.
            app.manage(AppState {
                uploader: Arc::clone(&uploader),
            });

            if cfg!(debug_assertions) {
                // In debug builds, also surface logs into the in-app console
                // window. In release builds, the OS-level logger is the
                // single sink (configured by tauri-plugin-log above).
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ipc::create_upload])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// IPC handler module — kept in its own file as the surface grows.
pub mod ipc {
    use std::sync::Arc;

    use byteport_transport::{UploadRequest, UploadTransport};
    use serde::{Deserialize, Serialize};
    use tauri::State;

    use super::AppState;

    /// IPC request body for `create_upload`.
    #[derive(Debug, Deserialize)]
    pub struct CreateUploadArgs {
        /// Object key (e.g. `/projects/demo/screenshot.png`).
        pub object_key: String,
        /// MIME type (e.g. `image/png`).
        pub content_type: String,
        /// Payload size in bytes.
        pub content_length: u64,
    }

    /// IPC response body for `create_upload`.
    #[derive(Debug, Serialize)]
    pub struct CreateUploadResponse {
        /// HTTP method to use for the upload.
        pub method: String,
        /// Pre-signed URL to `PUT` the bytes to.
        pub url: String,
        /// Headers to attach to the `PUT` (content-type, content-length, etc.).
        pub headers: std::collections::BTreeMap<String, String>,
    }

    /// Tauri command: build a pre-signed upload instruction for the
    /// frontend to perform. Returns the method, URL, and required
    /// headers.
    #[tauri::command]
    pub fn create_upload(
        state: State<'_, AppState>,
        args: CreateUploadArgs,
    ) -> Result<CreateUploadResponse, String> {
        let uploader: Arc<dyn UploadTransport> = Arc::clone(&state.uploader);
        let req = UploadRequest {
            object_key: args.object_key,
            content_type: args.content_type,
            content_length: args.content_length,
        };
        uploader
            .create_upload(&req)
            .map(|instr| CreateUploadResponse {
                method: instr.method,
                url: instr.url,
                headers: instr.headers,
            })
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_endpoint_default() {
        // We don't clear env vars in tests; just assert the function does
        // not panic and returns a non-empty string.
        let ep = upload_endpoint();
        assert!(!ep.is_empty());
    }

    #[test]
    fn upload_bucket_default() {
        let b = upload_bucket();
        assert!(!b.is_empty());
    }
}
