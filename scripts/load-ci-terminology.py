#!/usr/bin/env python3
"""Load the pinned private Oracle subset into the isolated XML test database."""

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import urlsplit


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / 'scripts/db/ci-terminology-manifest.json'
COLUMNS = {
    'meddra_terms': 'code term level version language active'.split(),
    'controlled_terminology_terms': 'dictionary version language scope code display_name active'.split(),
    'mfds_products': ('item_seq product_name_kr product_name_en manufacturer_name_kr '
                      'manufacturer_name_en permit_date cancellation_date cancellation_status version active').split(),
    'mfds_product_substances': ('item_seq substance_code substance_name_kr substance_name_en quantity unit '
                                'component_content material_sequence total_amount_sequence version active').split(),
}


def read_snapshot(path, manifest):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != manifest['sha256']:
        raise ValueError('Oracle terminology snapshot SHA-256 does not match the pinned manifest')
    snapshot = json.loads(data)
    tables = snapshot['tables']
    if set(tables) != set(COLUMNS):
        raise ValueError('Oracle terminology snapshot table set differs from the manifest')
    for table, columns in COLUMNS.items():
        rows = tables[table]
        if len(rows) != manifest['rows'][table]:
            raise ValueError(f'Oracle terminology snapshot row count differs: {table}')
        if any(set(row) != set(columns) or row['active'] is not True for row in rows):
            raise ValueError(f'Oracle terminology snapshot columns or active status differ: {table}')
    releases = snapshot['source_releases']
    actual = {(r['dictionary'], r['version'], r['language']) for r in releases if r['status'] == 'active'}
    expected = {(r['dictionary'], r['version'], r['language']) for r in manifest['releases']}
    if len(releases) != len(expected) or actual != expected:
        raise ValueError('Oracle terminology snapshot active releases differ from the manifest')
    return snapshot


def isolated_database(url, expected):
    if not re.fullmatch(r'e2br3_test_[a-z0-9_]+', expected) or urlsplit(url).path != '/' + expected:
        raise ValueError('Oracle terminology may only load into the isolated E2BR3_TEST_DATABASE_NAME')


def insert_sql(table, columns, rows):
    # JSON is a SQL string value; identifiers come only from the fixed schema above.
    value = json.dumps(rows, ensure_ascii=False).replace("'", "''")
    fields = ', '.join(columns)
    return f"INSERT INTO public.{table} ({fields}) SELECT {fields} FROM json_populate_recordset(NULL::public.{table}, '{value}');"


def load(snapshot, manifest, url, database):
    isolated_database(url, database)
    statements = [
        "SET LOCAL standard_conforming_strings = on;",
        f"DO $$ BEGIN IF current_database() <> '{database}' THEN RAISE EXCEPTION 'isolated database mismatch'; END IF; END $$;",
        "SET LOCAL ROLE e2br3_app_role;",
        "SELECT set_current_user_context('00000000-0000-0000-0000-000000000001');",
        "SELECT set_config('app.current_user_role', 'system_admin', true);",
        "SELECT set_authorization_isolation_context('00000000-0000-0000-0000-000000000000', true);",
    ]
    releases = []
    for source in snapshot['source_releases']:
        release = next(r for r in manifest['releases'] if r['dictionary'] == source['dictionary'])
        releases.append({
            'dictionary': source['dictionary'], 'version': source['version'], 'language': source['language'],
            'status': source['status'], 'loaded_rows': release['subset_rows'],
            'source_path': 'oracle-ci-subset:' + snapshot['source'], 'source_checksum': manifest['sha256'],
            'note': f"CI subset; original source rows={source['loaded_rows']}; original checksum={source['source_checksum']}",
        })
    statements.append(insert_sql('terminology_releases', list(releases[0]), releases))
    for table, columns in COLUMNS.items():
        statements.append(insert_sql(table, columns, snapshot['tables'][table]))
    result = subprocess.run(
        ['psql', '-X', '-q', '--single-transaction', '-v', 'ON_ERROR_STOP=1', '-v', 'VERBOSITY=sqlstate'],
        env={**os.environ, 'PGDATABASE': url}, input='\n'.join(statements), text=True, capture_output=True,
    )
    if result.returncode:
        # PostgreSQL details may contain licensed row values; expose SQLSTATE only.
        states = re.findall(r'ERROR:\s+([0-9A-Z]{5})\b', result.stderr)
        raise ValueError(f'Oracle terminology import failed (psql exit {result.returncode}, SQLSTATE {states}); transaction rolled back')
    print('Loaded verified Oracle terminology subset into isolated XML test database')


def main():
    manifest = json.loads(MANIFEST.read_text())
    snapshot_path = os.environ.get('E2BR3_CI_TERMINOLOGY_SNAPSHOT')
    if not snapshot_path:
        raise ValueError('web-server XML tests require E2BR3_CI_TERMINOLOGY_SNAPSHOT (private Oracle snapshot; see scripts/db/ci-terminology-manifest.json)')
    snapshot = read_snapshot(Path(snapshot_path), manifest)
    load(snapshot, manifest, os.environ['SERVICE_DB_URL'], os.environ['E2BR3_TEST_DATABASE_NAME'])


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError) as error:
        print(f'CI terminology requirement failed: {error}', file=sys.stderr)
        sys.exit(1)
