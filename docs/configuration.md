# Configuration reference

A site's configuration is one TOML file, `_folia/folia.toml` by default (`--config` names another).
Every key is listed here, with what it is, its type and its default. **A key Folia does not know is
an error**, and so is a required key that is missing, so a misspelling never silently changes a
build. The [full example](#full-example) at the end is parsed by a test, so it cannot name a key that
does not exist; the public
[example site](https://github.com/ggirelli/folia-web/tree/main/examples/minimal-site/_folia/folia.toml)
is a smaller, working one.

Paths are relative to the site root. Names are kebab-case (`time-zone`, `display-name`).

## Top level

| Key | Type | Default | What |
|-|-|-|-|
| `time-zone` | string, **required** | | The zone dates are read in (`Europe/Stockholm`): file-name dates are midnight in it, and the build time is shown in it |
| `include` | list of paths | `[]` | Paths published although their name would hide them (`.htaccess`) |
| `exclude` | list of globs | `[]` | Paths that are not part of the site. `*` stays within one path segment; a pattern matching a directory excludes all of it |
| `templates-dir` | string | `_folia/templates` | The directory of the MiniJinja templates |
| `slice` | table | none | Build only the documents it names (see below). Without it the whole site is built |

## `[roots]` (required)

The directory of each mode's documents. Every other file with front matter is a page.

| Key | Type | What |
|-|-|-|
| `blog` | string, **required** | Posts, such as `_posts` (named `YYYY-MM-DD-slug.md`) |
| `micro` | string, **required** | Micro entries, such as `_micro`, in `notes/`, `likes/`, `bookmarks/` and `rsvps/` |
| `garden` | string, **required** | Garden notes, such as `_digital_garden` |

## `[site]` (required)

| Key | Type | What |
|-|-|-|
| `url` | string, **required** | Where the site is published, with no trailing `/` (`https://example.org`). Redirect stubs, feeds, the sitemap and search records use it |
| `title` | string, **required** | The site's name, as templates show it (`site.title`) |
| `description` | string, **required** | Its tagline (`site.description`) |

## `[authors.<key>]`

The key is what a document's front matter `author` names; a page naming an author that is not here
fails the build. Templates see each as `site.authors.<key>`. Fields left out are left out; **a
field set to an empty value is an error**.

| Key | Type | What |
|-|-|-|
| `name`, `display-name`, `avatar`, `web`, `description` | string, **required** | Full name; the name in a post's author box; the picture; their web page; a line about them |
| `nickname`, `url`, `default-text`, `blog`, `garden` | string | A short name; the URL the author page names as theirs; the picture's hover text; where their blog and garden are |
| `covers` | list of tables | The author page's backgrounds, in order: `[[authors.<key>.covers]]` with `path` (**required**), `placeholder`, `description` (HTML) and `position` (CSS `background-position`). Templates also get `covers_json` |
| `email`, `keybase`, `keyoxide`, `kofi`, `github`, `radicle`, `codeberg`, `hacker-news`, `lobste-rs`, `facebook`, `mastodon`, `bluesky`, `bluesky-did`, `twitter`, `reddit`, `substack`, `instagram`, `unsplash`, `linkedin`, `scholar`, `orcid` | string | Their address or account on each service, as the site's buttons put it in a link |

## `[micro]`

| Key | Type | Default | What |
|-|-|-|-|
| `author` | string | none | The author of every micro entry: a key of `[authors]` |
| `listings` | list of tables | `[]` | Pages that list micro entries: `[[micro.listings]]` with `source` (the page, **required**), `template` (**required**) and `kinds` (**required**: any of `notes`, `likes`, `bookmarks`, `rsvps`). The page keeps its route and front matter; its body is not rendered; the template gets `page.items`, newest first |

## `[blog]`

Each listing is built only when configured. A claimed page (`{ source, template }`) keeps its route
and front matter, and its body is not rendered.

| Key | Type | What |
|-|-|-|
| `index` | `{ source, template }` | The blog index: `/blog/` and `/blog/pageN/`, nine posts a page, the first with the featured ones |
| `archive-template` | string | The template of the category archives (`/blog/category/:name/`) |
| `categories` | `{ source, template }` | The page grouping posts by category (`page.groups`) |
| `tags` | `{ source, template }` | The page grouping posts by tag (`page.groups`) |

## `[garden]`

| Key | Type | What |
|-|-|-|
| `author` | string | The author of every note: a key of `[authors]`; the garden's feed names them |

## `[feeds]`

Without it, no feed is written.

| Key | Type | Default | What |
|-|-|-|-|
| `lang` | string | none | The `xml:lang` the feeds declare |
| `generator` | `{ name, uri, version }`, **required** | | What the feeds say generated them (all three **required**) |
| `limit` | integer | `10` | Items per feed |
| `combined` | `{ source, collections }` | none | One feed of several collections, at the page `source` (such as `feed.xml`); `collections` is a list of `posts`, `digital_garden` and `micro` |

The `generator` table has `name` (the text of `<generator>`), `uri` (its home page) and `version`,
all required: `jekyll-feed` wrote `Jekyll`, `https://jekyllrb.com/` and a version, so a site
keeping its feeds' bytes keeps them.

## `[search]`

Without it, no search script is written.

| Key | Type | What |
|-|-|-|
| `source` | string, **required** | The script's page, which search claims (`assets/js/search.js`) |
| `fragment` | string, **required** | The script's code after the `documents` array, read from the site and written as it is |
| `content-block` | string | The template block that holds a page's content. A listing is indexed by that block's text without the navigation and footer; without it, by the whole rendered page |

## `[redirects]`

How the redirect stubs (one per `redirect_from` URL) look for the moment they show. Without it, a
stub is what `jekyll-redirect-from` wrote, byte for byte, and the browser paints its own white
background until the target loads: on a dark site, a white flash.

| Key | Type | What |
|-|-|-|
| `background` | string | A CSS colour (`#1d1f21`, `rgb(29, 31, 33)`), painted from the stub's first frame |
| `color-scheme` | string | The stub's colour scheme: one or more of `normal`, `light`, `dark` and `only` (`dark`, `light dark`) |

Each is written into every stub as it is, so a value that is not a colour or a colour scheme is
refused.

## `[sass]`

| Key | Type | Default | What |
|-|-|-|-|
| `dir` | string | `_sass` | The directory Sass imports are found in |
| `style` | `expanded` or `compressed` | `expanded` | The output style |

Stylesheets are compiled only by a build of the whole site, with **Dart Sass exactly 1.100.0**
(`sass` on `PATH`, or `FOLIA_SASS`); a site without Sass stylesheets does not need it.

## `[slice]`

Only for building part of a site (parity work): the documents to build, each of which must build.

| Key | Type | What |
|-|-|-|
| `pages`, `posts`, `notes`, `micro` | list of source paths | The documents of each mode to build |

## Command-line flags

These are flags, not configuration keys: a `future`, `unpublished` or `at` key in `folia.toml` is an
unknown key, and an error. `--future` and `--unpublished` default to off, and `--at` to now.

| Flag | Commands | What |
|-|-|-|
| `--future` | `build`, `serve` | Also build posts, garden notes and micro entries dated after the build |
| `--unpublished` | `build`, `serve` | Also build documents whose front matter says `published: false` |
| `--at <TIME>` | `build` | Build as of an RFC 3339 timestamp, such as `2026-01-01T00:00:00Z`, instead of now |

A build with `--future` or `--unpublished` is a preview, not for publishing. See
[previewing drafts and scheduled posts](guide/building.md#previewing-drafts-and-scheduled-posts).

## Environment

| Variable | What |
|-|-|
| `FOLIA_SASS` | The Dart Sass program, when it is not `sass` on `PATH` |

## Full example

Every key above, in one file. A test parses it with Folia's own configuration reader.

```toml
# full-example
time-zone = "Europe/Stockholm"
include = [".htaccess"]
exclude = ["README.md", "vendor"]
templates-dir = "_folia/templates"

[roots]
blog = "_posts"
micro = "_micro"
garden = "_garden"

[site]
url = "https://example.org"
title = "Example"
description = "A site."

[authors.ada]
name = "Ada Example"
display-name = "Ada"
nickname = "ada"
avatar = "assets/images/avatar.svg"
web = "/about"
url = "https://example.org/"
description = "Writes the site."
default-text = "Hello"
blog = "/blog"
garden = "/dg"
email = "ada@example.org"
github = "ada"
mastodon = "example.social/@ada"
bluesky = "ada.example.org"

[[authors.ada.covers]]
path = "/assets/images/cover.webp"
placeholder = "/assets/images/cover-small.webp"
description = "A <a href=\"https://example.org\">lake</a>"
position = "50% 40%"

[micro]
author = "ada"

[[micro.listings]]
source = "blog/micro.html"
template = "micro_index.html"
kinds = ["notes", "likes", "bookmarks", "rsvps"]

[garden]
author = "ada"

[redirects]
background = "#1d1f21"
color-scheme = "dark"

[blog]
index = { source = "blog/index.html", template = "blog_index.html" }
archive-template = "archive.html"
categories = { source = "blog/categories.html", template = "blog_categories.html" }
tags = { source = "blog/tags.html", template = "blog_tags.html" }

[feeds]
lang = "en"
generator = { name = "Folia", uri = "https://github.com/ggirelli/folia-web", version = "0.2.0" }
limit = 10
combined = { source = "feed.xml", collections = ["posts", "digital_garden", "micro"] }

[search]
source = "assets/js/search.js"
fragment = "_folia/search/search.js"
content-block = "body"

[sass]
dir = "_sass"
style = "compressed"

[slice]
pages = ["about.md"]
posts = ["_posts/2026-01-10-hello-folia.md"]
notes = ["_garden/topic/index.md"]
micro = ["_micro/notes/2026-01-12-a-note.md"]
```
