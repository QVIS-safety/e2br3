"""Actual PostgreSQL bootstrap checks, using only newly created test databases.

MIGRATION_TEST_ADMIN_URL must be an explicit administrator connection.
Small fixture schemas test runner failure handling; the repository bootstrap
test executes the real, complete frozen application schema.
"""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from urllib.parse import urlsplit, urlunsplit
import uuid

ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / 'deploy/ec2/migrate-rds.sh'
spec = importlib.util.spec_from_file_location('bootstrap', RUNNER.with_suffix('.py'))
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)


class PostgresBootstrap(unittest.TestCase):
    def setUp(self):
        self.admin = os.environ['MIGRATION_TEST_ADMIN_URL']
        parts = urlsplit(self.admin)
        if not parts.username or parts.scheme not in ('postgres', 'postgresql'):
            raise ValueError('MIGRATION_TEST_ADMIN_URL must include an administrator user')
        self.database = 'e2br3_test_bootstrap_' + uuid.uuid4().hex[:12]
        subprocess.run(['createdb', '--maintenance-db', self.admin, self.database], check=True, capture_output=True)
        self.addCleanup(self.drop_database)
        target = urlunsplit((parts.scheme, parts.netloc, '/' + self.database, parts.query, ''))
        self.env = dict(os.environ, DATABASE_URL=target, APP_DB_USER=parts.username)
        self.temporary = tempfile.TemporaryDirectory(prefix='bootstrap-test-')
        self.addCleanup(self.temporary.cleanup)
        self.db_dir = Path(self.temporary.name)
        (self.db_dir / 'bootstrap').mkdir()
        self.source = self.db_dir / 'bootstrap/01.sql'
        self.source.write_text("CREATE TABLE preserved(id int PRIMARY KEY, value text); INSERT INTO preserved VALUES (1, 'original');")

    def drop_database(self):
        if not self.database.startswith('e2br3_test_bootstrap_'):
            raise ValueError('Unsafe cleanup target')
        subprocess.run(['dropdb', '--maintenance-db', self.admin, '--force', self.database], check=True, capture_output=True)

    def sql(self, query):
        result = subprocess.run(['psql', '-XAtw', '-d', self.env['DATABASE_URL'], '-v', 'ON_ERROR_STOP=1', '-c', query], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def run_bootstrap(self, *args, succeeds=True, directory=None):
        result = subprocess.run([str(RUNNER), '--db-dir', str(self.db_dir if directory is None else directory), *args], env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode == 0, succeeds, result.stdout[-1000:] + result.stderr[-3000:])
        return result

    def test_initialize_check_and_repeat(self):
        self.run_bootstrap('--initialize')
        self.run_bootstrap('--check')
        self.run_bootstrap()
        self.assertEqual(self.sql('SELECT value FROM preserved'), 'original')
        self.assertIn('empty public schema', self.run_bootstrap('--initialize', succeeds=False).stderr)

    def test_failure_rolls_back_entire_initialization(self):
        (self.db_dir / 'bootstrap/02.sql').write_text('SELECT 1/0;')
        self.run_bootstrap('--initialize', succeeds=False)
        self.assertEqual(self.sql("SELECT to_regclass('public.preserved') IS NULL AND to_regclass('public.schema_baseline') IS NULL"), 't')
        (self.db_dir / 'bootstrap/02.sql').write_text('SELECT 1;')
        self.run_bootstrap('--initialize')

    def test_changed_baseline_fails_without_writes(self):
        self.run_bootstrap('--initialize')
        self.source.write_text('DROP TABLE preserved;')
        self.assertIn('baseline mismatch', self.run_bootstrap('--check', succeeds=False).stderr)
        self.assertEqual(self.sql('SELECT value FROM preserved'), 'original')

    def test_existing_unmarked_database_is_never_adopted(self):
        self.sql('CREATE TABLE existing(id int); INSERT INTO existing VALUES (7);')
        self.assertIn('baseline is missing', self.run_bootstrap('--check', succeeds=False).stderr)
        self.run_bootstrap('--initialize', succeeds=False)
        self.assertEqual(self.sql('SELECT id FROM existing'), '7')

    def test_concurrent_initializer_is_rejected(self):
        lock = subprocess.Popen(['psql', '-XAtw', '-d', self.env['DATABASE_URL']], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            lock.stdin.write(f'BEGIN; SELECT pg_advisory_xact_lock({bootstrap.LOCK_KEY});\n\\echo locked\n')
            lock.stdin.flush()
            while lock.stdout.readline().strip() != 'locked':
                if lock.poll() is not None:
                    self.fail('Lock holder exited before acquiring lock')
            self.assertIn('database lock', self.run_bootstrap('--initialize', succeeds=False).stderr)
        finally:
            lock.communicate('ROLLBACK;\n\\q\n', timeout=10)

    def test_repository_schema_and_reference_data(self):
        self.run_bootstrap('--initialize', directory=ROOT / 'db')
        self.run_bootstrap('--check', directory=ROOT / 'db')
        self.assertEqual(self.sql('SELECT count(*) FROM public.schema_baseline'), '1')
        self.assertEqual(self.sql("SELECT to_regclass('public.schema_migrations') IS NULL"), 't')
        self.assertEqual(self.sql('SELECT count(*) FROM public.cases'), '0')
        self.assertEqual(self.sql('SELECT count(*) FROM public.organization_as2_profiles'), '0')
        self.assertEqual(self.sql('SELECT count(*) FROM public.iso_countries'), '249')
        self.assertEqual(self.sql('SELECT count(*) FROM public.fda_hierarchical_code_lists'), '487')
        self.assertEqual(self.sql("SELECT count(*) FROM public.e2b_code_lists WHERE list_name='drug_characterization' AND code='4'"), '1')
        self.assertEqual(self.sql("SELECT rolcanlogin OR rolinherit OR NOT rolbypassrls FROM pg_roles WHERE rolname='e2br3_whodrug_search_owner'"), 'f')
        self.assertEqual(self.sql("SELECT pg_get_userbyid(proowner) FROM pg_proc WHERE oid='public.search_active_whodrug_products(text,bigint)'::regprocedure"), 'e2br3_whodrug_search_owner')
        expected = json.loads((ROOT / 'db/bootstrap-inventory.json').read_text())
        actual = json.loads(self.sql((ROOT / 'scripts/db/schema-inventory.sql').read_text()))
        self.assertEqual(actual, expected['objects'])


if __name__ == '__main__':
    unittest.main()
