# Build and deploy

`folia serve` is for writing; `folia build` is for publishing. It writes the whole site into a
directory of plain files, which you upload to any static host.

```sh
folia build --site ../my-site --dest ../my-site-out
```

`--site` defaults to the current directory, so inside the site `folia build --dest ../out` is
enough. The options, the summary line and the exit status are in the
[command-line reference](../reference/cli.md).

## The destination

`--dest` must be outside the site. Folia writes the new site beside it, in
`.<name>.folia-new`, and swaps it in only when everything was written, so **a failed build never
touches the destination**. It also deletes nothing it did not write:

- an absent or empty destination is written;
- a non-empty one is replaced only if its record, `.<name>.folia-output` beside it (not in it, so
  it is never deployed), names that directory and lists every top-level entry it holds. Your own
  files, a moved copy or any other directory are refused, with nothing changed;
- a leftover `.<name>.folia-new` or `.<name>.folia-old` is refused, never deleted.

To build into a directory that holds something else, empty it yourself. A [preview
build](../guide/drafts.md#preview-builds) has one more rule, covered with drafts.

## Dates come from git

A page's last-modified date (the `<lastmod>` of the sitemap, `page.modified`) comes from its last
commit, so the site must be a git repository, and a site outside one fails to build. Uncommitted
edits do not move a date. In a CI checkout, fetch the full history (`fetch-depth: 0` in GitHub
Actions): a shallow clone gives every file the same date.

## Stylesheets

A `.scss` file with front matter is compiled to CSS (with its source map) when the whole site is
built. Folia compiles it with **Dart Sass 1.100.0 exactly**: `sass` on `PATH`, or the program
named by `FOLIA_SASS`. Another version is refused, and a site without Sass does not need it. The
[`[sass]`](../reference/configuration.md#sass) table sets the import directory and the output
style.

## Check the result

The output is plain files, but its links are root-relative (`/about`), so opening them directly
shows an unstyled site with broken links. Serve the directory with a static server that maps clean
URLs (`/about` to `about.html`) the way your host does. If you have a clone of this repository,
`scripts/serve.py DIR PORT` (or `just serve DIR`) does, using only Python's standard library. To
look at your site while you write, use [`folia serve`](../guide/preview.md) instead.

## Deploy

Upload `--dest` to any static host. Folia copies a `CNAME` file like any static file, and writes a
`404.html` if the site has one.

A GitHub Pages workflow that builds with a release binary looks like this (adapt the paths, and pin
actions by commit):

```yaml
- uses: actions/checkout@v5
  with:
    fetch-depth: 0                  # git history: the files' dates
- name: Install Dart Sass 1.100.0
  run: |
    curl -sSL https://github.com/sass/dart-sass/releases/download/1.100.0/dart-sass-1.100.0-linux-x64.tar.gz | tar xz
    echo "$PWD/dart-sass" >> "$GITHUB_PATH"
- name: Install Folia
  run: |
    base=https://github.com/ggirelli/folia-web/releases/download/vX.Y.Z
    file=folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
    curl -sSLO "$base/$file" && curl -sSLO "$base/$file.sha256"
    sha256sum -c "$file.sha256" && tar xzf "$file"
    echo "$PWD/${file%.tar.gz}" >> "$GITHUB_PATH"
- run: folia build --dest _out
- uses: actions/upload-pages-artifact@v5
  with:
    path: _out
```

Drop the Sass step if the site has no stylesheets. Add `--site` and `--config` if the site is not
the checkout's root or has its configuration elsewhere. Rehearse a first deployment somewhere that
is not production, and keep a way back (see
[Migrating from Jekyll](../guide/migrating.md#deploy-and-roll-back)).

Next: [Writing content](../guide/content.md).
