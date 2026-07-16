//! Benchora CLI
//!
//! Thin command-line surface over the `gauge` (xdd-lib) core. Lets you run a
//! benchmark suite, capture a JSON+Markdown report, store baselines, and
//! compare current runs against a stored baseline.
//!
//! Subcommands
//! -----------
//!
//! * `run`       — run a benchmark suite and write a report
//! * `report`    — read a saved report, summarize to stdout
//! * `baseline`  — promote a report to a named baseline
//! * `compare`   — diff the current report against a stored baseline
//! * `list`      — list stored baselines / reports
//!
//! All subcommands accept `--db <path>` (or the env var `BENCHORA_DB`) to
//! point at a SQLite file that stores baselines and report metadata. The
//! default is `benchora.db` in the current working directory.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub mod baseline;
pub mod compare;
pub mod error;
pub mod mutate;
pub mod report;

pub use error::CliError;

/// Top-level CLI.
#[derive(Parser, Debug)]
#[command(
    name = "benchora",
    version,
    about = "Benchora CLI — benchmark run/report/baseline/compare for the gauge xDD framework."
)]
pub struct Cli {
    /// Path to the Benchora SQLite DB.
    #[arg(
        long,
        env = "BENCHORA_DB",
        default_value = "benchora.db",
        global = true
    )]
    pub db: PathBuf,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Run a benchmark suite and capture a report.
    Run {
        /// Suite name (e.g. `core`, `mutation`, `property`).
        #[arg(long, default_value = "core")]
        suite: String,
        /// Optional output JSON path; defaults to `<suite>-<timestamp>.json`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Summarize a saved report.
    Report {
        /// Path to the report JSON.
        path: PathBuf,
    },
    /// Promote a report to a named baseline.
    Baseline {
        /// Baseline name (e.g. `nightly`, `release-1.0`).
        name: String,
        /// Path to the report JSON to promote.
        #[arg(long)]
        from: PathBuf,
    },
    /// Compare a report against a stored baseline.
    Compare {
        /// Baseline name.
        baseline: String,
        /// Path to the current report.
        #[arg(long)]
        current: PathBuf,
        /// Override the regression threshold (percent) for this run.
        /// Falls back to `BENCHORA_REGRESSION_THRESHOLD_PCT` env var,
        /// then the DB, then 5.0.
        #[arg(long, value_name = "PCT", env = "BENCHORA_REGRESSION_THRESHOLD_PCT")]
        regression_threshold_pct: Option<f64>,
    },
    /// Run mutation testing.
    Mutate {
        /// Package to pass through to the mutation runner.
        #[arg(long)]
        package: Option<String>,
        /// File to pass through to the mutation runner.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Minimum mutation score required for success.
        #[arg(long)]
        min_score: Option<f64>,
    },
    /// List stored baselines / reports.
    List {
        /// What to list.
        #[arg(value_enum, default_value_t = ListKind::Baselines)]
        kind: ListKind,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ListKind {
    Baselines,
    Reports,
}

/// Entry-point invoked by `src/bin/benchora.rs`.
pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.cmd {
        Cmd::Run { suite, out } => report::run_suite(&cli.db, &suite, out.as_deref()),
        Cmd::Report { path } => report::summarize(&path),
        Cmd::Baseline { name, from } => baseline::promote(&cli.db, &name, &from),
        Cmd::Compare {
            baseline,
            current,
            regression_threshold_pct,
        } => {
            // Per-invocation override takes precedence over env/DB/default.
            if let Some(v) = regression_threshold_pct {
                if v.is_finite() {
                    std::env::set_var("BENCHORA_REGRESSION_THRESHOLD_PCT", v.to_string());
                }
            }
            compare::diff(&cli.db, &baseline, &current)
        }
        Cmd::Mutate {
            package,
            file,
            min_score,
        } => mutate::run(package.as_deref(), file.as_deref(), min_score),
        Cmd::List { kind } => match kind {
            ListKind::Baselines => baseline::list(&cli.db),
            ListKind::Reports => report::list(&cli.db),
        },
    }
}
