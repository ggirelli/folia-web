# Releasing

How a Folia release is cut. There is **no release workflow** yet: a release is a tag and a GitHub
release made by hand from a commit of `main` that passed its checks, by the project owner. Nothing
here runs by itself, and nothing here publishes to crates.io (`publish = false`).

## Checklist

Everything on `main`, CI green:

1. **Pull requests:** the ones the release needs are merged, and `git status` is clean on `main`.
2. **Version:** the workspace `version` in `Cargo.toml` is the release's, `Cargo.lock` follows,
   `folia --version` prints it, and the example's feeds `generator` matches (a test checks it).
3. **Release notes:** `docs/release-notes/vX.Y.Z.md` and `CHANGELOG.md` are current, and say what the
   release is *not* (it does not claim the Jekyll site is replaced).
4. **Checks:** `just check` passes, CI on the commit is green, and `cargo deny check` is clean.
5. **Clean clone:** in a fresh clone of that commit, `just check`, `just build-example DIR` and
   `just serve DIR` work (the [manual testing guide](manual-testing.md); for a clean machine, its
   install commands).
6. **Real site (if you have its private inputs):** `just pages <label>` and `just determinism` on
   that commit say what you expect; the classified verdicts (*clean build*, *exact parity*) go in
   the release's private evidence, never in this repository.

Then, from the commit you checked:

```sh
git tag -a vX.Y.Z -m "Folia X.Y.Z"           # the owner's decision, after the checklist
git push origin vX.Y.Z
cargo build --release --locked -p folia      # target/release/folia
sha256sum target/release/folia > folia-vX.Y.Z-linux-x86_64.sha256
```

Create the GitHub release from the tag with `docs/release-notes/vX.Y.Z.md` as its text, and attach
the binary and its checksum if you want to offer one (built on the platform you name in the file:
CI and the install guide cover Ubuntu 24.04 only). Mark it a pre-release while the version is 0.x.

## A release workflow, if one is wanted

Not built; what would make sense, in order of how little it risks:

1. **On a tag `v*`**, a job that checks the tag equals the workspace version, runs `just check`'s
   Rust half, builds `folia` with `--release --locked` on Ubuntu (and macOS once its CI job has been
   green for a while), and uploads the binaries and their SHA-256 sums as workflow artifacts. It
   creates no release by itself.
2. **A draft release** from those artifacts and the matching release notes file, left as a draft
   for the owner to read and publish.
3. Never: publishing automatically, crates.io, or any step that needs private site inputs.

It could be tested before a real tag with `workflow_dispatch` on a branch, uploading artifacts
only. It is not a release blocker.
