"""Keep old guide links usable after the documentation path migration."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
GUIDES = {
    'ADMIN_ARCHITECTURE_CRITIQUE.md': 'admin-architecture-critique.md',
    'ADMIN_ARCHITECTURE_REVIEW.md': 'admin-architecture-review.md',
    'ARCHITECTURE.md': 'architecture-guide.md',
    'CHANGELOG_LEGACY.md': 'changelog-legacy.md',
    'CREDENTIAL_ENCLAVE.md': 'credential-enclave.md',
    'DOCKER.md': 'docker-guide.md',
    'ENTERPRISE_SCALE_PLANNING.md': 'enterprise-scale-planning.md',
    'GITHUB_CONTRACT_SPIKE.md': 'github-contract-spike.md',
    'INDEX.md': 'documentation-index.md',
    'INTEGRATION_ASSESSMENTS.md': 'integration-assessments.md',
    'INTEGRATION_DISCOVERY.md': 'integration-discovery.md',
    'INTEGRATION_DOMAIN_MODEL.md': 'integration-domain-model.md',
    'INTEGRATION_HOST.md': 'integration-host.md',
    'INTEGRATION_MODEL_IMPLEMENTATION.md': 'integration-model-implementation.md',
    'INTEGRATION_MODEL_REVIEW.md': 'integration-model-review.md',
    'INTEGRATION_SDK.md': 'integration-sdk.md',
    'INTEGRATION_STACK.md': 'integration-stack.md',
    'MERGE_QUEUE.md': 'merge-queue.md',
    'PRODUCT_SHIP_DEMO.md': 'product-ship-demo.md',
    'RELEASES.md': 'release-guide.md',
    'REPOSITORY.md': 'repository-guide.md',
    'RUNTIME_DATA.md': 'runtime-data.md',
    'TEST_RESULTS.md': 'test-results.md',
    'VOCABULARY.md': 'vocabulary-guide.md',
    'VOCABULARY_REVIEW.md': 'vocabulary-review.md',
}


def headings(text):
    """Compute the viewer's heading IDs for the guides' Markdown headings."""
    seen = {}
    result = []
    for heading in re.findall(r'^#{1,6}\s+(.+?)\s*#*$', text, re.M):
        label = re.sub(r'\[([^\]]+)\]\([^)]*\)', r'\1', heading).replace('`', '')
        slug = re.sub(r'[^\w\s-]', '', label.lower()).strip().replace(' ', '-')
        count = seen.get(slug, 0)
        seen[slug] = count + 1
        result.append(slug + (f'-{count}' if count else ''))
    return result


class GuideCompatibility(unittest.TestCase):
    def test_legacy_pages_and_section_links_are_packaged(self):
        for old, new in GUIDES.items():
            with self.subTest(old=old):
                legacy = (ROOT / 'docs' / old).read_text()
                canonical = (ROOT / 'docs' / new).read_text()
                self.assertTrue(legacy.startswith('# Document moved\n'))
                self.assertIn(f']({new})', legacy)
                links = re.findall(r'\]\(' + re.escape(new) + r'#([^)]*)\)', legacy)
                self.assertEqual(headings(legacy)[1:], headings(canonical))
                self.assertEqual(links, headings(canonical))
                for path in (old, new):
                    self.assertTrue((ROOT / 'target/docs-site/docs' / path).is_file(), path)
