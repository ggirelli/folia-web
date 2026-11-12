# Feeds, sitemap and search

Folia writes these outputs itself, from what it built, without a template. Each is optional or
automatic as noted.

## Redirects

A page's `redirect_from` makes a stub page at each old URL that redirects to the new one, and all
of them are listed in `/redirects.json`. The stub's target uses the configuration's `[site] url`.

A stub refreshes to its target at once, and names it as its canonical URL. On a dark site, give the
stubs the site's colours with [`[redirects]`](../reference/configuration.md#redirects), so that
following an old URL does not flash a white page:

```toml
[redirects]
background = "#1d1f21"
color-scheme = "dark"
```

## Atom feeds

Written only when the configuration has [`[feeds]`](../reference/configuration.md#feeds):

| Feed | URL |
|-|-|
| the blog | `/blog/feed.xml` |
| each tag | `/blog/tag/TAG.xml` (a tag with a space too: `/blog/tag/stem cells.xml`, linked percent-encoded) |
| the microblog, and each kind | `/micro/feed.xml`, `/feed/micro/KIND.xml` |
| the garden | `/dg/feed.xml` |
| combined (optional) | the page you name in `[feeds.combined]`, such as `/feed.xml` |

Each lists the newest `limit` items (10 by default). A feed's `<updated>` is its **newest item's
date, never the build time**, so rebuilding does not make every feed look new. There are no
category feeds. The `generator` table decides what the feeds say generated them.

## Sitemap

`/sitemap.xml` lists every page by URL except `/404.html` (redirect stubs are not pages), then the
`.htm`, `.html`, `.xhtml` and `.pdf` static files. A page's `<lastmod>` is its last modification; a
page with no source, such as a category archive, has none. A static file's is its last git commit.

## robots.txt

Folia writes a default `/robots.txt` that lets every crawler fetch everything and points to the
sitemap, using the configuration's `[site] url`:

```text
User-agent: *
Allow: /
Sitemap: https://example.org/sitemap.xml
```

To write your own, put a `robots.txt` at the root of the site. A file of the site takes the URL,
so it is copied as it is and Folia writes no default. A [preview build](drafts.md#preview-builds)
gets the same default; keep it off the public web by where you put it.

## Search

Written only when the configuration has [`[search]`](../reference/configuration.md#search). Folia writes
`documents`, the array of records that the [lunr](https://lunrjs.com) search needs
(`{id, url, title, body}`), followed by your own script, the `fragment` file, as it is. So the
search UI and the lunr setup are yours; Folia supplies the data.

- Records: the site's pages by URL (not `.xml`, `assets`, `category` or `tag` URLs), then the
  RSVPs, likes and bookmarks, the garden notes and the posts, oldest first. A micro entry's body
  starts with its date.
- A body is the content's text: markup removed, a space after each full stop, and a line break is
  a space.
- A listing page is indexed by the text of the template block `content-block` names, so the
  navigation and footer are not in the index. Without `content-block`, the whole rendered page is.

The script ends up at `[search] source` (`assets/js/search.js` in the example).
