# Install

Folia is one program, `folia`. This page gets it onto your machine; the next one,
[Your first site](first-site.md), uses it.

## What you need

- **Linux x86_64** is the supported platform. macOS is not guaranteed, and Windows is not
  supported (use WSL); see [platforms](../development.md#platforms).
- **git.** Folia reads each file's last-modified date from the site's git history, so a site is a
  git repository, and a file's date is the date of its last commit. A site outside a git
  repository fails to build.
- **Dart Sass 1.100.0**, only if the site has Sass stylesheets. Download the standalone release
  from <https://github.com/sass/dart-sass/releases/tag/1.100.0> and put `sass` on your `PATH`
  (or set `FOLIA_SASS` to the program). Another version is refused, because the CSS Folia writes
  must be Jekyll's byte for byte.
- **A Rust toolchain** (stable, from <https://rustup.rs>), only to install from source.

## A release binary (Linux x86_64)

Download `folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` and its `.sha256` file from the
[releases page](https://github.com/ggirelli/folia-web/releases), then:

```sh
sha256sum -c folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz.sha256
tar xzf folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
install -m 755 folia-vX.Y.Z-x86_64-unknown-linux-gnu/folia ~/.local/bin/folia
folia --version                    # folia X.Y.Z
```

The release is also attested: verify the file with `gh attestation verify FILE --repo
ggirelli/folia-web`. Not every release has a binary yet (the
[release workflow](../development/releasing.md) is new): if yours has none, build from source.

## With cargo

Folia is not on crates.io, but cargo can build it straight from the repository:

```sh
cargo install --git https://github.com/ggirelli/folia-web folia --locked
```

This puts `folia` in `~/.cargo/bin`.

## From a clone

```sh
git clone https://github.com/ggirelli/folia-web.git
cd folia-web
cargo build --release --locked -p folia      # target/release/folia
```

The clone also has the example site you will use next, in `examples/minimal-site`. To work on
Folia itself, see [development](../development.md) and
[manual testing](../development/manual-testing.md), which lists every tool and installs them on a
fresh Ubuntu.

Next: [Your first site](first-site.md).
