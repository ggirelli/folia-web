# Architecture

How Folia is built, for someone who wants to change it. For how to set up and test a change, see
[development setup](../contributing/development-setup.md).

## The crates

The Cargo workspace has one crate per collection (the code calls a collection a *mode*) and a few
shared ones. Each collection crate depends on `folia-core` only, never on another collection.

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
- **`crates/blog`, `crates/micro`, `crates/garden`, `crates/pages`**: one crate per collection. Each
  module comment holds the collection's route table. `folia_garden::WikiLinks` resolves wiki-links
  by title or file name, as the Jekyll wiki-link plugin the original site used did, and
  `folia_garden::build` (module `note`) builds notes' pages, with their backlinks, and the note
  graph; `folia_pages::build` builds pages; `folia_blog::category_counts` gives the categories
  every page shows, and `folia_blog::build` (module `post`) builds posts' pages. Each `build`
  documents the context its templates see.
- **`crates/folia`** (`folia`): the site builder and the `folia build` command (`main.rs`).
  `config` reads `folia.toml`, `discover` classifies the site's files, and `site` plans every
  output in one route table, checks it for duplicate URLs and colliding output files, and writes
  redirect stubs. `build` is the pipeline (below). `render` renders pages with MiniJinja from the
  site's templates (`_folia/templates` in the site), `redirects` serializes `/redirects.json` as
  `jekyll-redirect-from` writes it, and `output` writes the destination whole or not at all.
  `serve` is `folia serve`: it builds the whole site again on every change `watch` reports, with
  the site's URL set to the server's, and serves it (`http`), mapping URLs to files as the host
  does (`files`); `reload` is the live reload, whose script goes into pages as they are served,
  never into the files. Its destination's record says `folia serve` wrote it
  (`output::claim_for_serve`), so that serve never replaces what `folia build` wrote, and
  recovers its own interrupted builds.
- **Preview flags.** `build` and `serve` take `--future` and `--unpublished` (`main.rs`), which
  travel as a `Preview` (`folia_core::content`). They reach the modes through the `SiteClock`
  (`with_future`, `with_unpublished`), which the modes already receive; the sass check in
  `build.rs` reads it too. A preview build's summary line and warning are printed by `main.rs`,
  and `output` writes its `/folia future` and `/folia unpublished` lines into the record, which
  `folia build` checks before it replaces a destination.
- **`crates/parity`**: the dev-only parity tool and the content lint. It is described in the
  [parity governance](parity-governance.md#the-parity-tooling) page, and is not part of the `folia` binary.

## From files to output

`folia build` (`crates/folia/src/build.rs`) runs one pipeline:

1. **Plan** every output of the site into one route table, and check it for duplicate URLs and
   colliding output files (`site::plan`).
2. **Build the documents**, each by its collection's crate: pages, posts, notes (with the note
   graph) and micro entries, dated with their last git commits. A document Folia does not support
   yet is skipped, with the reason; a document that is supported but fails (a missing template, an
   unknown author) fails the whole build. Nothing supported is skipped silently.
3. **Assemble the site** that templates see: the site's identity and authors, the build's year and
   the blog's categories.
4. **Render**: check every page's template, then render each page with MiniJinja.
5. **Generate**: a redirect stub for each `redirect_from` URL, the files the collections generate
   (the note graph), the sitemap, the feeds, the search script, `redirects.json`, and a copy of
   every static file.
6. **Write** it all to the destination, whole or not at all (`output`).

