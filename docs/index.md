# Folia

![Folia: a little more joy in publishing](assets/folia-banner.png)

**A little more joy in publishing.**

Folia is a fast static site generator written in Rust. It reads a git repository of Markdown,
templates and one configuration file, and writes a static site you can host anywhere.

!!! warning "Fully vibe coded, and provided as is"
    An AI assistant (Claude) writes all of Folia's code, tests, scripts and docs, directed and
    reviewed by the maintainer. It is tested, not audited, so expect rough edges. Folia was started
    for one website, [ggirelli.info](https://ggirelli.info), and you are welcome to use it for
    yours. It is free software under the
    [MIT license](https://github.com/ggirelli/folia-web/blob/main/LICENSE), without warranty. The
    whole story is in [About Folia](about.md).

## What Folia is for

Folia is for a personal site that is more than a blog. It has four collections: a **blog**, a
**microblog** of short notes, likes, bookmarks and RSVPs, a **digital garden** of notes joined by
wiki-links and backlinks, and **pages** with URLs you choose. It also writes the Atom feeds, the
sitemap, search data and redirects.

I wrote it for three things. **Speed:** `folia serve` rebuilds as you save and writes only what
changed. **Simplicity:** one command, one configuration file, one way to go from a Markdown file
to a page. **Ownership:** your content is plain files in your own git repository, and the output
is a folder of static files you can host anywhere.

## Learning path

1. [Install](getting-started/install.md) Folia.
2. Build [your first site](getting-started/first-site.md): copy the example, edit it live, build
   it.
3. [Build and deploy](getting-started/build-and-deploy.md) it.
4. Learn to write [content](guide/content.md) and shape it with [templates](guide/templates.md),
   and look up keys in the [configuration reference](reference/configuration.md).

## Elsewhere in the docs

| If you want to | Read |
|-|-|
| preview a site while you write, or schedule a post | [Preview while you write](guide/preview.md), [Drafts and scheduling](guide/drafts.md) |
| know what the feeds, sitemap and search script do | [Feeds, sitemap and search](guide/feeds-sitemap-search.md) |
| look up a command, a configuration key or what a template sees | [Reference](reference/cli.md) |
| move an existing Jekyll site over | [Migrating from Jekyll](guide/migrating.md) |
| know what Folia does differently from Jekyll, and what it does not do | [Differences from Jekyll and known limitations](internals/differences-from-jekyll.md) |
| know where it comes from and how it is made | [About Folia](about.md) |
| work on Folia itself | [Architecture](internals/architecture.md), [Development setup](contributing/development-setup.md) |

## Status

Folia is pre-1.0: see the [release notes](release-notes/v0.2.0.md). Its output equals Jekyll's for
most of the author's site and differs, on purpose or by a known limitation, for the rest, so read
[Differences from Jekyll](internals/differences-from-jekyll.md) before replacing a Jekyll site.

## Support

Folia is free. If it is useful to you, you can support its upkeep through
[GitHub Sponsors](https://github.com/sponsors/ggirelli) or [Ko-fi](https://ko-fi.com/ggirelli).
Sponsoring buys no support, features or warranty.
