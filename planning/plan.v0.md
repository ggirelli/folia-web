# Folia --- Project Plan

**Status:** Initial planning draft\
**Project type:** Personal, in-house static-site publishing engine\
**Implementation language:** Rust\
**Working name:** Folia\
**Primary reference site:** `ggirelli.info` (currently generated with
Jekyll)

## 1. Purpose and motivation

Folia is a Rust-based static-site generator intended to replace Jekyll
for the author's personal website.

The primary goal is not to build a general-purpose competitor to Jekyll,
Hugo, or Zola. It is to make the website easier to understand, extend,
and maintain by building the publishing system in a language the author
knows well, with clear architectural boundaries and an explicit model of
how publishing features work.

The repository may be public, but the product requirements are driven by
one site. Avoid designing for a hypothetical user base, plugin
marketplace, theme ecosystem, hosted service, or full Jekyll
compatibility unless a concrete need emerges.

The project should be enjoyable to build, but scope must remain
manageable. Prefer a small, understandable core over a broad feature
set.

## 2. Existing website

The current Jekyll site has four publishing modes:

1.  **Blog** --- articles, blog-associated archives, pagination, tags,
    categories, and related feeds.
2.  **Microblog** --- typed entries such as notes, likes, bookmarks, and
    RSVPs.
3.  **Digital garden** --- notes connected by wiki-links, with
    backlinks, broken-link handling, and a graph/network visualization.
4.  **Pages** --- individually authored pages such as the landing page
    and About page, with manually controlled URLs and placement in site
    navigation or other locations.

The site also has global client-side search based on Lunr, shared
layouts and assets, feeds, SEO metadata, and static hosting/deployment
behavior.

The existing Jekyll source repository and its generated `_site` output
are the reference for migration. The target is functional and visual
parity where it matters, not byte-for-byte identical HTML.

## 3. Design principles

-   **Understandability over abstraction.** The author should be able to
    trace how a feature works and where to change it.
-   **Clear boundaries.** Content parsing, publishing semantics,
    rendering, feed generation, and build orchestration have distinct
    responsibilities.
-   **One owner per authored document.** Each authored document belongs
    to exactly one publishing mode.
-   **One document, multiple generated outputs.** A source document can
    generate more than one output, such as a canonical page and an
    archive entry.
-   **Modes are peers.** Blog, microblog, garden, and pages should not
    depend on one another directly.
-   **Shared representations.** Modes communicate through explicit
    shared models and page/artifact specifications rather than reaching
    into each other's internals.
-   **Static output.** The result is a directory of ordinary static
    files; a server is not required at runtime.
-   **Incremental ambition.** Start with a reliable clean build and
    validation. Add watch mode, a preview server, and incremental
    rebuilds when justified.
-   **Preserve existing behavior first.** Keep current URLs, search
    behavior, feeds, graph visualization, metadata, and frontend assets
    working during migration.
-   **Use existing crates where sensible.** Do not invent a template
    language, Markdown parser, feed serializer, or other infrastructure
    without a concrete reason.

## 4. Proposed workspace shape

These are provisional crate names and boundaries, to be reviewed before
implementation.

-   `folia-core`
    -   Discover and parse source documents.
    -   Parse front matter and Markdown into an in-memory
        representation.
    -   Provide shared source identifiers, metadata, and diagnostics.
    -   Avoid publishing-mode-specific rules.
-   `folia-blog`
    -   Blog posts, blog-specific page selection, pagination, archives,
        tags, and categories.
    -   Define which documents belong in blog listings and how they are
        ordered.
    -   Do not own shared feed serialization or HTML rendering.
-   `folia-micro`
    -   Microblog entry types: notes, likes, bookmarks, and RSVPs.
    -   Type-specific validation, URLs, listings, and derived data.
    -   Do not own shared feed serialization or HTML rendering.
-   `folia-garden`
    -   Garden note identity, wiki-link resolution, backlinks,
        broken-link diagnostics, and graph data.
    -   Generate graph artifacts compatible with the existing
        visualization.
    -   Do not depend on the blog or microblog crates.
-   `folia-pages`
    -   Individually authored pages with explicit routes and metadata.
    -   Keep URL identity separate from placement in navigation or other
        pages.
    -   Navigation should be a site-level concern, not hard-coded into
        this crate.
-   `folia-feeds`
    -   Shared RSS/Atom serialization and feed-related formatting.
    -   Consume selected published entries supplied by the builder.
    -   Support mode-specific feeds and combined feeds without depending
        directly on each publishing-mode crate.
    -   Feed membership and ordering should be explicit inputs, not
        hidden global behavior.
-   `folia-render`
    -   Render page specifications through templates/layouts/includes.
    -   Handle HTML escaping and template diagnostics.
    -   Avoid owning blog archives, garden graph logic, or other
        publishing semantics.
-   `folia-builder`
    -   Coordinate configuration, content loading, publishing modes,
        shared site assembly, validation, rendering, and output writing.
    -   Detect duplicate routes and output collisions.
    -   Ensure failed builds do not leave a misleading partially
        published output.
-   `folia-cli` (or a similarly named binary crate)
    -   Expose commands such as `build` and `check`; later, `serve` and
        `clean`.
    -   Keep command-line concerns out of the library crates.

A small shared `site-model` crate may be warranted for canonical content
identifiers, resolved URLs, page specifications, navigation, and
generated artifacts. Decide whether it should be a separate crate or
initially live in `folia-core`/`folia-builder` based on the actual
dependency graph. Do not add a crate solely to increase modularity.

Likewise, static asset copying and Sass handling need an explicit home,
but do not create another crate until its boundary is clear.

## 5. Architectural boundaries

