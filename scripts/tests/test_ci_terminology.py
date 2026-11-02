import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('ci_terminology', ROOT / 'scripts/load-ci-terminology.py')
loader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(loader)


class TerminologyBoundaryTests(unittest.TestCase):
    def test_only_matching_isolated_database_is_allowed(self):
        loader.isolated_database('postgres://host/e2br3_test_abc?sslmode=require', 'e2br3_test_abc')
        for url, name in [('postgres://host/app_db', 'app_db'),
                          ('postgres://host/app_db', 'e2br3_test_abc'),
                          ('postgres://host/e2br3_test_abc', 'e2br3_test_abc;DROP')]:
            with self.assertRaises(ValueError):
                loader.isolated_database(url, name)

    def test_changed_or_incomplete_snapshot_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'snapshot.json'
            path.write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'SHA-256'):
                loader.read_snapshot(path, {'sha256': '0' * 64})
            data = json.dumps({'tables': {}}).encode()
            path.write_bytes(data)
            with self.assertRaisesRegex(ValueError, 'table set'):
                loader.read_snapshot(path, {'sha256': hashlib.sha256(data).hexdigest()})

    def test_json_values_cannot_end_the_sql_literal(self):
        sql = loader.insert_sql('meddra_terms', ['term'], [{'term': "a'); DELETE FROM users; --\\"}])
        self.assertIn("a''); DELETE FROM users; --\\\\", sql)
        self.assertEqual(sql.count('INSERT INTO public.meddra_terms'), 1)

    def test_official_xml_is_preserved_and_only_documented_derivation_exists(self):
        import re
        fixtures = ROOT / 'crates/services/web-server/tests/xml/fixtures/mfds'
        source = (fixtures / 'official/kr-literature-initial-v1.0.xml').read_bytes()
        self.assertEqual(hashlib.sha256(source).hexdigest(),
                         '4125399a0b8c37f7bc3a2cdc518feb864fa0104eaed00b2fe224e96a9c868159')
        derived = (fixtures / 'derived/kr-literature-initial-v1.0-meddra-28.1.xml').read_bytes()
        # Ignore only the explicit provenance comment; keep every original byte otherwise.
        derived = re.sub(rb'<!--.*?-->\r?\n', b'', derived, count=1, flags=re.S)
        expected, versions = re.subn(rb'codeSystemVersion="22\.1"', b'codeSystemVersion="28.1"', source)
        expected, confirmations = re.subn(
            rb'[ \t]*<outboundRelationship2 typeCode="PERT">\r?\n[ \t]*<observation classCode="OBS" moodCode="EVN">\r?\n[ \t]*<code code="24"[^\n]*\r?\n[\s\S]*?</outboundRelationship2>\r?\n',
            b'', expected)
        self.assertEqual((versions, confirmations), (6, 2))
        self.assertEqual(derived, expected)
