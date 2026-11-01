# Folia --- Project Plan (v9)

**Status:** Planning draft, revision 9 (supersedes `plan.v8.md`)\
**Project type:** Personal, in-house static-site publishing engine\
**Implementation language:** Rust\
**Primary reference site:** `ggirelli.info`, Jekyll source in the
`filopoe` repository\
**Repository:** `https://github.com/ggirelli/folia.git`, Jujutsu
colocated with Git

Markers used in this document:

- **[DECIDED]**: proposed and justified; change only with new evidence.
- **[OPEN]**: needs the author's input; the recommendation is noted.
- **[SPIKE]**: to be settled by a small throwaway experiment.
- **[LATER]**: deliberately deferred; safe to postpone.

Changes since v8 (Folia `5e9c57a9`, baseline `BASELINE-B`):

- §12.2: step 2 is committed and pushed (`307d09aa`, 2026-11-01), with its safeguards; the docs
  are published to GitHub Pages (`5e9c57a9`). Step 3, two garden notes and the note graph, is
  built and checked locally, **uncommitted and not reviewed**. Its acceptance checks pass
  against `BASELINE-B`; the choices it made that the plan did not cover are listed there.
- §6.2: `ModeOutput` gains `artifacts` and `warnings`; an `Artifact` is a route and its text,
  serialized by the mode, not a JSON value (below §6.2's sketch).
- **Nothing is migrated or published.** The Jekyll site stays the production site; no `filopoe`
  change is merged, and no baseline was refreshed. The templates in `filopoe/_folia/` are still
  uncommitted.

Changes in v8, since v7 (Folia `6ef4a49e`, baseline `BASELINE-B`):

- §12.2: step 2, one post, is built and checked locally, **uncommitted and not reviewed**. Its
  acceptance checks pass against `BASELINE-B`; the choices it made that the plan did not cover are
  listed there, followed by a technical review of them and the author's decisions
  (2026-10-28): 1 and 3 accepted as revised, and implemented; 2 accepted; 4 and 5 deferred, as
  migration requirements. Step 2 is accepted as ready subject to two safeguards before step 3
  (the pages mode refuses front matter it cannot show; a test of a page's TOC), now built
  locally. Step 3 has not started.
- §5.2 and §5.3: presentation front matter is resolved in Rust (`content::presentation`), with
  its defaults there, not in `folia.toml`; `markdown::analyze` (headings) and `toc` are built;
  §6.2's `EntrySummary` gains `published` and `modified`.
- **Nothing is migrated or published.** The Jekyll site stays the production site; no `filopoe`
  change is merged, and no baseline was refreshed.

Changes in v7, since v6 (step 1 committed; Folia `450db9e6`, baseline `BASELINE-B`):

- §7.2: the content lint is decided and built in Folia as `parity liquid`, Markdown only, with
  code exempt (fenced and indented blocks, code spans), and a documented limit for the Jekyll
  site until the switch. Its CI workflow for `filopoe` is written but uncommitted, and its Folia
  pin is still `PENDING`, so it is not active; until it is, the lint protects nothing on its own
  (§14 Q13).
- §12.2: step 1, `/about`, is committed and pushed in Folia (`450db9e6`, 2026-10-26). Its
  templates in `filopoe/_folia/` are still uncommitted. Steps 2 and 3 are still planned.
- v6 broke the slice into reviewed steps and approved the workspace (§4), MiniJinja (§8), the
  Markdown rules (§10.1a) and Q5.

## 1. Purpose

Unchanged from v0. Folia replaces Jekyll for one site. Success means the
next website feature is easier to implement, not more features or more
crates.

## 2. Evidence from the existing site

Inventory of `filopoe` (Jekyll 4, kramdown GFM, Rouge), taken
2026-10-25. This replaces the speculative description in v0 §2.

### 2.1 Content

| Mode   | Source                                       | Docs | URL rule (Jekyll)                       | Output file                          |
| ------ | -------------------------------------------- | ---: | --------------------------------------- | ------------------------------------ |
| blog   | `_posts/YYYY/YYYY-MM-DD-slug.md`             |   79 | `/blog/:year/:month/:day/:title`        | `blog/2021/08/08/webmentions.html`   |
| micro  | `_micro/{notes,likes,bookmarks,rsvps}/*.md`  |   39 | `/micro/:name`, `/likes/:name`, …       | `bookmarks/2025-06-16-malleable-software.html` (underscores slugified) |
| garden | `_digital_garden/**.md` (submodule absent, §2.6) |   54 | `/dg/:path`                             | `dg/tracking/chat_control.html` (underscores kept) |
| pages  | `pages/*.md`, `me/*`, `blog/*.html`, root     |  ~25 | explicit `permalink`                    | `about.html`, `index.html`           |

- URLs are extensionless; files are `*.html`. Directory-style outputs
  exist too (`blog/category/code/index.html`, `blog/page2/index.html`).
  The host must map `/x` to `x.html` (GitHub Pages does).
- Live behavior, checked 2026-10-25:
  - `/bookmarks` returns 200 even though a `bookmarks/` directory
    exists, and `/bookmarks/` returns 404.
  - `/dg/coding/` and `/dg/coding/index` both return 200.
  - Redirect sources (`/blog/about`) are HTML stubs with a
    meta-refresh, a JS redirect, and a canonical link.
- **URL and output path are distinct.** A garden `index.md` has the URL
  `/dg/coding/index` (used in the graph and feeds) but is written to
  `dg/coding/index.html`. Post slugs keep their case
  (`/blog/2023/05/13/May-train-trip-pt1`).
- The micro kind is implied by its directory through Jekyll `defaults`,
  which set `layout`, `categories`, and `permalink`. Templates then
  detect the kind with `item.layout == "bookmark"`.
- Micro and post dates come from filenames. Garden notes have an
  optional `date` (29/54), and "Last updated" comes from
  `jekyll-last-modified-at`, which reads git history (§2.6 has the exact
  rules).
- Front matter has three kinds of keys. Each mode has semantic keys, such
  as `categories`, `tags`, `featured`, `rating`, `trip`, `bookmark`,
  `like`, `rsvp`, `event`, and `redirect_from`. Presentation knobs
  include `cover`, `toc`, `narrow_post`, `hide_website_junk`, and
  `corner_backlink`. There are also typos, such as `narrow-post` in
  `pages/thanks.md`.

### 2.2 Markdown dialect (highest migration risk)

- 218 kramdown IALs in 38 files. Mostly span IALs on links
  (`{: target="blank"}` ×86, `{: .internal-link}` ×35). The rest are
  block IALs for spacing (`{: .mt-1em }`, `.mb-3`, `.text-center`, …).
- 556 reference-link definitions, including shortcut references
  (`[welcome to the jungle]`).
- kramdown heading IDs (`webmentions-a-not-so-new-recommendation`), smart
  typography (`’`, `“`), and `<!--more-->` excerpts in 3 files.
- Rouge highlighting with Pygments short classes (`k`, `nf`, `nd`),
  styled by `_sass/_highlight.scss`. Fenced code appears in 17 files, and
  `{% highlight %}` in 2.
- Wiki-links `[[Title]]` and `[[Title|text]]` appear in 19 garden files.
  They are resolved by `_plugins/wiki_links.rb` with a regex on
  raw source, by title and then by filename. One broken link is live
  today, on `/dg/`.
- **Liquid inside content: 15 files** (14 posts plus
  `_digital_garden/home.md`):
  - 100 `{% assign %}` and ~200 variable uses for image galleries;
  - `{{ site.baseurl }}` (empty) and `"…" | prepend: site.baseurl`;
  - `{{ site.url }}`, `{{ site.paths.home }}`, `{{ site.webmentions.* }}`,
    and `{{ page.url }}`;
  - 11 `{% highlight %}` blocks in 2 files;
  - 1 `{% include digital-garden/notes_graph.html %}`, inline in the
    last paragraph of `_digital_garden/home.md`.

  There are no `{% raw %}` blocks.
- Excerpts: `excerpt_separator: <!--more-->` is set globally, but only 3
  files contain the marker. For all other documents Jekyll's excerpt is
  the whole content (verified by spike 1). Templates consume excerpts
  through `strip_html` with `truncate: 160` (page description) or
  `truncatewords: 30` (cards), and through `strip_html |
  normalize_whitespace` (feed summaries); `/micro` outputs
  `item.excerpt` as-is.
- Fenced code languages: unlabeled (38), `bash` (17), `python`,
  `text`, `javascript`, `html`, `toml`, `yaml`, `rust`, and `email`.
  Mermaid appears in `profile/tools.md`.
- A heuristic scan found no raw HTML blocks where a blank line is
  followed by 4-space-indented content. That pattern turns into a code
  block under CommonMark but not under kramdown, so the risk is lower
  than feared, but spike 1 still covers HTML-heavy pages.

### 2.3 Templates

- 1,858 lines in `_layouts` and `_includes`, plus ~6 Liquid-bearing
  pages. Tags used: `if` (164), `assign` (111), `include` (62), `capture`
  (27), `for` (32), `unless`, `case`, and `comment`.
- Much of that logic does string processing that belongs in Rust. For
  example, `_includes/toc.html` parses rendered HTML with
  `split`/`replace` to build a TOC (143 lines), and layouts compute CSS
  classes with chains of `assign`.
- Jekyll-specific filters are used: `date_to_string`,
  `date_to_xmlschema`, `markdownify`, `xml_escape`, `absolute_url`,
  `where_exp`, `smartify`, `normalize_whitespace`, `jsonify`, and
  `camelcase` (not a real filter; Jekyll silently ignores it).
- Navigation is hand-written HTML in
  `_includes/nav.html`.
- SEO tags are hand-written in `_includes/head.html`;
  `jekyll-seo-tag` is listed in the config but not used.

### 2.4 Generated artifacts

- **Feeds** (all Atom, 10 entries each, ordered by date):
  - `/feed.xml`: posts, garden, and micro, from the custom
    `_layouts/feed.xml`.
  - `/blog/feed.xml`, `/micro/feed.xml`, `/dg/feed.xml`.
  - `/feed/micro/{notes,likes,bookmarks,rsvps}.xml`.
  - `/blog/tag/<tag>.xml` for every tag.

  Entry `updated` is the git last-modified date.
- **Archives:** category pages `/blog/category/<name>/` (directory
  index), pagination `/blog/pageN/` (9 per page), and single-page
  `/blog/tags.html` and `/blog/categories.html`. Tags have no HTML
  archive pages, only feeds.
- **Listings authored as pages:** `blog/microblog.html` (`/micro`),
  `blog/bookmarks.html`, and `blog/likes.html` query `site.micro`
  directly. `blog/index.html` paginates `site.posts`.
- **Garden graph:** `/assets/json/notes_graph.json`
  (`{nodes:[{id,path,label≤25}], edges:[{source,target}]}`).
  Backlinks carry title, URL, and an excerpt (the raw first paragraph).
- **Search:** `assets/js/lunrsearchengine.js` is a Liquid-generated
  `var documents = [{id,url,title,body}]`.
- **Other:** `sitemap.xml`, `redirects.json`, redirect stub pages from
  `redirect_from`, and verbatim root files (CNAME, favicons,
  `robots.txt`, `security.txt`, `keybase.txt`, `manifest.json`).
- **Sass:** `assets/css/main.scss` and `dg_styles.scss`, with Bootstrap
  sources in `_sass`, compressed, using `@import`.

### 2.5 Existing defects (relevant to "preserve first")

- The Lunr template iterates `site.rsvps`, `site.likes`,
  `site.bookmarks`, and `site.without-plugin`, which no longer exist. As
  a result, **individual micro entries are not searchable** (only 3
  micro URLs are indexed).
- `_plugins/external_links.rb` hooks the `:notes`
  collection and `_pages/`, neither of which exists, so it never runs.
- The category jumbotron counts with `site.tags[category]`.
- Micro `datetime=" 2025-06-15"` has a leading space (format string
  `" %Y-%m-%d"`).
- The checked-in `_site` is a `jekyll serve` build, so its absolute URLs
  point to `http://localhost:4000`. **It is not a valid baseline**; see
  §9.

### 2.6 Production baseline facts (verified)

- `JEKYLL_ENV=production bundle exec jekyll build --destination <dir>`
  works locally (Jekyll 4.4.1, Ruby 3.3.8). It takes ~4 s and produces
  1,077 files at `filopoe` commit `SITE-MAIN`, with a clean tree.
- **Two builds of the same commit differ only in the feed-level
  `<updated>`** of each Atom feed, which Jekyll sets to the build time.
  Every other file is byte-identical, so output comparisons need exactly
  one normalization.
- A few `localhost:4000` strings remain in production output, inside the
  content of a post about running Jekyll. They are content, not build
  artifacts.

Verified by the spike 0 harness and during spike 3:

- **The garden submodule is absent.** `.gitmodules` declares
  `_digital_garden/submodule`, but commit `SITE-MAIN` has no gitlink for
  it and the directory does not exist. All 54 garden notes are ordinary
  tracked files. Folia implements no submodule-specific behavior; revisit
  only if a checkout with the submodule present appears. [LATER]
- **How Jekyll resolves `last_modified_at` today.** Read in
  `jekyll-last-modified-at` (pinned git revision `PLUGIN-REV`) and Jekyll
  4.4.1, and checked on a scratch clone of `filopoe`:
  - The plugin sets `data["last_modified_at"]` to a lazy `Determinator`
    in a `post_init` hook, for posts, pages and collection documents.
  - **Documents** (posts, micro, garden): `post_init` runs in
    `Document#initialize`, before `Document#read` deep-merges the front
    matter into `data`. **Front matter wins.** The 4 posts that set it
    show their front matter value.
  - **Pages:** `Page#initialize` reads the front matter first and fires
    `post_init` last, so the plugin overwrites it. **Git wins; front
    matter is ignored.** On the scratch clone, a page with
    `last_modified_at: 2001-02-03` got its commit date. No page sets it
    today, so no output shows this.
  - The `Determinator` resolves to the commit time of
    `git log -n 1 --format=%ct -- <path>`, run against the repository's
    `.git`, so it reads committed history only: uncommitted edits to a
    tracked file keep its commit date (checked on the clone). The time is
    shown in the site's `timezone` (Europe/Stockholm).
  - **When no commit touches the file** (untracked, or staged but never
    committed), it falls back to the file's **mtime**. That depends on the
    checkout, not the commit, so the harness rejects any baseline that
    contains one (`parity/check_dump.rb`).
  - The two Sass pages share their `data` with their source-map pages,
    whose `Determinator` points at a `.css.map` file that never exists.
    Jekyll never reads that value; the dump records it as unresolved.
- **Output that depends on the checkout.** Two builds of one checkout
  are identical, but a fresh clone of the same commit differs in two
  ways:
  - Jekyll reads directories in filesystem order (`Dir.entries`), so the
    order of pages differs between clones. In `SITE-MAIN`, `/` and `/blog/`
    swap places in `sitemap.xml` and in the Lunr index. The parity tool
    must compare both as sets (§9).
  - `jekyll-sitemap` dates the static files it lists by their mtime: 6
    PDFs and the Yandex verification page. In a clone that is the checkout
    time. This is existing Jekyll behavior, recorded and not normalized:
    comparing baselines from different checkouts reports `sitemap.xml`.

  Builds from one checkout are unaffected. `compare-builds.sh` compares
  `sitemap.xml` and the Lunr index (and its content dump) entry by entry,
  and reports a different order as *reordered*, explicitly; a changed
  entry still fails. Route parity does not depend on the checkout: it
  holds on a fresh clone too.
- **Garden excerpts keep raw wiki-links.** Jekyll extracts excerpts while
  reading, before the wiki-link generator runs, so the excerpts of all
  19 garden notes with wiki-links show them unresolved: 18 contain
  `[[…]]`, kramdown reads a line holding `[[Title|text]]` as a table
  row (10 of them), and in one `[[blogroll]]` becomes `[`, a reference
  link to an external site, and `]`. Folia fixes it in rendered excerpts,
  for listings, cards and feed summaries: D-003 (§10, spike 1). A note
  page's description keeps the raw `[[…]]`, as Jekyll's does (step 3,
  §12.2).
- **The body follows the front matter's blank lines.** Jekyll's front
  matter expression (`^(---|\.\.\.)\s*$\n?`) also consumes the blank
  lines after the closing fence; kramdown would otherwise write a line
  break for them. Folia's splitter does the same since spike 1.

## 3. Design principles

The v0 principles stand, with these refinements:

- **Markdown bodies are content, not templates.** [OPEN, recommended]
  Content files are not processed by the template engine. The 15
  affected files are migrated once (§7.2). This decouples the template
  engine choice from content, and makes content parseable without a
  rendering context.
- **Logic out of templates.** Values that templates compute today, such
  as the TOC, micro kind, excerpts, layout classes, and listing
  selection, become data that Rust computes and templates display.
- **Publishing semantics in code, site identity in config.** Permalink
  rules, micro kinds, and feed membership are Rust code in the mode
  crates. Configuration holds site identity, paths, and template-facing
  values. This is fine because Folia serves one site.
- **Preserve, then fix deliberately.** Parity is the default. Every
  intentional difference is recorded in a deviation ledger (§9.4) rather
  than fixed silently.

## 4. Workspace shape [DECIDED, approved 2026-10-25]

v0 proposed 9 crates (+ `site-model`). For ~200 documents most of those
boundaries buy nothing. A crate boundary is worth having where it
**enforces** a rule we care about. The only such rule here is that modes
are peers and do not depend on each other. Everything else can be a
module until it hurts.

```text
folia-core      content::  source docs, ids, front matter, common meta,
                           routes, Markdown, git dates, diagnostics
                publish::  contracts modes hand to the builder
                           (ModeOutput, PageSpec, EntrySummary, FeedSpec,
                           Artifact). No mode logic in either module.
   ▲    ▲    ▲    ▲
folia-blog  folia-micro  folia-garden  folia-pages     (each: core only)
   ▲    ▲    ▲    ▲
folia           lib + bin: config, discovery, site assembly, feeds,
                search, sitemap/redirects, render (MiniJinja), assets,
                output, CLI (clap)
```

Plus a dev-only `parity` tool (§9). It deliberately does **not** depend
on Folia: it compares two output directories as black boxes.

- `site-model` is merged into `folia-core`. The types modes emit are the
  types the builder consumes. A separate crate would only add a hop.
- `folia-feeds` and `folia-render` become modules of `folia`. They are
  leaves: nothing else depends on them, so a crate boundary enforces
  nothing. **Extraction criterion:** split a module into a crate when it
  gains a second consumer or measurably hurts compile times.
- The CLI is a bin target in `folia`. A separate crate is fine if keeping
  `clap` out of the lib matters; it is cheap either way. [LATER]
- **Downside:** `folia` is the largest crate, and discipline between its
  modules relies on convention and review.
- **Approved** on 2026-10-25, as built through spikes 1 and 2 and step 1 of the slice: the four
  mode crates depend on `folia-core` and no other workspace crate (besides `serde_json` and
  `jiff`), and the route and Markdown checks run through `folia`'s public API. The rule stands:
  a mode never depends on another mode.

## 5. `folia-core`: content model and publishing contracts

`folia-core` has two modules with different jobs. They stay in one
crate. A crate boundary between them would enforce nothing, because
every mode needs both.

- **`content`** describes what the author wrote and how it maps to the
  site: source documents, `DocId`, front matter, `CommonMeta`, `Route`,
  Markdown analysis and conversion, filename and git dates, and
  diagnostics. It has no notion of feeds, search, listings, or
  syndication.
- **`publish`** holds the types a mode hands to the site builder (§6.2).
  It defines the shape of the hand-off, not policy. Which entries are
  searchable, or which feeds exist, is decided by modes. How search
  documents are built, and how feeds are serialized, is decided by the
  `folia` crate.

Rule of thumb: if a type only makes sense once something is being
*published*, it belongs in `publish`. If code decides *what* gets
published, it belongs in a mode or in the builder.

### 5.1 Source document (`content`)

```rust
pub struct SourceDoc {
    pub id: DocId,               // source-relative path; stable identity
    pub mode: Mode,              // Blog | Micro | Garden | Pages, from discovery
    pub front: FrontMatter,      // the YAML mapping, read with Jekyll's conventions
    pub body: String,            // Markdown source after front matter and the blank lines after it
}
```

As built in spike 2. Filename dates are read on demand
(`dated_file_name`), and `git_modified` joins when later risk 6 needs it.
Front matter is parsed with `yaml-rust2` (pure Rust), because
`serde_yaml` and its forks depend on the unmaintained `unsafe-libyaml`,
which `cargo deny` rejects. It reads YAML 1.2, where Jekyll's Ruby reads
YAML 1.1: a bare `no` stays a string. The site uses no such value.

Mode assignment is by source directory, configured in `folia.toml`. A
document outside every mode root is a static file, not content.

### 5.2 Metadata: three tiers [DECIDED]

1. **Common semantic metadata** is typed in `content` (`CommonMeta`).
   It holds title, date, `last_modified_at`, description, lang,
   `redirect_from`, and author. These are only the fields that at least
   two modes use *and* that affect identity, routing, or dates.
   `syndications` (POSSE links on posts and micro entries) moved out in
   v2: only templates read it, so it belongs to tier 3. A mode may type
   it later if it acquires semantics.
2. **Mode semantic metadata** is typed in the mode crate (for example,
   `PostMeta { categories, tags, featured, hidden, rating, trip, … }`,
   or `MicroKind::Bookmark { url, label }`). Each embeds
   `#[serde(flatten)] common: CommonMeta`.
3. **Presentation metadata** (`cover`, `toc`, `narrow_post`,
   `hide_website_junk`, `syndications`, …) is collected by
   `#[serde(flatten)] extra` and passed to templates untyped. A configured allowlist of presentation
   keys turns unknown keys into warnings, which catches typos such as
   `narrow-post`.

This avoids a generic "any metadata" abstraction without forcing Rust
types onto purely visual knobs.

### 5.3 Markdown: source plus extracted facts, not a stored AST [DECIDED]

- Core stores the source string. `markdown::to_html(&src, links)` renders
  it, with wiki-links resolved by `links` (an `Option<&dyn
  LinkResolver>`; `None` leaves them as written), and
  `markdown::excerpt(&body)` gives the excerpt's source, and
  `markdown::excerpt_html(&body, links)` renders it with the body's
  resolver (D-003). `analyze()` (headings, links, wiki-links for
  backlinks) comes when a mode needs it; spike 1 did not. Step 2 of the slice built its headings
  part: `analyze(&src, links)` gives each heading's level, ID, inner HTML and classes, and
  `toc(&headings, min, max)` the table of contents (§12.2).
- **Why not store an AST:** Both pulldown-cmark events and comrak's
  arena AST borrow from the source, so stored ASTs mean lifetimes or
  arena plumbing throughout the model. Parsing ~200 documents twice
  costs milliseconds. Rendering builds a small owned tree from the
  events, transforms it, and drops it.
- Rendering Markdown to HTML happens **inside the mode stage** (§6), once
  that mode's links are resolved. Body HTML is data, and page HTML is
  template output. This matches what Jekyll does (convert, then wrap in
  a layout) and lets listing pages embed excerpts.
- **Parser: pulldown-cmark** [DECIDED, approved 2026-10-25]. Spike 1
  reached 99.5% structural parity with it (G4; 100% after the D-002
  source fix), with perfect heading IDs,
  links and attributes, and code text; the only difference was
  CommonMark's emphasis rule (D-002), which comrak shares and a source
  fix removed. comrak was not built, and is not to be without new
  evidence of a need.
- **Dialect ("Folia Markdown")** [DECIDED, approved 2026-10-25 (Q2)]:
  CommonMark + GFM tables/strikethrough (`~~` only)/task lists,
  reference links, wiki-links, kramdown's smart typography,
  kramdown-compatible heading IDs, kramdown's reading of raw HTML, and
  **the IAL subset actually used**:
  - span `{: .class key="value"}` after links, images, emphasis, code
    and inline HTML elements, possibly wrapped over a line;
  - block `{: .class }` on its own line: to the block before it with no
    blank line between, otherwise to the block after.

  Spike 1 implemented it (`markdown::ial`, about 300 lines with its
  tests), and every IAL of the site renders as kramdown renders it
  (G2). The alternative, a codemod to raw HTML, is not needed.

## 6. Publishing interfaces

### 6.1 What a mode owns [DECIDED]

A mode owns:

- typed metadata parsing and validation;
- routes for its documents;
- intra-mode reference resolution (garden wiki-links and backlinks);
- body HTML;
- derived pages (archives, pagination, kind listings);
- feeds for its own content;
- derived artifacts (graph JSON);
- per-entry searchability and feed membership.

It does **not** own templates, site-wide validation, combined feeds,
serialization, or writing files.

Wiki-links only target garden notes today, so link resolution is
**intra-mode**. That removes the main reason v0 had for a cross-mode
resolution phase.

### 6.2 Shape [DECIDED, details open]

Use plain functions, not a `Mode` trait. The builder calls four known
functions explicitly. A trait would only matter if we iterated over an
open set of modes, which we do not.

```rust
// in each mode crate
pub fn build(docs: Vec<SourceDoc>, cfg: &ModeConfig) -> Result<ModeOutput, Diagnostics>;

// folia_core::publish
pub struct ModeOutput {
    pub entries: Vec<EntrySummary>, // one per authored doc, with its prepared body
    pub pages: Vec<PageSpec>,       // canonical pages + generated pages
    pub feeds: Vec<FeedSpec>,       // feeds this mode defines
    pub artifacts: Vec<Artifact>,   // graph JSON, etc.
    pub diagnostics: Diagnostics,   // e.g. broken wiki-link (warning)
}

pub struct EntrySummary {
    pub doc: DocId,
    pub mode: Mode,
    pub kind: Option<String>,          // e.g. "bookmark"; mode-defined
    pub route: Route,
    pub title: String,
    pub published: Option<DateTime<FixedOffset>>,
    pub modified: Option<DateTime<FixedOffset>>, // §7.3 precedence, resolved by the mode
    pub body_html: String,             // prepared content, links resolved
    pub excerpt_html: Option<String>,
    pub categories: Vec<String>,       // feed <category> terms (categories + tags)
    pub redirect_from: Vec<Route>,
    pub searchable: bool,              // the mode decides; the builder builds the document
}

pub struct PageSpec {
    pub route: Route,
    pub template: String,              // runtime template name, §6.4
    pub source: Option<DocId>,         // canonical page of an entry; None if generated
    pub context: serde_json::Value,    // mode-specific template data
}

pub struct FeedSpec {
    pub route: Route,
    pub title: String,
    pub entries: Vec<DocId>,           // membership and order decided by the mode
    pub limit: usize,                  // 10 today
}

pub enum Artifact {
    Json { route: Route, value: serde_json::Value },
    Text { route: Route, body: String },
}
```

- For a page with a `source`, the renderer exposes that entry's
  `body_html` as `content`. Body HTML exists once, in the entry.
- As built (step 3), `ModeOutput` has `artifacts` and `warnings: Vec<Diagnostic>`, and an
  `Artifact` is `{route, body: String}`: the mode serializes its file, so it decides the key
  order of a JSON object. A `serde_json::Value` sorts keys, and Jekyll's note graph keeps the
  plugin's order (`nodes` before `edges`, `id`, `path`, `label`). A `Diagnostic` has no
  severity; where it is found says whether it is a warning.
- Compared with v1, `EntrySummary` no longer carries `SearchDoc` or
  `FeedEntry`. Building search documents and Atom entries is builder
  work. The mode only says *whether* an entry is searchable and *which*
  feeds contain it.
- Mode crates use `serde`/`serde_json` only, so they never see MiniJinja.
- `Route` is a core type holding the URL path and an output kind. It
  maps URLs to output files using one site-wide policy, so modes never
  build file paths themselves:
  - `Html`: `/x` → `x.html`, `/x/` → `x/index.html`, `/x.html` →
    `x.html`;
  - `File`: the URL names the file exactly (`/feed.xml`, `/CNAME`).

  The kind is needed because extensionless static files exist (`CNAME`).
  URLs are kept unescaped; escaping belongs to the HTML that links them.
- Until `ModeOutput` exists, modes hand the builder `PlannedOutput`s:
  route, source, generator, and `redirect_from` (spike 2).
- As built in step 1 of the slice, `ModeOutput` has `entries` and `pages` only, and
  `EntrySummary` has `doc`, `route`, `body_html` and `redirect_from` (step 2 adds `published`,
  and `modified` as `content::Modified`, its time and where it came from: front matter, git or
  the publication); the other fields arrive with
  the steps that use them. Times are `jiff` values, not `chrono`'s; a page's context carries them
  as strings in the site's time zone. Git dates reach a mode as `content::LastCommits`, read by
  the builder, so `SourceDoc` keeps its shape.

### 6.3 Listing pages [OPEN, recommended]

Today `/micro`, `/bookmarks`, `/likes`, and `/blog/` are pages whose
templates filter other collections.

- **Recommendation:** the owning mode generates these listings, using
  pages with filtered, ordered data. The intro prose lives in the
  template, or in an optional authored `_micro/index.md`. Selection and
  ordering are publishing semantics.
- **Escape hatch:** every template also receives a read-only `site`
  index, built from all `EntrySummary` values. An ordinary page (such as
  the landing page) can show "latest posts" without depending on the
  blog crate.

### 6.4 Templates are runtime files [DECIDED]

v1's `template: &'static str` was wrong. Templates live in the site
repository and are loaded at runtime, so names must be owned values.

- `PageSpec.template` is a plain `String`. It is a path relative to the
  site's template directory (`templates_dir` in `folia.toml`), such as
  `"post.html"` or `"micro/bookmark.html"`. A `TemplateName` newtype
  would add ceremony without catching anything a load-time check misses.
- **Where names come from:**
  - *Pages:* front matter `layout: page` maps to `"page.html"`. This is
    the concrete case that needs runtime names: the template is chosen
    by content, not code.
  - *Other modes:* mode code supplies default names, such as `post.html`,
    `micro/{kind}.html`, `garden_post.html`, `archive.html`, and
    `blog/index.html`. These names form a **template contract**,
    documented in one table in the mode's crate docs. A front-matter
    `layout` override is accepted only where Jekyll accepts one today.
- **Loading:** MiniJinja's path loader is rooted at `templates_dir`.
  Names containing `..` or absolute paths are rejected.
- **Fail early:** before rendering any page, the renderer collects the
  set of referenced template names and checks they all exist. It
  reports every missing one in a single diagnostic, together with the
  pages that reference it.
- **Downside:** a misnamed template is caught at build time, not at
  compile time. This is acceptable with the up-front check.

### 6.5 Stage contracts [DECIDED]

Each stage is a plain function call from the build pipeline (§11). No
traits, registries, or injection.

| Stage | Input | Responsibility | Output | Must not |
|-|-|-|-|-|
| **Mode** (`folia-blog`, …) | its `SourceDoc`s, mode config | publishing semantics: typed metadata, validation, routes, intra-mode links, body HTML, derived pages, feed membership, artifacts | `ModeOutput` | read other modes, load templates, write files |
| **Site builder** (`folia::site`) | all `ModeOutput`s, static file list, config | assemble: route table and duplicate/collision checks, redirects, combined feed, search documents, Atom entries, sitemap, `site` index | `Site`: every `PageSpec` plus every serialized non-HTML output, keyed by `Route` | interpret mode front matter, render templates |
| **Renderer** (`folia::render`) | `Site`, `templates_dir` | check template names (§6.4), then render each `PageSpec` with its context, `content`, and `site` | `Route` → HTML bytes | change routes or membership, read sources |
| **Output** (`folia::output`) | rendered and serialized outputs, static and Sass outputs, destination | post-render link validation; write to a temporary sibling directory; swap it into place atomically; deterministic order | the destination directory | leave a partial destination on failure |

The builder emits feeds, search documents, and the sitemap as *data*.
Their serialization (Atom XML, Lunr JS) is done by modules of `folia`
that the builder calls. They are not separate stages.

## 7. Site assembly, feeds, search (`folia` crate)

### 7.1 Site assembly [DECIDED]

These belong to the `site` module, because they need every mode's
output:

- Route table, duplicate-route and output-collision detection. Redirect
  sources count as routes.
- Redirect stubs and `redirects.json`, built from `redirect_from`.
- The read-only `site` index for templates.
- `sitemap.xml`.
- **Post-render link validation:** scan final HTML for `href` and `src`
  values and check them against the route table and asset list. This
  also catches broken links in templates, not only in content.
- **Navigation stays in the template.** It is hand-written today, and
  nothing requires modelling it. [LATER, only if a concrete need arises]

### 7.2 Content migration (one-off, done in `filopoe`) [DECIDED]

A script removes Liquid from Markdown by replacing each construct with
its literal production expansion (spike 3, §10). Then rebuild **with
Jekyll** and check that the output is unchanged. This proves the
rewrite is behavior-neutral before Folia exists.

The graph `include` is expanded inline, not replaced by a front-matter
flag as v1 proposed. It sits mid-paragraph near the end of
`_digital_garden/home.md`, so moving it into the layout would change
its position. The include's output is static HTML, so expanding it
verbatim keeps the output identical.

Until the switch to Folia, Jekyll still runs Liquid over content, so new
content must stay Liquid-free. A small content lint (`{%` or `{{`
outside code) enforces this.

**Content lint [DECIDED 2026-10-26].** Status: built and tested in Folia, not yet pushed; its CI
workflow is written but untracked in `filopoe`, with its pin `PENDING`, so it is not active
(below). Folia's `docs/baseline.md` documents it.

- **Command:** `parity liquid --site SITE --config CONFIG` (`just liquid` for the reference site).
  It is a check of the site's content, not a parity check: it needs no baseline.
- **Scope: Markdown only.** The body, after the front matter, of every Markdown document Folia's
  discovery finds (§7.1): not excluded files, hidden directories, or Markdown files without
  front matter. HTML pages, layouts and includes also go through Liquid in Jekyll and may hold
  Liquid Folia does not run; they are not checked (Q13).
- **Rule:** every `{{` and `{%` in the source as Liquid reads it fails. Jekyll runs Liquid before
  kramdown, so raw HTML, HTML comments, HTML's `<code>` and Markdown escapes hide nothing.
- **Exempt: code, as Folia's parser reads it** (`markdown::code_ranges`): fenced code blocks
  (backticks or tildes), indented code blocks, and code spans. Indented blocks are code because
  the parser makes them code. A code span kramdown keeps as text, as inside `<u>`, is not code.
- **Reported** as `file:line:column` (characters, lines from the top of the file); exit 1 on
  findings, 2 when it cannot check.
- **Limit, until the switch:** the lint keeps Liquid out of prose, which is Folia's contract. It
  does not make the Jekyll site right: Jekyll runs Liquid inside code too, so a Liquid example in
  code may still need `{% raw %}` there. The lint neither asks for it nor adds it, and existing
  content is not changed.
- **Result:** all 190 Markdown documents of `filopoe` pass at its local `SITE-COMMIT-B` (and at
  `SITE-COMMIT-A`), unchanged. Its public `main`, `SITE-MAIN`, fails: 363 delimiters in 14 documents,
  exactly the Liquid spike 3 removed. The 15th rewritten file, `webmentions`, held its 4 inside a
  fenced example, which is exempt (and which Jekyll evaluated). So the CI can only pass once
  `SITE-COMMIT-A` is published; the lint is not to be weakened for the unmigrated branch.
- **CI:** `filopoe/.github/workflows/content.yml` runs the command on pushes to `main` and pull
  requests that change Markdown or the workflow, and by hand. It checks the site and Folia out
  side by side, so discovery never walks Folia's files, and uses Folia's `parity/filopoe.toml`.
  Folia must be pinned to a full commit SHA, never a branch or tag (`FOLIA_COMMIT`); the pin is
  `PENDING`, which fails the job, until the lint is on Folia's `main`. A newer check or
  configuration is adopted by moving the pin, which reruns the workflow. The parity tools'
  checkout check ignores this one file, as it does `_folia/` (§12.2).
- **To activate:** push the lint to Folia's `main`; set `FOLIA_COMMIT` to that commit's full SHA;
  commit the workflow in `filopoe` and publish it, which is a separate decision because of the
  site's unpublished commits; then see a run pass, and record it in the next plan.

### 7.3 Feeds [DECIDED]

- Each mode declares its own feeds and their membership as `FeedSpec`s:
  blog, per-tag, micro, per-kind micro, and garden.
- The **combined** feed is declared in `folia.toml`
  (`include = ["blog", "garden", "micro"], limit = 10`). The site
  module assembles it from `EntrySummary` values, ordered by
  `published` (newest first) and truncated to `limit`.
- The `feeds` module builds Atom entries from `EntrySummary` and
  serializes them, either with `atom_syndication` or a template. It
  depends only on `folia-core`, so it never depends on a mode.
- Only Atom is needed; the current feeds are all Atom.

**Timestamps** [DECIDED]:

- **Entry `modified`**, resolved by the mode from `content` data, uses
  the first available of:
  1. front matter `last_modified_at` (4 posts set it today);
  2. the date of the last commit reachable from `HEAD` that touches the
     source file;
  3. `published`.

  Against Jekyll today (§2.6):
  - **Documents:** identical (front matter, then git).
  - **Pages:** Jekyll ignores front matter and uses git. No page sets
    the key, so no output differs. Folia applies one rule to every mode
    [OPEN, recommended]; if a page ever sets the key, the difference
    becomes a ledger entry. Until this is decided, the pages mode refuses a page that sets
    it (step 1 of the slice).
  - **Uncommitted files:** Jekyll uses the file's mtime; Folia uses
    `published`, and never reads mtimes. Parity cannot show this, because
    baselines never contain mtime dates (§2.6).

  Git dates come from committed history (`HEAD`), never from the working
  tree, so a dirty tree builds the same dates as its commit.
- **Entry `<updated>`** = `modified`. **Entry `<published>`** =
  `published`. Both match Jekyll today.
- **Feed `<updated>`** = the maximum `modified` over the entries
  *actually included* in that feed, after ordering and the limit. Editing
  an older post that is still in the feed's top 10 therefore advances the
  feed's `<updated>`. Editing a post that has dropped out of the feed does
  not, which is correct because the feed's content is unchanged.
- **Fallbacks:** an entry without `modified` contributes `published`. An
  entry with neither is excluded from feeds, with a warning. An empty
  feed uses the maximum `modified` across all entries of the site. It
  never uses build time.
- **Deviation:** Jekyll sets the feed-level `<updated>` to the build time
  (§2.6). Folia's value is deterministic and meaningful. This is
  recorded in the deviation ledger, and the parity tool compares the
  feed-level `<updated>` against this rule, not against Jekyll.

### 7.4 Search [DECIDED]

The site module builds search documents for entries with
`searchable = true` (and for pages, as Jekyll does today). The body is
text extracted from `body_html`, replicating the current
`replace`/`strip_html` chain. It emits a Lunr-compatible `documents`
array with the same `{id,url,title,body}` schema.

- Whether micro entries are included is the §2.5 defect. [OPEN: fix or
  preserve]
- The dead template also gave RSVPs a custom body (date, RSVP status,
  and event details). If micro search is fixed, that requirement returns
  as a micro-mode-provided search text. It is not dropped.

## 8. Rendering [DECIDED: MiniJinja, approved 2026-10-25]

| | `liquid` crate | MiniJinja | Askama |
|-|-|-|-|
| Reuse of current templates | Partial. Jekyll filters, Jekyll `include` params, and lenient unknown filters are not standard Liquid, so a compat layer is needed | Rewrite (~1.9k lines; the TOC and class hacks disappear) | Rewrite |
| Expressiveness | Weak (no macros or real expressions), which is why the logic ended up in `assign` chains | Inheritance, macros, expressions, `loop` | Full Rust |
| Diagnostics | Basic | Line/column with source excerpt; strict undefined catches typos | Compile errors |
| Iteration | Runtime | Runtime | Recompile per template edit |

- Templates will be rewritten anyway to consume computed data (§3). So
  "reuse Liquid" mostly means rebuilding Jekyll compatibility, which is
  a v0 non-goal.
- MiniJinja is actively maintained, serde-based, and makes
  `{{ content | safe }}` explicit.
- **Downside:** no line-by-line template diff against Jekyll. Parity is
  verified on output instead (§9).
- **Approved** on 2026-10-25, and used since step 1 of the slice: MiniJinja 3.0.3 (Apache-2.0,
  `cargo deny` clean), with only its `builtins`, `debug`, `multi_template` and `serde` features.
  `folia::render` sets it up as §6.4 says: its own loader below `templates_dir`, which rejects
  absolute names and `..`, `.` or empty segments; strict undefined, so even a test of a missing
  key fails; `trim_blocks` and `lstrip_blocks`. HTML is escaped by MiniJinja's rules, which also
  escape `/` (`&#x2f;`); parity decodes entities, so this does not show. One filter is Folia's:
  `date(format)`, which rewrites the times modes give (`2026-01-04T13:36:24+01:00`).

## 9. Testing and parity

1. **Baseline:** produced by the spike 0 harness (§10): a production
   build plus per-document dumps, at a recorded `filopoe` commit, in a
   directory outside both repos. The checked-in `_site` is a dev build
   and is invalid as a baseline.
2. **`parity` tool** (dev-only; reads two directories):
   - **Route inventory:** set equality of output paths, plus
     `redirects.json` and the sitemap URL set.
   - **Per-page fingerprint:** title, canonical, OG meta, headings
     (text + id), link and image sets, and main-content text with
     whitespace normalized. As built in step 1 of the slice (`parity pages`): the head values
     (title, canonical link, description, Open Graph), the headings in order, and the `a href`,
     `img src` and resource (`link href`, `script src`) multisets come from the whole page; the
     main content is `div.main-content`; and the whole body's text is compared too, so the
     site's frame (category counts, footer year) cannot drift unseen.
   - **Normalized DOM diff** of the main content region. Parse with
     html5ever, drop insignificant whitespace, sort attributes and
     classes, and ignore configured attributes. Spike 1's normalizer, applied to
     `div.main-content` (comments included); no attribute is ignored so far.
   - **Other outputs** Folia builds, redirect stubs and static files, are compared byte for byte.
   - **Artifacts:** compare feeds parsed as entries (link, title,
     published, updated, categories, order). Compare graph nodes and
     edges as multisets: each link occurrence is an edge, so a repeated
     link is a repeated edge. Compare the Lunr index as a set of `(url, title)`
     plus body similarity.
3. **Markdown regression tests:** real snippets of the site, as unit
   tests in `folia-core`. Their expected HTML is what the site's own
   Jekyll converter makes of each snippet, so they need neither `filopoe`
   nor a snapshot crate. The full check is `just markdown`.
4. **Deviation ledger:** `parity/deviations.toml` lists each
   intentional difference with a rationale. The tools suppress only
   accepted entries; a failure a proposed entry explains is reported as
   pending and still fails. An entry names the Markdown gates it
   explains (`markdown_gates`), and may require a check
   (`markdown_check`).
5. **Visual checks:** screenshot a handful of pages at two viewports and
   review them manually. [LATER]

## 10. Validation spikes

Spikes 1–3 address the three risks that are most expensive to get
wrong. Spike 0 is the shared harness they all depend on. Each spike's
code lands where it will eventually live, and only the code the spike
needs. Its checks stay as rerunnable regression tests.

Conventions used below:

- `FILOPOE=../filopoe` is the Jekyll site.
- `BL=../folia-baselines` is outside both repos.
- Deviations are recorded in `parity/deviations.toml` as `D-NNN` (§9.4).

### Spike 0: baseline harness (prerequisite)

**Status: done** (2026-10-25). All three checks pass; see the results at
the end of this section and [parity baselines](../baseline.md).

- **Behavior:** a reproducible production Jekyll build that also dumps
  Jekyll's own intermediate results. This avoids reverse-engineering
  them from page HTML.
- **Mechanism:** a passive Jekyll plugin,
  `parity/jekyll/folia_dump.rb`, in the Folia repo. It is loaded with
  `--plugins _plugins,<folia>/parity/jekyll`, so `filopoe` is not
  modified. It does nothing unless `FOLIA_DUMP=<dir>` is set. If
  `--plugins` turns out not to accept a list, the fallback is an extra
  `--config` file that sets `plugins_dir` to a list.
- **Dumps:**
  - `manifest.jsonl`: one line per page, document, and static file, with
    `{kind, collection, source|null, url, output}`. Generated pages
    (pagination, archives, feeds, redirect stubs) are included.
  - `content/<source>.html`: each document's HTML after Liquid, the
    wiki-link plugin, and kramdown, but before the layout. Captured with
    a `post_convert` hook.
  - `excerpt/<source>.html`: the rendered excerpt.
  - `meta/<source>.json`: title, date, `last_modified_at`, categories,
    and tags. `last_modified_at` is recorded with its origin
    (`front_matter`, `git` or `mtime`) and the raw front matter value, so
    §2.6 can be checked from the dumps.
- **Command:**
  `scripts/baseline.sh [--allow-dirty] $FILOPOE $BL/<label>`. The
  script:
  - refuses to run on a dirty tree unless `--allow-dirty` is given;
  - records the `filopoe` commit, the declared and present submodules,
    and Jekyll and Ruby versions in `BASELINE.json`;
  - runs `JEKYLL_ENV=production bundle exec jekyll build --destination
    $BL/<label>/site --disable-disk-cache`.

  `scripts/compare-builds.sh A B [expected-diffs]` compares two
  baselines. It normalizes only the feed-level `<updated>` (§2.6).
- **Success:**
  1. Two runs at `SITE-MAIN` produce byte-identical dumps, and identical
     sites after normalization.
  2. The site built with the plugin is identical to one built without
     it.
  3. The manifest accounts for all 1,077 output files.
- **Results:**
  - All three pass (`just baseline-verify SITE-MAIN`). Two runs give
    byte-identical dumps, and sites that differ only in the 52 feed-level
    `<updated>` values. A build without the plugin, and one with the
    plugin loaded but `FOLIA_DUMP` unset, give the same site.
  - Every build is also checked to leave `filopoe`'s working tree as it
    was, and to contain no mtime-derived date.
  - Generated pages are told apart by class: only plain `Jekyll::Page`
    instances and documents count as read from a file. A stale, ignored
    `assets/json/notes_graph.json` in the working tree had made the graph
    page look sourced.

### Spike 3: Liquid removal (run first after spike 0; Jekyll-only, no Rust)

**Status: done and accepted.** D-001 was accepted (Q1) on 2026-10-25,
and the rewrite committed in `filopoe` as `SITE-COMMIT-A`. The evidence is
kept in `parity/evidence/spike3/`.

- **Behavior:** every Liquid construct in the 15 content files (§2.2) is
  replaced by its literal production expansion. Jekyll's per-document
  converted HTML stays unchanged, except for the listed deviation.

| Construct | Occurrences | Rewrite | Expected change |
|-|-|-|-|
| `{% assign v = "…" %}` + `{{v}}` | 100 / ~200 | inline the literal | none |
| `{{ site.baseurl }}`, `"…" \| prepend: site.baseurl` | ~36 | literal path (baseurl is empty in production) | none |
| `{{ site.url }}`, `site.paths.home`, `site.webmentions.*`, `{{ page.url }}` | 5 | literal production value | none |
| `{% highlight lang %}…{% endhighlight %}` | 11 in `_posts/2020/2020-01-26-sample-code.md`, `_posts/2021/2021-08-17-sample-speed.md` | fenced code block, language lowercased (Rouge's tag, as Jekyll labels it) | **D-001:** wrapper markup changes from `figure.highlight > pre > code[data-lang]` to `div.language-x.highlighter-rouge > div.highlight > pre.highlight > code`, and the code gains one trailing newline (kramdown keeps it in every fenced block; all 40 existing ones have it). Language, code text and Rouge tokens are otherwise identical; visual check needed |
| `{% include digital-garden/notes_graph.html %}` | 1 | the include's 22 lines of HTML, verbatim (`link_classes` is empty) | none |

- **Tool:** `parity/codemods/deliquify.py`, a stdlib-only one-off run
  with `python3 -I`.
  - It rewrites only the patterns in the table above. Any other `{%` or
    `{{` makes it fail with the file and line.
  - Production values are an explicit table in the script.
  - It is idempotent. `--check` prints a diff; `--write` applies it.
  - A run of `assign`-only lines renders as empty lines, so it becomes
    exactly one blank line (none if one is already adjacent).
  - The include is expanded byte for byte, including its trailing
    newline, which leaves one double blank line in
    `_digital_garden/home.md`.
- **Commands:**

  ```sh
  just baseline-verify SITE-MAIN       # clean baseline, with the spike 0 checks
  just deliquify check               # review the diff
  just deliquify write
  just baseline deliquified --allow-dirty
  just compare SITE-MAIN deliquified parity/spike3-expected.txt
  just check-d001 SITE-MAIN deliquified
  ```

  The "after" build runs on the uncommitted tree on purpose. Git dates
  then stay unchanged, because `jekyll-last-modified-at` reads committed
  history (§2.6). Otherwise the 15 files' dates would move and pollute
  the comparison.
- **Timestamp protection:** the comparison never normalizes dates. Meta
  dumps (each date and its origin), `sitemap.xml` `lastmod` values and
  feed entry `<updated>` values are compared byte for byte. Any build
  containing an mtime-derived date fails outright. The codemod rewrites
  files in place (same inode), so directory order is unchanged, and
  touches no static file.
- **Success:**
  - No Liquid remains: `grep -rE '\{%|\{\{'` over content returns
    nothing.
  - Every per-document dump is byte-identical, except the 2 D-001
    files. For those, the code text and all text outside code is equal.
  - Every site file is identical after normalization, except files
    listed in `spike3-expected.txt`, each traced to D-001.
  - Any difference that is not explained fails the spike.
- **Results:** all pass.
  - No Liquid remains in any tracked Markdown file, and a second
    `--check` finds nothing to do.
  - The dumps differ in 4 files: the content and excerpt of the two
    D-001 posts. All 201 meta files and the manifest are byte-identical.
  - The site differs in 5 files: the two post pages, and the tag feeds
    `R.xml`, `plot.xml` and `python.xml`, which carry the posts' full
    content. The Lunr index, archives and pagination are unchanged.
  - `parity/codemods/check_d001.py` shows each of the 9 differs by D-001
    only: 11 converted blocks, and all other text and the other posts'
    fenced blocks byte-identical.
- **Afterwards:**
  - Done: the change is committed in `filopoe` (`SITE-COMMIT-A`), alone; D-001
    is accepted in the ledger, scoped to the two posts and the nine
    outputs; the reference baseline is `BASELINE-A`.
  - Committing moved the git dates of the 13 rewritten files without a
    front matter date, and that is the only difference from the verified
    build (`parity/check_date_shift.py`; evidence in
    `parity/evidence/rebaseline-BASELINE-A/`).
  - The content lint from §7.2 is built; its CI in `filopoe` is not active yet (§7.2).

### Spike 2: route parity

**Status: done** (2026-10-25). Route parity holds against `BASELINE-A`; see
the results at the end of this section and
`parity/evidence/spike2/`.

- **Behavior:** Folia's discovery and route rules reproduce Jekyll's
  `(url, output)` for every output:
  - classification of each source file (mode document, page, static,
    excluded via `_config.yml` `exclude`, `_` directories, dotfiles);
  - routes for authored documents in each mode;
  - explicit `permalink`s, including `/` and permalinks ending in
    `.html` (`/blog/tags.html`);
  - the URL-to-output policy (`/x` to `x.html`, `/x/` to
    `x/index.html`, garden `/dg/a/index` to `dg/a/index.html`);
  - generated routes: pagination, category archives, per-tag, per-mode
    and combined feeds, redirect stubs, the sitemap, `redirects.json`,
    the graph JSON, the Lunr JS, and Sass outputs.

  The *content* of generated outputs is out of scope.
- **Code under test:**
  - `folia_core::content::Route` (URL and `output_path()`);
  - `folia::discover`;
  - a route function in each mode crate;
  - `folia_blog` generated routes.

  Nothing renders.
- **Fixtures:** the whole spike 0 manifest, plus named edge cases kept
  as unit tests with expected values copied from the manifest:

| Source | URL | Output |
|-|-|-|
| `_posts/2023/2023-05-13-Sample-Trip-pt1.md` | `/blog/2023/05/13/May-train-trip-pt1` | case kept |
| `_micro/bookmarks/2025-06-16-sample_bookmark_entry.md` | `/bookmarks/2025-06-16-internet-resiliency-club` | `_` slugified |
| `_digital_garden/watch/topic_note.md` | `/dg/tracking/chat_control` | `_` kept |
| `_digital_garden/topic/index.md` | `/dg/coding/index` | `dg/coding/index.html` |
| `profile/index.html` (`permalink: /`) | `/` | `index.html` |
| `pages/rss.xml` (`permalink: /feed.xml`) | `/feed.xml` | `feed.xml` |
| `blog/index.html` + pagination | `/blog/`, `/blog/page2/` … | `blog/index.html`, `blog/page2/index.html` |
| `pages/profile.md` `redirect_from: /blog/about` | `/blog/about` | `blog/about.html` (stub) |

- **Commands:**
  - `cargo test --workspace` runs the edge cases and needs no baseline.
  - `cargo run -p parity -- routes --site $FILOPOE --baseline $BL/<commit>`
    reports missing, extra, and mismatched `(url, output)` pairs.
- **Success:**
  - 100% of `(source, url, output)` match for authored documents, pages,
    and static files.
  - The set of generated output paths matches exactly, and so does the
    set of redirect stubs.
  - Folia's route table has no duplicates or collisions.
  - Any intentional difference is a ledger entry. None are expected.
- **Results:** all pass, with no ledger entry.
  - Authored outputs: 985 (source, URL, output) on both sides,
    identical. Classification of the 977 sources: identical (79 posts,
    39 micro entries, 54 garden notes, 30 pages, 775 static files).
  - Generated outputs: 92 (URL, output) on both sides, identical; the 26
    redirect stubs among them, identical. No duplicate URL or output
    file.
  - Negative tests fail as they should: a changed URL in the manifest, a
    missing or extra exclusion, and a baseline from another commit.
  - It also holds on a fresh clone of `SITE-COMMIT-A`.

#### 10.2a Rules spike 2 settled [DECIDED]

Read from Jekyll's and the plugins' source, then verified by parity:

- **Discovery.** A name starting with `.`, `_`, `#` or `~`, or ending
  with `~` or `.`, is hidden, with everything below it; `include`
  overrides. Exclusions are explicit globs in `folia.toml` (`*` stays in
  one segment). Jekyll also matches its patterns as string prefixes
  (`LICENSE` excluded `LICENSE-*.txt`); Folia does not, and the site's
  configuration says `LICENSE*` instead. Below the blog root only dated
  file names are posts. Below the micro and garden roots, files without
  front matter are rejected for now, since none exist.
- **Routes.** As in each mode crate's route table: posts
  `/blog/:year/:month/:day/:title` (pretty slug, case kept); micro kinds
  by directory, `:name` slugified; garden `/dg/:path`; pages by
  permalink, else `/:dir/:name` (HTML), `/:dir/` (index) or
  `/:dir/:name.ext`. Output extensions: Markdown → `.html`, Sass →
  `.css`, a permalink's extension wins.
- **Generated outputs.** Listing pages 2..N (9 per page, hidden posts not
  counted) from `blog/index.html`; category archives at
  `/blog/category/` and the category's slug; tag feeds only for tags of
  letters, digits and `_`; mode feeds; redirect stubs; Sass source maps;
  and the sitemap, `robots.txt` and `redirects.json` unless a file takes
  the URL.
- **Publishing.** `published: false` drops a document; posts and micro
  entries dated after the build by their file name, and garden notes by
  their front matter `date`, are held back (Jekyll compares against the
  build time; parity uses the baseline's).
- **Stricter than Jekyll, on purpose** (none occurs in the content):
  four-digit years in file names, literal permalinks only, no front
  matter `date` on posts, and garden dates as `YYYY-MM-DD[ HH:MM[:SS]]`.
- **Open:** the combined feed (`pages/rss.xml`) and the Lunr index are
  Jekyll pages, so Folia plans them as pages for now; §7.3 and §7.4 make
  them generated outputs, which will claim those sources. Listing
  ownership is still Q4.

### Spike 1: Markdown compatibility

**Status: done and closed** (2026-10-25). Against the clean baseline
`BASELINE-B` (below) every hard gate passes, G5 with the accepted D-003
only; G4 is at 100%, and the G6 highlighting rule is met (96.3%), which
decides the highlighter (Q12). The evidence is in
`parity/evidence/spike1/`.

- **Behavior:** `folia_core::content::markdown::to_html` turns the
  de-Liquefied sources into the body HTML in the spike 0 `content/`
  dumps, and `markdown::excerpt` gives the excerpts in `excerpt/`, for
  these features:
  - **1a heading IDs**, including punctuation, non-ASCII, emoji, and
    duplicates;
  - **1b IAL subset**, span and block;
  - **1c reference and shortcut links**;
  - **1d smart typography**;
  - **1e excerpts**;
  - **1f syntax highlighting**;
  - **1g raw and inline HTML**;
  - **1h wiki-links.** The spike's resolver
    (`folia_garden::WikiLinks`) is built from the manifest's garden URLs
    and Jekyll's titles (the meta dump, which includes the titles Jekyll
    makes for notes without one), and mirrors the Ruby rules: title
    first, then file name with `-` read as a space, both ignoring case
    and surrounding whitespace; the later note wins a shared key.
- **Scope:** all 190 Markdown documents (79 posts, 39 micro entries, 54
  garden notes, 18 pages) and the 172 documents' excerpts, not only the
  representative files v4 listed; those are covered by the regression
  tests (§9.3).
- **Measurement:** `parity` parses both fragments with html5ever and
  normalizes them:
  - collapses whitespace outside `pre`, and drops it where blocks meet;
  - decodes entities, so `&rsquo;` and `’` compare equal;
  - sorts attributes and class tokens;
  - compares code blocks by text only (highlighting is gated
    separately, below).

  Each document is reported as *identical*, *equal after
  normalization*, or *differs*, with the first differing node path.
- **Gates:**
  - **G1 heading IDs:** 100% equal per document, in order. Hard gate.
  - **G2 links and attributes:** the `href`/`src` multisets are 100%
    equal, and every element's `class`, `target` and `title` (in
    document order, with what it links to) are equal. Hard gate.
  - **G3 text:** normalized text is 100% equal, except ledger entries.
    Hard gate.
  - **G4 structure:** at least 95% of documents are equal after
    normalization, and every remaining difference is categorized as
    "fix" or "ledger". Decision gate.
  - **G5 excerpts:** the values templates actually consume match for
    100% of documents, byte for byte:
    - `strip_html | strip_newlines | truncate: 160` (page description);
    - `strip_html | truncatewords: 30` (cards);
    - `strip_html | normalize_whitespace` (feed summaries; v4 missed
      this one);
    - for micro entries, the raw excerpt on `/micro`, compared as HTML
      after normalization.

    The filters are reimplemented from Liquid 4.0.4 and Jekyll 4.4.1.
    Because `strip_html` keeps whitespace, this gate requires kramdown's
    exact layout of blank lines and indentation, which Folia reproduces.
  - **G6 highlighting:**
    - Code text is 100% equal. Hard gate.
    - Token-class agreement against Rouge is reported per language,
      counted on non-whitespace characters of the blocks Rouge lexes.
    - **Decision rule:** if a Pygments-class emitter reaches about 90%
      agreement on the fixture files, keep `_syntax.scss`. Otherwise
      generate CSS for the chosen highlighter and visually check those
      pages.
- **Commands:**

  ```sh
  just baseline-verify BASELINE-B               # filopoe SITE-COMMIT-B, with the D-002 fix
  just check-date-shift BASELINE-A BASELINE-B      # Jekyll's output unchanged but one git date
  just markdown BASELINE-B [--report FILE] [--doc PATH] [--dump DIR]
  just markdown-negative BASELINE-B             # must fail, gate by gate
  just routes BASELINE-B
  cargo test --workspace --locked            # no site needed
  ```

  `just markdown` runs `cargo run -p parity -- markdown --site $FILOPOE
  --baseline $BL/<label>` and reads the ledger from
  `parity/deviations.toml`. It exits 1 while any failure is unexplained
  or explained only by a proposed entry, or an entry lists a source that
  does not show it.
- **Uncommitted site changes.** A source fix is verified before it is
  committed, as spike 3's rewrite was. A baseline built with
  `--allow-dirty` records `git diff --binary HEAD` as `site.diff`, and
  `parity` accepts a checkout with uncommitted changes only when they
  are exactly that diff and there are no untracked files; a dirty
  baseline without `site.diff` is refused (`site-diff-check.txt`). The
  D-002 fix was verified that way, as `BASELINE-A-d002`
  (`just compare BASELINE-A BASELINE-A-d002`), then committed in `filopoe` as
  `SITE-COMMIT-B` and baselined clean as `BASELINE-B`, which differs from it by
  the post's git date only (`parity/evidence/rebaseline-BASELINE-B/`).
- **Results** (`just markdown BASELINE-B`, exit 0):

  | Gate | Kind | Result |
  |-|-|-|
  | G1 heading IDs | hard | 190 of 190 documents |
  | G2 link targets and attributes | hard | 190 of 190 |
  | G3 text | hard | 190 of 190 |
  | G4 structure | decision, 95% | met: 190 of 190 (100%) equal after normalization, 135 byte-identical |
  | G5 excerpt values | hard | 153 of 172 equal; 19 are D-003 (accepted), each passing its check |
  | G6 code text | hard | 77 of 77 blocks |
  | G6 token classes | decision, about 90% | met: 96.3% of 11,444 characters (Python 97.8, JavaScript 96.5, HTML 98.8, TOML 99.5, Bash 87.1, YAML 84.5, R 99.8, Rust 100, email 0) |

  - Before the review, against `BASELINE-A`: G3 and G4 189 of 190 (D-002),
    and G5 161 of 172, because excerpts were then compared unresolved.
    At the first review, G6 token classes were 83.2%.
  - The checks were shown to fail when they should: a copy of the
    baseline changed once per gate, and a ledger with D-003 narrowed
    and widened, fail exactly those checks (`negative.txt`).
  - Route parity (spike 2) still holds (`routes.txt`).
- **Deviations** (`parity/deviations.toml`):
  - **D-002**, CommonMark emphasis [withdrawn]: the one `eggs**!**` is
    now `eggs<strong>!</strong>`, committed in `filopoe` as `SITE-COMMIT-B`.
    Jekyll's output is unchanged, dumps byte for byte
    (`d002-compare.txt`), and Folia matches it with no deviation. The
    entry stays, withdrawn, so its number is not reused; a withdrawn
    entry explains nothing.
  - **D-003**, garden excerpts [accepted 2026-10-25, narrowed
    2026-11-01]: Folia renders an excerpt with the body's wiki-link
    resolver (`markdown::excerpt_html`), so the rendered excerpts of
    listings, cards and feed summaries show link text instead of `[[…]]`
    or tables; no step builds them yet. A note page's description (its
    `description` and Open Graph meta) is not covered: it keeps the raw
    wiki-links, as Jekyll's does, for parity (step 3, §12.2,
    `coding/index.md`'s ends `[[Version Con...`). It lists the
    19 notes with wiki-links (the first count, 18, compared unresolved
    excerpts and missed `indieweb/index.md`, where `[[blogroll]]`
    becomes a reference link). It applies to the G5 excerpt comparison
    only. A note's G5 difference counts as D-003
    only when resolving changes its excerpt, Jekyll's excerpt differs
    from Jekyll's body, and Folia's excerpt values equal those of
    Jekyll's body (its excerpt is its body; the plugin resolved the
    body). Other notes must match Jekyll exactly, and a listed note
    that stops showing it fails. Regression tests: the `excerpt_html`
    tests in `markdown/tests.rs`.
- **Highlighting** [DECIDED, Q12, 2026-10-25]: syntect, pure Rust with
  `fancy-regex`, using bat's grammars (`two-face` 0.5.2, a syntect dump
  of bat 0.26.1's syntaxes), with each token mapped to Rouge's Pygments
  short class, so the site keeps `_syntax.scss`.
  `highlighting/README.md` has the review.
  - The first review measured 83.2% by class and 89.5% by rendered
    style: short of the 90% rule, mostly because syntect's own grammars
    have no TOML. This round changed two things, and nothing in the
    measurement: bat's grammars for every language (TOML included), and
    mapping rules each copied from a rule of Rouge 4.7.0's lexers
    (Python calls, `def`/`class`/`lambda`, `self`, `in`, `->`; HTML tag
    delimiters, `=` and `<script>` content; Bash options, assignments
    and `|`; TOML keys, `=` and table brackets).
  - By class, 96.3% agree with Rouge, on the same 11,444 characters,
    TOML and the e-mail block included: the 90% rule is met. By
    rendered style under the site's compiled CSS, 98.0%.
  - On 109 files of code it was not tuned on (Rouge's demos, Folia's
    own scripts and configuration, `filopoe`'s layouts and JavaScript),
    agreement went from 74.0% to 87.8% by class and from 84.2% to 93.9%
    by style, improving or holding in every language: the rules
    generalize. Weak spots remain in languages the site barely uses
    (YAML keys, Rust calls and doc comments, Bash variables in strings).
  - `_syntax.scss` has a rule for every class Folia emits, except `n`,
    `nx` and `p`, which it leaves plain for Rouge too; it sets only
    color, weight, slant and background on tokens, which the style
    measure compares.
  - Screenshots of all 19 documents: TOML is highlighted as Rouge does;
    what remains is a few characters per block (magic-method names after
    `def`, JavaScript builtin methods, Bash `;`, `<placeholder>` and
    GRUB `=`, the one e-mail address), visible side by side, not alone.
  - Not chosen: tree-sitter (`arborium`: C builds, a mapping to write
    and tune from scratch, to beat 96.3%), browser-side highlighting
    (JavaScript on every page with code, new markup and stylesheet), a
    generated stylesheet (no class lacks a rule), and giallo (EUPL
    licence, Oniguruma).
- **`bincode` 1** [DECIDED with Q12]: `cargo deny` reports
  RUSTSEC-2025-0141, and nothing else, without the exception.
  - It is informational ("unmaintained"): bincode's authors stopped
    after a harassment campaign, consider 1.3.3 complete, and list no
    vulnerability or patched version.
  - Path: `bincode 1.3.3` ← `syntect 5.3.0` ← `folia-core`, directly
    and through `two-face`. syntect's `parsing` feature enables
    `dump-create`, and `two-face` enables `dump-load`; both need
    bincode, so no feature set of syntect 5.3.0, the latest release,
    avoids it. It decodes only the syntax set compiled into the binary
    (`include_bytes!`), at build time, never outside input.
  - `deny.toml` ignores that advisory by ID only, and bans `bincode`
    except through syntect (`wrappers`), so no other crate can bring it
    in under the exception. Revisit when syntect drops bincode 1, or if
    an advisory reports a vulnerability.

#### 10.1a Rules spike 1 settled [DECIDED, approved 2026-10-25]

Read from kramdown 2.5.2, kramdown-parser-gfm 1.1.0 and Jekyll 4.4.1,
then verified by parity:

- **Heading IDs.** The heading's text after typography and entities,
  lowercased; characters other than letters, marks, decimal digits,
  connectors, `-`, space and tab dropped (so an emoji's variation
  selector stays); spaces and tabs become `-`; repeats get `-1`, `-2`.
- **Typography.** kramdown's ordered quote rules, which look at the
  character before the quote only when nothing else took it, and ahead
  into the raw source; `---`, `--`, `...`. No `<<`/`>>` (none in the
  content).
- **Raw HTML.** Markdown inside an inline element is parsed only for
  kramdown's span content model (`a`, `em`, `small`, `span`, …); `<u>`
  keeps its source. `<x />` is an empty element. A line starting with an
  inline element is a paragraph. An element left open is closed at the
  end of its paragraph.
- **Lists.** An item's first paragraph gets `<p>` when a blank line
  follows it inside the item (per item, not per list); the last item
  follows the earlier ones.
- **Layout.** kramdown's whitespace: one line break per run of blank
  lines between blocks (IAL lines and link definitions do not split a
  run), two spaces of indentation per level, and a final newline added
  to the source.
- **Code.** `div.language-x.highlighter-rouge > div.highlight >
  pre.highlight > code`, language as written; code spans get
  `class="language-plaintext highlighter-rouge"`; `~text~` is not
  struck through.
- **Links.** URLs are written as given, not percent-encoded; an image's
  `alt` is its source text with escapes removed.

Approved on 2026-10-25 as the dialect Folia supports: these rules, the IAL subset of §5.3, and
no more. Each is held by the regression tests of `markdown/tests.rs` and by `just markdown`
(§10, spike 1); widening the dialect needs new evidence, and a ledger entry where Jekyll's output
would change.

### Later risks (unchanged; after spikes 0–3)

- **4. Template engine.** Port `default`/`post` (with a computed TOC) to
  MiniJinja and render one post. Run the `parity` fingerprint on it.
- **5. Sass.** Compile `main.scss` and `dg_styles.scss` with `grass`.
  The fallback is shelling out to `dart-sass`.
- **6. Git dates.** Compute `modified` (§7.3) from a single `git log`
  pass. Compare it with `meta/*.json` `last_modified_at` and its origin
  from spike 0.

Syntax highlighting, v1's separate risk 5, became gate G6 of spike 1.

## 11. Build pipeline [DECIDED]

1. Load `folia.toml`.
2. Discover files. Assign content to modes by directory; everything else
   is a static file.
3. Read front matter and body, filename dates, and git dates (one pass).
4. **Modes** (independent; can run in parallel): typed metadata,
   validation, routes, intra-mode resolution (garden wiki-links,
   backlinks, graph), body HTML, derived pages, feed specs, artifacts.
5. **Site assembly:** route table and duplicates, redirects, combined
   feed, `site` index, search documents, sitemap. Serialize feeds, the
   search index, the sitemap, `redirects.json`, and artifacts (§6.5).
6. **Render:** check that all referenced templates exist (§6.4), then
   render every `PageSpec`.
7. **Assets:** copy static files and compile Sass. This is independent
   of steps 4–6; only the file list is needed for step 8.
8. **Validate** the final HTML links against routes and assets. Errors
   fail the build.
9. **Write** to a temporary directory, then swap it into the output
   directory atomically. Iteration order is deterministic (`BTreeMap`,
   sorted inputs). No output depends on build time; feed `<updated>`
   follows §7.3.

## 12. First vertical slice (revised)

### 12.1 Spike sequence

Run in the order **0 → 3 → 2 → 1**:

1. **Spike 0** is the harness all the others depend on.
2. **Spike 3** next. It is Jekyll-only and small, and it changes the
   sources. Doing it early means spikes 1 and 2 compare against the
   final, Liquid-free baseline. If Q1 is declined, spike 1 runs on the
   176 Liquid-free documents and excludes the 15.
3. **Spike 2** next. It is cheap, and it creates `Route` and discovery,
   which spike 1's wiki-link resolver uses.
4. **Spike 1** last. It is the largest spike, and its results close Q2
   and the parser and highlighter choices. Done and closed on
   2026-10-25, with Q2, Q10, Q11 and Q12 decided.

Scaffolding is limited to what the spikes need:

- `folia-core` (`content::{route, markdown}`);
- a route function in each mode crate;
- `folia::discover`;
- a dev-only `parity` crate;
- `parity/` scripts and the Jekyll plugin.

No templates, feeds, search, rendering, or CLI until the spike results
are reviewed; they were, and the vertical slice (§12.2) followed.

### 12.2 Vertical slice

Three steps, in order, each a reviewed change of its own. Each step adds only what it needs,
renders through the pipeline of §11, and is checked against baseline `BASELINE-B`. After each,
`just check`, `just routes BASELINE-B` and `just markdown BASELINE-B` must still pass, and work stops
for review. Steps 2 and 3 must not touch each other's crates: a step's diff that touches
`crates/garden` (step 2) or `crates/blog` (step 3) fails the slice's boundary test.

**Step 1: `/about`** (`pages/profile.md`, `layout: page`, `redirect_from: /blog/about`).
**Status: committed and pushed in Folia** (`450db9e6`, 2026-10-26; built 2026-10-25,
when all checks passed). The templates in `filopoe/_folia/` are still uncommitted. What was
built and found follows the acceptance line below.

1. `folia_core::publish`: `PageSpec` (§6.2), and `EntrySummary` with only the fields the page
   uses; the rest arrive with the steps that need them.
2. `folia_pages::build` for pages: route (existing), `layout` → template name (§6.4), body HTML
   from `markdown::to_html`, `redirect_from` routes.
3. `folia::render`: MiniJinja with a path loader rooted at `templates_dir`, rejecting `..` and
   absolute names; strict undefined; every referenced template checked before rendering, all
   missing ones in one diagnostic (§6.4).
4. Templates: port `_layouts/default.html` → `default.html` → `page.html` and the includes they
   need (`base_head.html`, navigation, footer), with Liquid's computed values moved into Rust
   (§3). No other layout.
5. Redirect stubs and assets: the stub `jekyll-redirect-from` writes (meta refresh, script and
   canonical link, §2.1), and static files copied verbatim. Sass stays later risk 5: the page's
   stylesheet URL must be a planned route, not a built file.
6. `folia::output` writes to a temporary sibling directory and swaps it in (§6.5); a `folia
   build` command (clap) runs the pipeline with the site's configuration.
7. `parity`: the per-page fingerprint of §9.2 (title, canonical, OG meta, headings, link and
   image sets, main-content text) and the main-content DOM comparison, reusing the spike 1
   normalizer.

Acceptance: `/about`'s fingerprint and main content equal Jekyll's; `blog/about.html` is
byte-identical to the baseline's; a missing template and a `..` name each fail with the
diagnostic of §6.4; route parity still holds for every output.

*Choices the plan did not cover*, decided by the author on 2026-10-25 before building:

- **The page's frame needs site data.** Every page shows the blog's categories with their post
  counts, the footer's `2012-{{ site.time | date: "%Y" }}`, and the page's last-modified date.
  Computed now: `folia_blog::category_counts` (Jekyll's order: first appearance, posts by date
  then path) feeds the `site` index of §6.3; the year is the build clock's, which publishing
  already takes as input (parity passes the baseline's build time), an exception to §11's "no
  output depends on build time" that Jekyll's output has too; and a page's `modified` is its
  last commit's (`content::LastCommits`, one `git log -n 1` per file), brought forward from
  step 2.
- **The fingerprint's region** is the whole page, and the main content is `div.main-content`
  (§9.2).
- **Templates uncommitted in `filopoe`:** the parity tools' checkout check ignores `_folia/`,
  which neither Jekyll nor Folia's discovery reads, so the checks run against the clean
  baseline while the templates await review. Any other change still fails the check (tested).
  It ignores `.github/workflows/content.yml`, the content lint's workflow (§7.2), for the same
  reason, and only that file: other workflows still fail the check (tested).
- **What `folia build` writes:** `/about` only (`build::SLICE_PAGES`), its stub and every static
  file. The other 300 planned outputs are counted, not built.

*Built:*

- `folia_core`: `publish::{ModeOutput, EntrySummary, PageSpec}`; `content::LastCommits`;
  `SiteClock::{xmlschema, year}`.
- `folia_pages::build`: Markdown pages with a layout and a title; anything else (HTML pages,
  no layout, a front matter `last_modified_at`) is reported as not built yet.
- `folia::render` (§8), `folia::output` (a temporary sibling, then two renames: the
  destination is briefly absent, never partial; it must lie outside the site and not hold it),
  `folia::build` (the pipeline; every output it writes must be in the route table), and
  `site::redirect_stub`, which writes jekyll-redirect-from's page in Rust, unescaped as the
  plugin does.
- `folia build --site S --config C --dest D` (`just build D`). The output directory is an
  argument, not configuration: a destination inside the site is refused. `parity/filopoe.toml`
  gains `templates-dir` and `[site]` (`url`, `title`, `description`).
- Templates in `filopoe/_folia/templates`: `base.html`, `default.html`, `page.html`, and the
  includes they use, with the paths of `_includes`. Includes without Liquid are copied byte
  for byte; the others are ported with only the branches `/about` takes, and say so.
- `parity pages` (`just pages`) and `just pages-negative` (§9.2).

*Results* (baseline `BASELINE-B`):

- `just pages BASELINE-B`: `/about` and `/blog/about` have Jekyll's URLs and files; the
  fingerprint and `div.main-content` are equal; the stub and all 775 static files are
  byte-identical.
- `just pages-negative BASELINE-B` fails, as it should, on a copy of the baseline changed in a
  category count, the footer year, the updated date, a template comment, the stub and a static
  file.
- `just routes BASELINE-B` and `just markdown BASELINE-B` pass unchanged; `just check` and
  `cargo test --workspace --locked` pass, with negative tests for missing, invalid and
  uncompilable templates, a template extending outside the directory, an undefined value, a
  render failure that leaves the previous build, a failed copy that leaves the destination as
  it was, and destinations inside the site, holding it, or a file. They build a fixture site in
  a throwaway repository and need no `filopoe`.

**Step 2: one post**, `_posts/2021/2021-08-08-sample-post.md`: a filename date, a pretty
permalink, IALs on 16 lines, `toc: {enabled: true, position: left}`, and three highlighted
HTML blocks. **Status: committed and pushed in Folia** (`307d09aa`, 2026-11-01, with the
safeguards below; built and checked 2026-10-28). The 28 templates of steps 1 and 2 are committed
in `filopoe` on the branch `folia/templates` (`TEMPLATES-COMMIT-A`), and step 3's four, the note
layout, the graph include and the garden's branches of `base_head.html` and `site_title.html`, in a commit
after it (`TEMPLATES-COMMIT-B`); both are local, not pushed or published. What was built and found follows
the acceptance line below.

1. `folia_blog`: `PostMeta` for the keys the post uses, the date from the file name, the body
   from `markdown::to_html`.
2. `markdown::analyze` (§5.3): headings with level, text and ID, from which Rust builds the TOC
   that `_includes/toc.html` (182 lines) makes from rendered HTML.
3. `modified` (§7.3) for this one file, from front matter, then `git log -1`; the single-pass
   reader is later risk 6.
4. Port `post.html` and the includes it needs.

Acceptance: the post's fingerprint and main content equal Jekyll's; the TOC equals Jekyll's
after DOM normalization; its dates equal the baseline's meta dump (value and origin); the
highlighted blocks keep spike 1's classes; no change under `crates/garden`.

*Choices the plan did not cover*, made while building; the author decided them on 2026-10-28
(*Decisions*, below), and this list says what is built now:

- **Presentation front matter is resolved in Rust** (decision 1, revised). As first built,
  `_config.yml`'s `defaults` were copied into `folia.toml` as `[presentation]` and merged into
  each document's front matter; that is removed. `content::presentation` reads the keys the
  content uses (`toc`, `cover`, `hide_website_junk`), gives what a document leaves out its
  default, and both the pages and the blog mode pass the result to templates. Jekyll's
  `defaults` for posts (`layout: post`) stay mode code (Q8). The allowlist warning of §5.2 is
  not built (decision 5).
- **Authors are site identity** (`[authors.<key>]`: name, display name, avatar, web,
  description), shown to templates as `site.authors`.
- **The TOC is data** (decision 2): `page.toc` is none, or `{placement, title, entries}`, where
  `entries` is a tree that follows jekyll-toc's nesting rules, counted in heading levels, and
  `includes/sections/toc.html` writes it as nested lists with the classes its caller sets. No
  document of the site skips a level.
- **The description is Rust's**: `markdown::description` applies Liquid's `strip_html |
  strip_newlines | truncate: 160` to the excerpt's HTML. Its own code; the parity tool's Liquid
  filters stay separate, as an oracle.
- **Rust resolves, templates map** (decision 3, revised): the TOC's placement (`left`, `right`
  or `top`) is resolved once, in `content::table_of_contents`; the templates choose the content
  row, the side column and the article's column class (`col-lg-10` beside a TOC, else
  `col-lg-8 offset-lg-2`) from it, and no longer combine `enabled` and `position`. The
  `hide_website_junk` switches stay template `if`s over values Rust gives, each `true` or
  `false`.
- **What the post template cannot show is refused**, as step 1 refused what `/about` did not
  need: a front matter key outside the post's set, another layout, a cover not behind the post,
  a TOC not on the left, and syndications to Hacker News, Mastodon or Bluesky (which the site
  turns into an interactions crawler) are "not built yet". The templates port only the branches
  the post takes, and say which they leave out.
- **Syndications keep their order**: `serde_json`'s map sorts keys, so the mode reads the
  mapping's entries in order (`FrontMatter::entries`) and gives `[{name, url}]`.
- **Shared parts:** `page.html` and `post.html` share `includes/sections/post.html`, so the pages
  mode now gives the keys a page lacks as empty (`date`, `author`, … none; `categories`, `tags`,
  `syndications` empty) and the resolved presentation values. `/about`'s output changed only in
  whitespace.
- **What `folia build` writes:** `/about`, its stub, every static file, and the post
  (`build::SLICE_POSTS`). The blog mode reads every published post for the previous and next
  links, and builds only the slice's.
- **The boundary test** is `just slice-boundary 2` (`scripts/slice-boundary.sh`): it fails when
  the changes since `main`, untracked files included, touch `crates/garden` (step 2) or
  `crates/blog` (step 3).

*Built:*

- `folia_core`: `markdown::{analyze, Heading, toc, TocEntry, description}`;
  `content::{table_of_contents, Toc, TocPlacement, cover, Cover, hide_website_junk}`
  (module `presentation`); `content::{modified, Modified, ModifiedFrom}`; `FrontMatter::{keys,
  json, entries}`; `EntrySummary::{published, modified}`.
- `folia_blog::build` (module `post`): `post.html`'s context (documented in the module), the
  date from the file name, `modified` by §7.3 (front matter, then `git log -1`, then the date),
  the body from `markdown::to_html`, previous and next by date then path, and a title from the
  file name when there is none, as Jekyll makes it.
- `folia_pages::build` gives pages the resolved presentation values, and entries their
  `modified`.
- `folia::config`: `authors` (a `[presentation]` table is rejected); `folia::build`:
  `SLICE_POSTS`, `site.authors`, and `Built::entries`.
- Templates in `filopoe/_folia/templates`: `post.html`, `includes/sections/toc.html`,
  `author_photo.html`, `post_categories.html` and `post_tags.html` are new; `default.html`
  (side TOC by its placement, `hide_website_junk`), `base_head.html` (description, `article:`
  tags, `og:image`; always dark), `site_title.html` and `footer.html` (cover behind the page),
  `sections/post.html` (author box, column class by the TOC's placement, categories, tags, the
  previous and next row, cover script), `post_date.html` (the date, with the site's leading
  space in `datetime`) and `post_syndications.html` gain the post's branches.
- `parity pages` checks, besides step 1's: each entry's dates against the dump (value and
  origin), the table of contents (`div.side-summary`) after normalization, and that the page's
  code blocks are its body's (language, text, token classes), showing their agreement with
  Rouge. `just pages-negative` changes the post's date, a TOC link and the dumped origin too.

*Results* (baseline `BASELINE-B`, evidence in `parity/evidence/slice-step2/`):

- `just pages BASELINE-B`: both pages' fingerprints and `div.main-content` are equal; the post's
  TOC is equal; its 3 highlighted blocks are its body's, and agree with Rouge on 633 of 641
  non-whitespace characters (98.8%, within Q12's accepted differences); both documents' dates
  equal the dump (the post: `2021-08-08T00:00:00+02:00`, modified `2021-08-24` from the front
  matter; `/about`: from git); the stub and 775 static files are byte-identical.
- `just pages-negative BASELINE-B` fails, as it should, on each change, the post's included.
- `just routes BASELINE-B` and `just markdown BASELINE-B` pass, with reports identical to before the
  step; `just liquid` passes; `just check` passes (172 tests; 176 after decisions 1 and 3, see
  *Decisions*; 180 with the safeguards).
- Against a build of `main` with step 1's templates, Folia's output differs only by the new post
  and whitespace in `/about.html`. Folia builds no garden output, before or after.
- `just slice-boundary 2` passes: nothing under `crates/garden` changed.

*Review of step 2's choices* (2026-10-28). A technical review of the code, the templates and the
site's content. "Recommends" is the review's advice; the author's decisions follow it
(*Decisions*), and where they differ, they win.

| # | Choice | Recommends | Work before step 2 is accepted | Before the migration |
|-|-|-|-|-|
| 1 | Configuration boundaries | **Revise**: authors stay in `folia.toml`; `[presentation]` goes (table below) | If accepted: drop `[presentation]`, give the semantic defaults in Rust, the display text in templates | Every presentation key of the content classified (rows 4 and 5) |
| 2 | TOC as Rust data | **Accept** as built | None | Each widened mode keeps `parity pages`' TOC check |
| 3 | Layout conditionals | **Revise**: Rust resolves the TOC's placement; templates map it to classes; `hide_website_junk` stays a template switch | If accepted: one resolved `toc` placement in the context, used by the three templates and the mode's checks | §3's rule restated as below |
| 4 | Unsupported front matter | **Revise** the policy into four classes (below); the slice's strict refusal is safe meanwhile | None: refusing more than needed changes no output | The pages mode gets the same policy before it builds more pages |
| 5 | Unknown presentation keys | **Defer**, as a migration requirement | None | A site-wide key inventory that fails on any key no mode classifies |

**1. Configuration boundaries.** Reproducing `_config.yml`'s `defaults` in `folia.toml` puts
three copies of the same knowledge in three places: the defaults in the configuration, the keys
the blog accepts (`PRESENTATION_KEYS`) and its fallbacks (TOC levels 1 to 6, which never apply),
and the templates' branches. Most defaults are also constants in practice. What the content does
with each, counted in the front matter of every Markdown and HTML file of `filopoe` at `SITE-COMMIT-B`
outside `_site`, `_layouts`, `_includes` and `_folia`:

| Default | Set by | Recommended home |
|-|-|-|
| `use_dark_layout: true` | 4, all `true` | Template constant (the site is dark); front matter `true` a no-op, `false` refused |
| `section_class: ""` | none | Dropped; refused if set |
| `show_author: true` | 3 pages, all `true` | Rust: a post shows its author box; front matter `true` a no-op |
| `syndications: []` | 35 | Content metadata; absent is none, in Rust |
| `share_buttons: false` | none, and no template reads it | Dropped |
| `comments: false` | 7, all `false` | Dropped; `false` a no-op, `true` refused (the site has no comments) |
| `pagination: true` (previous and next) | none: `blog/index.html`'s `pagination` is the paginator's | Rust: posts have neighbours, pages do not |
| `cover: {position: after_title, show: true}` | 86 | Content metadata, with its defaults in Rust (shared by the modes, so `folia_core`) |
| `toc: {enabled: false, position: top, min_level: 2, max_level: 4}` | 12; levels never set | Content metadata, defaults in Rust |
| `toc.title: Content` | never set | Template text |
| `hide_website_junk: {…: false}` | 10 | Content metadata; absent is `false`, in Rust |
| `hide_post_junk: {…: false}` | none | Dropped |
| `corner_backlink: {show: false, url: /, icon_class: …}` | 2 `me/` pages | Pages mode, when it builds them; the icon class is template markup |

So: **site configuration** holds identity (`[site]`, `[authors]`); **content** holds per-document
choices (`cover`, `toc`, `syndications`, `hide_website_junk`); **Rust** holds what those mean
when absent and resolves them (§3); **templates** hold markup and display text. Authors are site
identity and stay in configuration, with only the fields pages show.

**2. TOC.** Heading analysis belongs in Rust: it reuses the body's own parse, so IDs and inner
HTML cannot drift from the body. The tree (`id`, `html`, `children`) names no element or
class, so templates can write it as any list. All 11 documents of the site with a TOC (66
entries) give Jekyll's TOC exactly, in an ad hoc check outside the repository
(`parity/evidence/slice-step2/README.md`); unit tests cover the nesting rules, including the
skipped and risen levels the site does not use. Not reproduced, and used by no document: headings
written as raw HTML, which jekyll-toc counted.

**3. Layout conditionals.** §3 asks for layout classes in Rust. The review restates it: **Rust
resolves which presentation applies; templates map it to markup.** The article's column class
follows from where the TOC goes (beside the article, above it, or none), a semantic choice that
today the templates rebuild from `toc.enabled` and `toc.position` in three places, and the blog
mode again in its checks. Recommended: the mode gives one resolved placement, and the templates
choose the classes from it. `hide_website_junk` is a genuine per-page choice of which parts of
the frame to show (10 documents: listing pages, three posts, two `me/` pages), named after the
Mediumish theme; whether to include a part is a rendering decision, so its `if`s stay in the
templates, with Rust giving every sub-key a value (row 5). Renaming it is [LATER].

**4. Unsupported front matter.** The slice refuses every key outside a fixed list. Precise
policy, recommended:

| Class | Example | Outcome |
|-|-|-|
| Changes the output, and Folia cannot reproduce it yet | `comments: true`, a cover after the title, a TOC not on the left, `rating` | Error: file, key and feature |
| Known, with no effect on the outputs built so far | `lang` (feeds only), `use_dark_layout: true`, `comments: false` | Accepted; each such key is listed, with the output that will read it |
| Conditional on a feature or kind of page | `redirect_from`, `permalink`, `hidden` (already handled by the blog's routes), `pagination` (only the paginator's page) | Accepted where the mode handles it, an error elsewhere |
| Unknown | a typo such as `narrow-post` | Error during the slice; at the switch, see 5 |

At the review, the blog over-refused: it reads `permalink`, `slug`, `redirect_from`, `hidden`,
`category` and `tag` for its routes but refuses them in `build`; it still does. The pages mode
then refused nothing but a `last_modified_at`; it now has its own accepted keys and refuses front
matter its template cannot show (*Safeguards before step 3*, below). Neither changes the slice's
output. A problem names the file and the first key; reporting every key of a file is a small
later improvement.

**5. Unknown presentation keys.** §5.2's warning exists to catch a typo that would otherwise
drop a presentation choice silently. Under 4 it is the "unknown" class. A warning alone can be
missed in a build log, so the migration requirement is stronger: before the switch, a parity
check lists every front matter key of the site by mode and fails on one no mode classifies, and
the build then treats an unknown key as an error or a warning (the author's choice). Tests: a
typo is reported with its file; a classified no-op key is not; a key with an effect the mode
lacks is an error.

*Decisions* (the author, 2026-10-28). The status of each decision is apart from that of its
implementation and testing; step 2 as a whole awaits the author's acceptance.

| # | Choice | Decision | Implementation | Tested by |
|-|-|-|-|-|
| 1 | Configuration boundaries | **[DECIDED]** revised: configuration for identity and settings (authors); content for per-document choices; Rust for defaults and resolution; templates for markup and display text | Done: `[presentation]` removed; defaults in `content::presentation`; dead knobs dropped | `presentation` unit tests; a config with `[presentation]` is rejected; `just pages` |
| 2 | TOC as Rust data | **[DECIDED]** accepted as built | Unchanged, but for living in `page.toc.entries` | `analyze` unit tests; `just pages` TOC check; the ad hoc check of all 11 TOCs |
| 3 | Layout conditionals | **[DECIDED]** revised: Rust resolves which presentation applies, templates map it to markup and classes; `hide_website_junk` stays a template switch | Done: one `TocPlacement`, read by `default.html` and `sections/post.html` | `presentation` and blog unit tests; `just pages` |
| 4 | Unsupported front matter | **[LATER]** the slice keeps refusing conservatively | The blog's refusal as built; the pages mode's added as a safeguard (below) | The blog's and the pages mode's refusal tests |
| 5 | Unknown presentation keys | **[LATER]** | None | None yet |

Where each default of the first build now is (decision 1):

| Default | Now |
|-|-|
| `toc` (off; `top`; levels 2 to 4) | Rust (`content::table_of_contents`); the title's default, "Content", is template text |
| `cover` (after the title, shown) | Rust (`content::cover`) |
| `hide_website_junk` (nothing hidden) | Rust (`content::hide_website_junk`); the switches are the templates' |
| `syndications` (none) | Content; none when absent (the blog mode) |
| `use_dark_layout: true` | Template constant; the key is not read |
| `show_author: true`, `pagination: true` | Rust: every post shows its author box, every article the previous and next row; the keys are not read |
| `section_class`, `share_buttons`, `comments`, `hide_post_junk` | Dropped: no page sets them otherwise, and no template reads them |
| `corner_backlink` | Dropped for now; the pages mode gives it its defaults when it builds the two `me/` pages that set it |
| authors | Configuration (`[authors.<key>]`) |

A key that is no longer read is still refused if a built post or page sets it (decision 4
keeps the slice's refusal), so none of them is ignored silently: of the slice's documents, only
`/about` sets one (`comments: false`, a no-op the pages mode accepts as such).

*Safeguards before step 3* (the author, 2026-10-28; built and tested, committed with step 2). The
final review of step 2 found the pages mode would ignore, on pages outside the slice, front
matter that changes the output: Jekyll's featured image and cover credits after the title
(`privacy`, `security-policy`, `tos`, `security-hall-of-fame`), and the title icon and corner
backlink of the two `me/` pages, shown by rendering them against the baseline. It also found a
table of contents on the right or at the top would be dropped. No page of the site places one so,
and the four pages with a TOC on the left give Jekyll's TOC exactly. The author accepted step 2
as ready subject to two safeguards, which are now built, without porting any feature:

- **The pages mode refuses what its template cannot show** (`folia_pages`, the module's table),
  following the blog's approach but with the pages' own keys:
  - built: `layout`, `title`, `permalink`, `redirect_from`, `published`, `hide_website_junk`; a
    `cover` without a picture or shown behind the page; a `toc` disabled or on the left;
  - built as no-ops: `comments` and `narrow_post` set to `false`, the site's default;
  - not built yet: `title_icon`, `corner_backlink`, `hide_title`, `last_modified_at`, `comments`
    or `narrow_post` set to `true`, a cover after the title, a TOC on the right or at the top;
  - invalid: a switch that is not `true` or `false`, a cover position or TOC placement Jekyll's
    templates do not know;
  - unknown, possibly a typo: any other key.

  Each refusal names the file and the key or feature. Of the site's 18 Markdown pages, 11 build
  (their text equals Jekyll's in the review's render) and 7 are refused: the six above, and
  `changelog`, which has no layout. `security-hall-of-fame`'s `narrow-post: true` is refused as
  unknown; Jekyll ignores it too, so whether it was meant as `narrow_post` is the author's to say.
- **A regression test for a page's table of contents** (`folia_pages` tests): a page shaped like
  `pages/disclosure.md` with a TOC on the left resolves to `left`, with its entries' IDs,
  labels, order and nesting (levels outside 2 to 4 left out, `&` escaped); a page with the TOC
  disabled or absent, and `/about`, have none.

Not done, as the author asked: the cover after the title, title icons, corner backlinks and other
TOC placements are not ported, and no page joins the slice.

*Migration requirements* that decisions 4 and 5 leave open:

- **Unsupported front matter (4):** behavior that changes the output must never be dropped
  silently; each known key with no effect yet has an explicit policy; feature- or
  page-conditional keys are judged by the mode that handles them; unknown keys are reported,
  never ignored. Known limitations now: the blog refuses `permalink`, `slug`, `redirect_from`,
  `hidden`, `category` and `tag`, which it reads for its routes; each mode's accepted keys are
  its own lists, not a site-wide classification; sub-keys are not checked (a post's
  `cover.box_position`, read only by listings, is ignored); and a problem names only the first
  key of its file.
- **Unknown presentation keys (5):** before the switch, a check classifies every front matter key
  of the site by mode and fails on any it cannot classify. A warning in the build log alone is
  not enough.
- **`narrow-post` in `pages/thanks.md`:** the author's to decide; until then the
  source stays as it is. Jekyll reads only `narrow_post`, so it ignores this key, and the
  baseline's `security-hof.html` has no `narrow-post` class. Deleting the key keeps the live
  page as it is. Correcting it to `narrow_post: true` would add that class to
  `div.page-content`, whose styles pad it and limit its article to 800px (`main.scss`,
  `screen.css`): an intended change to the production output, which needs the author's explicit
  authorization, Folia support for `narrow_post: true`, and updated parity evidence.

*Found besides the five*, none affecting the slice's output:

- **Titles with markup:** `2021-08-21-6-steps-personal-static-website` has `<br />` in its title,
  and `2014-05-12-bufala-servita-trento` a double space, which Jekyll's `post.html` turns into
  `<br />`. MiniJinja escapes titles, so both would differ, in their own page and in their
  neighbours' links. A title policy is needed before those posts are built.
- **The shared article template:** pages carry the post's keys, empty, because `page.html` and
  `post.html` share `includes/sections/post.html`. This is an interface between modes, documented
  in each mode's table; keep it explicit (strict undefined values catch a missing key).
- **Truthiness:** Rust always gives `cover.path`, null when the front matter has none, and the
  templates test its truthiness, as Liquid did; so an absent or null path shows no cover in
  either. An empty string still differs: Liquid holds it true, MiniJinja false. No document of
  the site writes one, and no test covers it.

**Step 3: garden notes.** **Status: built and checked locally (2026-11-01); reviewed the same
day (accept with limitations), the review's fixes made, uncommitted, awaiting the author's
acceptance.** No two notes of the site link to each other today: `_digital_garden/topic/index.md`
links `coding/rust.md`, whose only wiki-link, `[[First setup: rust]]`, is broken. These two
exercise a link, a backlink and a broken link; a unit-test fixture of two notes covers the mutual
case.

1. `folia_garden::build`: links resolved by the existing `WikiLinks`; backlinks (title, URL and
   the raw first paragraph, §2.4) for every note; the graph JSON (§2.4) for every note, since it
   needs them all and they are cheap.
2. A broken link is a warning diagnostic naming the note and the target, and renders as
   `<span class="broken-link">`, as today.
3. `modified` for these two notes, as in step 2; port `garden_post.html` and the graph include.

Acceptance: both notes' fingerprints and main content equal Jekyll's, backlinks included; the
graph JSON's nodes and edges equal the baseline's `assets/json/notes_graph.json` as multisets
(each link occurrence an edge); the broken link gives exactly one warning; no change under
`crates/blog`.

*Choices the plan did not cover*, made while building and revised after the review:

- **Every note counts, but only due notes get pages.** `rust.md`'s backlink comes from
  `coding/index.md`, whose own comes from `/dg/index`, outside the slice. Two sets of notes are
  kept apart. The *notes of the garden* are every note but those with `published: false`, as
  Jekyll's collection kept them for the plugin, including notes dated after the build: wiki-links
  resolve to them, backlinks come from them, and they are the graph's nodes. The notes *due* at
  the build are those whose date, from the front matter or else the file name, is not after it:
  only they are routed and rendered, as Jekyll wrote only them, and `build` renders those of them
  in `SLICE_NOTES`. Broken-link warnings are given only for the notes built, whose bodies are
  rendered. The first build dropped notes with a future front matter date from links and the
  graph, and published notes dated in the future by their file name only; the review found
  both, and neither happens in the site today.
- **Note order.** The plugin walks the collection in Jekyll's document order: by date, then by
  path, compared byte by byte, with a note's date from its front matter, else from a dated file
  name. Folia orders dated notes so; it decides which note wins a shared key (the later) and the
  order of backlinks and of the graph, which comes out byte-identical to Jekyll's. A note with
  neither date is ordered as if dated at the build, after every other but one dated later: this
  is Folia's choice, not Jekyll's. Jekyll sorted the collection before it dated such a note, so
  it compared the note with the others by path only, a comparison that is not consistent with
  the dates, and its place depended on the order Jekyll read the files. Folia does not reproduce
  that; no note of the site is undated.
- **Every link occurrence counts.** A note linking twice to another is two backlinks and two
  edges, as in the plugin; no note of the site does so today. Labels longer than 25 characters
  are cut to 22 and `...`, counted in characters.
- **The description keeps wiki-links as written**, as Jekyll took excerpts before the plugin ran
  (`coding/index.md`'s ends `[[Version Con...`). D-003 is narrowed accordingly (§10, Q11): it
  covers rendered excerpts for listings, cards and feeds, which no step builds yet.
- **The site's defaults for notes**, from `_config.yml`, are Rust's: the layout is
  `garden_post`, and the category menu is hidden (`hide_website_junk.cat_jumbotron`), set rather
  than merged, which holds while notes refuse their own `hide_website_junk`. The default
  `author: gg` is not shown by the layout, so it is not read. Templates tell a note by
  `page.layout == "garden_post"`, for `dg_styles.css` in the head and the garden's tagline,
  where Jekyll tested `page.collection`.
- **Refusals.** As the blog and pages modes do, a built note refuses what its template cannot
  show: any front matter key but `title`, `date` and `hide_garden_map`; a `hide_garden_map` that
  is not `true` or `false`; and a note without a date (Jekyll would date it at the build). A
  note of the garden without a title is refused even when its page is not built, since links
  name notes by title; all 54 have one. Jekyll would title it from its file name.
- **Generated files.** A mode returns a generated file as an `Artifact`: its route and its text,
  serialized by the mode, which decides, for example, the graph's key order. The builder writes
  it as it is, and refuses two outputs at one path instead of keeping the later.
- **Graph parity.** A graph one side wrote and the other did not fails `just pages`; when
  neither wrote one, there is nothing to compare. The graph's route is checked with the pages'
  and the stubs'.
- **The backlink excerpt** is carried in the context, as §2.4 has it, but no template reads it.
- **Kept from the layout:** its stray second `</time>`, which parsing drops, and the "Last
  updated" text, which Jekyll's `page.type != 'pages'` always showed. The template's
  `{% if page.modified %}` never hides it, since a built note has a date; it guards the `date`
  filter, and the template says so.

*Decisions* (the author, 2026-11-01, in the review's remediation brief). As for step 2, the
status of each decision is apart from that of its implementation and testing; step 3 as a whole
awaits the author's acceptance.

| # | Choice | Decision | Implementation | Tested by |
|-|-|-|-|-|
| 1 | Notes of the garden and due notes | **[DECIDED]** revised: every note but unpublished ones counts for links, backlinks and the graph; only notes due at the build are routed and rendered | `Note::is_due`, from the one date `read_note` resolves | garden unit tests: future front matter and file-name dates, links to and from such a note, pages as the build time advances; the builder test; `just pages` |
| 2 | Note order | **[DECIDED]** Jekyll's for dated notes; an undated note last, Folia's choice, not Jekyll's | `read_notes` | garden unit tests (file-name date, undated note, shared key); `just pages` (graph byte-identical) |
| 3 | Every link occurrence counts | **[DECIDED]** as in the plugin | Backlinks and edges per occurrence | the mutual-links fixture; graph multisets in `just pages`; the negative run's edge change |
| 4 | Descriptions keep wiki-links | **[DECIDED]** for parity; D-003 narrowed to listings and feeds | `description(&excerpt_html(body, None))` | garden unit test; `just pages` fingerprints |
| 5 | Graph parity | **[DECIDED]** a graph only one side wrote fails | `check_graph_file` | parity unit tests; `just pages` |
| 6 | Site defaults and layout detection | **[OPEN]** as built | `frame()`; templates test `page.layout` | garden unit test; `just pages` |
| 7 | Refusals | **[OPEN]** the slice keeps refusing conservatively, as step 2's decision 4 | The refusals above | garden refusal tests |
| 8 | Generated files | **[OPEN]** as built | `publish::Artifact`; the builder refuses an output path built twice | the builder tests |

*Built:*

- `folia_core`: `markdown::wiki_link_targets`, with unit tests (fenced blocks, code spans,
  Liquid tags and outputs, repeats); `publish::Artifact`, `ModeOutput::{artifacts, warnings}`;
  `SiteClock::now`.
- `folia_garden::build` (module `note`): `garden_post.html`'s context (documented in the
  module), backlinks, the note graph, broken-link warnings and the refusals; `routes` and `build`
  share how notes are read, and which are due.
- `folia::build`: `SLICE_NOTES`; writes the modes' artifacts; refuses an output path built twice;
  `Built::{artifacts, warnings}`, which `folia build` prints as warnings. A builder test builds
  two notes and one dated after the build in a fixture site: the graph file, both pages, no page
  for the third, and the broken link's warning in `Built`.
- Templates in `filopoe/_folia/templates`: `garden_post.html` and
  `includes/digital-garden/notes_graph.html` are new; `base_head.html` (`dg_styles.css`) and
  `site_title.html` (the garden's tagline) gain the note's branches, the tagline as one
  expression, so that every other page keeps step 2's bytes.
- `parity pages` (module `garden`) checks the note graph's nodes and edges as multisets, and
  shows whether the file is byte-identical, failing when only one side wrote it; checks the
  graph's route; and that each built page has one warning per `span.broken-link` on Jekyll's,
  with no warning about a document not built. `just pages-negative` changes a backlink, a broken
  link, a graph label and a graph edge too.

*Results* (baseline `BASELINE-B`; first build in `parity/evidence/slice-step3/`, after the review's
fixes in `parity/evidence/slice-step3-remediation/`):

- `just pages BASELINE-B`: both notes' fingerprints and `div.main-content` are equal, backlinks
  included; their dates equal the dump (both modified from git); `rust.md`'s highlighted block
  agrees with Rouge on all 104 characters; the graph has Jekyll's 54 nodes and 73 edges and is
  byte-identical; the one broken link gives one warning.
- `just pages-negative BASELINE-B` fails, as it should, on each change.
- `just routes BASELINE-B` and `just markdown BASELINE-B` pass, with reports identical to step 2's;
  `just liquid` passes; `just check` passes (204 tests; 193 before the fixes).
- Two builds into fresh directories are identical. Against a build by `main` (`5e9c57a9`) with
  step 2's templates (`filopoe`'s `folia/templates`, `TEMPLATES-COMMIT-A`), Folia's output differs
  only by the two notes and the graph: `/about`, the post, the stub and the 775 static files are
  byte-identical (the evidence's README has the commands).
- `just slice-boundary 3` passes: nothing under `crates/blog` changed.

*Deferred* from the review: warnings are still lost when a build fails; front matter dates with
seconds are read in the site's time zone, where Jekyll read them as UTC; key normalization
differs from Ruby's for some Unicode; Jekyll would run Liquid over a graph holding `{{` or `{%`;
`routes` refuses a file name with an impossible date, which Jekyll rolled over.

### 12.3 Before the slice starts

- **Q5** is decided (§14): templates in `filopoe/_folia/templates`. Committing them there is a
  separate step, alone, as was the spike 3 rewrite; so is publishing `filopoe`'s two local
  commits (`SITE-COMMIT-A`, `SITE-COMMIT-B`).
- **Q8, configuration:** the slice extends `parity/filopoe.toml` (`templates-dir`, `[site]`) and
  keeps it in Folia; its home is `filopoe/_folia/folia.toml` (Q5), where `folia build` looks by
  default, once the site adopts Folia.
- **Steps 2 and 3 start from step 1:** `LastCommits` gives their `modified` (front matter first
  for documents, §7.3); `includes/sections/post.html` gains the branches a post takes (date,
  categories, tags, syndications, previous and next) and `base_head.html` the excerpt
  description; each step builds its documents next to `build::SLICE_PAGES` in the pipeline. Step
  1 added `category_counts` to `crates/blog`, so neither later step needs to touch the other's
  crate.
- Not needed by the slice: Q3, Q4, Q6, Q7, feeds, search, the sitemap, pagination and Sass.
- Done when the three steps pass their checks and are reviewed; the next plan then picks between
  widening a mode to all its documents and later risks 5 and 6.

## 13. Non-goals

Unchanged from v0. In addition:

- No navigation model.
- No `Mode` trait or plugin registry.
- No RSS 2.0, because the current feeds are Atom.
- No template processing of Markdown bodies.

## 14. Open decisions (prioritized)

1. **Remove Liquid from Markdown** via a one-off codemod in `filopoe`,
   verified with Jekyll (§7.2, spike 3)? **[DECIDED]** Yes: D-001
   accepted and the change committed (`SITE-COMMIT-A`) on 2026-10-25.
2. **IAL policy:** support the used subset in Folia Markdown, or codemod
   it to HTML (§5.3)? **[DECIDED]** Support the subset the site uses,
   approved on 2026-10-25; the 38 files keep their IALs.
3. **Defects:** preserve or fix micro search inclusion, the category
   count, the micro `datetime`, and the dead external-link plugin
   (§2.5)? *Recommended: fix search inclusion and `datetime` as recorded
   deviations; preserve the rest initially.*
4. **Listing ownership:** move `/micro`, `/bookmarks`, `/likes`,
   `/blog/`, the tag and category index pages into their modes (§6.3)?
   *Recommended: yes.*
5. **Where templates, content, and assets live:** stay in `filopoe`
   (Folia runs against it), or move into a site directory alongside
   Folia? **[DECIDED]** Stay in `filopoe` (2026-10-25): the site's templates, content and
   assets live together there, with Folia's configuration and templates in `_folia/`
   (`_folia/templates`, later `_folia/folia.toml`), apart from authored content and ignored by
   Jekyll. Rust code and build tooling stay in the Folia repository.
6. **Hosting:** stay on GitHub Pages? The extensionless URLs and
   redirect stubs depend on the host's behavior, which was verified
   live on 2026-10-25 (§2.1). The question only matters if the host
   changes.
7. **External tools:** is a `dart-sass` binary acceptable if `grass`
   falls short?
8. **Configuration:** a new minimal `folia.toml` instead of reading
   `_config.yml`? *Recommended: yes. Jekyll `defaults` become mode code.*
9. **Spike tooling:** is a stdlib-only Python codemod acceptable for the
   one-off spike 3? Is it acceptable for the spike 0 harness to live in
   the Folia repo as a Jekyll plugin loaded via `--plugins`, so that
   `filopoe` stays untouched? *Recommended: yes to both.*

10. **D-002** (§10, spike 1): accept CommonMark's emphasis for the one
    `eggs**!**`, or rewrite that source as `eggs<strong>!</strong>`?
    **[DECIDED]** Rewrite the source (2026-10-25). Jekyll's output is
    unchanged and the deviation is gone, so D-002 is withdrawn. The fix
    is committed in `filopoe` as `SITE-COMMIT-B`, and baselined as `BASELINE-B`.
11. **D-003** (§10, spike 1): resolve wiki-links in garden excerpts,
    fixing the raw `[[…]]` (and tables) in page descriptions, cards and
    feed summaries? **[DECIDED]** Yes, accepted on 2026-10-25, for the 19
    notes with wiki-links, with a per-note check. Narrowed on 2026-11-01
    (step 3): resolved excerpts are for listings, cards and feeds, still
    to be built; a note page's description keeps the raw wiki-links, as
    Jekyll's does, for parity.
12. **Highlighter** (§10, spike 1): which highlighter, and is the
    `bincode` 1 advisory exception (RUSTSEC-2025-0141, unmaintained
    only) acceptable? **[DECIDED]** syntect with bat's grammars
    (`two-face`) and the Pygments-class mapping, keeping `_syntax.scss`:
    96.3% class agreement with Rouge (98.0% by rendered style) meets the
    unchanged 90% rule, and the visual review of all 19 documents shows
    only small differences (2026-10-25). The exception is approved,
    scoped to that advisory, with `bincode` banned outside syntect.

13. **Content lint** (§7.2): how is new Liquid kept out of the content
    until the switch? **[DECIDED]** `parity liquid` (2026-10-26):
    Markdown only, code exempt (fenced and indented blocks, code spans),
    run by `filopoe`'s CI with Folia pinned to a commit. Still open:
    publishing Folia's lint and setting the pin, then committing the
    workflow in `filopoe` and seeing it run; whether HTML pages need a
    check too (recommended: later, with the templates' migration); and
    adding `_folia/folia.toml` to the workflow's paths once the
    configuration moves there (Q8).

[LATER]: watch/serve, incremental builds, Lunr replacement, visual
regression tooling, and splitting `folia` into more crates.
