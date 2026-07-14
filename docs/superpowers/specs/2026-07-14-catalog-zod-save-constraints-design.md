# Catalog-Driven Zod Save Constraints

## Purpose

Make the backend validation catalog the single source of truth for constraints
that define whether a value can be persisted. The frontend must still report
these errors immediately, but it must not maintain a second authority-specific
rule inventory.

The design separates representation constraints from business validation:

- Representation constraints reject values that cannot be stored as valid E2B
  data. They run in the frontend before a save and in the backend before every
  mutation.
- Business validation describes case completeness or submission readiness. It
  may produce blocking validation issues, but it does not prevent draft saves.

## Goals

- Source `max_length`, type/shape, allowed-value, format, vocabulary, and
  nullFlavor constraints from the backend catalog.
- Load catalog metadata only when a Case Editor section is first opened.
- Build a generic frontend Zod evaluator from the returned constraints.
- Reject invalid values at every backend mutation boundary before a database
  write.
- Remove frontend hard-coded authority filtering and duplicated hard
  constraints as sections migrate.
- Preserve draft saves when only required, conditional, companion, future-date,
  or other business-validation rules fail.

## Non-Goals

- Reimplementing backend conditional `RuleFacts` evaluation in TypeScript.
- Running required or conditional-mandatory rules as frontend save gates.
- Polling or periodically refreshing catalog metadata while an editor is open.
- Replacing the existing backend case-validation report.
- Changing exporter or submission-readiness behavior.

## Constraint Classification

Every catalog rule exposed to the editor has an explicit enforcement value:

- `save_blocking`: executable representation constraint; prevents persistence.
- `validation_only`: business validation metadata; never prevents a draft save.

The first `save_blocking` constraint kinds are:

- `max_length`
- `primitive_type`
- `numeric_shape`
- `allowed_values`
- `format`
- `vocabulary`
- `null_flavor`

Required, conditional-mandatory, companion, forbidden-by-business-condition,
future-date, and submission/export rules remain `validation_only`. A rule's
existing validation severity does not implicitly make it save-blocking.

## Catalog Model

The canonical catalog owns all executable metadata. Frontend paths must not be
maintained in a separate TypeScript map.

Each editor-consumable rule provides:

```json
{
  "code": "ICH.G.k.2.2.LENGTH.MAX",
  "authority": "ich",
  "section": "G",
  "fieldPathTemplate": "drugs[].medicinalProduct",
  "enforcement": "save_blocking",
  "constraint": {
    "kind": "max_length",
    "maxLength": 250
  },
  "message": "Must be 250 characters or fewer."
}
```

`fieldPathTemplate` uses the canonical Case Editor model path. Repeating rows
use `[]`, including nested repetitions such as
`drugs[].dosageInformation[].dose`. The catalog version hash includes the path,
enforcement, constraint kind, and constraint payload so behavior changes always
change the version.

Constraint payloads use a tagged JSON union. A payload includes only data that
is portable between Rust and TypeScript. Format constraints use stable format
identifiers such as `e2b_datetime`, `ich_identifier`, or `base64`; arbitrary
Rust or JavaScript regular expressions are not part of the API contract.

## Section Catalog API

Extend the existing validation-rules endpoint with section and authority-profile
filtering:

```http
GET /api/validation/rules?authorities=fda,mfds&section=G
```

Rules returned for a profile set include ICH rules plus rules belonging to the
requested regional authorities. Duplicate ICH rules are removed by canonical
rule identity. A single-authority request remains supported for compatibility.

The response retains the existing catalog version header and returns both
`save_blocking` and `validation_only` rules for the requested section. The
frontend can therefore use one response for hard constraints and existing UI
metadata such as required markers.

## Frontend Loading

The frontend cache key is the sorted authority-profile set plus the canonical
section code, for example `fda,mfds:G`.

When a section first opens:

1. Start one request for its cache key.
2. Disable saving that section while the request is pending.
3. Store the normalized rules in memory after success.
4. Reuse the same rules on later visits without another request.
5. Deduplicate simultaneous requests with one shared promise.

