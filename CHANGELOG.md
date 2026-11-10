# Changelog

## Unreleased

- `folia serve`: builds the site, serves it locally and builds it again, reloading the open pages,
  when a file changes ([#46](https://github.com/ggirelli/folia-web/issues/46);
  [guide](docs/guide/building.md#previewing-while-you-write),
  [limitations](docs/known-limitations.md#folia-serve)).
- Documentation: a user guide (getting started, content, templates, building and deploying, feeds,
  sitemap and search), a guide to migrating from Jekyll, and the parity tools reduced to a reference
  for that exercise. The design plans were removed from the docs; they remain in the git history.
- `folia build` and `folia serve` take `--future` (documents dated after the build) and
  `--unpublished` (documents whose front matter says `published: false`). Each makes a preview
  build, not for publishing, and `folia build` refuses to replace a normal build with one, or the
  reverse ([#47](https://github.com/ggirelli/folia-web/issues/47);
  [guide](docs/guide/building.md#previewing-drafts-and-scheduled-posts)).
- `folia build --at <TIME>` builds as of an RFC 3339 timestamp instead of now, so that a build is
  reproducible ([#50](https://github.com/ggirelli/folia-web/issues/50);
  [guide](docs/guide/building.md#building-as-of-a-moment)).
- The front matter `lang` of a post, garden note, micro entry or page reaches templates as
  `page.lang`, as written (`[italian]` or `italian`), or absent without one
  ([#56](https://github.com/ggirelli/folia-web/issues/56)).
- `folia serve` stamps each rebuild with the local time of day (`[14:03:22] Changed: …`).
- `folia serve` rewrites only the files a rebuild changed, and removes those it no longer makes,
  instead of replacing the whole destination: on a slow disk, a rebuild took minutes
  ([guide](docs/guide/building.md#previewing-while-you-write)).
- A release workflow that builds the Linux x86_64 binary from a tag
  ([releasing](docs/development/releasing.md#the-release-workflow)).

## 0.1.0

The first pre-1.0 release: it builds the reference site, refuses to delete what it did not write
and fails on template errors. It is not a claim that the Jekyll site has been replaced. See the
[release notes](docs/release-notes/v0.1.0.md) and the
[known limitations](docs/known-limitations.md).
