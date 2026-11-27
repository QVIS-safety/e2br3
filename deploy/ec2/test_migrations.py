"""Isolated bootstrap-runner contract tests, not production validation."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('bootstrap', Path(__file__).with_name('migrate-rds.py'))
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)
ROOT = Path(__file__).resolve().parents[2]


class BootstrapFiles(unittest.TestCase):
    def test_frozen_files_are_complete_and_checksummed(self):
        files = bootstrap.read_files(ROOT / 'db/bootstrap')
        self.assertEqual(len(files), 7)
        self.assertFalse(list((ROOT / 'db/migrations').glob('*.sql')))
        for name, checksum, content in files:
            self.assertEqual(checksum, hashlib.sha256(content.encode()).hexdigest())

    def test_missing_empty_and_symlink_sources_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            for target in (directory, directory / 'absent'):
                with self.assertRaises(ValueError):
                    bootstrap.read_files(target)
            (directory / '01.sql').symlink_to(ROOT / 'db/bootstrap/00-roles.sql')
            with self.assertRaisesRegex(ValueError, 'Invalid SQL'):
                bootstrap.read_files(directory)

    def test_execution_uses_verified_snapshot(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / '01.sql'
            source.write_text('SELECT 1;')
            files = bootstrap.read_files(root)
            source.write_text('SELECT 2;')
            script = bootstrap.script_for(files, [], root, initialize=True)
            self.assertEqual((root / 'bootstrap-01.sql').read_text(), 'SELECT 1;')
            self.assertIn('pg_try_advisory_xact_lock', script)
            self.assertIn('empty public schema', script)

    def test_default_check_is_read_only_and_does_not_include_sql(self):
        with tempfile.TemporaryDirectory() as temporary:
            script = bootstrap.script_for([('01.sql', 'hash', 'DROP TABLE existing;')], [], Path(temporary))
            self.assertIn('SET TRANSACTION READ ONLY', script)
            self.assertNotIn('DROP TABLE', script)
            self.assertNotIn('CREATE TABLE', script)
            self.assertIn('automatic adoption/reset is forbidden', script)


if __name__ == '__main__':
    unittest.main()
