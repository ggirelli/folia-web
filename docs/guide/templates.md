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

## What a template sees

Three names, always:

| Name | What |
|-|-|
| `page` | The page being rendered: its URL, title, date and so on (below) |
| `content` | The page's body as HTML. Write it as `{{ content \| safe }}` |
| `site` | The site: `site.url`, `site.title`, `site.description` (from `[site]`), `site.year` (the build's), `site.authors.<key>` (from `[authors]`) and `site.categories` (`name`, `anchor`, `count` for each) |

**An undefined value is an error**, never an empty string: a misspelt `{{ page.titel }}` stops the
build and says where. Test a value that may be absent with `{% if page.author %}`.

### `page`, by kind

Every kind has `page.url`, `page.title`, `page.layout`, `page.date` (an ISO time, or none for a
page), `page.modified`, `page.description` and `page.author`. The rest depends on the kind:

| Kind | Also |
|-|-|
| post | `categories` and `tags` (each `{name, anchor}`), `previous` and `next` (`{url, title}`), `toc`, `cover`, `syndications`, `footer`, `narrow_post`, `hide_website_junk`, `interactions` |
| garden note | `backlinks` (`{title, url, excerpt}`), `garden_map`, `toc`, `cover` |
| micro entry | the posts' keys with the kind's defaults; `page.like`, `page.bookmark`, `page.rsvp` and `page.event` hold the kind's own data |
| page | `hide_title`, `title_icon`, `toc`, `cover`, `corner_backlink`, `error_type`, `additional_body_class`, `hide_website_junk` |
| blog index | `items` (post cards, newest first, each `{url, title, cover_path, box_position, summary, author, date}`), `featured` (page 1 only) and `paginator` (`page`, `total_pages`, `previous_page_path`, `next_page_path`) |
| categories and tags page | `groups`: each `{name, anchor, count, items, feed}` (`feed` is none for a category), by name; `items` are post cards |
| micro listing | `items`: the entries, newest first, each with `kind`, `url`, `title`, `date` and `summary` |

The exact contents of each are in the
[example templates](https://github.com/ggirelli/folia-web/tree/main/examples/minimal-site/_folia/templates),
which use almost all of them, and in the doc comments of the modes' crates (`crates/blog`,
`crates/garden`, `crates/micro`, `crates/pages`).

### Filters

Besides MiniJinja's own, `date(format)` writes a time in a `strftime` format: for example
`{{ page.date | date("%d %b %Y") }}`. The title filters (`head_title`, `heading_title`,
`title_markup`) are for the author's site; read their source before using them.

## Blocks and the search index

A template can define blocks (`{% block body %}`). When the site has a search script, listings
are indexed by the text of the block named by `[search]`'s `content-block`, so the navigation and
footer are not in the index; see [Feeds, sitemap and search](feeds-sitemap-search.md).
