# Terminology Source Boundaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move finite controlled vocabularies to approved terminology releases, preserve parser-based general UCUM validation, split MFDS domestic product codes from foreign WHODrug codes, and provide a service-key-based MFDS collector.

**Architecture:** Registry dictionary files declare vocabulary selection and source contracts. PostgreSQL terminology releases own operational finite code sets and activation state. Validation batches only values used by a case into `VocabularyContext`; external APIs are used only by import tooling.

**Tech Stack:** Rust, Tokio, SQLx/PostgreSQL, Axum, Python 3 standard library, JSON Schema, Cargo tests, Python unittest.

## Global Constraints

- No fallback between embedded snapshots, Rust code lists, stale DB rows, or external APIs.
- Missing finite terminology releases fail closed.
- General UCUM expressions remain parser-validated with `octofhir_ucum`.
- `DATA_GO_KR_SERVICE_KEY` is read only from the environment and is never logged or persisted.
- Existing untracked files under `tmp/` are not modified or committed.

---

### Task 1: Terminology Tables and Release Types

**Files:**
- Modify: `db/bootstrap/09-terminology.sql`
- Modify: `db/bootstrap/10-triggers.sql`
- Modify: `crates/libs/lib-core/src/model/terminology.rs`
- Modify: `crates/libs/lib-core/src/model/terminology_import.rs`
- Test: `crates/libs/lib-core/tests/terminology/terminology_queries.rs`

**Interfaces:**
- Produces: active scoped-term membership and MFDS product membership/search queries.
- Produces: release dictionaries `iso3166`, `ich_constrained_ucum`, `edqm`, and `mfds_product`.

- [ ] **Step 1: Write failing database/model tests**

Add tests that insert two releases, activate one, and assert:

```rust
assert!(ControlledTermBmc::contains_active(mm, "iso3166", "country", "KR").await?);
assert!(!ControlledTermBmc::contains_active(mm, "iso3166", "country", "ZZ").await?);
assert!(MfdsProductBmc::contains_active(mm, "200000001").await?);
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `cargo test -p lib-core --test terminology terminology_queries -- --nocapture`

Expected: compilation failure because `ControlledTermBmc` and `MfdsProductBmc` do not exist.

- [ ] **Step 3: Add normalized tables and indexes**

Add `controlled_terminology_terms` keyed by dictionary/version/language/scope/code and `mfds_products` keyed by version/item_seq. Add audit IDs, active flags, search indexes, RLS, and audit triggers. Extend the release dictionary check to all supported dictionaries.

- [ ] **Step 4: Add BMC membership/search methods**

Implement batched methods:

```rust
pub async fn existing_active_codes(
    mm: &ModelManager,
    dictionary: &str,
    scope: &str,
    codes: &[String],
) -> Result<HashSet<String>>;

