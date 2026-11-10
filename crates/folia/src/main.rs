//! The `folia` command: builds the site, or builds and serves it while it is edited.
//!
//! ```sh
//! cargo run -p folia -- build --site path/to/site --config path/to/folia.toml --dest path/to/out
//! cargo run -p folia -- serve --site path/to/site
//! ```

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use folia::build::{build_as, preview_notice};
use folia::config::Config;
use folia::output::Writer;
use folia_core::content::Preview;
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
        /// Also build documents dated after now. The output is a preview, not for publishing
        #[arg(long)]
        future: bool,
        /// Also build documents whose front matter says `published: false`. The output is a
        /// preview, not for publishing
        #[arg(long)]
        unpublished: bool,
        /// Build as of this moment instead of now, an RFC 3339 timestamp such as
        /// 2026-01-01T00:00:00Z: documents dated after it count as future
        #[arg(long, value_name = "TIME", value_parser = parse_time)]
        at: Option<Timestamp>,
    },
    /// Build the site, serve it on this machine, and build it again when its sources change,
    /// reloading the pages open in a browser
    ///
    /// The site is built whole on every change, with its URL set to the server's, so that its
    /// links stay local. A build that fails is reported in the terminal and over the open pages,
    /// and the last good build is still served. Stop it with Ctrl+C.
    Serve {
        /// The site
        #[arg(long, default_value = ".")]
        site: PathBuf,
        /// Folia's configuration for the site, read again on every build [default:
        /// <SITE>/_folia/folia.toml]
        #[arg(long)]
        config: Option<PathBuf>,
        /// Where to build the site: a directory outside it, absent, empty or written by `folia
        /// serve` (never one `folia build` wrote) [default: folia/serve/<NAME>-<HASH> in the
        /// user's cache directory, kept between runs]
        #[arg(long)]
        dest: Option<PathBuf>,
        /// The address to listen on; any but a loopback address may let other machines see the
        /// site. The site's URL is this address, or the loopback one for 0.0.0.0 or ::
        #[arg(long, default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
        host: IpAddr,
        /// The port to listen on; 0 picks a free one
        #[arg(long, default_value_t = 4000)]
        port: u16,
        /// Also build documents dated after now. The output is a preview, not for publishing
        #[arg(long)]
        future: bool,
        /// Also build documents whose front matter says `published: false`. The output is a
        /// preview, not for publishing
        #[arg(long)]
        unpublished: bool,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Build {
            site,
            config,
            dest,
            future,
            unpublished,
            at,
        } => build_site(
            &site,
            config,
            &dest,
            Preview {
                future,
                unpublished,
            },
            at.unwrap_or_else(Timestamp::now),
        ),
        Command::Serve {
            site,
            config,
            dest,
            host,
            port,
            future,
            unpublished,
        } => {
            let options = folia::serve::Options {
                config: config.unwrap_or_else(|| site.join("_folia/folia.toml")),
                site,
                dest,
                address: SocketAddr::new(host, port),
                preview: Preview {
                    future,
                    unpublished,
                },
            };
            match folia::serve::run(&options) {
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(2)
                }
            }
        }
    }
}

/// Reads the `--at` value, an RFC 3339 timestamp.
fn parse_time(text: &str) -> Result<Timestamp, String> {
    text.parse().map_err(|error| {
        format!(
            "{error}; expected an RFC 3339 timestamp with a time and an offset, \
             such as 2026-01-01T00:00:00Z"
        )
    })
}

fn build_site(
    site: &Path,
    config: Option<PathBuf>,
    dest: &Path,
    preview: Preview,
    now: Timestamp,
) -> ExitCode {
    let config_path = config.unwrap_or_else(|| site.join("_folia/folia.toml"));
    let config = match Config::load(&config_path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    match build_as(site, &config, dest, now, Writer::Build, preview) {
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
            let notice = preview_notice(built.preview);
            if let Some(label) = built.preview.label() {
                eprintln!(
                    "warning: this is a preview build ({label}): it is not for publishing, and \
                     `folia build` will not replace it with a normal build"
                );
            }
            println!(
                "Built {} pages, {} redirect stubs, {} generated files and {} static files into {}; {} planned outputs are not built yet.{}",
                built.pages.len(),
                built.stubs.len(),
                built.artifacts.len(),
                built.statics,
                dest.display(),
                built.not_built,
                notice.unwrap_or_default()
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
