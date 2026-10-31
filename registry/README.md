# E2BR3 Registry

This directory is the canonical registry for E2BR3 field coverage.

It is not a documentation dump. The registry is structured data used to verify
that every tracked field has a consistent mapping across:

- E2BR3 code
- authority scope
- backend BMC field
- frontend field

## Authority Rule

`authority` is exactly one of:

- `ICH`
- `FDA`
- `MFDS`

Do not use combined values such as `ICH+FDA`. If a field is FDA-specific, set
`authority` to `FDA`. If a field is MFDS-specific, set `authority` to `MFDS`.

## Layout

- `SPEC.md`: registry schema and consistency contract.
- `schema.json`: formal row contract.
- `dictionary.schema.json`: formal dictionary-file contract.
- `dictionary-rules.schema.json`: formal rules-file contract.
- `index.json`: section file registry.
- `sections/*.json`: canonical editable field rows.
- `editor-contracts/*.json`: field-level editor projection, PATCH, roundtrip,
  and constraint evidence used by strict page gates. Business-validation mappings
  belong to `crates/libs/validator/field-rules/`.
- `execution-ledger.json`: hash-pinned headed UI/Jev proof index for every
  canonical editor identity. CI rejects registry or frontend drift from it.
  Referenced result and lifecycle artifacts remain in private audit storage
  because they can contain Case values; the committed ledger pins their hashes.
- `dictionary/*.json`: canonical E2BR3 data-element dictionaries (generated, committed).
- `dictionary/rules/*.json`: per-authority business-rule prose, keyed by element code
  (generated, committed). Kept out of the entries so the dictionaries stay lean.
- `sources/`: spec source documents the dictionaries are built from.
- `tools/validate.py`: registry validator.
- `tools/build_dictionary.py`: dictionary generator (rerun only when a source spec changes).
- `tools/extract_frontend_fields.py`: frontend input-field extractor.

## Dictionary

The dictionaries are the standards layer: the authoritative list of E2BR3 data
elements per authority, with conformance, data types, allowed values,
nullFlavors, and OIDs. Registry rows are mapping decisions *about* dictionary
elements; the dictionaries define which elements exist at all.

- `dictionary/ich-e2br3.json`: ICH base elements, built from the ICH E2B(R3)
  core data elements and business rules table (`sources/ich-core-data-elements-v1.csv`).
- `dictionary/mfds-regional.json`: official MFDS KR extension elements, built
  from the MFDS Safety R3 business-rule workbook (`sources/mfds-safety-r3-business-rules.xlsx`).
- `dictionary/fda-regional.json`: official FDA regional elements (`FDA.*`),
  built from the FDA combined core/regional table
  (`sources/fda-core-regional-data-elements-v1.csv`). Entries carry per-profile
  conformance (`profiles.post_market` / `pre_market` / `vaers`) and HL7 XPaths.

Known source corrections: `dictionary/ich-e2br3.json` records the following
elements as `optional` even though `sources/ich-core-data-elements-v1.csv`
currently lists them as `Conditional-Mandatory`: `C.2.r.2.5`, `D.8.r.2a`,
`D.8.r.2b`, `D.8.r.3a`, `D.8.r.3b`, `G.k.2.1.1a`, `G.k.2.1.1b`,
`G.k.2.1.2a`, `G.k.2.1.2b`, `G.k.4.r.2`. The ICH E2B(R3) Implementation
Guide PDF `Conformance` rows state these elements are optional. Preserve these
dictionary overrides if the source CSV is regenerated.

