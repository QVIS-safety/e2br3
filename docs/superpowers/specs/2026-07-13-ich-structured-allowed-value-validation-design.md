# ICH Structured Allowed-Value Validation Design

## Goal

Implement every currently inactive, machine-checkable ICH
`allowed_value_constraint` through reusable validation infrastructure while
keeping the dictionary as the standards source of truth.

The target inventory is 103 ICH constraints:

| Kind | Count |
|---|---:|
| `numeric` | 40 |
| `vocabulary` | 26 |
| `format` | 23 |
| `boolean` | 7 |
| `true_marker` | 6 |
| `code_set` | 1 |
| **Total** | **103** |

The following are outside this design:

- The 90 ICH constraints classified as `descriptive`, because their source
  prose does not define a deterministic executable predicate.
- FDA and MFDS regional allowed-value and max-length constraints.
- Changes to required, max-length, future-date, or nullFlavor behavior except
  where an existing helper is reused.

## Source-Of-Truth Contract

The source chain is:

```text
official specification or terminology release
    -> registry source artifact
    -> normalized dictionary/vocabulary snapshot
    -> catalog constraint metadata
    -> executable validator coverage
```

No rule-specific value list may be introduced only in a section validator.
Finite values, format parameters, vocabulary scopes, source version, and
source hash must originate in committed registry data or an approved private
source import.

The existing raw `allowed_values` text remains unchanged for auditability.
Structured metadata is the executable representation of that text.

## Constraint Metadata

`registry/dictionary.schema.json` and the dictionary builder will extend
`allowed_value_constraint` with parameters required to execute each kind:

- `numeric_shape`: `decimal`, `integer`, or `dotted_version`.
- `format_name`: `e2b_datetime`, `base64`, or `ich_identifier`.
- `vocabulary_scope`: a named subset such as `all`, `time`, `gestation`,
  `dose`, or `frequency`.
- `identifier_profile`: `mpid`, `phpid`, or `substance_id`.
- `enforcement`: `case_validate` or `representation_enforced`.

The builder must reject incompatible combinations. For example, a
`code_set` requires `values`, a `numeric` constraint requires
`numeric_shape`, and a `format` constraint requires `format_name`.

MPID, PhPID, and SubstanceID are identifiers, not finite vocabularies. Their
current `vocabulary` classification will be replaced by an identifier profile
that validates the available ICH representation without pretending to verify
membership in an unavailable global product or substance registry.

## Official Vocabulary Snapshots

Normalized snapshots will live under `registry/vocabularies/`. Each snapshot
must contain:

- vocabulary name;
- upstream version or retrieval date;
- official source URL or source document identifier;
- SHA-256 of the imported source artifact;
- normalized active codes and any scope tags;
- license or redistribution notice required by the source.

Source handling differs by vocabulary:

- UCUM: import the official UCUM source/essence XML and preserve the UCUM
  license notice. The snapshot contains prefixes and base/derived unit symbols;
  it is not treated as a finite list of every valid UCUM expression. Runtime
  validation uses a UCUM grammar/parser so composed expressions such as
  `mg/kg` remain valid. Field-specific constrained sets come from the official
  ICH CL25/CL26 artifacts and may add ICH-defined annotations such as `{DF}`
  without altering the upstream UCUM snapshot.
- ISO 639: import the official Set 2 three-letter codes used by E2B(R3).
- EDQM: consume an authenticated, approved EDQM API export. CI and runtime
  validation must not depend on network access or EDQM credentials.
- MedDRA: continue using the existing licensed terminology context and DB
  release mechanism.

Importers must be deterministic. Re-running an importer against the same
source artifact must produce byte-identical normalized output.

## Runtime Architecture

The shared engine will introduce a typed constraint value without extending
the required-rule `RuleValue` abstraction:

```rust
pub(crate) enum ConstraintValue<'a> {
    Text(Option<Cow<'a, str>>),
    Boolean(Option<bool>),
    Decimal(Option<Decimal>),
    Date(Option<Date>),
}
```

The catalog owns semantic dispatch:

```rust
pub(crate) fn is_allowed_value_valid(
    rule_code: &str,
    value: ConstraintValue<'_>,
    vocabulary: &VocabularyContext,
) -> bool;
```

