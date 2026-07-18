# E2BR3 Field Registry Specification

## Purpose

The registry is the canonical source of truth for E2BR3 field coverage across the
local system. It exists to prevent loose markdown drift and to answer, for every
tracked field, whether the implementation has a consistent mapping between:

- E2BR3 field code
- authority scope
- backend BMC field
- frontend field

There are no committed generated reports. The registry itself is the product:
section JSON files, schema, and validator.

## Canonical Files

Editable source files:

- `registry/index.json`
- `registry/sections/*.json`
- `registry/schema.json`

Tools:

- `registry/tools/validate.py`
- `registry/tools/extract_frontend_fields.py`

There must not be a committed `registry/generated/` directory, generated matrix
JSON, generated inventory JSON, generated markdown report, or second canonical
mapping file. Source inventories are derived by the validator at runtime and are
discarded after validation.

## Section Files

Each section file contains a JSON array of registry rows. Section files are split
by E2BR3 section:

| File | Section |
|---|---|
| `registry/sections/n-message-header.json` | `N` |
| `registry/sections/c-safety-report.json` | `C` |
| `registry/sections/d-patient.json` | `D` |
| `registry/sections/e-reaction.json` | `E` |
| `registry/sections/f-test.json` | `F` |
| `registry/sections/g-drug.json` | `G` |
| `registry/sections/h-narrative.json` | `H` |

`registry/index.json` is the loader contract. A section file is not part of the
registry unless it is listed in `index.json`.

## Row Identity

Every row must have a stable `id` and `e2br3_code`.

For normal E2BR3 fields, use the E2BR3 code as both values:

```json
{
  "id": "C.3.2",
  "e2br3_code": "C.3.2"
}
```

For FDA regional fields, use the FDA-prefixed code:

```json
{
  "id": "FDA.C.1.7.1",
  "e2br3_code": "FDA.C.1.7.1"
}
```

For MFDS regional fields, use the KR regional code:

```json
{
  "id": "G.k.2.1.KR.1a",
  "e2br3_code": "G.k.2.1.KR.1a"
}
```

For structured E2B values where one published field carries multiple
implementation components, keep one registry row per backend/frontend field and
suffix the component after `@`:

```json
{
  "id": "C.4.r.2@mediaType",
  "e2br3_code": "C.4.r.2@mediaType"
}
```

Use this only when the implementation has separate persisted fields for the
components of one E2B value, such as ED attachment payload metadata. The portion
before `@` remains the owning E2B field code, and the portion after `@` must name
the implementation component being mapped.

Duplicate `id` values are invalid. Duplicate `e2br3_code` values are invalid.

## Authority

`authority` is exactly one of:

- `ICH`
- `FDA`
- `MFDS`

Combined authorities are invalid. Do not use `ICH+FDA`, `ICH+MFDS`,
`ICH+FDA+MFDS`, `regional`, or `local`.

Authority means the field's owning conformance source:

| Authority | Meaning |
|---|---|
| `ICH` | ICH core E2B(R3) field. |
| `FDA` | FDA regional field or FDA-only field. |
| `MFDS` | MFDS regional field or MFDS-only field. |

If a field is FDA-specific, set `authority` to `FDA`. If a field is MFDS-specific,
set `authority` to `MFDS`. Do not mark a regional field as both regional and ICH.

Authority-specific code constraints:

- `FDA` rows must use an `e2br3_code` beginning with `FDA.`.
- `MFDS` rows must use an `e2br3_code` containing `.KR.`.
- `ICH` rows must not use FDA-prefixed or KR regional field codes.

## 1-1-1 Mapping Contract

The registry uses a strict 1-1-1 mapping contract:

```text
one E2BR3 code
  -> one backend BMC field
  -> one frontend field
```

For a row with `status: "complete"`:

- `backend.status` must be `mapped`.
- `frontend.status` must be `mapped`.
- The mapped backend side must identify exactly one model and one field.
- The mapped frontend side must identify exactly one section and one field.
- Each mapped side must include evidence.

The 1-1-1 contract does not mean the runtime data model cannot contain repeated
child rows. It means the registry row has one canonical implementation home for
that E2BR3 field. Repeating structures should map to a single canonical child
model and field path, not multiple competing homes.

