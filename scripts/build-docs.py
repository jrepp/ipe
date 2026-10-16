#!/usr/bin/env python3
"""Generate local docs data atomically; leave previous results intact on failure."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    required = (ROOT / ".python-version").read_text().strip()
    actual = ".".join(map(str, sys.version_info[:3]))
    if actual != required:
        raise RuntimeError(f"Python {required} is required, got {actual}. Run make docs-site.")
    result = subprocess.run(
        ["cargo", "run", "--locked", "--release", "-p", "ipe-core", "--example", "docs_data"],
        cwd=ROOT, stdout=subprocess.PIPE, text=True, check=True,
    )
    data = json.loads(result.stdout)
    if data.get("schema_version") != 1 or not data.get("examples") or not data.get("workloads"):
        raise ValueError("Incomplete docs export")
    destination = ROOT / "docs/data/site-data.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", dir=destination.parent, delete=False) as output:
            temporary = output.name
            json.dump(data, output, indent=2, allow_nan=False)
            output.write("\n")
        os.replace(temporary, destination)
    finally:
        if temporary and os.path.exists(temporary):
            os.unlink(temporary)
    # Include reference documents with their original relative paths for portable previews.
    package = ROOT / "target/docs-site"
    if package.exists():
        shutil.rmtree(package)
    sources = list((ROOT / "docs").glob("*.html"))
    sources += list((ROOT / "docs").glob("*.css")) + list((ROOT / "docs").glob("*.js"))
    sources += [destination]
    tracked = subprocess.run(["git", "ls-files", "*.md"], cwd=ROOT, capture_output=True, text=True, check=True)
    sources += [ROOT / path for path in tracked.stdout.splitlines()]
    sources += [ROOT / "crates/ipe-core/examples/docs_data.rs", ROOT / "crates/ipe-core/src/ast/nodes.rs"]
    for source in sources:
        output = package / source.relative_to(ROOT)
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, output)
    print(f"Built {len(data['examples'])} parser trees and {len(data['workloads'])} measured workloads.")
    print(f"Portable artifact: {package}")
    print("Run make docs-serve, then open http://127.0.0.1:8000/docs/")


if __name__ == "__main__":
    main()
