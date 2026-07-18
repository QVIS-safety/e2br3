# Presave Registry Design

## Purpose

Extend the existing E2BR3 field registry so the repository can prove both of
these contracts:

1. A presave form field is persisted by the intended presave backend field.
2. A persisted presave field is transferred to the intended case backend field.

The design must cover sender, receiver, product, reporter, study, and narrative
presaves. Reporter is the first implementation slice because it exercises
renamed fields, grouped fields, and dedicated nullFlavor columns.

## Current State

The canonical case registry under `registry/sections/` maps one E2BR3 or
app-local code to one case backend field and one case frontend field. Its schema
and validator intentionally exclude presave backend models and presave forms.

Presave models and forms exist independently. Some presave-to-case transfers are
implemented in frontend conversion functions, such as
`reporterPresaveToPrimarySource`, while other presave relationships participate
in backend import and case-link workflows. There is no canonical registry that
joins these paths today.

Dedicated case nullFlavor inputs are now mapped to their real backend columns.
Those rows use existing app-local codes such as
`C.2.r.local.reporterCountryNullFlavor`. Presave rows will reuse these codes.

The normal and strict backend registry validations currently pass. Strict
frontend inventory validation currently fails because its configured
`SectionC*.tsx` source glob no longer matches the reorganized frontend. That
baseline must be repaired before presave strict validation is enabled.

## Chosen Architecture

Presave mappings live in a separate namespace while reusing the existing row
schema exactly:

```text
registry/
├── schema.json
├── sections/                     # case mappings
└── presaves/
    ├── index.json
    └── sections/
        ├── c-reporter.json
        ├── c-sender.json
        ├── c-receiver.json
        ├── g-product.json
        ├── c-study.json
        └── h-narrative.json
```

Each presave row keeps the existing fields, statuses, and mapping objects:

- `e2br3_code` is the join key to the case registry.
- `backend` identifies one Rust presave model and field.
- `frontend` identifies one canonical presave frontend section and field.
- `local_only` retains its existing meaning for app fields that are not
  independent authority dictionary elements.

Codes must be unique within the presave registry. A code may occur once in the
case registry and once in the presave registry because those files represent
different mapping domains.

This layout was selected instead of adding `presave_backend` and
`presave_frontend` properties to every case row. It preserves the existing row
format, avoids empty presave mappings on unrelated case rows, and gives presave
inventory validation a clear boundary.

## Mapping Semantics

### Transferable fields

A transferable presave row has a matching case registry row with the same
`e2br3_code`. The two rows describe this chain:

```text
presave frontend field
  -> presave backend field
  -> case backend field
  -> case frontend field
```

The case backend target is obtained from the joined case row. The presave row
does not gain a third mapping property.

The validator must not accept prose evidence as proof that the middle transfer
exists. Production conversion functions must express source-to-target
assignments in the statically extractable object-construction and field-copy
patterns supported by the transfer extractor. Contract tests additionally
execute those same production functions to verify value semantics.

### Presave-only fields

Relationship identifiers, lifecycle controls, and UI-only values that are not
transferred into an E2BR3 case field remain explicit rows. They use
`local_only: true` and the existing status vocabulary. When no case transfer is
applicable, the row uses `not_applicable`; it is not required to join to a case
backend target.

This exception is limited to fields whose production behavior is genuinely
presave-only. It must not be used to suppress a missing transfer for a business
field.

### NullFlavor fields

Dedicated nullFlavor inputs remain independent app-local rows and reuse the
existing case registry codes. For reporter, this includes the existing address,
country, and other dedicated nullFlavor companions that have corresponding
presave fields.

The validator compares allowed nullFlavor sets across all applicable sources:

- authority dictionary constraints for the owning E2BR3 element;
- frontend presave schema or field constraint;
- backend presave validation;
- the presave-to-case transfer contract.

In-band nullFlavor handling, where a base field carries either a value or a
flavor token and the API separates it, remains represented by the base E2BR3
row. A separate local-only row is required only when the implementation exposes
a distinct frontend input or persistent presave field.

### Repeating child structures

