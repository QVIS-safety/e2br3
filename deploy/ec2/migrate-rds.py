#!/usr/bin/env python3
"""Run immutable migrations through psql with one lock and a transaction per file."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


# Only these reviewed, byte-identical files may have their transaction wrappers removed.
LEGACY_TRANSACTIONS = {
    "20260804_cioms_dechallenge_result.sql": ("0fad6c30ae44c19bdb56beedd9cd87711064ddf83fdd0e9c9103c517653e5aad", 13, 22),
    "20260820_repurpose_user_blind_flag.sql": ("7ffe54720ea77a2b256bc6e7cc524239aa93829b2bc7687c6ff5c929252eed79", 2, 15),
    "20260909_allow_draft_drug_role.sql": ("95d74c241c18f6d643ba8aef0001768c7b3f54a2256fce8848e36d1ca552219b", 2, 8),
    "20261005_patient_age_group_foetus.sql": ("9f0048f45f02e07f5ce95e953ae47c6b3cbed0ca373235d0f35a39ca2efb00af", 1, 30),
    "20261007_drug_not_administered_terminology.sql": ("701a8ac09e4b825e583f08be24394a694d1e4a79aafb977550c3544bc31551cb", 1, 12),
}
LOCK_KEY = "1768378981, 1835624306"


def validate_sql(sql):
    """Inspect top-level SQL only; quoted function bodies belong to PostgreSQL."""
    tokens = []
    i = 0
    while i < len(sql):
        if sql[i].isspace():
            i += 1
        elif sql.startswith("--", i):
            end = sql.find("\n", i)
            i = len(sql) if end < 0 else end + 1
        elif sql.startswith("/*", i):
            depth = 1
            i += 2
            while depth and i < len(sql):
                if sql.startswith("/*", i):
                    depth += 1
                    i += 2
                elif sql.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            if depth:
                raise ValueError("Unterminated SQL comment")
        elif sql[i] in "'\"":
            quote = sql[i]
            escaped = quote == "'" and i > 0 and sql[i - 1] in "eE" and (i < 2 or not (sql[i - 2].isalnum() or sql[i - 2] == '_'))
            i += 1
            while i < len(sql):
                if escaped and sql[i] == "\\":
                    i += 2
                elif sql[i] == quote:
                    i += 1
                    if i < len(sql) and sql[i] == quote:
                        i += 1
                    else:
                        break
                else:
                    i += 1
            else:
                raise ValueError("Unterminated SQL quote")
        elif sql[i] == '$' and (tag := re.match(r'\$(?:[A-Za-z_][A-Za-z_0-9]*)?\$', sql[i:])):
            delimiter = tag.group()
            end = sql.find(delimiter, i + len(delimiter))
            if end < 0:
                raise ValueError("Unterminated SQL dollar quote")
            i = end + len(delimiter)
        elif sql[i] == '\\':
            raise ValueError("psql commands are not allowed in migrations")
        elif sql[i] == ';':
            check_statement(tokens)
            tokens = []
            i += 1
        elif word := re.match(r'[A-Za-z_][A-Za-z_0-9]*', sql[i:]):
            tokens.append(word.group().upper())
            i += len(word.group())
        else:
            i += 1
    check_statement(tokens)


def check_statement(tokens):
    if not tokens:
        return
    if tokens[0] in {"BEGIN", "START", "COMMIT", "END", "ROLLBACK", "ABORT", "SAVEPOINT", "RELEASE"} or tokens[:2] == ["PREPARE", "TRANSACTION"]:
        raise ValueError("Transaction control belongs to the migration runner")
    if tokens[0] in {"SET", "RESET"} and any(t in tokens for t in ("STANDARD_CONFORMING_STRINGS", "ALL")):
        raise ValueError("Migrations must preserve standard_conforming_strings")


def read_migrations(directory):
    if not directory.is_dir():
        raise ValueError(f"Missing migrations directory: {directory}")
    migrations = []
    for path in sorted(directory.glob('*.sql')):
        if path.is_symlink() or not path.is_file() or not re.fullmatch(r'[0-9][A-Za-z_0-9-]*\.sql', path.name):
            raise ValueError(f"Invalid migration file: {path.name}")
        raw = path.read_bytes()
        checksum = hashlib.sha256(raw).hexdigest()
        sql = raw.decode('utf-8')
        legacy = path.name in LEGACY_TRANSACTIONS
        if legacy:
            expected, begin, commit = LEGACY_TRANSACTIONS[path.name]
            if checksum != expected:
                raise ValueError(f"Legacy migration checksum mismatch: {path.name}")
            lines = sql.splitlines(keepends=True)
            if lines[begin - 1].strip() != 'BEGIN;' or lines[commit - 1].strip() != 'COMMIT;':
                raise ValueError(f"Invalid legacy transaction wrapper: {path.name}")
            sql = ''.join(line for index, line in enumerate(lines, 1) if index not in (begin, commit))
        try:
            validate_sql(sql)
        except ValueError as error:
            raise ValueError(f"{path.name}: {error}") from error
        migrations.append((path.name, checksum, sql, legacy))
    if not migrations:
        raise ValueError("No migration SQL files found")
    return migrations


def literal(value):
    return "'" + value.replace("'", "''") + "'"


def history_guard(manifest):
    return f"""
