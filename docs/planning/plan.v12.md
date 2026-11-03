# Folia --- Project Plan (v12)

**Status:** Planning draft, revision 12 (supersedes `plan.v11.md`)\
**Project type:** Personal, in-house static-site publishing engine\
**Implementation language:** Rust\
**Primary reference site:** `ggirelli.info`, Jekyll source in the `filopoe` repository\
**Repository:** `https://github.com/ggirelli/folia.git`, Jujutsu colocated with Git

Markers used in this document:

- **[DECIDED]**: settled; change only with new evidence or the author's explicit revision.
- **[OPEN]**: needs the author's input; options, a recommendation and a decide-by milestone are
  given.
- **[LATER]**: deliberately deferred; listed in §9.2 with a revisit milestone or a failing check.

Status words, used strictly (§2 applies them):

- **validated**: built, and checked against the baseline by a repeatable command whose output is
  kept as evidence;
- **built**: the code exists and runs, with unit tests at most;
- **routed**: only the output's URL and file are planned (route parity);
- **designed**: decided on paper, no code;
- **simulated**: a count derived by an ad hoc script during the assessment, not by a kept check;
- **not started**: none of the above.

Three operations are kept apart throughout:

- **push**: sending commits to a remote branch;
- **deploy**: a workflow publishing a site to a host (Folia's own docs deploy on every push to
  Folia's `main`, `.github/workflows/docs.yml`; `filopoe`'s Jekyll deploy runs only by hand,
  `filopoe/.github/workflows/jekyll.yml:9-11`);
- **publish**: making the migrated site, or anything from the migration, public as the site.

**Nothing from the migration may be published or deployed until the migration is finalized and
explicitly approved.** A push needs its own authorization (Q19), and must not trigger a deploy
that publishes migration material unless that deploy is authorized too. The Jekyll site stays the
production site.

## 0. How to read v12

