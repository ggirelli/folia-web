# docs/requirements.txt pins MkDocs below 2.0, so Material's banner warning about MkDocs 2.0 is noise
export NO_MKDOCS_2_WARNING := "true"

# Pinned linters, run through uvx and npx so nothing needs installing globally
shellcheck := "uvx --from shellcheck-py==0.11.0.1 shellcheck"
markdownlint := "npx --yes markdownlint-cli2@0.18.1"
ruff := "uvx ruff@0.17.2"
# StandardRB is pinned in the Gemfile, and installed into ./vendor/bundle
bundle := "BUNDLE_PATH=vendor/bundle bundle"

# The reference Jekyll site, and where baselines go (outside both repositories)
site := env("FOLIA_SITE", "../filopoe")
baselines := env("FOLIA_BASELINES", "../folia-baselines")
# The tree hash of the templates the pages check expects in the site's _folia/templates: filopoe's
# `folia/templates` at TEMPLATES-COMMIT-B (the `templates.tree_sha256` of a build's build.json)
templates_sha256 := env("FOLIA_TEMPLATES_SHA256", "TEMPLATES-TREE-SHA256")
# The recorded reference `just determinism` compares Folia's builds with (plan v12, §4.2: A0's
# manifest), and the baseline it was built as of: its build time and site commit
reference := env("FOLIA_REFERENCE", "parity/evidence/a0-reference/sha256.txt")
reference_label := env("FOLIA_REFERENCE_LABEL", "BASELINE-B")
# The Jekyll toolchains `just toolchain-compare` builds the site with (A2.7), and the time each
# build is pinned to: BASELINE-B's build time, A0's
toolchains := env("FOLIA_TOOLCHAINS", "parity/toolchain/environments.txt")
toolchain_at := env("FOLIA_TOOLCHAIN_AT", "2026-10-25T14:43:51Z")

# List recipes
default:
    @just --list --unsorted

# Run every check
[group('checks')]
check: file-length fmt-check clippy test doc deny lint-sh lint-md lint-rb lint-py baseline-negative docs-build

# Check no source file exceeds 1,000 lines
[group('checks')]
file-length:
    scripts/check-file-length.sh

# Format the Rust code
[group('rust')]
fmt:
    cargo fmt --all

# Check the Rust formatting
[group('rust')]
fmt-check:
    cargo fmt --all -- --check

# Lint the Rust code (pedantic, warnings are errors)
[group('rust')]
clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings

# Run the Rust tests
[group('rust')]
test:
    cargo test --workspace --locked

# Check the API docs
[group('rust')]
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

# Check advisories, licences and sources
[group('rust')]
deny:
    cargo deny --locked check

# Lint the shell scripts
[group('lint')]
lint-sh:
    {{shellcheck}} scripts/*.sh

# Lint the Markdown files
[group('lint')]
lint-md:
    {{markdownlint}} '**/*.md' '.github/**/*.md'

# Fix what markdownlint can fix
[group('lint')]
fix-md:
    {{markdownlint}} --fix '**/*.md' '.github/**/*.md'

# Lint the Ruby code
[group('lint')]
lint-rb: ruby-tools
    {{bundle}} exec standardrb

# Format the Ruby code
[group('lint')]
fmt-rb: ruby-tools
    {{bundle}} exec standardrb --fix

[private]
ruby-tools:
    @{{bundle}} check >/dev/null || {{bundle}} install --quiet

# Lint the Python code and check its formatting
[group('lint')]
lint-py:
    {{ruff}} format --check
    {{ruff}} check

# Format the Python code
[group('lint')]
fmt-py:
    {{ruff}} format
    {{ruff}} check --fix

# Build a Jekyll baseline of the reference site
[group('parity')]
baseline label *flags:
    scripts/baseline.sh {{flags}} {{site}} {{baselines}}/{{label}}

# Compare two baselines
[group('parity')]
compare a b expected="":
    scripts/compare-builds.sh {{baselines}}/{{a}} {{baselines}}/{{b}} {{expected}}

# Run the spike 0 checks on fresh builds, keeping them: the first becomes the baseline `label`
# if it does not exist yet; an existing one is left as it is
[group('parity')]
baseline-verify label *flags:
    scripts/verify-baseline.sh {{flags}} {{site}} {{baselines}} {{label}}

# Check the baseline scripts refuse changes outside the approved paths, never touch an existing
# baseline and keep every build (throwaway repositories, a stand-in for Jekyll; part of `check`,
# as it needs neither the site nor a baseline)
[group('parity')]
baseline-negative:
    scripts/baseline-negative.sh "$(mktemp -d)/baseline-negative"

# Build one commit of the reference site with each Jekyll toolchain (Ruby, Bundler, gems) in a
# pinned container, as of one time, and compare the baselines (A2.7), keeping everything in `out`
# (a new directory) or, by default, a new `toolchain-*` one beside the baselines
[group('parity')]
toolchain-compare out="":
    scripts/toolchain-compare.sh {{site}} {{toolchains}} {{toolchain_at}} {{ if out == "" { "\"$(mktemp -u -d " + baselines + "/toolchain-$(date -u +%Y%m%dT%H%M%SZ)-XXXX)\"" } else { quote(out) } }} {{baselines}}/{{reference_label}}

