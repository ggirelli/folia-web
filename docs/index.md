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
      [MIT license](https://github.com/ggirelli/folia/blob/main/LICENSE), provided as is,
      **without warranty**.

## Status

Folia has finished its validation spikes ([plan v5, §10](planning/plan.v5.md#10-validation-spikes));
the first vertical slice is next. There is no generator yet. What exists is the parity harness,
which records production builds of the Jekyll site with Jekyll's own intermediate results,
Folia's discovery and routes, which reproduce every output path of the site, and its Markdown
renderer, which reproduces the body HTML of every Markdown document. See
[parity baselines](baseline.md).
