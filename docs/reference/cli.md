# Command line

Folia has two commands, `folia build` and `folia serve`, and no others: there is no `new`, `init`,
`clean` or `watch`. `folia --version` prints the version, and `--help` on the program or on a
command prints its options.

```sh
folia build --dest DIR [options]
folia serve [options]
```

## `folia build`

Builds the whole site into a directory. This is the command you publish with; see
[Build and deploy](../getting-started/build-and-deploy.md).

| Option | Default | What |
|-|-|-|
| `--site PATH` | `.` | The site: a git repository |
| `--config PATH` | `<site>/_folia/folia.toml` | The [configuration](configuration.md) |
| `--dest PATH` | required | Where to write the site: a directory outside the site, absent, empty, or written by an earlier build |
| `--future` | off | Also build documents dated after the build. The output is a preview |
| `--unpublished` | off | Also build documents whose front matter says `published: false`. The output is a preview |
| `--at TIME` | now | Build as of this moment, an RFC 3339 timestamp with a time and an offset, such as `2026-01-01T00:00:00Z` |

`--future` and `--unpublished` are explained in [Drafts and scheduling](../guide/drafts.md), and
`--at` there too. They are flags, not configuration keys: a `future`, `unpublished` or `at` key in
`folia.toml` is an error.

### What it prints

While it builds, Folia prints **warnings** (a broken wiki-link) and `skipped:` lines for any
document it does not support yet, each with the reason. Then comes the summary on stdout:

```text
Built 18 pages, 1 redirect stubs, 15 generated files and 2 static files into /tmp/out; 1 planned outputs are not built yet.
```

The counts are what Folia wrote: pages rendered from your content, a stub for each `redirect_from`
URL, files it generated itself (feeds, sitemap, search data, the garden's graph) and files copied
as they are.

**"N planned outputs are not built yet"** counts the URLs Folia plans for every site but has no
generator for. Today that is one, `/robots.txt`: Folia reserves the URL and never writes the file
itself, so the summary counts it. Put a `robots.txt` in the site and it is copied as a static file,
the count drops to 0, and nothing else changes. A site without one works fine. In the example site,
the `1` is this.

A preview build adds `PREVIEW BUILD (...): not for publishing.` to the summary, and a `warning:`
line says the same.

### Exit status

| Status | Meaning | Output |
|-|-|-|
| `0` | The site was built into `--dest`. Warnings and `skipped:` documents do not change it | warnings and skips on stderr, then the summary on stdout |
| `1` | The build failed: planning, a missing or failing template, a render error, an unknown author, a destination Folia refuses, or a write error. **Nothing was written**; `--dest` is as it was | every problem on stderr, then `The build failed with N problems; nothing was written, …` |
| `2` | Usage or configuration error: an unknown command or option, a missing `--dest`, a bad `--at`, a configuration that cannot be read or has an unknown key | the error on stderr |

A run that exits 0 may have skipped documents. A script that must not publish a partial site
should check stderr for `skipped:`.

## `folia serve`

Builds the site, serves it, and builds again when a file changes. See
[Preview while you write](../guide/preview.md) for how it behaves.

| Option | Default | What |
|-|-|-|
| `--site PATH` | `.` | The site |
| `--config PATH` | `<site>/_folia/folia.toml` | The configuration, read again on every build |
| `--dest PATH` | `folia/serve/<site name>-<hash>` in your cache directory | Where to build: a directory outside the site, absent, empty or written by `folia serve` (never one `folia build` wrote) |
| `--host ADDRESS` | `127.0.0.1` | The address to listen on. Any but a loopback address may let other machines see the site |
| `--port PORT` | `4000` | The port to listen on; `0` picks a free one, and the address is printed |
| `--future` | off | As for `build` |
| `--unpublished` | off | As for `build` |

There is no `--at`: every rebuild is as of its own moment.

`folia serve` runs until you stop it with Ctrl+C. It exits with status `2` before serving anything
if the site or configuration cannot be read, the destination is refused, the address cannot be
listened on or the sources cannot be watched. A build that fails later is not an exit: the last
good site stays up.

## Environment

| Variable | What |
|-|-|
| `FOLIA_SASS` | The Dart Sass program, when it is not `sass` on `PATH` (see [Stylesheets](../getting-started/build-and-deploy.md#stylesheets)) |
| `XDG_CACHE_HOME` | Where `folia serve` keeps its default destination, if set (else `~/.cache`) |