Every URL the plan reserves and no step produced is counted as not built yet: today that is
`/robots.txt` alone (see the [command-line reference](../reference/cli.md#what-it-prints)).
Warnings, such as a broken wiki-link, do not stop a build.

## Design decisions

Choices that shape the rest of the code, and that you will want to know before changing it.

- **Collections stay apart.** Each knows its own routes, front matter and template context, and
  hands the builder `PageSpec`s and `PlannedOutput`s. The builder knows no collection's rules, which
  is why adding a URL clash check or a feed does not touch a collection.
- **One route table.** Every output, including generated ones, is planned first, so a clash is
  found before anything is rendered or written.
- **All or nothing.** A build writes to a sibling directory and swaps it in, and records what it
  wrote, so a failed build leaves the destination as it was and Folia never deletes a file it does
  not own.
- **Strict, not forgiving.** An unknown front matter key, an undefined template value or an unknown
  author is an error. Jekyll ignores such things; Folia would rather stop than publish a page
  that differs from what its author meant.
- **Jekyll-compatible on purpose.** Markdown, slugs, dates, feeds, the sitemap and the redirect
  list are made as Jekyll and its plugins made them, because the first site Folia built was
  migrated from Jekyll. This is also why some parts look odd to someone who never used Jekyll.

## How each part is built

**Front matter Folia does not know is refused**, because a misspelt key would otherwise leave a
page silently different from what its author meant. Jekyll ignores such keys, so a published site
can have some. The exceptions are listed, one exact key each, with why, in
`folia_pages::IGNORED_TYPOS`. So far there is one, owner decision: `narrow-post` (for
`narrow_post`), set to `true` or `false`, on pages only, is accepted and changes nothing. No
template reads it, so Jekyll published the page at its usual width, and Folia builds it the same
way. Posts and other modes still refuse it, and every other unknown key is still refused.

**Authors** are the configuration's `[authors.<key>]` tables, typed in
`folia::config::Author`, with the fields listed in its module: names, picture, pages, the
author page's backgrounds (`[[authors.<key>.covers]]`) and an account on each service the site
links to. An unset field is left out; an empty one is refused. Templates see every field as
`site.authors.<key>`, and `covers_json`, the backgrounds as the author page's script reads them,
serialized by Folia as Jekyll's `jsonify` wrote them. A page names its author with its front
matter `author`; one that names an author the configuration does not have is not rendered.

**Micro entries** are built by kind, the directory they sit in (`notes`, `likes`, `bookmarks`,
`rsvps`), with the defaults the site's Jekyll configuration gave each kind: layout and category,
the frame's navigation and titles hidden, a corner link back to `/micro`, no previous and next
row, categories and tags hidden (see `folia_micro`'s `entry` module). Their author is the
configuration's `[micro]` `author`, a key of `[authors]`; without one, they have none.

**Micro listings** are the site's pages that list micro entries, which micro *claims*: each
`[[micro.listings]]` names its `source`, the `template` that renders it and the `kinds` it lists.
The page keeps its route, redirects and front matter, its body is not rendered, and the template
gets `page.items`, the entries newest first (same dates by path, descending, as Jekyll sorted them),
each with its excerpt, its text and its first 25 words.

**Blog listings** are built by the blog (`folia_blog::listings`) when the configuration's `[blog]`
names them: `index`, the claimed source of `/blog/` (its `source` and `template`), whose pages
list 9 posts each (not `hidden`), the first also the `featured` ones; and `archive-template`, which
renders an archive per category. Each post is a card: its cover, title, the first 30 words of its
excerpt, author and date. Archives are not dark, as the site's front matter defaults never
reached them (`use_dark_layout` false). `[blog]`'s `categories` and `tags` claim the pages listing
the posts by category and by tag (their `source` and `template`): each gets `page.groups`, every
group by name in byte order, with its posts' cards newest first.

**Feeds** are the Atom feeds the modes plan (the blog's, one per tag, micro's, one per
micro kind, and the garden's), written by `folia::feed` when the configuration has `[feeds]`:
its `lang`, the `generator` the feeds name (`name`, `uri`, `version`) and how many items each
lists (`limit`, 10). Each mode gives a `FeedItem` per entry it builds; a feed lists the newest of
those whose pages were rendered, serialized as `jekyll-feed` 0.17 wrote them, without a template.
Its `<updated>` is its latest item's, never the build time. Notes name the
configuration's `[garden]` `author`, as micro entries name `[micro]`'s, and their summaries show
wiki-links resolved (D-003). A tag's feed is `/blog/tag/:tag.xml` for every tag that can be a
file name; one with a space is written with it (`stem cells.xml`) and linked percent-encoded
(`stem%20cells.xml`; the group's `feed`, `null` for a category, which has none). The **combined
feed** is a page of the site (`[feeds.combined]`: its `source`, such as `feed.xml`, and the
`collections` it lists): the newest items of those collections, titled as Jekyll's
layout titled it (the site's title, then ` | ` and nothing, trailing separator kept), with a
summary and no content.

**The sitemap**, `/sitemap.xml`, is written by `folia::sitemap` as `jekyll-sitemap` 1.4 wrote it:
every page rendered but `/404.html` (redirect stubs are not pages), by URL, then the static
`.htm`, `.html`, `.xhtml` and `.pdf` files. A page's `<lastmod>` is its entry's `modified` or
date, or its claimed source's last commit; a page without a source, such as a category archive,
has none. A static file's is its last commit, where Jekyll used the file's modification time.

**The search script** is written by `folia::search` when the configuration has `[search]`: it
claims the script's `source` (`assets/js/lunrsearchengine.js`) and writes, at its route, the
`documents` array the site's Liquid made, followed by `fragment`, the script's code after the
array, read as it is from the site (`_folia/`). The records are the site's pages
by URL (with the stubs and `redirects.json`; not `.xml`, `assets`, `category` or `tag` URLs),
then the RSVPs, likes and bookmarks, the notes and the posts, oldest first, each with its body's
text by the script's filter chain, except that a line break in a body is a space, not nothing as
in Jekyll ([differences from Jekyll](differences-from-jekyll.md)). Micro entries are listed
(an RSVP's body is its answer and event), and strings are JSON-escaped, with no
`markdownify`. A listing page's body is the text of its template's block named by
`content-block` (`[search]`), so the navigation, the footer and the rest of the frame are not
indexed; without it, the rendered page. A redirect stub has no body.