The validator enforces the mapping as a set comparison:

```text
backend fields extracted from Rust source
  must equal
backend fields declared by mapped registry rows

frontend fields extracted from frontend source
  must equal
frontend fields declared by mapped registry rows
```

The registry JSON is the only place where an extracted field is classified as an
E2BR3 code, authority, section, and status. Extracted source inventories do not
carry regulatory meaning by themselves.

During the registry migration, source-inventory enforcement may be run as an
explicit strict validation mode while the existing seeded rows are expanded. The
normal schema validation command must remain CI-safe until every configured
backend and frontend source field has a registry row.

## Validator Inputs

The validator has exactly two input classes:

1. Registry source: `registry/index.json`, `registry/schema.json`, and the
   section files listed by `registry/index.json`.
2. Implementation source: backend Rust model files and frontend form/source
   files.

The validator must not read a hand-maintained `REQUIRED_BACKEND_FIELDS` constant,
committed inventory JSON, spreadsheet export, or markdown report as an input.
Those files would create another source of truth and are invalid for this
registry.

## Backend Inventory

Backend inventory is extracted from Rust structs owned by case BMCs. A backend
registry key has this shape:

```text
ModelName.field_name
```

Example:

```text
PatientInformation.patient_age_group
```

The backend extractor must parse the Rust source, find configured case-domain
model structs, and collect their public data fields. It must ignore technical
fields that are not E2BR3 business fields, such as internal IDs, foreign keys,
audit timestamps, and ordering fields.

The initial backend scope is the case-domain BMC model layer under:

```text
crates/libs/lib-core/src/model/
```

Out-of-scope backend models include users, organizations, permissions, audit
logs, import/export history, terminology dictionaries, presave templates, and
admin settings unless a future registry change explicitly adds them.

When the backend extractor finds `ModelName.field_name`, exactly one registry
row must declare:

```json
{
  "backend": {
    "status": "mapped",
    "model": "ModelName",
    "field": "field_name"
  }
}
```

If the backend field exists in source but no registry row maps it, validation
must fail with a deterministic missing-mapping error. The fix is to add a row to
the correct existing `registry/sections/*.json` file. If the frontend side is not
known yet, the row must use `status: "frontend_missing"` with `backend.status:
"mapped"` and `frontend.status: "missing"`. This keeps the backend field tracked
without falsely claiming the 1-1-1 chain is complete.

If a registry row maps `ModelName.field_name` but the extractor cannot find that
field in Rust source, validation must fail with an unknown-backend-mapping error.
The fix is to correct the registry row or update the backend source. Do not
silence this by adding a generated inventory entry.

## Frontend Inventory

Frontend inventory is extracted from actual editable frontend input fields, not
from broad DTO types or markdown notes. The canonical extractor is:

```text
registry/tools/extract_frontend_fields.py
```

The extractor's job is simple:

```text
frontend source input declarations
  -> normalized frontend input-field inventory
```

A frontend registry key has this shape:

```text
section.field_path
```

Example:

```text
patientInformation.patientAge.value
```

The extractor must collect reliable form field declarations from source
constructs that create or register editable inputs:

- React Hook Form `Controller` `name` props
- `register(...)` calls
- `useFieldArray({ name: ... })` roots when paired with child input names
- explicit editable field path constants used by case section forms

The extractor must not scrape rendered labels, table headers, comments,
free-form markdown, broad frontend DTO interfaces, or API response shapes as
field inventory. Those sources can be evidence in a registry row, but they are
not proof that a user-editable input exists.

The extractor output is a derived runtime inventory only. It must be printed to
stdout or returned to the validator in memory. Do not commit extractor output as
JSON, markdown, CSV, or a generated registry file.

### Frontend Extractor Scope

Initial extractor scope is the case edit frontend section files:

```text
../frontend/E2BR3-frontend/components/case-form/sections/
```

The initial configured files are:

| Frontend file | Registry section |
|---|---|
| `SectionC*.tsx`, sender/literature/study safety-report section files | `C` |
| `SectionD.tsx`, `SectionDH.tsx` | `D` |
| `SectionE.tsx` | `E` |
| `SectionF.tsx` | `F` |
| `SectionG.tsx` | `G` |
| `SectionH.tsx` | `H` |

