#!/usr/bin/env python3
"""Serve the product ship mocks without exposing repository or host credentials."""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent
MODEL = ROOT.parents[1] / "testing/integration/hosted/product-ship"
ASSETS = {
    "/": ("index.html", "text/html"),
    **{"/" + name: (name, "text/html") for name in ["index.html", "board.html", "network.html", "console.html", "fleet.html"]},
    "/projection.js": ("projection.js", "text/javascript"),
    "/fleet.js": ("fleet.js", "text/javascript"),
    "/fleet-model.js": ("fleet-model.js", "text/javascript"),
    "/fleet.css": ("fleet.css", "text/css"),
    "/app.js": ("app.js", "text/javascript"),
    "/styles.css": ("styles.css", "text/css"),
    **{"/fonts/" + name: ("fonts/" + name, "font/woff2") for name in [
        "IBMPlexSans-Regular.woff2", "IBMPlexSans-Medium.woff2", "IBMPlexSans-SemiBold.woff2"
    ]},
    "/fonts/LICENSE.txt": ("fonts/LICENSE.txt", "text/plain"),
    "/data.json": ("data.json", "application/json"),
}


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.respond(False)

    def do_HEAD(self):
        self.respond(True)

    def respond(self, head):
        path = urlsplit(self.path).path
        if path == "/planning.md":
            file, mime = ROOT.parents[1] / "docs/ENTERPRISE_SCALE_PLANNING.md", "text/plain"
        elif path == "/model/evaluate.js":
            file, mime = MODEL / "product-ship.js", "text/javascript"
        elif path in ASSETS:
            name, mime = ASSETS[path]
            file = ROOT / name
        else:
            self.send_error(404)
            return
        if not file.is_file() or file.is_symlink():
            self.send_error(404)
            return
        content = file.read_bytes()
        self.send_response(200)
        self.send_header("Content-Type", mime + "; charset=utf-8")
        self.send_header("Content-Length", str(len(content)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        if not head:
            self.wfile.write(content)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8768)
    args = parser.parse_args()
    print(f"Product ship mockups: http://127.0.0.1:{args.port}/", flush=True)
    ThreadingHTTPServer(("127.0.0.1", args.port), Handler).serve_forever()
