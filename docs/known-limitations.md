# Known limitations and compatibility decisions

What Folia does differently from Jekyll, and what it does not do yet. Read it before treating a
difference as a bug, or a green run of the parity tools as proof that the output is Jekyll's.

**A clean build is not exact parity.** Zero failed assertions, zero unclassified rows and a
successful build say that Folia wrote every output and that nothing failed unexplained. They do
not say the output equals Jekyll's. Every difference below is documented and most were approved,
and a documented difference is still a difference: the real-site coverage report counts them
apart from the identical rows (see [coverage classes](migrating/parity-reference.md#coverage-classes)).

This page describes behaviour only. The reference site's content, its evidence and the owner's
decision records are private and kept outside this repository.

## Differences from Jekyll

### Garden notes: whitespace in two meta values

Six garden notes (the four `dg/book-club/…-books_list` pages, `dg/book-club/book-quotes` and
`dg/coding/python`) are covered by the ledger's decision D-004, which the project owner extended to
them on 2026-11-08. It excuses two attributes only, `meta[name=description]@content` and
`meta[property=og:title]@content`, and only whitespace in them: Jekyll's value is Folia's wrapped in
two newlines on each side, because its head template's `{% capture %}` blocks keep the newlines
around their tags. Nothing else on those pages differs, and the approval accepts no other
difference, on these pages or any other. They are *accepted differences*, not identical output: a
clean build is still not exact parity.

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

### Drafts

Drafts, future-dated content and `published: false` documents are built only with `--drafts`,
`--future` and `--unpublished`, and such a build is a preview (see [previewing drafts and scheduled posts](guide/building.md#previewing-drafts-and-scheduled-posts)).
Jekyll's flags differ from Folia's in these ways:

- `--drafts` reads only the `_drafts` directory at the site's root, and only its Markdown files
  (`.md`, `.markdown`), as posts. A `_drafts` directory anywhere else is not read.
- A draft's date is its file's modification time. A checkout resets modification times, and
  editing a draft changes its date and URL, so a draft's output is not reproducible.
- `--unpublished` is independent of the other two, as in Jekyll: `--drafts` alone leaves out a
  draft with `published: false`.
- A date at the start of a draft's file name is not read: it stays in the slug, which is the file
  name without its extension (unless the front matter sets `slug`).

## Source-content issues Folia reproduces

- **Bare-domain references.** Reference-style link definitions written without a scheme
  (`[Name]: github.com`) are relative URLs, and Jekyll rendered them as such. Folia does the same,
  so the link check reports them as internal links that resolve to nothing. They are a defect in the
  site's Markdown, to be fixed there; Folia does not guess a scheme. Three exist in the reference
  site.
- **Broken wiki-links** are shown as broken and warned about, once each. A site lists the warnings
  it expects.
- **Unknown front matter** is refused, except the one documented exception (see
  [Writing content](guide/content.md)); Jekyll ignored it silently.

## `folia serve`

- Every change builds the whole site again; there is no incremental build.
- A git commit alone triggers nothing, though a page's last-modified date comes from its last
  commit: build again by changing a file.
- The served site's URL is the local address, so its output differs from production's. Deploy
  what `folia build` writes.
- `folia serve` takes `--drafts`, `--future` and `--unpublished` like `folia build`. It may switch
  its destination between a normal and a preview build, which `folia build` refuses to do.

## Not covered by parity checks

- Stylesheets need Dart Sass exactly 1.100.0.
- Syntax highlighting uses syntect, not Rouge. Token classes agree with Jekyll's on about 98% of
  blocks; the rest differ in class names or span splits.
- Only Markdown and HTML pages are built from `pages/`; other page types are skipped, each named
  with the reason.
- The parity tooling compares a build with one recorded Jekyll baseline. A site that changed since
  the baseline needs a new baseline first.
