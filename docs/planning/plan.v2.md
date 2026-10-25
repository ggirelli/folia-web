# Folia --- Project Plan (v2)

**Status:** Planning draft, revision 2 (supersedes `plan.v1.md`)\
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

Changes since v1:

- §5 splits `folia-core` into a content model and publishing contracts,
  as two modules of one crate.
- Templates are runtime files identified by `String` names (§6.4).
- Feed `<updated>` is derived from entry modification times (§7.3).
- New §6.5 sets out stage contracts.
- §10 defines spikes 0–3 precisely, and §12 sequences them.
- §2 adds facts verified while defining the spikes.

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
| garden | `_digital_garden/**.md` (+ git submodule)     |   54 | `/dg/:path`                             | `dg/tracking/chat_control.html` (underscores kept) |
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
  `jekyll-last-modified-at`, which reads git history, including the
  `_digital_garden/submodule` submodule.
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
  probably the whole content; spike 1 verifies this. Templates consume
  excerpts only through `strip_html` with `truncate: 160` or
  `truncatewords: 30`, except `/micro`, which outputs `item.excerpt`
  as-is.
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

## 4. Workspace shape [DECIDED, pending review]

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
    pub front: serde_yaml::Mapping, // raw; each mode deserializes its own type
    pub body: String,            // Markdown source after front matter
    pub filename_date: Option<NaiveDate>,
    pub git_modified: Option<DateTime<FixedOffset>>,
}
```

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

- Core stores the source string. `markdown::analyze(&src)` returns
  `Analysis { headings, links, wiki_links, excerpt_range }`.
  `markdown::to_html(&src, &LinkResolver)` renders it. Both use one
  parser configuration, so they cannot disagree.
- **Why not store an AST:** Both pulldown-cmark events and comrak's
  arena AST borrow from the source, so stored ASTs mean lifetimes or
  arena plumbing throughout the model. Parsing ~200 documents twice
  costs milliseconds.
- Rendering Markdown to HTML happens **inside the mode stage** (§6), once
  that mode's links are resolved. Body HTML is data, and page HTML is
  template output. This matches what Jekyll does (convert, then wrap in
  a layout) and lets listing pages embed excerpts.
- **Parser:** pulldown-cmark is the leading candidate. Its event stream
  is easy to transform (IAL post-pass, wiki-link events, heading IDs).
  It supports heading attributes, smart punctuation, and wiki-links.
  comrak is the alternative if GFM fidelity is better. [SPIKE: §10, spike 1]
- **Dialect ("Folia Markdown")** [OPEN, recommended]: CommonMark + GFM
  tables/strikethrough/task lists, reference links, wiki-links, smart
  punctuation, kramdown-compatible heading IDs, and **the IAL subset
  actually used**:
  - span `{: .class target="…" title="…"}` after links and inline
    elements;
  - block `{: .class }` on its own line.

  The alternative is a codemod that rewrites the 38 files to raw HTML.
  It is cheaper to build but worse to author in.

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
- Compared with v1, `EntrySummary` no longer carries `SearchDoc` or
  `FeedEntry`. Building search documents and Atom entries is builder
  work. The mode only says *whether* an entry is searchable and *which*
  feeds contain it.
- Mode crates use `serde`/`serde_json` only, so they never see MiniJinja.
- `Route` is a core type holding the URL path. It also maps URLs to
  output files using one site-wide policy: extensionless → `x.html`, and
  trailing slash → `x/index.html`. Modes never build file paths
  themselves.

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

### 7.2 Content migration (one-off, done in `filopoe`) [OPEN, recommended]

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
  2. the git date of the last commit touching the source file, read from
     the submodule's own history for submodule files;
  3. `published`.

  This matches `jekyll-last-modified-at` (front matter wins, otherwise
  git); spike 0 should confirm the precedence from the dumped metadata.
  Git dates come from committed history (`HEAD`), never from the working
  tree or mtimes. A dirty tree therefore builds the same as its commit,
  and an untracked new file falls back to `published`.
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

## 8. Rendering [DECIDED: MiniJinja, pending review]

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
     whitespace normalized.
   - **Normalized DOM diff** of the main content region. Parse with
     html5ever, drop insignificant whitespace, sort attributes and
     classes, and ignore configured attributes.
   - **Artifacts:** compare feeds parsed as entries (link, title,
     published, updated, categories, order). Compare graph nodes and
     edges as sets. Compare the Lunr index as a set of `(url, title)`
     plus body similarity.
3. **Markdown golden tests:** snippets with expected HTML taken from the
   baseline (`insta` snapshots).
4. **Deviation ledger:** a `parity/deviations.toml` lists each
   intentional difference with a rationale. The tool suppresses only
   those.
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
    and tags.
- **Command:**
  `scripts/baseline.sh [--allow-dirty] $FILOPOE $BL/<label>`. The
  script:
  - refuses to run on a dirty tree unless `--allow-dirty` is given;
  - records the `filopoe` commit, submodule commits, and Jekyll and Ruby
    versions in `BASELINE.json`;
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

### Spike 3: Liquid removal (run first after spike 0; Jekyll-only, no Rust)

- **Behavior:** every Liquid construct in the 15 content files (§2.2) is
  replaced by its literal production expansion. Jekyll's per-document
  converted HTML stays unchanged, except for the listed deviation.

| Construct | Occurrences | Rewrite | Expected change |
|-|-|-|-|
| `{% assign v = "…" %}` + `{{v}}` | 100 / ~200 | inline the literal | none |
| `{{ site.baseurl }}`, `"…" \| prepend: site.baseurl` | ~36 | literal path (baseurl is empty in production) | none |
| `{{ site.url }}`, `site.paths.home`, `site.webmentions.*`, `{{ page.url }}` | 5 | literal production value | none |
| `{% highlight lang %}…{% endhighlight %}` | 11 in `_posts/2020/2020-01-26-sample-code.md`, `_posts/2021/2021-08-17-sample-speed.md` | fenced code block | **D-001:** wrapper markup changes from `figure.highlight > pre > code[data-lang]` to `div.language-x.highlighter-rouge > div.highlight > pre.highlight > code`; text identical; visual check needed |
| `{% include digital-garden/notes_graph.html %}` | 1 | the include's 22 lines of HTML, verbatim (`link_classes` is empty) | none |

- **Tool:** `parity/codemods/deliquify.py`, a stdlib-only one-off run
  with `python3 -I`.
  - It rewrites only the patterns in the table above. Any other `{%` or
    `{{` makes it fail with the file and line.
  - Production values are an explicit table in the script.
  - It is idempotent. `--check` prints a diff; `--write` applies it.
- **Commands:**

  ```sh
  scripts/baseline.sh $FILOPOE $BL/SITE-MAIN
  python3 -I parity/codemods/deliquify.py --write $FILOPOE
  scripts/baseline.sh --allow-dirty $FILOPOE $BL/deliquified
  scripts/compare-builds.sh $BL/SITE-MAIN $BL/deliquified parity/spike3-expected.txt
  ```

  The "after" build runs on the uncommitted tree on purpose. Git dates
  then stay unchanged, because `jekyll-last-modified-at` reads committed
  history; the spike verifies this. Otherwise the 15 files' dates would
  move and pollute the comparison.
- **Success:**
  - No Liquid remains: `grep -rE '\{%|\{\{'` over content returns
    nothing.
  - Every per-document dump is byte-identical, except the 2 D-001
    files. For those, the code text and all text outside code is equal.
  - Every site file is identical after normalization, except files
    listed in `spike3-expected.txt`, each traced to D-001. Expected
    entries are the two post pages, and possibly the Lunr index if
    whitespace around code differs.
  - Any difference that is not explained fails the spike.
- **Afterwards:** with your approval (§14 Q1), commit the change in
  `filopoe`, add D-001 to the ledger, and re-baseline at the new commit.
  That baseline is the reference for spikes 1 and 2. Add the content
  lint from §7.2.

### Spike 2: route parity

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
| `_digital_garden/submodule/**` | (submodule) | as Jekyll |
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

### Spike 1: Markdown compatibility

- **Behavior:** `folia_core::content::markdown::to_html` turns the
  de-Liquefied sources into the body HTML in the spike 0 `content/`
  dumps, for these features:
  - **1a heading IDs**, including punctuation, non-ASCII, emoji, and
    duplicates;
  - **1b IAL subset**, span and block;
  - **1c reference and shortcut links**;
  - **1d smart typography**;
  - **1e excerpts**;
  - **1f syntax highlighting**;
  - **1g raw and inline HTML**;
  - **1h wiki-links.** The spike's resolver is built from the
    manifest's garden URLs and mirrors the Ruby rules: title first, then
    filename, both case-insensitive.
- **Representative real files:**
  - 1a: `_posts/2021/2021-08-08-sample-post.md`,
    `_posts/2014/2014-04-14-third-sample.md` (`Cos’è l’autismo?`),
    and `_posts/2022/2022-09-25-sample-week.md` (repeated emoji headings).
    `_digital_garden/sayings.md` and `pages/questions.md` cover the rest of
    the 12 non-ASCII-heading files.
  - 1b: `_posts/2021/2021-08-21-sample-steps.md`
    (24 IALs), `_digital_garden/web/index.md`,
    `pages/making-of.md`, `pages/directory.md`,
    `_micro/likes/2021-08-18-sample-like.md` (`target` plus
    `.u-in-reply-to`), and `_digital_garden/home.md` (block IALs
    before lists, and a span IAL after a wiki-link).
  - 1c: `pages/setup.md` (46 definitions),
    `_posts/2025/2025-12-30-sample-list.md`, and
    `_digital_garden/home.md` (shortcut references and code-span
    labels).
  - 1d: `_posts/2021/2021-08-08-sample-post.md` and the 2014 Italian
    posts.
  - 1e: `_posts/2017/2017-01-13-sample-tools.md`,
    `_posts/2017/2017-02-21-sample-colours.md`, and
    `_micro/notes/2026-04-08-sample-note.md` (with `<!--more-->`), plus
    every document without the marker.
  - 1f: `_digital_garden/topic/note.md`,
    `_posts/2022/2022-08-27-sample-setup.md` (14 fences), and the
    two former `{% highlight %}` posts.
  - 1g: `pages/profile.md`, `profile/tools.md` (Mermaid), and
    `_digital_garden/home.md` (inline graph HTML mid-paragraph).
  - 1h: the 19 garden files with wiki-links, including the live broken
    link on `/dg/`.
- **Measurement:** `parity` parses both fragments with html5ever and
  normalizes them:
  - collapses whitespace outside `pre`;
  - decodes entities, so `&rsquo;` and `’` compare equal;
  - sorts attributes and class tokens;
  - compares code blocks by text only (highlighting is gated
    separately, below).

  Each document is reported as *identical*, *equal after
  normalization*, or *differs*, with the first differing node path.
- **Gates:**
  - **G1 heading IDs:** 100% equal per document, in order. This is a
    hard gate: inbound anchors and the TOC depend on the IDs.
  - **G2 links and attributes:** the `href`/`src` multisets are 100%
    equal. IAL-derived `class`, `target`, and `title` sit on the same
    elements, 100%. Hard gate.
  - **G3 text:** normalized text is 100% equal, except ledger entries.
    Hard gate.
  - **G4 structure:** at least 95% of documents are equal after
    normalization, and every remaining difference is categorized as
    "fix" or "ledger". This is a decision gate: it selects the parser
    and the IAL policy. It is not a release criterion.
  - **G5 excerpts:** the values templates actually consume match for
    100% of documents:
    - `strip_html | strip_newlines | truncate: 160`;
    - `strip_html | truncatewords: 30`;
    - the raw excerpt on `/micro`.
  - **G6 highlighting:**
    - Code text is 100% equal.
    - Token-class agreement against Rouge is reported per block.
    - **Decision rule:** if a Pygments-class emitter reaches about 90%
      agreement on the fixture files, keep `_syntax.scss`. Otherwise
      generate CSS for the chosen highlighter and visually check those
      pages.
- **Commands:**
  - Run the comparison:

    ```sh
    cargo run -p parity -- markdown --site $FILOPOE --baseline $BL/<commit> \
      [--parser pulldown|comrak] [--doc PATH] [--report spike1.md]
    ```

  - `cargo run -p parity -- extract-fixtures` copies small real
    snippets and their kramdown output into
    `crates/folia-core/tests/fixtures/markdown/<feature>/`.
  - `cargo test -p folia-core` then runs those fixtures as `insta`
    snapshots without needing `filopoe`.
- **Known exceptions:**
  - D-001 (highlight wrapper);
  - wiki-link HTML is reproduced as the Ruby plugin emits it
    (`a.internal-link`, `span.broken-link`);
  - kramdown quirks found during the spike become ledger entries with a
    reason, or get fixed.
- **Closes:** the parser choice (pulldown-cmark first; comrak only if
  G4 fails for structural reasons), Q2 (the IAL policy), and the
  highlighter choice.

### Later risks (unchanged; after spikes 0–3)

- **4. Template engine.** Port `default`/`post` (with a computed TOC) to
  MiniJinja and render one post. Run the `parity` fingerprint on it.
- **5. Sass.** Compile `main.scss` and `dg_styles.scss` with `grass`.
  The fallback is shelling out to `dart-sass`.
- **6. Git dates.** Compute `modified` (§7.3) from a single `git log`
  pass, including the submodule. Compare it with `meta/*.json`
  `last_modified_at` from spike 0.

Syntax highlighting, v1's separate risk 5, is now gate G6 of spike 1.

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
   and the parser and highlighter choices.

Scaffolding is limited to what the spikes need:

- `folia-core` (`content::{route, markdown}`);
- a route function in each mode crate;
- `folia::discover`;
- a dev-only `parity` crate;
- `parity/` scripts and the Jekyll plugin.

No templates, feeds, search, rendering, or CLI until the spike results
are reviewed.

### 12.2 Vertical slice (after the spikes)

1. One page (`/about`) through the whole pipeline: route, MiniJinja
   layout, assets, a redirect stub, and the `parity` route and
   fingerprint check.
2. One blog post: filename date, permalink, IALs, TOC, and Rouge-class
   highlighting.
3. Two garden notes linking to each other: wiki-links, backlinks, graph
   JSON, and a broken-link diagnostic.

The slice tests the crate boundaries: steps 2 and 3 must not need to
touch each other's crates.

## 13. Non-goals

Unchanged from v0. In addition:

- No navigation model.
- No `Mode` trait or plugin registry.
- No RSS 2.0, because the current feeds are Atom.
- No template processing of Markdown bodies.

## 14. Open decisions (prioritized)

1. **Remove Liquid from Markdown** via a one-off codemod in `filopoe`,
   verified with Jekyll (§7.2, spike 3)? This includes committing the
   change in `filopoe` and accepting D-001 (the `{% highlight %}`
   wrapper markup in 2 posts). *Recommended: yes.*
2. **IAL policy:** support the used subset in Folia Markdown, or codemod
   it to HTML (§5.3)? *Recommended: support the subset; confirm after
   spike 1.*
3. **Defects:** preserve or fix micro search inclusion, the category
   count, the micro `datetime`, and the dead external-link plugin
   (§2.5)? *Recommended: fix search inclusion and `datetime` as recorded
   deviations; preserve the rest initially.*
4. **Listing ownership:** move `/micro`, `/bookmarks`, `/likes`,
   `/blog/`, the tag and category index pages into their modes (§6.3)?
   *Recommended: yes.*
5. **Where templates, content, and assets live:** stay in `filopoe`
   (Folia runs against it), or move into a site directory alongside
   Folia? *Recommended: stay in `filopoe`; Folia stays a tool.*
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

[LATER]: watch/serve, incremental builds, Lunr replacement, visual
regression tooling, and splitting `folia` into more crates.
