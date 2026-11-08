# Folia

Folia is the Rust static-site generator behind [ggirelli.info](https://ggirelli.info), written to
replace Jekyll. It publishes four kinds of content:

- **Blog:** articles, archives, pagination, tags, and categories.
- **Microblog:** notes, likes, bookmarks, and RSVPs.
- **Digital garden:** notes connected by wiki-links, with backlinks and a note graph.
- **Pages:** individually authored pages with explicit URLs.

Folia is built for one website. It is not meant as a general-purpose alternative to Jekyll, Hugo,
or Zola, although the code is public.

!!! info "Free, open source and vibe coded"
    - **Fully vibe coded:** an AI assistant (Claude) writes all of its code, tests, scripts and
      docs; see [how it is made](how-it-is-made.md).
    - **Free and open source** under the
      [MIT license](https://github.com/ggirelli/folia-web/blob/main/LICENSE), provided as is,
      **without warranty**.

## Status

`folia build` builds the whole reference site: its pages, posts, micro entries and garden notes,
the blog and micro listings, redirects, stylesheets (with Dart Sass), the Atom feeds, the sitemap
and the search script; it skips, and names, the one document it cannot build yet (the combined
`/feed.xml`). The pages are checked against Jekyll's output with the parity harness
([parity baselines](baseline.md)); the feeds, sitemap and search script are tested on fixtures
only, not yet compared with Jekyll's. [Plan v13](planning/plan.v13.md) is the plan for the rest of
the migration. Nothing is migrated: the Jekyll site is still the one published.
[Manual testing](development/manual-testing.md) shows how to try Folia on a public example site.
