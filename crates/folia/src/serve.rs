//! `folia serve`: builds the site, serves it over HTTP on the local machine, and builds it again,
//! whole, each time its sources change, telling the open pages to reload.
//!
//! - The site is built as [`build`](crate::build::build) builds it, with one change: its URL is
//!   the server's (`http://127.0.0.1:4000`), so that the links, feeds and redirects it writes stay
//!   local.
//! - It is written to a destination on disk, through [`crate::output`]'s whole-or-nothing swap, and
//!   served from there: a build that fails leaves the last good site in place, and the server
//!   keeps serving it, while the problems are shown in the terminal and over the open pages.
//! - The destination must be one `folia serve` may write ([`output::claim_for_serve`]): never
//!   one that holds files Folia did not write, nor one `folia build` wrote. It is checked before
//!   anything is served, and what an interrupted serve left beside it is recovered.
//! - [`http`] answers the requests, mapping URLs to files as the host does ([`files`]); [`reload`]
//!   is the live reload, whose script goes into the HTML pages as they are served, never into the
//!   files; [`watch`] tells the builder when the sources changed.
//!
//! The destination, unless one is given, is a directory of the user's cache directory named after
//! the site, kept between runs so that a new run replaces the last one's site.

pub mod files;
pub mod http;
pub mod reload;
pub mod watch;

use std::convert::Infallible;
use std::ffi::OsString;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use jiff::Timestamp;
use tiny_http::Server;

use crate::build::build_as;
use crate::config::Config;
use crate::output::{self, Writer};
use reload::Versions;
use watch::{Skipped, Sources, Stopper, Watch};

/// What `folia serve` serves, and where.
#[derive(Clone, Debug)]
pub struct Options {
    /// The site.
    pub site: PathBuf,
    /// Folia's configuration for the site.
    pub config: PathBuf,
    /// Where the site is built; by default, a directory of the user's cache directory.
    pub dest: Option<PathBuf>,
    /// The address to listen on: port 0 picks a free port.
    pub address: SocketAddr,
}

/// Builds and serves the site until the process is stopped; the build's outcomes are printed.
///
/// # Errors
///
/// Fails, before serving anything, when the site or its configuration cannot be read, the
/// destination is not one `folia serve` may write, the address cannot be listened on, or the
/// sources cannot be watched; and later if the server stops. A build that fails is not an error:
/// the server keeps serving the last good site.
pub fn run(options: &Options) -> Result<Infallible, String> {
    let site = options
        .site
        .canonicalize()
        .map_err(|e| format!("cannot read the site {}: {e}", options.site.display()))?;
    let skipped = Skipped::of(&Config::load(&options.config).map_err(|e| e.to_string())?);
    let config = options
        .config
        .canonicalize()
        .map_err(|e| format!("cannot read {}: {e}", options.config.display()))?;
    let dest = match &options.dest {
        Some(dest) => dest.clone(),
        None => default_dest(&site)?,
    };
    let dest = output::claim_for_serve(&dest, &site).map_err(|e| e.to_string())?;
    let (Some(dest_parent), Some(dest_name)) = (dest.parent(), dest.file_name()) else {
        return Err(format!("cannot serve from {}", dest.display()));
    };

    let server = Server::http(options.address)
        .map_err(|e| format!("cannot listen on {}: {e}", options.address))?;
    let address = server
        .server_addr()
        .to_ip()
        .ok_or("the server has no IP address")?;
    let mut watch = Watch::start(Sources {
        site: site.clone(),
        config: config.clone(),
        dest: (
            dest_parent.to_owned(),
            dest_name.to_string_lossy().into_owned(),
        ),
        skipped,
    })
    .map_err(|e| format!("cannot watch {} for changes: {e}", site.display()))?;
    let versions = Arc::new(Versions::new(&server_id()));
    let builder = Builder {
        site,
        config,
        dest,
        url: site_url(address),
    };

    let (error, skipped) = builder.build(true);
    versions.publish(error);
    if let Some(skipped) = skipped {
        watch.skip(skipped);
    }
    println!(
        "Serving {} at {}/ from {} (Ctrl+C to stop)",
        builder.site.display(),
        builder.url,
        builder.dest.display()
    );
    let root = builder.dest.clone();
    let shared = Arc::clone(&versions);
    let stop = StopOnDrop(watch.stopper());
    std::thread::spawn(move || {
        let _stop = stop;
        http::listen(&server, &root, &shared);
    });

    while let Some(changed) = watch.next_change(|e| eprintln!("warning: watching: {e}")) {
        let shown = changed.strip_prefix(&builder.site).unwrap_or(&changed);
        if shown.as_os_str().is_empty() {
            println!("The site may have changed; building again");
        } else {
            println!("Changed: {}; building again", shown.display());
        }
        let (error, skipped) = builder.build(false);
        versions.publish(error);
        if let Some(skipped) = skipped {
            watch.skip(skipped);
        }
    }
    Err("the server stopped".to_owned())
}

/// Stops the watch, and so `folia serve`, when the server's thread ends, even by a panic.
struct StopOnDrop(Stopper);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.stop();
    }
}

/// The site's build, as `folia serve` makes it.
struct Builder {
    site: PathBuf,
    config: PathBuf,
    dest: PathBuf,
    url: String,
}

