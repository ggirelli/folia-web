# Development

The conventions are in
[CONTRIBUTING.md](https://github.com/ggirelli/folia-web/blob/main/CONTRIBUTING.md). Run `just` to list
the recipes and `just check` before every commit. [Manual testing](development/manual-testing.md)
explains how to build the example site, check it in a browser, and run real-site parity.

## How the repository is organized

The Cargo workspace follows [plan v6, §4](planning/plan.v6.md#4-workspace-shape-decided-approved-2026-10-25).
Each mode crate depends on `folia-core` only, never on another mode.

- **`crates/core`** (`folia-core`): the content model.
  - `content::route`: `Route`, a URL and the one policy that maps it to an output file.
  - `content::source`, `content::front_matter`: source documents and their front matter, read
    with Jekyll's conventions.
  - `content::slug`, `content::date`: Jekyll-compatible slugs, the site clock, dated file names.
  - `content::git`: `LastCommits`, each source file's last commit time, for `modified`.
  - `content::modified`: a document's last modification (front matter, then git, then its date)
    and where it came from.
  - `content::presentation`: front matter only templates read (table of contents, cover, the
    frame's switches), resolved with its defaults.
  - `content::markdown`: Markdown to HTML as Jekyll's kramdown renders it, parsed with
    pulldown-cmark and adjusted by passes, one per subject: `wiki_link`, `html` (raw HTML as
    kramdown reads it), `ial`, `lists`, `typography`, `heading` (IDs), `render` (kramdown's HTML
    and layout) and `highlight` (syntect with bat's grammars from `two-face`, mapped to Rouge's
    Pygments classes); `excerpt`, with the page description made of it; and `analyze`, the
    headings and the table of contents built from them.
  - `publish`: what a mode hands to the site builder: `PlannedOutput` (routes), and `ModeOutput`
    with its `EntrySummary`s and `PageSpec`s.
- **`crates/blog`, `crates/micro`, `crates/garden`, `crates/pages`**: one crate per mode. Each
  module comment holds the mode's route table. `folia_garden::WikiLinks` resolves wiki-links by the
  site's plugin's rules, and `folia_garden::build` (module `note`) builds notes' pages, with their
  backlinks, and the note graph; `folia_pages::build` builds pages; `folia_blog::category_counts`
  gives the categories every page shows, and `folia_blog::build` (module `post`) builds posts'
  pages. Each `build` documents the context its templates see.
- **`crates/folia`** (`folia`): the site builder and the `folia build` command (`main.rs`).
  `config` reads `folia.toml`, `discover` classifies the site's files, and `site` plans every
  output in one route table, checks it for duplicate URLs and colliding output files, and writes
  redirect stubs. `build` is the pipeline: of the configuration's slice (plan v9, §12.2), each
  document of which must build, or, without a slice, of every document, skipping those Folia
  cannot build yet. `render` renders pages with MiniJinja from the site's templates
  (`_folia/templates` in the site), `redirects` serializes `/redirects.json` as
  `jekyll-redirect-from` writes it, and `output` writes the destination whole or not at all.
- **`crates/parity`**: the dev-only parity tool (`just routes`, `just markdown`, `just pages`,
  `just determinism`), and the content lint (`just liquid`).
  It reads baselines as black boxes and calls only Folia's public API. `markdown` holds spike 1's
  check: `dom` (html5ever parsing and normalization), `liquid` (the template filters excerpts go
  through), `ledger` and `report`. `builds` builds the site at an explicit time into a new
  directory it keeps, with the output's manifest and a record of the inputs; `document` compares
  whole HTML documents, and trees of them, by plan v12's §5.1 rules. `pages` builds the
  site through it and compares what Folia writes with Jekyll's files, `pages::entries` the
  documents' dates, tables of contents and code blocks, and `pages::garden` the note graph and the
  broken-link warnings. `coverage` classifies every row of the baseline's manifest from what
  Folia plans and writes, the checks' verdict on each output, and the ledger's coverage entries
  (plan v12, §5.4, A2.1). `links` checks that every internal `href` and `src` of the HTML Folia
  writes resolves to a planned output (A2.8), and `warnings` that every warning Folia gives is
  one the site's expected-warnings file expects (A2.9). `determinism` builds the site twice through
  `builds`, as of one time, and compares the two builds byte for byte, and each with a recorded
  reference (A2.5).
- **`parity/`**: tools that check Folia's output against the Jekyll site.
  - `jekyll/folia_dump.rb`: the passive Jekyll plugin that dumps Jekyll's intermediate results
    (see [parity baselines](baseline.md)). Nothing else may live in `jekyll/`, because Jekyll
    loads every Ruby file there.
  - `codemods/check_d001.py`: checks that the only output change of spike 3's rewrite of the
    Liquid in the site's Markdown is D-001.
  - `check_dump.rb`: checks that a dump accounts for every built file and that no date fell
    back to a file mtime.
  - `same_entries.py`: compares the sitemap or the Lunr index entry by entry.
  - `diff_regions.py`: lists where two versions of a text file differ, token by token, for the
    toolchain comparison's report.
  - `site-exclusions.txt`: the paths in the site that are Folia's (`_folia/`, the content lint's
    workflow), which `scripts/baseline.sh` and the parity tools' checkout check both leave out,
    from this one list.
  - `accept.py`: the acceptance runner's verdict (A2.10): it finds and verifies the baseline's
    lockfile, and judges each criterion of plan v12, §8, from what the checks left, into
    `accept.json` and `accept.md`.
  - `check_date_shift.py`: checks that two baselines differ only by changed last-modified dates.
- **`scripts/`**: the scripts behind the `just` recipes.
  - `baseline.sh`: builds a baseline, recording its identities, tools and lockfile; with
    `--lockfile` and `--at`, from exactly that lockfile and as of a pinned time.
  - `baseline-container.sh`: builds one with `baseline.sh` in a Docker image pinned by digest.
  - `toolchain-compare.sh`: builds one commit with each toolchain and compares them (A2.7);
    `toolchain-negative.sh` shows the container baselines refuse what they must, in the images.
  - `compare-builds.sh`: compares two baselines.
  - `verify-baseline.sh`: runs the spike 0 acceptance checks on fresh builds, which it keeps;
    an existing baseline is left as it is. `baseline-negative.sh` shows both scripts refuse
    changes outside the approved paths, on throwaway repositories with a stand-in for Jekyll
    (part of `just check`).
  - `determinism-negative.sh`: shows the determinism check fails on a changed reference and
    refuses an existing or misplaced directory, a malformed reference and other templates.
  - `accept.sh`: the acceptance runner behind `just accept` (A2.10): runs every check built so far
    against a baseline, in order, into one evidence folder. `accept-negative.sh` shows it fails,
    refuses and records a tree with changes as it should, from a throwaway copy with a stand-in
    for the checks (part of `just check`).
  - `determinism-reference.sh`: chooses the determinism reference, fail-closed: A0's for
    `FOLIA_A0_LABEL`, another baseline's only when configured and declared its own (`just determinism`,
    `just determinism-negative`, `just accept`).
  - `evidence-provenance.sh`: checks an evidence folder keeps its builds' and baselines' records,
    made from one clean Folia commit, and is (or can be) committed on its own, on that commit; it
    only reads.
    `evidence-provenance-negative.sh` runs it on throwaway repositories, one per way of breaking
    the convention.
  - `check-file-length.sh`: enforces the 1,000-line limit.
  - `serve.py`: serves a built site locally with the host's clean URLs (`just serve`).
  - `slice-boundary.sh`: checks a step of the vertical slice keeps out of the other step's mode.
- **`.github/`**: the CI and docs workflows, the pull request template, and the Dependabot
  configuration.
- **`docs/`**: this documentation, and the versioned plans in `docs/planning/`.
- **`examples/minimal-site/`**: a small public site with every kind of content Folia builds, for
  manual testing (`just build-example`) and its integration test (`crates/folia/tests`).
- **`assets/brand/`**: logos and the banner.

## Building a site

```sh
cargo run --release -p folia -- build --site ../my-site --dest ../my-site-out
```

Without a `[slice]` in the site's `folia.toml`, the whole site is built; documents Folia cannot
build yet are skipped, each named with the reason.

**Stylesheets need Dart Sass 1.100.0** (plan v12, Q7): the version Jekyll's `sass-embedded`
runs, so that the CSS and its source maps are Jekyll's, byte for byte. Install the standalone
release from <https://github.com/sass/dart-sass/releases/tag/1.100.0> and put its `sass` on
`PATH`, or set `FOLIA_SASS` to the program. A site with Sass stylesheets fails to build, saying
so, when Dart Sass is missing or another version; no other compiler is used. The site's Sass
settings are its configuration's `[sass]` table: `dir`, the directory imports are found in
(default `_sass`), and `style`, `expanded` (default) or `compressed`. Stylesheets are compiled in
whole-site builds only, so a slice's output is unchanged.

**Front matter Folia does not know is refused**, because a misspelt key would otherwise leave a
page silently different from what its author meant. Jekyll ignores such keys, so a published site
can have some. The exceptions are listed, one exact key each, with why, in
`folia_pages::IGNORED_TYPOS`. So far there is one, owner decision Q3h: `narrow-post` (for
`narrow_post`), set to `true` or `false`, on pages only, is accepted and changes nothing. No
template reads it, so Jekyll published the page at its usual width, and Folia builds it the same
way. Posts and other modes still refuse it, and every other unknown key is still refused.

**Authors** are the configuration's `[authors.<key>]` tables (plan v11, Q16b), typed in
`folia::config::Author`, with the fields listed in its module: names, picture, pages, the
author page's backgrounds (`[[authors.<key>.covers]]`) and an account on each service the site
links to. An unset field is left out; an empty one is refused. Templates see every field as
`site.authors.<key>`, and `covers_json`, the backgrounds as the author page's script reads them,
serialized by Folia as Jekyll's `jsonify` wrote them. A page names its author with its front
matter `author`; one that names an author the configuration does not have is not rendered.

**Micro entries** are built by kind, the directory they sit in (`notes`, `likes`, `bookmarks`,
`rsvps`), with the defaults the site's Jekyll configuration gave each kind: layout and category,
the frame's navigation and titles hidden, a corner link back to `/micro`, no previous and next
row, categories and tags hidden (see `folia_micro`'s `entry` module). Their author is the
configuration's `[micro]` `author`, a key of `[authors]`; without one, they have none.

**Micro listings** are the site's pages that list micro entries, which micro *claims* (plan v12,
§3.3–3.4): each `[[micro.listings]]` names its `source`, the `template` that renders it and the
`kinds` it lists. The page keeps its route, redirects and front matter, its body is not
rendered, and the template gets `page.items`, the entries newest first (same dates by path,
descending, as Jekyll sorted them), each with its excerpt, its text and its first 25 words.

**Blog listings** are built by the blog (`folia_blog::listings`) when the configuration's `[blog]`
names them: `index`, the claimed source of `/blog/` (its `source` and `template`), whose pages
list 9 posts each (not `hidden`), the first also the `featured` ones; and `archive-template`, which
renders an archive per category. Each post is a card: its cover, title, the first 30 words of its
excerpt, author and date. Archives are not dark, as the site's front matter defaults never
reached them (`use_dark_layout` false). `[blog]`'s `categories` and `tags` claim the pages listing
the posts by category and by tag (their `source` and `template`): each gets `page.groups`, every
group by name in byte order, with its posts' cards newest first.

**Feeds** are the Atom feeds the modes plan (the blog's, one per plain tag, micro's, one per
micro kind, and the garden's), written by `folia::feed` when the configuration has `[feeds]`:
its `lang`, the `generator` the feeds name (`name`, `uri`, `version`) and how many items each
lists (`limit`, 10). Each mode gives a `FeedItem` per entry it builds; a feed lists the newest of
those whose pages were rendered, serialized as `jekyll-feed` 0.17 wrote them, without a template.
Its `<updated>` is its latest item's, never the build time (plan v9, §7.3). Notes name the
configuration's `[garden]` `author`, as micro entries name `[micro]`'s, and their summaries show
wiki-links resolved (D-003). The combined `/feed.xml` is not built yet.

**The sitemap**, `/sitemap.xml`, is written by `folia::sitemap` as `jekyll-sitemap` 1.4 wrote it:
every page rendered but `/404.html` (redirect stubs are not pages), by URL, then the static
`.htm`, `.html`, `.xhtml` and `.pdf` files. A page's `<lastmod>` is its entry's `modified` or
date, or its claimed source's last commit; a page without a source, such as a category archive,
has none. A static file's is its last commit, where Jekyll used the file's modification time
(plan v12, Q3g).

**The search script** is written by `folia::search` when the configuration has `[search]`: it
claims the script's `source` (`assets/js/lunrsearchengine.js`) and writes, at its route, the
`documents` array the site's Liquid made, followed by `fragment`, the script's code after the
array, read as it is from the site (`_folia/`; plan v12, Q20). The records are the site's pages
by URL (with the stubs and `redirects.json`; not `.xml`, `assets`, `category` or `tag` URLs),
then the RSVPs, likes and bookmarks, the notes and the posts, oldest first, each with its body's
text by the script's filter chain. Following the plan's recommendations, micro entries are listed
(Q3c; an RSVP's body is its answer and event), listing pages show their rendered text (Q3f),
and strings are JSON-escaped, with no `markdownify` (Q3q).

## A site's private inputs

Folia's repository holds the generator and the generic parity tools, never a site's data. What
the parity recipes need of a site is kept by its owner, outside this repository, and named by the
environment; a recipe that needs one refuses to run without it, and `just check` and CI need
none:

| Variable | What it names |
|-|-|
| `FOLIA_SITE`, `FOLIA_BASELINES` | the site's checkout, and where its baselines go |
| `FOLIA_CONFIG` | Folia's configuration for the site (`folia.toml`), with the slice's documents |
| `FOLIA_LEDGER` | the deviation ledger (`deviations.toml`) |
| `FOLIA_EXPECTED_WARNINGS` | the warnings Folia is expected to give, each with its decision |
| `FOLIA_TEMPLATES_SHA256` | the tree hash of the templates the pages check expects |
| `FOLIA_A0_LABEL`, `FOLIA_A0_REFERENCE` | the reference baseline, and its recorded manifest |
| `FOLIA_TOOLCHAINS`, `FOLIA_TOOLCHAIN_AT`, `FOLIA_TOOLCHAIN_DIR` | the toolchain comparison's environments, pinned time and lockfiles |

The site's evidence (each spike's and step's reviewed outputs, the builds' and baselines'
records), its spike 3 list, its own negative checks (which change copies of the real site and
baseline) and the tests of its ledgers' contents are kept with them. The `.gitignore` keeps the
reference site's from being added here again.

## Owner approvals in the ledger

The deviation ledger declares, at its top, the `owners`: the only names an approval may give.
A waiver or removal (`check = "coverage"`) classifies an output only when it has
`status = "accepted"`, a valid `accepted` date, a `decision` and a `rationale`, and an
`accepted_by` that is exactly one of `owners`; a missing or empty `owners` approves nothing, and
proposed and withdrawn entries classify nothing (plan v12, §5.2 and §5.4).

**What the tool checks is the recorded approval, not who wrote it.** It cannot tell whether the
owner made or approved a ledger change: anyone who can edit the file can type an owner's name, or
add a name to `owners`. A change to `owners` is visible, not prevented: a test kept with the
site's ledger pins the list, and every coverage report records it. So review must confirm, for
each change to `owners` and
each accepted entry, that the owner actually approved it; an implementer never writes an owner's
name into `accepted_by` or `owners` on their own.

## Checks

`just check` runs every check; CI runs the same recipes, in three jobs of `.github/workflows/ci.yml`
and one of `.github/workflows/docs.yml`, which also publishes the docs to
[GitHub Pages](https://ggirelli.github.io/folia-web/) from `main`:

| Job | Recipes |
|-|-|
| `rust` | `fmt-check`, `clippy` (pedantic, warnings are errors), `test`, `doc` |
| `deny` | `cargo deny check` (advisories, licences, duplicate crates, sources) |
| `lint` | `file-length`, `lint-sh` (shellcheck), `lint-md` (markdownlint), `lint-rb` (StandardRB), `lint-py` (Ruff) |
| `docs` | `docs-build` (MkDocs, strict) |
