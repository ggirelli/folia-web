<h1 align="center">
  <img src="assets/brand/folia-banner.png" alt="Folia: a publishing engine for your digital garden">
</h1>

Folia is the Rust static-site generator behind [ggirelli.info](https://ggirelli.info),
written to replace Jekyll. It publishes four kinds of content:

- **Blog:** articles, archives, pagination, tags, and categories.
- **Microblog:** notes, likes, bookmarks, and RSVPs.
- **Digital garden:** notes connected by wiki-links, with backlinks and a
  note graph.
- **Pages:** individually authored pages with explicit URLs.

Folia is built for one website. It is not meant as a general-purpose
alternative to Jekyll, Hugo, or Zola, although the code is public.

> [!IMPORTANT]
>
> - **Fully vibe coded:** an AI assistant (Claude) writes all of its code, tests, scripts and docs.
>   Read the [disclaimer](DISCLAIMER.md) before relying on it.
> - **Free and open source** under the [MIT license](LICENSE), provided as is, **without warranty**.

## Status

`folia build` builds the whole reference site: its pages, posts, micro entries and garden notes,
the blog and micro listings, redirects, stylesheets (with Dart Sass), the Atom feeds, the sitemap
and the search script; it skips, and names, the one document it cannot build yet (the combined
`/feed.xml`). The pages are checked against Jekyll's output with the parity harness; the feeds,
sitemap and search script are tested on fixtures only, not yet compared with Jekyll's. The Jekyll
site is still the one published.

The documentation is published at <https://ggirelli.github.io/folia-web/>.

- [Parity baselines](docs/baseline.md): how to build and compare them.
- [Development](docs/development.md): how the repository is organized.
- [Manual testing](docs/development/manual-testing.md): how to build and check the example site,
  and run real-site parity.
- [Plans](docs/planning/): the architecture proposal; `plan.v13.md` is current.

Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md).
