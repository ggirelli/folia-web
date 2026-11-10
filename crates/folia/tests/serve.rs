//! `folia serve` end to end: the binary builds a copy of the example site, serves it on a free
//! port, maps clean URLs and the 404 page as the host does, builds the site again when a source
//! changes, and tells the open pages to reload, or why the build failed.
//!
//! ```sh
//! cargo test -p folia --test serve
//! ```

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long anything the server does may take before the test fails: generous, as a debug build
/// of the site on a busy machine is slow, and the test only waits that long when it fails.
const PATIENCE: Duration = Duration::from_secs(60);

fn example_site() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/minimal-site")
}

/// A scratch directory with a copy of the example site committed to its own repository (the
/// build reads each file's last commit); removed when dropped.
struct Scratch {
    dir: PathBuf,
    site: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("folia-serve-{}-{test}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let site = dir.join("site");
        let copied = Command::new("cp")
            .arg("-r")
            .arg(example_site())
            .arg(&site)
            .status()
            .unwrap();
        assert!(copied.success());
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec!["commit", "--quiet", "-m", "site"],
        ] {
            let ok = Command::new("git")
                .arg("-C")
                .arg(&site)
                .args([
                    "-c",
                    "user.name=Folia",
                    "-c",
                    "user.email=folia@example.com",
                ])
                .args(&args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        }
        Self { dir, site }
    }

    /// `folia serve` on the site, into `dest`, on a free port.
    fn serve(&self, dest: &Path) -> Child {
        Command::new(env!("CARGO_BIN_EXE_folia"))
            .args(["serve", "--port", "0", "--site"])
            .arg(&self.site)
            .arg("--dest")
            .arg(dest)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// `folia serve` running on a [`Scratch`] site, stopped when dropped; what it printed is shown
/// if the test fails.
struct Served<'a> {
    server: Child,
    /// The server's address, `127.0.0.1:<port>`.
    address: String,
    printed: Arc<Mutex<String>>,
    scratch: &'a Scratch,
}

impl<'a> Served<'a> {
    fn start(scratch: &'a Scratch) -> Self {
        let mut server = scratch.serve(&scratch.dir.join("out"));
        let printed = Arc::new(Mutex::new(String::new()));
        let stderr = BufReader::new(server.stderr.take().unwrap());
        let errors = Arc::clone(&printed);
        std::thread::spawn(move || {
            for line in stderr.lines().map_while(Result::ok) {
                errors.lock().unwrap().extend([line.as_str(), "\n"]);
            }
        });
        let mut lines = BufReader::new(server.stdout.take().unwrap()).lines();
        let address = loop {
            let Some(Ok(line)) = lines.next() else {
                panic!(
                    "the server stopped before serving:\n{}",
                    printed.lock().unwrap()
                );
            };
            if let Some((_, rest)) = line.split_once(" at http://") {
                break rest.split('/').next().unwrap().to_owned();
            }
        };
        // Keep reading what the server prints, so that it never blocks on a full pipe.
        let output = Arc::clone(&printed);
        std::thread::spawn(move || {
            for line in lines.map_while(Result::ok) {
                output.lock().unwrap().extend([line.as_str(), "\n"]);
            }
        });
        Self {
            server,
            address,
            printed,
            scratch,
        }
    }

    /// The status and body of `GET path`.
    fn get(&self, path: &str) -> (u16, String) {
        let response = self.request("GET", path);
        let status = response.split(' ').nth(1).unwrap().parse().unwrap();
        let body = response
            .split_once("\r\n\r\n")
            .map_or("", |(_, body)| body)
            .to_owned();
        (status, body)
    }

