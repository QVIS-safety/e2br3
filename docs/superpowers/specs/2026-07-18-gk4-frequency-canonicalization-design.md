# G.k.4.r.2/G.k.4.r.3 Frequency Canonicalization Design

**Date:** 2026-07-18

## Goal

Make the ICH dosage interval pair canonical across the database, Rust models,
REST payloads, XML import/export, validation, registry, and sibling frontend:

- G.k.4.r.2 `Number of Units in the Interval` owns the numeric value.
- G.k.4.r.3 `Definition of the Time Interval Unit` owns the constrained unit.

Remove the invented `frequency_value` / `frequencyValue` field and replace the
G.k.4.r.3 select with a searchable autocomplete containing the exact approved
field-specific vocabulary.

## Regulatory and XML Model

G.k.4.r.2 and G.k.4.r.3 are separate E2B data elements represented together
in XML as attributes of one periodic interval:

```xml
<period value="3" unit="d"/>
```

The canonical application mapping is therefore:

| E2B element | Application field | Database column | XML attribute |
| --- | --- | --- | --- |
| G.k.4.r.2 | `numberOfUnits` | `number_of_units` | `period/@value` |
| G.k.4.r.3 | `frequencyUnit` | `frequency_unit` | `period/@unit` |

There is no third frequency-number concept. `frequency_value`,
`frequencyValue`, and the legacy `doseFrequencyValue` alias are removed rather
than synchronized or retained as compatibility fields.

## Canonical Data Model

Remove `frequency_value` from the bootstrap schema, demo seeds,
`DosageInformation`, its create/update inputs, XML import helper structs, REST
payload handling, test fixtures, and generated or maintained contract
artifacts that are source-controlled.

Remove `frequencyValue` and `doseFrequencyValue` from sibling-frontend types,
defaults, detail transforms, save transforms, Zod paths, previews, and tests.
Remove the unused legacy `doseFrequencyUnit` alias as well; `frequencyUnit` is
the only frontend unit property.

No database migration or data-copy compatibility path is added. The target
environment can be reinitialized, so the updated bootstrap schema defines the
database directly.

## XML Data Flow

On import, read `period/@value` once into `number_of_units` and read
`period/@unit` into `frequency_unit`. Do not duplicate the numeric value in an
intermediate or persisted frequency field.

On export, emit the periodic interval when either canonical component is
present. Serialize `number_of_units` as `period/@value` and `frequency_unit` as
`period/@unit`. A fully populated form row with `3` and `d` must produce:

```xml
<period value="3" unit="d"/>
```

Existing handling of dosage dates, duration, dose quantity, route, and dosage
text is unchanged.

## Validation

The G.k.4.r.3 companion requirement is triggered by `number_of_units`, not by
the removed `frequency_value` field. Both frontend Zod validation and the Rust
validator must report the missing G.k.4.r.3 unit when G.k.4.r.2 is populated.

Existing vocabulary validation remains authoritative. The UI list is an
input constraint and must not weaken backend validation.

## Registry

Delete the entire registry entry with ID
`G.k.local.dosage.frequencyValue`. Keep the official rows only:

- G.k.4.r.2 mapped to `number_of_units` / `numberOfUnits`
- G.k.4.r.3 mapped to `frequency_unit` / `frequencyUnit`

Update misleading labels or evidence that describe G.k.4.r.3 as a combined
value-and-unit field. Registry validation must pass without creating another
local alias or replacement inventory row.

## Searchable Autocomplete

Use the sibling frontend's existing `FormAutocomplete` component with a static,
field-specific option list. Do not depend on the general UCUM endpoint because
G.k.4.r.3 has a small normative vocabulary and includes three non-unit
expressions.

Expose exactly these nine values:

| Stored value | Display/search label |
| --- | --- |
| `a` | `a:year` |
| `mo` | `mo:month` |
| `wk` | `wk:week` |
| `d` | `d:day` |
| `h` | `h:hour` |
| `min` | `min:minute` |
| `{cyclical}` | `{cyclical}:cyclical` |
| `{asnecessary}` | `{asnecessary}:as necessary` |
| `{total}` | `{total}:total` |

Users can search by code or English label. Selecting an option stores only its
canonical value in `frequencyUnit`. Clearing the autocomplete stores an empty
value consistent with neighboring optional fields. The existing audit table,
audit column, record ID, E2B field number, and validation error presentation
remain attached to the field.

Seconds, decades, trimesters, arbitrary UCUM expressions, and free text are not
offered for G.k.4.r.3 because they are outside this field's approved list.

## Testing

Implementation follows red-green TDD. Tests must prove:

1. XML import maps `period value="10" unit="d"` only to
   `number_of_units = 10` and `frequency_unit = "d"`.
2. XML export maps `number_of_units = 3` and `frequency_unit = "d"` to one
   `<period value="3" unit="d"/>` fragment.
3. Rust validation requires G.k.4.r.3 when `number_of_units` is populated.
4. Frontend Zod validation requires `frequencyUnit` when `numberOfUnits` is
   populated.
5. The G.k.4.r.3 autocomplete exposes exactly the nine specified options,
   supports code/label search, and stores the canonical selected value.
6. The dosage save and detail adapters send and restore only
   `number_of_units` and `frequency_unit` for this pair.
7. Registry validation passes after removal of
   `G.k.local.dosage.frequencyValue`.
8. Active source, schema, model, adapter, registry, and test-fixture paths no
   longer contain the removed `frequency_value`, `frequencyValue`, or
   `doseFrequencyValue` fields.

The currently disconnected G-section import/export tests must be made runnable
or replaced by focused active tests so the regression is exercised by the
normal test commands.

## Deployment and Compatibility

Backend and sibling frontend changes are one coordinated contract change.
Reinitialize the database after deployment so `dosage_information` is created
without `frequency_value`. No compatibility alias, fallback read, fallback
write, or migration remains.

## Out of Scope

- Broad redesign of the general UCUM terminology service
- Adding reference-only values such as `s`, `10.a`, or `{trimester}`
- Changes to G.k.4.r.1 dose value/unit or G.k.4.r.6 duration value/unit
- Unrelated DG layout or registry refactoring
