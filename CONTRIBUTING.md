# Contributing to Folia

Folia is the static-site generator behind [ggirelli.info](https://ggirelli.info). It is built for
that one site, and it is [fully vibe coded](DISCLAIMER.md).

## Getting started

You need [just](https://just.systems), a Rust toolchain with `cargo-deny`,
[uv](https://docs.astral.sh/uv/) (for the docs, shellcheck and Ruff) and Node.js (for
markdownlint); that is all `just check` needs. Ruby with Bundler (for StandardRB and the Jekyll
reference builds) is needed only for `just check-parity` and the Ruby recipes. The parity recipes
also need a checkout of the reference Jekyll site (`FOLIA_SITE`) and its owner's private inputs (see
[A site's private inputs](docs/development.md#a-sites-private-inputs)).
The linters are pinned in the justfile and fetched on first use; nothing needs installing globally.

```sh
just          # list the recipes
just check    # run every check
just check-parity    # the parity-only checks (need Ruby)
just docs     # preview the docs
```

[Manual testing](docs/development/manual-testing.md) shows how to build the public example site and
check it in a browser, with no private inputs.

## Making a change

1. Keep changes focused: one feature or fix per change, split into small commits that each pass
   the checks.
2. Run `just check`, and `just check-parity` too when the change touches `crates/parity`, `parity/`,
   the parity scripts or the Ruby files. CI runs the same recipes on every pull request, and `main`
   only takes changes that pass them.
3. Add or update tests and checks for what you changed.
4. Update the docs that describe it: the guide in [docs/](docs/) (what a user does) or
   [docs/development.md](docs/development.md) (how it is built), and [README.md](README.md) for
   the overview.

## Style

### Code

- Prefer clear names over comments, and comment the *why* when it is not obvious.
- Write code a reader can follow from top to bottom: small functions named after what they do, the
  main path first and the details after it, no clever shortcuts where a plain loop or `match`
  reads better.
- Match the code around you: its naming, its idioms and how much it comments.
- Every file opens with a comment saying what it is for and, for scripts, how to run it.
- Rust code is formatted with `cargo fmt`, and passes `cargo clippy` (with `clippy::pedantic`)
  and `cargo deny` with no warnings. Every public item has a doc comment, and every module opens
  with a `//!` comment. Public functions that can fail say when in an `# Errors` section, and
  library code returns errors rather than panicking. `just fmt` formats it.
- Shell scripts pass `shellcheck` with no findings, use `set -euo pipefail`, and quote their
  variables.
- Markdown files pass `markdownlint` with no findings (`.markdownlint-cli2.yaml`): prose wraps at
  100 columns, and nested lists are indented by 2 spaces. `just fix-md` fixes what it can.
- Ruby passes StandardRB (pinned in the `Gemfile`) with no findings. `just fmt-rb` formats it;
  CI only checks, it never rewrites code.
- Python is standard library only, runs with `python3 -I`, and passes Ruff (pinned in the
  justfile, configured in `ruff.toml`) with no findings. `just fmt-py` formats it.

### Modules

- A source file stays under 1,000 lines; `just check` fails on longer ones. Tests may live in a
  separate file.
- Split code by subject, not by kind: a module holds one topic (routes, feeds, wiki-links) with its
  types and functions together, so a reader can find code by what it is about.
- When a module grows too big, turn it into a folder: `feeds.rs` becomes `feeds/mod.rs` with
  sibling modules for its topics, re-exporting what used to be public so callers keep their paths.
- Code that moves keeps its shape: a change that moves code does not also change it, so the diff
  (`git diff --color-moved`) shows that nothing else changed.

### Tests

- Unit tests live next to the code they test. Small modules keep them in a `#[cfg(test)] mod tests`
  at the end; big ones put them in a sibling file (`mod tests;` → `tests.rs`, or a `tests/` folder
  split by subject).
- Test names are sentences describing the behavior (`an_index_note_keeps_index_in_its_url`).

### Recipes

- Every task has a `just` recipe, in a `[group(...)]` (`parity`, `checks`, `docs`, …), with a
  one-line description that starts with a verb ("Build a Jekyll baseline of the reference site").
- Recipes stay short; logic longer than a few lines goes into a script in `scripts/`.

### Words and commits

- Messages shown to people are short, plain sentences.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `docs:`…), with a body explaining what changed and why.

By contributing, you agree that your contributions are licensed under the [MIT license](LICENSE).
