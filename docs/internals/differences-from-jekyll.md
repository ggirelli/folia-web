# Differences from Jekyll and known limitations

Folia is not a drop-in replacement for Jekyll. This page lists, in plain terms, where its output
differs from Jekyll's, what it refuses to do, and what it does not do yet. Read it before treating
a difference as a bug, and before moving a Jekyll site over ([how to](../guide/migrating.md)).

**A clean build is not the same as identical output.** The comparison tools (see the
[parity tools reference](parity-reference.md)) can report zero failed checks and zero
unexplained differences, which says that Folia wrote every output and that nothing failed
unexplained. It does not say the output equals Jekyll's. Every difference below is documented and
most were accepted by the maintainer on purpose, and a documented difference is still a
difference: the comparison report counts them apart from the identical outputs (see
[coverage classes](parity-reference.md#coverage-classes)).

This page describes behaviour only. A site's content, its evidence and its owner's decision
records are private and kept outside this repository.

## What Folia refuses

- **Unknown front matter.** Jekyll ignores a key it does not use; Folia stops and names the key,
  except one documented exception (see [Writing content](../guide/content.md)).
- **Liquid.** Folia does not run it, in content or in templates (which are MiniJinja). An HTML page
  with Liquid in it is refused as "not built yet".
- **`_drafts`.** See [Drafts](#drafts) below.

## Differences in what Folia writes

### Garden notes: whitespace in two meta values

Some garden notes can differ from Jekyll's output in two attributes only,
`meta[name=description]@content` and `meta[property=og:title]@content`, and only in whitespace:
Jekyll's value can be Folia's wrapped in newlines, because its head template's `{% capture %}`
blocks keep the newlines around their tags. Nothing else on those pages differs. A site accepts
this difference in its ledger, and the acceptance covers no other difference.

### Category and tag listings

The category and tag listing pages link differently on purpose:

- Jekyll linked a feed for every category, but only tags have feeds; those links were dead, and
  Folia does not write them.
- A tag with a space in its name has a feed, which Folia writes (`/blog/tag/<tag with space>.xml`)
  and links percent-encoded. Jekyll's link to it was dead (see [Folia-only outputs](#folia-only-outputs)).

### Feeds

- A feed's `<updated>` is its newest item's date, never the build time, so that
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

### robots.txt

Folia writes a default `robots.txt` when the site has none; Jekyll wrote none, so a site that
shipped its own keeps it, and one that had none gains the default.

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
names tag feeds for letters, digits and `_` only). It has no Jekyll counterpart, so nothing
compares it; the comparison report lists it as Folia-only.

### Documents without a title

As Jekyll titles them (checked against a Jekyll 4.4.1 build of the same file names, and pinned by
`crates/folia/tests/untitled.rs`): a page without a `title`, or with an empty one, has an empty
title; a post, a garden note and a micro entry are titled from their file name, the date and the
extension dropped, split at each `-` and each word capitalized (`2026-01-12-a-note_x.md` is
`A Note_x`, `Mixed-CASE-name.md` is `Mixed Case Name`, `index.md` is `Index`). A title in the front
matter always wins. Wiki-links find a note by the title Jekyll gave it, so they follow. An empty
title makes a page's `title` an empty string, not undefined: a template that tests it sees it
false.

### Drafts

Folia does not read a `_drafts` directory and has no `--drafts` flag; Jekyll has both. A draft is a
document with `published: false`, built only with `--unpublished`, and scheduled content is built
only with `--future`. Such a build is a preview (see
[Drafts and scheduling](../guide/drafts.md#preview-builds)).

Jekyll dates a draft by its file's modification time. A checkout resets modification times, and
editing a draft changes its date and URL, so a draft's output is not reproducible. Folia dates
every post by its file name. Instead of a `_drafts` directory, put the draft in the posts
directory with a dated file name and `published: false`
([migrating](../guide/migrating.md#sites-with-a-_drafts-directory)).

## Problems in the source that Folia reproduces

- **Bare-domain references.** Reference-style link definitions written without a scheme
  (`[Name]: github.com`) are relative URLs, and Jekyll rendered them as such. Folia does the same,
  so the link check reports them as internal links that resolve to nothing. They are a defect in
  the site's Markdown, to be fixed there; Folia does not guess a scheme.
- **Broken wiki-links** are shown as broken and warned about, once each. A site lists the warnings
  it expects.

## `folia serve`

- Every change builds the whole site again; there is no incremental build. Only the files whose
  bytes changed are written, though.
- A git commit alone triggers nothing, though a page's last-modified date comes from its last
  commit: build again by changing a file.
- The served site's URL is the local address, so its output differs from production's. Deploy
  what `folia build` writes.
- `folia serve` takes `--future` and `--unpublished` like `folia build`. It may switch
  its destination between a normal and a preview build, which `folia build` refuses to do.

See [Preview while you write](../guide/preview.md).

## Not covered by the comparison tools

- Stylesheets need Dart Sass exactly 1.100.0.
- Syntax highlighting uses syntect, not Rouge. Token classes agree with Jekyll's on about 98% of
  blocks; the rest differ in class names or span splits.
- Only Markdown and HTML pages are built from `pages/`; other page types are skipped, each named
  with the reason.
- The comparison tools check a build against one recorded Jekyll baseline. A site that changed
  since the baseline needs a new baseline first.

## Not supported

Folia has no plugin system, hooks, shortcodes, data files or user-defined collections, and the
URL prefixes (`/blog/`, `/dg/`, `/micro/`, `/likes/`, `/bookmarks/`, `/rsvps/`) are fixed. The
three collections in `[roots]` are all required. Footnotes, math and definition lists are not
supported in Markdown.
