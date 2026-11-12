# Templates

Folia renders each page through a template. Templates are [MiniJinja](https://docs.rs/minijinja)
files (Jinja syntax: `{{ value }}`, `{% if %}`, `{% for %}`, `{% extends %}`, `{% block %}`),
in the directory the configuration's `templates-dir` names (`_folia/templates` by default). They
are **not** Liquid: a Jekyll site's layouts and includes have to be rewritten.

The example site's [`_folia/templates`](https://github.com/ggirelli/folia-web/tree/main/examples/minimal-site/_folia/templates)
is a complete, small set to start from.

## Which template renders what

| Content | Template |
|-|-|
| a post | `post.html` |
| a garden note | `garden_post.html` |
| a micro note, like, bookmark, RSVP | `page.html`, `like.html`, `bookmark.html`, `rsvp.html` |
| a page | the front matter's `layout`: `layout: page` is `page.html`, `layout: error` is `error.html` |
| a page with no `layout` | none: the page is its own content, as Jekyll writes it |
| the blog index, category and tag pages, micro listings | the template the configuration's `[blog]` and `[[micro.listings]]` name |
| a category archive | the `[blog]` `archive-template` |

A page that needs a template that does not exist, or whose template fails to render, **fails the
build** and names the page and the template. Nothing is skipped silently.

## What a template can use

Every template sees `page` (the page being rendered), `content` (its body as HTML, written
`{{ content | safe }}`) and `site` (the site's title, URL, authors and categories). **An undefined
value is an error**, never an empty string: a misspelt `{{ page.titel }}` stops the build and says
where. Test a value that may be absent with `{% if page.author %}`. The full list of what each
kind of page offers, and the filters, are in the
[template context reference](../reference/template-context.md); the example templates use almost
all of them.

## Blocks and the search index

A template can define blocks (`{% block body %}`). When the site has a search script, listings
are indexed by the text of the block named by `[search]`'s `content-block`, so the navigation and
footer are not in the index; see [Feeds, sitemap and search](feeds-sitemap-search.md).
