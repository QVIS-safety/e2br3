# Cross-organization HTTP fuzzer

Creates **five new test organizations and three users per organization** (one sponsor admin and two custom-role operators), entirely through the real HTTP API. Each run gets a new namespace. Uses the opt-in synthetic MedDRA fixture **28.1 / 99999999**; these are isolated test results, not production or licensed terminology evidence.

## Run

Use the repository's isolated runner with its normal database credentials/configuration. Never use the shared development or production database:

```sh
UI_TEST_TERMINOLOGY_FIXTURE=1 \
SERVICE_BOOTSTRAP_ENABLED=true \
E2BR3_FRONTEND_ROOT=/path/to/e2br3-frontend \
scripts/ui-env.sh bombadil_api_fuzz_example 18961 18962
```

In another terminal, use the PID of that runner's **web-server** process:

```sh
node --test scripts/api-fuzzer/run.test.mjs
node scripts/api-fuzzer/run.mjs \
  --environment bombadil_api_fuzz_example --backend-pid SERVER_PID \
  --seed 20261123 --cases 100 --hops 5
```

Reproduce one indexed case without running the preceding cases:

```sh
node scripts/api-fuzzer/run.mjs \
  --environment bombadil_api_fuzz_example --backend-pid SERVER_PID \
  --seed 20261123 --case-index 13 --hops 5
```

Requires Linux `/proc`, Node 22+, the existing built backend, PostgreSQL/UI-runner dependencies, bootstrap accounts, and the frontend checkout containing `scripts/bombadil/xml-cross-organization.mjs` and `xml-preservation.mjs`. No npm dependencies are added. The runner verifies the process name, loopback bind address, exact named test database, and explicit fixture flag before making requests. Credentials/cookies are never logged. The bootstrap account is read from the repository bootstrap constants and only used in that verified test environment.

`--cases` and `--hops` default to 100 and 5. Cases rotate through all five source organizations and ICH/FDA/MFDS; field order, users and the next organization are seeded. Each page receives one common ICH contract field and, when applicable, one field for the case authority. Valid values, prerequisites and fixtures come directly from the editor contracts. The default 100 cases schedule every current contract field and the run fails if any field does not complete a verified import/export. Each case retains one authority across its hops, so changing formats cannot excuse regional data loss. C.1.2 deliberately advances before each hop to exercise follow-up versions on return visits. Expected target versions come from previously observed cases in the run. Intentional mutations are restored before exporting.

## Checks and evidence

- Independently compare generated inputs against persisted editor-contract projections before exporting. Decimal projections use the contract's explicit fixed scale. `checkedFields` is updated only after the populated values survive the first cross-organization import/export.
- Compare all 11 editor graphs plus message headers after each import, retaining array order, duplicates, nulls, empty collections, missing fields, dates, fractions, booleans and Unicode/XML escaping.
- Verify fresh IDs, correct parent references, target organization, target sender link and expected version using the existing cross-organization comparator. Raw differences remain in the artifacts.
- Reject cross-organization reads; compare reads by different users within the target organization.
- Check repeated export equality and that exporting/importing leaves the source unchanged.
- Reject blank strings, false E.i.3.2 markers, and the redundant `EQ` qualifier without changing the case; save then explicitly clear a narrative field with null and check omitted fields remain intact.
- Continue forwarding successfully imported cases after a value mismatch, while retaining the failure and returning a nonzero exit code. A structural/API failure stops that case; other cases continue.

Artifacts default to `tmp/api-fuzzer/<timestamp>-<seed>/`; `--output` may specify a **new** directory. `report.json` contains counts, phases, `contractFields`, `checkedFields`, `uncheckedFields`, failures, source hashes and organization/user IDs. `requests.jsonl` contains requests/responses with credentials removed. Per-case plans name their target fields; before/after snapshots, exported XML and comparisons make each failure inspectable. A copy of the running script is retained. Reusing a seed reproduces generated choices; the namespace and server-generated UUIDs/timestamps change. Failed runs are never overwritten or marked successful. Test data stays in the named DB for inspection; stop its runner before deliberately resetting it with `scripts/ui-env.sh reset NAME`.

## Scope

This is a sequential, stateful integrity fuzzer, not a concurrency or throughput benchmark. The default campaign targets every field in the 11 current editor contracts, including nested arrays, nullFlavor and regional values. It uses each contract's reviewed valid literal rather than inventing unconstrained values; the existing Rust dictionary boundary matrix remains complementary. Product/sender catalog mapping combinations beyond the generated per-organization product, arbitrary malformed XML, concurrency, and all authorization role combinations remain outside scope. Empty or merely scheduled fields are never counted as checked.
