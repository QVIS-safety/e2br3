# Generated Catalog Save Constraints

## Purpose

Make the backend validation catalog the single source of truth for constraints
that define whether a Case Editor value can be persisted. The frontend receives
a deterministic generated snapshot of those constraints at build time and
evaluates the portable subset with Zod. It does not call a catalog API while the
editor is running and does not maintain handwritten copies of catalog rules.

Representation constraints and business validation remain separate:

- Representation constraints reject values that cannot be stored as valid E2B
  data. Portable constraints run in the frontend before save, and every
  constraint runs in the backend before mutation.
- Business validation describes completeness or submission readiness. It may
  produce validation issues but does not prevent draft saves.

## Goals

- Generate frontend constraint metadata from the backend catalog.
- Statically import the generated snapshot and build a generic Zod evaluator.
- Reject invalid representations at the backend mutation boundary.
- Keep required, conditional, companion, future-date, and submission rules out
  of the draft-save gate.
- Prevent handwritten frontend rule inventories and path maps from drifting.
- Support all ICH, FDA, and MFDS metadata in one generated artifact.

## Non-Goals

- Fetching catalog metadata at runtime.
- Reimplementing backend conditional `RuleFacts` evaluation in TypeScript.
- Shipping large or changing terminology databases in the frontend bundle.
- Replacing the existing backend case-validation report.
- Changing exporter or submission-readiness behavior.

## Source Ownership

The source chain is:

```text
official source -> dictionary -> canonical catalog -> editor binding registry
                                             |                |
                                             +------ exporter-+
                                                        |
                                      generated frontend JSON
```

The generated JSON is an artifact, not a source. Its header states that it must
not be edited manually. The frontend contains no handwritten authority-specific
constraint list and no separate rule-code-to-field-path map.

The canonical catalog owns rule semantics. A single backend
`EditorFieldBinding` registry owns editor placement:

```rust
struct EditorFieldBinding {
    rule_code: &'static str,
    editor_page: EditorPage,
    value_path_template: &'static str,
    null_flavor_path_template: Option<&'static str>,
}
```

Both the snapshot exporter and backend save evaluator use this registry. Paths
must not also be added as independent strings to `CanonicalRule` or TypeScript.

`EditorPage` matches actual Case Editor pages, including `CI`, `RP`, `SD`, `LR`,
`SI`, `DM`, `DH`, `AE`, `LB`, `DG`, and `NR`. It does not reuse the current
coarse catalog `section` grouping.

Path templates use the canonical editor payload shape. Repeating rows use `[]`,
including nested repetitions such as
`drugs[].dosageInformation[].dose`. Concrete indexed paths supplied by a
mutation are preserved; no canonical-owner fallback rewrites them.

## Constraint Classification

Every exported constraint has an explicit enforcement classification:

- `client_and_server`: portable representation constraint evaluated by Zod and
  by the backend.
- `server_only`: representation constraint requiring server-side state.
- `validation_only`: business rule that never prevents a draft save.

Initial `client_and_server` kinds are:

- `max_length`
- `primitive_type`
- `numeric_shape`
- `allowed_values` for small closed code sets
- portable `format`
- `null_flavor`

Active terminology membership such as MedDRA, EDQM, MFDS products, and WHODrug
is `server_only`. Autocomplete may constrain normal user input, but the backend
is the final membership gate. Required, conditional-mandatory, companion,
future-date, forbidden-by-business-condition, and submission/export rules are
`validation_only`.

Severity does not determine enforcement. Missing optional values do not violate
a representation constraint unless a rule explicitly says that a present
representation is invalid.

## Generated Artifact

Add a workspace tool crate at
`crates/tools/validation-catalog-exporter`. It depends on `validator` and
provides the deterministic exporter command:

```text
cargo run -p validation-catalog-exporter -- \
  --output <frontend-repo>/lib/validation/generated/editor-save-constraints.json
```

Output is sorted by authority, editor page, rule code, and path so repeated
generation is byte-for-byte identical. The command also supports `--check
<path>`, which exits nonzero when regenerating would change the target.

The artifact has one schema version and one content-derived catalog version:

```json
{
  "schemaVersion": 1,
  "catalogVersion": "sha256:...",
  "constraints": [
    {
      "code": "ICH.G.k.2.2.LENGTH.MAX",
      "authority": "ich",
      "editorPage": "DG",
      "valuePathTemplate": "drugs[].medicinalProduct",
      "nullFlavorPathTemplate": null,
      "enforcement": "client_and_server",
      "constraint": {
        "kind": "max_length",
        "maxLength": 250
      },
      "message": "Must be 250 characters or fewer."
    }
  ]
}
```

Constraint payloads use a tagged JSON union shared by Rust and TypeScript.
Formats use stable identifiers such as `e2b_datetime`, `ich_identifier`, and
`base64`, not arbitrary Rust or JavaScript regular expressions. Numeric rules
define wire-value semantics explicitly, including whether the editor supplies a
string or JSON number, before typed Rust deserialization occurs.

The backend repository keeps an exporter golden test. The frontend commits:

- `lib/validation/generated/editor-save-constraints.json`;
- `lib/validation/generated/catalog-source.json`, containing the backend Git
  commit and expected `catalogVersion`.

Frontend scripts provide these commands:

```text
npm run validation:catalog:sync
npm run validation:catalog:check
```