Message header `N` fields may be added after the case-identification frontend
source homes are configured.

Out-of-scope frontend files include presave forms, admin screens, tests,
artifacts, scripts, schemas, type-only files, and API client response mappers
unless a future registry change explicitly adds them to the frontend extractor
configuration.

Presave forms are handled by the separate `registry/presaves/` namespace and
presave inventory extractor. Presave rows retain this specification's row
shape and join to case rows by `e2br3_code`; duplicates remain forbidden within
either namespace. Reporter is currently the only strict presave scope. Sender,
Receiver, Product, Study, and Narrative are not yet claimed as covered.

Dedicated presave and case nullFlavor columns receive local companion rows.
NullFlavor encoded in-band in a field does not create a second mapping row.

### Frontend Field Normalization

The extractor must normalize dynamic repeatable indexes to a stable registry
field path.

Examples:

```text
reactions.${activeIndex}.reactionCountry
reactions.${index}.reactionCountry
reactions.0.reactionCountry
```

all normalize to:

```text
reactions.reactionCountry
```

Nested repeatables follow the same rule:

```text
patientInformation.medicalHistoryEpisodes.${index}.comments
```

normalizes to:

```text
patientInformation.medicalHistoryEpisodes.comments
```

The normalized frontend registry key is the normalized input path itself. The
registry row splits that key into:

```json
{
  "frontend": {
    "section": "patientInformation",
    "field": "medicalHistoryEpisodes.comments"
  }
}
```

Normalization must remove only repeatable index tokens. It must not rename
business fields. For example, `reactionCountry` must not become `countryCode`;
that API/backend translation belongs in row evidence, not in the frontend
inventory key.

### Frontend Extractor CLI Contract

`registry/tools/extract_frontend_fields.py` must support:

```sh
python3 registry/tools/extract_frontend_fields.py
```

Default output is deterministic JSON to stdout:

```json
[
  {
    "key": "reactions.reactionCountry",
    "section": "reactions",
    "field": "reactionCountry",
    "file": "../frontend/E2BR3-frontend/components/case-form/sections/SectionE.tsx",
    "raw": "reactions.${activeIndex}.reactionCountry"
  }
]
```

Output rows must be sorted by `key`, then `file`, then `raw` so diffs and test
failures are stable.

The extractor must fail closed:

- if a configured frontend source file cannot be read
- if a configured source glob matches no files
- if a field declaration cannot be parsed safely after being matched

The extractor must not mutate registry files or frontend files.

When the frontend extractor finds `section.field_path`, exactly one registry row
must declare:

```json
{
  "frontend": {
    "status": "mapped",
    "section": "section",
    "field": "field_path"
  }
}
```

If the frontend field exists in source but no registry row maps it, validation
must fail with a deterministic missing-frontend-mapping error. The fix is to add
or update a row in the correct existing section JSON. If the backend side is not
known yet, the row must use `status: "backend_missing"` with `frontend.status:
"mapped"` and `backend.status: "missing"`.

If a registry row maps a frontend field that the extractor cannot find,
validation must fail with an unknown-frontend-mapping error.

### Frontend Validation Modes

Normal validation remains CI-safe until the frontend extractor has enough
coverage for all configured case section files:

```sh
python3 registry/tools/validate.py
```

Strict frontend validation is explicit:

```sh
python3 registry/tools/validate.py --strict-frontend-inventory
```

Presave structural and source/transfer validation are explicit:

```sh
python3 registry/tools/validate.py --strict-presave-registry
python3 registry/tools/validate.py --strict-presave-inventory
```

Strict frontend validation compares:

```text
normalized input fields extracted by registry/tools/extract_frontend_fields.py
  must equal
frontend mappings declared by mapped registry rows
```

Rows with `frontend.status: "missing"`, `frontend.status: "not_applicable"`, or
`frontend.status: "conflict"` do not satisfy extracted frontend inventory
coverage. A real extracted input field needs a row with `frontend.status:
"mapped"`.

