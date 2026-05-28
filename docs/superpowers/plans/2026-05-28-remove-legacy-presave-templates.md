# Remove Legacy Presave Templates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the legacy generic presave template route, model, and database tables with no data migration.

**Architecture:** Canonical section presave BMCs and `/api/presaves/{section}` routes become the only backend presave implementation. Internal sender option and XML import code must read canonical sender tables instead of `presave_templates`. Legacy `PRESAVE_TEMPLATE_*` permission names remain for now as INFO presave permissions.

**Tech Stack:** Rust, Axum, sqlx/PostgreSQL bootstrap SQL, `cargo test`, existing web-server API integration test harness.

---

## File Map

- Modify `crates/services/web-server/src/web/routes_rest.rs`: remove `routes_presave_templates` merge.
- Modify `crates/services/web-server/src/web/rest/mod.rs`: remove `presave_template_rest` module and route builder.
- Delete `crates/services/web-server/src/web/rest/presave_template_rest.rs`: legacy REST handlers.
- Modify `crates/services/web-server/src/openapi.rs`: remove legacy OpenAPI path declarations and tag.
- Modify `crates/libs/lib-core/src/model/mod.rs`: remove `presave_template` module export.
- Delete `crates/libs/lib-core/src/model/presave_template.rs`: legacy BMC/model/audit code.
- Modify `crates/services/web-server/src/web/rest/section_presave_rest.rs`: replace `PresaveEntityType` import with a local scope enum.
- Modify `crates/libs/lib-rest-core/src/lib.rs`: replace sender options query against `presave_templates` with canonical sender tables.
- Modify `crates/libs/lib-core/src/xml/import_runtime/c.rs`: replace default sender lookup against `PresaveTemplateBmc` with canonical sender BMCs.
- Modify `db/bootstrap/01-safetydb-schema.sql`: remove legacy tables, indexes, and RLS policies.
- Modify `db/bootstrap/10-triggers.sql`: remove legacy audit/update triggers and trigger functions if they become unused.
- Modify `crates/libs/lib-core/src/_dev_utils/dev_db.rs`: remove legacy compatibility ALTER/index statements.
- Modify or delete affected tests under `crates/services/web-server/tests/api`.
- Delete `crates/libs/lib-core/tests/presave.rs` if no longer needed after model deletion.

## Task 1: Remove Public Route and OpenAPI Surface

**Files:**
- Modify: `crates/services/web-server/src/web/routes_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/mod.rs`
- Delete: `crates/services/web-server/src/web/rest/presave_template_rest.rs`
- Modify: `crates/services/web-server/src/openapi.rs`
- Test: `crates/services/web-server/tests/api/presave_contract_web.rs`

- [ ] **Step 1: Add failing route removal test**

Add a test near other presave contract route tests:

```rust
#[serial]
#[tokio::test]
async fn test_legacy_presave_templates_route_is_removed() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	for uri in [
		"/api/presave-templates".to_string(),
		format!("/api/presave-templates/{}", Uuid::new_v4()),
		format!("/api/presave-templates/{}/audit", Uuid::new_v4()),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{value:?}");
	}

	Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p web-server --test api test_legacy_presave_templates_route_is_removed -- --nocapture --test-threads=1
```

Expected: FAIL because `/api/presave-templates` still returns a routed response.

- [ ] **Step 3: Remove route registration**

In `crates/services/web-server/src/web/routes_rest.rs`, remove:

```rust
.merge(rest::routes_presave_templates(mm.clone()))
```

In `crates/services/web-server/src/web/rest/mod.rs`, remove:

```rust
pub mod presave_template_rest;
```

Delete the whole `routes_presave_templates` function.

- [ ] **Step 4: Remove OpenAPI legacy paths**

In `crates/services/web-server/src/openapi.rs`, remove the path entries:

```rust
list_presave_templates,
create_presave_template,
get_presave_template,
update_presave_template,
delete_presave_template,
list_presave_template_audits,
```

Remove the legacy `presave-templates` tag and the function blocks for paths under:

```text
/api/presave-templates
/api/presave-templates/{id}
/api/presave-templates/{id}/audit
```

- [ ] **Step 5: Delete legacy REST file**

Delete:

```text
crates/services/web-server/src/web/rest/presave_template_rest.rs
```

- [ ] **Step 6: Run route test**

Run:

```bash
cargo test -p web-server --test api test_legacy_presave_templates_route_is_removed -- --nocapture --test-threads=1
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/services/web-server/src/web/routes_rest.rs crates/services/web-server/src/web/rest/mod.rs crates/services/web-server/src/openapi.rs crates/services/web-server/tests/api/presave_contract_web.rs
git add -u crates/services/web-server/src/web/rest/presave_template_rest.rs
git commit -m "Remove legacy presave template routes"
```

## Task 2: Replace Legacy Section Type in Canonical Scope Code

