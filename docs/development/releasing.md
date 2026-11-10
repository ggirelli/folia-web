# Releasing

How a Folia release is cut. The tag and the GitHub release are made by hand by the project owner,
from a commit of `main` that passed its checks; the Linux binary is built by the
[release workflow](#the-release-workflow). Nothing here publishes to crates.io (`publish = false`).

## Checklist

Everything on `main`, CI green:

1. **Pull requests:** the ones the release needs are merged, and `git status` is clean on `main`.
2. **Version:** the workspace `version` in `Cargo.toml` is the release's, `Cargo.lock` follows,
   `folia --version` prints it, and the example's feeds `generator` matches (a test checks it).
3. **Release notes:** `docs/release-notes/vX.Y.Z.md` and `CHANGELOG.md` are current, and say what the
   release is *not* (it does not claim the Jekyll site is replaced).
4. **Checks:** `just check` and `just check-parity` pass, CI on the commit is green, and
   `cargo deny check` is clean.
5. **Clean clone:** in a fresh clone of that commit, `just check`, `just check-parity`,
   `just build-example DIR` and
   `just serve DIR` work (the [manual testing guide](manual-testing.md); for a clean machine, its
   install commands).
6. **Real site (if you have its private inputs):** `just pages <label>` and `just determinism` on
   that commit say what you expect; the classified verdicts (*clean build*, *exact parity*) go in
   the release's private evidence, never in this repository.

Then, from the commit you checked, the owner tags it and pushes the tag:

```sh
git tag -s -a vX.Y.Z -m "Folia X.Y.Z"   # signed, after the checklist
git push origin vX.Y.Z
```

The tag starts the release workflow (below), which builds the binary and leaves a **draft** release
with it attached. Read the draft, check that its text is `docs/release-notes/vX.Y.Z.md`, and
publish it (mark it a pre-release while the version is 0.x). The links in the notes are relative
(they work in the docs site); the workflow rewrites them to the tag's files on GitHub.

## The release workflow

`.github/workflows/release.yml` builds `folia` for **Linux x86_64** (glibc 2.35, Ubuntu 22.04) with
`cargo build --release --locked`, with no dependency cache. It then:

- refuses a tag that is not `vX.Y.Z` or does not equal the workspace version;
- checks the binary (`folia --version` is the version; it builds the example site);
- packs `folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` (the binary, `LICENSE`, `DISCLAIMER.md`)
  with a `.sha256` file, and attests the tarball's build provenance (GitHub artifact attestation);
- on a pushed tag, attaches both to the release: a new **draft** from the notes file if the tag has
  no release, else the existing one, whose text it never changes.

Run by hand (Actions, "Release", with the tag) it builds an existing tag; ticking `publish` attaches
the files to that tag's release. That is how a release made before the workflow got its binary.

Verify a download:

```sh
sha256sum -c folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz.sha256
gh attestation verify folia-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz --repo ggirelli/folia-web
```

The workflow's permissions are the least that work (`contents: write` only in the job that
attaches files); its actions are pinned by commit. It builds nothing from a site's private inputs,
and never publishes by itself. macOS is not built: its CI job is informational and has not been
green for long. Windows is not supported.
