"""Validate the repository configuration against Mergify's public schema."""
import json
from pathlib import Path
import subprocess

import jsonschema
import yaml

ROOT = Path(__file__).resolve().parents[2]
SCHEMA_URL = "https://docs.mergify.com/mergify-configuration-schema.json"


def main():
    result = subprocess.run(
        ["curl", "--fail", "--silent", "--show-error", "--location",
         "--max-time", "30", "--max-filesize", str(2 * 1024 * 1024), SCHEMA_URL],
        check=True, capture_output=True, timeout=35,
    )
    schema = json.loads(result.stdout)
    config = yaml.safe_load((ROOT / ".mergify.yml").read_text())
    jsonschema.validate(config, schema)
    print("Mergify configuration matches the published schema.")


if __name__ == "__main__":
    main()
