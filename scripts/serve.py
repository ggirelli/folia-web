"""Serve a built site locally, as its host serves it, for checking it in a browser.

Run: python3 -I scripts/serve.py DIR [PORT]   (or `just serve DIR [PORT]`)

A URL without an extension is served from its `.html` file (`/about` from `about.html`), a
directory from its `index.html`, and a URL that matches no file gets `404.html` with status 404,
as GitHub Pages does. It listens on 127.0.0.1 only, and is meant for local checks, not hosting.
"""

import contextlib
import functools
import http.server
import pathlib
import sys
import urllib.parse


class Handler(http.server.SimpleHTTPRequestHandler):
    """Serves files from the site, resolving clean URLs and the error page."""

    def send_head(self):
        path = urllib.parse.urlsplit(self.path).path
        found = self.resolve(path)
        if found is None:
            self.not_found()
            return None
        self.path = found
        return super().send_head()

    def resolve(self, path):
        """The URL path of the file `path` is served from, or None."""
        root = pathlib.Path(self.directory)
        relative = urllib.parse.unquote(path).lstrip("/")
        index = (relative.rstrip("/") + "/index.html").lstrip("/")
        candidates = [relative, relative + ".html", index]
        for candidate in candidates:
            file = (root / candidate).resolve()
            if file.is_file() and file.is_relative_to(root.resolve()):
                return "/" + urllib.parse.quote(candidate)
        return None

    def not_found(self):
        """Sends the site's 404 page, with status 404."""
        page = pathlib.Path(self.directory) / "404.html"
        if not page.is_file():
            self.send_error(404)
            return
        body = page.read_bytes()
        self.send_response(404)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def main():
    if len(sys.argv) not in (2, 3):
        sys.exit("usage: serve.py DIR [PORT]")
    root = pathlib.Path(sys.argv[1])
    if not root.is_dir():
        sys.exit(f"{root} is not a directory: build the site into it first")
    port = int(sys.argv[2]) if len(sys.argv) == 3 else 8000
    handler = functools.partial(Handler, directory=str(root))
    with http.server.ThreadingHTTPServer(("127.0.0.1", port), handler) as server:
        print(f"Serving {root} at http://127.0.0.1:{port}/ (Ctrl+C to stop)")
        with contextlib.suppress(KeyboardInterrupt):
            server.serve_forever()


if __name__ == "__main__":
    main()
