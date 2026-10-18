#!/usr/bin/env python3
"""Deploy a local Integration Workload and HTTPS ServiceNow stand-in, then exercise it."""
from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("accept_integration", ROOT / "scripts" / "accept-integration-discovery.py")
acceptance = importlib.util.module_from_spec(spec)
spec.loader.exec_module(acceptance)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target" / "debug" / "ipe-integration")
    parser.add_argument("--build", action="store_true", help="Build ipe-integration first")
    parser.add_argument("--serve", action="store_true", help="Keep the local deployment running until Ctrl-C")
    parser.add_argument("--report", type=Path, default=ROOT / "target" / "integration-demo-report.json")
    args = parser.parse_args()
    if args.build:
        subprocess.run(["cargo", "build", "-p", "ipe-integration"], cwd=ROOT, check=True)
    runtime = acceptance.Acceptance(args.binary, args.report)
    try:
        runtime.configure_inventory()
        runtime.start()
        runtime.check("agent connection check and discovery", runtime.test_inventory_success)
        runtime.check("CLI and MCP agent interfaces", runtime.test_cli_mcp)
        runtime.report["passed"] = not runtime.report["failures"]
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(runtime.report, indent=2) + "\n")
        print(json.dumps(runtime.report, indent=2), flush=True)
        if not runtime.report["passed"]:
            return 1
        if args.serve:
            # Operator-only handoff. Tokens go into a mode-0600 file, never stdout.
            handoff = runtime.directory / "agent.env"
            handoff.write_text("export IPE_INTEGRATION_TOKEN='" + runtime.token + "'\n")
            handoff.chmod(0o600)
            print(f"Local integration workload: http://127.0.0.1:{runtime.port}")
            print(f"Operator-only token environment file: {handoff}")
            print(f"Opaque connection reference: {runtime.connection_ref}")
            print("Source the environment file to use integration-toolkit.py; Ctrl-C tears down the deployment.", flush=True)
            while runtime.process.poll() is None:
                time.sleep(0.2)
            return 1
        return 0
    except KeyboardInterrupt:
        return 0
    finally:
        runtime.close()


if __name__ == "__main__":
    sys.exit(main())
