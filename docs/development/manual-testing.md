# Manual testing

How to test Folia by hand: the automated checks, a build of the public example site, and the
pages it writes, checked in a browser. Anyone can do all of it from a clone of this repository.

Real-site parity, comparing Folia's build of the reference site with Jekyll's, is separate and
optional: it needs the site owner's private inputs, which are not in this repository (see
[real-site parity](#real-site-parity-optional)). **A good build of the example site is no proof
of parity with the real site.** It shows that Folia builds every kind of content, not that it
writes what Jekyll wrote.

## Prerequisites

| Tool | Needed for | Check |
|-|-|-|
| Rust, stable, with `rustfmt` and `clippy` (`rustup component add rustfmt clippy`) | building and testing; the workspace uses edition 2024 | `cargo --version`, `cargo clippy --version` |
| [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) (`cargo install cargo-deny --locked`) | `just deny` | `cargo deny --version` |
| [`just`](https://just.systems) | every recipe | `just --version` |
| git | Folia reads dates from the site's git history, so a site must be a git repository | `git --version` |
| [uv](https://docs.astral.sh/uv/) | shellcheck, Ruff and MkDocs, pinned in the justfile and fetched on first use | `uvx --version` |
| Node.js, with `npx` | markdownlint, pinned in the justfile | `node --version` |
| Ruby and Bundler | StandardRB, `just baseline-negative`, and Jekyll baselines | `ruby --version`, `bundle --version` |
| Python 3 | the parity scripts and `just serve` (standard library only) | `python3 --version` |
| [Dart Sass](https://github.com/sass/dart-sass/releases/tag/1.100.0), exactly 1.100.0 | only sites with Sass stylesheets; the example site has none | `sass --version` prints `1.100.0` |
| Docker | only `just toolchain-compare` and `just toolchain-negative` | `docker --version` |

### Installing them on a fresh Ubuntu 24.04

These commands were run, in order, in a new `ubuntu:24.04` container as a non-root user on
2026-11-08. The guide's [automated checks](#automated-checks) and
[example site](#the-example-site) then passed in a fresh clone: `just check` exited 0 (about a
minute after the toolchain was installed), the example site built (18 pages) and `just serve`
served its pages, feeds, sitemap and search script, and answered 404 for a missing page.

**What was tested:** the clone was of the branch `feat/known-links-register` at `450c125`, which
contained the coverage-classification and known-links changes (PRs #32 and #33) before they were
merged; it was **not** a clone of `main`. The tools those PRs changed are not on the path of the
steps above, but this is the commit that was tested.

The system packages gave Ruby 3.2.3 and Node.js 18.19, older than the versions the author uses
(Ruby 3.3, Node 22); StandardRB, markdownlint and everything else passed with them.

```sh
sudo apt-get update
sudo apt-get install -y git curl ca-certificates build-essential pkg-config xz-utils \
  python3 ruby-full nodejs npm
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal && . "$HOME/.cargo/env"
rustup component add rustfmt clippy
curl -sSf https://just.systems/install.sh | bash -s -- --to "$HOME/.local/bin"
curl -LsSf https://astral.sh/uv/install.sh | sh
cargo install cargo-deny --locked                 # compiles it: about a minute
sudo gem install bundler --no-document
curl -sSL -o sass.tgz \
  https://github.com/sass/dart-sass/releases/download/1.100.0/dart-sass-1.100.0-linux-x64.tar.gz
tar xzf sass.tgz && mkdir -p "$HOME/.local/sass" && mv dart-sass/* "$HOME/.local/sass/" \
  && ln -sf "$HOME/.local/sass/sass" "$HOME/.local/bin/sass"
export PATH="$HOME/.local/bin:$PATH"              # also add it to your shell profile
```

Dart Sass is needed only for sites with stylesheets (`just check` does not need it; the ignored
tests do). Docker is not needed for anything in this guide. On another system, install the same
tools by your package manager and check the versions in the table.

Environment variables:

- `FOLIA_SASS`: the Dart Sass program, when it is not `sass` on `PATH`.
- `FOLIA_SITE`, `FOLIA_CONFIG` and the other `FOLIA_*` variables name a real site's private inputs
  (see [a site's private inputs](../development.md#a-sites-private-inputs)). Nothing in this
  guide needs them before [real-site parity](#real-site-parity-optional); a recipe that needs one
  refuses to run without it.

## Automated checks

```sh
just check          # every check below, as CI runs them
```

Or one at a time:

| Command | What it checks |
|-|-|
| `just fmt-check` (`just fmt` fixes) | Rust formatting |
| `just clippy` | Clippy, pedantic, warnings as errors, every target |
| `just test` | every unit and integration test of the workspace |
| `just doc` | the API docs, warnings as errors |
| `just deny` | advisories, licences and sources of the dependencies |
| `just file-length` | no source file over 1,000 lines |
| `just lint-sh`, `just lint-md`, `just lint-rb`, `just lint-py` | shellcheck, markdownlint, StandardRB, Ruff |
| `just baseline-negative`, `just accept-negative` | the baseline and acceptance scripts refuse what they must, on throwaway repositories |
| `just docs-build` | this documentation, with MkDocs in strict mode |

CI runs the same recipes in the jobs [Checks](../development.md#checks) lists, so a branch that
passes `just check` passes CI.

Fixture tests build throwaway sites in the temporary directory, each its own git repository, and
need nothing else. Two are worth knowing by name:

```sh
cargo test -p folia --test example_site       # builds examples/minimal-site (below)
cargo test --workspace --locked -- --ignored  # also compiles Sass: needs Dart Sass 1.100.0
```

## The example site

[`examples/minimal-site`](https://github.com/ggirelli/folia-web/tree/main/examples/minimal-site)
is a small public site with every kind of content Folia builds. Its configuration is
`_folia/folia.toml`, where `folia build` looks by default, and its templates are in
`_folia/templates`.

| Source | What it tests | Output |
|-|-|-|
| `index.md`, `about.md` (with a `redirect_from`) | pages, permalinks, redirect stubs | `index.html`, `about.html`, `me.html`, `redirects.json` |
| `404.html` | an HTML page and the error layout | `404.html` |
| `_posts/*.md` | posts: dates, categories, tags, a table of contents, highlighted code, previous and next | `blog/2026/…/*.html` |
| `blog/index.html`, `blog/categories.html`, `blog/tags.html` | the blog index, category archives, the categories and tags pages (`[blog]`) | `blog/index.html`, `blog/category/*/index.html`, `blog/categories.html`, `blog/tags.html` |
| `_micro/{notes,likes,bookmarks,rsvps}/*.md` | one micro entry of each kind | `micro/`, `likes/`, `bookmarks/`, `rsvps/` |
| `blog/micro.html` | the micro listing (`[[micro.listings]]`) | `micro.html` |
| `_garden/topic/*.md` | garden notes, wiki-links, backlinks, a broken wiki-link | `dg/topic/*.html`, `assets/json/notes_graph.json` |
| `[feeds]` | the Atom feeds | `blog/feed.xml`, `blog/tag/*.xml` (a tag with a space too), `micro/feed.xml`, `feed/micro/*.xml`, `dg/feed.xml`, `feed.xml` (combined; `feed.xml` page) |
| every page | the sitemap | `sitemap.xml` |
| `assets/js/search.js`, `_folia/search/search.js`, `search.html` (`[search]`) | the search script and a page that uses it | `assets/js/search.js`, `search.html` |
| `assets/` | static files | copied as they are |

### Build it

Build into a directory outside the site that is absent, empty, or one a previous Folia build
wrote (see [the destination](../guide/building.md#the-destination)). A failed build exits non-zero
and leaves the destination as it was:

```sh
just build-example /tmp/folia-example
# or: cargo run --locked -p folia -- build --site examples/minimal-site --dest /tmp/folia-example
```

`folia --version` (or `-V`) prints the package version, `folia 0.1.0`.

It prints one warning, which is expected: the example has a broken wiki-link on purpose.

```text
warning: _garden/topic/index.md: the wiki-link [[Missing note]] names no note, so it is shown as broken
Built 18 pages, 1 redirect stubs, 15 generated files and 2 static files into /tmp/folia-example; 1 planned outputs are not built yet.
```

Folia also says which documents it skipped, and why (`skipped: <source>: <reason>`): a document
it does not support yet (a template that fails is an error, not a skip). The example has
none; a new one, after a change, is a bug in that change, or in the example.

### Inspect the output

```sh
find /tmp/folia-example -type f | sort
cat /tmp/folia-example/redirects.json
python3 -m json.tool /tmp/folia-example/assets/json/notes_graph.json
head -20 /tmp/folia-example/sitemap.xml
head -5 /tmp/folia-example/assets/js/search.js
```

What to look at, by feature:

- **Pages:** `about.html` has the page's body; `me.html` is a stub redirecting to
  `https://example.org/about`, the configuration's `[site] url`.
- **Posts:** each post has its date, categories, tags, a table of contents, highlighted code, and
  links to the previous or next post.
- **Micro entries:** each kind has its template (`page.html` for notes, `like.html`,
  `bookmark.html`, `rsvp.html`), and `micro.html` lists all four, newest first.
- **Garden:** `dg/topic/leaf.html` lists `A topic` among its backlinks; `dg/topic/index.html`
  shows `[[Missing note]]` as a broken link; `notes_graph.json` has both notes and their edges.
- **Listings:** `blog/index.html` lists both posts; `blog/category/journal/index.html` lists both,
  `blog/category/notes/index.html` one; `blog/categories.html` and `blog/tags.html` group them.
- **Feeds:** each feed's `<updated>` is its newest entry's date, not the build time; every tag
  has a feed (`two words.xml`, linked as `two%20words.xml`), no category does; `feed.xml` lists the
  newest posts, notes and micro entries together, with summaries and no content.
- **Sitemap:** every page but `404.html`, by URL; a post's `<lastmod>` is its last modification
  (its date until its source is committed).
- **Search:** `assets/js/search.js` starts with `var documents = [`, one record per page and entry,
  followed by `_folia/search/search.js`, unchanged. A listing's body (`/micro`,
  `/blog/categories.html`) is its `body` block's text, without the navigation and footer; a
  redirect stub's is empty.

### Change it

Edit a source, build again, and compare. Folia reads last-modified dates from **committed**
history only, so a page's `modified` date changes only once its source is committed. To try
changes without touching this repository, copy the site and make it its own repository:

```sh
cp -r examples/minimal-site /tmp/my-site
git -C /tmp/my-site init --quiet && git -C /tmp/my-site add . && git -C /tmp/my-site commit --quiet -m site
cargo run --locked -p folia -- build --site /tmp/my-site --dest /tmp/my-site-out
```

To try Sass, add a stylesheet with front matter, such as `assets/css/main.scss` starting with two
`---` lines: Folia compiles it to `main.css`, with its source map, and needs Dart Sass 1.100.0 for it.

## Browser checks

Serve a build locally:

```sh
just serve /tmp/folia-example          # http://127.0.0.1:8000/; just serve DIR PORT for another port
```

`just serve` (`scripts/serve.py`) serves `/about` from `about.html` and `/blog/` from
`blog/index.html`, as the host does, and `404.html` for anything else. It is for local checks only.

- [ ] **Navigation:** every link in the menu opens a page: `/`, `/about`, `/blog/`, `/micro`,
  `/dg/topic/index`, `/blog/categories.html`, `/blog/tags.html`, `/search`.
- [ ] **Links:** a post's previous and next links, the categories in the footer, a garden note's
  wiki-links and backlinks.
- [ ] **Redirects:** `/me` redirects to `https://example.org/about` (the configured site URL, not
  the local server).
- [ ] **Layout:** at a phone's width (the browser's responsive mode) and a desktop's: no page
  scrolls sideways but code blocks.
- [ ] **Assets:** `/assets/css/style.css` is applied (code is coloured), the author's avatar shows
  in a post's byline and as the tab icon; the browser's console and network panel show no 404.
- [ ] **Search:** on `/search`, typing `minijinja` lists the bookmark and the `/micro` listing
  that names it; `/search?q=leaf` searches on
  load. Its results link to the configured site URL, so they leave the local server.
- [ ] **Feeds:** `/blog/feed.xml`, `/micro/feed.xml`, `/feed/micro/likes.xml` and `/dg/feed.xml`
  open as XML.
- [ ] **Sitemap:** `/sitemap.xml` lists the pages, and not `404.html`.
- [ ] **Listings:** `/micro` lists the four micro entries, newest first; `/blog/` both posts.
- [ ] **Error page:** `/no-such-page` shows the 404 page, with status 404.

To look at pages without a browser window, take screenshots with headless Chrome, at a desktop and
a phone width, while `just serve` runs:

```sh
google-chrome --headless=new --window-size=375,800 --screenshot=/tmp/post.png \
  http://127.0.0.1:8000/blog/2026/01/10/hello-folia
```

## Real-site parity (optional)

Only for a developer the site owner has given its private inputs. These are the owner's data:
they live outside this repository, in a directory of their own, and nothing of them is ever
committed here.

### Set up the evidence directory

Keep everything in one directory beside the checkouts, for example `../folia-web-evidence/`:

```text
folia-web-evidence/
  README.md            provenance: where each file came from, what is verified, what is not
  pins.env             the FOLIA_* variables, pointing into this directory
  config/              Folia's configuration for the site (FOLIA_CONFIG)
  ledgers/             the deviation ledger and the expected warnings
  baselines/<label>/   Jekyll baselines (FOLIA_BASELINES)
  references/          the recorded reference build (FOLIA_A0_REFERENCE)
  toolchain/           the Jekyll lockfiles and images (FOLIA_TOOLCHAINS)
  runs/                the reports of the runs that support a decision
```

Check out the reference site (`FOLIA_SITE`) at the baseline's commit, with its Folia templates in
its `_folia/templates`, then load the pins in the shell that runs the recipes:

```sh
set -a; . ../folia-web-evidence/pins.env; set +a
```

### Generate or refresh a Jekyll baseline

```sh
just baseline <label>                          # a new baseline of the site's checkout
just baseline <label> --lockfile <lockfile> --lockfile-sha256 <sha256> --at <time>  # pinned
just baseline-verify <label>                   # build twice, and without the dump plugin, and compare
just compare <a> <b>                           # compare two baselines
```

The [parity tools reference](../migrating/parity-reference.md) explains each step and what a
baseline holds. A baseline is never overwritten: a new commit of the site is a new label.

### Build and compare

```sh
just build /tmp/site-out        # the whole site, with FOLIA_CONFIG
just routes <label>           # every output path, against the baseline ; with
                              # FOLIA_LEDGER, an approved `folia-only` entry allows an output Jekyll never wrote
just markdown <label>         # every Markdown body, with the ledger's D-001 to D-003 
just liquid                   # no Liquid left in the site's Markdown
just pages <label> [out]      # every file Folia writes, against Jekyll's
just determinism [out]        # two builds, byte for byte, and the recorded reference
just accept <label> [out]     # every check, into one evidence folder
```

`just pages` keeps its build and reports in `out` (by default a new directory in the temporary
directory, which it prints):

| File | What it says |
|-|-|
| `dom-report.md`, `dom-report.json` | every whole-document difference, and the ledger's verdict on it |
| `coverage.md`, `coverage.json` | every output of the baseline and its class, the verdicts *clean build* and *exact parity*, and the Folia-only outputs ([coverage classes](../migrating/parity-reference.md#coverage-classes)) |
| `warnings.json` | each warning Folia gave, and the expected-warnings entry that matches it |
| `links.json` | internal links that resolve to nothing |
| `build.json`, `sha256.txt` | the build's record (site and Folia commits, configuration and templates hashes) and manifest |

### The determinism reference

`just determinism` builds the site twice, compares the two builds byte for byte, and compares the
first with a *reference*: a recorded output manifest (`<sha256>  ./<path>`, the `sha256.txt` of a
kept build). The reference defaults to A0's (`FOLIA_A0_REFERENCE`, a slice-era build made when the
check was introduced), which is only the right one for A0's baseline; another is chosen with
`FOLIA_REFERENCE`, declared the baseline's by `FOLIA_REFERENCE_LABEL`.

A reference goes **stale** when Folia learns to write more outputs: the check then reports
"written, not in the reference" for each. That is not a failure of determinism (the two builds
are still identical), and the old entries may all still match, which is evidence nothing in them
changed. Do not let it stay an unexplained failure:

1. Check what the comparison says: if any file is "in the reference, not written" or "bytes
   differ", a Folia change is to approve or fix first.
2. If the differences are only additions, record a new reference from a reviewed build: copy its
   `sha256.txt` into a new directory of the evidence (`references/<label>-<release>/`), beside
   a README that says which Folia commit, site commit, templates, configuration and baseline
   time made it, and why.
3. Point `FOLIA_REFERENCE` at it and `FOLIA_REFERENCE_LABEL` at the baseline's label, in `pins.env`.
4. Never overwrite or delete the old reference: it is history.

From then on, any change to Folia's output bytes fails the check until its reviewed result is
recorded the same way.

### Read the ledgers

- **Deviations** (`FOLIA_LEDGER`): each accepted entry explains one kind of difference from Jekyll,
  on the outputs it lists, bound to the hashes of the baseline's file and of the Folia file that
  was reviewed. A difference no entry explains fails the run. An entry whose output changed is
  **stale**: it fails too, and the owner must review the new output before its hashes are updated.
  Only the owner approves an entry ([owner approvals](../development.md#owner-approvals-in-the-ledger)).
- **Expected warnings** (`FOLIA_EXPECTED_WARNINGS`): each entry matches one warning Folia gives,
  with the decision that makes it expected. A warning no entry matches fails the run, and so does an
  entry no warning matches: remove an entry once its warning is fixed.

### Stale or missing evidence

| Symptom | Cause |
|-|-|
| `FOLIA_CONFIG is not set: …` | the pins are not loaded in this shell |
| `…/BASELINE.json: No such file or directory` | no baseline has that label in `FOLIA_BASELINES` |
| `<site> is at <commit>, but the baseline was built from <commit>` | check out the site at the baseline's commit |
| `<site> has uncommitted changes` | tracked changes in the site; only `_folia/` and the paths of `parity/site-exclusions.txt` may differ |
| `the templates in … have the tree hash …, not the expected …; nothing was built` | the site's templates changed since `FOLIA_TEMPLATES_SHA256` was pinned: review the change, then pin the hash `build.json` records |
| `stale` entries in `dom-report.md` | a listed output's bytes changed; the owner reviews it again |
| an expected warning that matches nothing | the warning was fixed; remove its entry |

### Keep private data out of Git

- Keep the evidence directory outside every repository, and point at it with `pins.env`; never
  copy a site's configuration, ledger, baseline, hash or report into this repository.
- `.gitignore` refuses the paths the reference site's inputs once had here (`parity/filopoe.toml`,
  `parity/deviations.toml`, `parity/evidence/`, `pins.env`, …); it is a safety net, not a place
  to keep them.
- Before committing, read `git status` and `git diff --staged`: a site's paths, commit IDs or
  hashes in a change are a sign something private slipped in.

## Troubleshooting

| Message | What to do |
|-|-|
| `cannot read <site>/_folia/folia.toml: No such file or directory` | pass `--config`, or put the configuration there |
| `invalid configuration …: unknown field …` | a misspelt or unsupported key: Folia refuses what it does not know |
| `git log -- <file> failed: fatal: not a git repository …` | the site must be a git repository: `git init`, then commit |
| `cannot write the site to <dest>: it is inside the site` | build into a directory outside the site |
| `Dart Sass 1.100.0 is needed to compile the site's stylesheets, and …` | `sass` is missing or another version: install Dart Sass 1.100.0, or set `FOLIA_SASS` to it |
| `… cannot be rendered with <template>: undefined value (in <template>:<line>)` and `The build failed with N problems` | the template reads a key the page does not have: templates are strict; test it with `is defined`. The build fails and the destination is as it was |
| `these templates cannot be used: <template>: missing; used by …` | a page's layout has no template: add it, or fix the page's `layout` |
| `cannot write the site to <dir>: it is not empty, and no record shows a Folia build wrote it` | Folia replaces only what it wrote: choose a new or empty directory, or remove this one yourself |
| `… it holds N entries Folia did not write` | the previous output has files added to it: move them out, or build elsewhere |
| `… .<name>.folia-new exists, left by an interrupted build or not Folia's` | Folia never deletes it: look at it, remove it, and build again |
| `skipped: <source>: only Markdown and HTML pages are built yet` (or another reason) | Folia cannot build that document yet; the rest of the site is still built |
| `warning: <note>: the wiki-link [[…]] names no note, so it is shown as broken` | fix the link, or, for a real site, record it as an expected warning |
| a recipe fails with `uvx`, `npx` or `bundle` not found | install uv, Node.js or Ruby with Bundler ([prerequisites](#prerequisites)) |