DO $migration_guard$
DECLARE problem text;
BEGIN
    IF to_regclass('public.schema_migrations') IS NULL THEN
        RAISE EXCEPTION 'Migration history is missing; explicitly reconcile the existing database. Automatic baselining is forbidden.';
    END IF;
    IF NOT EXISTS (SELECT FROM public.schema_migrations) THEN
        RAISE EXCEPTION 'Migration history is empty; explicitly reconcile the existing database.';
    END IF;
    SELECT m.filename INTO problem FROM public.schema_migrations m
    LEFT JOIN jsonb_each_text({manifest}::jsonb) e ON e.key = m.filename
    WHERE e.key IS NULL OR e.value <> m.checksum ORDER BY m.filename LIMIT 1;
    IF problem IS NOT NULL THEN
        RAISE EXCEPTION 'Unknown or modified applied migration: %', problem;
    END IF;
    SELECT e.key INTO problem FROM jsonb_each_text({manifest}::jsonb) e
    LEFT JOIN public.schema_migrations m ON m.filename = e.key
    WHERE m.filename IS NULL AND e.key COLLATE "C" < (SELECT max(filename COLLATE "C") FROM public.schema_migrations)
    ORDER BY e.key LIMIT 1;
    IF problem IS NOT NULL THEN
        RAISE EXCEPTION 'Out-of-order pending migration: %; reconcile explicitly', problem;
    END IF;
END $migration_guard$;
"""


def script_for(db_dir, migrations, directory, initialize=False, seed=False, check=False):
    manifest = literal(json.dumps({name: checksum for name, checksum, _, _ in migrations}))
    script = ["SET standard_conforming_strings = on;", "SET search_path = public, pg_catalog;",
              "SET lock_timeout = '5s';", "SET statement_timeout = '15min';"]
    if check:
        script += ['BEGIN READ ONLY;', history_guard(manifest),
                   f"SELECT 'pending ' || e.key FROM jsonb_each_text({manifest}::jsonb) e LEFT JOIN public.schema_migrations m ON m.filename=e.key WHERE m.filename IS NULL ORDER BY e.key COLLATE \"C\";",
                   'COMMIT;']
        return '\n'.join(script)
    script.append(f"DO $$ BEGIN IF NOT pg_try_advisory_lock({LOCK_KEY}) THEN RAISE EXCEPTION 'Another migration runner holds the database lock'; END IF; END $$;")

    def include(name, content):
        path = directory / name
        path.write_text(content, encoding='utf-8')
        escaped = str(path).replace('\\', '\\\\').replace("'", "\\'")
        return "\\i '" + escaped + "'"

    def group(name):
        paths = sorted((db_dir / name).glob('*.sql'))
        if not paths:
            raise ValueError(f"No {name} SQL files found")
        for path in paths:
            script.extend([f'\\echo ==> {name}/{path.name}', 'BEGIN;',
                           include(name + '-' + path.name, path.read_text()), 'COMMIT;'])

    if initialize:
        script.append("""DO $$ BEGIN
IF EXISTS (SELECT FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public')
OR EXISTS (SELECT FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public')
OR EXISTS (SELECT FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE n.nspname='public') THEN
    RAISE EXCEPTION 'Initialization requires an empty public schema; existing databases must use verified migration history';
END IF; END $$;""")
        group('bootstrap')
        script.append("""CREATE TABLE public.schema_migrations (
filename text PRIMARY KEY, checksum text NOT NULL, applied_at timestamptz NOT NULL DEFAULT now());
REVOKE ALL ON public.schema_migrations FROM PUBLIC;""")
    else:
        script.append(history_guard(manifest))
    for name, checksum, sql, legacy in migrations:
        script += [f"SELECT NOT EXISTS (SELECT FROM public.schema_migrations WHERE filename={literal(name)}) AS pending \\gset",
                   '\\if :pending', f'\\echo ==> migrations/{name}', 'BEGIN;']
        if legacy:
            script.append('\\echo Verified legacy transaction wrapper removed; recording original SHA-256')
        script += [include(name, sql),
                   f"INSERT INTO public.schema_migrations(filename, checksum) VALUES ({literal(name)}, {literal(checksum)});",
                   'COMMIT;', '\\endif']
    if seed:
        group('seed')
    script += [f'SELECT pg_advisory_unlock({LOCK_KEY});', '\\echo Database migrations complete.']
    return '\n'.join(script) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--db-dir', type=Path)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument('--initialize', action='store_true', help='Bootstrap an empty database and record every migration')
    mode.add_argument('--check', action='store_true', help='Read-only history verification and pending migration list')
    mode.add_argument('--validate-only', action='store_true', help='Validate files without connecting to a database')
    parser.add_argument('--seed', action='store_true', help='Explicitly load development seed during initialization')
    args = parser.parse_args()
    if args.seed and not args.initialize:
        parser.error('--seed requires --initialize')
    for key in ('BASELINE', 'BASELINE_IF_EMPTY'):
        if key in os.environ and os.environ[key] != '0':
            parser.error(f'{key} is forbidden; verify and reconcile existing database history explicitly')
    db_dir = args.db_dir if args.db_dir is not None else Path(os.environ['PROJECT_DIR'] if 'PROJECT_DIR' in os.environ else Path.cwd()) / 'db'
    migrations = read_migrations(db_dir / 'migrations')
    if args.validate_only:
        print(f'Validated {len(migrations)} migration files')
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
    command = ['psql', '-X', '-w', '-d', env['PGDATABASE'], '-v', 'ON_ERROR_STOP=1', '-f', '-']
    if args.initialize:
        if 'APP_DB_USER' not in env or not env['APP_DB_USER']:
            parser.error('APP_DB_USER is required for initialization')
        command += ['-v', 'app_db_user=' + env['APP_DB_USER']]
    with tempfile.TemporaryDirectory(prefix='e2br3-migrations-') as temporary:
        script = script_for(db_dir, migrations, Path(temporary), args.initialize, args.seed, args.check)
        return subprocess.run(command, input=script, text=True, env=env).returncode


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(f'Migration error: {error}', file=sys.stderr)
        sys.exit(1)
