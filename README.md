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

Validation spikes, done; the first vertical slice is under way. Its first
step, built with its checks passing and committed in Folia (its templates
are not yet committed in the site), builds one page (`/about`) with
`folia build`: rendered from templates kept in the site, with its redirect
stub and the static
files, matching Jekyll's output. The repository also holds the parity
harness, which records production builds of the Jekyll site; Folia's
discovery and routes, which reproduce every output path of the site
(spike 2); and its Markdown renderer, which reproduces the body HTML of
every Markdown document of the site (spike 1).

- [Parity baselines](docs/baseline.md): how to build and compare them.
- [Development](docs/development.md): how the repository is organized.
- [Plans](docs/planning/): the architecture proposal; `plan.v7.md` is current.

Contributions follow [CONTRIBUTING.md](CONTRIBUTING.md).
