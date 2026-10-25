//! Parity: checks Folia against a Jekyll baseline of the reference site (plan v3, §9).
//!
//! It reads the baseline as a black box, from the files the spike 0 harness wrote, and never
//! shares code with what it checks beyond calling Folia's public API.
//!
//! ```sh
//! cargo run -p parity -- routes --site ../filopoe --baseline ../folia-baselines/BASELINE-A --config parity/filopoe.toml
//! ```

mod baseline;
mod routes;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Checks Folia against a Jekyll baseline of the reference site")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare Folia's routes with the baseline's: sources, URLs and output files (spike 2)
    Routes {
        /// The Jekyll site
        #[arg(long)]
        site: PathBuf,
        /// A baseline made by scripts/baseline.sh from the same commit
        #[arg(long)]
        baseline: PathBuf,
        /// Folia's configuration for the site
        #[arg(long)]
        config: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Routes {
            site,
            baseline,
            config,
        } => match routes::run(&site, &baseline, &config) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::FAILURE,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        },
    }
}