They use `BACKEND_REPO`, defaulting to `../../e2br3` from the frontend repository
root. `sync` refuses a dirty backend checkout, invokes the Rust exporter, and
records `git -C "$BACKEND_REPO" rev-parse HEAD`. Frontend CI reads that SHA,
checks out `https://github.com/QVIS-safety/e2br3.git` at the exact commit into a
temporary directory, regenerates the artifact, and compares it byte-for-byte
with the committed file. It also validates the JSON with the TypeScript schema
and compares both catalog version values. This makes the frontend copy
reproducible generated output; editing either generated file manually fails CI.

Because the repositories deploy separately, a coordinated change is released
in this order: backend gate first, matching frontend artifact second. An older
frontend may miss immediate preflight for a newly added rule, but the backend
gate still rejects the invalid mutation with a structured issue. This is an
acceptable temporary UX difference and cannot permit invalid persistence.

## Frontend Evaluation

The frontend statically imports the generated artifact. There is no loading
state, request cache, ETag, polling, or catalog availability failure.

For an editor save, it selects `client_and_server` constraints by:

1. affected editor page or pages;
2. ICH plus the case's active regional profiles;
3. the concrete values present in the outgoing payload.

Authority filtering in the frontend is only an early UX optimization. It is not
an integrity boundary.

A generic `z.any().superRefine(...)` evaluator expands path templates against
the outgoing payload and adds Zod issues at concrete React Hook Form paths. For
example, `drugs[].dosageInformation[].dose` may produce
`drugs.2.dosageInformation.1.dose`.

The evaluator validates every page affected by the save orchestration, not only
the currently selected tab. This covers existing saves that normalize or write
cross-page message fields. Any emitted issue prevents the mutation requests.

Existing handwritten syntax rules are removed page by page only after generated
coverage and parity tests exist. `fieldVisibility.ts` is deleted only after no
remaining local syntax rule depends on it.

## Backend Save Gate

Browser validation is not a security or integrity boundary. The backend invokes
one shared representation gate from the common mutation service before any Case
write. The gate must not be copied into individual route handlers.

The gate:

1. resolves authoritative profiles from server-owned case/receiver policy;
2. determines every editor page affected by the prospective mutation;
3. selects `client_and_server` and `server_only` catalog constraints;
4. evaluates the incoming wire value or prospective model before persistence;
5. returns a structured HTTP 422 response on failure;
6. performs no write when any issue exists.

Request-provided authority values may help select UI behavior but cannot reduce
the server-owned authority set. A direct caller cannot send only `ich` to bypass
FDA or MFDS constraints.

If current route architecture cannot provide one common mutation service, the
intermediate implementation must include an inventory test proving that every
Case mutation route invokes the gate. Consolidation remains the preferred
boundary.

## Error Contract

The REST error layer adds an explicit unprocessable-entity variant mapped to
HTTP 422:

```json
{
  "error": "validation_failed",
  "catalogVersion": "sha256:...",
  "issues": [
    {
      "code": "ICH.G.k.2.2.LENGTH.MAX",
      "fieldPath": "drugs.2.medicinalProduct",
      "message": "Must be 250 characters or fewer."
    }
  ]
}
```

The frontend maps these issues into the same field banners as local Zod issues.
Business-validation failures continue through the existing validation report
and do not return this save-blocking response.

## Migration

Migration proceeds one editor page at a time:

1. Add complete `EditorFieldBinding` coverage for the page.
2. Add portable constraint payloads and Rust evaluator coverage.
3. Regenerate the frontend artifact.
4. Enable the generic Zod evaluator for that page.
5. Verify Rust/TypeScript parity with shared fixtures.
6. Remove the page's replaced handwritten frontend constraints.

A page does not use generated preflight until all intended portable constraints
for that page have bindings. Backend gate coverage is enabled independently and
remains authoritative throughout migration.

## Verification

Backend tests cover:

- exact binding coverage and duplicate rule/path rejection;
- actual editor-page mapping rather than coarse catalog sections;
- deterministic snapshot generation and catalog-version hashing;
- serialization and evaluation for every tagged constraint kind;
- authoritative profile resolution and attempted authority downgrade;
- scalar, repeated, and nested path expansion without fallback rewriting;
- direct mutation rejection with HTTP 422 and no database change;
- route inventory or common-service coverage for every Case mutation;
- business-validation issues not rejecting draft mutations.

Frontend tests cover:

- generated artifact schema and supported schema version;
- ICH plus regional-profile selection;
- every portable constraint kind and wire-value representation;
- scalar, repeated, and nested concrete issue paths;
- all pages affected by multi-page save orchestration;
- backend 422 issues mapped to the same field errors;
- required metadata excluded from the hard-save evaluator;
- removal of migrated local syntax and visibility rules.

Parity fixtures are exported from the backend with the artifact and run against
both Rust and TypeScript evaluators. They assert identical pass/fail results,
rule codes, and concrete field paths for `client_and_server` constraints.

## Success Criteria

- The frontend has no handwritten authority-specific inventory for migrated
  representation constraints.
- Opening and editing a page performs no catalog API request.
- Invalid values are rejected by frontend preflight where portable and by every
  direct backend mutation.
- Required and other business-validation failures remain draft-saveable.
- Rust and Zod evaluators agree for every portable parity fixture.
- A generated artifact can be reproduced byte-for-byte from the backend catalog
  and editor binding registry.