`rule_table.rs` owns collection traversal and issue emission. It will provide
four table/evaluator shapes:

- scalar;
- indexed;
- nested parent/child;
- grandchild.

Each section table supplies only:

- catalog rule code;
- concrete issue path builder;
- Rust model value extractor;
- parent/child ownership keys where nesting requires them.

No section implements code-set membership, numeric parsing, date parsing,
vocabulary lookup, or true-marker semantics itself.

## Vocabulary Context

`VocabularyContext` will retain the existing MedDRA data and add immutable
lookups for normalized ISO 639 and EDQM snapshots. UCUM uses a grammar parser
backed by the official prefix/unit snapshot; constrained ICH fields additionally
check a CL25/CL26 scope set. This keeps a valid general UCUM expression from
being rejected merely because it is composed, while still rejecting it where
an ICH field permits only a constrained subset.

Snapshots are loaded once per process or embedded at compile time. Case
validation must not perform network requests. DB-backed terminology remains
appropriate for licensed or administrator-managed releases such as MedDRA.

## Enforcement Boundary

Not every source constraint can produce an invalid persisted case value:

- Rust `bool` fields already exclude non-boolean representations.
- `Decimal` fields already exclude non-numeric persisted values.
- typed `Date` fields already exclude invalid persisted dates.

These rules will use `representation_enforced` only when the complete source
constraint is guaranteed by deserialization and storage typing. They require
boundary tests proving invalid input is rejected. They must not be marked
`CaseValidate` merely to make inventory counts pass.

String-backed numeric, format, identifier, and vocabulary values remain
`case_validate` and must emit normal `ValidationIssue` records with concrete
indexed paths.

## Delivery Sequence

1. Add a failing catalog coverage test that inventories all 103 constraints
   and rejects unclassified metadata-only entries.
2. Extend dictionary schema and builder metadata, then regenerate and prove
   exact dictionary/catalog parity.
3. Add deterministic UCUM, ISO 639, and EDQM importers and normalized snapshot
   validation. EDQM tests use a minimal licensed-safe fixture; production data
   comes from an approved export.
4. Generalize `VocabularyContext` without changing current MedDRA behavior.
5. Add the shared typed constraint evaluator and its scalar/indexed/nested/
   grandchild traversal wrappers.
6. Migrate code-set and true-marker rules.
7. Migrate boolean and numeric rules, classifying representation-enforced
   fields explicitly.
8. Migrate format rules, including E2B date/time text, base64, and ICH
   identifier formats.
9. Migrate UCUM, ISO 639, EDQM, MPID, PhPID, and SubstanceID rules.
10. Enable `CaseValidate` only for rules with executable case checks and close
    the 103-rule coverage inventory with zero unclassified gaps.

Each migration step uses test-first development and ends with a focused
commit. Section migrations may be split by C/D, E/F, and G/H/N to keep review
scope bounded.

## Error Behavior

- Empty optional fields do not fail an allowed-value rule; presence remains
  the responsibility of required rules.
- A nullFlavor accepted by the model does not run a value constraint unless
  the field also contains a value.
- Unknown vocabulary snapshots or scopes are configuration errors and must
  fail initialization or tests, not silently accept values.
- Invalid values emit the catalog rule code and message at the concrete case
  path. Indexed paths are never replaced by canonical owner paths.
- External vocabulary services being unavailable cannot affect validation,
  because runtime validation uses local snapshots only.

## Verification

The implementation is complete only when all of the following hold:

- The 103 target constraints are partitioned exactly into
  `case_validate` and `representation_enforced`, with no metadata-only gap.
- Every `case_validate` code appears in a production section rule table and
  has a failing-boundary regression test.
- Every `representation_enforced` code has an input-boundary test showing the
  invalid representation cannot be persisted.
- Dictionary constraint metadata matches generated catalog metadata exactly.
- Vocabulary snapshot source hashes and versions are validated.
- General UCUM validation accepts valid composed expressions and rejects
  unknown unit symbols; constrained fields match their official CL25/CL26 set.
- Scalar, indexed, nested, and grandchild paths preserve their actual indexes.
- Existing required, max-length, future-date, MedDRA, case validation, and XML
  validation tests remain green.
