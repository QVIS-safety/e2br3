#!/usr/bin/env python3
"""Initialize an empty database from the frozen schema, or verify its baseline.

Historical entrypoint name retained for deployment compatibility. Existing
databases are never upgraded, reset, or silently marked compatible.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

LOCK_KEY = "1768378981, 1835624306"


def read_files(directory):
    if not directory.is_dir():
        raise ValueError(f"Missing SQL directory: {directory}")
    files = []
    for path in sorted(directory.glob('*.sql')):
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"Invalid SQL file: {path}")
        raw = path.read_bytes()
        files.append((path.name, hashlib.sha256(raw).hexdigest(), raw.decode('utf-8')))
    if not files:
        raise ValueError(f"No SQL files found: {directory}")
    return files


def literal(value):
    return "'" + value.replace("'", "''") + "'"


def baseline_guard(manifest):
    return f"""DO $baseline_guard$
BEGIN
    IF to_regclass('public.schema_baseline') IS NULL THEN
        RAISE EXCEPTION 'Frozen schema baseline is missing. Existing databases require explicit schema verification or a new empty database; automatic adoption/reset is forbidden.';
    END IF;
    IF (SELECT count(*) FROM public.schema_baseline) <> 1
       OR NOT EXISTS (SELECT FROM public.schema_baseline WHERE singleton AND manifest = {manifest}::jsonb) THEN
        RAISE EXCEPTION 'Frozen schema baseline mismatch. No database changes were made.';
    END IF;
END $baseline_guard$;"""


def script_for(files, seed_files, directory, initialize=False):
    manifest = literal(json.dumps({name: checksum for name, checksum, _ in files}))
    script = ["SET LOCAL search_path = public, pg_catalog;",
              "SET LOCAL lock_timeout = '5s';", "SET LOCAL statement_timeout = '15min';"]
    if not initialize:
        return '\n'.join(['SET TRANSACTION READ ONLY;', *script, baseline_guard(manifest),
                          '\\echo Frozen schema baseline verified.']) + '\n'
    script.append(f"DO $$ BEGIN IF NOT pg_try_advisory_xact_lock({LOCK_KEY}) THEN RAISE EXCEPTION 'Another initializer holds the database lock'; END IF; END $$;")
    script.append("""DO $$ BEGIN
IF EXISTS (SELECT FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public')
OR EXISTS (SELECT FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public')
OR EXISTS (SELECT FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE n.nspname='public') THEN
    RAISE EXCEPTION 'Initialization requires an empty public schema. Existing data will not be changed.';
END IF; END $$;""")
    for group, contents in [('bootstrap', files), ('seed', seed_files)]:
        for name, _, content in contents:
            path = directory / (group + '-' + name)
            path.write_text(content, encoding='utf-8')
            escaped = str(path).replace('\\', '\\\\').replace("'", "\\'")
            script.extend([f'\\echo ==> {group}/{name}', "\\i '" + escaped + "'"])
    script.extend(["""CREATE TABLE public.schema_baseline (
singleton boolean PRIMARY KEY CHECK (singleton), manifest jsonb NOT NULL);
REVOKE ALL ON public.schema_baseline FROM PUBLIC;""",
                   f"INSERT INTO public.schema_baseline VALUES (true, {manifest}::jsonb);",
                   '\\echo Frozen schema initialization complete.'])
    return '\n'.join(script) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db-dir', type=Path)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--initialize', action='store_true', help='Atomically initialize an empty database')
    mode.add_argument('--check', action='store_true', help='Read-only baseline verification (default)')
    mode.add_argument('--validate-only', action='store_true', help='Read and checksum SQL files without a database')
    parser.add_argument('--seed', action='store_true', help='Explicitly include development seed at initialization')
    args = parser.parse_args()
    if args.seed and not args.initialize:
        parser.error('--seed requires --initialize')
    for key in ('BASELINE', 'BASELINE_IF_EMPTY'):
        if key in os.environ and os.environ[key] != '0':
            parser.error(f'{key} is forbidden; automatic adoption of existing databases is not supported')
    db_dir = args.db_dir if args.db_dir is not None else Path(os.environ['PROJECT_DIR'] if 'PROJECT_DIR' in os.environ else Path.cwd()) / 'db'
    files = read_files(db_dir / 'bootstrap')
    seed_files = read_files(db_dir / 'seed') if args.seed else []
    if args.validate_only:
        print(f'Validated {len(files)} frozen bootstrap files')
        return 0
    env = dict(os.environ)
    if 'DATABASE_URL' in env:
        env['PGDATABASE'] = env.pop('DATABASE_URL')
    elif 'SERVICE_MIGRATION_DB_URL' in env:
        env['PGDATABASE'] = env.pop('SERVICE_MIGRATION_DB_URL')
    elif not all(key in env and env[key] for key in ('PGDATABASE', 'PGUSER', 'PGHOST')):
        parser.error('DATABASE_URL, SERVICE_MIGRATION_DB_URL, or explicit PGDATABASE/PGUSER/PGHOST is required')
    if not env['PGDATABASE']:
        parser.error('Database connection must not be empty')
    command = ['psql', '-X', '-w', '-d', env['PGDATABASE'], '--single-transaction', '-v', 'ON_ERROR_STOP=1', '-f', '-']
    if args.initialize:
        if 'APP_DB_USER' not in env or not env['APP_DB_USER']:
            parser.error('APP_DB_USER is required for initialization')
        command += ['-v', 'app_db_user=' + env['APP_DB_USER']]
    with tempfile.TemporaryDirectory(prefix='e2br3-bootstrap-') as temporary:
        script = script_for(files, seed_files, Path(temporary), args.initialize)
        return subprocess.run(command, input=script, text=True, env=env).returncode


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(f'Bootstrap error: {error}', file=sys.stderr)
        sys.exit(1)
