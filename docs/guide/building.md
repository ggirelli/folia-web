# Building and deploying

## The command

```sh
folia build --site ../my-site --dest ../my-site-out
```

| Option | Default | What |
|-|-|-|
| `--site` | `.` | The site: a git repository |
| `--config` | `<site>/_folia/folia.toml` | The configuration ([reference](../configuration.md)) |
| `--dest` | required | Where to write the site |

`folia --version` prints the version, and `folia build --help` the options. Folia has one
command: there is no `serve` and no watch mode yet
([#46](https://github.com/ggirelli/folia-web/issues/46)), and no `--drafts` or `--future`
([#47](https://github.com/ggirelli/folia-web/issues/47)). A post dated after the build, or with
`published: false`, is not built.

While it builds, Folia prints **warnings** (a broken wiki-link) and `skipped:` lines for any
document it does not support yet, each with the reason, then `Built N pages, …`.

## Exit status

Scripts can rely on it:

| Status | Meaning | Output |
|-|-|-|
| `0` | The site was built into `--dest`. Warnings and `skipped:` documents do not change it | the warnings and skips on stderr, then `Built …` on stdout |
| `1` | The build failed: planning, a missing or failing template, a render error, an unknown author, a destination Folia refuses, or a write error. **Nothing was written**; `--dest` is as it was | every problem on stderr, then `The build failed with N problems; nothing was written, …` |
| `2` | Usage or configuration error: an unknown subcommand, a missing `--dest`, a configuration that cannot be read or has an unknown key | the error on stderr |

A run that exits 0 may have skipped documents. A script that must not publish a partial site
should check stderr for `skipped:`.

## The destination

`--dest` must be outside the site. Folia writes the new site beside it, in
`.<name>.folia-new`, and swaps it in only when everything was written, so **a failed build never
touches the destination**. It deletes nothing it did not write:

- an absent or empty destination is written;
- a non-empty one is replaced only if its record, `.<name>.folia-output` beside it (not in it, so
  it is never deployed), names that directory and lists every top-level entry it holds. Your own
  files, a moved copy or any other directory are refused, with nothing changed;
- a leftover `.<name>.folia-new` or `.<name>.folia-old` is refused, never deleted.

To build into a directory that holds something else, empty it yourself.

## Stylesheets

A `.scss` file with front matter is compiled to CSS (with its source map) when the whole site is
built. Folia compiles it with **Dart Sass 1.100.0 exactly**: `sass` on `PATH`, or the program
named by `FOLIA_SASS`. Another version is refused, and a site without Sass does not need it. The
`[sass]` table sets the import directory (`dir`, default `_sass`) and the output `style`
(`expanded` or `compressed`).

## Dates come from git

A page's last-modified date (the `<lastmod>` of the sitemap, `page.modified`) comes from its last
commit. In a CI checkout, fetch the full history (`fetch-depth: 0` in GitHub Actions): a shallow
clone gives every file the same date.

## Serving locally

The output is plain files, but its links are root-relative (`/about`), so opening the files
directly shows an unstyled site with broken links. Serve the directory with a static server that
maps clean URLs the way your host does: the repository's `scripts/serve.py DIR PORT` does, using
only Python's standard library.

## Deploying

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

Add `--site` and `--config` if the site is not the checkout's root or has its configuration
elsewhere. Rehearse a first deployment somewhere that is not production, and keep a way back (see
[Migrating from Jekyll](../migrating/index.md#deploy-and-roll-back)).
