# Folia --- Project Plan (v1)

**Status:** Planning draft, revision 1 (supersedes `plan.v0.md`)\
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
- **Liquid inside content: 15 files.** These are mostly
  `{% assign img = "…" %}` and `{{ img }}` for image galleries,
  `{{ site.baseurl }}` (which is empty), 2 `{% highlight %}` blocks, and
  1 `{% include digital-garden/notes_graph.html %}`.

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

```
folia-core      model, front matter, Markdown, routes, diagnostics,
                ModeOutput types, git dates. No mode logic.
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

## 5. Content model (`folia-core`)

### 5.1 Source document

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

1. **Common semantic metadata** is typed in core (`CommonMeta`). It
   holds title, date, description, lang, `redirect_from`, syndications,
   and author. These are only the fields at least two modes actually use.
2. **Mode semantic metadata** is typed in the mode crate (for example,
   `PostMeta { categories, tags, featured, hidden, rating, trip, … }`,
   or `MicroKind::Bookmark { url, label }`). Each embeds
   `#[serde(flatten)] common: CommonMeta`.
3. **Presentation metadata** (`cover`, `toc`, `narrow_post`,
   `hide_website_junk`, …) is collected by `#[serde(flatten)] extra` and
   passed to templates untyped. A configured allowlist of presentation
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
  comrak is the alternative if GFM fidelity is better. [SPIKE §10.1]
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
- per-entry search and feed data.

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

// in folia-core
pub struct ModeOutput {
    pub pages: Vec<PageSpec>,      // canonical pages + generated pages
    pub artifacts: Vec<Artifact>,  // graph JSON, etc.
    pub feeds: Vec<FeedSpec>,      // mode-defined feeds (membership decided here)
    pub entries: Vec<EntrySummary>,// index of authored docs for listings/search/feeds
    pub diagnostics: Diagnostics,  // warnings (e.g. broken wiki-link)
}
pub struct PageSpec {
    pub route: Route,
    pub template: &'static str,      // e.g. "micro/bookmark.html"
    pub source: Option<DocId>,       // None for generated pages
    pub context: serde_json::Value,  // mode-specific data for the template
}
pub struct EntrySummary {
    pub doc: DocId, pub mode: Mode, pub kind: Option<&'static str>,
    pub route: Route, pub title: String,
    pub published: Option<DateTime<FixedOffset>>, pub updated: Option<DateTime<FixedOffset>>,
    pub excerpt_html: Option<String>, pub search: Option<SearchDoc>,
    pub feed: Option<FeedEntry>, pub redirect_from: Vec<Route>,
}
pub enum Artifact { Json { route: Route, value: serde_json::Value }, Text { route: Route, body: String } }
```

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

A script removes Liquid from Markdown:

- inline the gallery `assign` variables;
- drop `{{ site.baseurl }}`;
- convert `{% highlight %}` to fences;
- replace the graph `include` with a front-matter flag.

Then rebuild **with Jekyll** and check that the output is unchanged.
This proves the rewrite is behavior-neutral before Folia exists.

### 7.3 Feeds [DECIDED]

- Each mode declares its own feeds and their membership as `FeedSpec`s:
  blog, per-tag, micro, per-kind micro, and garden.
- The **combined** feed is declared in `folia.toml`
  (`include = ["blog", "garden", "micro"], limit = 10`). The site
  module assembles it from `EntrySummary.feed`.
- The `feeds` module serializes Atom, either with `atom_syndication` or
  a template. It depends only on core's `FeedEntry`, so it never
  depends on a mode.
- Only Atom is needed; the current feeds are all Atom.

### 7.4 Search [DECIDED]

The site module collects `EntrySummary.search` and emits a
Lunr-compatible `documents` array with the same `{id,url,title,body}`
schema. Whether micro entries are included is the §2.5 defect.
[OPEN: fix or preserve]

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

1. **Baseline:** run `JEKYLL_ENV=production bundle exec jekyll build`
   at a recorded `filopoe` commit, into a directory outside both repos.
   The current `_site` is a dev build and is invalid as a baseline.
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

## 10. Risks and validating spikes

Ordered by cost of being wrong:

1. **Markdown fidelity.** Render all ~200 bodies with pulldown-cmark,
   adding the IAL subset, kramdown heading IDs, and smart punctuation.
   Diff the normalized body HTML against the baseline. Report the
   identical %, then repeat with comrak.
2. **URL rules.** Compute routes for every source (pure functions, no
   rendering) and diff them against the baseline route inventory. This
   catches slug rules such as `:name` slugifying while `:path` does not.
3. **Content de-Liquefaction.** Run the §7.2 codemod and confirm the
   Jekyll output is byte-identical.
4. **Template engine.** Port `default`/`post` (with a computed TOC) to
   MiniJinja and render one post. Run the `parity` fingerprint on it.
5. **Syntax highlighting.** Choose between syntect with a regenerated
   CSS and a highlighter that emits Pygments short classes, so that
   `_syntax.scss` keeps working.
6. **Sass.** Compile `main.scss` and `dg_styles.scss` with `grass`.
   The fallback is shelling out to `dart-sass`.
7. **Git dates.** Compute last-modified from a single `git log` pass,
   including the submodule, and compare with the garden "Last updated"
   values and feed `updated`.

## 11. Build pipeline [DECIDED]

1. Load `folia.toml`.
2. Discover files. Assign content to modes by directory; everything else
   is a static file.
3. Read front matter and body, filename dates, and git dates (one pass).
4. **Modes** (independent; can run in parallel): typed metadata,
   validation, routes, intra-mode resolution (garden wiki-links,
   backlinks, graph), body HTML, derived pages, feed specs, artifacts.
5. **Site assembly:** route table and duplicates, redirects, combined
   feed, `site` index, search documents, sitemap.
6. **Render:** templates for all `PageSpec`s; serialize feeds and
   artifacts.
7. **Assets:** copy static files and compile Sass. This is independent
   of steps 4–6; only the file list is needed for step 8.
8. **Validate** the final HTML links against routes and assets. Errors
   fail the build.
9. **Write** to a temporary directory, then swap it into the output
   directory atomically. Iteration order is deterministic (`BTreeMap`,
   sorted inputs). Feed `<updated>` uses the newest entry date instead of
   the build time. [deviation]

## 12. First vertical slice (revised)

Run spikes §10.1–10.3 first; they decide the content model. Then:

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
   verified with Jekyll (§7.2)? *Recommended: yes.*
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
   redirect stubs depend on the host's behavior.
7. **External tools:** is a `dart-sass` binary acceptable if `grass`
   falls short?
8. **Configuration:** a new minimal `folia.toml` instead of reading
   `_config.yml`? *Recommended: yes. Jekyll `defaults` become mode code.*

[LATER]: watch/serve, incremental builds, Lunr replacement, visual
regression tooling, and splitting `folia` into more crates.
