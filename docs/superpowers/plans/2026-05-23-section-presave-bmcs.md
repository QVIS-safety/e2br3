# Section Presave BMCs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement section-specific INFO presave BMCs/tables with authority-aware fields, field-level audit support through normal table columns, and TDD coverage.

**Architecture:** Replace new development on generic `PresaveTemplateBmc` with section-specific Rust models in `lib-core`. Each section table has its own metadata and real columns; authority-specific columns are nullable and enforced by BMC validation. Repeatable INFO rows use child tables.

**Tech Stack:** Rust, sqlx, Postgres bootstrap SQL, `base_uuid` model helpers, existing `RegulatoryAuthority`, `audit_logs` triggers, serial integration tests.

---

## Files

- Create: `crates/libs/lib-core/src/model/presave.rs`
- Modify: `crates/libs/lib-core/src/model/mod.rs`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `db/bootstrap/10-triggers.sql`
- Create: `crates/libs/lib-core/tests/section_presave.rs`

## Task 1: Add Section Presave Tables And Audit Triggers

**Files:**
- Test: `crates/libs/lib-core/tests/section_presave.rs`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `db/bootstrap/10-triggers.sql`

- [ ] **Step 1: Write failing table-existence test**

Add `crates/libs/lib-core/tests/section_presave.rs` with:

```rust
mod common;

use crate::common::{demo_ctx, init_test_mm, Result};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn section_presave_tables_exist() -> Result<()> {
	let mm = init_test_mm().await;
	let ctx = demo_ctx();
	let tables = [
		"sender_presaves",
		"sender_presave_gateways",
		"sender_presave_responsible_persons",
		"receiver_presaves",
		"receiver_presave_consignees",
		"product_presaves",
		"product_presave_substances",
		"product_presave_fda_cross_reported_inds",
		"product_presave_mfds_regional_items",
		"reporter_presaves",
		"study_presaves",
		"study_presave_registration_numbers",
		"narrative_presaves",
		"narrative_presave_sender_diagnoses",
		"narrative_presave_case_summaries",
	];

	for table in tables {
		let exists: bool = sqlx::query_scalar(
			"SELECT EXISTS (
				SELECT 1 FROM information_schema.tables
				WHERE table_schema = 'public' AND table_name = $1
			)",
		)
		.bind(table)
		.fetch_one(mm.dbx().db())
		.await?;
		assert!(exists, "missing table {table}");
	}

	let _ = ctx;
	Ok(())
}
```

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test -p lib-core --test section_presave section_presave_tables_exist -- --nocapture
```

Expected: FAIL because `sender_presaves` and other section presave tables do not exist.

- [ ] **Step 3: Add bootstrap tables**

Add section presave tables to `db/bootstrap/01-safetydb-schema.sql` after `presave_template_audits`. Tables must include metadata columns, authority checks, FK relationships, and child-table unique sequence constraints exactly as described in `docs/superpowers/specs/2026-05-23-section-presave-bmc-design.md`.

- [ ] **Step 4: Add audit and updated_at triggers**

Add audit triggers in `db/bootstrap/10-triggers.sql` for every new section presave parent and child table:

```sql
CREATE TRIGGER audit_sender_presaves AFTER INSERT OR UPDATE OR DELETE ON sender_presaves
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();
```

Also add `update_*_updated_at` triggers using the existing updated-at trigger function.

- [ ] **Step 5: Run test to verify GREEN**

Run:

```bash
cargo test -p lib-core --test section_presave section_presave_tables_exist -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/libs/lib-core/tests/section_presave.rs db/bootstrap/01-safetydb-schema.sql db/bootstrap/10-triggers.sql
git commit -m "feat: add section presave tables"
```

## Task 2: Add Parent Section Presave BMCs

**Files:**
- Test: `crates/libs/lib-core/tests/section_presave.rs`
- Create: `crates/libs/lib-core/src/model/presave.rs`
- Modify: `crates/libs/lib-core/src/model/mod.rs`

- [ ] **Step 1: Write failing CRUD test for parent BMCs**

Append tests that import:

```rust
use lib_core::model::presave::{
	SenderPresaveBmc, SenderPresaveForCreate, SenderPresaveForUpdate,
	ReceiverPresaveBmc, ReceiverPresaveForCreate,
	ProductPresaveBmc, ProductPresaveForCreate,
	ReporterPresaveBmc, ReporterPresaveForCreate,
	StudyPresaveBmc, StudyPresaveForCreate,
	NarrativePresaveBmc, NarrativePresaveForCreate,
};
use lib_core::regulatory::RegulatoryAuthority;
```

Create one record per parent BMC, assert `authority`, `name`, and one section field round-trip. Update sender `organization_name`, then list sender records by authority and assert the created row appears.

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test -p lib-core --test section_presave section_presave_parent_bmcs_crud_roundtrip -- --nocapture
```

Expected: FAIL because `model::presave` and section BMCs do not exist.

- [ ] **Step 3: Implement parent structs and BMCs**

Create `crates/libs/lib-core/src/model/presave.rs` with parent structs, `ForCreate`, `ForUpdate`, filters, and BMCs:

```rust
pub struct SenderPresaveBmc;
impl DbBmc for SenderPresaveBmc {
	const TABLE: &'static str = "sender_presaves";
}
```

Use `base_uuid::create`, `base_uuid::get`, `base_uuid::list`, `base_uuid::update`, and `base_uuid::delete` for parent CRUD. Add `pub mod presave;` to `model/mod.rs`.

