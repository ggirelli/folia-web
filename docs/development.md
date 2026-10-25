# Development

The conventions are in
[CONTRIBUTING.md](https://github.com/ggirelli/folia/blob/main/CONTRIBUTING.md). Run `just` to list
the recipes and `just check` before every commit.

## How the repository is organized

The Cargo workspace follows [plan v6, §4](planning/plan.v6.md#4-workspace-shape-decided-approved-2026-10-25).
Each mode crate depends on `folia-core` only, never on another mode.

- **`crates/core`** (`folia-core`): the content model.
  - `content::route`: `Route`, a URL and the one policy that maps it to an output file.
  - `content::source`, `content::front_matter`: source documents and their front matter, read
    with Jekyll's conventions.
  - `content::slug`, `content::date`: Jekyll-compatible slugs, the site clock, dated file names.
  - `content::git`: `LastCommits`, each source file's last commit time, for `modified`.
  - `content::markdown`: Markdown to HTML as Jekyll's kramdown renders it, parsed with
    pulldown-cmark and adjusted by passes, one per subject: `wiki_link`, `html` (raw HTML as
    kramdown reads it), `ial`, `lists`, `typography`, `heading` (IDs), `render` (kramdown's HTML
    and layout) and `highlight` (syntect with bat's grammars from `two-face`, mapped to Rouge's
    Pygments classes); and `excerpt`.
  - `publish`: what a mode hands to the site builder: `PlannedOutput` (routes), and `ModeOutput`
    with its `EntrySummary`s and `PageSpec`s.
- **`crates/blog`, `crates/micro`, `crates/garden`, `crates/pages`**: one crate per mode. Each
  module comment holds the mode's route table. `folia_garden::WikiLinks` resolves wiki-links by the
  site's plugin's rules; `folia_pages::build` builds pages, and documents the context their
  templates see; `folia_blog::category_counts` gives the categories every page shows.
- **`crates/folia`** (`folia`): the site builder and the `folia build` command (`main.rs`).
  `config` reads `folia.toml`, `discover` classifies the site's files, and `site` plans every
  output in one route table, checks it for duplicate URLs and colliding output files, and writes
  redirect stubs. `build` is the pipeline (so far for `/about`, plan v6, §12.2), `render` renders
  pages with MiniJinja from the site's templates (`_folia/templates` in the site), and `output`
  writes the destination whole or not at all.
- **`crates/parity`**: the dev-only parity tool (`just routes`, `just markdown`, `just pages`).
  It reads baselines as black boxes and calls only Folia's public API. `markdown` holds spike 1's
  check: `dom` (html5ever parsing and normalization), `liquid` (the template filters excerpts go
  through), `ledger` and `report`. `pages` builds the site and compares what Folia writes with
  Jekyll's files.
- **`parity/`**: tools that check Folia's output against the Jekyll site.
  - `jekyll/folia_dump.rb`: the passive Jekyll plugin that dumps Jekyll's intermediate results
    (see [parity baselines](baseline.md)). Nothing else may live in `jekyll/`, because Jekyll
    loads every Ruby file there.
  - `filopoe.toml`: Folia's configuration for the reference site, until it moves to the site's
    `_folia/folia.toml`.
  - `codemods/deliquify.py`: the spike 3 rewrite of the Liquid in the reference site's
    Markdown; `codemods/check_d001.py` checks that its only output change is D-001.
  - `check_dump.rb`: checks that a dump accounts for every built file and that no date fell
    back to a file mtime.
  - `same_entries.py`: compares the sitemap or the Lunr index entry by entry.
  - `check_date_shift.py`: checks that two baselines differ only by changed last-modified dates.
  - `deviations.toml`: the deviation ledger; an entry may name the Markdown gates it explains
    (`markdown_gates`). `spike3-expected.txt`: the differences spike 3 expects, each traced to a
    ledger entry.
  - `evidence/`: the reviewed outputs of each spike and rebaseline.
- **`scripts/`**: the scripts behind the `just` recipes.
  - `baseline.sh`: builds a baseline.
  - `compare-builds.sh`: compares two baselines.
  - `verify-baseline.sh`: runs the spike 0 acceptance checks.
  - `markdown-negative.sh`, `pages-negative.sh`: show the Markdown and page checks fail when
    they should.
  - `check-file-length.sh`: enforces the 1,000-line limit.
- **`.github/`**: the CI workflow and the pull request template.
- **`docs/`**: this documentation, and the versioned plans in `docs/planning/`.
- **`assets/brand/`**: logos and the banner.

## Checks

`just check` runs every check; CI (`.github/workflows/ci.yml`) runs the same recipes in four jobs:

| Job | Recipes |
|-|-|
| `rust` | `fmt-check`, `clippy` (pedantic, warnings are errors), `test`, `doc` |
| `deny` | `cargo deny check` (advisories, licences, duplicate crates, sources) |
| `lint` | `file-length`, `lint-sh` (shellcheck), `lint-md` (markdownlint), `lint-rb` (StandardRB), `lint-py` (Ruff) |
| `docs` | `docs-build` (MkDocs, strict) |
