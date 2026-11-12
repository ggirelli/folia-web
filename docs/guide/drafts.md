# Drafts and scheduling

A normal build leaves out two things: a document with `published: false` in its front matter (a
draft), and a post, garden note or micro entry dated after the build (scheduled content). Two
flags build them, so you can look before they are published. Both default to off, and they are
command-line flags on `folia build` and `folia serve`, not configuration keys.

```sh
folia build --site ../my-site --dest ../my-site-preview --future --unpublished
folia serve --future --unpublished
```

- **`--unpublished`** builds the documents whose front matter says `published: false`: posts,
  garden notes, micro entries and pages (and stylesheets). A draft post is written like any post,
  in the posts directory with a dated file name, and gets its date from the name. A draft that is
  also dated after the build needs `--future` as well.
- **`--future`** builds the posts, garden notes and micro entries dated after the build.

## Preview builds

A build with either flag is a **preview build**, and Folia says so in three places:

- the summary line ends with `PREVIEW BUILD (future, unpublished): not for publishing.`, naming
  the flags that were set;
- a `warning:` line on stderr says the same;
- the record of the destination, `.<name>.folia-output`, gets a `/folia future` and/or
  `/folia unpublished` line.

The record is how `folia build` protects you: it never replaces a normal build with a preview, nor
a preview with a normal build. It refuses, and nothing is changed. Build the preview into another
directory, or remove the old one yourself. `folia serve` is not bound by this rule. Keep a preview
out of production: deploy only what a normal `folia build` wrote.

## Building as of a moment

A build takes the time of the build from the clock, and a post dated after it is not built. `--at`
sets that moment instead, so that the same sources give the same site on any day.

```sh
folia build --site ../my-site --dest ../my-site-out --at 2026-01-01T00:00:00Z
```

The value is an RFC 3339 timestamp with a time and an offset (`Z` or `+02:00`); anything else is
an error, and nothing is built. It is not a preview: documents dated after it are held back, as
with the clock, unless `--future` is set. `folia serve` has no `--at`, because it builds again on
every change and each build is as of its own moment.

## If you used Jekyll's `_drafts`

Folia does not read a `_drafts` directory and has no `--drafts` flag. Put the draft in the posts
directory with a dated file name and `published: false`, and see it with `--unpublished`. The
reasons are in [Differences from Jekyll](../internals/differences-from-jekyll.md#drafts).
