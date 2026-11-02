# Folia --- Project Plan (v10)

**Status:** Planning draft, revision 10 (supersedes `plan.v9.md`)\
**Project type:** Personal, in-house static-site publishing engine\
**Implementation language:** Rust\
**Primary reference site:** `ggirelli.info`, Jekyll source in the `filopoe` repository\
**Repository:** `https://github.com/ggirelli/folia.git`, Jujutsu colocated with Git

Markers used in this document:

- **[DECIDED]**: settled; change only with new evidence or the author's explicit revision.
- **[OPEN]**: needs the author's input; options and a recommendation are given.
- **[LATER]**: deliberately deferred; safe to postpone, and listed in §9 so it is not lost.

Status words, used strictly (§2 applies them):

- **validated**: built, and checked against the baseline by a repeatable command whose output is
  kept as evidence;
- **built**: the code exists and runs, with unit tests at most;
- **routed**: only the output's URL and file are planned (route parity);
- **designed**: decided on paper, no code;
- **not started**: neither.

**Nothing from the migration may be published or deployed until the migration is finalized and
explicitly approved.** The Jekyll site stays the production site.

## 0. How to read v10

v10 is the plan for finishing the migration. It restructures v9 around what remains, and it
records three decisions the author settled on 2026-11-01 (Q4, Q14, Q15, §10).

- **Superseded in v9:** §6.3 (listing pages), the parity strategy of §9, the roadmap of §12.3 and
  the decision list of §14. Their replacements are §3, §5, §6 and §10 here.
- **Carried forward by reference**, still current and not restated: v9 §2 (evidence from the
  site), §4 (workspace shape), §5.1–§5.3 (content model, metadata tiers, Markdown), §6.4
  (templates are runtime files), §7.2 (content lint), §8 (MiniJinja), §10 (spikes and the rules
  they settled), §11 (build pipeline), and §12.2 (the records of slice steps 1 to 3, with their
  choices and decisions). Where v10 changes one of them, it says so at the point of change.
- **Evidence** is cited by path under `parity/evidence/` and by source location; the
  migration-wide assessment of 2026-11-01 (read-only, against Folia `0302e08`, `filopoe`
  `BASELINE-B` and baseline `BASELINE-B`) is the source of the counts in §2 unless another source is
  named. Counts derived by simulation rather than by a kept check are marked *simulated*.

Changes since v9:

- Step 3 is committed locally (Folia `0302e08`) and its templates are committed on `filopoe`'s
  `folia/templates` (`TEMPLATES-COMMIT-B`); v9 still called it uncommitted (§1.2 reconciles this
  and other stale statements).
- **Q4 [DECIDED]:** listings are built in Rust; templates render prepared contexts (§3.3).
- **Q14 [DECIDED]:** feeds, the sitemap, `redirects.json` and the search index are produced by
  Rust serializers, not templates (§3.4).
- **Q15 [DECIDED]:** HTML parity is a whole-document normalized DOM comparison with an explicit
  deviations ledger and hard document assertions; fingerprints become diagnostics (§5).
- A phased roadmap to cutover (§6, §7) and measurable final acceptance criteria (§8) replace
  v9 §12.3's "the next plan picks".

## 1. Executive summary and status

Folia plans every output Jekyll writes, exactly, and what it builds matches Jekyll. It builds very
little yet: 6 of the 302 outputs that are not static files. The architecture holds; four shared
contracts should be hardened before any mode is widened, because each costs more with every mode
that depends on it. The parity tooling is strict for content and weak for the page frame and for
generated outputs, so it is hardened in the same first phase.

| Area | Status |
|-|-|
| Routes | validated: 1,077 of 1,077 outputs planned with Jekyll's URL and file |
| Static files | validated: 775 of 775 byte-identical |
| Markdown bodies and excerpts | validated: 190 of 190 bodies, 172 excerpts (19 under D-003) |
| Rendered documents | validated for 4 (`/about`, one post, two notes) of 202 sources |
| Redirect stubs | validated for 1 of 26 |
| Note graph | validated, byte-identical |
| Feeds, sitemap, `redirects.json`, search, Sass, pagination, archives, listings, micro | routed or designed; not started |
| Not built | 296 of 1,077 outputs (`slice-step3-remediation/pages.txt:3`) |

The roadmap (§6, §7) has four phases:

- **A. Contracts and parity** (no output change): shared contracts (A1), parity infrastructure
  (A2), a site-wide front matter inventory (A3).
- **B. Content coverage**, one mode at a time to all its documents: garden (B1), pages (B2),
  blog (B3), micro (B4).
- **C. Generated and aggregate outputs:** listings and archives (C1), feeds (C2), sitemap and
  redirects (C3), search (C4), Sass (C5).
- **D. Integration and cutover:** link validation (D1), migration cleanup (D2), cutover
  preparation (D3).

### 1.1 Repository state (observed 2026-11-01)

| Repository | State |
|-|-|
| Folia | `0302e08` (step 3) on a detached HEAD; `main` = `origin/main` = `5e9c57a` (step 2 and the docs). Moving `main` and pushing is a separate authorization (Q19). |

The state of the site's repositories, workspaces and baselines is recorded privately, with the
site's inputs.

### 1.2 Stale statements in v9, reconciled

