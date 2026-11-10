# Getting started

This page installs Folia and builds the example site, which has one of everything. The next
pages explain each part.

## What you need

- **Linux x86_64** is the supported platform. macOS is not guaranteed, and Windows is not
  supported (use WSL); see [platforms](development.md#platforms).
- **git.** Folia reads each file's last-modified date from the site's git history, so a site is a
  git repository, and a file's date is the date of its last commit.
- **Dart Sass 1.100.0**, only if the site has Sass stylesheets. Download the standalone release
  from <https://github.com/sass/dart-sass/releases/tag/1.100.0> and put `sass` on your `PATH`
  (or set `FOLIA_SASS` to the program). Another version is refused, because the CSS Folia writes
  must be Jekyll's byte for byte.

## Install

### A release binary (Linux x86_64)

Download `folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` and its `.sha256` file from the
[releases page](https://github.com/ggirelli/folia-web/releases), then:

```sh
sha256sum -c folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz.sha256
tar xzf folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
install -m 755 folia-vX.Y.Z-x86_64-unknown-linux-gnu/folia ~/.local/bin/folia
folia --version                    # folia X.Y.Z
```

The tarballs are built by CI from the tag; to verify where one came from, run
`gh attestation verify FILE --repo ggirelli/folia-web`. Not every release has a binary yet (the
[release workflow](development/releasing.md) is new): if yours has none,
build from source.

### From source

You need the stable Rust toolchain (<https://rustup.rs>):

```sh
git clone https://github.com/ggirelli/folia-web.git
cd folia-web
cargo build --release --locked -p folia      # target/release/folia
```

To work on Folia itself, see [development](development.md) and
[manual testing](development/manual-testing.md), which lists every tool and installs them on a
fresh Ubuntu.

## Build the example site

The repository has a small, complete site in
[`examples/minimal-site`](https://github.com/ggirelli/folia-web/tree/main/examples/minimal-site).
Clone it (`git clone https://github.com/ggirelli/folia-web.git`), and from the clone:

```sh
folia build --site examples/minimal-site --dest /tmp/folia-example
```

It prints one warning, on purpose (the example has a broken wiki-link), then
`Built 18 pages, …`. The output is plain files in `/tmp/folia-example`. Serve it, so that links
like `/about` find `about.html` (the clone has the small server):

```sh
python3 scripts/serve.py /tmp/folia-example 8000       # then open http://127.0.0.1:8000/
```

`scripts/serve.py` is a small static server (standard library only) that maps clean URLs the way
a typical host does. Any static server that does the same works.

To build and serve in one step, and build again when a file changes, use
[`folia serve`](guide/building.md#previewing-while-you-write):

```sh
folia serve --site examples/minimal-site              # then open http://127.0.0.1:4000/
```

## Start your own site

A Folia site is a git repository with:

```text
my-site/
  _folia/
    folia.toml          the configuration: where things are, who writes, feeds, search
    templates/          the page templates (MiniJinja)
    search/search.js    the code of the search script, if the site has search
  _posts/               blog posts: 2026-01-10-hello.md
  _micro/{notes,likes,bookmarks,rsvps}/    microblog entries
  _garden/              garden notes, in any folders you like
  about.md, index.md    pages: any other file with front matter
  assets/               static files, copied as they are
```

The directory names `_posts`, `_micro` and `_garden` are the example's: you name them in the
configuration's `[roots]`. Copy `examples/minimal-site` and change it, rather than starting from
nothing, and keep these pages open:

- [Writing content](guide/content.md): the files and their front matter.
- [Templates](guide/templates.md): what each template can use.
- [Configuration reference](configuration.md): every key of `folia.toml`.
- [Building and deploying](guide/building.md): the command, its exit status and the destination.
