# Template context

What a [template](../guide/templates.md) can use. The example site's
[templates](https://github.com/ggirelli/folia-web/tree/main/examples/minimal-site/_folia/templates)
use almost all of it, and the doc comments of the collections' crates (`crates/blog`,
`crates/garden`, `crates/micro`, `crates/pages`) hold the exact contents of each key.

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
| page | `hide_title`, `noindex`, `title_icon`, `toc`, `cover`, `corner_backlink`, `error_type`, `additional_body_class`, `hide_website_junk` |
| blog index | `items` (post cards, newest first, each `{url, title, cover_path, box_position, summary, author, date}`), `featured` (page 1 only) and `paginator` (`page`, `total_pages`, `previous_page_path`, `next_page_path`) |
| categories and tags page | `groups`: each `{name, anchor, count, items, feed}` (`feed` is none for a category), by name; `items` are post cards |
| micro listing | `items`: the entries, newest first, each with `kind`, `url`, `title`, `date` and `summary` |

#### `page.lang`

Posts, garden notes, micro entries and pages have `page.lang` **only when their front matter sets
`lang`**, exactly as written (`lang: [italian]` is a list, `lang: italian` a string). Blog and listing
pages never have it. Because an undefined value is an error, guard it:
`{% if page.lang is defined %}`.

### Filters

Besides MiniJinja's own, Folia adds four.

`date(format)` writes a time in a `strftime` format: `{{ page.date | date("%d %b %Y") }}`.

The other three treat a title that may contain `<br />` (or `<br/>`) line breaks. They exist
because the site Folia was written for sets some titles over two lines:

| Filter | What it does |
|-|-|
| `head_title` | Turns each `<br />` into a space: the title for `<title>` and meta tags |
| `title_markup` | Escapes the title, but keeps each `<br />` as a real line break (safe to write as is) |
| `heading_title` | As `title_markup`, and turns each double space into a line break too |

You can ignore them if your titles have no line breaks.
