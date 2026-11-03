# Parity baselines

A baseline is a production build of the Jekyll site Folia replaces, recorded together with
Jekyll's own intermediate results. Folia's output is checked against baselines, so they must be
reproducible and trustworthy: building the same commit twice gives the same result, and recording
the intermediate results does not change the site.

## Requirements

- The reference Jekyll site checked out next to this repository (`../filopoe`), with its gems
  installed (`bundle install` there). Set `FOLIA_SITE` to use another path.
- Ruby and Bundler, as the site's `Gemfile` requires.
- Baselines are written to `../folia-baselines`, outside both repositories. Set
  `FOLIA_BASELINES` to use another directory.

## Building a baseline

```sh
just baseline SITE-MAIN
```

This runs `scripts/baseline.sh ../filopoe ../folia-baselines/SITE-MAIN`, which:

1. refuses to overwrite an existing baseline;
2. refuses to build a site with uncommitted changes, untracked files included, unless
   `--allow-dirty` is given (`just baseline my-label --allow-dirty`); then it saves them as
   `site.diff` (`git diff --binary HEAD`). `--allow-dirty` is for changes to tracked files, such
   as a source fix verified before it is committed: it still refuses untracked files, which
   `site.diff` cannot record and the parity tools refuse, so the baseline could not be used
   (stage a new file to include it). The paths in
   [`parity/site-exclusions.txt`](#the-approved-paths) are left out of all of this: a change
   there is not a change to the site;
3. records the site and Folia commits, the tool versions and the build flags in `BASELINE.json`;
4. builds the site with `JEKYLL_ENV=production bundle exec jekyll build --disable-disk-cache`,
   loading the dump plugin from this repository;
5. checks that the build left the site's working tree as it found it, that the manifest
   accounts for every built file, and that no date fell back to a file's mtime: such a date
   depends on the checkout, not the commit, so the baseline would not be reproducible.

The baseline directory holds:

| Path | Contents |
|-|-|
| `site/` | the production build |
| `dump/manifest.jsonl` | one row per page, document and static file (below) |
| `dump/content/<source>.html` | body HTML after Liquid, generators and Markdown conversion, before layouts |
| `dump/content/_by_output/<output>.html` | the same, for pages that share a source (pagination) |
| `dump/excerpt/<source>.html` | each document's rendered excerpt |
| `dump/meta/<source>.json` | title, date, `last_modified_at` (resolved, its origin, and the front matter value), categories, tags |
| `BASELINE.json` | what was built, and with what; `site.excluded` lists the approved paths left out of `site.status` and `site.diff` |
| `site.diff` | for a build of uncommitted changes, those changes |
| `build.log` | Jekyll's output |

`last_modified_at_from` says where each date came from: `front_matter`, `git` (the last commit
touching the file), or `mtime` (no commit touches it). Jekyll uses front matter only for documents;
for pages the git date always wins (plan v3, §2.6).

Each manifest row has these fields:

| Field | Meaning |
|-|-|
| `kind` | `page`, `document` or `static` |
| `collection` | the collection label (`posts`, `micro`, `digital_garden`), or `null` |
| `class` | the Ruby class, which tells generated pages apart (`JekyllFeed::PageWithoutAFile`, `JekyllRedirectFrom::RedirectPage`, …) |
| `source` | the source path, or `null` for generated pages |
| `url` | the URL Jekyll assigned |
| `output` | the file written, relative to `site/` |
| `written` | whether Jekyll wrote it |
| `content`, `excerpt`, `meta` | the dump files for this row, or `null` |

## Comparing baselines

```sh
just compare SITE-MAIN other-label [expected-diffs.txt]
```

`scripts/compare-builds.sh` compares the `dump/` trees byte for byte and the `site/` trees after
exactly one normalization: the feed-level `<updated>` of each Atom feed, which Jekyll sets to the
build time. The sitemap and the Lunr index are also compared entry by entry, so a different order
is reported as *reordered* rather than as a change (see below). Any other difference fails the
comparison, unless it is listed in the optional expected-differences file, one path per line
relative to the baseline root (`site/about.html`, `dump/content/pages/profile.md.html`), with `#`
starting a comment.

## Verifying the harness

```sh
just baseline-verify BASELINE-B                 # an existing label: left as it is
just baseline-verify my-label                # a new label: the first build becomes the baseline
just baseline-verify my-label --allow-dirty  # a site with uncommitted changes
```

`scripts/verify-baseline.sh` runs the spike 0 acceptance checks on fresh builds of the site:

1. a second run gives byte-identical dumps and the same normalized site;
2. a build without the plugin gives the same normalized site;
3. the manifest accounts for every built file.

Every build goes to a new directory, and nothing is removed or overwritten:

- **A new label:** the first build is made at `../folia-baselines/<label>` and kept as the
  baseline.
- **An existing label:** the baseline is neither read nor written, so the command works with
  the labels this page documents. The first build is made in the verification directory instead,
  as `run1/`, and the checks compare the fresh builds with each other, not with the existing
  baseline.

The verification directory, `../folia-baselines/<label>.verify-XXXXXX`, is new on every run. It
keeps the other builds (`run2/`, `no-plugin/`), each with its `BASELINE.json` and `build.log`, and
each comparison's output (`check1.txt`, `check2.txt`), whether the checks pass or fail; its path is
printed. A build that fails keeps its directory too. Each run keeps two or three builds (about
180 MB each for `filopoe`); remove old verification directories by hand. `--allow-dirty` is
passed to every build, and the checks are otherwise the same.

### The approved paths

`parity/site-exclusions.txt` lists the paths in the site that are Folia's, and that neither
Jekyll nor Folia's discovery reads as part of the site: `_folia/` (Folia's templates and
configuration) and `.github/workflows/content.yml` (the content lint's workflow). It is the one
list that `scripts/baseline.sh` and the parity tools' checkout check (below) read, so the two
cannot disagree. Each path is matched literally: a directory covers what is below it, a file only
itself. A change to these paths, tracked or untracked, does not make the site dirty; any other
change does, untracked files, other workflows and look-alike names (`content.yml.bak`,
`_foliage/`) included. `just baseline-negative` shows this on throwaway repositories, with a
stand-in for Jekyll, together with `--allow-dirty`'s refusal of untracked files and
`baseline-verify`'s handling of existing labels, `--allow-dirty` and kept builds. It needs
neither the site nor a baseline, so `just check` and CI run it.

## How the dump plugin works

`parity/jekyll/folia_dump.rb` is loaded with
`--plugins <site>/_plugins,<folia>/parity/jekyll`, so the site itself is not modified. It
registers no hooks unless `FOLIA_DUMP` names an empty directory. When it does, a `post_convert`
hook copies each body as it stands between conversion and layouts, and a `post_write` hook writes
the manifest and the dumps. The hooks only read site state.

!!! note "Keep `parity/jekyll/` for the plugin only"
    Jekyll loads every `.rb` file below a plugins directory, so helper scripts live in `parity/`.

## Reproducibility limits

Two builds of the same checkout are identical apart from the feed-level `<updated>`. A fresh
clone of the same commit differs in two ways, both existing Jekyll behavior:

- Jekyll reads directories in filesystem order, so the order of pages in `sitemap.xml` and the
  Lunr index differs between clones. That order carries no meaning: `compare-builds.sh` compares
  these files entry by entry (`parity/same_entries.py`) and reports them as *reordered*, which
  does not fail the comparison. A changed entry still does.
- `jekyll-sitemap` dates the 7 static files it lists (6 PDFs and the Yandex verification page)
  by their mtime, which in a clone is the checkout time. This is not normalized: comparing
  baselines from different checkouts reports `sitemap.xml` as changed.

`parity/evidence/fresh-clone-BASELINE-A/` holds the comparison that shows both.

## Removing Liquid from the content (spike 3)

```sh
just deliquify check                      # print the rewrite as a diff
just deliquify write                      # apply it to the reference site
just baseline deliquified --allow-dirty   # build the uncommitted result
just compare SITE-MAIN deliquified parity/spike3-expected.txt
just check-d001 SITE-MAIN deliquified       # the expected differences are D-001 only
```

`parity/codemods/deliquify.py` rewrites only the constructs in plan §10, in the 15 files listed in
the script, and fails with the file and line on any other Liquid. The expected differences are
listed in `parity/spike3-expected.txt`, and the deviation they come from in
`parity/deviations.toml`.

### Keeping Liquid out (content lint)

Folia does not run Liquid, but Jekyll still does, so new Liquid in the prose of the site's Markdown
would work in Jekyll and show as written in Folia. The content lint (plan v7, §7.2) fails on it:

```sh
just liquid                                # the reference site, ../filopoe or $FOLIA_SITE
cargo run --locked -p parity -- liquid --site SITE --config parity/filopoe.toml
```

- **Checked:** the body of every Markdown document Folia discovers. Not front matter, excluded
  files, hidden directories, or Markdown files without front matter, which Jekyll copies as they
  are. Only Markdown: HTML pages, layouts and includes go through Liquid in Jekyll too, but are
  not checked.
- **Found:** each `{{` and `{%`, in the source as Liquid reads it. Jekyll runs Liquid before
  kramdown, so raw HTML, HTML comments, HTML's `<code>` and Markdown escapes (`\{{`) do not hide
  a delimiter.
- **Exempt:** code, as Folia's Markdown parser reads it: fenced code blocks (backticks or
  tildes), indented code blocks, and code spans. A code span kramdown keeps as text, such as one
  in `<u>`, is not code.
