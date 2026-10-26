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

1. refuses to build a site with uncommitted changes, unless `--allow-dirty` is given
   (`just baseline my-label --allow-dirty`); then it saves them as `site.diff`
   (`git diff --binary HEAD`);
2. refuses to overwrite an existing baseline;
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

`last_modified_at_from` says where each date came from: `front_matter`, `git` (the last commit
touching the file), or `mtime` (no commit touches it). Jekyll uses front matter only for documents;
for pages the git date always wins (plan v3, §2.6).
| `BASELINE.json` | what was built, and with what |
| `site.diff` | for a build of uncommitted changes, those changes |
| `build.log` | Jekyll's output |

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
just baseline-verify SITE-MAIN
```

This builds the baseline and runs the spike 0 acceptance checks against two throwaway builds:

1. a second run gives byte-identical dumps and the same normalized site;
2. a build without the plugin gives the same normalized site;
3. the manifest accounts for every built file.

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
build time to decide what was future-dated. Changes below `_folia/`, Folia's own templates and
configuration in the site, do not count, nor do changes to `.github/workflows/content.yml`, the
content lint's workflow: neither Jekyll nor Folia's discovery reads them. Any other file counts,
other workflows included.

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
just pages-negative BASELINE-B   # the check fails when it should
just build ../folia-out       # just build, into a directory outside the site
```

The parity tool builds the site with `folia::build`, as of the baseline's build time and with the
templates in the site's `_folia/templates`, into a temporary directory, and compares every file
it writes with the one Jekyll wrote at the same path (plan v6, §9.2):

1. each page and redirect stub has Jekyll's URL and output file;
2. each page has Jekyll's fingerprint: title, canonical link, description and Open Graph values;
   headings in order; `a href`, `img src` and `link href`/`script src` multisets; and the text of
   its `div.main-content` and of its body, whitespace collapsed;
3. each page's `div.main-content` is equal after the Markdown check's normalization;
4. redirect stubs and static files are byte-identical.

So far Folia builds `/about`, its stub and the static files (step 1 of the slice); the outputs
it does not build yet are counted, not compared.

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
the site's repository they exist only in the local checkout, and these steps need them:

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
```

The slice's templates are in `filopoe/_folia/templates`, uncommitted in the local checkout until
they are reviewed.

The D-002 fix was first verified uncommitted, as `BASELINE-A-d002`: on `SITE-COMMIT-A`, apply the change,
then `just baseline BASELINE-A-d002 --allow-dirty` and `just compare BASELINE-A BASELINE-A-d002`. `parity`
accepts that baseline only for a checkout with exactly its recorded changes (`site.diff`).

Baselines of the same commit from different checkouts can differ in page order and static-file
dates (see the reproducibility limits above); route parity does not depend on either.