**Files:**
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest.rs`

- [ ] **Step 1: Verify compile fails after model removal is anticipated**

Run:

```bash
cargo check -p web-server
```

Expected before implementation: code still imports `lib_core::model::presave_template::PresaveEntityType`.

- [ ] **Step 2: Add local scope enum**

In `section_presave_rest.rs`, remove:

```rust
use lib_core::model::presave_template::PresaveEntityType;
```

Add near `PresaveAuthorityQuery`:

```rust
#[derive(Clone, Copy)]
enum PresaveScopeSection {
	Sender,
	Product,
	Study,
}
```

Change helper signatures and matches from `PresaveEntityType` to `PresaveScopeSection`:

```rust
async fn allowed_scope_for_section(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	section: PresaveScopeSection,
) -> Result<Option<HashSet<String>>> {
	if lib_rest_core::is_admin(ctx, mm).await? {
		return Ok(None);
	}
	let user: lib_core::model::user::User =
		UserBmc::get(ctx, mm, ctx.user_id()).await?;
	let values = match section {
		PresaveScopeSection::Sender => {
			lib_rest_core::scope_values_from_raw(user.access_sender_ids.as_deref())
		}
		PresaveScopeSection::Product => {
			lib_rest_core::scope_values_from_raw(user.access_product_ids.as_deref())
		}
		PresaveScopeSection::Study => {
			lib_rest_core::scope_values_from_raw(user.access_study_ids.as_deref())
		}
	};
	Ok(Some(normalized_set(values)))
}
```

Rename call sites:

```rust
allowed_scope_for_entity(ctx, mm, PresaveEntityType::Sender)
```

to:

```rust
allowed_scope_for_section(ctx, mm, PresaveScopeSection::Sender)
```

- [ ] **Step 3: Run canonical presave scope tests**

Run:

```bash
cargo test -p web-server --test api test_canonical_product -- --nocapture --test-threads=1
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/services/web-server/src/web/rest/section_presave_rest.rs
git commit -m "Decouple canonical presaves from legacy template type"
```

## Task 3: Replace Runtime Sender Reads

**Files:**
- Modify: `crates/libs/lib-rest-core/src/lib.rs`
- Modify: `crates/libs/lib-core/src/xml/import_runtime/c.rs`
- Test: affected existing import/routing tests

- [ ] **Step 1: Update sender option SQL**

In `crates/libs/lib-rest-core/src/lib.rs`, replace the `sender_master_options` CTE that reads `presave_templates` with canonical sender gateway rows:

```sql
WITH sender_master_options AS (
	SELECT DISTINCT
	       NULLIF(BTRIM(g.sender_identifier), '') AS sender_identifier,
	       0::bigint AS case_count
	FROM sender_presaves s
	JOIN sender_presave_gateways g ON g.sender_presave_id = s.id
	WHERE s.organization_id = $1
	  AND s.deleted = FALSE
	  AND NULLIF(BTRIM(g.sender_identifier), '') IS NOT NULL
),
case_sender_options AS (
	...
)
```

Keep the existing `case_sender_options` CTE and final aggregation unchanged.

- [ ] **Step 2: Run sender option tests**

Run the specific tests that mention sender template access:

```bash
cargo test -p web-server --test api sender_template_access -- --nocapture --test-threads=1
```

If the test filter finds no exact test, run:

```bash
cargo test -p web-server --test api routing_profile_sender_options -- --nocapture --test-threads=1
```

Expected: FAIL until test setup creates canonical sender/gateway records instead of `/api/presave-templates`.

- [ ] **Step 3: Update XML import default sender lookup**

In `crates/libs/lib-core/src/xml/import_runtime/c.rs`, remove imports:

```rust
use crate::model::presave_template::{
	PresaveEntityType, PresaveTemplateBmc, PresaveTemplateListFilter,
};
```

Add canonical imports:

```rust
use crate::model::presave::{
	SenderPresaveBmc, SenderPresaveGatewayBmc, SenderPresaveResponsiblePersonBmc,
};
```

Rewrite `default_sender_from_presave` to:

```rust
async fn default_sender_from_presave(
	ctx: &Ctx,
	mm: &ModelManager,
	authority: Option<RegulatoryAuthority>,
) -> Result<Option<c_helpers::SenderImport>> {
	let mut senders = SenderPresaveBmc::list(ctx, mm, None)
		.await
		.map_err(Error::Model)?;
	senders.retain(|sender| {
		!sender.deleted
			&& sender.is_default
			&& authority.map_or(true, |authority| sender.authority == authority)
	});
	let Some(sender) = senders.into_iter().next() else {
		return Ok(None);
	};
	let gateways = SenderPresaveGatewayBmc::list_by_parent(ctx, mm, sender.id)
		.await
		.map_err(Error::Model)?;
	let gateway = gateways
		.iter()
		.find(|gateway| gateway.is_default_for_authority.unwrap_or(false))
		.or_else(|| gateways.first());
	let responsible_people =
		SenderPresaveResponsiblePersonBmc::list_by_parent(ctx, mm, sender.id)
			.await
			.map_err(Error::Model)?;
	let responsible = responsible_people
		.iter()
		.find(|person| person.is_default.unwrap_or(false))
		.or_else(|| responsible_people.first());

	Ok(Some(c_helpers::SenderImport {
		sender_type: sender.sender_type,
		sender_organization: sender.organization_name,
		sender_department: sender.department,
		sender_title: responsible.and_then(|person| person.person_title.clone()),
		sender_given_name: responsible.and_then(|person| person.person_given_name.clone()),
		sender_middle_name: responsible.and_then(|person| person.person_middle_name.clone()),
		sender_family_name: responsible.and_then(|person| person.person_family_name.clone()),
		sender_street: sender.street_address,
		sender_city: sender.city,
		sender_state: sender.state,
		sender_postcode: sender.postcode,
		sender_country_code: sender.country_code,
		sender_telephone: sender.telephone,
		sender_fax: sender.fax,
		sender_email: sender.email,
		message_sender_identifier: gateway.and_then(|gateway| gateway.sender_identifier.clone()),
		message_receiver_identifier: None,
		batch_sender_identifier: gateway.and_then(|gateway| gateway.routing_identifier.clone()),
		batch_receiver_identifier: None,
	}))
}
```

If field names differ in `SenderImport`, inspect the existing struct in `c_helpers` and map the same fields currently populated from JSON.

- [ ] **Step 4: Run import tests**

Run:

```bash
cargo test -p web-server --test api import_contract_web -- --nocapture --test-threads=1
```

Expected: FAIL until tests seed canonical sender data.

- [ ] **Step 5: Commit runtime replacement**

```bash
git add crates/libs/lib-rest-core/src/lib.rs crates/libs/lib-core/src/xml/import_runtime/c.rs
git commit -m "Read sender presaves from canonical tables"
```

## Task 4: Remove Legacy Model and Schema

**Files:**
- Modify: `crates/libs/lib-core/src/model/mod.rs`
- Delete: `crates/libs/lib-core/src/model/presave_template.rs`
- Delete or rewrite: `crates/libs/lib-core/tests/presave.rs`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `db/bootstrap/10-triggers.sql`
- Modify: `crates/libs/lib-core/src/_dev_utils/dev_db.rs`

- [ ] **Step 1: Remove model module export**

In `crates/libs/lib-core/src/model/mod.rs`, remove:

```rust
pub mod presave_template;
```

Delete:

```text
crates/libs/lib-core/src/model/presave_template.rs
crates/libs/lib-core/tests/presave.rs
```

- [ ] **Step 2: Remove schema objects**

In `db/bootstrap/01-safetydb-schema.sql`, remove the `CREATE TABLE` blocks for:

```sql
presave_templates
presave_template_audits
```

Remove legacy indexes:

```sql
idx_presave_templates_org
idx_presave_templates_entity_type
idx_presave_templates_authority
idx_presave_templates_entity_authority
idx_presave_templates_created_by
idx_presave_templates_created_at
idx_presave_template_audits_template_id
idx_presave_template_audits_org
```

Remove RLS blocks for:

```sql
presave_templates
presave_template_audits
```

- [ ] **Step 3: Remove legacy triggers**

In `db/bootstrap/10-triggers.sql`, remove:

```sql
CREATE TRIGGER audit_presave_templates ...
CREATE TRIGGER audit_presave_templates_dedicated ...
CREATE TRIGGER update_presave_templates_updated_at ...
```

Remove trigger function code that only inserts into `presave_template_audits` if it has no remaining use.

- [ ] **Step 4: Remove dev-db compatibility ALTERs**

In `crates/libs/lib-core/src/_dev_utils/dev_db.rs`, remove statements that alter or index `presave_templates`.

- [ ] **Step 5: Run lib-core tests**

Run:

```bash
cargo test -p lib-core
```

Expected: PASS after all imports/tests referencing `presave_template` are removed or rewritten.

- [ ] **Step 6: Commit**

```bash
git add crates/libs/lib-core/src/model/mod.rs db/bootstrap/01-safetydb-schema.sql db/bootstrap/10-triggers.sql crates/libs/lib-core/src/_dev_utils/dev_db.rs
git add -u crates/libs/lib-core/src/model/presave_template.rs crates/libs/lib-core/tests/presave.rs
git commit -m "Remove legacy presave template model and tables"
```

## Task 5: Rewrite or Delete Legacy API Tests

**Files:**
- Modify: `crates/services/web-server/tests/api/presave_contract_web.rs`
- Modify: `crates/services/web-server/tests/api/import_contract_web.rs`
- Modify: `crates/services/web-server/tests/api/submission_lifecycle_web.rs`
- Modify: `crates/services/web-server/tests/api/scope_visibility_web.rs`

- [ ] **Step 1: Remove pure legacy template tests**

Delete tests in `presave_contract_web.rs` that only validate generic template behavior:

```text
test_presave_contract_supports_all_six_entity_types
test_presave_contract_rejects_invalid_entity_type
test_presave_contract_enforces_org_isolation
test_presave_contract_write_requires_admin
test_presave_contract_update_delete_and_audit
test_presave_templates_filter_by_authority_and_include_global
test_presave_update_delete_respect_assigned_product_scope
test_presave_product_list_follows_assigned_product_scope
test_presave_sender_list_follows_assigned_sender_scope
test_presave_study_list_follows_assigned_study_scope
test_presave_audit_respects_assigned_scope
test_presave_non_sender_sender_default_flag_does_not_clear_default_sender
test_presave_sender_default_is_org_level_singleton
test_presave_sender_default_is_authority_scoped
```

Keep canonical tests including:

```text
test_canonical_product_presaves_respect_assigned_product_scope
test_canonical_product_parent_soft_delete_requires_delete_permission
test_section_presave_*_rest_contract
*_presave_details_*
```

- [ ] **Step 2: Rewrite setup helpers**

Replace helper calls that POST `/api/presave-templates` with canonical helpers already present in `presave_contract_web.rs`, such as:

```rust
create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;
create_receiver_presave_via_api(&app, &admin_cookie, "fda").await?;
create_product_presave_via_api(&app, &admin_cookie, "fda").await?;
```

Where tests need a sender identifier, create a sender and gateway:

```rust
let sender_id = create_sender_presave_via_api(&app, &admin_cookie, "fda").await?;
let gateway_id = create_sender_gateway_via_api(
	&app,
	&admin_cookie,
	sender_id,
	1,
	"FDA",
	"SENDER-VISIBLE",
)
.await?;
```

If that helper does not exist, add it beside the other canonical helper functions.

- [ ] **Step 3: Run affected API suites**

Run:

```bash
cargo test -p web-server --test api presave_contract_web -- --nocapture --test-threads=1
cargo test -p web-server --test api import_contract_web -- --nocapture --test-threads=1
cargo test -p web-server --test api submission_lifecycle_web -- --nocapture --test-threads=1
cargo test -p web-server --test api scope_visibility_web -- --nocapture --test-threads=1
```

Expected: PASS after all legacy helper usage is removed.

- [ ] **Step 4: Commit**

```bash
git add crates/services/web-server/tests/api/presave_contract_web.rs crates/services/web-server/tests/api/import_contract_web.rs crates/services/web-server/tests/api/submission_lifecycle_web.rs crates/services/web-server/tests/api/scope_visibility_web.rs
git commit -m "Update tests for canonical presaves only"
```

## Task 6: Final Scan and Verification

**Files:**
- Any file found by the scans below.

- [ ] **Step 1: Scan runtime code for removed symbols**

Run:

```bash
rg -n "PresaveTemplateBmc|PresaveTemplateAuditBmc|PresaveTemplateFor|PresaveTemplateListFilter|PresaveEntityType|presave_template_rest|/api/presave-templates|presave_templates|presave_template_audits" crates db
```

Expected: no matches in runtime code, bootstrap SQL, or tests. If matches remain in historical docs only, leave them.

- [ ] **Step 2: Format**

Run:

```bash
cargo fmt
```

Expected: no output or successful formatting.

- [ ] **Step 3: Full verification**

Run:

```bash
cargo test -p lib-core
cargo test -p lib-rest-core
cargo test -p web-server --test api presave_contract_web -- --nocapture --test-threads=1
cargo test -p web-server --test api import_contract_web -- --nocapture --test-threads=1
cargo test -p web-server --test api scope_visibility_web -- --nocapture --test-threads=1
cargo test -p web-server --test api submission_lifecycle_web -- --nocapture --test-threads=1
```

Expected: PASS.

- [ ] **Step 4: Commit final cleanup**

```bash
git add crates db
git commit -m "Finalize legacy presave template removal"
```

If there are no remaining changes after verification, do not create an empty commit.

## Self-Review

- Spec coverage: The plan covers route removal, OpenAPI cleanup, legacy model deletion, bootstrap schema removal, dev-db cleanup, runtime sender option/import replacement, and test rewrites.
- Placeholder scan: No intentional placeholders remain. The only conditional instruction is to inspect `SenderImport` field names if they differ from the existing code, because that struct must be mapped exactly from local code during implementation.
- Type consistency: The plan consistently uses canonical section BMCs and a local `PresaveScopeSection` replacement instead of keeping `PresaveEntityType`.
