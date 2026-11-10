# Writing content

A site has four kinds of content. Which kind a file is depends on where it lives: the directories
are named in the configuration's [`[roots]`](../configuration.md#roots-required), and **every other
file with front matter is a page**. A file with no front matter is a static file, copied as it is.

Every document is Markdown (or, for pages, HTML) with a YAML front matter block between two `---`
lines. The examples below come from the
[example site](https://github.com/ggirelli/folia-web/tree/main/examples/minimal-site).

!!! warning "Folia refuses what it does not know"
    A front matter key a document's kind does not support is an **error that names the key**, not
    something ignored, so a misspelling (`narow_post`) never silently changes a page. The tables
    below list the keys each kind supports. A value Folia cannot show is refused too.

!!! note "URL prefixes are fixed"
    Posts are published under `/blog/`, garden notes under `/dg/`, and micro entries under
    `/micro/`, `/likes/`, `/bookmarks/` and `/rsvps/`. They come from the sites Folia was written
    for, and are not configurable yet. Pages choose their own URLs.

## Markdown

Folia renders Markdown to the HTML Jekyll's kramdown wrote, so a site moved from Jekyll keeps its
pages:

- Headings get an `id`. Tables, strikethrough, task lists, smart punctuation, inline attribute
  lists (`{:.class}`), wiki-links in garden notes and raw HTML work as they do under kramdown's
  GFM parser. Footnotes and definition lists are not supported.
- Fenced code blocks are highlighted by [syntect](https://github.com/trishume/syntect), with class
  names matching Rouge's, so a Jekyll site's code stylesheet keeps working. A few percent of
  blocks get slightly different token classes than Rouge gave.
- **Liquid is not run.** `{{ ... }}` and `{% ... %}` in a document are shown as written. Move any
  Liquid out of the content before switching; see [Migrating from Jekyll](../migrating/index.md).

## Posts

Files in the `blog` root, named `YYYY-MM-DD-slug.md`. **The file name dates the post**: a `date`
key in the front matter is refused. A post dated after the build is left out, as is one with
`published: false`.

```markdown
---
title: Hello, Folia
author: ada
categories: [journal]
tags: [folia, rust]
toc:
  enabled: true
  position: left
---

The first post of the example site.
```

The post's URL is `/blog/YYYY/MM/DD/slug` (the slug from the file name, or `slug:` in the front
matter, or `permalink:`). Without a `title`, the title is made from the file name.

| Key | What |
|-|-|
| `title`, `author` | The title, and an author key of `[authors]` |
| `categories` or `category`, `tags` (or `tag`) | Lists, or a space-separated string for categories |
| `toc` | `enabled: true`, `position` (`top`, `left`, `right`), `title`, `min_level`, `max_level` |
| `cover` | A picture and where it goes (`position`: `behind` the page or `after_title`) |
| `featured`, `hidden` | `featured` posts head the blog index; `hidden` ones are left out of its listings |
| `published`, `slug`, `permalink`, `redirect_from` | Whether it is built; its URL; old URLs that redirect to it |
| `lang` | Its language, as written (`[italian]` or `italian`); templates read it as `page.lang` |
| `last_modified_at`, `footer`, `syndications`, `narrow_post` | Extras the post template can show |

The **modified date** a template sees comes from the front matter, else the file's last git
commit, else the post's date. Commit your changes: an uncommitted edit does not move it.

## Garden notes

Files anywhere under the `garden` root, in any folders; the path below the root is the URL, so
`_garden/coding/python.md` becomes `/dg/coding/python`. A note has a title and a date, and links to
others with wiki-links:

```markdown
---
title: A topic
date: 2026-01-05
---

A garden note links to others by title: see [[A leaf note]], or [[A leaf note|with other text]].
```

- A `[[Title]]` link finds the note with that title. One that names no note is shown as broken, and
  the build **warns** (it does not fail).
- Each note's page gets its **backlinks**, and the build writes a graph of all links to
  `assets/json/notes_graph.json`.
- Supported keys: `title`, `date`, `hide_garden_map`.

## Microblog entries

Files in the `micro` root, in a folder named for their kind. The kind decides the template, the
URL and the category:

| Folder | Template | URL | Extra front matter |
|-|-|-|-|
| `notes/` | `page.html` | `/micro/NAME` | none |
| `likes/` | `like.html` | `/likes/NAME` | `like: {url, label}` |
| `bookmarks/` | `bookmark.html` | `/bookmarks/NAME` | `bookmark: {url}` |
| `rsvps/` | `rsvp.html` | `/rsvps/NAME` | `rsvp: "yes"`, `event: {date, time, name, organizer, url}` |

Files are named `YYYY-MM-DD-name.md`, and the date of the file name is the entry's date. Common
keys: `title`, `published`, `permalink`, `redirect_from`, `syndications`, `narrow_post`.

## Pages

Any other file with front matter. A page's URL is its `permalink`, else its path without the
extension (`about.md` is `/about`, `dir/index.md` is `/dir/`). Markdown becomes `.html` and Sass
becomes `.css`. An HTML page is written as it is.

```markdown
---
layout: page
title: About
permalink: /about
redirect_from:
  - /me
---
```

| Key | What |
|-|-|
| `layout` | The template, `page` meaning `page.html`. **Without a layout the page is its content alone** |
| `title`, `author`, `hide_title`, `title_icon` | What the template shows |
| `noindex` | `true` asks the template to keep the page out of search engines, as `page.noindex`; the template writes the robots tag |
| `permalink` | The page's URL, including its extension when it has one (`/feed.xml`) |
| `redirect_from` | Old URLs: each becomes a stub that redirects to this page; all are listed in `/redirects.json` |
| `published` | `false` leaves it out |
| `cover`, `toc`, `hide_website_junk`, `corner_backlink`, `additional_body_class`, `error-type` | Presentation the templates read |

An HTML page with Liquid in it is refused ("not built yet"); an HTML page without any is copied.

## Static files

Everything else (images, scripts, fonts, a `CNAME`) is copied to the same path. Add paths that
begin with a dot to the configuration's `include`, and keep files out with `exclude`. Sass files
(`.scss` with front matter) are compiled; see [Building](building.md#stylesheets).
