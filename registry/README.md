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

### Editor input coverage

`editor-contracts/*.json` binds dictionary elements through each field's
`dictionaryInputs`. The existing `constraint` remains the HTTP/UI smoke test;
`dictionaryInputs.checks` records **all structured** `max_length`,
`allowed_value_constraint` (except descriptive prose), and `null_flavors`
requirements, including the exact dictionary value so source drift fails CI.

UUID-backed roundtrips use `roundTripReference` (`case-reaction` or
`product-presave`) with a null `roundTripValue`, not an invented portable UUID.
The isolated HTTP harness creates the referenced record through its API and uses
the returned UUID for save/readback, omission, clear/blank boundaries and audit
checks. Null is only the literal/clear serialization vector, not a successful
positive-reference value. These isolated contracts are not dev browser evidence.
`dictionaryInputs.guidance` mirrors dictionary `notes`, including reviewed
official versions, URLs and field-specific conditions; stale guidance also fails.
Requiredness, cross-field rules, and descriptive prose are not scalar input
requirements; their existing business-rule mappings remain in validator.

Each requirement explicitly names its owner:

- `input-contracts`: production Rust function, issue code, positive and negative
  cases. Length checks include ASCII and Unicode at N−1, N and N+1; code sets
  and nullFlavor checks include every allowed dictionary value.
- `validator`: existing catalog rule codes and the reason for business ownership.
  This reference is not a claim that its business tests ran in the input suite.
- `unresolved`: a documented coverage/ownership gap, **not** a passing test.

There are 264 dictionary bindings for 263 editor codes: FDA.D.1 is a regional
nullFlavor overlay on D.1. N.1.1 is the one explicit `dictionaryExclusions`
entry: the registry records no editable input. Together these account for all
265 dictionary elements; accounting for an element does not mean all its
requirements are enforced. After the 2026-11-19 official-source review there are
382 executable scalar checks, 37 validator-owned requirements and 13 unresolved
requirements. The original 32 gaps all have source-backed guidance: three gained
scalar checks, eight gained submission-marker bindings/tests, and parent age
now correctly binds its existing `a` / `10.a` business rule.

Age/gestation units are field-specific code sets, not arbitrary UCUM expressions.
G.k.4.r.9.2a is a terminology version, not a dose-form code. C.1.8.1 distinguishes
original creation from preservation on retransmission; it must not acquire a
blanket format regex. True-marker rules run at case validation, not draft save;
only true without a nullFlavor or NI without a boolean is submission-valid.
The 13 open requirements cover identity/provenance (2), regional identifiers (7),
and terminology version/codes (4). These are implementation gaps,
not unknown official requirements or passing checks.

Included Documents (`C.1.6.1.r.2`) now uses the same Base64 input integrity
check as Literature (`C.4.r.2`), including the editable raw payload input.
Missing/empty draft attachments remain allowed; attachment presence, MIME and
recipient-specific submission requirements remain separate business rules.

Boundary values are isolated test fixtures, not production Case data. A
`{"repeat":"X","count":100}` value expands to a boundary string in the test
runner. Each test asserts only its named issue code; a length-boundary string
may independently violate a code-set rule. Existing HTTP save/roundtrip tests
remain separate; these scalar tests do not prove endpoint wiring or persistence.

After updating the dictionary or these bindings, run:

```sh
python3 registry/tools/generate_editor_contract_web_tests.py --write
python3 -m unittest registry/tools/test_generate_editor_contract_web_tests.py
python3 registry/tools/generate_editor_contract_web_tests.py --check
cargo test -p input-contracts --locked
```

The existing registry CI runs the generator check; registry changes already
select the input-contracts crate tests. Missing elements, properties, source
changes, missing boundaries and stale generated tests fail these gates.
Backend-only dictionary fixtures and guidance are stripped by the frontend
snapshot sync. The three fixed-unit input checks are also implemented in the
frontend; the patient trimester option and XML output use canonical `{trimester}`.
No existing case data is rewritten. Invalid legacy units or false submission
markers can now require correction before a save/submission respectively.

Duration and dose submission checks use `vocabularies/ich-time.json` (CL26 v2.2)
and `vocabularies/ich-dose.json` (CL25 v2.3), extracted from the official ICH
package linked in each snapshot. These are authoritative pinned lists, not a
fallback for a missing database. CL25 selects current `dosef=y` entries, including
the 2025 additions; duration excludes frequency annotations and gestation-only
units. Generic UCUM imports cannot widen these lists. Regenerate with
`import_vocabularies.py --ich-dose-source <CL25.xml> --ich-interval-source <CL26.xml>`.
Reaction duration's internal 800–805 codes retain the existing XML mapping.
FDA device manufacturer countries are checked against the active ISO country
dataset plus EU; a missing dataset is an explicit validation error.

