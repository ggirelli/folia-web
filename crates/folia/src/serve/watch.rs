//! Watching the site's sources for changes: the whole site (content, templates, Sass and static
//! files are all in it) and the configuration, which may be outside it.
//!
//! What the builder writes is never a change: not the destination, the files beside it the
//! build writes through (`.<name>.folia-new`, `.folia-old`, and its record). Nor is what the
//! build skips ([`Skipped`]): a name starting with `.` (Git's directory, an editor's swap files)
//! unless the configuration includes it, a path it excludes, and an editor's backup and scratch
//! files. A burst of changes (an editor saving several files, a `git checkout`) makes one
//! rebuild: the builder waits for the changes to settle ([`settled_at`]).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use globset::GlobSet;
use notify::event::{AccessKind, MetadataKind, ModifyKind};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Config;
use crate::discover::glob_set;

/// How long the sources must be quiet before a rebuild starts.
pub const QUIET: Duration = Duration::from_millis(200);

/// The longest a burst of changes can put off a rebuild.
pub const MOST: Duration = Duration::from_secs(2);

/// Which paths are the site's sources, and which are not.
#[derive(Clone, Debug)]
pub struct Sources {
    /// The site's directory.
    pub site: PathBuf,
    /// The configuration file.
    pub config: PathBuf,
    /// The destination's directory, and the name of the destination in it.
    pub dest: (PathBuf, String),
    /// What the build skips in the site.
    pub skipped: Skipped,
}

/// What the build skips, by the configuration's `include` and `exclude`: a change to it is no
/// change.
#[derive(Clone, Debug, Default)]
pub struct Skipped {
    include: GlobSet,
    exclude: GlobSet,
    /// The directories and files the build reads whatever `exclude` says: the templates, the
    /// Sass directory, the search script's fragment.
    read: Vec<String>,
}

impl Skipped {
    /// What the build skips with `config`. Invalid patterns skip nothing: the build reports them.
    #[must_use]
    pub fn of(config: &Config) -> Self {
        let mut read = vec![config.templates_dir.clone(), config.sass.dir.clone()];
        read.extend(config.search.iter().map(|search| search.fragment.clone()));
        Self {
            include: glob_set(&config.include).unwrap_or_default(),
            exclude: glob_set(&config.exclude).unwrap_or_default(),
            read: read
                .into_iter()
                .map(|path| path.trim_matches('/').to_owned())
                .collect(),
        }
    }

    /// Whether the build skips `relative`, a path of the site with `/` between its segments.
    #[must_use]
    pub fn skips(&self, relative: &str) -> bool {
        let segments: Vec<&str> = relative.split('/').filter(|s| !s.is_empty()).collect();
        if segments.last().is_some_and(|name| is_scratch(name)) {
            return true;
        }
        let mut path = String::new();
        let mut excluded = false;
        for segment in segments {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(segment);
            if self.read.contains(&path) {
                return false;
            }
            if self.include.is_match(&path) || self.include.is_match(segment) {
                continue;
            }
            if segment.starts_with('.') || self.exclude.is_match(&path) {
                // A directory that holds what the build reads is still walked into.
                let holds_read = self.read.iter().any(|read| {
                    read.strip_prefix(path.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
                });
                if !holds_read {
                    return true;
                }
                excluded = true;
            }
        }
        excluded
    }
}

/// Whether `name` is an editor's backup or scratch file: `name~`, Vim's `4913`, Emacs's
/// `#name#` and `.#name`, Vim's `.name.swp` (the last two also start with `.`).
fn is_scratch(name: &str) -> bool {
    name == "4913"
        || name.ends_with('~')
        || (name.starts_with('#') && name.ends_with('#'))
        || name.starts_with(".#")
}

impl Sources {
    /// Whether a change to `path` is a change to the site's sources. Every path is absolute, its
    /// directories' symbolic links resolved.
    #[must_use]
    pub fn matters(&self, path: &Path) -> bool {
        let (dest_parent, dest_name) = &self.dest;
        if path.starts_with(dest_parent.join(dest_name)) {
            return false;
        }
        let beside_dest = format!(".{dest_name}.folia-");
        let in_dest_parent = path.strip_prefix(dest_parent).ok();
        if in_dest_parent
            .and_then(|inside| inside.components().next())
            .is_some_and(|first| {
                first
                    .as_os_str()
                    .to_string_lossy()
                    .starts_with(&beside_dest)
            })
        {
            return false;
        }
        if path == self.config {
            return true;
        }
        match path.strip_prefix(&self.site) {
            Ok(relative) => !self.skipped.skips(&relative.to_string_lossy()),
            Err(_) => false,
        }
    }

    /// The first path an event names whose change matters, if any: reading a file, or its access
    /// time changing, never does. When the watcher lost events, the whole site may have changed.
    #[must_use]
    pub fn changed(&self, event: &Event) -> Option<PathBuf> {
        if event.need_rescan() {
            return Some(self.site.clone());
        }
        let changes = match event.kind {
            EventKind::Access(AccessKind::Close(notify::event::AccessMode::Write)) => true,
            EventKind::Access(_)
            | EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)) => false,
            _ => true,
        };
        if !changes {
            return None;
        }
        if event.paths.is_empty() {
            return Some(self.site.clone());
        }
        event.paths.iter().find(|path| self.matters(path)).cloned()
    }
}

