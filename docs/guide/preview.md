# Preview while you write

```sh
folia serve
```

Folia builds the site, serves it at <http://127.0.0.1:4000/> and builds it again whenever a file
of the site changes. A page open in the browser reloads when the new build is ready. Stop it with
Ctrl+C. The options (`--host`, `--port`, `--dest` and the rest) are in the
[command-line reference](../reference/cli.md#folia-serve).

## What to expect

- **Every change builds the whole site**, with the configuration read again and `site.url` set to
  the local address, so that links stay on the local server. The output therefore differs from
  production's in that URL: `folia build` is still what you deploy.
- **Each rebuild is stamped with the time** in the terminal, in your local time zone:
  `[14:03:22] Changed: _posts/hello.md; building again`.
- **A failed build keeps the last good one.** The problems are printed in the terminal and shown
  over the open pages, which clear when a build succeeds.
- **Only what changed is written.** `folia build` replaces its destination whole, but
  `folia serve` updates its destination in place: it writes the files whose bytes changed and
  removes those the build no longer makes, so a rebuild costs what it changed even on a slow disk.
  Each file is replaced whole, so a page is never served half-written; a disk error partway leaves
  the site partly updated until the next build.
- **URLs** map to files as a typical host does: `/about` is `about.html`, `/blog/` is
  `blog/index.html`, and a missing page shows the site's `404.html`.
- **A commit alone starts nothing**, though a page's last-modified date comes from its last
  commit: change a file to build again.

## The destination

By default `folia serve` builds into `folia/serve/<site name>-<hash>` in your cache directory
(`$XDG_CACHE_HOME`, or `~/.cache`; `~/Library/Caches` on macOS), and keeps it between runs. A
`--dest` follows the rules of [the destination](../getting-started/build-and-deploy.md#the-destination),
checked before anything is served, with two differences:

- `folia serve` refuses a destination `folia build` wrote, which you may be about to deploy: the
  destination's record says which command wrote it. `folia build` may still replace what
  `folia serve` wrote.
- In a destination `folia serve` wrote, it recovers what a stopped serve left beside it
  (`.<name>.folia-new`, `.<name>.folia-old`) instead of refusing it.

The destination itself may not be a symbolic link.

## What is watched

The whole site, and the configuration's directory if it is outside the site. A change to what the
build skips starts no build: the destination and the `.folia-*` files beside it, names starting
with `.` (`.git`, editors' swap files) unless `include` names them, paths `exclude` names (except
the templates, the Sass directory and the search fragment, which the build reads anyway), and
editors' backup files (`name~`, `#name#`, Vim's `4913`). Changes are gathered: a build starts after
200 ms without one, and at most 2 s after the first.

## Live reload

A script is added to the HTML as it is served, so the files on disk do not have it. The page
listens to server-sent events at `/__folia/events`.

## Drafts and scheduled posts

`folia serve --unpublished --future` shows them; see [Drafts and scheduling](drafts.md). Unlike
`folia build`, `folia serve` may switch its destination between a normal and a preview build.
