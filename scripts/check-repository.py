"""Check the bounded repository contract documented in docs/repository-guide.md."""
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
ENTRY_DOCS = (
    'README.md', 'QUICKSTART.md', 'CONTRIBUTING.md',
    'docs/repository-guide.md', '.agent/repository.md',
)
NODE_PACKAGES = ('docs', 'testing/integration', 'tools/integration-packages')


def check(root: Path) -> list[str]:
    errors = []
    manifest = tomllib.loads((root / 'Cargo.toml').read_text())
    members = manifest['workspace']['members']
    mapping = (root / 'docs/repository-guide.md').read_text()
    rows = re.findall(r'^\| `(crates/[^`]+)` \| ([^|]+) \| ([^|]+) \|', mapping, re.M)
    paths = [path for path, _, _ in rows]
    if len(paths) != len(set(paths)):
        errors.append('Repository map contains duplicate workspace members')
    if set(paths) != set(members):
        errors.append(f'Workspace map mismatch: {sorted(set(paths) ^ set(members))}')
    for path, state, projects in rows:
        if state.strip() not in {'implemented', 'placeholder'}:
            errors.append(f'{path}: invalid implementation state')
        if not re.fullmatch(r'P[1-5](, P[1-5])*', projects.strip()):
            errors.append(f'{path}: invalid product project')
        if not (root / path / 'Cargo.toml').is_file():
            errors.append(f'{path}: missing Cargo.toml')
    for doc in ENTRY_DOCS:
        source = root / doc
        if not source.is_file():
            errors.append(f'{doc}: missing entry document')
            continue
        text = re.sub(r'```.*?```', '', source.read_text(), flags=re.S)
        for target in re.findall(r'\]\(([^)\s]+)\)', text):
            url = urlsplit(target.strip('<>'))
            if url.scheme or url.netloc or not url.path:
                continue
            path = (source.parent / unquote(url.path)).resolve()
            if not path.is_relative_to(root.resolve()) or not path.exists():
                errors.append(f'{doc}: missing or external local target {target}')
    for package in NODE_PACKAGES:
        for filename in ('package.json', 'package-lock.json'):
            if not (root / package / filename).is_file():
                errors.append(f'{package}: missing {filename}')
    make = (root / 'Makefile').read_text()
    targets = set(re.findall(r'^([a-z][a-z0-9-]*):', make, re.M))
    just = (root / 'justfile').read_text()
    for recipe in re.findall(r'^    (.+)$', just, re.M):
        if recipe == '@just --list':
            continue
        if not recipe.startswith('make ') or recipe[5:] not in targets:
            errors.append(f'Just recipe does not delegate to a Make target: {recipe}')
    return errors


if __name__ == '__main__':
    problems = check(ROOT)
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        sys.exit(1)
    print('Repository inventory, entry links, lockfiles, and aliases passed.')
