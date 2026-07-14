import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import import_mfds_products as importer


FIXTURES = Path(__file__).parent / "fixtures"


class MfdsProductImportTests(unittest.TestCase):
    def fixture(self, name: str) -> bytes:
        return (FIXTURES / name).read_bytes()

    def test_collects_all_pages_merges_duplicates_and_preserves_cancellation(self):
        pages = iter(
            [
                self.fixture("mfds-products-page-1.json"),
                self.fixture("mfds-products-page-2.json"),
            ]
        )
        requested_pages = []

        def fetch(page: int, rows: int, service_key: str) -> bytes:
            requested_pages.append(page)
            self.assertEqual(2, rows)
            self.assertEqual("secret-key", service_key)
            return next(pages)

        artifact, raw_pages = importer.collect_products(
            "2026-07-14", "secret-key", fetch=fetch, rows_per_page=2
        )

        self.assertEqual([1, 2], requested_pages)
        self.assertEqual(2, len(raw_pages))
        self.assertEqual("mfds_product", artifact["dictionary"])
        self.assertEqual(3, len(artifact["products"]))
        self.assertEqual(
            ["200000001", "200000002", "200000003"],
            [row["item_seq"] for row in artifact["products"]],
        )
        cancelled = artifact["products"][1]
        self.assertEqual("20251231", cancelled["cancel_date"])
        self.assertEqual("취하", cancelled["cancel_name"])

    def test_conflicting_duplicate_identity_is_rejected(self):
        first = json.loads(self.fixture("mfds-products-page-1.json"))
        second = json.loads(self.fixture("mfds-products-page-2.json"))
        second["body"]["items"][0]["ITEM_NAME"] = "다른제품"
        pages = iter([json.dumps(first).encode(), json.dumps(second).encode()])

        with self.assertRaisesRegex(ValueError, "conflicting ITEM_SEQ 200000001"):
            importer.collect_products(
                "v1", "key", fetch=lambda *_: next(pages), rows_per_page=2
            )

    def test_api_error_is_rejected(self):
        error = json.dumps(
            {"header": {"resultCode": "03", "resultMsg": "NO_DATA"}}
        ).encode()

        with self.assertRaisesRegex(ValueError, "MFDS API error 03"):
            importer.collect_products(
                "v1", "key", fetch=lambda *_: error, rows_per_page=2
            )

    def test_atomic_artifacts_and_raw_pages_never_contain_service_key(self):
        pages = iter(
            [
                self.fixture("mfds-products-page-1.json"),
                self.fixture("mfds-products-page-2.json"),
            ]
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / "normalized.json"
            raw_dir = root / "raw"
            importer.collect_to_paths(
                version="v1",
                service_key="do-not-persist",
                output=output,
                raw_dir=raw_dir,
                fetch=lambda *_: next(pages),
                rows_per_page=2,
            )

            files = [output, *sorted(raw_dir.glob("*.json"))]
            self.assertEqual(3, len(files))
            for path in files:
                self.assertNotIn("do-not-persist", path.read_text())
            self.assertFalse(list(root.rglob("*.tmp")))

    def test_fetch_percent_encodes_key_without_logging_it(self):
        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = b"{}"
        with mock.patch("urllib.request.urlopen", return_value=response) as open_url:
            importer.fetch_page(1, 100, "a+b/c=")

        request = open_url.call_args.args[0]
        self.assertIn("serviceKey=a%2Bb%2Fc%3D", request.full_url)

    def test_live_collection_requires_environment_key(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(ValueError, "DATA_GO_KR_SERVICE_KEY"):
                importer.service_key_from_environment()

    def test_rows_per_page_cannot_exceed_mfds_api_limit(self):
        with self.assertRaisesRegex(ValueError, "must be between 1 and 500"):
            importer.collect_products(
                "v1", "key", fetch=lambda *_: b"{}", rows_per_page=501
            )

    @unittest.skipUnless(
        os.environ.get("DATA_GO_KR_SERVICE_KEY"),
        "DATA_GO_KR_SERVICE_KEY is not configured",
    )
    def test_live_api_returns_a_valid_product_page(self):
        raw = importer.fetch_page(
            1, 1, importer.service_key_from_environment()
        )
        _, body = importer._response_parts(raw)
        items = importer._body_items(body)
        self.assertTrue(items)
        importer._normalize_product(items[0])


if __name__ == "__main__":
    unittest.main()