    /// The whole response to `method path`, headers and all.
    fn request(&self, method: &str, path: &str) -> String {
        let mut stream = self.connect();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    fn connect(&self) -> TcpStream {
        let stream = TcpStream::connect(&self.address).unwrap();
        stream.set_read_timeout(Some(PATIENCE)).unwrap();
        stream
    }

    /// Waits until `GET path` answers a body that `ready` accepts, and returns it.
    fn wait_for(&self, path: &str, ready: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let (_, body) = self.get(path);
            if ready(&body) {
                return body;
            }
            assert!(Instant::now() < deadline, "{path} never changed: {body}");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Opens the event stream of a page served from the version `since`.
    fn events(&self, since: &str) -> TcpStream {
        let mut stream = self.connect();
        write!(
            stream,
            "GET /__folia/events?since={since} HTTP/1.1\r\nHost: {}\r\n\r\n",
            self.address
        )
        .unwrap();
        stream
    }
}

impl Drop for Served<'_> {
    fn drop(&mut self) {
        let _ = self.server.kill();
        let _ = self.server.wait();
        if std::thread::panicking() {
            eprintln!("folia serve printed:\n{}", self.printed.lock().unwrap());
        }
    }
}

/// Runs `folia serve` into `dest`, which it must refuse before serving, and returns what it
/// printed on its standard error.
fn refused(scratch: &Scratch, dest: &Path) -> String {
    let mut server = scratch.serve(dest);
    let deadline = Instant::now() + PATIENCE;
    let status = loop {
        if let Some(status) = server.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = server.kill();
            panic!("folia serve did not refuse {}", dest.display());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut stderr = String::new();
    server
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert_eq!(status.code(), Some(2), "{stderr}");
    stderr
}

/// Reads `stream` until what it sent contains `wanted`, and returns what it sent.
fn read_until(stream: &mut TcpStream, wanted: &str) -> String {
    let mut seen = Vec::new();
    let mut buffer = [0; 4096];
    while !String::from_utf8_lossy(&seen).contains(wanted) {
        let read = stream.read(&mut buffer).unwrap();
        assert!(
            read > 0,
            "the stream ended: {}",
            String::from_utf8_lossy(&seen)
        );
        seen.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8_lossy(&seen).into_owned()
}

/// The version a served page names in its reload script.
fn version(page: &str) -> String {
    let (_, rest) = page.split_once("since=").expect("no reload script");
    rest.split('"').next().unwrap().to_owned()
}

#[test]
fn the_site_is_served_with_clean_urls_its_404_page_and_the_reload_script() {
    let scratch = Scratch::new("urls");
    let served = Served::start(&scratch);
    std::fs::write(served.scratch.dir.join("secret.html"), "secret").unwrap();

    let (status, home) = served.get("/");
    assert_eq!(status, 200);
    assert!(
        home.contains("<title>Home | Folia example</title>"),
        "{home}"
    );
    assert!(home.contains("/__folia/events?since="), "{home}");
    let (status, about) = served.get("/about");
    assert_eq!(status, 200);
    assert_eq!(served.get("/about.html"), (200, about));
    assert_eq!(served.get("/blog").0, 200);
    assert_eq!(served.get("/blog/").0, 200);

    let (status, missing) = served.get("/no/such/page");
    assert_eq!(status, 404);
    assert!(
        missing.contains("<title>404 | Folia example</title>"),
        "{missing}"
    );
    for outside in [
        "/../secret.html",
        "/%2e%2e/secret.html",
        "/blog/..%2f..%2fsecret.html",
        "/..%5csecret.html",
    ] {
        let (status, body) = served.get(outside);
        assert_eq!(status, 404, "{outside}");
        assert!(!body.starts_with("secret"), "{outside}: {body}");
    }

    let (status, css) = served.get("/assets/css/style.css");
    assert_eq!(status, 200);
    assert!(!css.contains("__folia"), "only pages get the script");
    let (_, feed) = served.get("/feed.xml");
    assert!(
        feed.contains(&format!("http://{}/", served.address)),
        "the site's URL is the server's: {feed}"
    );
    let refused = served.request("POST", "/");
    assert!(refused.starts_with("HTTP/1.1 405"), "{refused}");
    assert!(refused.contains("Allow: GET, HEAD"), "{refused}");
    let head = served.request("HEAD", "/__folia/events");
    assert!(
        head.starts_with("HTTP/1.1 200"),
        "a HEAD request is answered at once: {head}"
    );
    assert!(head.contains("text/event-stream"), "{head}");

    let on_disk = std::fs::read_to_string(served.scratch.dir.join("out/index.html")).unwrap();
    assert!(!on_disk.contains("__folia"), "the script is never written");
}

#[test]
fn a_change_rebuilds_the_site_and_a_failed_build_keeps_the_last_good_one() {
    let scratch = Scratch::new("rebuild");
    let served = Served::start(&scratch);
    let page = served.get("/about").1;
    let mut events = served.events(&version(&page));
    read_until(&mut events, "\r\n\r\n");

    let about = served.scratch.site.join("about.md");
    let text = std::fs::read_to_string(&about).unwrap();
    std::fs::write(&about, format!("{text}\nEdited while served.\n")).unwrap();
    let page = served.wait_for("/about", |body| body.contains("Edited while served."));
    read_until(&mut events, "event: reload");

    let config = served.scratch.site.join("_folia/folia.toml");
    let good = std::fs::read_to_string(&config).unwrap();
    let mut events = served.events(&version(&page));
    std::fs::write(&config, format!("{good}\nbogus = true\n")).unwrap();
    let sent = read_until(&mut events, "unknown field `bogus`");
    assert!(sent.contains("event: failed"), "{sent}");
    let (status, still) = served.get("/about");
    assert_eq!(status, 200);
    assert!(still.contains("Edited while served."), "{still}");

    std::fs::write(&config, good).unwrap();
    read_until(&mut events, "event: reload");
}

#[test]
fn a_destination_folia_does_not_own_or_build_wrote_is_refused_before_serving() {
    let scratch = Scratch::new("refused");
    let foreign = scratch.dir.join("mine");
    std::fs::create_dir(&foreign).unwrap();
    std::fs::write(foreign.join("id_rsa"), "key").unwrap();
    let stderr = refused(&scratch, &foreign);
    assert!(
        stderr.contains("Folia only replaces a directory it wrote"),
        "{stderr}"
    );
    assert!(refused(&scratch, &scratch.dir).contains("it holds the site"));

    let built = scratch.dir.join("dist");
    let status = Command::new(env!("CARGO_BIN_EXE_folia"))
        .args(["build", "--site"])
        .arg(&scratch.site)
        .arg("--dest")
        .arg(&built)
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let stderr = refused(&scratch, &built);
    assert!(stderr.contains("`folia build` wrote it"), "{stderr}");
    assert_eq!(
        std::fs::read_to_string(foreign.join("id_rsa")).unwrap(),
        "key"
    );
}

#[test]
fn a_build_left_unfinished_by_a_stopped_serve_is_recovered() {
    let scratch = Scratch::new("recovered");
    drop(Served::start(&scratch));
    // As if stopped between the two renames of the swap: the output moved aside, the new one
    // not yet in its place.
    let new = scratch.dir.join(".out.folia-new");
    let old = scratch.dir.join(".out.folia-old");
    std::fs::create_dir(&new).unwrap();
    std::fs::rename(scratch.dir.join("out"), &old).unwrap();
    let again = Served::start(&scratch);
    assert_eq!(again.get("/about").0, 200);
    assert!(!new.exists());
    assert!(!old.exists());
}
