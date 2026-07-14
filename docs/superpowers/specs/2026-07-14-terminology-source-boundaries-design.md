# Terminology Source Boundaries Design

## Goal

Make terminology ownership explicit and remove validation drift between registry
snapshots, Rust libraries, hand-seeded database tables, and runtime terminology
releases. Add an account-gated MFDS product collector without making case
validation depend on external network availability.

## Ownership

The registry owns regulatory rule definitions and source contracts. It answers
which vocabulary and scope apply to each dictionary field, but it does not own
large or frequently changing operational term sets.

The terminology database owns imported term data, release status, activation,
rollback, search, and exact membership checks. The validator reads only active
releases and never calls an external terminology service during case validation.

Parser libraries own grammar validation where the valid value space is not a
finite list. General UCUM expressions remain parser-validated for this reason.

## Vocabulary Classification

| Vocabulary | Canonical runtime source | Validation |
|---|---|---|
| ISO 3166-1 alpha-2 | Active terminology release | Exact active-code membership |
| ICH `EU` country extension | ICH-scoped terminology entry | Exact active-code membership |
| General UCUM | `octofhir_ucum` parser | Grammar validation |
| ICH CL25/CL26 constrained UCUM | Active terminology release, scoped by field use | Exact active-code membership |
| MFDS product `ITEM_SEQ` | Active terminology release | Exact active-code membership for domestic reports |
| WHODrug | Active terminology release | Exact active-code membership for foreign reports |
| MedDRA | Active terminology release | Version and term membership |
| EDQM dose form and route | Active terminology release with domain scopes | Exact active-code membership |

`registry/vocabularies` is reduced to source manifests, deterministic importer
contracts, and small test fixtures. Full operational datasets are not committed
there. Existing files are migrated only when their consumers have moved to the
new source; there is no compatibility fallback between sources.

## MFDS Dictionary Split

The following fields currently combine two meanings under `WHODrug`:

- `D.8.r.1.KR.1b`
- `D.10.8.r.1.KR.1b`
- `G.k.2.1.KR.1b`

For domestic reports, these fields use vocabulary `MFDS_PRODUCT` with scope
`item_seq` and the MFDS OID. For foreign reports, they use `WHODrug` and its
corresponding OID. Dictionary and generated catalog metadata must preserve this
condition explicitly instead of assigning one unconditional vocabulary.

## MFDS Collection Pipeline

The collector reads `DATA_GO_KR_SERVICE_KEY` from the process environment. The
key is never accepted as a command-line argument, written to a file, logged, or
committed.

One command performs a complete collection:

```bash
DATA_GO_KR_SERVICE_KEY=... python registry/tools/import_mfds_products.py
```

The command calls the official `getDrugPrdtPrmsnInq07` operation, follows
pagination until `totalCount` is satisfied, verifies every successful response,
and writes a private temporary raw artifact under `tmp/mfds-products/`. It then
normalizes `ITEM_SEQ`, product names, manufacturer names, permit dates, and
cancellation status into a staged terminology release.

Collection is atomic at the release boundary. A network, schema, pagination, or
normalization failure marks the candidate release failed and leaves the active
release unchanged. Duplicate `ITEM_SEQ` records are merged deterministically;
conflicting identity data fails staging rather than choosing a value silently.

The collector can be tested without an account by supplying fixture responses
through its transport boundary. A live integration test runs only when
`DATA_GO_KR_SERVICE_KEY` is present and otherwise reports an explicit skip.

## Terminology Data Model

`terminology_releases.dictionary` is extended to support `iso3166`,
`ich_constrained_ucum`, `mfds_product`, and `edqm` in addition to `meddra` and
`whodrug`.

MFDS products use a dedicated table because product identity and search metadata
do not fit a generic code-only table. Each row stores:

- `item_seq`
- Korean and English product names when supplied
- Korean and English manufacturer names when supplied
- permit date
- cancellation date and status
- release version
- active status and audit identity

ISO country, constrained UCUM, and EDQM entries use scoped controlled-term
storage with release version, code, display text where available, scope, and
active status. Scope is part of membership so a code valid in one domain cannot
be accepted in another.

Only one release per dictionary and language/scope set is active. Approval,
activation, retirement, and rollback use the existing terminology release
workflow and permissions.

## Runtime Flow

Terminology search endpoints read active rows and support UI autocomplete. MFDS
product search accepts code and product-name queries.

Case validation gathers only vocabulary values present in the current case and
performs batched active-release membership queries while building
`ValidationContext`. It does not load complete product or terminology tables into
memory.

Missing active releases fail closed for rules that require exact membership.
There is no Rust crate, embedded snapshot, stale database table, or network
fallback. General UCUM remains the sole parser-based exception because it tests
grammar rather than membership.

## Migration

1. Extend release types and add terminology tables and indexes.
2. Add release import and membership-query interfaces with failing tests first.
3. Move ISO country validation from the `country_code` crate to active
   terminology membership, including the explicit ICH `EU` entry.
4. Keep general UCUM parser validation; replace hand-seeded `ucum_units` as a
   validation source with active constrained-UCUM releases.
5. Split the three MFDS domestic/foreign dictionary constraints and regenerate
   catalog metadata.
6. Add the MFDS API collector and fixture-driven tests.
7. Add terminology search and release-management API coverage.
8. Remove obsolete source paths only after parity tests prove every consumer has
   moved.

## Verification

Tests must prove:

- dictionary-to-catalog parity preserves conditional vocabulary selection;
- domestic MFDS codes use only active `MFDS_PRODUCT/item_seq` entries;
- foreign values use WHODrug and never fall back to MFDS products;
- country validation uses the active ISO release and treats `EU` only as an ICH
  extension;
- general UCUM expressions use the parser;
- constrained UCUM and EDQM values require matching active scopes;
- missing releases fail closed;
- failed imports do not alter the active release;
- activation and rollback change validator and search results consistently;
- API keys never appear in snapshots, logs, errors, or committed files.

## Out of Scope

- Creating or managing a public-data portal account.
- Committing a live MFDS product export without an issued service key.
- Replacing the UCUM grammar parser with a finite code table.
- Adding fallback behavior for unavailable terminology releases.