- [ ] **Step 4: Run test to verify GREEN**

Run:

```bash
cargo test -p lib-core --test section_presave section_presave_parent_bmcs_crud_roundtrip -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/lib-core/src/model/presave.rs crates/libs/lib-core/src/model/mod.rs crates/libs/lib-core/tests/section_presave.rs
git commit -m "feat: add section presave parent bmcs"
```

## Task 3: Add Child Repeatable BMCs

**Files:**
- Test: `crates/libs/lib-core/tests/section_presave.rs`
- Modify: `crates/libs/lib-core/src/model/presave.rs`

- [ ] **Step 1: Write failing child CRUD test**

Add a test that creates a sender, product, study, and narrative presave, then creates:

- `SenderPresaveGateway`
- `SenderPresaveResponsiblePerson`
- `ProductPresaveSubstance`
- `ProductPresaveFdaCrossReportedInd`
- `ProductPresaveMfdsRegionalItem`
- `StudyPresaveRegistrationNumber`
- `NarrativePresaveSenderDiagnosis`
- `NarrativePresaveCaseSummary`

Assert each child round-trips and is listable by parent ID.

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test -p lib-core --test section_presave section_presave_child_bmcs_crud_roundtrip -- --nocapture
```

Expected: FAIL because child BMCs do not exist.

- [ ] **Step 3: Implement child structs and BMCs**

Add child structs, `ForCreate`, `ForUpdate`, parent filters, and BMCs in `presave.rs`. Each child BMC uses `base_uuid` helpers and has a `list_by_parent` convenience method.

- [ ] **Step 4: Run test to verify GREEN**

Run:

```bash
cargo test -p lib-core --test section_presave section_presave_child_bmcs_crud_roundtrip -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/lib-core/src/model/presave.rs crates/libs/lib-core/tests/section_presave.rs
git commit -m "feat: add section presave child bmcs"
```

## Task 4: Add Authority Validation

**Files:**
- Test: `crates/libs/lib-core/tests/section_presave.rs`
- Modify: `crates/libs/lib-core/src/model/presave.rs`

- [ ] **Step 1: Write failing authority validation tests**

Add tests:

- ICH product rejects non-null `fda_ind_number_occurred`.
- FDA product rejects non-null `mfds_domestic_product_code`.
- MFDS product rejects non-null `fda_ind_number_occurred`.
- MFDS reporter accepts `qualification_kr1`.
- FDA reporter rejects `qualification_kr1`.
- MFDS study accepts `study_type_reaction_kr1`.
- ICH study rejects `study_type_reaction_kr1`.

- [ ] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test -p lib-core --test section_presave authority_specific_fields_are_enforced -- --nocapture
```

Expected: FAIL because BMCs do not validate inactive authority fields.

- [ ] **Step 3: Implement validation**

Add validation helpers in `presave.rs`, called by create/update methods for Product, Reporter, and Study. Return `crate::model::Error::Store` with a clear message such as `inactive FDA product fields are not allowed for mfds presave`.

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```bash
cargo test -p lib-core --test section_presave authority_specific_fields_are_enforced -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/lib-core/src/model/presave.rs crates/libs/lib-core/tests/section_presave.rs
git commit -m "feat: enforce presave authority fields"
```

## Task 5: Add Field Audit Verification

**Files:**
- Test: `crates/libs/lib-core/tests/section_presave.rs`

- [ ] **Step 1: Write failing/passing audit behavior test**

Add a test that creates a product presave, updates `brand_name`, switches to auditor role with the existing test helper pattern, and queries:

```sql
SELECT changed_fields
FROM audit_logs
WHERE table_name = 'product_presaves'
  AND record_id = $1
  AND action = 'UPDATE'
ORDER BY created_at DESC
LIMIT 1
```

Assert `changed_fields` contains `brand_name`.

- [ ] **Step 2: Run test to verify RED or existing trigger failure**

Run:

```bash
cargo test -p lib-core --test section_presave section_presave_field_audit_records_changed_column -- --nocapture
```

Expected before Task 1 triggers: FAIL. Expected after Task 1 if triggers are correct: PASS. If it passes immediately at this point, document that Task 1 already implemented the behavior and keep the test.

- [ ] **Step 3: Fix triggers if needed**

If the audit test fails, adjust `db/bootstrap/10-triggers.sql` so `product_presaves` uses `audit_trigger_function()`.

- [ ] **Step 4: Run test to verify GREEN**

Run:

```bash
cargo test -p lib-core --test section_presave section_presave_field_audit_records_changed_column -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/lib-core/tests/section_presave.rs db/bootstrap/10-triggers.sql
git commit -m "test: verify section presave field audit"
```

## Task 6: Final Verification

- [ ] **Step 1: Run section presave tests**

```bash
cargo test -p lib-core --test section_presave -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run existing presave tests**

```bash
cargo test -p lib-core --test presave -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run formatting/checks**

```bash
cargo fmt --check
cargo check -p lib-core
```

Expected: PASS.

- [ ] **Step 4: Commit any final fixes**

```bash
git status --short
git add crates/libs/lib-core/src/model/presave.rs crates/libs/lib-core/src/model/mod.rs crates/libs/lib-core/tests/section_presave.rs db/bootstrap/01-safetydb-schema.sql db/bootstrap/10-triggers.sql
git commit -m "fix: finalize section presave bmcs"
```
