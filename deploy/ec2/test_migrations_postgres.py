"""Actual PostgreSQL runner tests. Fixtures exercise isolated migration logic only.

MIGRATION_TEST_ADMIN_URL must identify a real database administrator connection.
Only randomly named databases created by this suite are modified or dropped.
"""
import json
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest
from urllib.parse import urlsplit, urlunsplit
import uuid

ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / 'deploy/ec2/migrate-rds.sh'
spec = importlib.util.spec_from_file_location('migrations', RUNNER.with_suffix('.py'))
migrations = importlib.util.module_from_spec(spec)
spec.loader.exec_module(migrations)


class PostgresMigrations(unittest.TestCase):
    def setUp(self):
        admin = os.environ['MIGRATION_TEST_ADMIN_URL']
        parts = urlsplit(admin)
        if not parts.username or parts.scheme not in ('postgres', 'postgresql'):
            raise ValueError('MIGRATION_TEST_ADMIN_URL must include an explicit administrator user')
        self.database = 'e2br3_test_migrations_' + uuid.uuid4().hex[:12]
        self.env = dict(os.environ, PGDATABASE=admin)
        self.env.pop('DATABASE_URL', None)
        self.env.pop('SERVICE_MIGRATION_DB_URL', None)
        result = subprocess.run(['createdb', '--maintenance-db', admin, self.database], env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.addCleanup(self.drop_database, admin, self.database)
        target = urlunsplit((parts.scheme, parts.netloc, '/' + self.database, parts.query, ''))
        self.env.update(PGDATABASE=target, DATABASE_URL=target, APP_DB_USER=parts.username,
                        PGAPPNAME='e2br3-migration-tests')
        temporary = tempfile.TemporaryDirectory(prefix='migration-test-')
        self.addCleanup(temporary.cleanup)
        self.db_dir = Path(temporary.name)
        (self.db_dir / 'bootstrap').mkdir()
        (self.db_dir / 'migrations').mkdir()
        (self.db_dir / 'bootstrap/01.sql').write_text('CREATE TABLE preserved(id integer PRIMARY KEY, value text NOT NULL);')
        self.write('001.sql', "INSERT INTO preserved VALUES (1, 'original');")

    def drop_database(self, admin, database):
        if not database.startswith('e2br3_test_migrations_'):
            raise ValueError('Unsafe test database cleanup')
        result = subprocess.run(['dropdb', '--maintenance-db', admin, database], env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def write(self, name, sql):
        (self.db_dir / 'migrations' / name).write_text(sql)

    def sql(self, sql):
        result = subprocess.run(['psql', '-XqAtw', '-d', self.env['PGDATABASE'], '-v', 'ON_ERROR_STOP=1', '-c', sql], env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def run_migrations(self, *args, succeeds=True, db_dir=None):
        directory = self.db_dir if db_dir is None else db_dir
        result = subprocess.run([str(RUNNER), '--db-dir', str(directory), *args], env=self.env, capture_output=True, text=True)
        if succeeds:
            self.assertEqual(result.returncode, 0, result.stdout[-1000:] + result.stderr[-5000:])
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        return result

    def test_initialize_upgrade_and_repeat(self):
        self.run_migrations('--initialize')
        first = self.sql('SELECT filename,checksum,applied_at FROM schema_migrations ORDER BY filename')
        self.run_migrations()
        self.assertEqual(first, self.sql('SELECT filename,checksum,applied_at FROM schema_migrations ORDER BY filename'))
        self.write('002.sql', "ALTER TABLE preserved ADD COLUMN note text; UPDATE preserved SET note='upgraded';")
        self.assertIn('pending 002.sql', self.run_migrations('--check').stdout)
        self.assertEqual(self.sql('SELECT count(*) FROM schema_migrations'), '1')
        self.run_migrations()
        self.assertEqual(self.sql('SELECT value,note FROM preserved'), 'original|upgraded')
        self.assertEqual(self.sql('SELECT count(*) FROM schema_migrations'), '2')
        self.run_migrations('--initialize', succeeds=False)

    def test_sql_failure_rolls_back_file_and_retry_succeeds(self):
        self.run_migrations('--initialize')
        self.write('002.sql', "UPDATE preserved SET value='lost'; CREATE TABLE must_rollback(id int); SELECT 1/0;")
        self.run_migrations(succeeds=False)
        self.assertEqual(self.sql('SELECT value FROM preserved'), 'original')
        self.assertEqual(self.sql("SELECT to_regclass('must_rollback') IS NULL"), 't')
        self.assertEqual(self.sql('SELECT count(*) FROM schema_migrations'), '1')
        self.write('002.sql', "UPDATE preserved SET value='changed';")
        self.run_migrations()
        self.assertEqual(self.sql('SELECT value FROM preserved'), 'changed')

    def test_history_failure_also_rolls_back_sql(self):
        self.run_migrations('--initialize')
        self.sql("CREATE FUNCTION reject_history() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test history write failure'; END $$; CREATE TRIGGER reject_history BEFORE INSERT ON schema_migrations FOR EACH ROW EXECUTE FUNCTION reject_history();")
        self.write('002.sql', "UPDATE preserved SET value='lost';")
        self.assertIn('test history write failure', self.run_migrations(succeeds=False).stderr)
        self.assertEqual(self.sql('SELECT value FROM preserved'), 'original')
        self.assertEqual(self.sql('SELECT count(*) FROM schema_migrations'), '1')

    def test_unknown_changed_and_out_of_order_history_rejected_before_writes(self):
        self.run_migrations('--initialize')
        self.write('002.sql', "UPDATE preserved SET value='lost';")
        original = (self.db_dir / 'migrations/001.sql').read_text()
        self.write('001.sql', original + '\n')
        self.assertIn('modified applied', self.run_migrations(succeeds=False).stderr)
        self.write('001.sql', original)
        self.write('000.sql', 'SELECT 1;')
        self.assertIn('Out-of-order', self.run_migrations(succeeds=False).stderr)
        (self.db_dir / 'migrations/000.sql').unlink()
        (self.db_dir / 'migrations/001.sql').unlink()
        self.assertIn('Unknown', self.run_migrations(succeeds=False).stderr)
        self.assertEqual(self.sql('SELECT value FROM preserved'), 'original')

    def test_missing_history_and_baseline_rejected_without_writes(self):
        self.sql('CREATE TABLE existing(id int); INSERT INTO existing VALUES (7)')
        self.assertIn('history is missing', self.run_migrations('--check', succeeds=False).stderr)
        self.run_migrations(succeeds=False)
        self.run_migrations('--initialize', succeeds=False)
        self.env['BASELINE_IF_EMPTY'] = '1'
        self.assertIn('forbidden', self.run_migrations(succeeds=False).stderr)
        self.assertEqual(self.sql('SELECT id FROM existing'), '7')
        self.assertEqual(self.sql("SELECT to_regclass('schema_migrations') IS NULL"), 't')

    def test_two_runners_cannot_apply_simultaneously(self):
        self.run_migrations('--initialize')
        self.write('002.sql', "SELECT pg_sleep(3); UPDATE preserved SET value='once';")
        with subprocess.Popen([str(RUNNER), '--db-dir', str(self.db_dir)], env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) as first:
            try:
                for _ in range(60):
                    if self.sql("SELECT count(*) FROM pg_stat_activity WHERE application_name='e2br3-migration-tests' AND query LIKE 'SELECT pg_sleep(3)%' AND state='active'") == '1':
                        break
                    time.sleep(0.05)
                else:
                    self.fail('First runner did not enter its migration')
                self.assertIn('database lock', self.run_migrations(succeeds=False).stderr)
            finally:
                stdout, stderr = first.communicate(timeout=15)
            self.assertEqual(first.returncode, 0, stdout + stderr)
        self.assertEqual(self.sql('SELECT count(*) FROM schema_migrations'), '2')
        self.assertEqual(self.sql('SELECT value FROM preserved'), 'once')

    def test_full_repository_initialization_and_legacy_atomicity(self):
        self.run_migrations('--initialize', db_dir=ROOT / 'db')
        expected = migrations.read_migrations(ROOT / 'db/migrations')
        self.assertEqual(self.sql('SELECT count(*) FROM schema_migrations'), str(len(expected)))
        self.assertEqual(json.loads(self.sql('SELECT json_object_agg(filename,checksum) FROM schema_migrations')),
                         {name: checksum for name, checksum, _, _ in expected})
        before = self.sql('SELECT filename,checksum,applied_at FROM schema_migrations ORDER BY filename')
        self.run_migrations(db_dir=ROOT / 'db')
        self.assertEqual(before, self.sql('SELECT filename,checksum,applied_at FROM schema_migrations ORDER BY filename'))

    def test_runtime_reconciliation_is_atomic_and_repeatable(self):
        repair = '20261102_reconcile_runtime_schema.sql'
        shutil.rmtree(self.db_dir / 'bootstrap')
        shutil.copytree(ROOT / 'db/bootstrap', self.db_dir / 'bootstrap')
        (self.db_dir / 'migrations/001.sql').unlink()
        for path in (ROOT / 'db/migrations').glob('*.sql'):
            if path.name < repair:
                shutil.copyfile(path, self.db_dir / 'migrations' / path.name)
        self.run_migrations('--initialize')
        # Isolated drift fixture; this is not evidence of production execution history.
        self.sql("""BEGIN;
          SELECT set_current_user_context('00000000-0000-0000-0000-000000000001');
          DROP FUNCTION public.search_active_whodrug_products(text,bigint);
          DROP TRIGGER audit_fda_device_information ON fda_device_information;
          DROP TRIGGER audit_fda_device_codes ON fda_device_codes;
          DROP TRIGGER audit_source_documents ON source_documents;
          ALTER TABLE case_validation_summaries DROP COLUMN validation_revision;
          ALTER TABLE patient_information DROP CONSTRAINT patient_information_age_group_check,
            ADD CONSTRAINT patient_information_age_group_check CHECK (age_group IN ('1','2','3','4','5','6'));
          DELETE FROM e2b_code_lists WHERE (list_name,code) IN (('age_group','0'),('drug_characterization','4'));
          DROP FUNCTION audit_jsonb_is_distinct(jsonb,jsonb);
          CREATE FUNCTION reject_history() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
            RAISE EXCEPTION 'repair history failure'; END $$;
          CREATE TRIGGER reject_history BEFORE INSERT ON schema_migrations
            FOR EACH ROW EXECUTE FUNCTION reject_history();
          COMMIT;""")
        before_schema = self.schema_description()
        before_audit = self.sql('SELECT count(*),max(id) FROM audit_logs')
        shutil.copyfile(ROOT / 'db/migrations' / repair, self.db_dir / 'migrations' / repair)
        self.assertIn('repair history failure', self.run_migrations(succeeds=False).stderr)
        self.assertEqual(before_schema, self.schema_description())
        self.assertEqual(before_audit, self.sql('SELECT count(*),max(id) FROM audit_logs'))
        self.assertEqual(self.sql("SELECT count(*) FROM schema_migrations WHERE filename='" + repair + "'"), '0')
        self.sql('DROP TRIGGER reject_history ON schema_migrations; DROP FUNCTION reject_history();')
        self.run_migrations()
        self.assertEqual(self.sql("SELECT count(*) FROM e2b_code_lists WHERE active AND ((list_name='age_group' AND code='0' AND display_name='Foetus') OR (list_name='drug_characterization' AND code='4' AND display_name='Drug Not Administered'))"), '2')
        self.assertEqual(self.sql("SELECT count(*) FROM pg_trigger WHERE NOT tgisinternal AND tgenabled='O' AND tgfoid='audit_trigger_function()'::regprocedure AND tgname IN ('audit_fda_device_information','audit_fda_device_codes','audit_source_documents')"), '3')
        self.assertEqual(self.sql("SELECT to_regprocedure('audit_jsonb_is_distinct(jsonb,jsonb)') IS NOT NULL"), 't')
        self.assertEqual(self.sql("SELECT data_type || ':' || is_nullable FROM information_schema.columns WHERE table_schema='public' AND table_name='case_validation_summaries' AND column_name='validation_revision'"), 'text:YES')
        after_schema = self.schema_description()
        after_audit = self.sql('SELECT count(*),max(id) FROM audit_logs')
        self.run_migrations()
        self.sql('BEGIN; SET LOCAL search_path=public,pg_catalog; ' + (ROOT / 'db/migrations' / repair).read_text() + ' COMMIT;')
        self.assertEqual(after_schema, self.schema_description())
        self.assertEqual(after_audit, self.sql('SELECT count(*),max(id) FROM audit_logs'))

    def test_runtime_reconciliation_audit_and_value_contracts(self):
        self.run_migrations('--initialize', db_dir=ROOT / 'db')
        self.sql("""BEGIN;
          SELECT set_current_user_context('00000000-0000-0000-0000-000000000001');
          DO $$
          DECLARE
            fixture_user_id uuid := '00000000-0000-0000-0000-000000000001';
            case_key uuid; drug_key uuid; device_key uuid; code_key uuid; document_key uuid;
          BEGIN
            INSERT INTO cases(organization_id,created_by)
              VALUES ('00000000-0000-0000-0000-000000000000',fixture_user_id) RETURNING id INTO case_key;
            INSERT INTO patient_information(case_id,age_group,created_by) VALUES(case_key,'0',fixture_user_id);
            INSERT INTO test_results(case_id,sequence_number,test_name,test_date_null_flavor,created_by)
              VALUES(case_key,1,'reconciliation fixture','MSK',fixture_user_id),
                    (case_key,2,'reconciliation fixture','UNK',fixture_user_id),
                    (case_key,3,'reconciliation fixture',NULL,fixture_user_id);
            BEGIN
              UPDATE patient_information SET age_group='7' WHERE case_id=case_key;
              RAISE EXCEPTION 'invalid age group accepted';
            EXCEPTION WHEN check_violation THEN NULL;
            END;
            BEGIN
              UPDATE test_results SET test_date_null_flavor='NI' WHERE case_id=case_key;
              RAISE EXCEPTION 'invalid null flavor accepted';
            EXCEPTION WHEN check_violation THEN NULL;
            END;
            INSERT INTO drug_information(case_id,sequence_number,drug_characterization,medicinal_product,created_by)
              VALUES(case_key,1,'4','reconciliation fixture',fixture_user_id) RETURNING id INTO drug_key;
            INSERT INTO fda_device_information(drug_id,sequence_number,device_brand_name,created_by)
              VALUES(drug_key,1,'before',fixture_user_id) RETURNING id INTO device_key;
            INSERT INTO fda_device_codes(device_id,element,sequence_number,value_code,created_by)
              VALUES(device_key,'follow_up_type',1,'1',fixture_user_id) RETURNING id INTO code_key;
            INSERT INTO source_documents(case_id,sequence_number,source_document_name,created_by)
              VALUES(case_key,1,'before',fixture_user_id) RETURNING id INTO document_key;
            UPDATE fda_device_information SET updated_at=now() WHERE id=device_key;
            UPDATE fda_device_codes SET updated_at=now() WHERE id=code_key;
            UPDATE source_documents SET updated_at=now() WHERE id=document_key;
            UPDATE fda_device_information SET device_brand_name='한글 변경' WHERE id=device_key;
            UPDATE fda_device_codes SET value_code='2' WHERE id=code_key;
            UPDATE source_documents SET source_document_name='한글 변경' WHERE id=document_key;
            DELETE FROM fda_device_codes WHERE id=code_key;
            DELETE FROM fda_device_information WHERE id=device_key;
            DELETE FROM source_documents WHERE id=document_key;
            IF (SELECT count(*) FROM (
              SELECT record_id FROM audit_logs WHERE record_id IN (device_key,code_key,document_key)
              GROUP BY record_id HAVING array_agg(action::text ORDER BY id)=ARRAY['CREATE','UPDATE','DELETE']
            ) audited) <> 3 THEN
              RAISE EXCEPTION 'missing or duplicate CRUD audit events';
            END IF;
            IF NOT EXISTS (SELECT FROM audit_logs WHERE record_id=document_key AND action='UPDATE'
              AND changed_fields->'source_document_name'=jsonb_build_object('old','before','new','한글 변경')) THEN
              RAISE EXCEPTION 'Unicode audit value was lost';
            END IF;
            IF EXISTS (
              WITH samples(value) AS (VALUES (NULL::jsonb),('null'::jsonb),('1'::jsonb),('1.0'::jsonb),
                ('{}'::jsonb),('{"nested":{"한글":"값"}}'::jsonb),('[1,null,"é"]'::jsonb),
                (to_jsonb(repeat('한글',10000))))
              SELECT FROM samples a CROSS JOIN samples b
              WHERE audit_jsonb_is_distinct(a.value,b.value) IS DISTINCT FROM (a.value IS DISTINCT FROM b.value)
            ) THEN RAISE EXCEPTION 'JSONB equality semantics changed'; END IF;
          END $$;
          ROLLBACK;""")
        self.assertEqual(self.sql("SELECT has_function_privilege('e2br3_app_role','public.search_active_whodrug_products(text,bigint)','EXECUTE') AND NOT EXISTS (SELECT FROM pg_proc p, LATERAL aclexplode(p.proacl) a WHERE p.oid='public.search_active_whodrug_products(text,bigint)'::regprocedure AND a.grantee=0 AND a.privilege_type='EXECUTE')"), 't')
        self.assertEqual(self.sql("SELECT has_table_privilege('e2br3_whodrug_search_owner','public.whodrug_products','SELECT') AND has_table_privilege('e2br3_whodrug_search_owner','public.terminology_releases','SELECT') AND NOT has_table_privilege('e2br3_whodrug_search_owner','public.cases','SELECT')"), 't')
        self.assertEqual(self.sql("SET ROLE e2br3_app_role; SELECT count(*) FROM public.search_active_whodrug_products(NULL,10);"), '0')

    def schema_description(self):
        return self.sql("""SELECT string_agg(description, E'\\n' ORDER BY description) FROM (
          SELECT 'column ' || c.relname || '.' || a.attname || ' ' || format_type(a.atttypid,a.atttypmod) || ' ' || a.attnotnull || ' ' || COALESCE(pg_get_expr(d.adbin,d.adrelid),'') AS description
          FROM pg_attribute a JOIN pg_class c ON c.oid=a.attrelid JOIN pg_namespace n ON n.oid=c.relnamespace
          LEFT JOIN pg_attrdef d ON d.adrelid=c.oid AND d.adnum=a.attnum
          WHERE n.nspname='public' AND a.attnum>0 AND NOT a.attisdropped AND c.relkind IN ('r','p')
          UNION ALL SELECT 'constraint ' || c.relname || ' ' || con.conname || ' ' || pg_get_constraintdef(con.oid) FROM pg_constraint con JOIN pg_class c ON c.oid=con.conrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public'
          UNION ALL SELECT 'function ' || pg_get_functiondef(p.oid) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public' AND p.prokind='f'
          UNION ALL SELECT 'index ' || indexdef FROM pg_indexes WHERE schemaname='public'
          UNION ALL SELECT 'trigger ' || pg_get_triggerdef(t.oid) FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND NOT t.tgisinternal
          UNION ALL SELECT 'policy ' || row(p.*)::text FROM pg_policies p WHERE schemaname='public'
        ) descriptions""")

    def test_real_legacy_history_failure_and_upgrade_matches_fresh(self):
        self.run_migrations('--initialize', db_dir=ROOT / 'db')
        fresh_schema = self.schema_description()
        second = 'e2br3_test_migrations_' + uuid.uuid4().hex[:12]
        admin = os.environ['MIGRATION_TEST_ADMIN_URL']
        result = subprocess.run(['createdb', '--maintenance-db', admin, second], env=self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.addCleanup(self.drop_database, admin, second)
        parts = urlsplit(admin)
        target = urlunsplit((parts.scheme, parts.netloc, '/' + second, parts.query, ''))
        self.env.update(PGDATABASE=target, DATABASE_URL=target)
        shutil.rmtree(self.db_dir / 'bootstrap')
        shutil.copytree(ROOT / 'db/bootstrap', self.db_dir / 'bootstrap')
        (self.db_dir / 'migrations/001.sql').unlink()
        legacy = '20261007_drug_not_administered_terminology.sql'
        for path in (ROOT / 'db/migrations').glob('*.sql'):
            if path.name < legacy:
                shutil.copyfile(path, self.db_dir / 'migrations' / path.name)
        self.run_migrations('--initialize')
        self.sql("BEGIN; SELECT set_current_user_context('00000000-0000-0000-0000-000000000001'); UPDATE e2b_code_lists SET display_name='atomic rollback probe' WHERE list_name='drug_characterization' AND code='4'; COMMIT;")
        before = self.sql("SELECT display_name FROM e2b_code_lists WHERE list_name='drug_characterization' AND code='4'")
        self.sql("CREATE FUNCTION reject_history() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'legacy history failure'; END $$; CREATE TRIGGER reject_history BEFORE INSERT ON schema_migrations FOR EACH ROW EXECUTE FUNCTION reject_history();")
        self.assertIn('legacy history failure', self.run_migrations(db_dir=ROOT / 'db', succeeds=False).stderr)
        self.assertEqual(before, self.sql("SELECT display_name FROM e2b_code_lists WHERE list_name='drug_characterization' AND code='4'"))
        self.assertEqual(self.sql("SELECT count(*) FROM schema_migrations WHERE filename='" + legacy + "'"), '0')
        self.sql('DROP TRIGGER reject_history ON schema_migrations; DROP FUNCTION reject_history();')
        self.run_migrations(db_dir=ROOT / 'db')
        self.assertEqual(fresh_schema, self.schema_description())


if __name__ == '__main__':
    unittest.main()
