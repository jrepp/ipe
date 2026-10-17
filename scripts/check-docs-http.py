#!/usr/bin/env python3
"""Serve the packaged site at /ipe/ and check its local links over HTTP."""
from functools import partial
from html.parser import HTMLParser
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import shutil
import tempfile
from threading import Thread
from urllib.parse import urldefrag, urljoin, urlsplit
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "target/docs-site"


class PageLinks(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
        self.refresh = None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        for key in ("href", "src"):
            if attrs.get(key):
                self.links.append(attrs[key])
        if tag == "meta" and attrs.get("http-equiv", "").lower() == "refresh":
            self.refresh = attrs.get("content", "").partition("url=")[2].strip()


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *_args):
        pass


def check_site(base):
    # The data file is loaded by JavaScript, rather than an HTML link.
    pending = {base, urljoin(base, "docs/"), urljoin(base, "docs/data/site-data.json")}
    pending.update(urljoin(base, page.relative_to(SITE).as_posix()) for page in SITE.rglob("*.html"))
    visited = set()
    while pending:
        url = pending.pop()
        if url in visited:
            continue
        if not url.startswith(base):
            raise ValueError(f"Local link escapes the /ipe/ project path: {url}")
        with urlopen(url, timeout=10) as response:
            if response.status != 200 or not response.url.startswith(base):
                raise ValueError(f"Unexpected response for {url}: {response.status} {response.url}")
            content = response.read()
            content_type = response.headers.get_content_type()
        visited.add(url)
        if url.endswith("/data/site-data.json"):
            data = json.loads(content)
            if data.get("schema_version") != 1 or not data.get("examples") or not data.get("workloads"):
                raise ValueError("The HTTP data response lacks parser or benchmark output")
        if content_type != "text/html":
            continue
        page = PageLinks()
        page.feed(content.decode("utf-8"))
        if url == base:
            if not page.refresh or urljoin(base, page.refresh) != urljoin(base, "docs/"):
                raise ValueError("The project root must redirect to /ipe/docs/")
        for link in page.links:
            parsed = urlsplit(link)
            if parsed.scheme or parsed.netloc:
                continue
            pending.add(urldefrag(urljoin(url, link)).url)
    print(f"HTTP check passed: {len(visited)} pages, assets, and references under /ipe/.")


def main():
    if not (SITE / "index.html").is_file():
        raise FileNotFoundError("Build the site first with make docs-site")
    with tempfile.TemporaryDirectory(prefix="ipe-docs-http-") as directory:
        shutil.copytree(SITE, Path(directory) / "ipe")
        handler = partial(QuietHandler, directory=directory)
        with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
            thread = Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                check_site(f"http://127.0.0.1:{server.server_port}/ipe/")
            finally:
                server.shutdown()
                thread.join()


if __name__ == "__main__":
    main()
