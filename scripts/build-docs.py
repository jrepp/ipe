#!/usr/bin/env python3
"""Generate local docs data atomically; leave previous results intact on failure."""
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
    sources += [data_file]
    # Source references from contract.html resolve inside the packaged artifact.
    sources += [ROOT / "crates/ipe-core/src/runtime_policy.rs",
                ROOT / "crates/ipe-core/src/runtime_facts.rs"]
    tracked = subprocess.run(["git", "ls-files", "*.md"], cwd=ROOT, capture_output=True, text=True, check=True)
    sources += [ROOT / path for path in tracked.stdout.splitlines()]
    for source in sources:
        output = package / source.relative_to(ROOT)
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, output)
    shutil.copyfile(ROOT / "scripts/site-index.html", package / "index.html")
    print(f"Built {len(data['examples'])} parser trees and {len(data['workloads'])} measured workloads.")
    print("Portable artifact:", package)
    print("Run make docs-serve, then open http://127.0.0.1:8000/docs/")


if __name__ == "__main__":
    main()