C.5.4 includes codes 4–6 from the [ICH E2D(R1) alignment paper](https://www.fda.gov/media/195119/download?attachment=) (16 December 2025, section 3). The dictionary builder reapplies this amendment after reading the older CSV. FDA AEMS accepts these codes; MFDS recipient validation retains 1–3, with CT=1 and CU=2. Input contracts check the shared code format; regional eligibility stays in the validator.

ICH `conditional_mandatory` entries may carry `condition_text` copied from the
ICH E2B(R3) Implementation Guide PDF `Conformance` row. The source CSV does not
consistently include these conditions in `ICH BUSINESS RULE`.

Entries carry structural, machine-consumed facts: conformance, conditional
conformance text, profiles, data types, allowed values, nullFlavors, OIDs, HL7
data types, and XPaths (ICH/MFDS XPaths come from
`sources/mfds-icsr-element-xpath.csv`). Long business-rule prose lives in
`dictionary/rules/{ich,mfds,fda}.json` instead — one `{code: rule}` map per
authority, including each authority's rules for shared ICH elements. The
validator checks that every rule key references an existing dictionary element.

ICH entries with official `VALUE ALLOWED` text also carry an
`allowed_value_constraint`. The raw `allowed_values` source text is always
retained; the structured field classifies it as `code_set`, `boolean`,
`true_marker`, `numeric`, `format`, `vocabulary`, or `descriptive`. Explicit
`code_set` values are extracted in source order. The committed ICH dictionary
contains 223 such entries, including `C.1.10.r`; the source table emits that code
once as a header and again as the actual element, and the element row is
authoritative.

Dictionary files are validated for shape on every `validate.py` run. Codes must
be unique across all dictionary files.

## Commands

Validate the registry:

```sh
python3 registry/tools/validate.py
```

Validate the versioned execution ledger while building the current canonical
UI inventory:

```sh
python3 registry/tools/build_ui_inventory.py \
  --frontend-root ../e2br3-frontend \
  --execution-hints scripts/case-editor-audit/case-edit-persisted-field-inventory.json \
  --execution-ledger registry/execution-ledger.json --check
```

Validate a certified editor contract field by field:

```sh
python3 registry/tools/validate.py --strict-editor-contract CI
python3 registry/tools/validate.py --strict-editor-contract RP
```

All 26 CI fields and 30 RP fields are certified independently. A `complete`
row must include projection, PATCH, roundtrip, and constraint evidence, with
an explicit reason when a stage is not applicable. Business-rule tests and
field mappings are owned by the validator crate. The editor
contract gate does not infer section-level completion.

Canonical fields normally link to generated backend and frontend contract
tests. RE's separate save workflow and DG's runtime relationship IDs use
`automatedTests` on their registry rows, with repository-relative `file` and
`test` references. Backend references use the full Rust module/test name;
frontend references use the Jest title within the specified file.
`test_build_ui_inventory.py` requires both sides for every canonical field and
checks that explicit test declarations exist.

`check_contract_test_results.py` separately requires actual passing results:
Cargo output for backend generated/explicit tests, and Jest JSON for frontend
payload/explicit tests plus each verified invalid-input case. Missing, skipped,
failed, duplicate, or interrupted results fail the gate. Shared explicit tests
are required once. The frontend registry job shares its checked canonical
inventory with the API job, so both gates use the same field set. Dev image
publication requires the selected test jobs to pass. These checks do not turn a blocked
generic contract or historical UI proof into a new UI passing result.

```sh
python3 registry/tools/check_contract_test_results.py \
  --inventory /tmp/canonical-ui-inventory.json --backend-log /tmp/workspace-tests.log
python3 registry/tools/check_contract_test_results.py \
  --inventory /tmp/canonical-ui-inventory.json --frontend-root ../e2br3-frontend \
  --frontend-results /tmp/editor-contract-tests.json
```

Backend page registrations live in `tests/api_editor/pages/{page}.generated.rs`.
Verification lives under `fields/`, `rows/`, `relations/`, and `audit/`, with
shared fixtures in `support.rs`. Frontend payload cases share one `it.each` harness, and
invalid-input cases also report individually by page, authority, code and path.

[API specifications and coverage](../crates/services/web-server/tests/api_specs/README.md)
connect registered HTTP operations to request/response contracts, active test
assertions, and gaps. The index describes source evidence, not test run results.

Case API tests live in `tests/api_cases/contract/`, grouped by review receivers,
list views, validation caching, saves, and deletion/restoration. Shared HTTP
helpers stay in `support.rs`; feature-specific setup stays with its tests.

CI uses the same complete diff selection for PRs and `dev`/`main` pushes.
Production changes run the owning crate and its transitive consumers, using
Cargo's workspace dependency metadata. For example, a web-server change runs
web-server; a lib-core change also runs its consumers. Test-only edits run the
owning integration target; shared test helpers run all tests in that crate.
Renames include both old and new paths. Deleted targets select the remaining
crate tests. Documentation-only changes skip Rust tests and lint jobs.
DB changes select lib-core and its consumers; registry changes select input
contracts, validator and their consumers. Unknown shared inputs select every
crate and are listed as `full_reasons` in the Test Scope job summary.
Formatting uses the selected crates. Clippy checks the owning integration targets
for test-only edits; production changes retain `--all-targets` for the changed
crates and their consumers. Validator retains its dedicated rule-binding gate.
Each other selected crate runs sequentially against its own
isolated PostgreSQL database. `api_editor`, `api_cases`, and `api_import` own
canonical backend assertions. Selecting any of them requires registry/frontend
inventory checks and passing results for every canonical assertion in the
selected targets. A whole-crate run requires every canonical assertion. The
execution checker receives the same target selection as the test runner; a
missing, ignored, failed, or duplicate required result still fails publication.
A targeted run is not a full workspace validation.

Web integration targets follow their directory names:

| Cargo target | Responsibility |
| --- | --- |
| `api_cases` | Case lifecycle, validation, workflow, scopes, intake and child resources |
| `api_editor` | Editor page save/readback, clear, identity and audit contracts |
| `api_presave` | Presave HTTP CRUD and parent/child graph contracts |
| `api_submission` | Dispatch, headers, history, idempotency, callbacks and reconcile |
| `api_terminology` | Dictionary search/import/release and permission contracts |
| `api_administration` | Settings, roles, memberships, branding and audit HTTP behavior |
| `api_import` | XML upload settings, product linkage and import history HTTP behavior |
| `xml` | XML import/export, schema validation and section roundtrips |
| `authz` | Authentication, authorization and RLS through HTTP |

Each target has `tests/<target>.rs` and a matching `tests/<target>/` directory.
Its `support.rs` files contain shared fixtures/helpers, not test cases.
Test-only changes under a directory select that target, including its helpers.
`tests/common/` is shared by targets and selects the whole web crate. Keep
canonical test references in the registry in sync when moving an assertion.

Editor generated-page or contract JSON edits select that page. Changes to a leaf
verifier in `fields/rows/relations/audit` select the corresponding category only
on pages that bind that verifier in the registry. Shared helpers stay in the
parent modules; changing those modules, or a leaf with no registered consumers,
selects the full Editor target. Page/category prefixes are combined into one
`api_editor` invocation with exclusions for unselected categories, so multiple
prefixes share one isolated database setup. The two pure patch-builder checks
also run. The execution checker still requires every selected registered
contract result, exactly once; this selection does not claim a measured runtime
improvement or full Editor coverage for a partial change.

Pure SQL schema/migration and transaction tests live in
`lib-core/tests/database/` and run with `--test database`; they do not require
web-server. The core `--lib` target no longer contains the PostgreSQL transaction
tests. Web-private reconcile-lock and bootstrap checks remain with their owning
implementation; web `--lib`/`--bin` execution still requires the isolated DB runner.

Model authorization storage, revisions, isolation, audit and snapshot tests live
in `lib-core/tests/authorization/`, under the `authorization` Cargo target.
HTTP authorization extraction and route behavior stay in web-server's `authz`
target. Presave model tests live in `lib-core/tests/presave/`, grouped by schema,
access, CRUD, validation, lifecycle, child rows and audit. They retain one
`section_presave` target and share fixtures in its entrypoint.

For a local preview (the base revision must exist):

```sh
python3 scripts/ci_test_plan.py plan --event pull_request --base origin/dev --head HEAD
```

Implementation-layout tests (file placement, import spelling and line limits)
and decorative-copy/order checks were retired where they did not verify an
independent functional requirement. Field contracts, runtime authorization,
audit behavior, NullFlavor and data-preservation tests remain. The minimal
cache-response test was absorbed by the profile-cache persistence/invalidation
suite. Retired test definitions remain in Git history; this cleanup does not
replace the product's approved validation plan or its retained execution records.

| Retired checks | Retained requirement coverage |
| --- | --- |
| Frontend decorative subtitles and CI field display order (2 cases) | Canonical field inventory, payload mapping and input constraints remain; presentation preferences are outside the mandatory regression gate. |
| Frontend admin, DM, LR, NR and utility file-layout checks (14 cases) | Runtime role permissions, section save/readback and audit-access tests remain. Folder names and import spelling are not acceptance criteria. |
| Frontend shared-audit implementation and DG layout checks (11 net cases) | Readonly audit access guard, field audit actions, DG receiver selection, NullFlavor and soft-delete wiring remain. LB/SI functional checks were retained when their layout assertions were removed. |
| Backend minimal FDA validation response (1 case) | `case_contract_web::validation_cache::test_single_profile_validation_caches_each_profile` verifies FDA/MFDS responses, DB cache rows, list aggregation and invalidation after saves. |
| Backend overlapping permission/setup checks and safety-report singleton (10 net cases) | Five create-permission tests retain the original minimal admin payload/201 and viewer payload/403 in one fixture each. Case create/201 remains in viewer-list setup. Stronger audit/organization tests retain the same role/status assertions; the singleton test verifies the repeated safety-report POST and GET return the first ID. |
| Backend legacy authorization migration/source-shape checks (22 cases) | Runtime `authorization_snapshot`, `authorization_isolation`, `authorization_storage`, `rbac_*`, `rls_web`, role metadata/effective-access and compile-fail suites remain; permit/snapshot/organization binding and restore-action guards are retained. |

Validate registry rows against extracted backend BMC fields:

```sh
python3 registry/tools/validate.py --strict-backend-inventory
```

Extract frontend input-field inventory:

```sh
python3 registry/tools/extract_frontend_fields.py
```

Validate registry rows against extracted frontend input fields:

```sh
python3 registry/tools/validate.py --strict-frontend-inventory
```

Validate registry codes against the E2BR3 dictionaries (membership and
mandatory-element coverage):

```sh
python3 registry/tools/validate.py --strict-dictionary
```

Validate the configured presave namespace structurally, or compare it with the
production frontend, Rust model, and presave-to-case transfer inventories:

```sh
python3 registry/tools/validate.py --strict-presave-registry
python3 registry/tools/validate.py --strict-presave-inventory
```

Presave rows live under `registry/presaves/` and use the same row schema as the
case registry. An `e2br3_code` may occur once in each namespace; that code joins
the presave row to its case destination. Strict inventory validation covers all
production presave sections: Sender, Receiver, Product, Reporter, Study, and
Narrative. It verifies frontend fields, Rust storage fields, case joins, and
implemented presave-to-case transfers independently for every section.

Reporter nullFlavor fields with dedicated database columns use local companion
rows, while nullFlavor carried inside another field remains represented by that
field's normal row.

Strict dictionary rules:

- Every row code whose authority has a dictionary must be defined in that
  dictionary. ICH rows must use real ICH element codes; MFDS rows must use
  official KR extension codes; FDA rows must use official FDA regional codes.
  Synthetic placeholder codes (the old `@` convention) are no longer allowed.
- Rows with `"local_only": true` are exempt from membership: they declare a
  real app field that is intentionally **not** an E2BR3 data element (for
  example `G.k.local.rechallenge`, `C.3.receiver.*`, `E.local.*`). A
  `local_only` row must not use a code that exists in a dictionary.
- Every `mandatory` dictionary element must have a registry row.

- `catalog_only`: present in the catalog but not referenced by validator source.

Rebuild the dictionaries after a source spec change (requires `openpyxl`):

```sh
python3 registry/tools/build_dictionary.py
```
