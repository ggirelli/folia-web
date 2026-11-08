# Folia

Folia is a static-site generator written in Rust. It reads a git repository of Markdown, HTML,
templates and one configuration file, and writes a static site you can host anywhere. It was
written to replace Jekyll for [ggirelli.info](https://ggirelli.info), and it publishes four kinds
of content:

- **Blog:** posts, with archives, pagination, categories, tags and feeds.
- **Microblog:** short notes, likes, bookmarks and RSVPs.
- **Digital garden:** notes connected by wiki-links, with backlinks and a note graph.
- **Pages:** individually authored pages with explicit URLs, redirects included.

It also writes the Atom feeds, the sitemap, a search index script and the compiled Sass
stylesheets.

Folia is built for one website. It is not meant as a general-purpose alternative to Jekyll, Hugo
or Zola, although the code is public and you are welcome to use it.

!!! info "Free, open source and vibe coded"
    - **Fully vibe coded:** an AI assistant (Claude) writes all of its code, tests, scripts and
      docs; see [how it is made](how-it-is-made.md).
    - **Free and open source** under the
      [MIT license](https://github.com/ggirelli/folia-web/blob/main/LICENSE), provided as is,
      **without warranty**.

## Where to go

| If you want to | Read |
|-|-|
| install Folia and build a first site | [Getting started](getting-started.md) |
| write posts, notes and pages | [Writing content](guide/content.md) |
| write the templates that turn them into pages | [Templates](guide/templates.md) |
| build, serve and deploy a site | [Building and deploying](guide/building.md) |
| know what the feeds, sitemap and search script do | [Feeds, sitemap and search](guide/feeds-sitemap-search.md) |
| look up a configuration key | [Configuration reference](configuration.md) |
| move an existing Jekyll site over | [Migrating from Jekyll](migrating/index.md) |
| know what Folia does not do | [Known limitations](known-limitations.md) |
| work on Folia itself | [Development](development.md) |

## Status

Folia 0.1.0 is the first pre-1.0 release. It builds the author's own site, its checks pass, and it
refuses to overwrite files it did not write. It is a release of the software: its output equals
Jekyll's for most of that site and differs, on purpose or by a known limitation, for the rest, so
read [Known limitations](known-limitations.md) before replacing a Jekyll site. See the
[release notes](release-notes/v0.1.0.md).

## Support

Folia is free and written for the author's own site. If it is useful to you, you can support its
upkeep through [GitHub Sponsors](https://github.com/sponsors/ggirelli) or
[Ko-fi](https://ko-fi.com/ggirelli). Sponsoring buys no support, features or warranty.