Child collections such as product substances, study products, study reporters,
sender gateways, and responsible persons use one canonical row per business
field. The validator normalizes array indexes and verifies the parent/child
destination model. Relationship keys and ordering fields remain technical or
presave-only rows rather than masquerading as E2BR3 fields.

## Source Inventories

Presave inventories are derived at validation time. No generated inventory JSON
or second hand-maintained mapping matrix is committed.

### Frontend inventory

The frontend extractor reads production presave forms, canonical presave types,
and canonical write mappers. Tests may prove behavior but are not the sole field
inventory source. It normalizes camelCase form fields and nested/repeating paths
to stable registry keys.

### Backend inventory

The backend extractor reads configured Rust presave structs, including parent
and child entities. It ignores technical identity, organization, audit,
lifecycle, foreign-key, and ordering fields according to explicit model-scoped
rules. Business nullFlavor fields are not discarded merely because their names
end in `_null_flavor`.

### Transfer inventory

The transfer extractor reads production presave-to-case conversion paths and
records normalized pairs of:

```text
PresaveModel.source_field -> CaseModel.target_field
```

Direct assignments, deliberate renames, grouped-field handling, and repeating
child conversions must be represented. A target inferred through the case
registry join must match the production target exactly. If a production
conversion is not statically extractable, the conversion is refactored into a
supported explicit mapping pattern; the registry does not add a hand-maintained
transfer matrix as a workaround. Contract tests execute the production
conversion functions and complement, rather than replace, static inventory
validation.

## Validation Modes

Two explicit modes keep migration controlled:

```sh
python3 registry/tools/validate.py --strict-presave-registry
python3 registry/tools/validate.py --strict-presave-inventory
```

`--strict-presave-registry` validates the presave index, row schema, uniqueness,
statuses, mapping shape, and joins to case rows.

`--strict-presave-inventory` additionally compares production frontend and Rust
inventories and verifies presave-to-case transfers and nullFlavor constraints.
It includes the registry checks rather than requiring both flags.

The validator emits deterministic errors for:

- missing or unknown presave frontend mappings;
- missing or unknown presave backend mappings;
- missing case registry joins for transferable rows;
- missing presave-to-case assignments;
- assignments to the wrong case target;
- nullFlavor constraint mismatches;
- invalid repeating child destinations.

## Rollout

1. Repair the existing case frontend extractor source configuration and return
   its repository strict test to green.
2. Add the presave registry loader and schema/join validation.
3. Add reporter frontend, backend, transfer, and nullFlavor extraction tests.
4. Populate `c-reporter.json` and make reporter strict validation pass.
5. Reuse the same contract suite for sender, receiver, product, study, and
   narrative in that order.
6. Enable full presave strict validation in CI only after all configured
   presave types pass.

The intermediate reporter milestone may pass while later presave files are not
yet configured. The final strict configuration is complete only when all six
presave types are configured and covered.

## Testing Strategy

Unit tests cover:

- presave index loading and existing-schema validation;
- uniqueness within the presave namespace and allowed duplication across case
  and presave namespaces;
- Rust presave struct extraction and technical-field exclusions;
- frontend form/type/write-mapper extraction and path normalization;
- correct joins, missing joins, and wrong case targets;
- direct, renamed, grouped, and repeating-field transfers;
- nullFlavor allowed-set agreement and mismatch diagnostics.

Repository contract tests cover:

- the repaired case frontend inventory baseline;
- complete reporter frontend and backend inventory coverage;
- reporter-to-`PrimarySource` transfer coverage;
- reporter dedicated nullFlavor persistence and transfer;
- each additional presave type as it is configured;
- the final six-type strict presave inventory.

## Completion Criteria

The work is complete when:

- the existing case registry validations pass, including strict frontend and
  backend inventories;
- presave registry rows use the unchanged existing schema;
- all six presave types have complete configured inventories;
- every transferable row joins to the correct case backend field;
- every dedicated presave nullFlavor field has matching frontend, backend,
  constraint, and transfer coverage;
- presave-only exceptions are explicit and justified;
- both presave strict modes pass locally and in CI;
- no generated or hand-maintained secondary inventory is introduced.
