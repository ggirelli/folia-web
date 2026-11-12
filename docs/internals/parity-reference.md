# Parity tools reference

The tools behind [Migrating from Jekyll](../guide/migrating.md): they build your Jekyll site as a recorded
**baseline**, build the same commit with Folia, and compare what each wrote. They live in the
repository (`crates/parity`, `scripts/`, the `just` recipes) and need a clone; they are not part
of the `folia` binary. They are for the **one-off exercise of switching a site**, and nothing
about them is needed to build a Folia site.

## What you need

- A clone of this repository, and the tools in [manual testing](../contributing/manual-testing.md#prerequisites)
  (Rust, `just`, Ruby with Bundler, Python 3, git; Dart Sass if the site has stylesheets).
- Your Jekyll site as a git checkout, with its gems installed (`bundle install`), and **no
  uncommitted changes** other than the paths listed in `parity/site-exclusions.txt` (`_folia/`,
  where Folia's templates and configuration live).
- Folia's inputs for the site, kept **outside this repository** and named by environment
  variables. A recipe that needs one refuses to run without it:

| Variable | What it names |
|-|-|
| `FOLIA_SITE`, `FOLIA_BASELINES` | the site's checkout (default `../filopoe`), and where baselines go (default `../folia-baselines`) |
| `FOLIA_CONFIG` | Folia's configuration for the site (`folia.toml`) |
| `FOLIA_LEDGER` | the deviation ledger: the differences you accept (below) |
| `FOLIA_EXPECTED_WARNINGS` | the warnings Folia is expected to give, each with the reason |
| `FOLIA_TEMPLATES_SHA256` | the tree hash of the templates the comparison expects, so a changed template is noticed |
| `FOLIA_A0_LABEL`, `FOLIA_A0_REFERENCE` | the baseline the determinism check is recorded against, and its recorded manifest |

Keep these, the baselines and every report in a directory of their own, outside both repositories.
They hold your site's data: never commit them. [Manual
testing](../contributing/manual-testing.md#real-site-parity-optional) suggests a layout.

## A baseline

```sh
just baseline my-label                    # a production build of the site's checkout
just baseline-verify my-label             # build twice, and without the dump plugin, and compare
just compare label-a label-b              # two baselines, byte for byte
```

A baseline is `JEKYLL_ENV=production bundle exec jekyll build` of the site at one commit, run with
a passive plugin (`parity/jekyll/folia_dump.rb`) that **records Jekyll's own intermediate results**
beside the site: one row per page, document and static file in a manifest, each document's body
HTML before the layouts, its excerpt, and its resolved dates. The plugin changes nothing about the
output; `baseline-verify` shows it. A baseline is never overwritten: a new commit of the site is a
new label. The script refuses a site with uncommitted changes (`--allow-dirty` records them as
`site.diff`; it still refuses untracked files), and it keeps the lockfile it built with.

| Path | Contents |
|-|-|
| `site/` | the production build |
| `dump/manifest.jsonl` | one row per page, document and static file: its kind, collection, source, URL and output |
| `dump/content/`, `dump/excerpt/`, `dump/meta/` | each document's body and excerpt HTML, and its title, dates, categories and tags |
| `BASELINE.json`, `Gemfile.lock`, `sha256.txt`, `build.log` | what was built and with what, the lockfile used, a hash of every file, Jekyll's output |

`--lockfile FILE --lockfile-sha256 SHA256` builds with exactly that lockfile, and `--at TIME`
(`YYYY-MM-DDTHH:MM:SSZ`) pins Jekyll's `site.time`, which dates the feeds and decides what is
future-dated. Two builds of one commit are byte-identical except for the feeds' `<updated>` (the
build time), which `compare` normalizes.

**Dates must not come from file modification times.** Such a date depends on the checkout, not the
commit, so the baseline would not be reproducible: the build is refused if any date fell back to
one.

## The checks

```sh
just liquid                  # no Liquid left in the site's Markdown
just routes my-label         # every output URL and file, against the baseline's manifest
just markdown my-label       # every Markdown body, against kramdown's HTML
just pages my-label [out]    # every file Folia writes, against Jekyll's
just determinism [out]       # two builds, byte for byte, and the recorded reference
just accept my-label [out]   # all of them, into one evidence folder
```

- **`liquid`** fails on any `{{` or `{%` in the prose of the site's Markdown (code is exempt),
  reported as `file:line:column`. Folia does not run Liquid, so it would show as written. Run it
  before the first build.
- **`routes`** has Folia plan every output of the site and compares the plan with the baseline's
  manifest: authored outputs, each source's kind, generated outputs (pagination, archives, feeds,
  redirect stubs, the sitemap) and redirect stubs. With a ledger, an approved `folia-only` entry
  allows an output Jekyll never wrote.
