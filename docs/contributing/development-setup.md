# Development setup

The conventions are in
[CONTRIBUTING.md](https://github.com/ggirelli/folia-web/blob/main/CONTRIBUTING.md). Run `just` to
list the recipes and `just check` before every commit. [Manual testing](manual-testing.md)
explains how to build the example site and check it in a browser. How the code is organized is in
[Architecture](../internals/architecture.md).

## The repository

- **`crates/`**: the Cargo workspace (see [Architecture](../internals/architecture.md#the-crates)).
- **`scripts/`**: the scripts behind the `just` recipes. `check-file-length.sh` enforces the
  1,000-line limit, and `serve.py` serves a built site locally with a host's clean URLs
  (`just serve`). The others serve the
  [parity tooling](../internals/parity-governance.md#the-parity-tooling).
- **`.github/`**: the CI and docs workflows, the pull request template, and the Dependabot
  configuration.
- **`docs/`**: this documentation.
- **`examples/minimal-site/`**: a small public site with every kind of content Folia builds, for
  manual testing (`just build-example`) and its integration test (`crates/folia/tests`).
- **`assets/brand/`**: logos and the banner.

The tools you need are listed in [manual testing](manual-testing.md#prerequisites).

## Platforms

- **Linux** (Ubuntu 24.04) is the supported platform: CI runs there, and the
  [install commands](manual-testing.md#installing-them-on-a-fresh-ubuntu-2404) were run
  on it from nothing.
- **macOS** is informational: the whole workspace, tests included, compiles for it
  (`cargo check --workspace --all-targets --target aarch64-apple-darwin`), and CI runs the tests
  there without making them required. Treat a pass as a good sign, not a guarantee.
- **Windows** is not supported. The libraries and the `folia` binary compile for it
  (`cargo check --workspace --target x86_64-pc-windows-gnu`), but nothing has been run there; the
  tests use Unix-only APIs (`std::os::unix`, file modes), `cp` and `/dev/null`, and the `just`
  recipes call `bash`. Use WSL.

## Checks

`just check` runs every check that needs Rust, uv and Node; `just check-parity` runs the
parity-only ones, which need Ruby, and is required when a change touches `crates/parity`,
`parity/`, the parity scripts or the Ruby files. CI runs the same recipes, in four jobs of `.github/workflows/ci.yml`
and one of `.github/workflows/docs.yml`, which also publishes the docs to
[GitHub Pages](https://ggirelli.github.io/folia-web/) from `main`:

| Job | Recipes |
|-|-|
| `rust` | `fmt-check`, `clippy` (pedantic, warnings are errors), `test`, `doc` |
| `rust-macos` | `cargo test --workspace` on macOS (informational: `continue-on-error`, not required) |
| `deny` | `cargo deny check` (advisories, licences, duplicate crates, sources) |
| `lint` | `file-length`, `lint-sh` (shellcheck), `lint-md` (markdownlint), `lint-py` (Ruff) |
| `parity` | `check-parity`: `lint-rb` (StandardRB), `baseline-negative`, `accept-negative` |
| `docs` | `docs-build` (MkDocs, strict) |
