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
publication requires the API job to pass. These checks do not turn a blocked
generic contract or historical UI proof into a new UI passing result.

```sh
python3 registry/tools/check_contract_test_results.py \
  --inventory /tmp/canonical-ui-inventory.json --backend-log /tmp/workspace-tests.log
python3 registry/tools/check_contract_test_results.py \
  --inventory /tmp/canonical-ui-inventory.json --frontend-root ../e2br3-frontend \
  --frontend-results /tmp/editor-contract-tests.json
```

Backend page tests live in `tests/api/case_editor_contract/{page}.rs`, with
generated field declarations under `generated/`. `support.rs` owns shared
fixtures and generic contract verification; DM/AE-specific checks stay with
their page tests. Frontend payload cases share one `it.each` harness, and
invalid-input cases also report individually by page, authority, code and path.

Case API tests live in `tests/api/case_contract/`, grouped by review receivers,
list views, validation caching, saves, and deletion/restoration. Shared HTTP
helpers stay in `support.rs`; feature-specific setup stays with its tests.

CI first separates validator tests from the other backend suites by changed
paths. Validator-only changes skip the registry/audit/database jobs; unrelated
changes skip validator tests. Shared validator inputs run both. Formatting and
workspace Clippy remain enabled. When backend suites are selected, `dev`/`main`
pushes run the full remaining workspace and canonical backend execution check
before publishing the backend image. PRs that change only
known integration-test files run their complete Cargo test targets. Any API
target run still requires all canonical field tests, including explicit RE/DG
references. Documentation-only PRs skip the database test job; production,
shared helpers, schema, registry, CI configuration and unknown paths require
the full backend suite. Selected registry jobs include frontend contract checks.
The selected scope appears in the Test Scope job summary. A targeted PR result
does not stand for a full release validation. Test-only `authorization_kernel`
changes include its compile-fail fixtures; full backend pushes run it too.

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
