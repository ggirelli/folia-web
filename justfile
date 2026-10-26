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

# List recipes
default:
    @just --list --unsorted

# Run every check
[group('checks')]
check: file-length fmt-check clippy test doc deny lint-sh lint-md lint-rb lint-py docs-build

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

# Build a baseline and run the spike 0 checks
[group('parity')]
baseline-verify label:
    scripts/verify-baseline.sh {{site}} {{baselines}} {{label}}

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

# Check the pages Folia builds against a baseline of the reference site (vertical slice)
[group('parity')]
pages label:
    cargo run --quiet --locked -p parity -- pages --site {{site}} --baseline {{baselines}}/{{label}} --config parity/filopoe.toml

# Check the reference site's Markdown has no Liquid outside code (content lint)
[group('parity')]
liquid:
    cargo run --quiet --locked -p parity -- liquid --site {{site}} --config parity/filopoe.toml

# Check the page check fails on a baseline changed where it looks (vertical slice)
[group('parity')]
pages-negative label:
    scripts/pages-negative.sh {{site}} {{baselines}}/{{label}} "$(mktemp -d)/negative"

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
