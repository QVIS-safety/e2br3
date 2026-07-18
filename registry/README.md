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