The intended dependency direction is inward toward shared data models
and outward toward orchestration and infrastructure. Publishing modes
must not depend directly on one another.

In particular:

-   `folia-core` must not depend on `folia-blog`, `folia-micro`,
    `folia-garden`, or `folia-pages`.
-   Publishing modes share types through common models and do not import
    one another.
-   The renderer consumes page/render specifications; it does not
    implement publishing rules.
-   `folia-feeds` serializes feed entries supplied to it; it does not
    discover content or decide site-wide feed membership implicitly.
-   `folia-builder` coordinates the pipeline but should not absorb the
    logic of each mode.
-   The CLI and preview server are outer-layer interfaces.
-   HTTP serving is development infrastructure, not a requirement for
    producing a static site.

## 6. Conceptual build pipeline

The following is a starting model, not a finalized API:

1.  Load and validate site configuration.
2.  Discover source files and parse front matter and Markdown.
3.  Assign each authored document to exactly one publishing mode.
4.  Build a shared content/site model with stable identities.
5.  Run mode-specific transformations and validation.
6.  Resolve routes and cross-document references.
7.  Produce page specifications and derived artifacts (archives, graph
    data, search index inputs, feeds, sitemap, redirects).
8.  Validate duplicate routes, broken links, missing assets, and invalid
    metadata.
9.  Render HTML and serialize non-HTML artifacts.
10. Copy or process static assets.
11. Write the complete output directory deterministically.

The exact ordering and which stage owns each artifact should be refined
in planning, especially where garden link resolution, route resolution,
feed selection, and rendering depend on one another.

## 7. Rendering and templates

The existing site uses Jekyll/Liquid templates. Evaluate whether a Rust
Liquid implementation covers the actual tags, filters, includes, and
layout behavior in use. Also evaluate a Rust-native template engine such
as MiniJinja.

Do not decide based on language preference alone. Compare migration
effort, ease of authoring future features, template diagnostics, and
long-term maintainability.

Avoid writing a custom template language. The goal is to own the
publishing architecture, not reinvent every component.

## 8. Feeds and search

### Feeds

Feeds should be handled by a dedicated shared component if that boundary
remains useful after dependency analysis. Publishing modes provide or
identify entries; the builder selects feed membership and ordering;
`folia-feeds` serializes RSS/Atom and writes the requested outputs.

Initial needs include mode-specific feeds and a combined feed. Confirm
exact feed paths, content inclusion, ordering, dates, and metadata
against the generated Jekyll site.

### Search

Keep the current Lunr frontend initially. Identify the existing index
schema and generation path, then generate a compatible index from Folia.
Replacing Lunr is out of scope until the migration works.

## 9. Migration and acceptance baseline

Use both the Jekyll source and its generated `_site` output as
references.

Before broad implementation, create reproducible inventory and parity
tooling for:

-   Existing output routes and file paths.
-   URLs, trailing-slash conventions, redirects, and canonical URLs.
-   Titles, headings, dates, tags/categories, descriptions, and SEO
    metadata.
-   Blog archives, pagination, and taxonomy pages.
-   Every microblog entry type.
-   Garden links, backlinks, broken links, and graph nodes/edges.
-   Feed item URLs, dates, titles, and ordering.
-   Lunr search index schema and frontend behavior.
-   Static assets and the existing graph visualization.
-   Representative desktop and mobile page appearance.

HTML does not need to be byte-identical. Compare normalized structure
and meaningful behavior.

Keep Jekyll as the production build until Folia's output has been
reviewed and the acceptance checks pass.

## 10. Suggested first vertical slice

Do not implement every mode immediately.

Start with a clean end-to-end path that can:

1.  Parse one Markdown page with front matter.
2.  Construct the shared in-memory representation.
3.  Resolve its output route.
4.  Render it through a template.
5.  Copy required static assets.
6.  Validate basic metadata and internal links.
7.  Produce deterministic output.

Then add one representative blog post and one garden note. The blog
validates the ordinary publishing path; the garden validates
cross-document identity, reference resolution, and derived artifacts.

Use those slices to test whether the proposed crate boundaries make
changes easy before expanding to all features.

## 11. Explicit non-goals for the first version

-   General-purpose Jekyll compatibility.
-   A public plugin API or theme ecosystem.
-   A hosted CMS or deployment service.
-   Rewriting frontend JavaScript or redesigning the website.
-   Replacing Lunr before compatibility is established.
-   Complex incremental builds before clean builds are reliable.
-   Supporting hypothetical publishing modes before the current four
    work well.

## 12. Open decisions for planning

Resolve these with evidence from the repository before committing to
implementation:

1.  What exactly is the shared in-memory document model, and which
    metadata is genuinely common?
2.  Should Markdown be stored as source text, a parsed AST, or both?
3.  Which template engine best supports current templates while
    improving future authoring?
4.  What is the stable identity model for documents and URLs?
5.  How should modes emit page specifications and non-page artifacts?
6.  Where should shared route resolution, navigation, asset handling,
    and validation live?
7.  How should feed membership be expressed for mode-specific and
    combined feeds?
8.  How is the current Lunr index generated, and what schema does it
    expect?
9.  Which Markdown extensions and syntax-highlighting behavior must be
    preserved?
10. What are the exact build and deployment steps, including Sass
    compilation and last-modified dates?

## 13. Definition of done for the migration

Folia can build the current site into static output without invoking
Jekyll; the four publishing modes remain isolated behind clear
interfaces; existing URLs, feeds, search, garden graph, metadata, and
frontend behavior are preserved; validation catches important errors
with actionable diagnostics; and the output can be safely reviewed
before replacing the production build.

The measure of success is not the number of features or crates. It is
whether implementing the next website idea becomes easier because the
system is understandable and deliberately designed.