v12 revises v11 after the read-only audit of checkpoint A2.3 and the read-only A2 acceptance
review (both 2026-11-03). **It changes no architectural decision, no phase or its order, and no
acceptance criterion.** It records the project owner's decisions of 2026-11-03 (§10.1), the A2
checkpoints delivered and what of A2 remains (§5.4), and corrects the statements of v11 that the
work has made stale (§1.2). Section numbers are v11's, so the historical references that cite
v11 (`plan v11, §5.1`: the evidence folders and D-004's `decision`) still name the same sections;
the code and the current documentation cite v12.

v11 stays as written, as history. The owner's decisions of 2026-11-03 on §5.1, §5.2 and F13,
after the A2.3 audit, are recorded in v12 only.
D-004's `decision` cites v11's §5.2, the reference used when it was re-accepted that day; the
text it relies on is v12's §5.2.

v11 itself revised v10 after its independent review (verdict: revise; no critical findings, no
structural redesign). The three decisions of 2026-11-01 (Q4, Q14, Q15) are unchanged; v11 made
them implementable and consistent with the existing checks.

- **Superseded in v9, and still superseded:** §6.3 (listing pages), the parity strategy of §9
  (except item 5, visual checks, which F14 now defines), the roadmap of §12.3 and the decision
  list of §14. Their replacements are §3, §5, §6 and §10.
- **Carried forward from v9 by reference**, current and not restated, except where v11 says so at
  the point of change: §2 (evidence from the site), §4 (workspace shape), §5.1–§5.3 (content
  model, metadata tiers, Markdown), §6.2 (publishing shape; restated in part in §3.6), §6.4
  (templates are runtime files), §7.1 (site assembly), §7.2 (content lint), §7.3 (feeds and their
  timestamp rules; restated in part in §3.6), §7.4 (search; revised by §3.5), §8 (MiniJinja),
  §10 (spikes and the rules they settled), §11 (build pipeline; its stage order is revised by
  §3.7) and §12.2 (the records of slice steps 1 to 3).
- **Evidence** is cited by path under `parity/evidence/` and by source location. The
  migration-wide assessment and the v10 review (both read-only, 2026-11-01, against Folia
  `0302e08`, `filopoe` `BASELINE-B` and baseline `BASELINE-B`) are the source of counts not otherwise
  attributed. Remote refs (`origin/*`) are as of the last fetch, not necessarily the live remote;
  only SITE-PR was read live.

Changes since v11:

- §0, §1: status as of 2026-11-03; the repository state re-observed (§1.1); v11's stale
  statements listed and corrected (§1.2).
- §2: the whole-document comparison, the ledger and the hard assertions on the 4 built pages are
  validated (§2.2); the page frame of those pages is checked (§2.3).
- §5.1: the comparison as built (whole document, every difference), not as planned; HTML
  whitespace, full report values and `""` for one-sided text, as decided on 2026-11-03.
- §5.2: the ledger as it is (D-001 to D-004); every stale entry fails; Folia's side bound by its
  output manifest, as decided on 2026-11-03.
- §5.4: the checkpoints delivered, with the 2026-11-03 corrections and the source identity policy
  (an unknown identity fails `parity pages`, a source that is not a git checkout among them);
  checkpoint A2.3 complete; A2's status: done, remaining, deferred, undecided; the evidence
  provenance convention (Q26); work item A2.1, the coverage classes, built.
- §6: A2's status noted under its row; its scope and acceptance unchanged.
- §9.1: frame drift closed for the pages built.
- §10: the decisions of 2026-11-03 (§10.1), Q25 among them (raised and closed the same day: an
  unused `match` item fails the run) and Q26 (evidence provenance), each raised and closed the
  same day.

Changes in v11, since v10:

- §1, §2: status and citation corrections from the review (§1.2 lists them).
- §3: how route parity models outputs a mode or the builder claims from a legacy source (§3.4);
  where the legacy listing sources' redirects, frame switches and search text go; the search
  record model (§3.5); `FeedSpec` and the combined feed restated (§3.6); Render before Generate
  (§3.7).
- §4: the build set defined; stub rule; a reference build and Folia-to-Folia change approval;
  micro refuses a front matter `date`.
- §5: an implementable DOM contract, ledger schema and matcher, every-difference reporting,
  ledger negatives; thresholds derived from baseline variation.
- §6: dependencies corrected (A1 on A2.5; A3 on A1.1 and A1.6; route-level link checks early;
  C2/C3 may start earlier); B2's scope narrowed; `/` placed in B2.
- §8: F1–F16 revised (v10's F15 split into F15 and F16).
- §10: Q3 split per defect; Q16 split; new Q21–Q24; Q9 closed; Q20 moved to A2.

## 1. Executive summary and status

Folia plans every output Jekyll writes, exactly, and what it builds matches Jekyll. It builds very
little yet: 6 of the 302 outputs that are not static files. The architecture holds; four shared
contracts should be hardened before any mode is widened, because each costs more with every mode
that depends on it. The parity tooling was strict for content and weak for the page frame and for
generated outputs, so it is hardened in the same first phase: since A2's checkpoints A2.1 to A2.3
(§5.4) the 4 pages built are compared as whole documents, judged by the deviations ledger and the
hard assertions; generated outputs still have no comparator (§5.3), and the rest of A2 remains.

| Area | Status |
|-|-|
| Routes | validated: 1,077 of 1,077 outputs planned with Jekyll's URL and file |
| Static files | validated: 775 of 775 byte-identical |
| Markdown bodies and excerpts | validated: 190 of 190 bodies, 172 excerpts (19 under D-003) |
| Rendered documents | validated for 4 (`/about`, one post, two notes) of 202 sources, as whole documents since checkpoint A2.3: 8 differences, all explained by D-004; 23 hard assertions hold (`parity/evidence/a2-3-closure/`) |
| Redirect stubs | validated for 1 of 26 |
| Note graph | validated, byte-identical |
| Feeds, sitemap, `redirects.json`, search, Sass, pagination, archives, listings, micro | routed or designed; not started |
| Not built | 296 of 1,077 outputs (`slice-step3-remediation/pages.txt:3`) |

The roadmap (§6, §7) has four phases:

- **A. Contracts and parity**, with output unchanged except by approved changes (§4.2): the
  reference build (A0), parity infrastructure (A2), shared contracts (A1), a site-wide front
  matter inventory (A3).
- **B. Content coverage**, one mode at a time to all its documents: garden (B1), pages and `/`
  (B2), blog (B3), micro (B4).
- **C. Generated and aggregate outputs:** listings and archives (C1), feeds (C2), sitemap and
  redirects (C3), search (C4), Sass (C5).
- **D. Integration and cutover:** link validation (D1, begun in A2), migration cleanup (D2),
  cutover preparation (D3).

### 1.1 Repository state (observed 2026-11-03, read only; `origin/*` as last fetched, not fetched)

Repository state observed on 2026-11-03, before the commits that add this plan and the
implementation it describes; *local and uncommitted* below refers to that moment.

| Repository | State |
|-|-|
| Folia | `f671c72` on a detached HEAD: on step 3 (`0302e08`) and the pulldown-cmark 0.13.6 upgrade (`7f34662`), A0's reference build (`dc02602`), plan v10 (`af972ff`), checkpoints A2.1 to A2.3 with their hardening and plan v11 (`33ffaa9`) and the dependency evidence (`f671c72`). Pushed, with the author's authorization, only to the review branch `parity/a2-foundation` = `f671c72`: draft PR #1 into `main`, open, not merged. `main` = `origin/main` = `5e9c57a` (step 2 and the docs); moving `main` is still Q19(1). The 2026-11-03 corrections (§5.4) and this file were, when observed, local and uncommitted. |

The state of the site's repositories, workspaces and baselines is recorded privately, with the
site's inputs.

### 1.2 Stale statements, reconciled

Stale in v11, corrected in v12 (2026-11-03):

| Statement | Where (v11) | Actual state |
|-|-|-|
| Checkpoint A2.3's hardening reports *partial staleness* without failing | §5.4's checkpoint table | Superseded by the owner's decision of 2026-11-03: every stale entry fails the run; *partly stale* no longer exists (§5.2, §5.4) |
| Only `div.main-content` is compared as a DOM | §5.1 | The whole document is compared (`crates/parity/src/document.rs`) since checkpoint A2.2, and decides `just pages` since A2.3; the `div.main-content` comparison remains as a check of its own |
| `dom::first_difference` stops at the first difference | §5.1 | The whole-document comparison lists every difference per output; `first_difference` serves only spike 1's Markdown check and the `div.main-content` check |
| `parity/deviations.toml` holds D-001 to D-003, read by `parity markdown` only | §5.2 | It also holds D-004 (`check = "dom"`), read by `parity pages` |
| Robots, `html@lang` and the page frame are unchecked | §2.3 | On the 4 pages built, the whole document is compared and robots, canonical, title, description, `lang`, h-entry and the stub are asserted (§5.1) |
| Frame drift unseen, a current gap | §9.1 | Closed for the pages built; open for every output not yet built |
| Folia at `0302e08`; plans v10 and v11 untracked | §1.1 | Committed through `f671c72`, pushed to `parity/a2-foundation` only (§1.1) |
| No record of the corrections of 2026-11-03 | §5.4 | Recorded in §5.4 and §10.1 |
| D-004 "equal once whitespace runs are collapsed" | `parity/deviations.toml` | Its `change` and `boundary` say HTML whitespace (§5.1); scope unchanged |
| An entry is stale only if none of its differences occurs; one that explains part of what it lists is *partly stale*, reported without failing; whitespace for collapse and trim undefined; Folia's file identified by the hash of its bytes only | §5.1, §5.2, F13 | Superseded by the owner's decisions of 2026-11-03, recorded in v12's §5.1, §5.2 and F13 |

Carried from v11, unchanged unless stated:

| Statement | Where | Actual state |
|-|-|-|
| Step 3 "uncommitted and not reviewed" | `plan.v9.md:20-22`, `:1724-1726` | Reviewed (accept with limitations), remediated, and committed with the author's authorization as `0302e08` (detached, unpushed) |
| Step 1's templates "still uncommitted" | `plan.v9.md:27-28`, `:53`, `:1360` | Committed on `folia/templates` (`TEMPLATES-COMMIT-A`, `TEMPLATES-COMMIT-B`), local |
| "Built and checked locally; not committed" | `parity/evidence/slice-step3-remediation/README.md:5` | That README is part of `0302e08`; to be corrected in A2 (evidence READMEs are updated, never their reports) |
| Content lint "not yet pushed", pin `PENDING` | `plan.v9.md:49-50`, `:648-650`, Q13 | The lint is on Folia's `main` (`6ef4a49`); `folia/content-lint` pins it and is pushed with SITE-PR, whose check passed; only the untracked copy in the default workspace says `PENDING` |
| `SITE-COMMIT-A` and `SITE-COMMIT-B` exist only in the local checkout | `docs/baseline.md:276-277` | Both are on `origin/folia/content-lint`, not on `main` |
| Parity "deliberately does not depend on Folia" | `plan.v9.md:326-327` | `crates/parity` depends on `folia`, `folia-core` and `folia-garden` (`crates/parity/Cargo.toml:12-14`). The rule is: parity uses Folia's public API to build and plan, and never its internals to judge. |
| D-003's `change` text: page descriptions show the fix | `parity/deviations.toml:98-106` | Narrowed on 2026-11-01 (v9 Q11): note page descriptions keep raw wiki-links. Updated in A2. |
| The feed-level `<updated>` deviation "is recorded in the deviation ledger" | `plan.v9.md:731-734` | No such entry exists; added in A2 as `proposed`, accepted before C2 |
| `docs/baseline.md` describes step 1 only and calls the templates uncommitted | `docs/baseline.md:221-241`, `:296-297` | Updated in A2 |
| `just baseline-verify BASELINE-B` reproduces a baseline | `docs/baseline.md:286-291` | It fails, for two reasons, in this order: `scripts/baseline.sh:39` stops with "already exists" because the documented label `BASELINE-B` is an existing baseline; with a new label, `baseline.sh:42-47` stops because the untracked `_folia/` and `content.yml` count as dirty, which the parity tools exclude (`crates/parity/src/routes.rs:315`). `scripts/verify-baseline.sh:22` cannot pass `--allow-dirty`. Fixed in A2.6. |
| Plan v9 is current | `mkdocs.yml` nav ("Plan v9 (current)"), `docs/planning/index.md`, `README.md`, `docs/index.md` | v11 is current once reviewed. Recommended (Q19(1)): update the navigation and index in the commit that adds v10 and v11, before any push of Folia's `main`; none of this is authorized yet. *(v12: `33ffaa9` named v11 current; v12's change, uncommitted when observed (2026-11-03), names v12; Folia's `main` was not moved)* |

## 2. Verified implementation baseline and evidence

### 2.1 What Jekyll writes, and what Folia builds

From `folia-baselines/BASELINE-B/dump/manifest.jsonl` (1,077 rows, all written) and
`parity/evidence/slice-step3-remediation/`.

| Output group | Jekyll | Folia built | Status | Evidence |
|-|-:|-:|-|-|
| Static files | 775 | 775 | validated | `pages.txt:25` |
| Posts | 79 | 1 | 1 validated | `pages.txt` |
| Garden notes | 54 | 2 | 2 validated | `pages.txt` |
| Micro entries | 39 | 0 | routed | `routes.txt` |
| Page outputs (30 sources; `blog/index.html` written 9 times) | 38 | 1 | 1 validated | `pages.txt` |
| Category archives | 10 | 0 | routed | `routes.txt` |
| jekyll-feed feeds (blog, 44 tags, micro, 4 micro kinds, garden) | 51 | 0 | routed; designed (§3.6) | `routes.txt` |
| Combined `/feed.xml` (counted among the pages) | (1) | 0 | routed as a page | `routes.txt` |
| Redirect stubs | 26 | 1 | 1 validated | `pages.txt:25` |
| `redirects.json` | 1 | 0 | routed | `routes.txt` |
| `sitemap.xml` | 1 | 0 | routed | `routes.txt` |
| Source maps | 2 | 0 | routed | `routes.txt` |
| Note graph | 1 | 1 | validated, byte-identical | `pages.txt:23-24` |
| **Total** | **1,077** | **781** | 296 not built | `pages.txt:3` |

Garden coverage, precisely: all 54 notes are routed and are nodes of the validated graph; 2 are
rendered and validated; all 54 pass today's front matter gate (*simulated*).

### 2.2 Validated

| What | Check | Evidence |
|-|-|-|
| Every output's URL and file; source classification; generated outputs; stubs | `just routes` (exact multisets) | `slice-step3-remediation/routes.txt` |
| 190 Markdown bodies (135 identical, 55 equal after normalization) | `just markdown`, gates G1–G6 | `slice-step3-remediation/markdown.txt:4` |
| 172 excerpts (19 explained by D-003) | `just markdown`, G5 | `spike1/README.md:6,28`; `slice-step3-remediation/markdown.txt:13` |
| 775 static files, 1 stub | `just pages`, byte comparison | `pages.txt:25` |
| `/about`, `webmentions`, `coding/index`, `coding/rust`: title, canonical, description, Open Graph, headings, link/resource multisets, `div.main-content` DOM, body text; TOC; dates (value and origin); code blocks | `just pages` | `pages.txt:4-21` |
| Note graph (54 nodes, 73 edges, byte-identical), broken-link warning | `just pages` | `pages.txt:22-24`; counts in `slice-step3-remediation/README.md:19` |
| The 4 pages as whole documents (§5.1 rules 1 to 7, every difference by path and kind): 8 differences, each explained by D-004 for the files reviewed; none stale; Folia's manifest agrees with its pages; deterministic reports | `just pages` | `a2-3-closure/pages.txt`, `dom-report.json` |
| The hard assertions (§5.1) on the 4 pages and the stub: 23 hold | `just pages` | `a2-3-closure/pages.txt` |
| The source identity of each build (§5.4): an unknown identity fails | `just pages`, `just ledger-negative` | `a2-3-closure/` |
| The checks fail when they should | `just pages-negative`, `just markdown-negative`, `just ledger-negative` | `a2-3-closure/pages-negative.txt`, `a2-3-closure/ledger-negative.txt`, `spike1/` |
| Liquid kept out of Markdown prose | `just liquid` (190 documents) | `liquid.txt`; SITE-PR's check |
| Two builds identical | ad hoc `diff -r` | `slice-step3-remediation/README.md` (no recipe yet: A2.5) |

Highlighting agrees with Rouge on 96.3% of token classes across the site (Q12's 90% rule;
`slice-step3-remediation/markdown.txt:33`); the agreement is reported, not enforced, by both
`parity markdown` and `parity pages` (`pages.txt:10-17` for the built pages). F8 makes the floor enforced.

### 2.3 Built, not validated

- **Dates:** front matter and file-name dates, the future-date holdback (blog, micro, garden),
  git `modified` dates. Unit tests cover them; parity covers the 4 built entries. No content is
  future-dated or unpublished today.
- **Page frame:** on the 4 pages built it is checked since checkpoint A2.3: the whole document
  is compared (§5.1), so every element and attribute of the frame is, and the robots
  `NOINDEX, NOFOLLOW` meta (`filopoe/_includes/head.html:24`), `<html lang>`, the canonical
  link, title and description are asserted. The frame's branches those pages do not take are
  not built, so not checked; each B and C step brings its outputs under §5.1. (v11 recorded the
  frame as unchecked outside the fingerprint, `crates/parity/src/pages.rs:186-197`.)
- **Templates:** ported with only the branches the four built pages take; 24 of Jekyll's 53
  includes and 5 of its 12 layouts have a port (`git ls-tree -r SITE-COMMIT-B _includes`).

### 2.4 Not started

- **Generators:** no code exists for Atom feeds, the sitemap, `redirects.json`, the search
  index, Sass, pagination, archives or listings; only their routes are planned (`site.rs:97-194`,
  each mode's `routes`). `ModeOutput` (`crates/core/src/publish.rs:53-62`) has no feeds.
- **Micro:** no `build` function (`crates/micro/src/lib.rs:37` is its only public function).
- **HTML pages:** the pages mode refuses every non-Markdown source
  (`crates/pages/src/lib.rs:138`): `404.html`, six `blog/*.html`, `profile/index.html`,
  `pages/rss.xml`, the search JS and two `.scss`.
- **Post-render link validation** (v9 §7.1): no code.
- **Content comparators** for feeds, search, sitemap, `redirects.json` and CSS: none.
- **A determinism recipe and a pinned build time:** none; the CLI builds as of
  `Timestamp::now()` (`crates/folia/src/main.rs:48`), and the footer shows the build's year.

### 2.5 What the content needs that Folia refuses (simulated)

Simulating the current front matter gates over all sources (not a kept check; A3 makes it one):

- **Posts:** 1 of 79 passes. Main reasons: a cover after the title (the default; 77 posts, of
  which 62 show an image), `narrow_post` (58), interaction syndications (18), `trip` (10),
  `featured` (6), `use_dark_layout` (4), `footer` (2), `hidden`, `category`, `cover: top` (1 each).
- **Page sources:** 11 of 27 Markdown and HTML sources pass. Of the 16 refused, 9 are not
  Markdown (one of them, `404.html`, also sets `error-type`); the rest hit a cover after the
  title (4), `title_icon`, `corner_backlink`, `hide_title`, the `narrow-post` typo, or a missing
  layout. `author`,
  `show_author`, `additional_body_class`, `collections` and `pagination` occur only in sources
  that become claimed by Q4 (§3.4) and in `profile/index.html`.
- **Garden:** 54 of 54 pass.
- **Micro:** 0 of 39 can build (no builder); keys `bookmark` 11, `like` 8, `rsvp` 7, `event` 7,
  `syndications` 10, `narrow_post` 16, `redirect_from` 1.

Some refusals are of keys with no effect in Jekyll (`trip` is read nowhere; `featured: false`,
`hidden: false`, `use_dark_layout: true` are defaults): over-refusal, addressed by A3.

## 3. Target architecture and ownership

v9 §3 (principles), §4 (workspace) and §6.1 (what a mode owns) stand.

### 3.1 Three layers

| Layer | Owns | Must not |
|-|-|-|
| **Domain mode** (`folia-blog`, `-micro`, `-garden`, `-pages`) | typed metadata, validation, routes, ordering, intra-mode links, body HTML, its derived pages and their *data* (pagination, archives, kind listings), membership of its feeds, per-entry searchability, its artifacts, and the front matter of the legacy sources it claims (§3.4) | read another mode, load templates, serialize site-wide formats, write files |
| **Builder** (`folia`: `site`, generator modules, `render`, `output`) | orchestration; the route table and collision checks; cross-mode aggregation (combined feed, search, sitemap, redirects, frame data); serialization of structured outputs; rendering; link validation; writing | interpret a mode's front matter |
| **Templates** (`filopoe/_folia/templates`) | HTML markup and display text for prepared contexts | query, filter, sort or assemble site-wide collections; produce non-HTML formats |

**Frame data** (site identity, authors, the build's year, category counts) is computed in Rust
and given to every page. Category counts come from a named blog function today
(`folia_blog::category_counts`, called by `build.rs` `site_index`); A1.6 makes frame data one
documented input of the base page context, so its data path is explicit.

### 3.2 What is generated where

| Output | Data from | Assembled by | Serialized by | Validated by (§5) |
|-|-|-|-|-|
| Entry pages (posts, notes, micro, pages) | mode | mode (`PageSpec`) | template | DOM + assertions |
| `/` (`profile/index.html`, layout `author`: author data, no collection) | pages, with author data from config (Q16b) | pages (B2) | template | DOM + assertions (`rel=me`, h-card) |
| `/404.html` | pages | pages (B2) | template | DOM + assertions |
| `/blog/`, `/blog/pageN/`, category archives, `/blog/categories.html`, `/blog/tags.html` | blog, claiming the legacy sources (§3.4) | blog | template | DOM + assertions |
| `/micro`, `/bookmarks`, `/likes` | micro, claiming the legacy sources (§3.4) | micro | template | DOM + assertions |
| Per-mode feeds | mode (`FeedSpec`, §3.6) | builder | Rust Atom serializer | feed comparator |
| Combined `/feed.xml` | `EntrySummary`s of the configured modes (§3.6) | builder, claiming `pages/rss.xml` | Rust Atom serializer | feed comparator |
| `sitemap.xml` | route table, `modified` | builder | Rust XML serializer | sitemap comparator |
| `redirects.json`, stubs | every `redirect_from`, including the claimed sources' (§3.4) | builder | Rust | §5.3 |
| Search index | §3.5 | builder, claiming `assets/js/lunrsearchengine.js` | Rust (§3.5) | search comparator |
| Note graph | garden | garden (`Artifact`) | garden | graph comparator (built) |
| CSS and maps | `assets/css/*.scss` | builder (assets) | Sass compiler (Q7) | CSS comparator |

### 3.3 Listing ownership [DECIDED, Q4, 2026-11-01]

**Rust builds listings.** Domain modes own their domain data and its transformations; the
builder orchestrates aggregation across modes and generates outputs; MiniJinja templates receive
prepared, typed presentation contexts and only render them.

- A listing's selection, filtering, ordering, grouping, pagination and the per-item values a
  card shows (title, URL, date, excerpt text, cover, categories) are computed in Rust. The
  template decides markup and display text.
- **`/blog/` belongs to the blog mode,** with pages 2–9 and the category and tag index pages;
  the micro listings belong to micro. Today `/blog/` is a pages-mode output of `blog/index.html`
  while pages 2–9 are blog outputs (`crates/blog/src/lib.rs:32,209`); that split ends in C1.
- **The `site` index is frame data only.** v9 §6.3's escape hatch (a `site` index of every
  `EntrySummary` that templates may query) is withdrawn.
- **Intro prose** of `/micro`, `/bookmarks` and `/likes` lives in the template, unless C1 finds
  text that differs per page and belongs in content; settled in C1.

### 3.4 Claimed legacy sources

Eight Jekyll page sources produce outputs that Rust now owns: `blog/index.html` (9 outputs),
`blog/categories.html`, `blog/tags.html`, `blog/bookmarks.html`, `blog/likes.html`,
`blog/microblog.html`, `pages/rss.xml` and `assets/js/lunrsearchengine.js`. Jekyll still needs
them until cutover, and they carry behaviour beyond their Liquid bodies:

| Behaviour in the source | Example (evidence) | Where it goes |
|-|-|-|
| `permalink` | every source but `blog/index.html`, whose outputs come from pagination | the owner's routes, which must equal it (checked: §5.5, A3) |
| `redirect_from` (6 of the 26 stubs) | `blog/bookmarks.html`: `/links`, `/postroll`, `/micro/bookmarks`; `blog/likes.html`: `/micro/likes`; `blog/microblog.html`: `/microblog`, `/social` | read by the owner from the claimed file's front matter, as data; stubs and `redirects.json` keys follow |
| Frame and page switches | `title`, `author`, `show_author`, `comments`, `hide_website_junk`, `additional_body_class` (`blog/categories.html`, `blog/tags.html`) | read by the owner from the front matter, classified by A3 like any page's |
| Selection and ordering | the Liquid bodies (`site.micro \| sort: "date" \| reverse`, the paginator) | reimplemented in Rust (Q4); the body is not rendered |
| Combined-feed membership | `pages/rss.xml`'s `collections: posts, digital_garden, micro` | `folia.toml` (§3.6); A3 checks the two agree until cutover |
| Search inclusion | `assets/js/lunrsearchengine.js`'s `sitemap: false` and its loops | §3.5 |
| Search body text | Jekyll indexed these pages' content as it stood at index time, raw Liquid for some | §3.5 |

**The model (designed; Q20 confirms it in A2):**

- **Claimed, not excluded.** The files stay discovered. The owning mode or the builder declares,
  in a fixed table in its own code with explicit paths, which legacy sources it claims and the
  outputs each yields; it reads their front matter as data and never renders their bodies.
- **Route parity** gains a source class *claimed by `<owner>`*. `check_classification` accepts a
  Jekyll `page` as a Folia *claimed* source only for paths in the claim tables; any other
  mismatch still fails. `check_authored` is unchanged: the owner's outputs carry the claimed file
  as their `source`, so (source, URL, output) still equals Jekyll's. Every claim must yield
  exactly the outputs Jekyll wrote from that source.
- **`missing_templates`** (`crates/folia/src/site.rs:196-215`) accepts a generated output whose
  source is either an authored output or a claimed source; an unclaimed, unpublished source still
  fails.
- **Incremental migration:** until its step builds them, a claimed source's outputs are planned
  and counted as *not built yet* (§5.4), exactly as today; route accounting stays exact.
- **At cutover** the files remain in `filopoe` until the rollback window closes (Q6), because a
  Jekyll rollback needs them; deleting them, and moving their front matter into content or
  configuration, is a later decision (Q20).

### 3.5 Search [revises v9 §7.4]

**Record model (designed).** The index is a list of records `{id, url, title, body}`, the shape
of the baseline's `var documents` (193 records). Jekyll's records come from `site.pages`, the
garden and the posts (`filopoe/assets/js/lunrsearchengine.js:7-73`), so they include, besides 79
posts and 54 notes: the 18 Markdown pages, `/`, `/404.html`, `/blog/` and pages 2–9,
`/blog/categories.html`, `/micro`, `/bookmarks`, `/likes`, the 26 stubs and `/redirects.json`
(whose body is the text of `redirects.json`, so search follows C3); `/blog/tags.html` is excluded
by a substring rule (`contains 'tag'`, line 9) and micro entries are absent (the dead loops, v9
§2.5). Titles are interpolated unescaped, and bodies went through `markdownify` over HTML (Q3q).

- **Record sources:** each mode, and the builder for its own outputs (stubs, `redirects.json`),
  says which outputs are searchable and gives a title. Membership reproduces Jekyll's, and each
  defect in it is a separate decision (Q3c, Q3d, Q3e).
- **Bodies:** entries' bodies are text from their `body_html`, by the v9 §7.4 chain. For other
  pages, Jekyll's body is `page.content` at the moment the index rendered, which is rendered text
  for some (`/blog/`) and raw Liquid source for others (`/micro`, `/blog/page2/`). Whether to
  reproduce that is Q3f; the parity contract compares bodies by similarity (§5.3), not bytes.
  If Q3f preserves Jekyll's bodies, the claimed sources' raw text is an input to search
  generation; otherwise the rendered pages are.
- **Stage order:** search is generated after rendering (§3.7), so rendered page text is
  available whichever way Q3f goes.
- **The file:** `assets/js/lunrsearchengine.js` holds the generated `documents` array and about 60
  lines of site UI code (`lunr(...)`, `lunr_search`, offcanvas and jQuery calls, lines 75-138).
  **Recommended (Q20, whose model is decided in A2):** keep one output at the same route, written by
  the Rust serializer as the generated array followed by the UI code read verbatim from a static
  JavaScript fragment in `_folia/`, with no templating; route parity and byte layout then stay as
  they are. Splitting it into static JS plus a generated data file is the alternative; it adds a
  route and changes the page head, so it would be an accepted deviation.

### 3.6 Feeds [restates v9 §6.2's `FeedSpec` and §7.3]

- **Per-mode feeds:** each mode declares its feeds as `FeedSpec { route, title, entries,
  limit }`: entries are `DocId`s in the mode's order, and `limit` is 10 today. The blog declares
  its feed and 44 tag feeds (the tag filter `[a-zA-Z0-9_]`, `crates/blog/src/lib.rs:228-235`),
  micro its feed and 4 kind feeds, garden its feed. `ModeOutput` gains `feeds` in C2.
- **Combined feed** [DECIDED in v9 §7.3]: declared in `folia.toml`
  (`include = ["blog", "garden", "micro"], limit = 10`) and assembled by the builder from
  `EntrySummary`s, newest first. Its title and author follow Q3b.
- **Timestamps:** v9 §7.3's rules stand: entry `modified` (front matter `last_modified_at`, then
  the last commit, then `published`); feed-level `<updated>` from the included entries, never
  the build time, as a ledgered deviation.
- **Fields entries need** beyond A1.3's: the Atom `<id>` differs from the route (an id without
  the date prefix, for example `https://ggirelli.info/dg/book-club/book-quotes/metafisica-orientale`
  for the dated URL), and `/feed.xml` reads `author`, `lang`, `image` and `description ||
  excerpt` (`_layouts/feed.xml`). C2 adds these to `EntrySummary` (or `FeedSpec`
  entries) after settling the schema from the baseline; A1.3 does not guess them.

### 3.7 Stage contracts (revises v9 §6.5 and §11's order)

| Stage | Input | Responsibility | Output |
|-|-|-|-|
| **Plan** (`folia::site::plan`) | discovery, every mode's `routes`, claim tables | the route table; duplicate and collision checks | `Site`, with its `SiteClock` (A1.5) |
| **Mode build** | its `SourceDoc`s, claimed sources, the build context (A1.2) | §3.1's mode duties | `ModeBuild`: output and diagnostics (A1.4) |
| **Assemble** (`folia::site`) | every `ModeOutput`, the route table | aggregation (§3.2), frame data | page specs and generator inputs |
| **Render** (`folia::render`) | page specs, templates | template check, then rendering | HTML keyed by `Route` |
| **Generate** (generator modules) | generator inputs, rendered HTML where needed (search, §3.5) | serialization (§3.8, Q14) | bytes keyed by `Route` |
| **Assets** | static list, Sass sources | copy, compile | bytes keyed by `Route` |
| **Validate and write** (`folia::output`) | everything above | link validation (D1), no output path built twice, every output planned, atomic write | the destination |

Generate follows Render so that search can read rendered text; feeds, the sitemap and
`redirects.json` need only assembled data and are unaffected by the order.

### 3.8 Generated data formats [DECIDED, Q14, 2026-11-01]

**Structured outputs are produced by Rust serializers, not MiniJinja templates:** Atom feeds,
the combined feed, `sitemap.xml`, `redirects.json`, and the search index (§3.5). Templates
remain responsible for HTML.

- **Interfaces.** Modes declare *what* (membership, order, searchability) through `ModeOutput`,
  `EntrySummary` and `FeedSpec`; each generator is a function from prepared data to bytes, with
  no template engine, unit-tested against fixtures and compared with the baseline by §5.3.
- **Consequences.** MiniJinja's `json`, `urlencode` and `loop_controls` features stay off unless
  an HTML template needs them; Liquid filters used only by feeds and search (`smartify`,
  `xml_escape`, `strip_html`, `normalize_whitespace`, `jsonify`) are Rust functions in the
  generator that needs them.
- **Schemas known from evidence:** Atom 1.0 as jekyll-feed 0.17.0 and
  `_layouts/feed.xml` write it (10 entries each); the search records (§3.5); the
  sitemap's `<url>` with `<loc>` and `<lastmod>` (222 URLs); `redirects.json` as an object from
  source URL to target (26 keys).
- **Schema questions, each settled in its C step from the baseline before code:**
  - **feeds (C2):** element set and order per feed; `<content>` vs `<summary>` (`/feed.xml` has
    summaries only); the entry id rule (§3.6); an Atom crate or hand-written serialization;
  - **sitemap (C3):** the inclusion rule (of 242 non-static HTML outputs, the 26 stubs and
    `/404.html` are absent; 7 static files are present: 6 PDFs and the Yandex page; the search JS
    sets `sitemap: false`); `lastmod` for static files, which Jekyll takes from file mtimes;
  - **`redirects.json` (C3):** key order. Jekyll writes keys in the order its generator meets
    documents, `site.docs_to_write` then `site.pages`
    (`jekyll-redirect-from-0.16.0/lib/jekyll-redirect-from/generator.rb:18`); the baseline starts
    with a micro entry's alias, then pages' aliases interleaved by source. Page read order
    depends on the checkout (v9 §2.6), so the comparison is of the parsed object, keys and values
    exactly, with order not compared; Folia's own order is deterministic and decided in C3;
  - **search (C4):** body text extraction; record order (the file layout is part of Q20, §3.5).

## 4. Shared contract hardening (A1)

### 4.1 Changes

Each item names the evidence of the problem and what "done" means. They are chosen because they
get more expensive with every widened mode; the assessment's smaller findings are in §9.2.

| # | Change | Why (evidence) | Done when |
|-|-|-|-|
| A1.1 | **A configured build set** replaces `only: &[&str]` in the mode `build` APIs and the public `SLICE_*` constants (below) | `crates/folia/src/build.rs:34-43,171-200`; `blog/src/post.rs:72,85`; `garden/src/note.rs:81,109`; parity can only see the slice (`parity/src/pages.rs:26,51`) | No mode signature mentions the slice; the slice's 4 documents build from configuration, output unchanged (§4.2) |
| A1.2 | **One mode build context** (clock, git commits, mode root, the build set) for every mode's `build`, and `routes` where they need it | Four different signatures; v9 §6.2's `ModeConfig` never built | All four modes take the context; micro gains `build` with it in B4 |
| A1.3 | **`EntrySummary` gains** title, excerpt HTML, categories, tags. `kind` and `searchable` are added with their first consumer (C2, C4), and feed-only fields in C2 (§3.6) | Titles exist only in untyped context JSON (`post.rs:127-143`, `note.rs:264-279`, `pages/src/lib.rs:160-176`) | The three constructors fill them; unit tests per mode |
| A1.4 | **Diagnostics travel with output**, on success and on failure: a mode returns its output and its diagnostics, each with a severity; the builder keeps warnings when another step fails. `parity pages` (`pages.rs:118`, `garden::check_warnings`) and `pages-negative.sh` change in the same step | Warnings are dropped in three places (`garden/src/note.rs:121-125`; `build.rs:196-216`; `build.rs:121-137` with `main.rs:64-73`) | A failing build prints its warnings; tested |
| A1.5 | **One clock per build**, carried by `Site`, and settable: `folia build --now <timestamp>` (pinned build time) | Built twice per build (`site.rs:55`, `build.rs:77`), a third time in parity; `main.rs:48` uses `Timestamp::now()` | One `SiteClock` per build; the CLI accepts a pinned time (A2.5 pins it through the library API meanwhile) |
| A1.6 | **A shared base page context**: the keys every page template reads (url, title, layout, dates, description, toc, cover, frame switches) and the frame data (§3.1) from one Rust builder, with mode keys added on top. `serde_json` sorts keys and strict undefined makes added keys inert, so the risk to output is a changed value (for example pages' `description: null`), which §4.2 catches | Three hand-written `json!` blocks; pages pads post-only keys with `null` (`pages/src/lib.rs:168-175`) | One builder; the padding is explicit and documented in one table |
| A1.7 | **Statics and stubs written from the planned routes**: statics from their planned `Route`s; **a redirect stub is written if and only if its target is built** | Statics planned as `Route::file` but written from raw ids (`site.rs:103` vs `build.rs:117-119`); stubs planned from `PlannedOutput` and written from `EntrySummary` (`site.rs:108-113` vs `build.rs:102-110`) | The writer's inputs are the planner's outputs; 1 stub today, still |
| A1.8 | **One date policy per mode, stated and enforced.** Micro **refuses** a front matter `date`, as the blog does. Rationale: Jekyll honours a front matter `date` on collection documents, but micro ignores it (`micro/src/lib.rs:73-77`), so accepting it would silently change output; no micro entry sets it, so refusing changes nothing today (v9 step 2, decision 4: refuse where accepting would silently change output). Mode roots normalized once (`discover.rs:154` trims slashes, `DocId::below` does not, `core/content/source.rs:58-60`) | Divergent rules surface when modes widen | Each mode's date rule is stated in core's docs and tested; micro's refusal tested; a configured root with a trailing slash works |

**The build set (A1.1), designed:**

- **Location:** `folia.toml` (today `parity/filopoe.toml`; later `_folia/folia.toml`, Q8), in an
  optional `[build]` table: `only = ["<path>", …]`, explicit source paths, no globs. Omitted
  means every document. The CLI may narrow it further for a run (`--only`), never widen it.
- **Semantics: whole context, partial pages.** Every document of every mode is read, routed and
  used as context (neighbours, backlinks, the graph, category counts, feeds' membership); only
  documents in the set, and outputs derived only from them, are rendered and written. Outputs not
  written are *not built yet* in the coverage report (§5.4).
- **Gates:** every document's front matter gate runs, whether or not it is in the set; a refusal
  outside the set is reported as a *gate* result, not a build error, in a report kept as
  evidence (A3); a refusal inside the set fails the build, as today.
- **Cutover:** the shipped configuration has no `[build]` table; F1 fails if one is present.

### 4.2 No output change, and approved changes

- **Reference build (A0):** before any A1 change, record a build of Folia `0302e08` with the
  `TEMPLATES-COMMIT-B` templates, `parity/filopoe.toml` as of `0302e08`, and `filopoe` at
  `BASELINE-B`. At `0302e08` the only way to keep a build is `folia build`, which builds as of the
  current time (`crates/folia/src/main.rs:48`; `parity pages` builds as of the baseline's time but
  deletes its output, `crates/parity/src/pages.rs:68`). The reference is therefore that `folia
  build`, run from an export of `0302e08` as step 3's evidence did
  (`slice-step3-remediation/README.md`), its build time recorded and its files stored with their
  hashes in A0's evidence. Its time differs from later pinned builds (baseline time
  `2026-10-25T14:43:51Z`), which changes nothing as long as both fall in the same calendar year
  (the footer's year) and no source is dated between them (the future-date holdback); both hold
  for `BASELINE-B`, whose latest document is dated 2026-10-24, if the reference is built in 2026.
  If A1's comparisons slip into another year, A0 is redone in that year and the comparisons pin a
  time in it.
- **A1's check:** each A1 change rebuilds with the same templates, configuration and pinned time
  and compares with the reference byte for byte (A2.5's recipe). A1 therefore depends on A2.5;
  until A2.5 exists, the comparison is the procedure above run by hand, recorded the same way.
- **Approved Folia-to-Folia changes:** an output change that A1 or A2 makes deliberately (for
  example a template fix A2's whole-document comparison reveals) is listed in the step's evidence
  as *Folia change*: the output, the difference, the reason, the author's approval. A new
  reference build is recorded after it. This list is separate from the Jekyll deviations ledger:
  a Folia change brings Folia closer to Jekyll or is neutral to it; a difference from Jekyll that
  remains is a ledger entry.
- Template fixes during phase A are allowed only as approved Folia changes.

## 5. Parity and validation strategy

### 5.1 HTML parity [DECIDED, Q15, 2026-11-01]

**Whole-document normalized DOM comparison, with an explicit deviations ledger and hard
assertions.** Fingerprints stay as diagnostics. The whole document is compared since checkpoint
A2.2 (`crates/parity/src/document.rs`, extending spike 1's normalizer
`crates/parity/src/markdown/dom.rs`), and decides `just pages` since checkpoint A2.3; the earlier
`div.main-content` comparison and the fingerprints still run, and fail `just pages`, as checks of
their own (v11 recorded only `div.main-content` as compared).

**Coverage:** the whole document: the doctype, `<html>` and its attributes, every head and body
element with its attributes, text and comments, in order.

**Normalization: these rules and nothing else** (rules 1 to 5 are the existing normalizer's; 6
and 7 are new):

1. **Parsing:** html5ever, as a browser parses; entities and character references are decoded,
   so `&#x2f;` and `/` are equal.
2. **Attributes:** compared as a map (order ignored); the tokens of `class` are sorted.
   Attribute values are otherwise compared exactly: no trimming or collapsing.
3. **Whitespace in text:** runs of HTML whitespace (space, tab, LF, CR, FF) collapse to one space
   everywhere except inside `pre` and `textarea`.
4. **Whitespace at block edges:** a text node's leading or trailing whitespace is removed where it
   meets the start or end of an element in this set, or a sibling element in this set, and a text
   node left empty is dropped (`dom.rs`'s `tidy`). The set is `dom.rs`'s `BLOCKS` list: `address`,
   `article`, `aside`, `blockquote`, `div`, `dl`, `dd`, `dt`, `figcaption`, `figure`, `footer`,
   `form`, `h1`–`h6`, `header`, `hr`, `li`, `link`, `main`, `nav`, `ol`, `p`, `pre`, `script`,
   `section`, `style`, `table`, `tbody`, `td`, `tfoot`, `th`, `thead`, `tr`, `ul`.
5. **`pre`** is compared by its text only; highlighting inside it is measured by the Rouge
   agreement (F8), not the DOM.
6. **Head-level whitespace:** the set in rule 4 gains `html`, `head`, `body`, `title`, `meta`
   and `base`, so whitespace between head elements, and at the edges of `title`'s text, is
   insignificant. The rest of `title`'s text is compared after rule 3.
7. **Doctype:** compared by name, public id and system id (the parser exposes it; today's
   normalizer drops it).

**`script`, `style` and `textarea`:** their text is compared after rule 3 (collapsed), except
`textarea`, compared exactly. A script or style whose collapsed text differs is a difference.

**Known difference, named:** Jekyll's head writes `<title>` and some description and Open Graph
`content` values with newlines and repeated spaces from Liquid captures (for example
`<title>\n\nAbout me\n\n | Filopoetica</title>` and `content="\nFilopoetica - \n\nAbout me\n\n\n"`,
baseline `site/about.html:13-19, 28-32, 38-42`), where Folia writes one line. `<title>` text is
equal after rules 3 and 6; the attribute values are not, since rule 2 compares them exactly. A2
records this as one ledger entry listing, per output, the index paths (below) of exactly the
attributes the comparison shows to differ (on `/about`: the description and `og:title` `content`),
each with the kind *attribute value equal after collapse and trim* (below); a value that differs
otherwise still fails. The entry is stale, and fails, unless each of its `match` items explains a
difference on each of its outputs, with the files reviewed (§5.2).

**Every difference is reported:** the comparison walks the whole document and lists every
difference per output, not the first, as built (spike 1's `dom::first_difference`, which stops at
one, now serves only the Markdown and `div.main-content` checks).
`dom-report.json` keeps both values of each difference in full; only the Markdown summary
shortens them for reading.

**A difference** is `(output, path, kind)`:

- `path` is the node's position from the document root, each step `tag[n]` (the n-th child
  element with that tag, 1-based), ending in `@attr` for an attribute or `#text[n]` /
  `#comment[n]` for a text or comment node; for example
  `html[1]/head[1]/meta[3]@content`;
- `kind` is one of: element missing, element extra, tag differs, attribute missing, attribute
  extra, attribute value differs, *attribute value equal after collapse and trim* (the values are
  equal once runs of HTML whitespace are collapsed to one space and the ends trimmed; usable only
  in a ledger `match`), text differs, comment differs, doctype differs. **HTML whitespace** here,
  and in the title and description assertions below, is rule 3's set only: space, tab, LF, CR
  and FF (U+0020, U+0009, U+000A, U+000D, U+000C); a no-break space or any other Unicode space
  is a character like any other (decided after the A2.3 audit, 2026-11-03).

**Settled comparator rules** (checkpoint A2.3, 2026-11-01, §5.4; `crates/parity/src/document.rs`):

- A matched node's path uses the expected (Jekyll) side's sibling indices; a node only the actual
  (Folia) side has uses that side's indices; `tag[n]` counts siblings with the same tag.
- A text or comment node on one side only is *text differs* or *comment differs*, its value on
  the other side empty (the empty string in the report, never `null`; a normalized text or
  comment node is never empty, so the two cannot be confused); a doctype on one side only is
  *doctype differs*.
- Children are aligned so that one missing node does not shift its siblings: the alignment
  matches as many children of the same sort (an element of the same tag, a text, a comment) as
  possible, and among those, as many equal pairs as possible; ties keep the earliest pairs.
  Between matched children, unmatched ones pair up in order, element with element (*tag
  differs*), text with text, comment with comment; the rest are missing or extra. The alignment
  is deterministic, but where several are equally good (two siblings replaced by one that equals
  neither, equal siblings of which one is removed) it names one of them, the earliest.

**Hard assertions** (Q15), each a failure of its own, reported even when the DOM comparison is
explained by the ledger, each with a negative mutation (A2.4):

| Assertion | Applies to |
|-|-|
| `meta[name=robots]` exists and its `content` equals Jekyll's, attribute names and values matched case-insensitively (the site writes `<META NAME="ROBOTS" CONTENT="NOINDEX, NOFOLLOW">`) | every HTML page Jekyll gives one |
| `link[rel=canonical]` equals Jekyll's and is absolute on the site's URL | every HTML page |
| `<title>` text equals Jekyll's after rules 3 and 6 | every HTML page |
| `meta[name=description]` `content` equals Jekyll's after HTML whitespace collapse and trim (the ledger kind's operation) | every HTML page with one |
| `html@lang` equals Jekyll's | every HTML page |
| every `rel="me"` link, and the root `h-card` (`u-url u-uid`, `p-name`), present and equal | `/` (`_layouts/profile.html:21-23`) |
| `h-entry` with `p-name`, `e-content`, `dt-published`, `u-url` where Jekyll has them | entry pages |
| a stub's target and canonical | redirect stubs (also byte-compared) |

**Fingerprints** stay as diagnostics, printed to explain a difference, never the acceptance
criterion. **Fragments** (`/changelog`, a bare fragment today) are compared the same way, without
the document-level assertions.

### 5.2 The deviations ledger

`parity/deviations.toml` holds D-001 to D-003 (Markdown entries, read by `parity markdown`) and
D-004 (`check = "dom"`, read by `parity pages`, §5.1); A2 extends its schema to every check and
makes every tool read it (done for `markdown` and `dom`; the other checks have no ledger yet).

- **Fields (machine-readable):** `id`; `status` (`proposed`, `accepted`, `withdrawn`); `check`
  (the check it applies to); `outputs` (explicit paths; a pattern only with a written reason and
  the count it must match); `match` (a list of `(path, kind)` from §5.1, or the check's own
  difference form for non-HTML checks); `change` (what differs); `rationale`; `accepted` (date);
  `accepted_by`; `decision` (a reference to this plan's decision log or a step review);
  `follow_up` and `follow_up_status` (`open`, `closed`, `none`).
- **Rules:** only `accepted` entries suppress a failure, and only for the differences their
  `match` names; a `proposed` entry reports the failure as pending and still fails; an entry that
  matches nothing fails (stale); a difference on an output an entry names, but outside its
  `match`, still fails; withdrawn ids are not reused.
- **Review:** an entry is accepted by the author, recorded in `accepted_by` and `decision`, never
  by the implementer alone. Each step's evidence README lists the entries it added or used. The
  ledger names the owners once, at its top (`owners`); a coverage entry is approved only when its
  `accepted_by` is one of them (decided 2026-11-03), and changing `owners` is itself an owner's
  decision. (DOM and Markdown entries are not checked against `owners` yet.)
- **Coverage entries** (`check = "coverage"`; work item A2.1, decided 2026-11-03): `class` is
  `waived` (written, comparison waived) or `removed`; `outputs` is a list of explicit paths, each
  a manifest row; `rationale` is required; an accepted one needs `accepted` (a date), `decision`
  and an owner's `accepted_by`. The descriptive fields are `title`, `change`, `withdrawn` and
  `resolution`; any other field is a problem. A proposed entry classifies nothing: its rows are
  pending, and fail; a withdrawn one is ignored. The tool checks the recorded approval, not who
  wrote it: review confirms that the owner gave each approval and each change to `owners`, which
  a test pins and every coverage report records (`docs/development.md`).
- **DOM entries** (`check = "dom"`; checkpoint A2.3, decided by the author 2026-11-01; both
  sides bound by the A2.3 hardening, 2026-11-02) are also bound to the builds reviewed, so an
  accepted entry cannot carry over to another baseline, a changed baseline file or a changed
  page: `expected_build`, the SHA-256 of the expected side's record (`BASELINE.json`), and, for
  each of `outputs`, both files reviewed: `expected_sha256`, the SHA-256 of the baseline's
  (Jekyll's) file, and `actual_sha256`, that of the Folia file. `BASELINE.json` alone does not
  identify the baseline's files (it records how they were built, not their contents), hence the
  per-file `expected_sha256`. Neither hash is ever taken from the ledger as true; on every run:
  - `expected_sha256` is compared with the SHA-256 of the baseline file's bytes (a baseline has
    no output manifest);
  - `actual_sha256` is compared with the SHA-256 that Folia's output manifest (the kept build's
    `sha256.txt`, §5.4) records for that output, the authoritative record of Folia's output;
    and every compared page's manifest hash is checked against the SHA-256 of its bytes: a page
    the manifest does not list, or lists with another hash, fails the run (decided after the
    A2.3 audit, 2026-11-03).

  Neither Folia's `build.json` nor the hash of the whole manifest serves: `build.json` records
  the Folia source, which holds the ledger, so it would change with every edit of the ledger, and
  either would change with any output, so a change to an unrelated output would invalidate the
  entry. Each output is bound by its own two files only. A difference is explained only when the
  record, the output and both its files match and a `match` item's path and kind do; the
  ledger-only kind matches an *attribute value differs* only when the two values are equal after
  HTML whitespace collapse and trim (§5.1). **An entry is valid only when** the record is the one
  reviewed, every output it lists was compared with both its files the ones reviewed, and every
  `match` item explains a difference on every output it lists. An accepted or proposed entry
  that is not valid is *stale*, and fails the run, naming the entry and each reason (the record,
  an output absent or changed on either side, a `match` item that explains nothing on an
  output, Q25); a difference an entry covers but cannot explain, because its record or files are
  not the ones reviewed, is stale for that entry and fails too. (As first written, v11 let an entry
  that explained part of what it lists pass a run as *partly stale*, refused only by F13; the
  decision of 2026-11-03, after the A2.3 audit, makes every stale entry a failure of the run.)
  An accepted entry needs `accepted`, `accepted_by` and `decision`; every DOM entry needs
  `follow_up_status`, and a `follow_up` when it is `open`, which does not fail a run but fails
  F13. A difference two entries explain, accepted or proposed, fails as an overlap. An entry
  that is not well formed fails, and so does a field the schema does not know (in the entry, an
  output or a `match` item) and an entry of any check without an `id` that is not blank.
- **Entries known to be needed** (written when their check exists): the feed-level `<updated>`
  (v9 §7.3, `proposed` in A2, accepted before C2); the named head whitespace difference (§5.1);
  each Q3 defect fixed rather than preserved; D-003's text updated to its narrowing. D-001's open
  follow-up (a visual check of its two posts) is carried by F13 and F14 until closed.

### 5.3 Generated output comparators

Each lands with its generator (C2–C5). **Thresholds are derived, not chosen:** for each metric,
build Jekyll twice from one checkout and once from a fresh clone of the same commit (as
`parity/evidence/fresh-clone-BASELINE-A/` and `rebaseline-*` did), measure the metric's natural
variation, and set the threshold so Folia must be no further from the reference than Jekyll is
from itself; the measurement and the resulting threshold are recorded as a decision in the
step's evidence.

| Output | Comparison | Known needs |
|-|-|-|
| Feeds | parsed Atom: feed metadata; entry set and order; per entry id, link, title, published, updated, categories, author, content or summary (normalized DOM, §5.1) | feed `<updated>` by v9 §7.3's rule (ledger); D-003 changes garden summaries |
| `sitemap.xml` | `<url>` set; `lastmod` per URL | order across checkouts is not a difference (v9 §2.6); static files' mtime `lastmod` (Q3g or ledger) |
| `redirects.json` | parsed object: keys and values exactly; key order not compared (§3.8) | |
| Redirect stubs | bytes | |
| Search index | `(url, title)` set exactly; record count; body similarity per record above the derived threshold | §3.5 |
| CSS | bytes, or, if Q7's compiler cannot match them, the parsed rule set (selectors, properties, values in order), with every difference ledgered | Q7 |
| Source maps | bytes, or, if the compiler cannot match them, a ledger entry per map stating what differs; the CSS's `sourceMappingURL` must still resolve | Q7 |
| Note graph | multisets (built) | |

### 5.4 Coverage, determinism, baselines, negative tests

Two numberings have used the labels A2.1 to A2.3. The items of this section (A2.1 to A2.10) are
**work items**, and the rest of this plan (§6, §8) refers to them. The A2 work has been delivered
in reviewed **checkpoints**, written *checkpoint A2.n* (and in evidence folder names and ledger
`decision` fields, `a2-n` and "A2.n step review"); a checkpoint number never names a work item:

| Checkpoint | Delivered | Evidence kept |
|-|-|-|
| A2.1, A2.2 | kept builds with their record (`crates/parity/src/builds.rs`); the report-only whole-document comparison (`parity dom`, §5.1) | none: no evidence folder; D-004 was first reviewed from checkpoint A2.2's report, which is not kept |
| A2.3 | the `dom` ledger (§5.2) and the hard assertions (§5.1) deciding `just pages`, with their negatives: parts of work items A2.3 and A2.4 | `parity/evidence/a2-3-dom-ledger/` |
| A2.3 hardening | both sides of every reviewed output hash-bound (`expected_sha256`), strict ledger schema and ids, partial staleness reported (superseded on 2026-11-03: every stale entry fails), overlaps with proposed entries, `changes` pinned and failure explicit, assertion reads and h-entry scoping, more end-to-end negatives | `parity/evidence/a2-3-hardening/` |
| A2.3 audit corrections (2026-11-03) | the owner's decisions after the A2.3 audit: every stale entry fails the run; Folia's side of a DOM entry bound by the hash its output manifest records, checked against the page's bytes; HTML whitespace only for collapse and trim, shared by the ledger kind and the title and description assertions; values in full in `dom-report.json`, `""` for a one-sided text or comment; untracked names read byte for byte; an untracked nested repository unknown; D-004 re-accepted, scope unchanged; `just ledger-negative` | `parity/evidence/a2-3-audit-fixes/` (built from an uncommitted tree, before Q26's convention; kept as history, its README says so) |
| A2.3 closure (2026-11-03) | the source identity policy below; end-to-end negatives for the title and description assertions and for an unknown identity; then a source that is not a git checkout made unknown, and Q25 closed | `parity/evidence/a2-3-closure/` (built from an uncommitted tree, before Q26's convention; kept as history, its README says so; to be rebuilt by it once the source is committed) |

**Checkpoint A2.3 is complete** with its hardening, audit corrections and closure. Its evidence
predates the provenance convention below (Q26): its checks are to be rerun by that convention
once its source is committed, which concerns how the evidence is tied to its source, not what it
checks.
Work item A2.4 is complete for the outputs built (the `rel="me"` and h-card negatives wait for
`/`, below); work item A2.3 is not (`markdown-negative.sh`'s per-gate messages remain). When
observed on 2026-11-03 (§1.1), before the implementation commits, the checkpoints of that day
were local and uncommitted, on `f671c72`, awaiting review.

**Source identity [DECIDED, 2026-11-03]:** a build's record identifies the site and the Folia source
it was built from by commit and `changes` (`crates/parity/src/builds.rs`). **An identity that is
`unknown`, of either, fails `parity pages`**: a parity result cannot be trusted when its complete
source identity cannot be established. The build is still kept with its record, log and reports, and
every other check still runs, so the diagnostics stay available. `changes` is `unknown`, naming the
cause, when the directory has no commit (it is not a git checkout, or has no commit yet; `commit` is
then `null`, and `changes` is never left out), when git fails, or when the tree holds what cannot be
identified: an untracked file or link that cannot be read (never a placeholder hash); an untracked
nested repository; or any submodule entry (a gitlink in the index or `HEAD`, whatever its state: the
diffs give a submodule only as a commit and a `-dirty` marker, which cannot tell its states apart).
Untracked names are read byte for byte on Unix, so a name that is not UTF-8 names its file; where a
path must be UTF-8, such a name makes the identity unknown. Ignored files are not part of the
identity, as before. A `.gitmodules` file that names no gitlink is a file like any other (`filopoe`
has one).

**A2's status (2026-11-03).** The A2 step (§6) is **not complete**; its checkpoints so far
deliver parts of work items A2.3 and A2.4 and the §5.1 and §5.2 machinery.

- **Done** (checkpoints A2.1 to A2.3 and their corrections; checkpoint A2.3 complete): kept builds
  with their record and source identity, unknown failing the run; the whole-document comparison
  (§5.1); the `dom` ledger with per-output binding and stale entries failing (§5.2); the hard
  assertions on the 4 pages and the stub; ledger negatives (a proposed entry, a stale entry, a
  second difference); an end-to-end negative for each assertion that applies to a page built
  (robots, canonical, title, description, `lang`, h-entry, stub).
- **Built, awaiting review (2026-11-03; local and uncommitted when observed, §1.1):** A2.1, the
  coverage classes (below: `crates/parity/src/coverage.rs`, `coverage.json` and `coverage.md` in
  each kept build, `just coverage-negative`); on `BASELINE-B`, 781 rows built and checked and 296 not
  built yet, none waived or removed. Its evidence is made by Q26's convention once it is committed.
- **Remaining:** A2.3's per-gate messages for `markdown-negative.sh`; A2.5 (`just determinism`);
  A2.6 (baseline verification, §1.2); A2.7 (toolchain comparison); A2.8 (route-level links); A2.9
  (expected warnings); A2.10 (`just accept`); route parity's *claimed* class and `missing_templates`
  (§3.4); F8's enforced Rouge floor and its negative; F14's two viewports; v11's corrections
  scheduled "in A2" (§1.2: the step 3 README, D-003's text, `docs/baseline.md`); the step's
  acceptance line (§6), including the coverage report.
- **Deferred, as planned:** the feed-level `<updated>` entry (written when its check exists,
  accepted before C2, §5.2); end-to-end negatives for the `rel="me"` and h-card assertions, which
  apply only to `/` (built in B2; unit tests cover them meanwhile). Nothing is waived.
- **Owner decisions due in A2, open:** Q3a, Q3l, Q20 (model and search file), Q22 and Q23
  (§10.2). Q25 and Q26 are closed (§10.1). A3 also depends on A1.1 and A1.6 (§6), and A1 has
  not started.

The work items, as v11 defined them, unchanged:

- **A2.1 Coverage report.** `just pages` classifies every manifest row as **built and checked**,
  **not built yet**, **written, comparison waived** (Folia writes it; an accepted ledger entry
  waives its comparison), or **removed** (Folia does not write it; an accepted ledger entry says
  so). Waived and removed rows are listed by explicit path. The report lists every row's class in
  the evidence folder and prints the counts on every run. *As built (2026-11-03):* each row gets
  exactly one class, or none, which fails the run with the reason. Every output Folia writes, static
  files included, gets a route check (planned, at Jekyll's URL and output file), which compares no
  contents. *Built and checked* needs Folia to write the output, at least one check other than the
  route check to compare it, and every check that compares it to pass (page checks and the
  whole-document comparison, assertions, bytes, note graph); *not built yet* needs Folia's route
  table to plan it; *waived* needs an approved coverage entry (§5.2), the output written, its bytes
  recorded by Folia's output manifest and its route check passing, and then no other check compares
  it; *removed* needs an approved entry and the output neither written nor planned, so that a
  removal cannot stand for an output not built yet. Nothing is waived or removed by default: a row
  Folia neither writes nor plans, one it writes but no check compares, or one whose check fails, is
  unclassified. Also failing: two entries claiming a row, a proposed or unapproved entry, an unknown
  class, an entry output that is no manifest row, a manifest row listed twice, an output Folia
  writes that is no manifest row, and Folia's own not-built count disagreeing. `coverage.json`
  (every row: output, URL, kind, class, the approval or the checks' verdicts, any problem) and
  `coverage.md` are written into the kept build, deterministically.
- **A2.2 Unplanned outputs** fail, as the builder already enforces (`build.rs:131-138`).
- **A2.3 Negative tests.** Every new check gets a mutation in `scripts/pages-negative.sh` (or its
  successor) that must fail with a named message; `markdown-negative.sh` gains per-gate messages,
  as `pages-negative.sh` has. The ledger has its own negatives: a `proposed` entry still fails; a
  stale entry fails; a second difference on an output after an accepted one fails.
- **A2.4 Hard assertions** (§5.1) each get a negative mutation.
- **A2.5 Determinism.** `just determinism` builds twice into fresh directories with a pinned build
  time and compares them recursively, byte for byte. It pins the time through Folia's library API,
  which already takes it (`folia::build::build(.., now)`, `crates/folia/src/build.rs:69-74`), for
  example as a `parity` subcommand that keeps its output; it does not wait for A1.5's CLI flag, so
  A1 can depend on A2.5 without a cycle. It also compares a build with a recorded reference (§4.2).
  At acceptance it also builds from two fresh clones (F11).
- **A2.6 Baseline verification** uses the parity tools' exclusions (`_folia/`,
  `.github/workflows/content.yml`, `routes.rs:315`), from one shared list; `just baseline-verify`
  takes a new label (or verifies into a temporary label), so the documented command works.
- **A2.7 Toolchain comparison (Q17):** build one commit's baseline with Ruby 3.3.8 and with
  production's Ruby 3.4.5 and its resolved gems, and compare them with `compare-builds.sh`;
  record whether they differ and where.
- **A2.8 Route-level link validation (begins D1):** every internal `href` and `src` in the HTML
  Folia writes resolves to a planned route, a static file or a redirect source. It needs only the
  complete route table, so it runs from A2 on, and each B and C step extends it to what it builds.
- **A2.9 Expected warnings:** a versioned file (proposed: `parity/expected-warnings.toml`), one
  entry per expected warning: matcher (exact text or pattern), source, decision reference. A
  warning not in the file fails; an entry that matches no warning fails. Today's one entry is the
  broken wiki-link in `coding/rust.md`.
- **A2.10 Acceptance command skeleton:** `just accept <baseline>` runs every check in order,
  writes one evidence folder, and records the commits, configuration, templates, build time,
  toolchain and lockfile used (F12). It grows with each step.
- **Evidence convention:** each step stores its reports under `parity/evidence/<step>/`, never
  overwriting an earlier step's, with a README naming commands, commits, Folia changes (§4.2) and
  ledger entries used.
- **Evidence provenance [DECIDED, Q26, 2026-11-03]**, for every evidence folder from A2 on (A0's
  and earlier folders keep their format):
  - **Built from a committed, clean Folia revision.** Every build the evidence reports on records
    the Folia source with a `commit` and `changes` `clean`, all the same commit *C*. A build of
    uncommitted work is a development run, never evidence.
  - **Records kept, machine-readable.** For each build, the folder keeps
    `builds/<name>/build.json` and `builds/<name>/sha256.txt`, copied verbatim from the kept
    build; a hash of a record kept elsewhere (`dom-report.json`'s `actual.record.sha256`) is not
    enough. The README names *C* and each build's identity fields.
  - **Committed on its own, as *C*'s child.** The commit that adds the folder changes nothing
    outside it and has *C* as its only parent. Why separately: a build's record identifies the
    Folia tree it ran from, and that tree would contain the evidence if the evidence were
    written into it. An untracked, not ignored folder is part of `changes`, so writing the
    evidence changes the identity it records (the A2.3 closure's builds name `f671c72` and a
    tree that existed only until its evidence was written), and a commit holding both the
    source and its evidence cannot be the commit its builds name. Built from *C*, kept outside
    the repository until the builds are done, then committed as *C*'s child, the evidence names
    its parent, never itself, and `git diff C <evidence commit>` shows that only the evidence was
    added. READMEs may be corrected in later commits; reports and records never are.
  - **One combined folder per source commit.** Everything evidenced from *C* goes in one
    folder, committed as *C*'s only child. A second folder from the same *C* would have the
    first evidence commit as its parent, and is refused; evidence of another checkpoint is
    either in that one folder or built from another source commit.
  - **Checked:** `just evidence-provenance <dir>` (`scripts/evidence-provenance.sh`, read only)
    checks the records (each `builds/<name>/` with both files; one clean Folia commit; a known
    site identity; a well-formed manifest with as many files as the record counts) and the
    commit: before it is made, HEAD is *C* and nothing outside the folder is changed or
    untracked; after, the first commit touching the folder has *C* as its only parent (a merge
    is refused) and changes only the folder, and later commits and the working tree change in
    it only its README. `just evidence-provenance-negative` runs it on throwaway repositories,
    one case per way of breaking the convention.
  - **Identity fields** of a build's record (`build.json`), portable, compared across machines:
    `folia.commit` and `folia.changes`; `site.commit` and `site.changes` (`state`, `sha256`);
    `config.sha256`; `templates.tree_sha256` and `templates.files`; `at` (the pinned build time);
    `rustc` (the toolchain; the lockfile is part of *C*); and the output, by `files` and
    `sha256.txt`. The baseline is identified as before (`BASELINE.json`'s hash, and each
    file's).
  - **Diagnostic, not identity:** every `path` (`site`, `folia`, `config`, `templates`: absolute,
    or relative as given, so they vary with where and how a run was started); the `status` lists
    (derived from a commit and its changes); `result`'s text; and therefore the SHA-256 of
    `build.json` as a whole, which `dom-report.json` cites as `actual.record`: it changes with
    the paths, so two identical builds made from different directories have different record
    hashes (seen 2026-11-03), and it is never compared as identity.

### 5.5 Front matter inventory (A3)

Implements v9 step 2's decisions 4 and 5 as migration requirements.

- A parity check lists every front matter key and sub-key of the 202 sources by mode, using the
  all-document gate report (§4.1), and classifies each as **supported** (read and rendered),
  **accepted no-op** (no effect in Jekyll, such as `trip`, or the default value), **conditional**
  (meaningful only on some pages, judged by the mode), **claimed-source data** (read by an owner
  from a claimed source, §3.4), **unsupported** (changes the output; refused until built), or
  **pending decision** (Q3h). An unclassified key fails.
- Over-refusal is removed where Jekyll has no effect (§2.5); refusal stays where accepting would
  silently change output.
- A refusal reports every bad key of a file, in every mode (`blog/src/post.rs:165-168`,
  `garden/src/note.rs:249`, `pages/src/lib.rs:196-215`).
- **Zero-use features become checks** (F9): each feature Folia does not implement and the site
  does not use today fails the inventory, or `parity liquid`, when it first appears: kramdown
  footnotes, abbreviations, definition lists, `$$` math, ALDs, `{:toc}`, `markdown="1"`, explicit
  `{#id}` headings; front matter dates with seconds; `{{` or `{%` in front matter values; NBSP or
  a final capital sigma in a title or wiki-link target; a slug ending in `-`.
- A claimed source's `permalink` must equal its owner's route, and `pages/rss.xml`'s
  `collections` must agree with `folia.toml`'s combined feed (§3.6), until cutover.

## 6. Remaining feature roadmap

Every step: `just check`, `just routes`, `just markdown`, `just pages`, the negative runs and (from
A2.5 on) `just determinism` pass; the step's evidence is stored per §5.4; the step is reviewed
before the next starts. Types: **arch**itecture, **impl**ementation, **val**idation, **clean**up.

### Phase A: contracts and parity

| Step | Type | Scope | Depends on | Acceptance |
|-|-|-|-|-|
| **A0** | val | the reference build (§4.2); Q19(1) (land `0302e08` on `main`, with the docs navigation updated) if granted, else every change is diffed against `0302e08`; Q19(5) (refresh or remove `SITE-WORKSPACE`) if granted, else it stays unused | none | the reference build recorded |
| **A2** | val | §5.1–§5.4: DOM contract, ledger, comparators' framework, coverage, A2.5 determinism, A2.6 baseline verification, A2.7 toolchain comparison, A2.8 route-level links, A2.9 expected warnings, A2.10 the acceptance skeleton; route parity's *claimed* class and `missing_templates` (§3.4); F8's enforced Rouge floor with its negative; F14's two viewports fixed; Q20's model, Q3a, Q3l, Q22 and Q23 decided | A0 | the 4 built pages pass §5.1, each remaining difference an accepted entry or an approved Folia change; every new check and ledger rule has a failing negative; the coverage report shows 296 not built |
| **A1** | arch | §4.1 | A0, A2.5 (or §4.2's manual procedure) | output for `BASELINE-B` unchanged against the reference, except approved Folia changes |
| **A3** | val | §5.5, using the all-document gate report | A1.1, A1.6 | 0 unclassified keys across 202 sources; over-refusals removed with tests; zero-use checks with negatives |

Status (2026-11-03): A0 done (`parity/evidence/a0-reference/`); A2 in progress, its checkpoints
A2.1 to A2.3 delivered (A2.3 complete), work item A2.1 (coverage) built and awaiting review, and
the rest remaining (§5.4); A1 and A3 not started.

A2 may start before A1 and run alongside it; A1's acceptance needs A2.5. A2 precedes B because
widening multiplies the frame over about 240 HTML outputs that fingerprints check only loosely.

### Phase B: content coverage

| Step | Type | Scope | Depends on | Acceptance | Main risks |
|-|-|-|-|-|-|
| **B1 Garden** | impl | all 54 notes; ordering, backlinks, graph participation, future-date behaviour, metadata | A | 54 notes pass §5.1; graph unchanged; dates of all 54 equal the dump | none known: all 54 pass today's gate |
| **B2 Pages and `/`** | impl | the 18 Markdown pages (cover after title, `title_icon`, `corner_backlink`, `hide_title`; `/changelog` per Q3i); `/404.html` (HTML-page support for its source; the error layout and its script-free branches, `_includes/scripts.html:1`); `/` (`profile/index.html`, layout `author`, author data per Q16b). Not in B2: the claimed sources (C1, C2, C4) and the `.scss` (C5) | A; Q16b; Q3i | every source in B2's scope passes §5.1, or is excluded by an accepted entry; their stubs byte-identical; `/`'s assertions pass | the frame's untested branches; `author.html`'s 20 author buttons |
| **B3 Blog posts** | impl | all 79 posts: `narrow_post`, the featured image after the title, `footer.extra`, cover credits, `cover: top` (Q3j), title filters (Q18), interactions (Q16a) | A; Q16a, Q18, Q3j | 79 posts pass §5.1; dates and TOCs equal; F8 holds | Jekyll quirks preserved unless Q3 decides otherwise |
| **B4 Micro** | impl | a micro `build`; bookmark, like and rsvp pages, whose layouts nest in `post` (`_layouts/{bookmark,like,rsvp}.html`), and note pages (`layout: page`); `event` fields as written; the micro defaults (`_config.yml:287-336`: hidden frame, `hide_post_junk`, no pagination row, the corner backlink); 1 redirect | A; B3 (whose interactions and post layout the nested micro layouts reuse); Q16a | 39 entries pass §5.1; ordering and dates tested (routes and the future-date holdback are already tested, `micro/src/lib.rs` tests) | the nested layouts; template naming (`micro/{kind}.html`, v9 §6.4) |

B1 goes first because every note passes today's gate, so it is the cheapest proof that phase A
works. B2 and B3 are independent once A is done and may swap.

### Phase C: generated and aggregate outputs

| Step | Type | Scope | Depends on | Acceptance |
|-|-|-|-|-|
| **C1 Listings** | impl | `/blog/` and pages 2–9, 10 category archives, categories and tags pages (blog); `/micro`, `/bookmarks`, `/likes` (micro); their claimed sources' redirects and frame switches (§3.4) | B1–B4; Q3k | all pass §5.1; their 6 stubs byte-identical; tie order equals Jekyll's (newest first, ties by path descending, observed in every listing checked) |
| **C2 Feeds** | impl + val | 51 feeds and `/feed.xml` (§3.6), Rust serializers; `ModeOutput.feeds`; the feed fields; `smartify` on titles; the feed quirks per Q3b | A1.3 and the modes' `EntrySummary`s for all documents (the whole-context build, §4.1), and B4 for micro's (its 5 feeds and its combined-feed entries); the feed `<updated>` entry accepted | the feed comparator passes for all 52 at the derived thresholds |
| **C3 Sitemap, redirects** | impl | `sitemap.xml`, `redirects.json`, Rust; §3.8's schema questions | A1.3 and the complete route table (both exist after A1); `modified` for all entries, micro's from B4 | §5.3's comparisons pass |
| **C4 Search** | impl + val | the search index (§3.5); Q3c–Q3f, Q3q | B, C1 (rendered pages), C3 (`/redirects.json`'s record) | `(url, title)` set and count exact; body similarity above the derived threshold |
| **C5 Sass** | impl | `main.scss`, `dg_styles.scss`, their maps (Q7) | none; may run in any phase | §5.3's CSS and source-map comparisons |

**C2 and C3 may start after A1:** feeds need entries' data, not their rendered pages, and the
sitemap needs routes and dates, all of which the whole-context build provides, except for micro,
which has no `build` until B4; their content
checks then cover every entry even while B renders a subset. They are listed after B because
feed `<content>` compares body HTML, which B validates per mode; starting them early is allowed,
and their acceptance is not final until B is done.

### Phase D: integration and cutover (§7)

D1 completes link validation (anchors and generated outputs), D2 is migration cleanup, D3 cutover
preparation.

## 7. Migration cleanup and cutover preparation

Roadmap items. None is done before its authorization (Q19), and none publishes anything before
the final approval.

- **D1 Internal link validation:** completes A2.8: anchors exist on their target page; links in
  generated outputs (feeds, sitemap) resolve. Failures name the page, the link and the target.
  Known Jekyll-origin broken links are ledgered or fixed per Q3k and Q3l.
- **D2 Migration cleanup:**
  - merge SITE-PR (Liquid removal and its lint) into `filopoe`'s `main`, by the method Q21
    decides, timing per Q19;
  - integrate the templates (`folia/templates`, re-applied onto the cutover base, since it is
    based on `SITE-MAIN`): the integrated `_folia/` must equal the validated tree file for file;
  - move the configuration to `filopoe/_folia/folia.toml` (Q8), with the content lint's workflow
    paths updated (Q13);
  - finalize the ledger: every entry accepted or withdrawn, every follow-up closed;
  - **rebaseline after the final history exists** (Q17, Q21), with the production toolchain and
    its lockfile stored in the evidence;
  - record Q20's deletion plan, carried out in D3 after the rollback window.
- **D3 Cutover preparation** (every workflow change and deploy here needs its own Q19
  authorization): `just accept` completed (F12); a Folia build and acceptance workflow in
  `filopoe`'s CI; the staged comparison with the production workflow's own Jekyll
  artifact (F15); the deploy procedure and domain settings (Q6); the rollback procedure with the
  last Jekyll Pages artifact retained (Q6); updated `docs/baseline.md` and operational
  documentation.

## 8. Final acceptance criteria

Run by `just accept` (F12) against a baseline built from the final history (Q17, Q21).

| # | Criterion | Measure | Built in | Enforced |
|-|-|-|-|-|
| F1 | Every output accounted for | coverage report (A2.1): every manifest row *built and checked*, *written, comparison waived* or *removed*, the last two by explicit path with an accepted entry each; *not built yet* = 0; every row written unless classified *removed*; no `[build]` table in the shipped configuration (§4.1) | A2.1 | automatic |
| F2 | No unplanned outputs | builder check, and parity's route check | exists | automatic |
| F3 | Routes | exact multiset equality of (source, URL, output) for authored and claimed outputs, of (URL, output) for generated ones, and of the stubs; source classification equal, with *claimed by `<owner>`* accepted only for the claim tables' paths (§3.4); every *removed* row planned as removed | exists; claimed class A2 | automatic |
| F4 | Byte-exact outputs | static files, redirect stubs, the note graph; CSS and source maps per §5.3 (bytes, else ledgered differences) | exists; C5 | automatic |
| F5 | HTML | the §5.1 contract (normalization rules 1–7, every difference reported by path and kind) passes for every HTML output; every remaining difference matched by an accepted entry's `match` | A2 | automatic; entries reviewed manually |
| F6 | Document assertions | §5.1's hard assertions, including `<title>` and meta description | A2.4 | automatic |
| F7 | Generated outputs | feeds, sitemap, `redirects.json`, search, CSS and source maps pass §5.3 at the derived thresholds | C2–C5 | automatic |
| F8 | Markdown and content | G1–G6 pass with accepted entries only; Rouge class agreement ≥ 90% site-wide (Q12), enforced, with a negative mutation that lowers it; dates of every entry equal the dump | exists; floor A2 | automatic |
| F9 | Front matter and unsupported features | 0 unclassified keys; every zero-use check (§5.5) passes | A3 | automatic |
| F10 | Warnings | every warning matches `parity/expected-warnings.toml`, and every entry matches a warning (A2.9) | A2.9 | automatic |
| F11 | Determinism | builds with a pinned time are byte-identical: twice in one tree, and from two fresh clones of the final commits | A2.5 | automatic |
| F12 | Reproducibility | `just accept` (built from A2.10, completed in D3) runs F1–F11, F13 and F15 and writes one evidence folder with the commits, configuration, templates, build time, toolchain and the lockfile itself; its inputs are commits pushed under Q19 (a push, not a publish or deploy) and a baseline whose toolchain is recorded | A2.10, D3 | automatic run; manual check that the inputs are pushed |
| F13 | Ledger | no `proposed` entries; every accepted entry has `accepted_by` and `decision`; every DOM entry has `expected_build` and each output's `expected_sha256` and `actual_sha256`, and matches the final builds: every listed output's two files are the ones reviewed and every `match` item explains a difference on every listed output (no entry stale: a stale entry fails every run, §5.2; `parity pages` lists what F13 would refuse); no `follow_up_status = open` | A2 | automatic |
| F14 | Visual review | a sample selected by rule: every layout Folia renders, at the two viewports v9 §9 named (one narrow, one wide, fixed in A2); every page an accepted entry changes visibly (D-001's two posts, D-003's cards); one page per frame variant (dark layout, micro's hidden frame, `narrow_post`); screenshots or notes in the evidence folder; the author's sign-off | D3 | manual |
| F15 | Production comparison | the F1–F11 suite passes against a baseline built from the production workflow's own Jekyll artifact for the cutover commit, with any difference between that artifact and the local baseline explained. `jekyll.yml` deploys whatever it builds, so the artifact comes from a build-only run (a workflow change made under its own authorization, Q19) or from a Jekyll deploy that is authorized anyway; never from an unauthorized deploy | D3 | automatic |
| F16 | Operations and approval | deploy, rollback and documentation (D3) reviewed; the author's explicit approval to publish | D3 | manual |

Byte parity applies where F4 says; elsewhere parity is semantic, as §5 defines it. A difference a
criterion allows is not a defect. F16 was F15's second half in v10; split so that F15 is
automatic.

## 9. Risks, dependencies and deferred work

### 9.1 Risks

| Risk | Kind | Evidence | Mitigation |
|-|-|-|-|
| Frame drift unseen | closed for the 4 pages built (checkpoint A2.3); a gap for every output not built yet | v11: `pages.rs:186-197`; the step 3 tagline whitespace was caught only by `diff -r` | A2's whole-document comparison (§5.1); each B and C step brings its outputs under it |
| Generated outputs unchecked | current gap | no comparators in `crates/parity` | §5.3 with each C step |
| Liquid on `filopoe`'s `main` | known blocker | 15 Markdown files on `SITE-MAIN` | D2 (SITE-PR) |
| **Git-derived dates move with history** | operational | dates follow the *committer* time: `BASELINE-A` was authored 10:35:05 and committed 10:35:27, and `rebaseline-BASELINE-A/README.md` shows dates moved to 10:35:27 | A squash or rebase merge of SITE-PR, or re-signing commits when publishing, creates new committer times and hashes. That moves every affected document's `last_modified_at` (the "Last updated" text, feed `<updated>`, sitemap `<lastmod>`) in Jekyll's own production output, and invalidates the baseline's checkout check. Q21 recommends fast-forward; D2 rebaselines after the final history exists. |
| Title filters and `smartify` missing | latent defect | `_includes/head.html:1-3`, `sections/post.html:39`; jekyll-feed `feed.xml:23,57` | B3 (Q18), C2 |
| kramdown features not implemented | theoretical | 0 uses in 190 documents | §5.5's zero-use checks (F9) |
| 55 bodies equal only after normalization | open | at most 19 are explained by highlighting | Q23 |
| Tie order in listings and the combined feed | theoretical | Jekyll: newest first, ties by path descending; the combined feed sorts a concatenation with an unstable sort | C1, C2 comparators check order |
| Acceptance baseline vs production toolchain | open | production uses Ruby 3.4.5 and no lockfile (`filopoe/.github/workflows/jekyll.yml:39`; `Gemfile.lock` ignored, `.gitignore:198`); `jekyll-last-modified-at` tracks a git branch | A2.7, Q17, F15 |
| Stale validation worktree | operational | `SITE-WORKSPACE` holds step 2's templates (§1.1) | not used for validation; Q19 |
| Docs deploy on push | operational | Folia's `docs.yml` deploys on push to `main`; `main`'s nav (`5e9c57a`) still says v9 is current, while `f671c72` names v11 and the local tree v12 | Q19: nav updated before any push |

### 9.2 Deferred technical work [LATER]

Each has a revisit milestone or is a failing check.

| Item | Evidence | Revisit |
|-|-|-|
| Dates with seconds: SafeYAML reads `YYYY-MM-DD HH:MM:SS` as UTC; Folia reads the site's zone | `core/content/date.rs:88-102`; 0 uses | failing check now (§5.5); decide refuse or match Jekyll before D2 |
| Impossible file-name dates: Folia refuses them, Jekyll rolls them over | 0 uses | Folia already fails loudly; keep, revisit only if a use appears |
| Unicode key normalization (NBSP, final sigma) | `garden/src/wiki_links.rs:44-46`; 0 uses | failing check (§5.5) |
| Liquid in front matter titles (and so in the graph) | 0 uses | failing check (§5.5) |
| `titleize` and a trailing `-` | `blog/src/lib.rs:165-178`; 0 uses | failing check (§5.5) |
| Single-pass git dates (later risk 6) | `core/content/git.rs`, per-file `git log` | B3, when every post's dates are read; keep `git log -- path` semantics |
| TOC analysed without wiki-link resolution | `presentation.rs:139` | B1, if a note sets `toc`; today notes refuse it |
| Small duplications (`file_route` ×3, path-below-root ×2, the collect-every-problem loop, O(n²) lookups ×3, two category anchor rules) | `blog/src/post.rs:204`, `folia/src/build.rs:261`, … | whichever A or B step touches each |
| Mode independence enforced by review only | `scripts/slice-boundary.sh` | A1: a cargo-metadata check replaces the per-step script |
| `Cover.position` is a string | `presentation.rs:79`, `blog/src/post.rs:175` | A3 |
| v9's [LATER]: CLI crate split, navigation model, renaming `hide_website_junk`, watch and serve, incremental builds, a Lunr replacement, visual regression tooling | v9 §4, §7.1, §12.2, §14 | after cutover (D3 records them as post-cutover work) |

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
| Q3o | v9 §2.5's "category count via `site.tags[category]`" is not a live defect: that branch (`_includes/banner.html:13-19`) runs only when `site.categories` is not a hash, which it always is; the live branch (`:20-26`) counts each category's posts, and the counts equal the baseline's on every built page (`slice-step3-remediation/pages.txt`, body text). Closed; nothing to preserve or fix | 2026-11-01 (v11) |
| **Q4** | **Listings are built in Rust;** modes own domain data, the builder orchestrates aggregation and output, templates render prepared contexts and never assemble collections; `/blog/` and its pages belong to the blog mode (§3.3). Revises v9 §6.3. | 2026-11-01 |
| Q5 | Templates, content and assets stay in `filopoe`, Folia's in `_folia/` | 2026-10-25 |
| Q9 | Spike tooling: a stdlib-only Python codemod (`parity/codemods/deliquify.py`) and the Jekyll dump plugin loaded with `--plugins` (`parity/jekyll/folia_dump.rb`); closed, both built and used | 2026-10-25 |
| Q10 | D-002 withdrawn: the source was fixed | 2026-10-25 |
| Q11 | D-003, narrowed on 2026-11-01 to listings, cards and feeds | 2026-10-25 |
| Q12 | syntect with bat's grammars, Pygments classes, the 90% rule (enforced by F8) | 2026-10-25 |
| Q13 | Content lint `parity liquid` (open parts: below) | 2026-10-26 |
| **Q14** | **Structured outputs by Rust serializers:** feeds and the combined feed, `sitemap.xml`, `redirects.json`, the search index; templates for HTML only (§3.8) | 2026-11-01 |
| **Q15** | **HTML parity is a whole-document normalized DOM comparison** with an explicit ledger and hard assertions (robots, canonical, title, description, `lang`, `rel=me`, h-card, h-entry, stubs); fingerprints are diagnostics (§5.1) | 2026-11-01 |
| A1.8 | Micro refuses a front matter `date` (§4.1): a deliberate compatibility boundary, not a feature decision. Evidence: no micro entry sets `date` (`git grep '^date:' SITE-COMMIT-B -- _micro`: none); Jekyll reads it on every collection document (`Document#read` runs `read_post_data`, whose `modify_date` keeps a front matter date over the file name's, `jekyll-4.4.1/lib/jekyll/document.rb:299-308, 491-536`) and the micro listings sort and show `item.date` (`blog/microblog.html:16,54-55`); Folia's micro reads only the file name's date (`crates/micro/src/lib.rs:73-77`). Accepting the key would silently change output, which v9 step 2's decision 4 forbids; refusing changes nothing today. Supporting it instead is B4 work if a micro entry ever needs it. Recorded in v11 for the author's review | 2026-11-01 (v11) |
| — | **A stale DOM entry fails every run** (no *partly stale*): an entry is valid only when its record is the one reviewed, every output it lists was compared with both its files the ones reviewed, and every `match` item explains a difference on every listed output (§5.2, F13). Owner's decision after the A2.3 audit | 2026-11-03 |
| — | **A DOM entry's Folia side is bound by Folia's output manifest:** `actual_sha256` against the hash `sha256.txt` records, that hash against the page's bytes (a mismatch or a missing page fails); `expected_sha256` against the baseline file's bytes; neither `build.json` nor the whole manifest's hash binds an entry (§5.2) | 2026-11-03 |
| — | **HTML whitespace** is space, tab, LF, CR and FF only, for the ledger's collapse-and-trim kind and the title and description assertions alike (§5.1) | 2026-11-03 |
| — | **Reports:** `dom-report.json` keeps values in full, the Markdown shortens them; a one-sided text or comment is `""` (§5.1) | 2026-11-03 |
| — | **D-004 re-accepted** with its hardened per-output binding; scope unchanged (4 outputs, 2 paths, 1 kind) | 2026-11-03 |
| — | **An unknown source identity fails `parity pages`**: an unreadable untracked file, an untracked nested repository or any submodule entry makes it unknown, and so does a source that is not a git checkout (no commit, so no git identity); the build and its diagnostics are kept; ignored files unchanged (§5.4) | 2026-11-03 |
| — | **Coverage classes (A2.1):** *waived* keeps this plan's meaning, *written, comparison waived* (F1 unchanged); waivers and removals are ledger entries with `check = "coverage"` and a `class`; an owner approves one, by an `accepted_by` that the ledger's `owners` lists (§5.2) | 2026-11-03 |
| **Q26** | **Evidence provenance:** evidence builds use a committed, clean Folia revision; the evidence keeps its builds' records (`build.json`, `sha256.txt`) and names the commit and identity fields; it is committed on its own, with that revision as its only parent, one combined folder per revision; absolute paths are diagnostic, never identity (§5.4). Rationale: a record identifies the tree it ran from, so evidence written into that tree changes the identity it records; built from a commit and committed after it, the evidence names its parent and never itself. Checked by `just evidence-provenance`. The A2.3 closure's evidence predates it and is rebuilt by it once its source is committed; A0's format is unchanged | 2026-11-03 |
| **Q25** | **A `match` item that explains no difference on an output with its reviewed files makes its entry stale, and fails the run** (§5.2), not only final acceptance (F13). Rationale: such an item makes the entry broader than what it explains, so it could later accept a difference nobody reviewed on that output; a run that only reported it would let the ledger drift wider than its review. Raised on 2026-11-03 to confirm the rule as implemented (`crates/parity/src/document/ledger.rs`, `EntryResult::unused`; test `a_match_item_unused_on_an_output_as_reviewed_makes_the_entry_stale`); confirmed by the owner | 2026-11-03 |
| — | Workspace (v9 §4), MiniJinja (v9 §8), pulldown-cmark (v9 §5.3), the combined feed in `folia.toml` (v9 §7.3), the slice's step decisions (v9 §12.2) | as recorded in v9 |

### 10.2 Open

Each with options, a recommendation and a decide-by milestone. The author decides each; the step
named in *Decide by* implements the decision. **Q3** is split into one row per Jekyll defect; for
each the options are *preserve* (parity, no ledger entry, changes deferred to after the switch) or
*fix* (an accepted ledger entry, a production change). Q3o is closed (§10.1).

| # | Question | Options and trade-offs | Recommendation | Decide by |
|-|-|-|-|-|
| **Q3a** | Atom feeds announced as `application/rss+xml` in every page's head (`_includes/feed-link.html`) | preserve / fix (changes every page's head) | preserve now; fixing later touches every validated page | A2 |
| **Q3b** | `/feed.xml`'s title `"Filopoetica \| "` and author name `gg` (no `site.data.authors`) | preserve / fix | preserve | C2 |
| **Q3c** | micro entries absent from search (dead loops) | preserve / fix (needs micro search text; RSVP bodies, v9 §7.4) | fix, as v9 Q3 recommended | C4 |
| **Q3d** | stubs and `redirects.json` indexed by search | preserve / drop them | preserve unless Q3c's fix revisits membership | C4 |
| **Q3e** | `/blog/tags.html` excluded by a substring rule, `/blog/categories.html` kept | preserve / fix | preserve | C4 |
| **Q3f** | search bodies of listing pages are raw Liquid or rendered text, by Jekyll's render order | reproduce from the claimed sources' text / use rendered text (ledgered) | use rendered text, ledgered, if the derived similarity threshold allows it; else reproduce | C4 |
| **Q3g** | sitemap `lastmod` of 7 static files from file mtimes | reproduce mtimes (checkout-dependent) / git dates (ledgered) | git dates, ledgered | C3 |
| **Q3h** | `narrow-post` typo in `pages/thanks.md:8` | keep as an accepted no-op (Jekyll ignores it) / delete / correct to `narrow_post` (changes production layout) | keep as a no-op until the author decides; correcting needs explicit authorization | A3 |
| **Q3i** | `/changelog` written as a bare fragment (no layout) | preserve / give it a layout | preserve | B2 |
| **Q3j** | `cover: top` on one post, which Jekyll renders as no cover | preserve / fix the source | preserve | B3 |
| **Q3k** | the dead `stem cells` tag feed link on `/blog/tags.html` | preserve (ledgered broken link) / fix the link | preserve, ledgered when C1 builds `/blog/tags.html` and A2.8's link check meets it | C1 |
| **Q3l** | the broken wiki-link in `coding/rust.md` | preserve (expected warning) / fix the content | preserve; it is A2.9's expected warning | A2 |
| **Q3m** | micro `datetime=" 2025-06-15"` leading space | preserve / fix | fix, as v9 Q3 recommended | B4 |
| **Q3n** | `manifest.json`'s `start_url` uses `www.ggirelli.info` while the site is `ggirelli.info` | preserve (static, byte-compared) / fix the file (ledgered) | preserve | D2 |
| **Q3p** | the dead `_plugins/external_links.rb` | preserve (Folia adds no `target`, matching production) / delete at cutover | delete the plugin at cutover | D2 |
| **Q3q** | search titles interpolated unescaped, and bodies made by `markdownify` over already-rendered HTML | reproduce / escape and extract text properly (ledgered) | reproduce where the derived similarity allows; decide with Q3f | C4 |
| **Q6** | Hosting, deploy and rollback | GitHub Pages as today vs another host | Stay on GitHub Pages, deployed through Actions artifacts. Record the custom domain (`CNAME` is `ggirelli.info`; with an Actions deploy the repository's Pages settings set the domain) and HTTPS; build with the base path the Pages action gives (`--baseurl`, `jekyll.yml`), which is empty today; retain the last Jekyll Pages artifact as the rollback, so a rollback does not rebuild with an unpinned toolchain; keep the claimed sources (§3.4) for the rollback window | D3 |
| **Q7** | Sass compiler and source maps | `grass` (pure Rust; may not match `@import` plus silenced deprecations; maps differ) vs `dart-sass` (an external binary; can match `sass-embedded` 1.100.0) | Try `grass` against the baseline's CSS; fall back to `dart-sass` if the CSS differs; maps may be ledgered (§5.3, F4, F7) | C5 |
| **Q8** | Configuration in `_folia/folia.toml` | a new minimal file (Jekyll `defaults` become mode code, as v9 Q8 said) / read `_config.yml` | the new file (in use as `parity/filopoe.toml`); move it at D2 | D2 |
| **Q13** (rest) | Activate the lint on `filopoe`'s `main`; HTML pages; workflow paths | activate with SITE-PR / earlier; HTML pages: a check later, or none, since the claimed sources are not rendered | activate with SITE-PR (Q21); no HTML-page lint (§3.4 renders no legacy body); add `_folia/folia.toml` to the paths at D2 | D2 |
| **Q16a** | Interactions (inter-pull: the JS form, `post_interactions.html`, the privacy policy's text, 27 documents) in scope? | in scope: port the include / out: ledger the removal and update `pages/policy.md` | in scope | B3 (B4 follows B3) |
| **Q16b** | Where author data lives (covers, social handles, `bluesky_did`) | `folia.toml`'s `[authors]` table (exists, `parity/filopoe.toml`; `deny_unknown_fields`) / content | `[authors]`, typed; needed by A1.6's frame data and `/` | A1 |
| **Q17** | Acceptance baseline toolchain and timing | rebaseline now with Ruby 3.4.5 / once at D2; fresh clone / working checkout; commit `Gemfile.lock` to `filopoe` (needs authorization) / store it in evidence only | Keep `BASELINE-B` for development; A2.7 compares toolchains early; rebaseline once at D2, after Q21's history, from a fresh clone, with production's Ruby; store the lockfile in the evidence, and consider committing it | A2 (comparison), D2 (rebaseline) |
| **Q18** | Title markup policy | reproduce Jekyll's filters (`<br />` → space in the head; double space → `<br />` in the heading) / edit the 3 titles | reproduce in Rust (presentation data) | B3 |
| **Q19** | Operational authorizations | each a separate, explicit authorization | (1) move Folia's `main` to `0302e08` and push, **after** updating the docs navigation (`mkdocs.yml`, `docs/planning/index.md`, `README.md`, `docs/index.md`), since the push deploys Folia's docs; (2) push the planning commits; (3) integrate `folia/templates`; (4) merge SITE-PR (method: Q21); (5) refresh or remove `SITE-WORKSPACE` | (1) A0; (2) with (1); (3)–(4) D2; (5) A0 |
| **Q20** | The claimed legacy sources (§3.4), including the search file's layout (§3.5) | keep, discovered and claimed (read as data, bodies unused) / exclude by configuration (breaks route parity and loses their redirects, frame data and search text) / delete. Search file: one file, generated data plus a verbatim static fragment / split into static JS and a data file (a new route, ledgered) | keep and claim; one search file; delete the sources only after the rollback window (Q6), moving their front matter into the owners' content or configuration then | A2 (model, search file); D2 (deletion plan); D3 (deletion) |
| **Q21** | SITE-PR's merge method | fast-forward (keeps `BASELINE-A` and `SITE-COMMIT-B` and their committer times) / merge commit (keeps them; adds one commit) / squash or rebase (new committer times and hashes) | fast-forward if the branch is still a descendant of `main` (it is, as of the last fetch). GitHub's pull-request merge offers no fast-forward, so this would mean pushing the branch's commits to `filopoe`'s `main` directly, which needs Q19(4)'s explicit authorization and would also close the PR; otherwise a merge commit; never squash or rebase. The final baseline is built after this history exists (D2) | D2 |
| **Q22** | Is the robots `NOINDEX, NOFOLLOW` meta and `robots.txt`'s `Disallow: /` intended? | intended, preserved / unintended (a production change) | ask the author; preserve and assert it meanwhile (§5.1) | A2 |
| **Q23** | The 55 bodies equal only after normalization | explain each by a normalization rule or a ledger entry / accept as normalized | explain each; at most 19 are highlighting | A2 |
| **Q24** | D-001's open follow-up (a visual check of its two posts) | do it now / at F14 | at F14, recorded against D-001 | D3 |

### 10.3 Superseded

- v9 §6.3 listing pages: decided as Q4.
- v9 §9.2's per-page fingerprint as acceptance: replaced by Q15.
- v9 §12.3's "the next plan then picks": §6.
- v10's Q16: split into Q16a and Q16b. v10's Q3: split into Q3a–Q3q (Q3o closed). v10's Q20 (decide
  by C1): moved to A2 and widened. v10's F15: split into F15 and F16.

## 11. Non-goals

v9 §13 stands: no navigation model, no `Mode` trait or plugin registry, no RSS 2.0, no template
processing of Markdown bodies. In addition: no template-generated structured formats (Q14), no
rendering of legacy listing sources' bodies (§3.4), and no fixes to Jekyll's behaviour during
migration beyond those Q3 accepts.
