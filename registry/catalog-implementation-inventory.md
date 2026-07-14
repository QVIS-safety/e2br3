# Catalog Implementation Inventory

## Release-Backed Terminology

| Vocabulary | Source owner | Storage / release | Validator path | Operational input status |
|---|---|---|---|---|
| ISO 3166 | Approved ISO release import | `controlled_terminology_terms` / `iso3166` | `VocabularyContext` active membership; no fallback | Release import required |
| ICH constrained UCUM | Approved ICH constrained lists | `controlled_terminology_terms` / `ich_constrained_ucum` | Scoped exact membership; general UCUM remains parser-based | Release import required |
| EDQM | Approved EDQM export | `controlled_terminology_terms` / `edqm` | Scoped exact membership and active release version | Authenticated export required |
| MFDS domestic products | MFDS public Drug Product Permission API (`ITEM_SEQ`) | `mfds_products` / `mfds_product` | KR receiver selects `MFDS_PRODUCT/item_seq` | Collector and staged loader implemented; live collection pending `DATA_GO_KR_SERVICE_KEY` |
| WHODrug foreign products | Licensed WHODrug release | `whodrug_products` / `whodrug` | FR receiver selects `WHODrug/all` | Licensed release import required |

MFDS product collection uses `registry/tools/import_mfds_products.py`. Raw pages and the normalized artifact are written below ignored `tmp/mfds-products/`. The service key is read only from `DATA_GO_KR_SERVICE_KEY` and is not persisted. Loading creates a `validated`, inactive release; approval and activation use the existing terminology release endpoints.
