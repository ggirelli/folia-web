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
  redirect stubs. `build` is the pipeline (so far for `/about`, one post and two notes, plan v9,
  §12.2), `render` renders pages with MiniJinja from the site's templates (`_folia/templates` in
  the site), and `output` writes the destination whole or not at all.
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
  (plan v12, §5.4, A2.1). `determinism` builds the site twice through `builds`, as of one time,
  and compares the two builds byte for byte, and each with a recorded reference (A2.5).
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
  - `site-exclusions.txt`: the paths in the site that are Folia's (`_folia/`, the content lint's
    workflow), which `scripts/baseline.sh` and the parity tools' checkout check both leave out,
    from this one list.
  - `check_date_shift.py`: checks that two baselines differ only by changed last-modified dates.
  - `deviations.toml`: the deviation ledger; an entry may name the Markdown gates it explains
    (`markdown_gates`). `spike3-expected.txt`: the differences spike 3 expects, each traced to a
    ledger entry.
  - `evidence/`: the reviewed outputs of each spike and rebaseline; from A2 on, each folder
    also keeps its builds' records (`builds/<name>/build.json`, `sha256.txt`), built from a clean
    Folia commit and committed on their own, as its child (plan v12, §5.4).
- **`scripts/`**: the scripts behind the `just` recipes.
  - `baseline.sh`: builds a baseline.
  - `compare-builds.sh`: compares two baselines.
  - `verify-baseline.sh`: runs the spike 0 acceptance checks on fresh builds, which it keeps;
    an existing baseline is left as it is. `baseline-negative.sh` shows both scripts refuse
    changes outside the approved paths, on throwaway repositories with a stand-in for Jekyll
    (part of `just check`).
  - `markdown-negative.sh`, `pages-negative.sh`: show the Markdown and page checks fail when
    they should.
  - `ledger-negative.sh`: shows a DOM ledger entry holds only for the baseline record and files
    it was reviewed against, on copies of the site and baseline, that the templates hash pin
    refuses changed templates, and that a build whose source identity is unknown fails.
  - `coverage-negative.sh`: shows the coverage report fails on waivers and removals no owner
    approved, on a copy of the ledger.
  - `determinism-negative.sh`: shows the determinism check fails on a changed reference and
    refuses an existing or misplaced directory, a malformed reference and other templates.
  - `evidence-provenance.sh`: checks an evidence folder keeps its builds' records, made from one
    clean Folia commit, and is (or can be) committed on its own, on that commit; it only reads.
    `evidence-provenance-negative.sh` runs it on throwaway repositories, one per way of breaking
    the convention.
  - `check-file-length.sh`: enforces the 1,000-line limit.
  - `slice-boundary.sh`: checks a step of the vertical slice keeps out of the other step's mode.
- **`.github/`**: the CI and docs workflows and the pull request template.
- **`docs/`**: this documentation, and the versioned plans in `docs/planning/`.
- **`assets/brand/`**: logos and the banner.

## Owner approvals in the ledger

`parity/deviations.toml` declares, at its top, the `owners`: the only names an approval may give.
A waiver or removal (`check = "coverage"`) classifies an output only when it has
`status = "accepted"`, a valid `accepted` date, a `decision` and a `rationale`, and an
`accepted_by` that is exactly one of `owners`; a missing or empty `owners` approves nothing, and
proposed and withdrawn entries classify nothing (plan v12, §5.2 and §5.4).

**What the tool checks is the recorded approval, not who wrote it.** It cannot tell whether the
owner made or approved a ledger change: anyone who can edit the file can type an owner's name, or
add a name to `owners`. A change to `owners` is visible, not prevented: a test pins the list
(`the_ledger_names_its_owner_and_holds_no_waiver_or_removal` in `crates/parity/src/coverage/`),
and every coverage report records it. So review must confirm, for each change to `owners` and
each accepted entry, that the owner actually approved it; an implementer never writes an owner's
name into `accepted_by` or `owners` on their own.

## Checks

`just check` runs every check; CI runs the same recipes, in three jobs of `.github/workflows/ci.yml`
and one of `.github/workflows/docs.yml`, which also publishes the docs to
[GitHub Pages](https://ggirelli.github.io/folia/) from `main`:

| Job | Recipes |
|-|-|
| `rust` | `fmt-check`, `clippy` (pedantic, warnings are errors), `test`, `doc` |
| `deny` | `cargo deny check` (advisories, licences, duplicate crates, sources) |
| `lint` | `file-length`, `lint-sh` (shellcheck), `lint-md` (markdownlint), `lint-rb` (StandardRB), `lint-py` (Ruff) |
| `docs` | `docs-build` (MkDocs, strict) |
