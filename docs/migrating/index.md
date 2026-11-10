# Migrating from Jekyll

Folia was written to replace one Jekyll site, and this page is the exercise that was done for it,
for anyone who wants to do the same. **None of it is needed to use Folia**: a site that starts on
Folia needs only the [guide](../getting-started.md).

Read [Known limitations](../known-limitations.md) first. Folia is not a drop-in Jekyll
replacement: it builds four kinds of content, in the way one site uses them, and it refuses
what it does not support (an unknown front matter key, Liquid in an HTML page) instead of guessing.
**Matching Jekyll's output byte for byte is not the goal, and is not required**: the goal is to know,
for every output, whether it is identical, differs for a reason you accept, or is a defect.

## The steps

1. **Check that Folia can build your content.** Does the site use only what it supports? Posts
   dated by file name, garden notes with a title and a date, micro entries in the four kinds,
   pages with the front matter [listed here](../guide/content.md)? Anything else is refused,
   naming the file. Move **Liquid out of your Markdown** (`{{` and `{%` are shown as written);
   `just liquid` finds what is left.
2. **Write Folia's side of the site** in `_folia/`: `folia.toml` (start from the
   [reference](../configuration.md)) and the [templates](../guide/templates.md). Layouts and
   includes written in Liquid become MiniJinja templates, which is most of the work. Build
   until it passes: `folia build --site . --dest ../out`. Every problem is named by page and
   template.
3. **Look at it.** [Serve the site](../guide/building.md#previewing-while-you-write) with
   `folia serve` (or [a finished build](../guide/building.md#serving-a-finished-build)) next to
   Jekyll's (`bundle exec jekyll serve`, on another port) and click through both: layout, links,
   images, small screens, dark mode, anything that animates. Most differences you will care about
   are visible here.
4. **Record a baseline and let the tools compare** (needs a clone of this repository; see the
   [parity tools reference](parity-reference.md)):
   1. `just baseline my-label` records a production Jekyll build, with Jekyll's own intermediate
      results, of one commit of your site.
   2. `just routes`, `just markdown` and `just pages` compare every output Folia writes with
      Jekyll's. Each difference is classified: identical, accepted (in your ledger), intentional,
      known, or a defect.
   3. Fix defects. Decide, one by one, which differences you accept, and write each as an entry in
      the ledger, with its reason.
   4. `just determinism` builds twice and checks that the builds are byte-identical.
5. **Check what the tools cannot.** They compare files, not what reads them. Subscribe to the
   feeds in a feed reader and compare each feed's entry ids (they should not change, or readers
   show every entry as new); try the search; validate the sitemap; and open the output on a real
   host (below).
6. **Deploy, and keep a way back.**

## Sites with a `_drafts` directory

Folia does not read `_drafts`. Move each draft into the posts directory with a dated file name
(`_posts/2026/2026-11-10-idea.md`) and add `published: false` to its front matter. Preview it with
`folia serve --unpublished`; remove the line when the post is ready. See the
[known limitations](../known-limitations.md#drafts).

## Reading the result

`just pages` ends with two verdicts that are never the same thing:

- **Clean build:** every output is accounted for, and none is a defect or lacks evidence.
- **Exact parity:** every output is identical to Jekyll's.

A migration needs the first. The second is rarely true and is not a requirement: Folia's own
[known limitations](../known-limitations.md) lists differences that are on purpose (a feed's
`<updated>` is its newest item's date, for example). Treat "zero failed assertions" as "nothing
failed unexplained", and "documented difference" as "still a difference you decided to accept".

## Deploy and roll back

Do the first deployment where nobody depends on it. Whatever the host, check:

- the home page, with styles and images, and an old URL that redirects;
- a page without an extension (`/about`) and a made-up URL (it should show your 404 page);
- the feeds, the sitemap and `robots.txt` load with sensible content types;
- search works on the real host;
- HTTPS, and the certificate.

Cutting over a live site is the step with real visitors, so decide the rollback **before** you
start and test it once:

- Keep the Jekyll deploy workflow and its last good run. If the host is GitHub Pages, switching
  the source back and re-running that workflow restores the old site in minutes.
- A CDN in front of the host (Cloudflare, for example) keeps serving cached pages: purge it after
  each switch, and test in a private window.
- Feed readers and search engines look at the site during the switch. Keeping feed ids and URLs
  the same is what makes it invisible to them.

You can run the old and the new site side by side as long as you like. Nothing about Folia forces
the date of the switch.
