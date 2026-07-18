# Remove Drug-Level Dosage Text Design

**Date:** 2026-07-18

## Goal

Remove the app-local `DrugInformation.dosage_text` field and make repeated
`DosageInformation.dosage_text` rows the only dosage-text source throughout
case editing, XML export, and CIOMS rendering.

## Regulatory Basis

ICH E2B(R3) models dosage information as repeatable `G.k.4.r` rows. Dosage
Text (`G.k.4.r.8`) belongs to one of those rows; there is no drug-level dosage
text element and no rule that an arbitrary first or last regimen supersedes
the others. Consequently, every populated dosage row must remain distinct in
E2B(R3) XML, while a single-field presentation such as CIOMS must preserve all
populated row texts in sequence order.

Primary references:

- ICH E2B(R3) Q&A v2.4, question 4.23
- FDA E2B(R3) ICSR Implementation Guide, G.k.4.r and G.k.4.r.8
- FDA E2B(R3) Backwards and Forwards Compatibility guidance, section 5.7.8

## Canonical Data Model

`DosageInformation.dosage_text` is the sole canonical dosage-text field.
`DrugInformation.dosage_text` is removed from:

- Rust read/create/update models and SQL bindings
- REST and OpenAPI DTOs
- bootstrap/init SQL and demo seed data
- case-editor detail and save mappings
- frontend types, defaults, previews, validation paths, and tests
- XML drug-level import/export handling
- CIOMS drug-level fallback
- registry row `G.k.local.supplemental.dosageText`

No database migration is added. Existing drug-level values are intentionally
discarded. Fresh databases are defined only by the updated bootstrap SQL.

## Data Flows

### Case editor

The existing repeatable dosage editor remains unchanged from the user's
perspective. Each `G.k.4.r` row loads and saves its own `dosageText`.
The hidden `drugDosageText` form property and the drug REST payload field are
removed.

### XML

Import reads `G.k.4.r.8` only from the matching repeated dosage node. It does
not populate a drug-level fallback. Export emits `G.k.4.r.8` only inside the
matching dosage fragment and emits no drug-level `<text>` dosage value.

### CIOMS

For the selected suspect drug, collect all non-empty
`DosageInformation.dosage_text` values ordered by `sequence_number`. Trim
each value, preserve duplicates because they may describe distinct regimens,
and join them with newline characters. If every row is blank, render an empty
dose string.

CIOMS route, therapy dates, and duration continue to use the existing selected
dosage-row behavior; changing those mappings is outside this scope.

## Compatibility and Deployment

The frontend must stop sending `drugDosageText` before or together with the
backend deployment. The backend no longer accepts or returns the drug-level
field after deployment. No compatibility alias or temporary fallback remains.

Because there is no migration, an existing database that still has the
`drug_information.dosage_text` column may retain an unused column; application
code will not read or write it. Reinitialized databases will not create it.

## Testing

Tests must prove:

1. CIOMS joins all populated dosage-row texts in sequence order and ignores
   blank rows.
2. CIOMS no longer reads `DrugInformation.dosage_text`.
3. XML export emits dosage text only within repeated dosage fragments.
4. XML import does not create a drug-level dosage value.
5. Backend models, bootstrap SQL, registry inventory, and frontend source no
   longer expose the removed field.
6. Existing per-dosage case-editor load/save tests remain green.

Implementation follows red-green TDD: each behavior-changing test must fail
for the expected old behavior before production code is changed.

## Out of Scope

- Migrating or copying existing drug-level values
- Changing dosage-row ordering semantics
- Reformatting structured dose, unit, frequency, route, or dates into CIOMS
- Consolidating `frequency_value` or other local-only fields