pub async fn existing_active_item_seqs(
    mm: &ModelManager,
    codes: &[String],
) -> Result<HashSet<String>>;
```

- [ ] **Step 5: Extend release activation and rollback**

Activation must deactivate only rows for the same dictionary/language, activate the requested version atomically, retire the prior release, and preserve the active release on failure.

- [ ] **Step 6: Reinitialize the development database and verify GREEN**

Run `docker compose down -v && docker compose up -d postgres`, wait for
`docker compose ps postgres` to report healthy, then run the focused test and
`cargo test -p lib-core --test terminology`.

---

### Task 2: Validator Terminology Context

**Files:**
- Modify: `crates/libs/validator/src/context.rs`
- Modify: `crates/libs/validator/src/allowed_value.rs`
- Modify: `crates/libs/validator/src/catalog.rs`
- Test: `crates/libs/validator/src/allowed_value.rs`
- Test: `crates/libs/validator/src/context.rs`

**Interfaces:**
- Consumes: batched active membership queries from Task 1.
- Produces: `VocabularyContext::contains_active_code(dictionary, scope, code)` without fallback.

- [ ] **Step 1: Write failing tests for finite and parser vocabularies**

Tests must prove active ISO membership accepts `KR`, ICH country scope accepts `EU`, missing scopes fail closed, general UCUM accepts a composed expression through the parser, and constrained UCUM requires exact active membership.

- [ ] **Step 2: Run validator tests and verify RED**

Run: `cargo test -p validator allowed_value -- --nocapture`

- [ ] **Step 3: Replace embedded finite snapshots with loaded active memberships**

Collect codes used by the case, query only those codes, and store results in a map keyed by `(dictionary, scope)`. Keep MedDRA version/term batching and general UCUM parser validation.

- [ ] **Step 4: Remove country and finite-vocabulary fallbacks**

Remove `country_code::CountryCode::VARS`, embedded ISO membership, and snapshot panic paths. Missing release availability returns invalid membership through an explicit unavailable state; it never consults another source.

- [ ] **Step 5: Verify focused and full validator suites**

Run: `cargo test -p validator --lib`.

---

### Task 3: Conditional MFDS Vocabulary Schema and Parity

**Files:**
- Modify: `registry/dictionary.schema.json`
- Modify: `registry/dictionary/mfds-regional.json`
- Modify: `registry/tools/validate.py`
- Modify: `registry/tools/build_dictionary.py`
- Modify: `registry/tools/test_build_dictionary.py`
- Modify: `crates/libs/validator/src/catalog.rs`
- Modify: `crates/libs/validator/src/catalog_dictionary_constraints.rs`

**Interfaces:**
- Produces: conditional vocabulary variants selected by receiver report route.
- Domestic selector: `KR -> MFDS_PRODUCT/item_seq`.
- Foreign selector: `FR -> WHODrug/all`, with the companion version field.

- [ ] **Step 1: Write failing schema/build tests**

Use the following dictionary shape in fixtures:

```json
"vocabulary_variants": [
  {"receiver": "KR", "vocabulary": "MFDS_PRODUCT", "vocabulary_scope": "item_seq"},
  {"receiver": "FR", "vocabulary": "WHODrug", "vocabulary_scope": "all"}
]
```

Assert that variants are mutually unique by receiver and forbidden alongside an unconditional `vocabulary` value.

- [ ] **Step 2: Run registry tests and verify RED**

Run: `python -m unittest registry.tools.test_build_dictionary registry.tools.test_validate`.

- [ ] **Step 3: Extend schema and strict validation**

Add `item_seq` to vocabulary scopes and validate the exact variant structure. Reject missing receiver selectors, duplicate selectors, unknown vocabularies, and mixed unconditional/conditional declarations.

- [ ] **Step 4: Split all three MFDS fields**

Apply the variants to `D.8.r.1.KR.1b`, `D.10.8.r.1.KR.1b`, and `G.k.2.1.KR.1b`. Preserve dictionary rule text, OIDs, lengths, and required conditions.

- [ ] **Step 5: Regenerate catalog constraints and assert parity**

Generated metadata must retain receiver applicability and must not emit an unconditional WHODrug rule for these fields.

- [ ] **Step 6: Run strict dictionary/catalog parity validation**

Run `python registry/tools/validate.py --strict-dictionary` and
`cargo test -p validator catalog -- --nocapture`.

---

### Task 4: MFDS/WHODrug Case Validation

**Files:**
- Modify: `crates/libs/validator/src/case/sections/d.rs`
- Modify: `crates/libs/validator/src/case/sections/g.rs`
- Modify: `crates/libs/validator/src/case/sections/rule_table.rs`
- Test: `crates/libs/validator/src/case/sections/d.rs`
- Test: `crates/libs/validator/src/case/sections/g.rs`

**Interfaces:**
- Consumes: conditional vocabulary metadata from Task 3 and memberships from Task 2.
- Produces: receiver-aware allowed-value failures with concrete indexed field paths.

- [ ] **Step 1: Write failing domestic/foreign characterization tests**

For each of the three fields, test:

- KR accepts only an active MFDS `item_seq`.
- KR rejects a code found only in WHODrug.
- FR accepts only the matching active WHODrug version/code.
- FR rejects a code found only in MFDS products.
- Missing active terminology rejects the populated value.

- [ ] **Step 2: Run focused section tests and verify RED**

Run: `cargo test -p validator case::sections::d` and `cargo test -p validator case::sections::g`.

- [ ] **Step 3: Add a shared conditional vocabulary evaluator**

Extend `rule_table.rs` with one evaluator that selects a declared vocabulary variant from receiver context and evaluates exact membership while preserving concrete indexed paths. Do not add section-local fallback logic.

- [ ] **Step 4: Register D and G table entries**

Move all three fields through the shared evaluator and retain existing required/companion behavior.

- [ ] **Step 5: Verify section and full validator suites**

Run: `cargo test -p validator --lib`.

---

### Task 5: MFDS API Collector and Loader

**Files:**
- Create: `registry/tools/import_mfds_products.py`
- Create: `registry/tools/fixtures/mfds-products-page-1.json`
- Create: `registry/tools/fixtures/mfds-products-page-2.json`
- Create: `registry/tools/test_import_mfds_products.py`
- Modify: `crates/tools/terminology-loader/Cargo.toml`
- Modify: `crates/tools/terminology-loader/src/main.rs`
- Modify: `.gitignore`

**Interfaces:**
- Collector input: `DATA_GO_KR_SERVICE_KEY` and official paginated JSON responses.
- Collector artifact: deterministic JSON containing release metadata and normalized products, excluding the key.
- Loader input: collector artifact; output: staged `mfds_product` terminology release.

- [ ] **Step 1: Write failing Python tests**

Tests cover pagination to `totalCount`, duplicate merge, conflicting identity rejection, cancellation preservation, API error rejection, atomic output, and secret redaction.

- [ ] **Step 2: Run Python tests and verify RED**

Run: `python -m unittest registry.tools.test_import_mfds_products -v`.

- [ ] **Step 3: Implement the standard-library collector**

Use `urllib.request`; read the key only from `os.environ`; percent-encode it only in requests; write raw responses under ignored `tmp/mfds-products/`; and atomically rename the normalized artifact after all pages validate.

- [ ] **Step 4: Write failing loader parse/stage tests**

Add a `MfdsProducts` subcommand and tests that parse the normalized artifact, reject duplicate/conflicting rows, and stage inactive release rows.

- [ ] **Step 5: Implement loader support**

The Python command invokes:

```bash
cargo run -p terminology-loader -- mfds-products \
  --input tmp/mfds-products/mfds-products-<version>.json \
  --version <version>
