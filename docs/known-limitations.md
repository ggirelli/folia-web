# Known limitations and compatibility decisions

What Folia does differently from the Jekyll site it replaces, and what it does not do yet. It is
for developers: read it before treating a difference as a bug, or a green run as proof of parity.

**A clean build is not exact parity.** Zero failed assertions, zero unclassified rows and a
successful build say that Folia wrote every output and that nothing failed unexplained. They do
not say the output equals Jekyll's. Every difference below is documented and most were approved,
and a documented difference is still a difference: the real-site coverage report counts them
apart from the identical rows (see [coverage classes](#coverage-classes)).

This page describes behaviour only. The reference site's content, its evidence and the owner's
decision records are private and kept outside this repository (see
[a site's private inputs](development.md#a-sites-private-inputs)).

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

## Differences from Jekyll

### Garden notes: whitespace in two meta values

Six garden notes (the four `dg/book-club/…-books_list` pages, `dg/book-club/book-quotes` and
`dg/coding/python`) differ from Jekyll's page only in the whitespace around the `description` and
`og:title` meta values: Jekyll's value is Folia's wrapped in two newlines on each side, because its
head template's `{% capture %}` blocks keep the newlines around their tags. The project owner
approved these six outputs as ledger-accepted differences on 2026-11-08, the same two attributes as
the other pages that differ this way, and nothing else on them differs. Like those pages, they are
*accepted differences*, not identical output.

### Category and tag listings

The category and tag listing pages link differently on purpose:

- Jekyll linked a feed for every category, but only tags have feeds; those links were dead, and
  Folia does not write them.
- A tag with a space in its name has a feed, which Folia writes (`/blog/tag/<tag with space>.xml`)
  and links percent-encoded. Jekyll's link to it was dead (see [Folia-only outputs](#folia-only-outputs)).

### Feeds

- A feed's `<updated>` is its newest item's date, never the build time (plan v9, §7.3), so that
  builds are reproducible. `jekyll-feed` wrote the build time.
- Entries match Jekyll's in count, ids, order, links, dates, categories and authors. Entry bodies
  differ in syntax-highlighting tokens, in raw-HTML serialization (Jekyll's kramdown rewrote some
  attributes and void elements; Folia passes raw HTML through) and in whitespace. A duplicated
  attribute that kramdown dropped is written once.
- The combined feed lists the newest items of its configured collections, with a summary and no
  content, under the title Jekyll's layout gave it, trailing separator kept.
- Notes' summaries show wiki-links resolved.

### Sitemap

The URL set equals Jekyll's. The order differs: Folia lists every page by URL, then static files,
where Jekyll listed collections, then pages by file name, then generated pages. A page's
`<lastmod>` is the same as Jekyll's; a static file's is its last commit, where Jekyll used the
file's modification time, which changes with every checkout.

### Search

- Micro entries are indexed; Jekyll's index omitted them.
- Redirect stubs are indexed with an empty body.
- A listing page's body is the text of its template's content block (`content-block` in
  `[search]`), not the whole page; without that setting it is the rendered page.
- Strings are JSON-escaped and never run through `markdownify`, so curly quotes from Jekyll's
  smart punctuation and raw backslashes in code differ.
- **An intentional improvement over Jekyll:** each run of line breaks in a body becomes one space,
  with the spaces around it (`\n`, `\r\n`). Jekyll's `strip_newlines` deleted them, so the last
  word of a hard-wrapped line and the first of the next, and the items of a list, were indexed as
  one word (`two`, `garden` as `twogarden`) and found by neither. A break that follows a space,
  as between two paragraphs, gives Jekyll's text; a body with no break is unchanged.
- The order of records is not Jekyll's, which varied between two of its own builds.

### Folia-only outputs

Folia writes a tag feed for a tag whose name has a space, where `jekyll-feed` wrote none (it
names tag feeds for letters, digits and `_` only). It has no Jekyll baseline, so nothing compares
it; the coverage report lists it as Folia-only.

### Documents without a title

As Jekyll titles them (checked against a Jekyll 4.4.1 build of the same file names, and pinned by
`crates/folia/tests/untitled.rs`): a page without a `title`, or with an empty one, has an empty
title; a post, a garden note and a micro entry are titled from their file name, the date and the
extension dropped, split at each `-` and each word capitalized (`2026-01-12-a-note_x.md` is
`A Note_x`, `Mixed-CASE-name.md` is `Mixed Case Name`, `index.md` is `Index`). A title in the front
matter always wins. Wiki-links find a note by the title Jekyll gave it, so they follow. An empty
title makes a page's `title` an empty string, not undefined: a template that tests it sees it
false.

## Source-content issues Folia reproduces

- **Bare-domain references.** Reference-style link definitions written without a scheme
  (`[Name]: github.com`) are relative URLs, and Jekyll rendered them as such. Folia does the same,
  so the link check reports them as internal links that resolve to nothing. They are a defect in the
  site's Markdown, to be fixed there; Folia does not guess a scheme. Three exist in the reference
  site.
- **Broken wiki-links** are shown as broken and warned about, once each. A site lists the warnings
  it expects.
- **Unknown front matter** is refused, except the one documented exception (see
  [Building a site](development.md#building-a-site)); Jekyll ignored it silently.

### Recording a known source-content link defect

The links check resolves every internal link and fails the run on each one that resolves to
nothing. A site owner records a defect that is the source's own as a `check = "links"` entry of
the deviation ledger (`class = "known-source-defect"`, `status = "recorded"`, with the `page`,
`at` (`a@href`), `value`, `source` and `rationale`). An entry only relabels the one broken link
it names exactly, and only when the Jekyll baseline's page has the same attribute value, so it
is shown to be no change Folia made. Every other broken link still fails. An entry is stale, and
fails the run, when its link is no longer broken or no longer there, when the baseline lacks it,
or when two entries name one link. It approves nothing and changes no output.

## Not covered by parity checks

- Stylesheets need Dart Sass exactly 1.100.0.
- Syntax highlighting uses syntect, not Rouge. Token classes agree with Jekyll's on about 98% of
  blocks; the rest differ in class names or span splits.
- Only Markdown and HTML pages are built from `pages/`; other page types are skipped, each named
  with the reason.
- The parity tooling compares a build with one recorded Jekyll baseline. A site that changed since
  the baseline needs a new baseline first.