- **Reported** as `file:line:column`, with lines counted from the top of the file. The lint exits
  1 if it finds any, and 2 if it cannot check (configuration, discovery or reading).

The reference site passes at its local `SITE-COMMIT-B`. Its public `main`, `SITE-MAIN`, still has the
Liquid spike 3 removed (in `SITE-COMMIT-A`, not yet published), and fails with 363 findings.

The exemption keeps code examples right for Folia, which shows them as written. Jekyll runs Liquid
inside code too, so until the switch a Liquid example in code may still need `{% raw %}` to show
as intended on the Jekyll site. The lint neither asks for that nor adds it.

`filopoe`'s `content` workflow runs the same command on Markdown changes, with Folia checked out
at a pinned commit beside the site. From a `filopoe` checkout with Folia next to it:

```sh
cargo run --locked --manifest-path ../folia/Cargo.toml -p parity -- liquid --site . --config ../folia/parity/filopoe.toml
```

## Checking routes (spike 2)

```sh
just routes BASELINE-A
```

The parity tool (`crates/parity`) has Folia plan every output of the reference site, from the
configuration in `parity/filopoe.toml`, and compares the plan with the baseline's manifest:

1. every authored output: the multiset of (source, URL, output file);
2. the classification of each source: post, micro entry, garden note, page or static file;
3. every generated output (pagination, archives, feeds, redirect stubs, the sitemap, …): the
   set of (URL, output file);