# Check the container baselines refuse what they must, in the real images (needs Docker and the
# network; reads the site only)
[group('parity')]
toolchain-negative:
    scripts/toolchain-negative.sh {{site}} {{toolchains}} "$(mktemp -d)/toolchain-negative"

# Rewrite the Liquid in the reference site's Markdown (mode: check or write)
[group('parity')]
deliquify mode="check":
    python3 -I parity/codemods/deliquify.py --{{mode}} {{site}}

# Check two baselines differ by D-001 only in the spike 3 files
[group('parity')]
check-d001 before after:
    python3 -I parity/codemods/check_d001.py {{baselines}}/{{before}} {{baselines}}/{{after}} \
        $(sed -e 's/#.*//' -e '/^[[:space:]]*$/d' parity/spike3-expected.txt)

# Check two baselines differ only by changed last-modified dates
[group('parity')]
check-date-shift before after:
    python3 -I parity/check_date_shift.py {{baselines}}/{{before}} {{baselines}}/{{after}}

# Check Folia's routes against a baseline of the reference site (spike 2)
[group('parity')]
routes label:
    cargo run --quiet --locked -p parity -- routes --site {{site}} --baseline {{baselines}}/{{label}} --config parity/filopoe.toml

# Check Folia's Markdown against a baseline of the reference site (spike 1)
[group('parity')]
markdown label *flags:
    cargo run --quiet --locked -p parity -- markdown --site {{site}} --baseline {{baselines}}/{{label}} {{flags}}

# Check the pages Folia builds against a baseline of the reference site (vertical slice), keeping
# the build in `out` (a new directory) or, by default, a new one in the temporary directory
[group('parity')]
pages label out="":
    cargo run --quiet --locked -p parity -- pages --site {{site}} --baseline {{baselines}}/{{label}} --config parity/filopoe.toml --templates-sha256 {{templates_sha256}} {{ if out == "" { "" } else { "--out " + quote(out) } }}

# Build the site twice as of the reference's baseline time and compare the builds byte for byte,
# with each other and with the reference (A2.5), keeping both builds and the report in `out` (a
# new directory) or, by default, a new one in the temporary directory
[group('parity')]
determinism out="":
    cargo run --quiet --locked -p parity -- determinism --site {{site}} --baseline {{baselines}}/{{reference_label}} --config parity/filopoe.toml --templates-sha256 {{templates_sha256}} --reference {{reference}} {{ if out == "" { "" } else { "--out " + quote(out) } }}

# Check the determinism check fails on a changed reference and refuses what it must (reads the
# site and baseline only)
[group('parity')]
determinism-negative:
    scripts/determinism-negative.sh {{site}} {{baselines}}/{{reference_label}} {{reference}} {{templates_sha256}} "$(mktemp -d)/determinism-negative"

# Check the reference site's Markdown has no Liquid outside code (content lint)
[group('parity')]
liquid:
    cargo run --quiet --locked -p parity -- liquid --site {{site}} --config parity/filopoe.toml

# Check the page check fails on a baseline changed where it looks (vertical slice)
[group('parity')]
pages-negative label:
    scripts/pages-negative.sh {{site}} {{baselines}}/{{label}} "$(mktemp -d)/negative"

# Check a DOM ledger entry holds only for the files it was reviewed against, and an unknown
# source identity fails (copies the site)
[group('parity')]
ledger-negative label:
    scripts/ledger-negative.sh {{site}} {{baselines}}/{{label}} "$(mktemp -d)/ledger-negative"

# Check the route-level link check fails on links to nothing planned, and on no other (copies the
# site, A2.8)
[group('parity')]
links-negative label:
    scripts/links-negative.sh {{site}} {{baselines}}/{{label}} "$(mktemp -d)/links-negative"

# Check the coverage report fails on waivers and removals no owner approved (copies the ledger)
[group('parity')]
coverage-negative label:
    scripts/coverage-negative.sh {{site}} {{baselines}}/{{label}} "$(mktemp -d)/coverage-negative"

# Check an evidence folder keeps its builds' records, made from one clean Folia commit, and is
# (or can be) committed on its own on that commit (plan v12, Q26)
[group('parity')]
evidence-provenance dir:
    scripts/evidence-provenance.sh {{dir}}

# Check the evidence provenance check refuses what breaks the convention (throwaway repositories)
[group('parity')]
evidence-provenance-negative:
    scripts/evidence-provenance-negative.sh "$(mktemp -d)/evidence-provenance-negative"

# Check a step of the vertical slice keeps out of the other step's mode (2 or 3)
[group('parity')]
slice-boundary step base="main":
    scripts/slice-boundary.sh {{step}} {{base}}

# Check the Markdown check fails on a baseline changed once per gate (spike 1)
[group('parity')]
markdown-negative label:
    scripts/markdown-negative.sh {{site}} {{baselines}}/{{label}} "$(mktemp -d)/negative"

# Build the reference site with Folia into a directory outside it
[group('site')]
build dest:
    cargo run --quiet --locked -p folia -- build --site {{site}} --config parity/filopoe.toml --dest {{dest}}

# Serve the docs with live reload
[group('docs')]
docs:
    uvx --with-requirements docs/requirements.txt mkdocs serve --livereload

# Build the docs into ./site
[group('docs')]
docs-build:
    uvx --with-requirements docs/requirements.txt mkdocs build --strict