impl Builder {
    /// Builds the site, its configuration read again, and prints the outcome: the skipped
    /// documents are listed on the `first` build only. Returns the problems if it failed, and
    /// what the configuration, if it could be read, skips.
    fn build(&self, first: bool) -> (Option<String>, Option<Skipped>) {
        let started = Instant::now();
        let mut skipped = None;
        let built = Config::load(&self.config)
            .map_err(|e| vec![e.to_string()])
            .and_then(|mut config| {
                skipped = Some(Skipped::of(&config));
                output::claim_for_serve(&self.dest, &self.site).map_err(|e| vec![e.to_string()])?;
                config.site.url.clone_from(&self.url);
                build_as(
                    &self.site,
                    &config,
                    &self.dest,
                    Timestamp::now(),
                    Writer::Serve,
                )
                .map_err(|problems| problems.iter().map(ToString::to_string).collect())
            });
        let error = match built {
            Ok(built) => {
                for warning in &built.warnings {
                    eprintln!("warning: {warning}");
                }
                if first {
                    for skipped in &built.skipped {
                        eprintln!("skipped: {skipped}");
                    }
                }
                if !built.skipped.is_empty() {
                    eprintln!(
                        "{} documents are skipped: Folia does not support them yet.",
                        built.skipped.len()
                    );
                }
                println!(
                    "Built {} pages in {:.1} s.",
                    built.pages.len(),
                    started.elapsed().as_secs_f64()
                );
                None
            }
            Err(problems) => {
                for problem in &problems {
                    eprintln!("{problem}");
                }
                eprintln!(
                    "The build failed with {}; the last good build, if any, is still served.",
                    count(problems.len(), "problem")
                );
                Some(problems.join("\n"))
            }
        };
        (error, skipped)
    }
}

/// `n` and `noun`, plural unless `n` is 1.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// The site's URL when served at `address`: a loopback address stands for an unspecified one
/// (`0.0.0.0`, `::`), which a browser cannot be sent to.
#[must_use]
pub fn site_url(address: SocketAddr) -> String {
    let ip = match address.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        ip => ip,
    };
    format!("http://{}", SocketAddr::new(ip, address.port()))
}

/// Where the site at `site` is built when no destination is given: `folia/serve/<name>-<hash>`
/// in the user's cache directory, the hash of the site's path telling sites of the same name
/// apart.
///
/// # Errors
///
/// Fails when the environment names no cache directory.
fn default_dest(site: &Path) -> Result<PathBuf, String> {
    let cache = cache_dir(|key| std::env::var_os(key)).ok_or(
        "cannot find the user's cache directory (from XDG_CACHE_HOME or HOME): give --dest",
    )?;
    let name = site
        .file_name()
        .map_or_else(|| "site".into(), |name| name.to_string_lossy());
    let hash = fnv1a(site.as_os_str().as_encoded_bytes());
    Ok(cache
        .join("folia")
        .join("serve")
        .join(format!("{name}-{hash:016x}")))
}

/// The user's cache directory, from the environment `var` reads: `$XDG_CACHE_HOME`, or
/// `$HOME/.cache`; on macOS, `$HOME/Library/Caches`; on Windows, `%LOCALAPPDATA%`. A relative
/// path is ignored.
fn cache_dir(var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let absolute = |key: &str| var(key).map(PathBuf::from).filter(|p| p.is_absolute());
    if cfg!(windows) {
        absolute("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
        absolute("HOME").map(|home| home.join("Library/Caches"))
    } else {
        absolute("XDG_CACHE_HOME").or_else(|| absolute("HOME").map(|home| home.join(".cache")))
    }
}

/// The 64-bit FNV-1a hash of `bytes`: stable across Rust versions and platforms, unlike the
/// standard library's hasher.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Names this server's versions, apart from an earlier server's: the time it started.
fn server_id() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_site_has_its_own_default_destination() {
        let a = default_dest(Path::new("/home/me/site")).unwrap();
        let b = default_dest(Path::new("/home/you/site")).unwrap();
        assert_ne!(a, b);
        assert_eq!(a, default_dest(Path::new("/home/me/site")).unwrap());
        let name = a.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("site-"), "{name}");
        assert!(
            a.parent().unwrap().ends_with("folia/serve"),
            "{}",
            a.display()
        );
    }

    #[test]
    fn the_hash_of_a_path_is_stable() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_cache_directory_is_xdgs_or_in_home() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |key: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| OsString::from(v))
            }
        };
        assert_eq!(
            cache_dir(env(&[("XDG_CACHE_HOME", "/c"), ("HOME", "/h")])),
            Some(PathBuf::from("/c"))
        );
        assert_eq!(
            cache_dir(env(&[("XDG_CACHE_HOME", "relative"), ("HOME", "/h")])),
            Some(PathBuf::from("/h/.cache"))
        );
        assert_eq!(cache_dir(env(&[])), None);
    }

    #[test]
    fn an_unspecified_address_is_served_as_the_loopback_one() {
        let url = |address: &str| site_url(address.parse().unwrap());
        assert_eq!(url("0.0.0.0:4000"), "http://127.0.0.1:4000");
        assert_eq!(url("[::]:4000"), "http://[::1]:4000");
        assert_eq!(url("192.168.1.2:80"), "http://192.168.1.2:80");
        assert_eq!(url("127.0.0.1:4000"), "http://127.0.0.1:4000");
    }

    #[test]
    fn counts_are_plural_unless_one() {
        assert_eq!(count(1, "problem"), "1 problem");
        assert_eq!(count(2, "problem"), "2 problems");
        assert_eq!(count(0, "problem"), "0 problems");
    }
}