4. the redirect stubs.

Folia itself refuses a plan where two outputs share a URL or an output file. The tool refuses a
site checkout that is not the commit the baseline was built from (with exactly the changes in
its `site.diff`, if it has one, and no untracked files), and uses the baseline's
build time to decide what was future-dated. Changes to [the approved paths](#the-approved-paths)
(`_folia/` and `.github/workflows/content.yml`) do not count, as for `baseline.sh`: neither
Jekyll nor Folia's discovery reads them. Any other file counts, other workflows included.

## Checking Markdown (spike 1)

```sh
just markdown BASELINE-B --report spike1.txt              # every Markdown document of the site
just markdown BASELINE-B --doc pages/profile.md --dump out  # one document, both sides' HTML
just markdown-negative BASELINE-B                         # the check fails when it should
```

`BASELINE-B` is `filopoe` at `SITE-COMMIT-B`, the D-002 source fix committed on top of `SITE-COMMIT-A`.

The parity tool renders each Markdown document of the reference site (and each document's
excerpt) with Folia, and compares it with the HTML kramdown made in the baseline's dump. Both
sides are parsed with html5ever and normalized: whitespace collapsed outside `pre` and dropped
where blocks meet, entities decoded, attributes and classes sorted, and code blocks compared by
text. The gates are those of plan v5, §10: heading IDs, link targets and attributes, text,
structure, the excerpt values templates show (computed with the same Liquid filters), and code
text, with the agreement of token classes with Rouge's reported per language.

A failure counts as explained only when an entry of `parity/deviations.toml` names the source
and the gate (`markdown_gates`), and passes the entry's own check (`markdown_check`) if it has
one: D-003's `resolved excerpt` requires that the note's excerpt has wiki-links, that Jekyll's
excerpt differs from Jekyll's body, and that Folia's excerpt values equal those of Jekyll's body.
Explained by an accepted entry, it passes; by a proposed one, it is reported as pending and the
check still fails; a withdrawn entry explains nothing. An entry that lists a source showing no
difference it explains also fails the check, so entries stay as narrow as the evidence.
`--dump DIR` writes `<source>.kramdown.html`
and `<source>.folia.html` for each document or excerpt that differs.

## Checking pages (vertical slice)

```sh
just pages BASELINE-B            # build with Folia and compare what it writes
just pages BASELINE-B ../out/p1  # the same, keeping the build in a new directory of your choice
just pages-negative BASELINE-B   # the check fails when it should
just ledger-negative BASELINE-B  # a DOM entry holds only for its files; an unknown identity fails
just coverage-negative BASELINE-B  # waivers and removals no owner approved fail
just evidence-provenance parity/evidence/<step>  # evidence built from a clean commit, on its own
just build ../folia-out       # just build, into a directory outside the site
```

The parity tool builds the site with `folia::build`, as of the baseline's build time and with the
templates in the site's `_folia/templates`, and compares every file it writes with the one Jekyll
wrote at the same path (plan v6, §9.2). The build is kept, never deleted, in a new directory:
`--out DIR` names it (it must not exist, and must not be inside the site), and otherwise a new
`folia-parity-pages-*` directory in the temporary directory is used; its path is printed. The
directory holds Folia's output (`site/`), its manifest (`sha256.txt`, one `<sha256>  ./<path>`
line per file, sorted by path), the inputs and result (`build.json`: the build time, the site's
commit and status, the configuration's hash, the templates' tree hash, the Folia source's commit
and status, the compiler) and the summary with any warnings or problems (`build.log`). A failed
build keeps its record and log. Each build is about 164 MB; builds are never pruned, so remove
old ones by hand. `just pages` also requires the templates to have the tree hash it names
(`templates_sha256` in the justfile, `--templates-sha256`), and builds nothing otherwise. The
record's `changes`, for the site and for the Folia source, identifies a checkout's uncommitted
changes beyond its commit: `{"state": "clean"}`, `{"state": "changed", "sha256": …}` (the hash
of the staged and unstaged diffs and of each untracked, not ignored file), or
`{"state": "unknown", "error": …}` when the directory has no commit (it is not a git checkout,
and `commit` is `null`), when git fails, or when the tree holds what cannot be identified: an
untracked file that cannot be read, an untracked nested repository (whose contents git does
not list) or a submodule (a gitlink in the index or `HEAD`, whose working tree the diffs give
only as a commit and a `-dirty` marker); the error names it. None of these is ever recorded as a
clean tree or given a hash. **An unknown identity, of the site or of the Folia source, fails
`just pages`** (plan v12, §5.4): `build.log` names it, the run prints it as a failure, and every
other check still runs, so the kept build and its reports keep their diagnostics. Untracked files
are read by the name git gives, byte for byte, so a name that is not UTF-8 still counts (where a
path must be UTF-8, such a name makes the identity unknown).
The diffs are taken with every configurable setting that would change their text given on the
command line (prefixes, quoting,
drivers, colour, file order, algorithm, indent heuristic, context, inter-hunk context, blank
lines, `diff.relative`, submodules, full object ids), so the hash does not depend on the git
configuration; which untracked files count follows the ignore files, the user's
`core.excludesFile` included, as `git status` does.

