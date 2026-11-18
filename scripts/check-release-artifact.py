"""Require the server output before reporting release size."""
from pathlib import Path
import sys


def artifact_size(directory: Path, target: str) -> int:
    name = 'ipe-server.exe' if 'windows' in target else 'ipe-server'
    artifact = directory / target / 'release' / name
    size = artifact.stat().st_size
    if not artifact.is_file() or size == 0:
        raise ValueError(f'Missing or empty release artifact: {artifact}')
    return size


if __name__ == '__main__':
    size = artifact_size(Path('target'), sys.argv[1])
    print(f'ipe-server: {size} bytes')
    # Preserve the existing advisory threshold; this is not a runtime size guarantee.
    if size > 2097152:
        print('Warning: server exceeds the historical 2 MiB advisory target.')
