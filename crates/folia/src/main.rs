//! The `folia` command: builds the site.
//!
//! ```sh
//! cargo run -p folia -- build --site path/to/site --config path/to/folia.toml --dest path/to/out
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use folia::build::build;
use folia::config::Config;
use jiff::Timestamp;

#[derive(Parser)]
#[command(
    version,
    about = "Folia: the static-site generator behind ggirelli.info"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the site into a new or empty directory, or one a previous build wrote
    Build {
        /// The site
        #[arg(long, default_value = ".")]
        site: PathBuf,
        /// Folia's configuration for the site [default: <SITE>/_folia/folia.toml]
        #[arg(long)]
        config: Option<PathBuf>,
        /// Where to write the site: a directory outside it, absent, empty or written by Folia
        #[arg(long)]
        dest: PathBuf,
    },
}

fn main() -> ExitCode {
    let Command::Build { site, config, dest } = Cli::parse().command;
    let config_path = config.unwrap_or_else(|| site.join("_folia/folia.toml"));
    let config = match Config::load(&config_path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    match build(&site, &config, &dest, Timestamp::now()) {
        Ok(built) => {
            for warning in &built.warnings {
                eprintln!("warning: {warning}");
            }
            for skipped in &built.skipped {
                eprintln!("skipped: {skipped}");
            }
            if !built.skipped.is_empty() {
                eprintln!(
                    "{} documents are skipped: Folia does not support them yet (nothing else failed).",
                    built.skipped.len()
                );
            }
            println!(
                "Built {} pages, {} redirect stubs, {} generated files and {} static files into {}; {} planned outputs are not built yet.",
                built.pages.len(),
                built.stubs.len(),
                built.artifacts.len(),
                built.statics,
                dest.display(),
                built.not_built
            );
            ExitCode::SUCCESS
        }
        Err(problems) => {
            for problem in &problems {
                eprintln!("{problem}");
            }
            eprintln!(
                "The build failed with {} problems; nothing was written, and {} is as it was.",
                problems.len(),
                dest.display()
            );
            ExitCode::FAILURE
        }
    }
}