Thirteen reviewed requirements remain unresolved: eleven regional identifier /
versioned terminology requirements need authoritative NDC, PhPID, UNII and
EDQM/FDA SPL sources and system/version mappings; C.1.7 needs verified R2 import
provenance, and C.1.8.1 needs original-sender / retransmission lifecycle enforcement.
These are not covered by the seven new unit/country rules, and remain visibly
unresolved in editor-contracts rather than being counted as validated.

### Empty string persistence policy

The application storage policy treats omitted fields as unchanged and explicit
`null` as a clear of an optional value. The editor excludes manually emptied
string inputs from saves; API row changes reject empty or whitespace-only strings
with `INPUT.EMPTY.REJECTED`. SQL checks cover every current model string column
and string-array element, including non-case tables. Required columns retain
`NOT NULL`; legacy optional blanks migrate to SQL `NULL`. Historical JSON/XML
payload contents are not rewritten. This policy does not change the ICH dictionary.

### Reviewed input baseline

`python3 registry/tools/check_input_contract_lock.py` checks the reviewed semantic
fingerprint in `.github/input-contract.sha256`. The Clippy runner calls it before
Cargo; CI also runs it in the unconditional Test Scope job. This is a contract
check alongside Clippy, not a Rust Clippy lint.

It pins dictionary input requirements, editor ownership (including unresolved
identities), error/constraint status, null/blank/omission behavior, and committed
vocabularies. Changing both dictionary and editor contract cannot silently weaken
the baseline. Test names, counts, boundary fixtures and Rust function names are
not fingerprinted; the existing consistency checks still apply to them.

An intentional standards correction requires official evidence, a reviewed
semantic diff, and owner approval before updating the fingerprint. There is no
automatic rebaseline command. Production Clippy prohibitions use `-F` so an
inline `#[allow(...)]` cannot disable them; test targets remain separately linted.

The fingerprint is not tamper-proof on its own. Repository-side required checks
and required code-owner approval must protect `.github/`, `clippy.toml`,
`Cargo.toml`, `scripts/ci_test_plan.py`, `scripts/ci_test_scope.py`, `registry/tools/`,
`registry/dictionary/`, `registry/editor-contracts/`, and `registry/vocabularies/`.
Protect Cargo configuration/toolchain files too, since they control lint execution.
Do not claim this protection is active until those settings and CODEOWNERS are
deployed. The agent's credentials must not be able to approve changes or bypass
or edit those settings; sharing a repository-admin credential with the reviewer
cannot enforce that separation.

Current owner: `@donihyun` in `.github/CODEOWNERS`. GitHub `main` and `dev`
require the GitHub Actions `Test Scope` and `Clippy` checks and code-owner
reviews, with stale approvals dismissed. Unowned files have no additional
approval-count requirement. CODEOWNERS review routing only takes effect after
this file is merged into each base branch. By explicit owner choice, admin
bypass remains enabled for the shared `donihyun` account: this is a review
guardrail, not a security boundary against the agent using that account.

### Source dictionaries

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
are required once. Backend requirements come directly from current registry
bindings; the full frontend registry job checks their exact agreement with the
extracted canonical inventory, globally and per target. Dev image
publication requires the selected test jobs to pass. These checks do not turn a blocked
generic contract or historical UI proof into a new UI passing result.

```sh
python3 registry/tools/check_contract_test_results.py \
  --registry-root registry --backend-log /tmp/workspace-tests.log
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
inventory checks when production code or contract inputs change, and always
requires passing results for every canonical assertion in the selected targets.
Test-only changes skip the frontend/registry job and read backend requirements
directly from current editor contracts and registry `automatedTests.backend`.
The full registry job compares those requirements with the freshly extracted
frontend inventory, both globally and per contract target, and retains its
frontend tests, ledger checks and inventory artifact. Historical hints are not
the backend requirement source. A whole-crate run requires every canonical assertion. The
execution checker receives the same target selection as the test runner; a
missing, ignored, failed, or duplicate required result still fails publication.
A targeted run is not a full workspace validation.

XML assets are prepared for the `xml` crate, whole web-server runs, and web
`api_import`, `api_submission`, or `xml` targets. Other web integration targets
skip the external sample/schema download. The CI plan and isolated database
runner share this decision; `scripts/ci_test_plan.py xml-assets -- <cargo args>`
prints it without running Cargo or creating a database. Whole workspace or
ambiguous package selections prepare assets conservatively. Clippy shares the
same asset flag, so XML-consuming integration targets still prepare their
runtime assets before linting. Preparation checks the eight consumed FDA files
and the root XSD before reuse and after extraction; this is a presence check,
not a checksum verification of the complete schema tree.

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
