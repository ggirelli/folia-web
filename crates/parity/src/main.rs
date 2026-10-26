//! Parity: checks Folia against a Jekyll baseline of the reference site (plan v3, §9).
//!
//! It reads the baseline as a black box, from the files the spike 0 harness wrote, and never
//! shares code with what it checks beyond calling Folia's public API.
//!
//! ```sh
//! cargo run -p parity -- routes --site ../filopoe --baseline ../folia-baselines/BASELINE-A --config parity/filopoe.toml
//! cargo run -p parity -- markdown --site ../filopoe --baseline ../folia-baselines/BASELINE-A
//! cargo run -p parity -- pages --site ../filopoe --baseline ../folia-baselines/BASELINE-B --config parity/filopoe.toml
//! cargo run -p parity -- liquid --site ../filopoe --config parity/filopoe.toml
//! ```
//!
//! `liquid` is the exception: a content lint for the site, which needs no baseline.

mod baseline;
mod liquid;
mod markdown;
mod pages;
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
    /// Compare Folia's Markdown HTML and excerpts with kramdown's (spike 1)
    Markdown {
        /// The Jekyll site
        #[arg(long)]
        site: PathBuf,
        /// A baseline made by scripts/baseline.sh from the same commit
        #[arg(long)]
        baseline: PathBuf,
        /// The deviation ledger
        #[arg(long, default_value = "parity/deviations.toml")]
        ledger: PathBuf,
        /// Only this source, such as `pages/profile.md`
        #[arg(long)]
        doc: Option<String>,
        /// Write the full report here
        #[arg(long)]
        report: Option<PathBuf>,
        /// Write both sides' HTML of each differing document below this directory
        #[arg(long)]
        dump: Option<PathBuf>,
    },
    /// Build the site with Folia and compare what it writes with Jekyll's files (vertical slice)
    Pages {
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
    /// Check the site's Markdown has no Liquid outside code, which Folia would show as written
    Liquid {
        /// The Jekyll site
        #[arg(long)]
        site: PathBuf,
        /// Folia's configuration for the site
        #[arg(long)]
        config: PathBuf,
    },
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Routes {
            site,
            baseline,
            config,
        } => routes::run(&site, &baseline, &config),
        Command::Markdown {
            site,
            baseline,
            ledger,
            doc,
            report,
            dump,
        } => markdown::run(&markdown::Options {
            site: &site,
            baseline: &baseline,
            ledger: &ledger,
            only: doc.as_deref(),
            report: report.as_deref(),
            dump: dump.as_deref(),
        }),
        Command::Pages {
            site,
            baseline,
            config,
        } => pages::run(&site, &baseline, &config),
        Command::Liquid { site, config } => liquid::run(&site, &config),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