```

unless `--collect-only` is supplied. The loader stages but does not auto-approve or auto-activate.

- [ ] **Step 6: Verify fixture and dry-run paths without an account**

Run Python tests and `cargo test -p terminology-loader`.

---

### Task 6: Terminology API and End-to-End Verification

**Files:**
- Modify: `crates/services/web-server/src/web/rest/terminology_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/misc.rs`
- Modify: `crates/services/web-server/src/openapi.rs`
- Modify: `crates/services/web-server/tests/api/terminology_contract_web.rs`
- Modify: `registry/catalog-implementation-inventory.md`

**Interfaces:**
- Produces: `GET /api/terminology/mfds-products?q=...&limit=...`.
- Reuses: existing release list, approve, activate, and rollback routes for new dictionaries.

- [ ] **Step 1: Write failing API contract tests**

Assert active-only MFDS product search, code/name matching, permission checks, release activation, and rollback-visible search changes.

- [ ] **Step 2: Run focused web tests and verify RED**

Run: `cargo test -p web-server --test api terminology_contract_web -- --nocapture`.

- [ ] **Step 3: Implement routes and OpenAPI declarations**

Reuse terminology permissions and response envelopes. Do not expose inactive candidate rows through search.

- [ ] **Step 4: Update implementation inventory**

Record the source owner, release state, validator path, API collector dependency, and the fact that live collection remains pending until a portal key is issued.

- [ ] **Step 5: Run full verification**

Run:

```bash
python -m unittest discover -s registry/tools -p 'test_*.py'
cargo test -p validator --lib
cargo test -p lib-core --test terminology
cargo test -p terminology-loader
cargo test -p web-server --test api terminology_contract_web
git diff --check
```

Expected: all commands pass; the live MFDS API test reports skip when `DATA_GO_KR_SERVICE_KEY` is absent.
