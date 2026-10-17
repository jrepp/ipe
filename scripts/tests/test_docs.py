"""Check the generated site contract, without asserting noisy timing thresholds."""
import json
import math
from html.parser import HTMLParser
from pathlib import Path
import unittest
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "target/docs-site"


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.targets = []

    def handle_starttag(self, tag, attrs):
        for key, value in attrs:
            if key in ("href", "src") and value:
                self.targets.append(value)


class DocsContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads((SITE / "docs/data/site-data.json").read_text())

    def test_export_metadata(self):
        self.assertEqual(self.data["schema_version"], 1)
        self.assertEqual(self.data["profile"], "release")
        for key in ("generated_at", "revision", "rustc", "platform"):
            self.assertTrue(self.data[key])

    def test_real_parser_examples_cover_requirements(self):
        examples = self.data["examples"]
        self.assertEqual(len(examples), 3)
        kinds = set()

        def visit(node):
            self.assertIsInstance(node["label"], str)
            self.assertIsInstance(node["children"], list)
            kinds.add(node["kind"])
            for child in node["children"]:
                visit(child)

        for example in examples:
            self.assertIn(f'policy {example["name"]}:', example["source"])
            self.assertEqual(example["tree"]["kind"], "Policy")
            visit(example["tree"])
        self.assertTrue({"Policy", "Requires", "Denies", "Where", "In", "Logical", "Binary", "Path", "Literal"} <= kinds)

    def test_measurements_are_consistent(self):
        self.assertEqual([w["comparisons"] for w in self.data["workloads"]], [1, 8, 32])
        for workload in self.data["workloads"]:
            for key in ("p50_us", "p95_us", "p99_us", "max_us", "throughput"):
                self.assertTrue(math.isfinite(workload[key]))
                self.assertGreaterEqual(workload[key], 0)
            self.assertGreater(workload["throughput"], 0)
            self.assertLessEqual(workload["p50_us"], workload["p95_us"])
            self.assertLessEqual(workload["p95_us"], workload["p99_us"])
            self.assertLessEqual(workload["p99_us"], workload["max_us"])
            quantiles = workload["quantiles"]
            self.assertEqual([q["percentile"] for q in quantiles], list(range(1, 101)))
            self.assertEqual([q["us"] for q in quantiles], sorted(q["us"] for q in quantiles))
            self.assertEqual(quantiles[98]["us"], workload["p99_us"])
            self.assertEqual(workload["samples"], 10000)
            self.assertTrue(0 <= workload["outliers"] <= workload["samples"])

    def test_html_links_and_assets_exist_in_artifact(self):
        for page in [SITE / "index.html", *(SITE / "docs").glob("*.html")]:
            parser = Links()
            parser.feed(page.read_text())
            for target in parser.targets:
                url = urlsplit(target)
                if url.scheme or url.netloc or not url.path:
                    continue
                with self.subTest(page=page.name, target=target):
                    self.assertTrue((page.parent / unquote(url.path)).exists())

    def test_packaged_data_matches_local_data(self):
        self.assertEqual(self.data, json.loads((ROOT / "docs/data/site-data.json").read_text()))


if __name__ == "__main__":
    unittest.main()
