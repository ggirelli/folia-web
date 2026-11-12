# Parity governance

Folia was written to replace one Jekyll site, and its repository holds tools that check Folia's
output against that site's Jekyll build. This page is for maintainers: where the tools live, where
the site's private data lives, and how approvals work. The tools' commands are in the
[parity tools reference](parity-reference.md), and what differs from Jekyll is in
[Differences from Jekyll and known limitations](differences-from-jekyll.md). None of this is
needed to use Folia.

## The parity tooling

**`crates/parity`**: the dev-only parity tool (`just routes`, `just markdown`, `just pages`, `just
  determinism`), and the content lint (`just liquid`). It reads baselines as black boxes and calls
  only Folia's public API. `markdown` holds the Markdown check: `dom` (html5ever parsing and
  normalization), `liquid` (the template filters excerpts go through), `ledger` and `report`.
  `builds` builds the site at an explicit time into a new directory it keeps, with the output's
  manifest and a record of the inputs. It always builds without the preview flags, and its
  `build.json` records them as `preview` (`future` and `unpublished`, both false), so that
  a preview build is never taken for a baseline's equivalent. `document` compares whole HTML
  documents, and trees of them, by their own rules. `pages` builds the site through it and compares what
  Folia writes with Jekyll's files, `pages::entries` the documents' dates, tables of contents and code
  blocks, and `pages::garden` the note graph and the broken-link warnings. `coverage` classifies
  every row of the baseline's manifest from what Folia plans and writes, the checks' verdict on each
  output, and the ledger's coverage entries. `links` checks that every internal `href` and `src` of
  the HTML Folia writes resolves to a planned output, and `warnings` that every warning Folia gives
  is one the site's expected-warnings file expects. `determinism` builds the site twice through
  `builds`, as of one time, and compares the two builds byte for byte, and each with a recorded
  reference.

**`parity/`**: tools that check Folia's output against the Jekyll site.

- `jekyll/folia_dump.rb`: the passive Jekyll plugin that dumps Jekyll's intermediate results (see
    the [parity tools reference](parity-reference.md)). Nothing else may live in
    `jekyll/`, because Jekyll loads every Ruby file there.
- `codemods/check_d001.py`: checks that the only output change of the Liquid rewrite of the
    Liquid in the site's Markdown is the one expected change.
- `check_dump.rb`: checks that a dump accounts for every built file and that no date fell
    back to a file mtime.
- `same_entries.py`: compares the sitemap or the Lunr index entry by entry.
- `diff_regions.py`: lists where two versions of a text file differ, token by token, for the
    toolchain comparison's report.
- `site-exclusions.txt`: the paths in the site that are Folia's (`_folia/`, the content lint's
    workflow), which `scripts/baseline.sh` and the parity tools' checkout check both leave out,
    from this one list.
- `accept.py`: the acceptance runner's verdict: it finds and verifies the baseline's
    lockfile, and judges each acceptance criterion from what the checks left, into
    `accept.json` and `accept.md`.
- `check_date_shift.py`: checks that two baselines differ only by changed last-modified dates.

**Scripts** behind the parity recipes (the rest of `scripts/` is in
[development setup](../contributing/development-setup.md#the-repository)):

- `baseline.sh`: builds a baseline, recording its identities, tools and lockfile; with
  `--lockfile` and `--at`, from exactly that lockfile and as of a pinned time.
- `baseline-container.sh`: builds one with `baseline.sh` in a Docker image pinned by digest.
- `toolchain-compare.sh`: builds one commit with each toolchain and compares them;
  `toolchain-negative.sh` shows the container baselines refuse what they must, in the images.
- `compare-builds.sh`: compares two baselines.
- `verify-baseline.sh`: runs the baseline's acceptance checks on fresh builds, which it keeps;
  an existing baseline is left as it is. `baseline-negative.sh` shows both scripts refuse
  changes outside the approved paths, on throwaway repositories with a stand-in for Jekyll
  (part of `just check-parity`).
- `determinism-negative.sh`: shows the determinism check fails on a changed reference and
  refuses an existing or misplaced directory, a malformed reference and other templates.
- `accept.sh`: the acceptance runner behind `just accept`: runs every check built so far
  against a baseline, in order, into one evidence folder. `accept-negative.sh` shows it fails,
  refuses and records a tree with changes as it should, from a throwaway copy with a stand-in
  for the checks (part of `just check-parity`).
- `determinism-reference.sh`: chooses the determinism reference, fail-closed: A0's for
  `FOLIA_A0_LABEL`, another baseline's only when configured and declared its own (`just determinism`,
  `just determinism-negative`, `just accept`).
- `evidence-provenance.sh`: checks an evidence folder keeps its builds' and baselines' records,
  made from one clean Folia commit, and is (or can be) committed on its own, on that commit; it
  only reads.
  `evidence-provenance-negative.sh` runs it on throwaway repositories, one per way of breaking
  the convention.
- `slice-boundary.sh`: checks a step of the vertical slice keeps out of the other step's mode.

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

The site's evidence (its reviewed outputs, the builds' and baselines'
records), its own negative checks (which change copies of the real site and
baseline) and the tests of its ledgers' contents are kept with them. The `.gitignore` keeps the
reference site's from being added here again.

## Owner approvals in the ledger

The deviation ledger declares, at its top, the `owners`: the only names an approval may give.
A waiver or removal (`check = "coverage"`) classifies an output only when it has
`status = "accepted"`, a valid `accepted` date, a `decision` and a `rationale`, and an
`accepted_by` that is exactly one of `owners`; a missing or empty `owners` approves nothing, and
proposed and withdrawn entries classify nothing.

Three more classes are decided the same way, none of them a waiver
(see the [coverage classes](parity-reference.md#coverage-classes)): `intentional` (a difference
Folia has on purpose; approved, and stale once the output matches Jekyll's), `folia-only` (an
output with no Jekyll baseline, so no manifest row; approved) and `known` (a difference recorded
with `status = "recorded"` and a `rationale`, **never** `accepted`: it has no approval, and the
run still fails while it stands). An entry of the first two with an unapproved status, or a
`known` one that claims acceptance, is invalid and classifies nothing.

**What the tool checks is the recorded approval, not who wrote it.** It cannot tell whether the
owner made or approved a ledger change: anyone who can edit the file can type an owner's name, or
add a name to `owners`. A change to `owners` is visible, not prevented: a test kept with the
site's ledger pins the list, and every coverage report records it. So review must confirm, for
each change to `owners` and
each accepted entry, that the owner actually approved it; an implementer never writes an owner's
name into `accepted_by` or `owners` on their own.
