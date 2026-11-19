"""Regression checks for repository drift and missing release outputs."""
import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


repository = load('check-repository')
release = load('check-release-artifact')


class RepositoryContract(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        # Copy only contract inputs; link targets need existence, not their content.
        for name in ('Cargo.toml', 'Makefile', 'justfile', *repository.ENTRY_DOCS):
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, target)
        import re
        from urllib.parse import urlsplit
        for name in repository.ENTRY_DOCS:
            source = self.root / name
            for target in re.findall(r'\]\(([^)\s]+)\)', source.read_text()):
                url = urlsplit(target)
                if not url.scheme and not url.netloc and url.path:
                    path = (source.parent / url.path).resolve()
                    if path.is_relative_to(self.root) and not path.exists():
                        path.parent.mkdir(parents=True, exist_ok=True)
                        if (ROOT / path.relative_to(self.root)).is_dir():
                            path.mkdir()
                        else:
                            path.touch()
        import tomllib
        for member in tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['members']:
            path = self.root / member / 'Cargo.toml'
            path.parent.mkdir(parents=True, exist_ok=True)
            path.touch()
        for package in repository.NODE_PACKAGES:
            (self.root / package).mkdir(parents=True, exist_ok=True)
            for name in ('package.json', 'package-lock.json'):
                (self.root / package / name).touch()

    def test_valid_contract(self):
        self.assertEqual(repository.check(self.root), [])

    def test_mapping_drift(self):
        path = self.root / 'docs/repository-guide.md'
        original = path.read_text()
        cases = (
            original.replace('`crates/ipe-core`', '`crates/missing`'),
            original + '\n| `crates/ipe-core` | implemented | P1 | duplicate |\n',
            original.replace('| implemented |', '| unknown |', 1),
        )
        for text in cases:
            with self.subTest(text=text[-80:]):
                path.write_text(text)
                self.assertTrue(repository.check(self.root))

    def test_missing_member_manifest(self):
        (self.root / 'crates/ipe-core/Cargo.toml').unlink()
        self.assertTrue(any('missing Cargo.toml' in e for e in repository.check(self.root)))

    def test_missing_link_and_lockfile(self):
        (self.root / 'QUICKSTART.md').write_text('[broken](absent.md)')
        (self.root / 'docs/package-lock.json').unlink()
        errors = repository.check(self.root)
        self.assertTrue(any('absent.md' in e for e in errors))
        self.assertTrue(any('package-lock.json' in e for e in errors))

    def test_invalid_alias(self):
        (self.root / 'justfile').write_text('build:\n    make absent\n')
        self.assertTrue(any('Just recipe' in e for e in repository.check(self.root)))


class ReleaseArtifact(unittest.TestCase):
    def test_missing_empty_and_valid_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for target, name in [('x86_64-unknown-linux-gnu', 'ipe-server'),
                                 ('x86_64-pc-windows-msvc', 'ipe-server.exe')]:
                with self.subTest(target=target):
                    with self.assertRaises(FileNotFoundError):
                        release.artifact_size(root, target)
                    path = root / target / 'release' / name
                    path.parent.mkdir(parents=True)
                    path.touch()
                    with self.assertRaises(ValueError):
                        release.artifact_size(root, target)
                    path.write_bytes(b'example artifact')
                    self.assertEqual(release.artifact_size(root, target), 16)