Rows with `status: "frontend_missing"` are allowed to map backend inventory, but
strict frontend validation must not treat them as frontend coverage.

## Mapping Blocks

Each row has two mapping blocks:

- `backend`
- `frontend`

Each block has a `status`:

| Status | Meaning |
|---|---|
| `mapped` | The side has one verified implementation home. |
| `missing` | The side is expected but not implemented or not found. |
| `not_applicable` | The side is not applicable for this row by design. |
| `conflict` | Multiple competing homes exist or evidence disagrees. |

When a block is `mapped`, it must include evidence. Evidence should be concrete:
file path, type name, field name, route, mapper, test name, or source
reference. Avoid general prose such as "implemented" or "exists".

Mapped backend blocks require `model`, `field`, and `evidence`.
Mapped frontend blocks require `section`, `field`, and `evidence`.

## Row Status

Each row has a top-level `status`:

| Status | Meaning |
|---|---|
| `complete` | Backend and frontend mapping blocks are verified and mapped. |
| `backend_missing` | Backend BMC home is missing or unverified. |
| `frontend_missing` | Frontend field home is missing or unverified. |
| `intentionally_unmapped` | Field is deliberately not implemented, with evidence and action. |
| `not_applicable` | Field is not applicable to this product/profile, with evidence. |
| `conflict` | Evidence shows competing or contradictory mappings. |

No other statuses are allowed. Do not use `TBD`, `unknown`, `maybe`, `partial`,
blank, or free-form status values.

## Required Consistency Rules

The validator must reject:

- invalid JSON
- section files not listed in `index.json`
- rows missing required fields
- duplicate `id`
- duplicate `e2br3_code`
- invalid `section`
- invalid `authority`
- combined authority values
- invalid top-level `status`
- invalid mapping block `status`
- `complete` rows where backend or frontend is not `mapped`
- `mapped` blocks without evidence
- FDA rows whose code does not start with `FDA.`
- MFDS rows whose code does not contain `.KR.`
- ICH rows whose code starts with `FDA.` or contains `.KR.`
- backend BMC fields present in code but absent from the registry
- frontend fields present in code but absent from the registry
- backend mappings declared in the registry but absent from backend source
- frontend mappings declared in the registry but absent from frontend source
- multiple registry rows pointing to the same backend model-field
- multiple registry rows pointing to the same frontend section-field

Exceptions to duplicate backend or frontend mappings are not allowed in the base
schema. If a future repeated-structure or alias case needs an exception, the
schema must first add an explicit exception field with constrained values and
evidence. Free-form duplicate waivers are invalid.

When XML export/import and validation layers are ready for systematic audit, this
spec can add mapping blocks for those layers. Until then, registry rows must not
contain XML exporter/importer or validation rule blocks.

## Failure Output Contract

Validation output must be deterministic and actionable. For source-inventory
coverage failures, errors must identify the side and key:

```text
missing backend mapping: PatientInformation.patient_age_group
unknown backend mapping: PatientInformation.patient_sex
missing frontend mapping: patient.patientAgeGroup
unknown frontend mapping: patient.patientSex
```

The validator must fail closed. If a configured source file cannot be read or a
configured model cannot be parsed, validation must fail instead of silently
skipping that source.

## Conflict Handling

Use `status: "conflict"` when evidence shows that a field has more than one
possible home or when source references disagree.

Examples:

- two backend BMC fields appear to store the same E2BR3 code
- frontend form field name differs from API DTO field name and no mapper explains it
- FDA/MFDS regional ownership is unclear from the available source

A conflict row must include:

- concrete evidence for each competing interpretation
- an `action` describing what decision or implementation change is needed
- no claim that the row is complete

## Not Applicable And Intentionally Unmapped

Use `not_applicable` only when the field is outside the product/profile scope and
there is evidence for that decision.

Use `intentionally_unmapped` only when the field is in scope but deliberately not
implemented. The row must include an `action` explaining the accepted behavior or
the condition required before implementation.

Neither status should be used to hide missing implementation work.

## Verification Commands

Run these commands after editing registry source files:

```sh
python3 -m unittest registry/tools/test_validate.py
python3 registry/tools/validate.py
```