/// When a burst of changes that began at `first`, the last of which came at `last`, is over: when
/// the sources were [`QUIET`] since the last one, or [`MOST`] after the first.
#[must_use]
pub fn settled_at(first: Instant, last: Instant) -> Instant {
    (last + QUIET).min(first + MOST)
}

/// What the watch hears: an event of the watcher, or that it must stop.
enum Message {
    Event(notify::Result<Event>),
    Stop,
}

/// A watch on the site's sources.
pub struct Watch {
    sources: Sources,
    sender: Sender<Message>,
    messages: Receiver<Message>,
    // Kept for as long as the watch: dropping it stops the events.
    _watcher: RecommendedWatcher,
}

/// Stops a [`Watch`] from another thread.
#[derive(Clone)]
pub struct Stopper(Sender<Message>);

impl Stopper {
    /// Makes the watch's [`Watch::next_change`] return `None`.
    pub fn stop(&self) {
        let _ = self.0.send(Message::Stop);
    }
}

impl Watch {
    /// Starts watching `sources`.
    ///
    /// # Errors
    ///
    /// Fails when the site or the configuration's directory cannot be watched.
    pub fn start(sources: Sources) -> notify::Result<Self> {
        let (sender, messages) = channel();
        let events = sender.clone();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = events.send(Message::Event(event));
        })?;
        watcher.watch(&sources.site, RecursiveMode::Recursive)?;
        if !sources.config.starts_with(&sources.site)
            && let Some(dir) = sources.config.parent()
        {
            // The directory, not the file: editors save by replacing the file.
            watcher.watch(dir, RecursiveMode::NonRecursive)?;
        }
        Ok(Self {
            sources,
            sender,
            messages,
            _watcher: watcher,
        })
    }

    /// What stops this watch.
    #[must_use]
    pub fn stopper(&self) -> Stopper {
        Stopper(self.sender.clone())
    }

    /// Changes what the build skips, as its configuration changed.
    pub fn skip(&mut self, skipped: Skipped) {
        self.sources.skipped = skipped;
    }

    /// Waits for a change to the sources and for the burst it begins to settle, and returns the
    /// first path changed; `None` once the watch is stopped. Errors of the watcher are given to
    /// `report`.
    pub fn next_change(&self, report: impl Fn(&notify::Error)) -> Option<PathBuf> {
        let changed = loop {
            match self.messages.recv().ok()? {
                Message::Event(Ok(event)) => {
                    if let Some(path) = self.sources.changed(&event) {
                        break path;
                    }
                }
                Message::Event(Err(error)) => report(&error),
                Message::Stop => return None,
            }
        };
        let first = Instant::now();
        let mut last = first;
        loop {
            let Some(left) = settled_at(first, last).checked_duration_since(Instant::now()) else {
                return Some(changed);
            };
            match self.messages.recv_timeout(left) {
                Ok(Message::Event(Ok(event))) => {
                    if self.sources.changed(&event).is_some() {
                        last = Instant::now();
                    }
                }
                Ok(Message::Event(Err(error))) => report(&error),
                Ok(Message::Stop) | Err(RecvTimeoutError::Disconnected) => return None,
                Err(RecvTimeoutError::Timeout) => return Some(changed),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> Sources {
        Sources {
            site: PathBuf::from("/home/me/site"),
            config: PathBuf::from("/home/me/folia.toml"),
            dest: (PathBuf::from("/home/me"), "out".to_owned()),
            skipped: Skipped::default(),
        }
    }

    #[test]
    fn a_change_in_the_site_or_to_its_configuration_matters() {
        let sources = sources();
        assert!(sources.matters(Path::new("/home/me/site/about.md")));
        assert!(sources.matters(Path::new("/home/me/site/_sass/main.scss")));
        assert!(sources.matters(Path::new("/home/me/site/_folia/templates/page.html")));
        assert!(sources.matters(Path::new("/home/me/folia.toml")));
    }

    #[test]
    fn a_change_outside_the_site_does_not_matter() {
        let sources = sources();
        assert!(!sources.matters(Path::new("/home/me/notes.txt")));
        assert!(!sources.matters(Path::new("/home/me/site-other/a.md")));
        assert!(!sources.matters(Path::new("/home/me/site/.git/index")));
    }

    #[test]
    fn what_the_build_writes_does_not_matter() {
        let sources = sources();
        assert!(!sources.matters(Path::new("/home/me/out")));
        assert!(!sources.matters(Path::new("/home/me/out/index.html")));
        assert!(!sources.matters(Path::new("/home/me/.out.folia-new/index.html")));
        assert!(!sources.matters(Path::new("/home/me/.out.folia-old")));
        assert!(!sources.matters(Path::new("/home/me/.out.folia-output")));
        assert!(!sources.matters(Path::new("/home/me/.out.folia-output.tmp")));
    }

    #[test]
    fn a_destination_in_the_watched_tree_is_not_watched() {
        // `folia serve` refuses a destination inside the site, as `folia build` does; were it
        // there, the builder would still not see its own writes as changes.
        let sources = Sources {
            site: PathBuf::from("/w/site"),
            config: PathBuf::from("/w/site/_folia/folia.toml"),
            dest: (PathBuf::from("/w/site/_folia"), "out".to_owned()),
            skipped: Skipped::default(),
        };
        assert!(!sources.matters(Path::new("/w/site/_folia/out/index.html")));
        assert!(!sources.matters(Path::new("/w/site/_folia/.out.folia-new/a")));
        assert!(sources.matters(Path::new("/w/site/_folia/folia.toml")));
    }

    #[test]
    fn reading_a_source_is_not_a_change() {
        let sources = sources();
        let event = |kind| Event::new(kind).add_path(PathBuf::from("/home/me/site/about.md"));
        let read = event(EventKind::Access(AccessKind::Open(
            notify::event::AccessMode::Read,
        )));
        assert_eq!(sources.changed(&read), None);
        let atime = event(EventKind::Modify(ModifyKind::Metadata(
            MetadataKind::AccessTime,
        )));
        assert_eq!(sources.changed(&atime), None);
        let written = event(EventKind::Modify(ModifyKind::Any));
        assert_eq!(
            sources.changed(&written),
            Some(PathBuf::from("/home/me/site/about.md"))
        );
        let closed = event(EventKind::Access(AccessKind::Close(
            notify::event::AccessMode::Write,
        )));
        assert!(sources.changed(&closed).is_some());
    }

    #[test]
    fn a_burst_settles_once_quiet_or_at_the_latest_after_the_longest_wait() {
        let first = Instant::now();
        assert_eq!(settled_at(first, first), first + QUIET);
        let later = first + Duration::from_millis(500);
        assert_eq!(settled_at(first, later), later + QUIET);
        let much_later = first + MOST;
        assert_eq!(settled_at(first, much_later), first + MOST);
    }

    fn config(extra: &str) -> Config {
        toml::from_str(&format!(
            "time-zone = \"UTC\"\n{extra}\n[roots]\nblog = \"_posts\"\nmicro = \"_micro\"\n\
             garden = \"_garden\"\n[site]\nurl = \"u\"\ntitle = \"t\"\ndescription = \"d\"\n\
             [search]\nsource = \"assets/js/search.js\"\nfragment = \"_folia/search/tail.js\"\n"
        ))
        .unwrap()
    }

    #[test]
    fn editor_scratch_files_and_dotfiles_are_skipped() {
        let skipped = Skipped::of(&config(""));
        for path in [
            ".about.md.swp",
            "blog/.post.md.swx",
            "4913",
            "_posts/4913",
            "about.md~",
            "#about.md#",
            ".#about.md",
            ".git/index",
            ".github/workflows/ci.yml",
        ] {
            assert!(skipped.skips(path), "{path}");
        }
        for path in [
            "about.md",
            "_posts/2026-01-01-a.md",
            "_sass/main.scss",
            "4913.md",
        ] {
            assert!(!skipped.skips(path), "{path}");
        }
    }

    #[test]
    fn what_the_configuration_excludes_is_skipped_unless_the_build_reads_it() {
        let skipped = Skipped::of(&config(
            "include = [\".htaccess\"]\nexclude = [\"vendor\", \"*.txt\", \"_folia\", \"_sass\"]",
        ));
        assert!(skipped.skips("vendor/gem/lib.rb"));
        assert!(skipped.skips("notes.txt"));
        assert!(!skipped.skips("blog/notes.txt"), "`*` stays in one segment");
        assert!(!skipped.skips(".htaccess"), "included");
        assert!(
            !skipped.skips("_folia/templates/page.html"),
            "the templates"
        );
        assert!(
            !skipped.skips("_folia/search/tail.js"),
            "the search fragment"
        );
        assert!(!skipped.skips("_sass/main.scss"), "the Sass directory");
        assert!(skipped.skips("_folia/notes.md"));
    }

    #[test]
    fn a_change_the_site_skips_does_not_matter() {
        let mut sources = sources();
        sources.skipped = Skipped::of(&config("exclude = [\"vendor\"]"));
        assert!(!sources.matters(Path::new("/home/me/site/vendor/a.rb")));
        assert!(!sources.matters(Path::new("/home/me/site/.about.md.swp")));
        assert!(sources.matters(Path::new("/home/me/site/about.md")));
    }

    #[test]
    fn lost_events_are_a_change_to_the_whole_site() {
        let sources = sources();
        let rescan = Event::new(EventKind::Other).set_flag(notify::event::Flag::Rescan);
        assert_eq!(
            sources.changed(&rescan),
            Some(PathBuf::from("/home/me/site"))
        );
        let unnamed = Event::new(EventKind::Any);
        assert_eq!(
            sources.changed(&unnamed),
            Some(PathBuf::from("/home/me/site"))
        );
    }
}
