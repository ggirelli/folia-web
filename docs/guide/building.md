# Building and deploying

## The command

```sh
folia build --site ../my-site --dest ../my-site-out
```

| Option | Default | What |
|-|-|-|
| `--site` | `.` | The site: a git repository |
| `--config` | `<site>/_folia/folia.toml` | The configuration ([reference](../configuration.md)) |
| `--dest` | required | Where to write the site |

`folia --version` prints the version, and `folia build --help` the options. Its
other command is [`folia serve`](#previewing-while-you-write). Both take `--drafts` and
`--future`, which build drafts and scheduled posts for a preview
([below](#previewing-drafts-and-scheduled-posts)). A post dated after the build, or with
`published: false`, is not built.

While it builds, Folia prints **warnings** (a broken wiki-link) and `skipped:` lines for any
document it does not support yet, each with the reason, then `Built N pages, …`.

## Exit status

Scripts can rely on it:

| Status | Meaning | Output |
|-|-|-|
| `0` | The site was built into `--dest`. Warnings and `skipped:` documents do not change it | the warnings and skips on stderr, then `Built …` on stdout |
| `1` | The build failed: planning, a missing or failing template, a render error, an unknown author, a destination Folia refuses, or a write error. **Nothing was written**; `--dest` is as it was | every problem on stderr, then `The build failed with N problems; nothing was written, …` |
| `2` | Usage or configuration error: an unknown subcommand, a missing `--dest`, a configuration that cannot be read or has an unknown key | the error on stderr |

A run that exits 0 may have skipped documents. A script that must not publish a partial site
should check stderr for `skipped:`.

## The destination

`--dest` must be outside the site. Folia writes the new site beside it, in
`.<name>.folia-new`, and swaps it in only when everything was written, so **a failed build never
touches the destination**. It deletes nothing it did not write:

- an absent or empty destination is written;
- a non-empty one is replaced only if its record, `.<name>.folia-output` beside it (not in it, so
  it is never deployed), names that directory and lists every top-level entry it holds. Your own
  files, a moved copy or any other directory are refused, with nothing changed;
- a leftover `.<name>.folia-new` or `.<name>.folia-old` is refused, never deleted.

A preview build (see below) is recorded as one, and `folia build` never replaces a normal build
with a preview, nor a preview with a normal build: it refuses, and nothing is changed. Build the
preview into another directory, or remove the old one yourself. `folia serve` is not bound by this
rule and may switch a destination either way.

To build into a directory that holds something else, empty it yourself.

## Previewing drafts and scheduled posts

Two flags build content that a normal build leaves out, so that you can look at it before it is
published. Both default to off, and a build without them is unchanged. They are command-line
flags on `folia build` and `folia serve`, not configuration keys.

```sh
folia build --site ../my-site --dest ../my-site-preview --drafts --future
```

- **`--drafts`** also builds the posts in `_drafts/` at the site's root, including those in its
  subdirectories. Only Markdown files (`.md`, `.markdown`) are read, as posts: other files
  in `_drafts/` are left out, a `_drafts` directory elsewhere in the site is not read, and the
  configuration's `exclude` still applies. A draft's date is the modification time of its
  file, as a civil date in the site's time zone. Its slug is the file name without its extension,
  so a date at the start of the name stays in the slug (`2026-01-01-idea.md` is
  `2026-01-01-idea`) and does not date the draft; a `slug` in its front matter replaces it. A
  `date` in a draft's front matter is an error, as it is for any post.
- **`--future`** also builds the posts, garden notes and micro entries dated after the build.
- **`published: false`** documents are left out even with `--drafts`. There is no flag for them.
- A modification time is not reproducible: a fresh checkout resets it, and editing a draft
  changes its date and so its URL. Drafts therefore exist only with the flag.

A build with either flag is a **preview build**, and Folia says so in three places:

- the summary line ends with `PREVIEW BUILD (drafts, future): not for publishing.`, naming the
  flags that were set;
- a `warning:` line on stderr says the same;
- the record of the destination, `.<name>.folia-output`, gets a `/folia drafts` and/or a
  `/folia future` line, which is how `folia build` recognizes the directory later (see
  [the destination](#the-destination)).

Keep a preview out of production: deploy only what a normal `folia build` wrote.

## Stylesheets

A `.scss` file with front matter is compiled to CSS (with its source map) when the whole site is
built. Folia compiles it with **Dart Sass 1.100.0 exactly**: `sass` on `PATH`, or the program
named by `FOLIA_SASS`. Another version is refused, and a site without Sass does not need it. The
`[sass]` table sets the import directory (`dir`, default `_sass`) and the output `style`
(`expanded` or `compressed`).

## Dates come from git

A page's last-modified date (the `<lastmod>` of the sitemap, `page.modified`) comes from its last
commit. In a CI checkout, fetch the full history (`fetch-depth: 0` in GitHub Actions): a shallow
clone gives every file the same date.

## Previewing while you write

```sh
folia serve --site ../my-site
```

Folia builds the site, serves it at <http://127.0.0.1:4000/> and builds it again whenever a file
of the site changes. A page open in the browser reloads when the new build is ready. Stop it with
Ctrl+C.

| Option | Default | What |
|-|-|-|
| `--site` | `.` | The site |
| `--config` | `<site>/_folia/folia.toml` | The configuration, read again on every build |
| `--dest` | `folia/serve/<site name>-<hash>` in your cache directory | Where to build the site |
| `--host` | `127.0.0.1` | The address to listen on. Any but a loopback address may let other machines see the site; for `0.0.0.0` or `::`, the site's URL uses the loopback address |
| `--port` | `4000` | The port; `0` picks a free one, and the address is printed |

- **Every change builds the whole site**, with the configuration read again and `site.url` set to
  the local address, so that links stay on the local server. The output therefore differs from
  production's in the URL: `folia build` is still what you deploy.
- **A failed build keeps the last good one.** The build is written as `folia build` writes it, whole
  or not at all. The problems are printed in the terminal and shown over the open pages, which
  clear when a build succeeds.
- **The destination** defaults to `folia/serve/<site name>-<hash>` in your cache directory
  (`$XDG_CACHE_HOME`, or `~/.cache`; `~/Library/Caches` on macOS), and is kept between runs.
  `--dest` follows the rules of [the destination](#the-destination), checked before anything is
  served, with two differences. `folia serve` refuses a destination `folia build` wrote, which you
  may be about to deploy: its record says which command wrote it. And in a destination
  `folia serve` wrote, it recovers what a stopped serve left beside it (`.<name>.folia-new`,
  `.<name>.folia-old`) instead of refusing it. `folia build` may still replace what `folia serve`
  wrote. The destination itself may not be a symbolic link.
- **What is watched**: the whole site, and the configuration's directory if it is outside the site.
  A change to what the build skips starts no build: the destination and the `.folia-*` files
  beside it, names starting with `.` (`.git`, editors' swap files) unless `include` names them,
  paths `exclude` names (except the templates, the Sass directory and the search fragment, which
  the build reads anyway), and editors' backup files (`name~`, `#name#`, Vim's `4913`). Changes
  are gathered: a build starts after 200 ms without one, and at most 2 s after the first.
- **URLs** map to files as a typical host does: `/about` is `about.html`, `/blog/` is
  `blog/index.html`, and a missing page shows the site's `404.html`.
- **Live reload** works by a script added to the HTML as it is served; the files on disk do not
  have it. The page listens to server-sent events at `/__folia/events`.

The [limitations of `folia serve`](../known-limitations.md#folia-serve) are listed there.

## Serving a finished build

The output is plain files, but its links are root-relative (`/about`), so opening the files
directly shows an unstyled site with broken links. Serve the directory with a static server that
maps clean URLs the way your host does: the repository's `scripts/serve.py DIR PORT` does, using
only Python's standard library
(`just serve DIR`). To preview while you write, use `folia serve` instead.

## Deploying

Upload `--dest` to any static host. Folia copies a `CNAME` file like any static file, and writes a
`404.html` if the site has one.

A GitHub Pages workflow that builds with a release binary looks like this (adapt the paths, and pin
actions by commit):

```yaml
- uses: actions/checkout@v5
  with:
    fetch-depth: 0                  # git history: the files' dates
- name: Install Dart Sass 1.100.0
  run: |
    curl -sSL https://github.com/sass/dart-sass/releases/download/1.100.0/dart-sass-1.100.0-linux-x64.tar.gz | tar xz
    echo "$PWD/dart-sass" >> "$GITHUB_PATH"
- name: Install Folia
  run: |
    base=https://github.com/ggirelli/folia-web/releases/download/vX.Y.Z
    file=folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
    curl -sSLO "$base/$file" && curl -sSLO "$base/$file.sha256"
    sha256sum -c "$file.sha256" && tar xzf "$file"
    echo "$PWD/${file%.tar.gz}" >> "$GITHUB_PATH"
- run: folia build --dest _out
- uses: actions/upload-pages-artifact@v5
  with:
    path: _out
```

Add `--site` and `--config` if the site is not the checkout's root or has its configuration
elsewhere. Rehearse a first deployment somewhere that is not production, and keep a way back (see
[Migrating from Jekyll](../migrating/index.md#deploy-and-roll-back)).
