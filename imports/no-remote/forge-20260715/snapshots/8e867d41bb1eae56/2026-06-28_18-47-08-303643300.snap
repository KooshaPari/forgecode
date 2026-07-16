//! `oci-lottery` — A1.Flex capacity lottery daemon.
//!
//! Loops `oci compute instance launch` across configured regions until
//! capacity is granted (or SIGTERM). On success, fires a hook chain
//! (post-acquire script, webhook, compute-mesh-state.md commit, iMessage).
//!
//! Per Phenotype scripting policy, this is the canonical Rust replacement
//! for prior shell-based "OCI rerun" loops.

mod config;
mod hooks;
mod oci;
mod state;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use rand::Rng;
use std::path::PathBuf;
use std::time::Duration;
use tokio::signal::unix::{SignalKind, signal};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::oci::{instance_public_ip, list_availability_domains, try_launch};
use crate::state::{AcquiredInstance, LotteryState, write_acquired};

#[derive(Debug, Parser)]
#[command(name = "oci-lottery", about = "OCI Always-Free A1.Flex capacity lottery daemon")]
struct Args {
    /// Path to JSON config (regions, OCIDs, shape).
    #[arg(long, env = "OCI_LOTTERY_CONFIG")]
    config: Option<PathBuf>,

    /// Override state file location.
    #[arg(long, env = "OCI_LOTTERY_STATE")]
    state_file: Option<PathBuf>,

    /// Override acquired-instance file location.
    #[arg(long, env = "OCI_LOTTERY_ACQUIRED")]
    acquired_file: Option<PathBuf>,

    /// Path to phenotype-infra repo root, for compute-mesh-state.md update.
    #[arg(long, env = "PHENOTYPE_INFRA_REPO")]
    infra_repo: Option<PathBuf>,

    /// Single attempt and exit (test mode — no loop, no signal handler).
    #[arg(long)]
    once: bool,
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let args = Args::parse();

    let config_path = args
        .config
        .unwrap_or_else(|| home().join(".cloudprovider/oci-lottery.json"));
    let state_path = args
        .state_file
        .unwrap_or_else(|| home().join(".cloudprovider/oci-lottery-state.json"));
    let acquired_path = args
        .acquired_file
        .unwrap_or_else(|| home().join(".cloudprovider/oci-instance.json"));

    let cfg = Config::load_or_default(&config_path)
        .await
        .context("loading config")?;
    let mut st = LotteryState::load(&state_path)
        .await
        .context("loading state")?;
    if st.started_at.is_none() {
        st.started_at = Some(Utc::now());
    }
    st.save(&state_path).await.ok();

    info!(
        regions = ?cfg.regions,
        shape = %cfg.shape,
        ocpus = cfg.ocpus,
        memory_gb = cfg.memory_gb,
        config = %config_path.display(),
        state = %state_path.display(),
        "oci-lottery starting"
    );

    // SIGTERM/SIGINT handlers.
    let mut sigterm = signal(SignalKind::terminate()).context("install SIGTERM handler")?;
    let mut sigint = signal(SignalKind::interrupt()).context("install SIGINT handler")?;

    loop {
        for region in &cfg.regions {
            // Determine ADs to try.
            let ad_indices: Vec<u8> = cfg
                .availability_domains
                .clone()
                .unwrap_or_else(|| vec![1, 2, 3]);

            // Resolve AD names once per region. If lookup fails, skip region.
            let ad_names = match list_availability_domains(&cfg, region).await {
                Ok(names) if !names.is_empty() => names,
                Ok(_) => {
                    warn!(region = %region, "no availability domains returned");
                    continue;
                }
                Err(e) => {
                    warn!(region = %region, error = %e, "AD list failed; skipping region this round");
                    continue;
                }
            };

            for idx in ad_indices.iter().copied() {
                let i = (idx as usize).saturating_sub(1);
                let ad_name = match ad_names.get(i) {
                    Some(n) => n.clone(),
                    None => continue,
                };

                st.attempts += 1;
                st.last_attempt = Some(Utc::now());
                st.last_region = Some(region.clone());
                st.last_ad = Some(idx);

                let attempt_no = st.attempts;
                info!(attempt = attempt_no, region = %region, ad = %ad_name, "launch attempt");

                match try_launch(&cfg, region, &ad_name).await {
                    Ok(out) if out.success => {
                        let ocid = out.instance_ocid.unwrap_or_default();
                        info!(region = %region, ocid = %ocid, "CAPACITY ACQUIRED");
                        let public_ip =
                            instance_public_ip(&cfg, region, &ocid).await.ok().flatten();
                        let acquired = AcquiredInstance {
                            instance_ocid: ocid,
                            region: region.clone(),
                            ad: idx,
                            public_ip,
                            acquired_at: Utc::now(),
                        };
                        write_acquired(&acquired_path, &acquired).await.ok();
                        st.acquired = true;
                        st.last_error = None;
                        st.save(&state_path).await.ok();

                        hooks::fire_all(&acquired, args.infra_repo.as_ref()).await;
                        info!("hook chain complete; exiting");
                        return Ok(());
                    }
                    Ok(out) if out.out_of_capacity => {
                        st.last_error = Some("out-of-capacity".into());
                        info!(region = %region, ad = %ad_name, "out of capacity");
                    }
                    Ok(out) => {
                        let err = if !out.raw_stderr.is_empty() {
                            out.raw_stderr
                        } else {
                            out.raw_stdout
                        };
                        warn!(region = %region, ad = %ad_name, error = %err, "launch failed");
                        st.last_error = Some(err);
                    }
                    Err(e) => {
                        error!(region = %region, error = %e, "oci CLI invocation error");
                        st.last_error = Some(e.to_string());
                    }
                }
                st.save(&state_path).await.ok();

                if args.once {
                    info!("--once flag: exiting after a single attempt");
                    return Ok(());
                }

                // Backoff jitter between attempts.
                let sleep_secs = jitter(cfg.backoff_min_secs, cfg.backoff_max_secs);
                info!(secs = sleep_secs, "backoff");
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(sleep_secs)) => {},
                    _ = sigterm.recv() => { info!("SIGTERM received"); return Ok(()); }
                    _ = sigint.recv()  => { info!("SIGINT received");  return Ok(()); }
                }
            }
        }
    }
}

fn jitter(min: u64, max: u64) -> u64 {
    let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
    if lo == hi {
        return lo;
    }
    rand::thread_rng().gen_range(lo..=hi)
}