The pages built are also compared with Jekyll's as whole documents (plan v12, §5.1), and the
comparison decides the check: every difference must be explained by an accepted `dom` entry of
the deviation ledger (`parity/deviations.toml`, `--ledger`; §5.2), no such entry may explain
nothing, and the hard document assertions (robots, canonical, title, description, `lang`,
`rel="me"` and the h-card on `/`, h-entry, redirect stubs) must hold; a page that cannot be read
as UTF-8 fails its assertions as a `read` failure. A `dom` entry is bound to the baseline's
`BASELINE.json` and, for each output, to both files reviewed: `expected_sha256`, checked against
the SHA-256 of the baseline file's bytes, and `actual_sha256`, checked against the SHA-256 that
Folia's output manifest (`sha256.txt`) records for the page. Every page compared must be in that
manifest with the hash of its bytes, or the check fails. A new baseline, a changed baseline file
or a changed page needs a new review; a change to an output the entry does not list does not.
An entry is valid only when its record is the one reviewed, every output it lists was compared
with both its files the ones reviewed, and every `match` item explains a difference on every
output it lists; an entry that is not valid is *stale*, and fails the check, with each reason.
The kind `attribute-value-equal-after-collapse-and-trim` collapses HTML whitespace only (space,
tab, line feed, carriage return, form feed): a no-break space is a character like any other, as
in the title and description assertions. An entry needs an `id` that is not blank, and a field
the ledger's schema does not know is a problem, never ignored. A difference two entries explain
(accepted or proposed) fails as an overlap. `dom-report.json` and `dom-report.md` in the build's
directory list every difference by node path and kind, with the ledger's verdict on it, each
entry's matches and the state of each output it lists, and every assertion; the JSON keeps both
values of each difference in full, and only the Markdown shortens them.
`parity dom --expected A --actual B [--report DIR]` compares any two HTML files or output trees
the same way, in report only. The checks:

