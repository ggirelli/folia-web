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
# The site's own inputs, which its owner keeps outside this repository (docs/development.md):
# Folia's configuration for the site, its deviation ledger and expected warnings, and the tree
# hash of the templates the pages check expects in the site's _folia/templates (the
# `templates.tree_sha256` of a build's build.json). A recipe that needs one refuses to run without
# it; none has a default, so that no site's data is in Folia
config := env("FOLIA_CONFIG", "")
ledger := env("FOLIA_LEDGER", "")
expected_warnings := env("FOLIA_EXPECTED_WARNINGS", "")
templates_sha256 := env("FOLIA_TEMPLATES_SHA256", "")
# The recorded reference `just determinism` compares Folia's builds with (plan v12, §4.2: A0's
# manifest), and the baseline it was built as of: its build time and site commit
a0_reference := env("FOLIA_A0_REFERENCE", "")
a0_label := env("FOLIA_A0_LABEL", "")
reference := env("FOLIA_REFERENCE", a0_reference)
reference_label := env("FOLIA_REFERENCE_LABEL", a0_label)
# The Jekyll toolchains `just toolchain-compare` builds the site with (A2.7), and the time each
# build is pinned to: the reference baseline's build time
toolchains := env("FOLIA_TOOLCHAINS", "")
toolchain_at := env("FOLIA_TOOLCHAIN_AT", "")

# List recipes
default:
    @just --list --unsorted

# Refuse to go on when the site input `name` (`value`) is not set
[private]
require name value:
    @[ -n {{quote(value)}} ] || { echo "{{name}} is not set: the site's inputs are kept outside Folia's repository (docs/development.md)" >&2; exit 2; }

# Run every check
[group('checks')]
check: file-length fmt-check clippy test doc deny lint-sh lint-md lint-rb lint-py baseline-negative accept-negative docs-build

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
toolchain-compare out="": (require "FOLIA_TOOLCHAINS" toolchains) (require "FOLIA_TOOLCHAIN_AT" toolchain_at)
    scripts/toolchain-compare.sh {{site}} {{toolchains}} {{toolchain_at}} {{ if out == "" { "\"$(mktemp -u -d " + baselines + "/toolchain-$(date -u +%Y%m%dT%H%M%SZ)-XXXX)\"" } else { quote(out) } }} {{baselines}}/{{reference_label}}

# Check the container baselines refuse what they must, in the real images (needs Docker and the
# network; reads the site only)
[group('parity')]
toolchain-negative: (require "FOLIA_TOOLCHAINS" toolchains)
    scripts/toolchain-negative.sh {{site}} {{toolchains}} "$(mktemp -d)/toolchain-negative"

# Check two baselines differ by D-001 only in the files `expected` (the site's spike 3 list) names
[group('parity')]
check-d001 before after expected:
    python3 -I parity/codemods/check_d001.py {{baselines}}/{{before}} {{baselines}}/{{after}} \
        $(sed -e 's/#.*//' -e '/^[[:space:]]*$/d' {{quote(expected)}})

# Check two baselines differ only by changed last-modified dates
[group('parity')]
check-date-shift before after:
    python3 -I parity/check_date_shift.py {{baselines}}/{{before}} {{baselines}}/{{after}}

# Check Folia's routes against a baseline of the reference site (spike 2)
[group('parity')]
routes label: (require "FOLIA_CONFIG" config)
    cargo run --quiet --locked -p parity -- routes --site {{site}} --baseline {{baselines}}/{{label}} --config {{quote(config)}}

# Check Folia's Markdown against a baseline of the reference site (spike 1)
[group('parity')]
markdown label *flags: (require "FOLIA_LEDGER" ledger)
    cargo run --quiet --locked -p parity -- markdown --site {{site}} --baseline {{baselines}}/{{label}} --ledger {{quote(ledger)}} {{flags}}

# Check the pages Folia builds against a baseline of the reference site (vertical slice), keeping
# the build in `out` (a new directory) or, by default, a new one in the temporary directory
[group('parity')]
pages label out="": (require "FOLIA_CONFIG" config) (require "FOLIA_LEDGER" ledger) (require "FOLIA_EXPECTED_WARNINGS" expected_warnings) (require "FOLIA_TEMPLATES_SHA256" templates_sha256)
    cargo run --quiet --locked -p parity -- pages --site {{site}} --baseline {{baselines}}/{{label}} --config {{quote(config)}} --ledger {{quote(ledger)}} --expected-warnings {{quote(expected_warnings)}} --templates-sha256 {{templates_sha256}} {{ if out == "" { "" } else { "--out " + quote(out) } }}

