# Case state verifier

This is the real-system, model-based Case and Presave state verifier. It is not
random-input fuzzing. The grammar selects uncovered state transitions and each
runner compares the headed UI, canonical API, PostgreSQL state, reload or
relogin state, and audit-chain integrity.

The suite writes test data. Run it only against a dedicated verification
database and task-owned UI/API processes. It does not provide mock data,
fallback terminology, or a fake-success mode.

## Required environment

- `API_URL`: real task API base URL
- `UI_URL`: real task UI base URL
- `STATE_VERIFIER_DB`: dedicated PostgreSQL database name
- `POSTGRES_CONTAINER`: PostgreSQL container used for raw-state checks
- `FIREFOX_BINARY`: headed Firefox binary
- `GECKODRIVER`: geckodriver binary
- `DISPLAY`: headed browser display

The database must contain the real terminology needed by the selected
transitions. In particular, v5 rejects an empty active `meddra_terms` table
instead of substituting fixture terms.

## Flow

Run `seed.py` first. It creates isolated source records and `seed-result.json`
through the real API. Then run the applicable transition scripts in grammar
order and finish with `verify_persistence_oracles.py`.

The v2 continuation scripts consume the preceding result file. The v3-v5
scripts build on `run_concurrent_edit_v3.py` helpers and the same seeded Case.
Generated JSON, screenshots, and traces stay local and are ignored by Git.

Example:

```sh
python scripts/case-state-verifier/seed.py
python scripts/case-state-verifier/run_import_roundtrip.py
python scripts/case-state-verifier/verify_persistence_oracles.py
```

Any missing credential, runtime, database, terminology release, or preceding
result is a hard failure.