There is no timer, focus refresh, per-keystroke request, or per-save catalog
request. A full browser reload creates a new memory cache and loads each opened
section once. If the request fails, the section remains unsaveable and exposes a
retry action; the frontend must not silently run fallback constraints.

## Frontend Zod Evaluator

The frontend creates a generic `z.any().superRefine(...)` evaluator from the
section's `save_blocking` rules. It does not generate a bespoke schema per rule
code.

For each rule the evaluator:

1. Expands `fieldPathTemplate` against the current section data.
2. Evaluates the tagged constraint payload.
3. Adds a Zod issue with the concrete React Hook Form path and catalog message.

For example, `drugs[].dosageInformation[].dose` can produce
`drugs.2.dosageInformation.1.dose`. Missing optional values do not violate a
representation constraint unless the constraint explicitly describes an
invalid present representation. Requiredness is not inferred.

Zod issues continue to drive immediate field banners and section markers. Any
Zod issue emitted from a `save_blocking` catalog rule prevents the save request.

## Backend Save Gate

Browser validation is not a security or integrity boundary. Every backend Case
mutation endpoint must invoke a shared representation-constraint gate before
writing data.

The gate:

1. Resolves the mutation's authority profiles and affected section.
2. Selects that section's `save_blocking` catalog rules.
3. Evaluates the incoming value or prospective model using the same constraint
   payload semantics as the frontend.
4. Returns HTTP 422 with concrete validation issues when a constraint fails.
5. Performs no write for the rejected mutation.

The response includes the catalog rule code, concrete field path, message, and
catalog version. Existing backend case validation remains responsible for
`validation_only` rules and must not be added to this mutation gate.

Each mutation is protected independently. The existing frontend Zod preflight
prevents the normal multi-request save workflow from starting when the active
section contains invalid values; direct API callers are protected by the gate
on each mutation endpoint.

## Migration

Migrate one canonical section at a time:

1. Add complete path and executable constraint metadata to the backend catalog.
2. Expose and contract-test the section API response.
3. Enable the frontend section loader and generic Zod evaluator.
4. Prove frontend/backend behavior parity with shared fixtures.
5. Remove migrated hard constraints from `lib/zod/sections`, `syntax.ts`, and
   `fieldVisibility.ts`.

The migration must fail closed. A section cannot use the new save path until
its catalog metadata is complete. `fieldVisibility.ts` is deleted only after no
remaining frontend syntax rule depends on it.

## Error Handling

- Catalog loading: save disabled until success.
- Catalog normalization failure: treat as load failure; do not discard malformed
  rules and continue.
- Frontend constraint failure: show the catalog message at the concrete field
  path and do not send a save request.
- Backend constraint failure: preserve the backend concrete issue, show it in
  the same field-banner path, and do not claim the case was saved.
- Business-validation failure: update semantic validation state but allow the
  draft mutation.

## Verification

Backend tests must cover:

- section and multi-authority filtering, including inherited ICH rules;
- DTO serialization for every constraint kind;
- catalog-version changes when executable metadata changes;
- path-template availability for every exposed `save_blocking` rule;
- direct mutation rejection with HTTP 422 and no database change;
- required and other `validation_only` issues not rejecting draft mutations.

Frontend tests must cover:

- one request per authority-profile/section cache key;
- request deduplication and retry after failure;
- save disabled during catalog load or after load failure;
- scalar, repeated, and nested repeated path expansion;
- each tagged constraint kind;
- concrete Zod issue paths and catalog messages;
- required metadata not entering the hard-save evaluator;
- removal of migrated local syntax and visibility rules.

Parity tests use the same serialized constraint fixtures against the Rust and
TypeScript evaluators and assert identical pass/fail results, rule codes, and
concrete field paths.

## Success Criteria

- The frontend has no manually maintained authority list for migrated hard
  constraints.
- Opening a section performs at most one catalog request for its profile set in
  the current page lifetime.
- Invalid representation values are rejected before any frontend save request
  and by direct backend mutation calls.
- Required and business-validation failures remain draft-saveable.
- Rust and Zod evaluators agree for every exposed constraint fixture.