1. each output Folia writes, static files included, is one it planned, with Jekyll's URL and
   output file;
2. each page has Jekyll's fingerprint: title, canonical link, description and Open Graph values;
   headings in order; `a href`, `img src` and `link href`/`script src` multisets; and the text of
   its `div.main-content` and of its body, whitespace collapsed;
3. each page's `div.main-content` is equal after the Markdown check's normalization;
4. redirect stubs and static files are byte-identical.

So far Folia builds 4 pages, a stub, the note graph and the static files; the outputs it does not
build yet are not compared, and the coverage report says which they are.

**Coverage** (plan v12, §5.4, A2.1): every run gives each row of the baseline's manifest exactly
one class, prints the counts, and writes `coverage.json` and `coverage.md` (every row, its class,
and the checks or the approval behind it) into the build's directory:

- **Built and checked:** Folia writes the output, at least one applicable check compares it, and
  every applicable check passes.
- **Not built yet:** Folia does not write the output; it remains in scope.
- **Waived:** Folia writes the output, but an explicitly owner-approved entry waives its
  comparison.
- **Removed:** Folia does not write the output because it has been intentionally removed from
  the site, with explicit owner approval.

How each is enforced: every output Folia writes, static files included, has a route check (it is
one Folia planned, at Jekyll's URL and output file), which compares no contents; *built and
checked* needs at least one other check (page checks, whole-document comparison, assertions,
bytes or the note graph) and every check passing. *Not built yet* needs Folia's route table to
plan the output. *Waived* needs the output written, Folia's output manifest to record its bytes,
and its route check to pass; no other check compares it. *Removed* needs the output neither
written nor planned, so a removal cannot stand for an output that is merely not built yet.

A waiver or removal is a `[[deviation]]` with `check = "coverage"`, `class = "waived"` or
`"removed"`, explicit `outputs` (manifest rows), a `rationale` and a `status`; accepted, it also
needs `accepted` (a date), `decision`, and `accepted_by` equal to one of the ledger's top-level
`owners` (see [development](development.md#owner-approvals-in-the-ledger): the tool checks the
recorded approval, review checks that the owner gave it). Proposed and withdrawn entries waive
and remove nothing. The run fails on a row with no class (one Folia writes that no check compares
or whose check fails, one Folia neither writes nor plans), on a row two entries claim, on a
proposed or unapproved entry, an unknown class, an output that is no manifest row, a waiver that
does not meet its conditions or a removal of an output Folia writes or plans. The ledger holds no
coverage entry today.

## Reproducing from a clean checkout

You need [just](https://just.systems), a stable Rust toolchain with `cargo-deny`, Ruby 3.3 with
Bundler, [uv](https://docs.astral.sh/uv/) and Node.js. Baselines stay outside both repositories,
in `../folia-baselines`.

```sh
git clone https://github.com/ggirelli/folia.git && cd folia
just check                                    # needs neither the site nor a baseline
git clone https://github.com/OWNER/SITE-REPO.git ../filopoe
(cd ../filopoe && bundle config set --local path vendor/bundle && bundle install)
```

`filopoe` does not track its `Gemfile.lock`; each baseline records the lock file's SHA-256 in
`BASELINE.json`, and the reference baselines used Jekyll 4.4.1 on Ruby 3.3.8.

**Spikes 0 and 3** start from the site before the Liquid rewrite, commit `SITE-MAIN`. Build them
from a separate clone at that commit, so the main checkout is untouched:

```sh
git clone ../filopoe ../filopoe-SITE-MAIN && git -C ../filopoe-SITE-MAIN checkout SITE-MAIN
cp ../filopoe/Gemfile.lock ../filopoe-SITE-MAIN/   # untracked, so a clone lacks it
(cd ../filopoe-SITE-MAIN && bundle config set --local path ../filopoe/vendor/bundle)
export FOLIA_SITE=../filopoe-SITE-MAIN
just baseline-verify SITE-MAIN                  # spike 0
just deliquify check && just deliquify write  # spike 3: review, then apply the rewrite
just baseline deliquified --allow-dirty
just compare SITE-MAIN deliquified parity/spike3-expected.txt
just check-d001 SITE-MAIN deliquified
unset FOLIA_SITE
```

**The rebaseline and spike 2** use `SITE-COMMIT-A`, the commit that applied the rewrite, and **spike
1** uses `SITE-COMMIT-B`, which adds the D-002 source fix on top of it. Until they are published to
the site's repository they exist only in the local checkout, and these steps need them (where the
baselines already exist, `just baseline-verify` leaves them as they are and verifies into new
directories, as described above):

```sh
git -C ../filopoe checkout SITE-COMMIT-A
just baseline-verify BASELINE-A
just check-date-shift deliquified BASELINE-A     # the rebaseline moved only dates
just routes BASELINE-A                           # spike 2
just markdown BASELINE-A                         # before the D-002 fix: fails on D-002

git -C ../filopoe checkout SITE-COMMIT-B
just baseline-verify BASELINE-B
just check-date-shift BASELINE-A BASELINE-B         # the fix moved one date and nothing else
just markdown BASELINE-B                         # spike 1
just markdown-negative BASELINE-B
just routes BASELINE-B
just pages BASELINE-B                            # the vertical slice, with ../filopoe/_folia
just pages-negative BASELINE-B
just ledger-negative BASELINE-B
```

The slice's templates are in `filopoe/_folia/templates`, uncommitted in the local checkout until
they are reviewed.

The D-002 fix was first verified uncommitted, as `BASELINE-A-d002`: on `SITE-COMMIT-A`, apply the change,
then `just baseline BASELINE-A-d002 --allow-dirty` and `just compare BASELINE-A BASELINE-A-d002`. `parity`
accepts that baseline only for a checkout with exactly its recorded changes (`site.diff`).

Baselines of the same commit from different checkouts can differ in page order and static-file
dates (see the reproducibility limits above); route parity does not depend on either.
