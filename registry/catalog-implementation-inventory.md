# Catalog Implementation Inventory

## Case Validator Coverage

The validator's executable coverage registry is derived from the rule tables
passed to the shared evaluators in `case/sections/rule_table.rs`, plus an
explicit inventory of rules emitted by section-specific conditional branches.
It does not scan arbitrary source strings.

| Catalog scope | Catalog rules | Evaluator tables | Direct inventory | Overlap | Direct-only | Missing | Unexpected |
|---|---:|---:|---:|---:|---:|---:|---:|
| `CaseValidate`, sections C/D/E/F/G/H/N, ICH/FDA/MFDS | 461 | 411 | 51 | 1 | 50 | 0 | 0 |

The exact-set regression is
`case::sections::tests::implemented_case_registry_matches_case_validate_catalog`.
Run it with:

```bash
cargo test -p validator implemented_case_registry_matches_case_validate_catalog --lib
```

The 411 table-backed rules cover required/presence, companion, allowed-value,
vocabulary, MedDRA, maximum-length, and future-date evaluators. The direct
inventory contains 64 rules, including one rule already registered through a
table, leaving 50 direct-only rules. These counts are enforced by
`case_rule_inventory_baseline_is_exact`; a catalog rule added to this scope
fails the exact-set test until its case implementation is registered.

## Release-Backed Terminology

| Vocabulary | Source owner | Storage / release | Validator path | Operational input status |
|---|---|---|---|---|
| ISO 3166 | Approved ISO release import | `controlled_terminology_terms` / `iso3166` | `VocabularyContext` active membership; no fallback | Release import required |
| ICH constrained UCUM | Approved ICH constrained lists | `controlled_terminology_terms` / `ich_constrained_ucum` | Scoped exact membership; general UCUM remains parser-based | Release import required |
| EDQM | Approved EDQM export | `controlled_terminology_terms` / `edqm` | Scoped exact membership and active release version | Authenticated export required |
| MFDS domestic products | MFDS public Drug Product Permission API (`ITEM_SEQ`) | `mfds_products` / `mfds_product` | KR receiver selects `MFDS_PRODUCT/item_seq` | Collector, staged loader, release activation, and active-only search endpoint implemented |
| WHODrug foreign products | Licensed WHODrug release | `whodrug_products` / `whodrug` | FR receiver selects `WHODrug/all` | Licensed release import required |

MFDS product collection uses `registry/tools/import_mfds_products.py`. Raw pages and the normalized artifact are written below ignored `tmp/mfds-products/`. The service key is read only from `DATA_GO_KR_SERVICE_KEY` and is not persisted. Loading creates a `validated`, inactive release; approval and activation use the existing terminology release endpoints. Runtime search reads only the active release through `GET /api/terminology/mfds-products`.
