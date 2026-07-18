# Reporter Element NullFlavor Hard-Cutover Design

## Goal

Replace the invalid reporter name-group and address-group nullFlavor fields with
independent nullFlavor companions for each E2B reporter element. The change is
a hard cutover: no legacy data migration, API alias, read fallback, or dual
write is retained.

## Removed contract

Delete these fields everywhere, including database columns, Rust models, REST
payloads, frontend types/forms/mappers, presave transfer logic, generated
validation bindings, tests, and registry rows:

- `reporterNameNullFlavor` / `reporter_name_null_flavor`
- `reporterAddressNullFlavor` / `reporter_address_null_flavor`
- `C.2.r.local.reporterNameNullFlavor`
- `C.2.r.local.reporterAddressNullFlavor`

Existing values in the two database columns are intentionally discarded.

## New element-level contract

| E2B element | Frontend field | Backend/database field |
|---|---|---|
| C.2.r.1.1 | `reporterTitleNullFlavor` | `reporter_title_null_flavor` |
| C.2.r.1.2 | `reporterGivenNameNullFlavor` | `reporter_given_name_null_flavor` |
| C.2.r.1.3 | `reporterMiddleNameNullFlavor` | `reporter_middle_name_null_flavor` |
| C.2.r.1.4 | `reporterFamilyNameNullFlavor` | `reporter_family_name_null_flavor` |
| C.2.r.2.1 | `reporterOrganizationNullFlavor` | `organization_null_flavor` |
| C.2.r.2.2 | `reporterDepartmentNullFlavor` | `department_null_flavor` |
| C.2.r.2.3 | `reporterStreetNullFlavor` | `street_null_flavor` |
| C.2.r.2.4 | `reporterCityNullFlavor` | `city_null_flavor` |
| C.2.r.2.5 | `reporterStateNullFlavor` | `state_null_flavor` |
| C.2.r.2.6 | `reporterPostcodeNullFlavor` | `postcode_null_flavor` |
| C.2.r.2.7 | `reporterTelephoneNullFlavor` | `telephone_null_flavor` |

`reporterTitleNullFlavor` (C.2.r.1.1) accepts `MSK`, `UNK`, `ASKU`, or `NASK`.
The other ten fields accept `MSK`, `ASKU`, or `NASK`. Each database column uses
`VARCHAR(4)` with the equivalent element-specific check constraint.

## Database and API

Add a migration that drops the two group columns and adds the eleven new
columns to both `primary_sources` and `reporter_presaves`. Update bootstrap SQL
to describe only the new schema. The migration does not copy old values.

The Rust `PrimarySource` and `ReporterPresave` read/create/update/insert models,
BMC column lists, REST direct/portable save parsing, OpenAPI DTOs, and presave
CRUD all expose the eleven snake_case fields. Removed camelCase and snake_case
names are rejected or ignored through the normal unknown/absent-field behavior;
they are never treated as aliases.

## Frontend behavior

Case Edit and Reporter Presave render the existing shared nullFlavor control
beside every corresponding value input. A control clears and disables only its
own value field; it never clears sibling reporter fields. Clearing the
nullFlavor re-enables that input.

Reporter Presave validation changes from group-level exceptions to element
rules. `reporterGivenName` is satisfied by
`reporterGivenNameNullFlavor`; `reporterOrganization` is satisfied by
`reporterOrganizationNullFlavor`. Other optional elements may independently
carry a value or nullFlavor.

Canonical read/write mappers and presave-to-case transfer copy every individual
field directly. No group aggregation or fan-out exists.

## XML import and export

Import the `nullFlavor` attribute independently from each C.2 reporter name,
organization/address, and telephone element. Export each element as either its
value or its own `nullFlavor` attribute. A nullFlavor must not be copied to
another element. Existing country and qualification nullFlavor behavior remains
unchanged.

## Registry

Keep the existing registry row schema. Remove the two group companion rows and
add eleven local persistence companion rows:

- `C.2.r.local.reporterTitleNullFlavor`
- `C.2.r.local.reporterGivenNameNullFlavor`
- `C.2.r.local.reporterMiddleNameNullFlavor`
- `C.2.r.local.reporterFamilyNameNullFlavor`
- `C.2.r.local.reporterOrganizationNullFlavor`
- `C.2.r.local.reporterDepartmentNullFlavor`
- `C.2.r.local.reporterStreetNullFlavor`
- `C.2.r.local.reporterCityNullFlavor`
- `C.2.r.local.reporterStateNullFlavor`
- `C.2.r.local.reporterPostcodeNullFlavor`
- `C.2.r.local.reporterTelephoneNullFlavor`

Each row notes the official E2B element whose `nullFlavor` attribute it
persists. The same eleven rows exist in the case and reporter-presave namespaces
so strict presave-to-case joins remain exact.

## Generated validation catalog

Update the catalog source and run
`scripts/validation/sync-catalog-constraints.mjs`; do not hand-maintain stale
group paths in `lib/zod/generated/catalogBindings.ts`. The generated bindings
must associate each value field only with its matching nullFlavor path.

Keep the shared case rule engine unchanged. Change the
`ICH.C.2.r.2.1.REQUIRED` value policy to `NonEmptyOrNullFlavor` and pass the
actual `organization`/`organization_null_flavor` pair to the evaluator. Do not
substitute a synthetic presence marker.

## Testing

Tests cover database model persistence, REST create/update/read, presave
canonical mapping, presave-to-case transfer, per-field UI clearing/disablement,
required-field exceptions, XML import/export isolation, absence of legacy field
names, and strict case/presave registry inventories. A repository-wide search
for the two removed names must return no production or test references.