| v9 statement | Actual state |
|-|-|
| Step 3 "uncommitted and not reviewed" / "awaiting the author's acceptance" (`plan.v9.md:20-22`, `:1724-1726`) | Reviewed (accept with limitations), remediated, and committed with the author's authorization as `0302e08` (detached, unpushed) |
| Step 1's templates "still uncommitted" (`:27-28`, `:53`, `:1360`) | Committed on `folia/templates` (`TEMPLATES-COMMIT-A`, `TEMPLATES-COMMIT-B`), local; v9's step 2 status line (`:1443-1447`) already says so |
| Content lint "not yet pushed", pin `PENDING` (`:49-50`, `:648-650`, Q13) | The lint is on Folia's `main` (`6ef4a49`); `folia/content-lint` pins it and is pushed with SITE-PR, whose check passed. Only the untracked working copy in the default workspace still says `PENDING`. |
| `SITE-COMMIT-A` and `SITE-COMMIT-B` exist only in the local checkout (`docs/baseline.md:276-277`) | Both are on `origin/folia/content-lint`, not on `main` |
| Parity "deliberately does not depend on Folia" (`:326-327`) | `crates/parity` depends on `folia`, `folia-core` and `folia-garden` (`crates/parity/Cargo.toml:12-14`), as `:339-342` and `docs/development.md` describe. v10 states the rule as: parity uses Folia's public API to build and plan, and never its internals to judge. |
| D-003 `change` text: page descriptions show the fix (`parity/deviations.toml:98-106`) | D-003 was narrowed on 2026-11-01 (v9 Q11): note page descriptions keep raw wiki-links. The ledger text is to be updated in A2. |
| §7.3: the feed-level `<updated>` deviation "is recorded in the deviation ledger" (`:731-734`) | No such entry exists; it is added before C2 (§6, C2) |
| `docs/baseline.md` describes step 1 only (`:221-241`) and calls the templates uncommitted (`:296-297`) | To be updated with A2 |
| `just baseline-verify` reproduces a baseline (`docs/baseline.md:286-291`) | It would refuse the current checkout: `scripts/baseline.sh:42-47` counts the untracked `_folia/` and `content.yml` as dirty, unlike the parity tools' exclusions (`crates/parity/src/routes.rs:315`). Fixed in A2. |

## 2. Verified implementation baseline and evidence

### 2.1 What Jekyll writes, and what Folia builds

From `folia-baselines/BASELINE-B/dump/manifest.jsonl` (1,077 rows, all written) and
`parity/evidence/slice-step3-remediation/`.

| Output group | Jekyll | Folia built | Status | Evidence |
|-|-:|-:|-|-|
| Static files | 775 | 775 | validated | `pages.txt` |
| Posts | 79 | 1 | 1 validated | `pages.txt` |
| Garden notes | 54 | 2 | 2 validated | `pages.txt` |
| Micro entries | 39 | 0 | routed | `routes.txt` |
| Page outputs (30 sources; `blog/index.html` written 9 times) | 38 | 1 | 1 validated | `pages.txt` |
| Category archives | 10 | 0 | routed | `routes.txt` |
| jekyll-feed feeds (blog, 44 tags, micro, 4 micro kinds, garden) | 51 | 0 | routed; designed (v9 §7.3) | `routes.txt` |
| Combined `/feed.xml` (counted among the pages) | (1) | 0 | routed as a page | `routes.txt` |
| Redirect stubs | 26 | 1 | 1 validated | `pages.txt` |
| `redirects.json` | 1 | 0 | routed | `routes.txt` |
| `sitemap.xml` | 1 | 0 | routed | `routes.txt` |
| Source maps | 2 | 0 | routed | `routes.txt` |
| Note graph | 1 | 1 | validated, byte-identical | `pages.txt:23-24` |
| **Total** | **1,077** | **781** | 296 not built | `pages.txt:3` |

### 2.2 Validated

| What | Check | Evidence |
|-|-|-|
| Every output's URL and file; source classification; generated outputs; stubs | `just routes` (exact multisets) | `slice-step3-remediation/routes.txt` |
| 190 Markdown bodies (135 identical, 55 equal after normalization), 172 excerpts | `just markdown`, gates G1–G6 | `slice-step3-remediation/markdown.txt` |
| 775 static files, 1 stub | `just pages`, byte comparison | `pages.txt` |
| `/about`, `webmentions`, `coding/index`, `coding/rust`: head values, headings, link/resource multisets, `div.main-content` DOM, body text; TOC; dates (value and origin); code blocks | `just pages` | `pages.txt` |
| Note graph (54 nodes, 73 edges), broken-link warning | `just pages` | `pages.txt:22-24` |
| The checks fail when they should | `just pages-negative`, `just markdown-negative` | `slice-step3-remediation/pages-negative.txt`, `spike1/` |
| Liquid kept out of Markdown prose | `just liquid` (190 documents) | `liquid.txt`; SITE-PR's check |
| Two builds identical | ad hoc `diff -r` | `slice-step3-remediation/README.md` (no recipe yet: A2) |

Highlighting agrees with Rouge on 96.3% of token classes across the site (decision Q12); on built
pages the agreement is reported, not enforced (`pages.txt:10-17`).

### 2.3 Built, not validated

- **Dates:** front matter and file-name dates, the future-date holdback (blog, micro, garden),
  git `modified` dates. Unit tests cover them; parity covers the 4 built entries. No content is
  future-dated or unpublished today.
