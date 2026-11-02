"""Deterministic SQL validation tests, not evidence of database deployment."""
import hashlib
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('migrations', Path(__file__).with_name('migrate-rds.py'))
migrations = importlib.util.module_from_spec(spec)
spec.loader.exec_module(migrations)
ROOT = Path(__file__).resolve().parents[2]


class MigrationFiles(unittest.TestCase):
    def test_repository_files_and_legacy_checksums(self):
        files = migrations.read_migrations(ROOT / 'db/migrations')
        self.assertEqual({name for name, _, _, legacy in files if legacy}, set(migrations.LEGACY_TRANSACTIONS))
        for name, checksum, sql, legacy in files:
            self.assertEqual(checksum, hashlib.sha256((ROOT / 'db/migrations' / name).read_bytes()).hexdigest())
            if legacy:
                self.assertNotIn('\nCOMMIT;', sql)

    def test_transaction_commands_cannot_escape(self):
        for sql in ('BEGIN;', '-- comment\nCOMMIT;', 'SELECT 1; END;', 'ROLLBACK;',
                    'START TRANSACTION;', 'PREPARE TRANSACTION \'x\';',
                    'SAVEPOINT x;', 'RELEASE SAVEPOINT x;', 'ABORT;',
                    'SELECT \'it\'\'s fine\'; CoMmIt;', '/* outer /* inner */ */ COMMIT;',
                    "SELECT E'escaped\\\'quote'; COMMIT;", '\\connect other',
                    'SET standard_conforming_strings=off;', 'RESET ALL;'):
            with self.subTest(sql=sql), self.assertRaises(ValueError):
                migrations.validate_sql(sql)

    def test_function_bodies_strings_and_comments_are_preserved(self):
        migrations.validate_sql('''-- BEGIN; COMMIT;
/* outer /* inner */ END; */
DO $body$ BEGIN RAISE NOTICE 'COMMIT;'; END $body$;
SELECT 'BEGIN; it''s text', E'escape\\\\', "END" FROM example;
''')

    def test_unterminated_constructs_fail(self):
        for sql in ("SELECT 'x", 'SELECT "x', 'DO $body$x', '/* x'):
            with self.subTest(sql=sql), self.assertRaises(ValueError):
                migrations.validate_sql(sql)

    def test_legacy_modification_and_empty_directory_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            with self.assertRaisesRegex(ValueError, 'No migration'):
                migrations.read_migrations(directory)
            name = next(iter(migrations.LEGACY_TRANSACTIONS))
            (directory / name).write_bytes((ROOT / 'db/migrations' / name).read_bytes() + b'\n')
            with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                migrations.read_migrations(directory)

    def test_execution_uses_verified_snapshot(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'migrations').mkdir()
            source = root / 'migrations/001.sql'
            source.write_text('SELECT 1;')
            files = migrations.read_migrations(source.parent)
            source.write_text('COMMIT;')
            out = root / 'out'
            out.mkdir()
            migrations.script_for(root, files, out)
            self.assertEqual((out / '001.sql').read_text(), 'SELECT 1;')

    def test_change_gate_rejects_backdated_migrations(self):
        for added, allowed in (('003.sql', False), ('005.sql', True)):
            with self.subTest(added=added), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                directory = root / 'db/migrations'
                directory.mkdir(parents=True)
                runner = root / 'deploy/ec2/migrate-rds.py'
                runner.parent.mkdir(parents=True)
                shutil.copyfile(ROOT / 'deploy/ec2/migrate-rds.py', runner)

                def git(*args):
                    return subprocess.check_output(
                        ['git', '-c', 'core.hooksPath=/dev/null',
                         '-c', 'user.name=Migration test', '-c', 'user.email=test@example.invalid',
                         '-c', 'commit.gpgsign=false', *args], cwd=root, text=True,
                        stderr=subprocess.PIPE).strip()

                git('init', '-q')
                for name in ('002.sql', '004.sql'):
                    (directory / name).write_text('SELECT 1;')
                git('add', '.')
                git('commit', '-qm', 'Initial fixture')
                base = git('rev-parse', 'HEAD')
                (directory / added).write_text('SELECT 2;')
                git('add', '.')
                git('commit', '-qm', 'Candidate fixture')
                result = subprocess.run(
                    ['sh', str(ROOT / 'scripts/db/check_migration_changes.sh'), base, 'HEAD'],
                    cwd=root, text=True, capture_output=True)
                if allowed:
                    self.assertEqual(result.returncode, 0, result.stderr)
                else:
                    self.assertNotEqual(result.returncode, 0, result.stdout)
                    self.assertIn('sort after', result.stderr)


if __name__ == '__main__':
    unittest.main()
