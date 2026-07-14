# Case Rule Table Consolidation Design

## Goal

Move the remaining case-validator rule decisions out of section-specific issue
branches and into a small declarative evaluation surface without duplicating
catalog conditions in Rust tables.

The catalog remains the source of truth for whether a required or conditional
rule applies. Section modules remain responsible for loading data and deriving
the `RuleFacts` consumed by catalog conditions.

This is a structural refactor except for removal of existing unresolved-owner
path fallbacks. Existing issue codes, resolved paths, severities, conditions,
nullFlavor behavior, and authority-specific behavior must remain unchanged.

## Corrected Baseline

The case catalog contains 461 unique `CaseValidate` rules in sections
C/D/E/F/G/H/N for ICH, FDA, and MFDS.

The current implementation inventory is:

| Inventory | Unique rules |
|---|---:|
| Existing evaluator tables | 398 |
| Explicit direct inventory | 64 |
| Present in both inventories | 1 |
| Direct-only rules | 63 |
| Combined catalog coverage | 461 |

`ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN` is the overlap. It is already in
`G_DOSAGE_FUTURE_DATE_RULES` and must not be counted as direct-only.

The existing catalog-to-case exact-set test remains the coverage gate, but the
table and direct inventories must be exposed separately so that overlap and
direct-only counts are computed rather than documented manually.

## Design Principles

1. A rule table must not repeat a catalog condition as a `trigger` closure.
2. A section may derive values, concrete paths, and `RuleFacts`; the catalog
   decides whether a conditioned rule applies.
3. Index topology must not produce more scalar/indexed/nested rule types.
4. Database access and cross-record assembly stay in section orchestration.
5. No owner, canonical-path, index, terminology, or condition fallback is
   introduced.
6. Custom predicate evaluation is limited to constraints that are not value
   policy or catalog-condition decisions.
7. Each issue code used by `CatalogValueRule` must own its condition and value
   policy metadata in the catalog; tables must not compose metadata from alias
   codes.

## Shared Evaluation Surface

### CatalogValueRule

`CatalogValueRule<T>` is the primary shape for required and conditioned values.

```rust
pub(crate) struct CatalogValueRule<T> {
    pub code: &'static str,
    pub path: fn(&T) -> String,
    pub value: for<'a> fn(&'a T) -> RuleValue<'a>,
    pub facts: fn(&T) -> RuleFacts,
}
```

Its evaluator iterates a slice of prepared items and delegates each row to the
existing catalog engine. It does not inspect the rule code, evaluate a trigger,
or substitute missing facts.

The current three-code conditioned helper is compatibility debt. In particular,
`FDA.C.1.12.RECOMMENDED` currently borrows the value policy of
`FDA.C.1.12.REQUIRED`. Before migration, the recommended issue code receives its
own equivalent catalog value-policy binding. `CatalogValueRule` then needs only
one code and cannot encode alias fallback.

This one shape covers:

- scalar and container presence;
- indexed and nested values;
- collection-wide aggregate presence;
- companion requirements represented by catalog conditions;
- FDA and MFDS receiver/profile requirements; and
- values assembled from database-backed regional context.

A scalar is evaluated as a one-item prepared slice. Indexed and nested paths
are carried by prepared items instead of encoded in separate evaluator types.

### ViolationRule

`ViolationRule<T>` is the restricted escape hatch for algorithmic invalidity
that cannot be expressed as catalog-conditioned value presence.

```rust
pub(crate) struct ViolationRule<T> {
    pub code: &'static str,
    pub path: fn(&T) -> String,
    pub violated: fn(&T) -> bool,
}
```

Allowed uses are:

- ordering between two dates;
- mutual exclusion between identifiers;
- a required member within a set of flags; and
- an allowed nullFlavor or profile-specific code combination.

It must not be used for a required or conditional-required decision that the
catalog can evaluate from `RuleFacts` and a value.

## Prepared Views

Complex sections create small, section-local views before evaluation. There is
no global validation view and no new generic indexing framework.

Examples include:

- `FdaStudyRuleView`: report type, receiver profile, normalized study number,
  registration presence, and concrete paths;