- **`markdown`** renders each Markdown document and its excerpt with Folia and compares the HTML
  with kramdown's, both parsed and normalized (whitespace, entity and attribute order): heading
  ids, link targets, text, structure and code text. A difference passes only when an accepted
  ledger entry names that source and gate, and a stale entry fails.
- **`pages`** builds the whole site as of the baseline's time and compares every file with the one
  Jekyll wrote at the same path: a page's fingerprint (title, canonical link, description, headings,
  links, text), its whole-document DOM, redirect stubs and static files byte for byte, the **links**
  (every internal `href` and `src` must resolve to a page, a file or a redirect), and the **expected
  warnings** (every warning must be listed, every listed one must occur). It keeps the build, the
  reports and `build.json` (the commits, hashes and compiler it came from) in `out`.
- **`determinism`** builds twice and compares the trees byte for byte, and each build with the
  recorded reference.
- **`accept`** runs `routes`, `markdown`, `pages` and `determinism` against one baseline and writes
  one evidence folder with each check's transcript and reports.

| `pages` writes | It says |
|-|-|
| `dom-report.md`, `dom-report.json` | every whole-document difference, and the ledger's verdict on it |
| `coverage.md`, `coverage.json` | every output of the baseline and its class, and the two verdicts |
| `warnings.json`, `links.json` | each warning and the entry that expects it; every link and where it resolves |
| `build.json`, `sha256.txt` | the build's record and manifest |

## The deviation ledger

Differences you accept are written in the ledger (`FOLIA_LEDGER`, a TOML file), each as an entry
that says what changes, why, and how it was verified. The checks suppress only what an entry
names; an entry is **stale**, and fails the run, when what it excuses no longer differs. A
difference is accepted only by an entry whose `accepted_by` is one of the ledger's `owners`. The
tool checks that an approval is recorded, not who wrote it: review each change to `owners` and
each accepted entry yourself.

An entry that excuses a difference in a page's whole-document DOM names each output with the
SHA-256 of **both** files reviewed (`expected_sha256` for Jekyll's, `actual_sha256` for Folia's),
so a changed baseline file or a changed page needs a new review.

## Coverage classes

`just pages` writes `coverage.json` and `coverage.md`: every row of the baseline's manifest gets
one class.

| Class | Meaning | Passes the run? |
|-|-|-|
| built and checked | Folia writes it and every check on it passes. Rows with *ledger-accepted differences* (output that differs from Jekyll's only as the ledger's accepted entries describe) are counted apart from the identical ones | yes |
| not built yet / waived / removed | planned but not written; owner-waived comparison; owner-removed output | the last two, with the owner's approval |
| intentional difference | Folia differs on purpose; an owner-approved ledger entry says so. It stays a failing check, and the entry is stale (an error) once the output matches | yes |
| known difference, excluded | Folia differs, the difference is recorded (`status = "recorded"`) and nobody has approved it, nor any ledger entry covers it | **no** |
| unresolved defect | a check fails and no entry covers it, or Jekyll writes an output Folia neither writes nor plans | **no** |
| missing evidence | Folia writes the output but no check compares its contents | **no** |
| unclassified | its coverage entry is invalid, pending or doubled | **no** |

**What `just pages` fails on.** Every check still prints every difference it finds, classified
or not, so the log stays complete. The run exits non-zero for what the checks cannot classify
(the ledger's integrity: a stale or invalid entry, a page whose bytes are not what Folia's
manifest records; the links; the expected warnings; an unknown source identity) and for any row
the coverage report does not pass: an unclassified, known, defective or evidence-less one. An
output with an approved *intentional* difference does not fail it, and that entry covers all that
output's differences, so its rationale must say precisely what differs. A run that fails nothing
ends "Page parity holds with documented differences: this is NOT exact parity", or, only when
no output differs at all, "holds, exactly".

Outputs Folia writes that Jekyll never did have no row. They pass only when an owner-approved
`folia-only` entry names them, and the report lists them apart. The report ends with two
verdicts: a *clean build* (every output accounted for, none a defect or lacking evidence) and
*exact parity* (every row built and checked with no accepted difference, and no Folia-only
output). Only the second says the output is Jekyll's.

## Recording a known source-content link defect

The links check resolves every internal link and fails the run on each one that resolves to
nothing. A site owner records a defect that is the source's own as a `check = "links"` entry of
the deviation ledger (`class = "known-source-defect"`, `status = "recorded"`, with the `page`,
`at` (`a@href`), `value`, `source` and `rationale`). An entry only relabels the one broken link
it names exactly, and only when the Jekyll baseline's page has the same attribute value, so it
is shown to be no change Folia made. Every other broken link still fails. An entry is stale, and
fails the run, when its link is no longer broken or no longer there, when the baseline lacks it,
or when two entries name one link. It approves nothing and changes no output.

## Keep private data out of Git

The ledger, the configuration, baselines, reports and the site's content are the owner's data.
Keep them outside this repository (the `.gitignore` guards the usual places) and never commit
them.
