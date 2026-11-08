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

Folia 0.1.0 is the first pre-1.0 release: it builds the whole reference site, its checks pass and it
refuses to overwrite files it did not write. **A clean build is not exact parity**: some outputs
differ from Jekyll's on purpose or are known to differ ([known limitations](docs/known-limitations.md)),
and Folia does not claim to replace a Jekyll site on its own. See the
[release notes](docs/release-notes/v0.1.0.md).

The documentation is published at <https://ggirelli.github.io/folia-web/>.

- [Getting started](docs/getting-started.md): install Folia and build the example site.
- [Writing content](docs/guide/content.md), [templates](docs/guide/templates.md),
  [building and deploying](docs/guide/building.md) and the
  [configuration reference](docs/configuration.md).
- [Migrating from Jekyll](docs/migrating/index.md): how to compare a Folia build with a Jekyll one.
- [Known limitations](docs/known-limitations.md): how Folia's output differs from Jekyll's.
- [Development](docs/development.md) and [manual testing](docs/development/manual-testing.md): how
  the repository is organized, and how to check it.

Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md).