- `FdaDrugRuleView`: drug index, device characteristics, malfunction facts,
  suspect status, required device values, and concrete paths; and
- `MfdsRelatednessRuleView`: receiver profile, method/result presence,
  normalized codes, parent indexes, and catalog facts.

The section may perform asynchronous reads while constructing a view. Once the
view exists, issue decisions are made only by the shared evaluators.

Prepared views must preserve real parent and child indexes. Failure to resolve
an owner or index must not silently map an item to parent or index zero.

## Rule Classification

The 63 direct-only rules currently fall into these implementation groups:

| Group | Rules | Target |
|---|---:|---|
| Existing catalog-conditioned calls | 24 | `CatalogValueRule` |
| Ordinary scalar required values | 3 | `CatalogValueRule` |
| Root or collection presence | 3 | `CatalogValueRule` |
| Collection aggregate presence | 5 | `CatalogValueRule` with prepared aggregate view |
| Companion requirements | 6 | `CatalogValueRule` with catalog facts |
| Date ordering | 3 | `ViolationRule` |
| Mutual exclusion | 2 | `ViolationRule` |
| Set/nullFlavor constraints | 2 | `ViolationRule` |
| FDA C special rules | 4 | Prepared view, then one of the two shared shapes |
| FDA G device rules | 11 | `FdaDrugRuleView`, then one of the two shared shapes |

The classification describes implementation shape, not new catalog categories.

## Data Flow

```text
ValidationContext / regional context / database rows
    -> section-local prepared views
    -> CatalogValueRule or ViolationRule table
    -> shared evaluator
    -> catalog condition and value policy
    -> ValidationIssue with the prepared concrete path
```

For `CatalogValueRule`, the shared evaluator must always call the catalog engine
with the table code, extracted value/nullFlavor, and extracted facts. It must not
short-circuit based on section-local condition logic.

For `ViolationRule`, `violated` is evaluated directly because the rule describes
an algorithmic relation rather than catalog-conditioned presence.

## Migration Sequence

1. Correct the inventory accounting and add assertions for 398 table rules,
   64 direct inventory rules, one overlap, and 63 direct-only rules.
2. Make every conditioned issue code self-contained in catalog condition and
   value-policy metadata, beginning with `FDA.C.1.12.RECOMMENDED`.
3. Add `CatalogValueRule` and its evaluator with scalar, indexed-view, and
   concrete-path tests.
4. Migrate the 24 calls already using
   `push_issue_if_conditioned_value_invalid`.
5. Migrate ordinary required, container, aggregate, and companion rules.
6. Add `ViolationRule` and migrate date ordering, exclusion, set, and
   nullFlavor constraints.
7. Build FDA C prepared views and migrate the four special rules.
8. Build FDA G prepared views and migrate the eleven device rules.
9. Remove the direct inventory after its direct-only difference reaches zero.

Each migration step changes one coherent rule group and keeps the exact catalog
coverage test green.

## Testing

Every migration group starts with characterization tests for its current issue
codes and concrete paths. Tests cover both the emitting and silent cases.

Required shared-evaluator tests include:

- catalog condition false: no issue even when the value is absent;
- catalog condition true with absent value: one issue;
- catalog condition true with present value or allowed nullFlavor: no issue;
- prepared indexed and nested paths retain their real indexes;
- unresolved ownership does not fall back to index zero; and
- `ViolationRule` emits exactly once at its prepared path.

Completion gates are:

- 461 catalog rules and 461 implemented case rules;
- zero missing and zero unexpected codes;
- zero direct-only codes;
- all existing validator golden tests unchanged and green; and
- no new fallback behavior.

The unresolved-owner tests intentionally change the old generic `"drugs"` or
index-zero fallback behavior: orphaned rows are not assigned a fabricated field
owner. This is the only planned behavioral correction in the refactor.

## Non-Goals

- Moving XML/import validation into the case layer.
- Replacing the catalog condition model with a new condition DSL.
- Moving database access into `rule_table.rs`.
- Converting all 398 existing table-backed rules to the new shape in this pass.
- Changing dictionary, catalog, or regulatory semantics while refactoring.
