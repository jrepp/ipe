#!/usr/bin/env python3
"""Package the static documentation site and archived measurement data."""
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    required = (ROOT / ".python-version").read_text().strip()
    actual = ".".join(map(str, sys.version_info[:3]))
    if actual != required:
        raise RuntimeError(f"Python {required} is required, got {actual}. Run make docs-site.")
    # docs_data.rs (the legacy-engine exporter) was deleted with the legacy engine
    # cluster; docs/data/site-data.json is the frozen final export (revision
    # 3d54de2) kept as an archived measurement for the AST explorer and the
    # performance dashboard.
    data_file = ROOT / "docs/data/site-data.json"
    data = json.loads(data_file.read_text())
    if data.get("schema_version") != 1 or not data.get("examples") or not data.get("workloads"):
        raise ValueError("Incomplete docs export")
    # Include reference documents with their original relative paths for portable previews.
    package = ROOT / "target/docs-site"
    if package.exists():
        shutil.rmtree(package)
    sources = list((ROOT / "docs").glob("*.html"))
    sources += list((ROOT / "docs").glob("*.css")) + list((ROOT / "docs").glob("*.js"))
    # Preview new top-level guides before they are added to the Git index.
    sources += list((ROOT / "docs").glob("*.md"))
    sources += [ROOT / "CONTRIBUTING.md", ROOT / "Makefile", ROOT / "justfile"]
    # Include new design records and their schema before they are added to Git.
    sources += [path for path in (ROOT / "docs-cms").rglob("*")
                if path.is_file() and path.suffix in {".md", ".yaml", ".json"}]
    # The overview uses the same local Plex fonts as the product preview.
    sources += list((ROOT / "tools/product-ship-mocks/fonts").glob("*.woff2"))
    sources += [ROOT / "tools/product-ship-mocks/fonts/LICENSE.txt", data_file]
    sources += [ROOT / "docs/benchmark-history.json", ROOT / "docs/benchmark-latest.json"]
    # Keep source examples from the runtime contract and evidence design available offline.
    sources += [ROOT / "crates/ipe-core/src/runtime_policy.rs",
                ROOT / "crates/ipe-core/src/runtime_facts.rs",
                ROOT / "crates/ipe-integration/src/host/governance.rs",
                ROOT / "tools/product-ship-mocks/app.js"]
    tracked = subprocess.run(["git", "ls-files", "*.md"], cwd=ROOT, capture_output=True, text=True, check=True)
    sources += [ROOT / path for path in tracked.stdout.splitlines()]
    for source in sources:
        output = package / source.relative_to(ROOT)
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, output)
    shutil.copyfile(ROOT / "scripts/site-index.html", package / "index.html")
    print(f"Packaged archived data: {len(data['examples'])} parser trees and {len(data['workloads'])} measured workloads.")
    print("Portable artifact:", package)
    print("Run make docs-serve, then open http://127.0.0.1:8000/docs/")


if __name__ == "__main__":
    main()
