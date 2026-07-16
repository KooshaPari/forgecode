//! tailscale-keygen — mint a single-use, ephemeral, tagged Tailscale auth-key
//! and print it to stdout (one line, no trailing whitespace) so it can be
//! captured by `$(tailscale-keygen)` in shell or by the oci-post-acquire hook.
//!
//! Per scripting policy (Rust default), this replaces what would otherwise be
//! a curl + jq one-liner that we'd have to harden against secret leakage,
//! retries, and TTL parsing.
//!
//! Usage:
//!   TS_API_KEY=tskey-api-... TS_TAILNET=koosha.github tailscale-keygen \
//!     --tag tag:phenotype-mesh --tag tag:oci --ttl 600
//!
//! API reference: POST /api/v2/tailnet/{tailnet}/keys
//!   https://tailscale.com/api#tag/keys/POST/api/v2/tailnet/{tailnet}/keys

use anyhow::{Context, Result, bail};
use clap::Parser;
use serde::{Deserialize, Serialize};

/// Mint an ephemeral, single-use, tagged Tailscale auth-key.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Tailscale API key (`tskey-api-...`). Required.
    #[arg(long, env = "TS_API_KEY", hide_env_values = true)]
    api_key: String,

    /// Tailnet name (e.g. `koosha.github` or `example.com`). Required.
    #[arg(long, env = "TS_TAILNET")]
    tailnet: String,

    /// Tags to attach to the new node (repeatable). Must already be declared
    /// in `tagOwners` of the ACL policy. Defaults to `tag:phenotype-mesh`.
    #[arg(long = "tag", default_values = ["tag:phenotype-mesh"])]
    tags: Vec<String>,

    /// Auth-key TTL in seconds. Default 600 (10 min) — long enough for a
    /// post-acquire bootstrap, short enough that a leaked key is near-useless.
    #[arg(long, default_value_t = 600)]
    ttl: u64,

    /// API base (override only for testing).
    #[arg(long, default_value = "https://api.tailscale.com", hide = true)]
    api_base: String,
}

#[derive(Serialize)]
struct CreateKeyRequest {
    capabilities: Capabilities,
    #[serde(rename = "expirySeconds")]
    expiry_seconds: u64,
    description: String,
}

#[derive(Serialize)]
struct Capabilities {
    devices: Devices,
}

#[derive(Serialize)]
struct Devices {
    create: CreateCaps,
}

#[derive(Serialize)]
struct CreateCaps {
    reusable: bool,
    ephemeral: bool,
    preauthorized: bool,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct CreateKeyResponse {
    key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // NOTE: this binary must keep logs off stdout — stdout is the transport
    // for the Tailscale API key (consumed via `$(tailscale-keygen)` in shell or
    // by the oci-post-acquire hook). We still wire to `phenotype-logging` for
    // the default-filter contract (so `RUST_LOG` semantics match the rest of
    // the mesh), but the local `with_writer(std::io::stderr)` override is
    // load-bearing and must stay here.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(phenotype_logging::DEFAULT_FILTER)
                }),
        )
        .with_writer(std::io::stderr) // never to stdout — stdout is the key
        .init();

    let cli = Cli::parse();

    if cli.api_key.is_empty() {
        bail!("TS_API_KEY is empty (set env or pass --api-key)");
    }
    if cli.tailnet.is_empty() {
        bail!("TS_TAILNET is empty (set env or pass --tailnet)");
    }
    if cli.tags.is_empty() {
        bail!("at least one --tag is required");
    }

    let url = format!(
        "{}/api/v2/tailnet/{}/keys",
        cli.api_base.trim_end_matches('/'),
        cli.tailnet
    );

    let body = CreateKeyRequest {
        capabilities: Capabilities {
            devices: Devices {
                create: CreateCaps {
                    reusable: false,     // single-use
                    ephemeral: true,     // node deregistered when offline
                    preauthorized: true, // skip admin review (gated by tags)
                    tags: cli.tags.clone(),
                },
            },
        },
        expiry_seconds: cli.ttl,
        description: format!(
            "phenotype/{} auto-mint by tailscale-keygen",
            cli.tags.first().map(String::as_str).unwrap_or("mesh")
        ),
    };

    tracing::info!(url = %url, tags = ?cli.tags, ttl = cli.ttl, "minting auth-key");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("build reqwest client")?;

    let resp = client
        .post(&url)
        .basic_auth(&cli.api_key, Some(""))
        .json(&body)
        .send()
        .await
        .context("POST /keys")?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        bail!("tailscale API returned {}: {}", status, text);
    }

    let parsed: CreateKeyResponse = resp.json().await.context("decode CreateKeyResponse")?;
    if parsed.key.is_empty() {
        bail!("tailscale API returned empty key");
    }

    // stdout = key only. No newline beyond `println!`'s single \n. Consumers:
    //   AUTH_KEY=$(tailscale-keygen --tag tag:oci)
    println!("{}", parsed.key);
    Ok(())
}