- **Page frame:** nav, site title, footer and body attributes are compared by text and URL
  multisets only; the robots `NOINDEX, NOFOLLOW` meta (`filopoe/_includes/head.html:24`,
  kept in Folia's `base_head.html`) is compared by nothing (`crates/parity/src/pages.rs:186-197`).
- **Templates:** ported with only the branches the four built pages take; 24 of Jekyll's 52
  includes and 5 of its 12 layouts have a port.

### 2.4 Not started

- **Generators:** no code exists for Atom feeds, the sitemap, `redirects.json`, the search
  index, Sass, pagination, archives or listings; only their routes are planned (`site.rs:97-194`,
  each mode's `routes`). `ModeOutput` has no feeds (`crates/core/src/publish.rs:5-7`).
- **Micro:** no `build` function (`crates/micro/src/lib.rs:37` is its only public function).
- **HTML pages:** the pages mode refuses every non-Markdown source (`pages/src/lib.rs:138`):
  `404.html`, six `blog/*.html`, `profile/index.html`, `pages/rss.xml`, the search JS and two `.scss`.
- **Post-render link validation** (v9 §7.1, [DECIDED]): no code.
- **Content comparators** for feeds, search, sitemap, `redirects.json` and CSS (v9 §9.2): none.

### 2.5 What the content needs that Folia refuses (simulated)

Simulating the current front matter gates over all sources (assessment, not a kept check):

- **Posts:** 1 of 79 passes. Main reasons: a cover after the title (the default; 77 posts, of
  which 62 show an image), `narrow_post` (58), interaction syndications (18), `trip` (10),
  `featured` (6), `use_dark_layout` (4), `footer` (2), `hidden`, `category`, `cover: top` (1 each).
- **Page sources:** 11 of 27 Markdown and HTML sources pass; 16 are refused (9 not Markdown;
  4 cover after title; `author`, `show_author`, `additional_body_class`, `title_icon`,
  `corner_backlink`, `hide_title`, `collections`, `pagination`, `error-type`, the `narrow-post`
  typo); the 3 asset pages are not Markdown either.
- **Garden:** 54 of 54 pass.
- **Micro:** 0 of 39 can build (no builder); keys `bookmark` 11, `like` 8, `rsvp` 7, `event` 7,
  `syndications` 10, `narrow_post` 16, `redirect_from` 1.

Some refusals are of keys that have no effect in Jekyll (`trip` is read nowhere;
`featured: false`, `hidden: false`, `use_dark_layout: true` are defaults): over-refusal, addressed
by A3, not features to port.

## 3. Target architecture and ownership

v9 §3 (principles), §4 (workspace) and §6.1 (what a mode owns) stand. This section adds the
ownership the three new decisions settle, and revises v9 §6.3 and §6.5.

### 3.1 Three layers

| Layer | Owns | Must not |
|-|-|-|
| **Domain mode** (`folia-blog`, `-micro`, `-garden`, `-pages`) | typed metadata, validation, routes, ordering, intra-mode links, body HTML, its derived pages and their *data* (pagination, archives, kind listings), membership of its feeds, per-entry searchability, its artifacts | read another mode, load templates, serialize site-wide formats, write files |
| **Builder** (`folia`: `site`, generator modules, `render`, `output`) | orchestration; the route table and collision checks; cross-mode aggregation (combined feed, search, sitemap, redirects, the frame data every page shows); serialization of structured outputs; rendering; link validation; writing | interpret a mode's front matter |
| **Templates** (`filopoe/_folia/templates`) | HTML markup and display text for prepared contexts | query, filter, sort or assemble site-wide collections; produce non-HTML formats |

### 3.2 What is generated where

| Output | Data from | Assembled by | Serialized by | Validated by (§5) |
|-|-|-|-|-|
| Entry pages (posts, notes, micro, pages) | mode | mode (`PageSpec`) | template | DOM + assertions |
| `/blog/`, `/blog/pageN/`, category archives, `/blog/categories.html`, `/blog/tags.html` | blog | blog | template | DOM + assertions |
| `/micro`, `/bookmarks`, `/likes` | micro | micro | template | DOM + assertions |
| `/` (author page) | config (author data, Q16) | builder or pages (C1) | template | DOM + assertions (`rel=me`, h-card) |
| Per-mode feeds | mode (`FeedSpec`, membership and order) | builder | Rust Atom serializer | feed comparator |
| Combined `/feed.xml` | `EntrySummary` of the included modes | builder | Rust Atom serializer | feed comparator |
| `sitemap.xml` | route table, `modified` | builder | Rust XML serializer | sitemap comparator |
| `redirects.json`, stubs | `redirect_from` | builder | Rust | byte |
| Search index | `EntrySummary.searchable` and text | builder | Rust, Lunr-compatible JS | search comparator |
| Note graph | garden | garden (`Artifact`) | garden | graph comparator (built) |
| CSS and maps | `assets/css/*.scss` | builder (assets) | Sass compiler (Q7) | CSS comparator |

### 3.3 Listing ownership [DECIDED, Q4, 2026-11-01]

**Rust builds listings.** Domain modes own their domain data and its transformations; the
builder orchestrates aggregation across modes and generates outputs; MiniJinja templates receive
prepared, typed presentation contexts and only render them.

- A listing's selection, filtering, ordering, grouping, pagination and the per-item values a
  card shows (title, URL, date, excerpt text, cover, categories) are computed in Rust. The
  template decides markup and display text.
- **`/blog/` belongs to the blog mode,** with pages 2–9. Today `/blog/` is a pages-mode output of
  `blog/index.html` while pages 2–9 are blog outputs, and the blog names that pages document
  (`crates/blog/src/lib.rs:32,209`). v10 ends the split: the blog plans and builds all nine; the
  authored `blog/index.html` stops being a pages-mode output.
- Likewise the micro listings belong to micro, and the category and tag index pages to the blog.
- **The `site` index shrinks to frame data.** v9 §6.3's escape hatch, a read-only `site` index of
  every `EntrySummary` that templates may query, is withdrawn: templates must not assemble
  collections. Every page still receives the prepared frame values the site's chrome shows (site
  identity, authors, the build's year, category counts), computed in Rust, as today
  (`crates/folia/src/build.rs` `site_index`).
- **The authored sources that Jekyll used as listing templates** (`blog/*.html`, and
  `pages/rss.xml` and `assets/js/lunrsearchengine.js` for §3.4) leave the pages mode: their
  routes are claimed by the owning mode or the builder. Whether the files are then deleted from
  `filopoe`, kept as intro-text sources, or ignored by configuration is open (Q20).
- **Unresolved:** where the intro prose of `/micro`, `/bookmarks` and `/likes` lives (template
  text, or an authored file the mode reads); which layer builds `/` (it lists no collection,
  only author data). Both are settled in C1.

### 3.4 Generated data formats [DECIDED, Q14, 2026-11-01]

**Structured outputs are produced by Rust serializers, not MiniJinja templates:** Atom feeds,
the combined feed, `sitemap.xml`, `redirects.json`, and the search index with any Lunr-compatible
wrapper. Templates remain responsible for HTML.

- **Interfaces.** Modes declare *what* (membership, order, searchability) through `ModeOutput`
  and `EntrySummary` (§4); generator modules of `folia` turn that data into bytes; each generator
  is a function from prepared data to a `String` or bytes, with no template engine, so it can be
  unit-tested against fixtures and compared with the baseline by §5's comparators.
- **Consequences.** MiniJinja's `json`, `urlencode` and `loop_controls` features stay off
  unless an HTML template needs them; Liquid filters used only by feeds and search
  (`smartify`, `xml_escape`, `strip_html`, `normalize_whitespace`, `jsonify`) are Rust functions
  in the generator that needs them.
- **Schemas known from evidence:** Atom 1.0 as jekyll-feed 0.17.0 and
  `_layouts/feed.xml` write it (10 entries each); the search JS's
  `var documents = [{id, url, title, body}]` (v9 §7.4); the sitemap's `<url>` with `<loc>` and
  `<lastmod>` (222 URLs); `redirects.json` as an object from source URL to target (26 keys).
- **Schema questions not yet settled** (each settled in its C step, against the baseline's
  files, before code): the exact element set and order of each feed (author, categories, `lang`,
  `<content>` vs `<summary>`; `/feed.xml` has summaries only); whether Folia reproduces Jekyll's
  feed-level quirks (Q3); whether an Atom crate or hand-written serialization; the sitemap's
  `lastmod` for static files, which Jekyll takes from file mtimes; the search body's text
  extraction, which Jekyll derived by running `markdownify` over HTML (§9.1).

### 3.5 Stage contracts (revises v9 §6.5)

| Stage | Input | Responsibility | Output |
|-|-|-|-|
| **Plan** (`folia::site::plan`) | discovery, every mode's `routes` | the route table; duplicate and collision checks | `Site`, with its `SiteClock` (§4, A1.5) |
| **Mode build** | its `SourceDoc`s, the build context (§4) | §3.1's mode duties | `ModeBuild`: output and diagnostics (§4) |
| **Assemble** (`folia::site`) | every `ModeOutput`, the route table | aggregation (§3.2), frame data | page specs and generator inputs |
| **Generate** (generator modules) | generator inputs | serialization (§3.4) | bytes keyed by `Route` |
| **Render** (`folia::render`) | page specs, templates | template check, then rendering | HTML keyed by `Route` |
| **Assets** | static list, Sass sources | copy, compile | bytes keyed by `Route` |
| **Validate and write** (`folia::output`) | everything above | link validation (D1), no output path built twice, every output planned, atomic write | the destination |

## 4. Shared contract hardening (A1)

Each item names the evidence of the problem and what "done" means. Items are chosen because
they get more expensive with every widened mode; the rest of the assessment's small findings
are in §9.2.

| # | Change | Why (evidence) | Done when |
|-|-|-|-|
| A1.1 | **A configured build set** replaces `only: &[&str]` in the mode `build` APIs and the public `SLICE_*` constants. Default: every document of a mode; the set narrows only for widening steps, from configuration or a builder argument, never from mode code. | `crates/folia/src/build.rs:34-43,171-200`; `blog/src/post.rs:72,85`; `garden/src/note.rs:81,109`; parity can only see the slice (`parity/src/pages.rs:26,51`) | No mode signature mentions the slice; the slice's 4 documents build from configuration with byte-identical output |
| A1.2 | **One mode build context** (clock, git commits, mode root, config) for every mode's `build`, and `routes` where they need it | Four different signatures (`blog`: `(docs, only, commits, clock)`, `garden`: `(docs, root, only, commits, clock)`, …); v9 §6.2's `ModeConfig` never built | All four modes take the context; micro gains `build` with it in B4 |
| A1.3 | **`EntrySummary` gains** title, excerpt HTML, categories, tags, kind, searchable (v9 §6.2's fields, as needed by §3.2's consumers) | Titles exist only in untyped context JSON (`post.rs:127-143`, `note.rs:264-279`, `pages/src/lib.rs:160-176`); feeds, search, sitemap and listings need them | The three constructors fill them; unit tests per mode |
| A1.4 | **Diagnostics travel with output**, on success and on failure: a mode returns its output and its diagnostics, each with a severity; the builder keeps warnings when another step fails | Warnings are dropped in three places (`garden/src/note.rs:121-125`; `build.rs:196-216`; `build.rs:121-137` with `main.rs:64-73`); deferred in v9 step 3 | A failing build prints its warnings; tested |
| A1.5 | **One clock per build**, carried by `Site` | Built twice per build (`site.rs:55`, `build.rs:77`), a third time in parity | One `SiteClock` constructed per build |
| A1.6 | **A shared base page context**: the keys every page template reads (url, title, layout, dates, description, toc, cover, frame switches) from one Rust builder, with mode keys added on top | Three hand-written `json!` blocks; pages pads post-only keys with `null` to satisfy strict undefined (`pages/src/lib.rs:168-175`) | One builder; the padding is explicit and documented in one table |
| A1.7 | **Route and output ownership** as §3.2; statics and stubs written from the planned routes, not recomputed | Statics planned as `Route::file` but written from raw ids (`site.rs:103` vs `build.rs:117-119`); stubs planned from `PlannedOutput` and written from `EntrySummary` (`site.rs:108-113` vs `build.rs:102-110`) | The writer's inputs are the planner's outputs |
| A1.8 | **Shared helpers where the contract needs one:** the date policy per mode documented in one place (blog refuses front matter `date`, micro ignores it, garden reads it: `blog/src/lib.rs:115-122`, `micro/src/lib.rs:73-77`, `garden/src/lib.rs:122-135`); mode roots normalized once (`discover.rs:154` trims slashes, `DocId::below` does not, `core/content/source.rs:58-60`) | Divergent rules surface when modes widen | Each mode's date rule is stated in core's docs and tested; a configured root with a trailing slash works |

Not in A1, deliberately: typed `CommonMeta` (v9 §5.2) as a refactor of its own, the single-pass
git dates (later risk 6), and the duplications in §9.2. A1 must leave the output for `BASELINE-B`
byte-identical to `0302e08`'s build, compared by the determinism recipe (A2.5).

## 5. Parity and validation strategy

### 5.1 HTML parity [DECIDED, Q15, 2026-11-01]

**Whole-document normalized DOM comparison, with an explicit deviations ledger.** The rendered
document is compared structurally, head and body, not only through fingerprints or text and URL
multisets. None of this exists yet beyond the `div.main-content` comparison (`pages.rs:222-233`);
A2 builds it.

- **Coverage:** the whole document: `<html>` attributes, every head element (meta, link, script,
  style, title), every body element with its attributes and text, in order, comments included,
  as spike 1's `div.main-content` comparison already treats them.
- **Normalization, and nothing else:** html5ever parsing; insignificant whitespace (between
  block elements, and runs collapsed outside `pre`, `textarea` and `script`); entity and
  character-reference forms (`&#x2f;` vs `/`); attribute order; class-token order. Any further
  normalization is a ledger entry (below), never a silent rule.
- **Hard assertions**, each a failure of its own, reported even if the DOM comparison is
  explained by the ledger:
  - `meta[name=robots]` equals Jekyll's (`NOINDEX, NOFOLLOW` today);
  - `link[rel=canonical]` equals Jekyll's, and is absolute on the site's URL;
  - `html@lang` equals Jekyll's;
  - on `/`, every `rel="me"` link and the root `h-card` (`u-url u-uid`, `p-name`) are present
    and equal Jekyll's (`_layouts/profile.html:21-23`);
  - on entries, `h-entry` with `p-name`, `e-content`, `dt-published` and `u-url` where Jekyll has
    them;
  - the redirect stubs' target and canonical (byte comparison already covers them).
- **Fingerprints stay, as diagnostics:** printed to explain a DOM difference, never the
  acceptance criterion.
- **Text-only outputs** (`/changelog`, a bare fragment today) are compared the same way as
  fragments.

### 5.2 The deviations ledger

`parity/deviations.toml` exists (D-001 to D-003, read by `parity markdown` only). A2 extends it
to every check.

- **An entry** names: id; status (`proposed`, `accepted`, `withdrawn`); the check and the outputs
  it explains (explicit paths, or a pattern only where the entry says why); what differs, and a
  matcher precise enough that a different difference on the same output still fails; why; who
  accepted it and when; a follow-up if any.
- **Rules,** as for Markdown today: only `accepted` entries suppress a failure; a `proposed`
  entry reports the failure as pending and still fails; an entry that explains nothing fails
  (stale entries cannot accumulate); withdrawn ids are not reused.
- **Review:** an entry is accepted by the author, in the plan's decision log or a step's review,
  never by the implementer alone. Each step's evidence README lists the entries it added or used.
- **Entries known to be needed** (to be written when their check exists): the feed-level
  `<updated>` (v9 §7.3, before C2); MiniJinja's whitespace and escaping where they survive §5.1's
  normalization; highlighting differences on built pages (Q12 decided the threshold; the ledger
  makes them explicit); every Q3 defect fixed rather than preserved.
- **D-003's** text is updated to its 2026-11-01 narrowing (§1.2).

### 5.3 Generated output comparators

Each lands with its generator (C2–C5); thresholds not given here are set from evidence then,
and recorded as decisions, not invented now.

| Output | Comparison | Known needs |
|-|-|-|
| Feeds | parsed Atom: feed metadata; entry set and order; per entry id, link, title, published, updated, categories, author, content or summary (normalized DOM) | feed `<updated>` by v9 §7.3's rule (ledger); D-003 changes garden summaries |
| `sitemap.xml` | `<url>` set; `lastmod` per URL | reordering between checkouts is not a difference (v9 §2.6); static files' mtime `lastmod` (Q3 or ledger) |
| `redirects.json`, stubs | bytes | |
| Search index | `(url, title)` set exactly; body similarity per entry above a threshold | threshold decided in C4 from the baseline's own variation |
| CSS, maps | bytes, or a semantic comparison | per Q7 |
| Note graph | multisets (built) | |

### 5.4 Coverage, determinism, baselines, negative tests

- **A2.1 Coverage report.** `just pages` classifies every manifest row as *built and checked*,
  *not built yet*, or *excluded* (an accepted ledger entry). Acceptance requires *not built yet* =
  0 (§8). The count is printed on every run.
- **A2.2 Unplanned outputs** fail, as the builder already enforces (`build.rs:131-138`).
- **A2.3 Negative tests.** Every new check gets a mutation in `scripts/pages-negative.sh` (or its
  successor) that must fail with a named message; `markdown-negative.sh` gains per-gate messages,
  as `pages-negative.sh` has.
- **A2.4 Hard assertions** (§5.1) each get a negative mutation.
- **A2.5 Determinism.** `just determinism` builds twice into fresh directories and compares them
  recursively, byte for byte. It also serves A1's "no output change" checks against a recorded
  build.
- **A2.6 Baseline verification** uses the parity tools' exclusions (`_folia/`,
  `.github/workflows/content.yml`, `routes.rs:315`) or documents moving them aside; one list,
  shared.
- **Evidence convention:** each step stores its reports under `parity/evidence/<step>/`, never
  overwriting an earlier step's, with a README naming commands, commits and the ledger entries
  used.

### 5.5 Front matter inventory (A3)

Implements v9 step 2's decisions 4 and 5 as migration requirements.

- A parity check lists every front matter key and sub-key of the 202 sources by mode and
  classifies each as **supported** (read and rendered), **accepted no-op** (known to have no
  effect in Jekyll, such as `trip`, or set to the default), **conditional** (meaningful only on
  some pages, judged by the mode), **unsupported** (changes the output; refused until built), or
  **pending decision** (the `narrow-post` typo). An unclassified key fails.
- Over-refusal is removed where Jekyll has no effect (§2.5); refusal stays where accepting would
  silently change output (v9 step 2, decision 4).
- A refusal reports every bad key of a file, not the first (`blog/src/post.rs:165-168`).

## 6. Remaining feature roadmap

Every step: `just check`, `just routes`, `just markdown`, `just pages`, the negative runs and
`just determinism` pass; the step's evidence is stored per §5.4; the step is reviewed before the
next starts, as for the slice. Each step's own acceptance is below. Types: **arch**itecture,
**impl**ementation, **val**idation, **clean**up.

### Phase A: contracts and parity (no output change)

| Step | Type | Scope | Depends on | Acceptance |
|-|-|-|-|-|
| **A1** | arch | §4 | none | output for `BASELINE-B` byte-identical to `0302e08`'s build |
| **A2** | val | §5.1–§5.4 | none; can run alongside A1 | the 4 built pages pass the whole-document comparison, or each difference is a reviewed ledger entry; every new check has a failing negative mutation; coverage report shows 296 not built |
| **A3** | val | §5.5 | A1.4 (diagnostics) | 0 unclassified keys across 202 sources; over-refusals removed with tests |

Sequencing: A2 precedes B because widening multiplies the frame over about 240 HTML outputs that
today's fingerprints check only loosely.

### Phase B: content coverage

| Step | Type | Scope | Depends on | Acceptance | Main risks |
|-|-|-|-|-|-|
| **B1 Garden** | impl | all 54 notes; ordering, backlinks, graph participation, future-date behaviour, metadata | A1–A3 | 54 notes pass §5.1; graph unchanged; dates of all 54 equal the dump | none known: all 54 pass today's gate |
| **B2 Pages** | impl | the Markdown pages (11 pass today's gate); then cover after title, `title_icon`, `corner_backlink`, `hide_title`, `author`/`show_author`; `/404.html` (the error layout and its script-free branches, `_includes/scripts.html:1`); `/changelog` (Q3) | A1–A3; Q3 for `/changelog` | every page source built and passing §5.1, or excluded by an accepted entry; their stubs byte-identical | the frame's untested branches |
| **B3 Blog** | impl | all 79 posts: `narrow_post`, the featured image after the title, `footer.extra`, cover credits, `cover: top` (Q3), title filters (Q18), interactions (Q16) | A1–A3; Q16, Q18 | 79 posts pass §5.1; dates and TOCs equal; Rouge agreement reported | Jekyll quirks are preserved unless Q3 decides otherwise |
| **B4 Micro** | impl | a micro `build`; bookmark, like, rsvp and note pages; `event` fields as written; the micro defaults (`_config.yml:287-336`: hidden frame, `hide_post_junk`, no pagination row, the corner backlink); 1 redirect | A1–A3; Q16 for syndications | 39 entries pass §5.1; ordering, dates, routes and future-date behaviour tested | Jekyll's micro layouts nest inside `post.html`; template naming (v9 §6.4: `micro/{kind}.html`) |

The order B1 to B4 is the recommended one. B2 and B3 are independent once phase A is done and
may swap; B1 goes first because every note already passes today's gate, so it is the cheapest
proof that A works.

### Phase C: generated and aggregate outputs

| Step | Type | Scope | Depends on | Acceptance |
|-|-|-|-|-|
| **C1 Listings** | impl | `/blog/` and pages 2–9, 10 category archives, categories and tags pages (blog); `/micro`, `/bookmarks`, `/likes` (micro); `/` with author data (Q16), per §3.3 | B1–B4 | all pass §5.1 and the `/` assertions; tie order equals Jekyll's (newest first, ties by path descending, observed in every listing checked) |
| **C2 Feeds** | impl + val | 51 jekyll-feed feeds and `/feed.xml`, Rust serializers (§3.4); the feed `<updated>` ledger entry; `smartify` on titles; the feed quirks per Q3 | B1–B4 (all entries); A1.3 | the feed comparator passes for all 52 |
| **C3 Sitemap, redirects** | impl | `sitemap.xml`, `redirects.json`, Rust | B (all routes final) | `<url>` set and `lastmod` per §5.3; `redirects.json` byte-identical |
| **C4 Search** | impl + val | the search index as Rust-generated, Lunr-compatible JS; Q3's micro and stub decisions | B, C1 | `(url, title)` set exact; body similarity above the threshold decided in C4 |
| **C5 Sass** | impl | `main.scss`, `dg_styles.scss`, their maps (Q7) | none; may run in any phase | CSS per Q7's parity contract |

### Phase D: integration and cutover (§7)

D1 link validation, D2 migration cleanup, D3 cutover preparation.

## 7. Migration cleanup and cutover preparation

These are roadmap items. None is to be done before its authorization (Q19), and none publishes
anything before the final approval.

- **D1 Internal link validation** (v9 §7.1): after rendering, every internal `href` and `src`
  in HTML resolves to a planned route, a static file, a redirect source or an anchor that exists
  on its target page. Failures name the page, the link and the target. Known Jekyll-origin
  broken links (the `stem cells` tag feed linked from `/blog/tags.html`; the broken wiki-link in
  `coding/rust.md`) are ledgered or fixed per Q3.
- **D2 Migration cleanup:**
  - merge SITE-PR (Liquid removal and its lint) into `filopoe`'s `main`, timing per Q19;
  - integrate the templates (`folia/templates`, rebased or re-applied onto the cutover base,
    since it is based on `SITE-MAIN`) without losing what was validated: the integrated `_folia/`
    must equal the validated tree file for file;
  - move the configuration to `filopoe/_folia/folia.toml` (Q8), with the content lint's workflow
    paths updated (Q13);
  - finalize the deviations ledger: every entry accepted or withdrawn, none proposed;
  - build a fresh baseline from the cutover commit with the production toolchain (Q17);
  - settle Q16 and Q20.
- **D3 Cutover preparation:** a Folia build and acceptance workflow in `filopoe`'s CI; a staged
  comparison of a Folia build with the Jekyll deploy of the same commit; the deploy procedure
  (Q6); a rollback procedure to the Jekyll deploy; updated `docs/baseline.md` and operational
  documentation.

## 8. Final acceptance criteria

Run against a baseline built from the cutover commit (Q17), by one command (F12). Criteria are
numbered F1 to F15 so they are not confused with the roadmap's steps.

| # | Criterion | Measure | Enforced |
|-|-|-|-|
| F1 | Every output accounted for | coverage report: every manifest row built and checked, or excluded by an accepted ledger entry; *not built yet* = 0 | automatic |
| F2 | No unplanned outputs | builder check, and parity's route check | automatic |
| F3 | Routes | exact multiset equality of (URL, output file), and of source classification | automatic |
| F4 | Byte-exact outputs | static files, redirect stubs, `redirects.json`, note graph, and CSS if Q7 so decides | automatic |
| F5 | HTML | §5.1 passes for every HTML output; every difference explained by an accepted entry | automatic; entries reviewed manually |
| F6 | Document assertions | §5.1's hard assertions pass on every page they apply to | automatic |
| F7 | Generated outputs | feeds, sitemap and search pass §5.3 at the thresholds decided in C2–C4 | automatic |
| F8 | Markdown and content | G1–G6 pass with accepted entries only; dates of every entry equal the dump | automatic |
| F9 | Front matter | 0 unclassified keys (§5.5) | automatic |
| F10 | Warnings | 0 warnings not on the documented list of expected warnings | automatic |
| F11 | Determinism | two clean builds identical (`just determinism`) | automatic |
| F12 | Reproducibility | one documented command runs F1–F11 and writes an evidence folder; it uses only pushed commits of Folia and `filopoe` and a baseline whose toolchain and lockfile hash are recorded | automatic run; manual check that inputs are published |
| F13 | Ledger | no `proposed` entries; each accepted entry has a decision reference | automatic + manual review |
| F14 | Visual review | a sample of pages per layout and viewport, compared side by side (v9 §9.5) | manual |
| F15 | Operations | deploy, rollback and documentation (D3) reviewed; the author's explicit approval to publish | manual |

Byte parity applies where F4 says; elsewhere parity is semantic, as §5 defines it. Exceeding
these criteria is not required: a difference that a criterion allows is not a defect.

## 9. Risks, dependencies and deferred work

### 9.1 Risks

| Risk | Kind | Evidence | Mitigation |
|-|-|-|-|
| Frame drift unseen | current gap | `pages.rs:186-197`; the step 3 tagline whitespace was caught only by `diff -r` | A2 |
| Generated outputs unchecked | current gap | no comparators in `crates/parity` | §5.3 with each C step |
| Liquid on `filopoe`'s `main` | known blocker | 15 Markdown files on `SITE-MAIN` | D2 (SITE-PR) |
| Title filters and `smartify` missing | latent defect | `_includes/head.html:1-3`, `sections/post.html:39`; jekyll-feed `feed.xml:23,57` | B3 (Q18), C2 |
| kramdown features not implemented | theoretical | footnotes, abbreviations, definition lists, math, ALDs, `{:toc}`: 0 uses (190 documents) | the dialect is frozen (v9 §5.3); parity catches new uses while it runs |
| 55 bodies equal only after normalization | open | at most 19 are explained by highlighting | identify the rest in A2, by the whole-document comparison's report |
| Tie order in listings and the combined feed | theoretical | Jekyll: newest first, ties by path descending; the combined feed sorts a concatenation with an unstable sort | C1, C2 comparators check order |
| Acceptance baseline vs production toolchain | open | production deploys with Ruby 3.4.5 and an untracked lockfile (`filopoe/.github/workflows/jekyll.yml:39`); the baseline used 3.3.8 | Q17 |
| Publishing `filopoe` commits may change hashes | operational | `check_checkout` requires the baseline's commit | rebaseline in D2 |
| Search quirks | open | substring exclusion (`assets/js/lunrsearchengine.js:9`), unescaped titles, `markdownify` over HTML, micro missing | Q3, C4 |

### 9.2 Deferred technical work [LATER]

Kept so it is not lost; each names when it must be revisited.

- **Dates with seconds:** SafeYAML reads `YYYY-MM-DD HH:MM:SS` as UTC; Folia reads the site's
  zone (`core/content/date.rs:88-102`). 0 uses. Revisit before the switch: refuse the form, or
  match Jekyll.
- **Impossible file-name dates:** Folia refuses them; Jekyll rolls them over. 0 uses.
- **Unicode key normalization:** Rust `trim` strips NBSP and other Unicode spaces, Ruby's `strip`
  does not; final capital sigma lowercases differently (`garden/src/wiki_links.rs:44-46`). 0 uses.
- **Liquid in graph titles:** Jekyll would run Liquid over the graph if a title held `{{` or `{%`.
  0 uses.
- **`titleize` and trailing `-`:** Rust's `split` keeps the trailing empty field Ruby drops
  (`blog/src/lib.rs:165-178`). 0 uses.
- **Single-pass git dates** (later risk 6), keeping `git log -- path` semantics.
- **The TOC analyses without wiki-link resolution** (`presentation.rs:139`) while garden bodies
  resolve them; latent while notes refuse `toc`.
- **Small duplications:** `file_route` ×3, the path-below-root expression ×2, the
  collect-every-problem loop, O(n²) document lookups ×3, two faithful category anchor rules in
  two crates (`blog/src/post.rs:204`, `folia/src/build.rs:261`). Fold in when a step touches
  them.
- **Mode independence is enforced by review only**; a dependency check (cargo metadata) would
  replace the per-step `scripts/slice-boundary.sh`.
- **`Cover.position` is a string** where `Toc.placement` is an enum; blog's check misses an
  unknown position without a path (`presentation.rs:79`, `blog/src/post.rs:175`). Fold into A3.
- v9's [LATER] items: CLI crate split, navigation model, renaming `hide_website_junk`, watch and
  serve, incremental builds, a Lunr replacement, visual regression tooling.

### 9.3 Dependencies outside Folia

`filopoe` (content, templates, SITE-PR, its CI), the local baselines directory, GitHub Pages
(Q6), and the Ruby toolchain for baselines (Q17).

## 10. Decision log

### 10.1 Decided

v9's decisions stand unless listed here as revised.

| # | Decision | Date |
|-|-|-|
| Q1 | Liquid removed from Markdown by a one-off codemod (D-001) | 2026-10-25 |
| Q2 | Folia Markdown dialect, with the IAL subset used | 2026-10-25 |
| **Q4** | **Listings are built in Rust;** modes own domain data, the builder orchestrates aggregation and output, templates render prepared contexts and never assemble collections; `/blog/` and its pages belong to the blog mode (§3.3). Revises v9 §6.3: the `site` index escape hatch is withdrawn. | 2026-11-01 |
| Q5 | Templates, content and assets stay in `filopoe`, Folia's in `_folia/` | 2026-10-25 |
| Q10 | D-002 withdrawn: the source was fixed | 2026-10-25 |
| Q11 | D-003, narrowed on 2026-11-01 to listings, cards and feeds | 2026-10-25 |
| Q12 | syntect with bat's grammars, Pygments classes, 90% rule | 2026-10-25 |
| Q13 | Content lint `parity liquid` (open parts: below) | 2026-10-26 |
| **Q14** | **Structured outputs by Rust serializers:** Atom feeds and the combined feed, `sitemap.xml`, `redirects.json`, the search index; templates for HTML only (§3.4). Settles v9 §7.3's "with `atom_syndication` or a template" for Rust. | 2026-11-01 |
| **Q15** | **HTML parity is a whole-document normalized DOM comparison** with an explicit ledger and hard assertions (robots, canonical, `lang`, `rel=me`, h-card); fingerprints are diagnostics (§5.1). Revises v9 §9.2. | 2026-11-01 |
| — | Workspace (v9 §4), MiniJinja (v9 §8), pulldown-cmark (v9 §5.3), the slice's step decisions (v9 §12.2) | as recorded in v9 |

### 10.2 Open

Each with options, trade-offs, a recommendation, and the latest point at which it must be
decided.

| # | Question | Options and trade-offs | Recommendation | Decide by |
|-|-|-|-|-|
| **Q3** | Preserve or fix Jekyll's defects: micro entries missing from search; stubs and `redirects.json` in the search index; the dead `stem cells` tag feed link; `/feed.xml`'s `"Filopoetica \| "` title and `gg` author name; `/changelog` as a bare fragment; `cover: top`; the micro `datetime` leading space; the `narrow-post` typo; RSS media type on Atom autodiscovery links | Preserving keeps parity simple and defers changes to after the switch; fixing during migration costs a ledger entry each and changes production output | Preserve by default; fix only where the defect is user-visible and the fix is small (micro search, `datetime`), each as an accepted entry; decide the typo with the source owner | per item, before the step that builds the output (B2, B3, B4, C2, C4) |
| **Q6** | Hosting, deploy and rollback | GitHub Pages as today (extensionless URLs and stubs verified live) vs another host | Stay on GitHub Pages; deploy through Actions artifacts as today; rollback = redeploy the Jekyll workflow | D3 |
| **Q7** | Sass compiler and source maps | `grass` (pure Rust; may not match `@import` plus silenced deprecations; maps differ) vs `dart-sass` (an external binary; can match the baseline's `sass-embedded` 1.100.0 output) | Try `grass` against the baseline's CSS; fall back to `dart-sass` if CSS differs; maps may be a ledger entry | C5 |
| Q8 | `_folia/folia.toml` as the configuration | — | Yes (in use as `parity/filopoe.toml`) | D2 |
| Q9 | Spike tooling (Python codemod, the Jekyll dump plugin) | — | Yes (in use) | closed by use |
| Q13 (rest) | Activate the lint on `filopoe`'s `main`; HTML-page lint; workflow paths for `_folia/folia.toml` | — | With D2 | D2 |
| **Q16** | Interactions (inter-pull: the JS form, `post_interactions.html`, the privacy policy's text) in scope? Where does author data live (covers, social handles, `bluesky_did`)? | In scope: port the include and extend `[authors]` in config; out of scope: ledger the removal, and update `pages/policy.md` | In scope (27 documents and a privacy statement depend on it); author data in `folia.toml`'s `[authors]`, typed | B3 |
| **Q17** | Acceptance baseline toolchain and timing | Rebaseline now with Ruby 3.4.5 (matches production; costs a re-run of all evidence) vs at D2 (one rebaseline, after `filopoe`'s commits are published) | Keep `BASELINE-B` (3.3.8) for development; rebaseline once at D2 from the published cutover commit with production's Ruby and a recorded lockfile; compare 3.3.8 and 3.4.5 builds of one commit once, early, to learn whether they differ | first comparison during A2; rebaseline at D2 |
| **Q18** | Title markup policy | Reproduce Jekyll's filters (`<br />` → space in the head; double space → `<br />` in the heading) vs edit the 3 titles in content | Reproduce in Rust (presentation data, not template logic) | B3 |
| **Q19** | Operational authorizations: moving Folia's `main` to `0302e08` and pushing; integrating `folia/templates` into `filopoe`; merging SITE-PR | Each is an explicit, separate authorization | Land `0302e08` before A1 starts, so A1's diffs are against `main`; templates and SITE-PR at D2 | as listed |
| **Q20** | What happens to the authored Jekyll listing and generator sources (`blog/*.html`, `pages/rss.xml`, `assets/js/lunrsearchengine.js`) once Rust owns their routes | delete in `filopoe` at cutover; keep as intro-text sources; exclude by configuration until cutover | Exclude by configuration until cutover, delete at cutover | C1 |

### 10.3 Superseded

- v9 §6.3 [OPEN, recommended] listing pages: decided as Q4.
- v9 §9.2's per-page fingerprint as acceptance: replaced by Q15.
- v9 §12.3's "the next plan then picks between widening a mode and later risks 5 and 6": §6.
- v9 Q6's and Q7's recommendations are carried into §10.2 with options.

## 11. Non-goals

v9 §13 stands: no navigation model, no `Mode` trait or plugin registry, no RSS 2.0, no template
processing of Markdown bodies. In addition: no template-generated structured formats (Q14), and
no fixes to Jekyll's behaviour during migration beyond those Q3 accepts.