# Build the site twice as of the reference's baseline time and compare the builds byte for byte,
# with each other and with the reference (A2.5), keeping both builds and the report in `out` (a
# new directory) or, by default, a new one in the temporary directory. The reference must be the
# baseline's: A0's for FOLIA_A0_LABEL by default; for another baseline, FOLIA_REFERENCE with
# FOLIA_REFERENCE_LABEL naming that baseline, or nothing is built
[group('parity')]
determinism out="": (require "FOLIA_CONFIG" config) (require "FOLIA_TEMPLATES_SHA256" templates_sha256)
    scripts/determinism-reference.sh {{quote(reference_label)}} {{quote(a0_label)}} {{quote(a0_reference)}} >/dev/null
    cargo run --quiet --locked -p parity -- determinism --site {{site}} --baseline {{baselines}}/{{reference_label}} --config {{quote(config)}} --templates-sha256 {{templates_sha256}} --reference {{quote(reference)}} {{ if out == "" { "" } else { "--out " + quote(out) } }}

# Check the determinism check fails on a changed reference and refuses what it must (reads the
# site and baseline only)
[group('parity')]
determinism-negative: (require "FOLIA_CONFIG" config) (require "FOLIA_TEMPLATES_SHA256" templates_sha256)
    scripts/determinism-reference.sh {{quote(reference_label)}} {{quote(a0_label)}} {{quote(a0_reference)}} >/dev/null
    scripts/determinism-negative.sh {{site}} {{quote(config)}} {{baselines}}/{{reference_label}} {{quote(reference)}} {{templates_sha256}} "$(mktemp -d)/determinism-negative"

# Check the reference site's Markdown has no Liquid outside code (content lint)
[group('parity')]
liquid: (require "FOLIA_CONFIG" config)
    cargo run --quiet --locked -p parity -- liquid --site {{site}} --config {{quote(config)}}

# Run every check built so far against a baseline, in order, and write one evidence folder with
# each criterion's status (pass, fail or pending) and every input used (A2.10, F12), into `out` (a
# new directory) or, by default, a new `accept-*` one beside the baselines. Determinism compares
# with A0's reference for FOLIA_A0_LABEL only; for another baseline, set FOLIA_REFERENCE to its
# reference and FOLIA_REFERENCE_LABEL to its label, or the determinism check is refused. The site's
# inputs come from FOLIA_CONFIG, FOLIA_LEDGER, FOLIA_EXPECTED_WARNINGS and FOLIA_TOOLCHAIN_DIR
[group('parity')]
accept label out="": (require "FOLIA_TEMPLATES_SHA256" templates_sha256)
    scripts/accept.sh {{site}} {{baselines}} {{label}} {{ if out == "" { "\"$(mktemp -u -d " + baselines + "/accept-" + label + "-$(date -u +%Y%m%dT%H%M%SZ)-XXXX)\"" } else { quote(out) } }} {{quote(a0_label)}} {{quote(a0_reference)}}

# Check the acceptance runner fails when a check fails, records a tree with changes as no
# evidence and refuses what it must (a throwaway repository, a stand-in for the checks; part of
# `check`)
[group('parity')]
accept-negative:
    scripts/accept-negative.sh "$(mktemp -d)/accept-negative"

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

# Build the reference site with Folia into a directory outside it
[group('site')]
build dest: (require "FOLIA_CONFIG" config)
    cargo run --quiet --locked -p folia -- build --site {{site}} --config {{quote(config)}} --dest {{dest}}

# Build the public example site (examples/minimal-site) into `dest`, a directory outside it
[group('site')]
build-example dest:
    cargo run --quiet --locked -p folia -- build --site examples/minimal-site --dest {{quote(dest)}}

# Serve a built site at http://127.0.0.1:`port`/, resolving clean URLs as its host does
[group('site')]
serve dir port="8000":
    python3 -I scripts/serve.py {{quote(dir)}} {{port}}

# Serve the docs with live reload
[group('docs')]
docs:
    uvx --with-requirements docs/requirements.txt mkdocs serve --livereload

# Build the docs into ./site
[group('docs')]
docs-build:
    uvx --with-requirements docs/requirements.txt mkdocs build --strict
