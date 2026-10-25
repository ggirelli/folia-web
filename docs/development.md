# Development

The conventions are in
[CONTRIBUTING.md](https://github.com/ggirelli/folia/blob/main/CONTRIBUTING.md). Run `just` to list
the recipes and `just check` before every commit.

## How the repository is organized

The Cargo workspace follows [plan v4, §4](planning/plan.v4.md#4-workspace-shape-decided-pending-review).
Each mode crate depends on `folia-core` only, never on another mode.

- **`crates/core`** (`folia-core`): the content model.
  - `content::route`: `Route`, a URL and the one policy that maps it to an output file.
  - `content::source`, `content::front_matter`: source documents and their front matter, read
    with Jekyll's conventions.
  - `content::slug`, `content::date`: Jekyll-compatible slugs, the site clock, dated file names.
  - `publish`: `PlannedOutput`, what a mode hands to the site builder.
- **`crates/blog`, `crates/micro`, `crates/garden`, `crates/pages`**: one crate per mode. Each
  module comment holds the mode's route table.
- **`crates/folia`** (`folia`): the site builder. `config` reads `folia.toml`, `discover`
  classifies the site's files, and `site` plans every output in one route table and checks it
  for duplicate URLs and colliding output files.
- **`crates/parity`**: the dev-only parity tool (`just routes`). It reads baselines as black
  boxes and calls only Folia's public API.
- **`parity/`**: tools that check Folia's output against the Jekyll site.
  - `jekyll/folia_dump.rb`: the passive Jekyll plugin that dumps Jekyll's intermediate results
    (see [parity baselines](baseline.md)). Nothing else may live in `jekyll/`, because Jekyll
    loads every Ruby file there.
  - `filopoe.toml`: Folia's configuration for the reference site.
  - `codemods/deliquify.py`: the spike 3 rewrite of the Liquid in the reference site's
    Markdown; `codemods/check_d001.py` checks that its only output change is D-001.
  - `check_dump.rb`: checks that a dump accounts for every built file and that no date fell
    back to a file mtime.
  - `same_entries.py`: compares the sitemap or the Lunr index entry by entry.
  - `check_date_shift.py`: checks that two baselines differ only by changed last-modified dates.
  - `deviations.toml`: the deviation ledger. `spike3-expected.txt`: the differences spike 3
    expects, each traced to a ledger entry.
  - `evidence/`: the reviewed outputs of each spike and rebaseline.
- **`scripts/`**: the scripts behind the `just` recipes.
  - `baseline.sh`: builds a baseline.
  - `compare-builds.sh`: compares two baselines.
  - `verify-baseline.sh`: runs the spike 0 acceptance checks.
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
