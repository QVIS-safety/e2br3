# Safe RBAC Policy Kernel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace role strings, menu-policy expansion, mutable process caches, duplicated admin gates, and handwritten frontend permission expressions with the approved PDF-derived typed Policy Kernel.

**Architecture:** A deterministic Rust Policy Registry declares grants, entitlements, actions, contextual decision stages, immutable built-in identities, and authorization-fact revision domains. PostgreSQL stores normalized roles, one role assignment per user/organization, canonical grants, revision state, isolation context, and append-only audit data; request middleware builds one immutable snapshot, while contextual reads and mutations require transaction-bound typed permits. Backend generators emit the frontend Action IDs, PDF Role & Privilege rows, endpoint metadata, and catalog hash, so the frontend renders backend decisions rather than reconstructing policy.

**Tech Stack:** Rust 2021, Axum 0.8, SQLx 0.8, PostgreSQL RLS/triggers, Serde, SHA-256, Next.js, React, TypeScript, Jest.

## Global Constraints

- Product behavior follows `QVIS Safety Database_UI Specification_18JUN2026_Updated.pdf`, with pages 7, 8, 41, 94, and 95 as the primary RBAC references.
- One enabled user-organization membership has exactly one active RBAC role; PDF pages 12 and 97 allow multiple roles only in a workflow-status rule.
- CASE Review, Lock, ordinary Edit, and Audit Trail read are independent actions; reviewed or locked cases retain Audit Trail access.
- Report Due Mail Read and Send remain visible, reserved, disabled, and unassignable; Settings Read remains absent.
- Privileged identity derives only from immutable registry-declared built-in UUIDs, never names, `stable_key`, role class, or grants.
- `SubjectOnly` is valid only when no contextual fact can change the decision; Collection, Proposed, Existing, Parent, and ResourceSet actions require a final typed authorizer.
- Contextual reads require `AuthorizedRead<'tx, C>` and contextual writes require `AuthorizedMutation<'tx, C>` in the same database transaction as projection or mutation.
- Complete authorization snapshots, principal facts, and final decisions are request-local and are never stored in a process-global or cross-request cache.
- Public organization/principal revisions are canonical decimal strings; compound snapshot versions use identity equality, never numeric ordering.
- Access windows are start-inclusive and end-exclusive: `[access_start_at, access_end_at)`.
- RLS consumes typed snapshot-derived isolation context and never interprets role strings, grants, menu keys, or frontend concepts.
- Frontend production authorization contains no role-name checks, raw permissions, summary booleans, or handwritten entitlement expressions after cutover.
- Backend and frontend deploy as one coordinated contract cutover; mixed legacy/new production authorization is unsupported.
- Before Task 1, use `superpowers:using-git-worktrees` to create isolated `codex/rbac-policy-kernel` worktrees from backend `dev` (including this plan commit) and frontend `origin/dev`; never execute this plan in either currently dirty/shared worktree. Record the returned absolute paths as `RBAC_BACKEND_WORKTREE` and `RBAC_FRONTEND_WORKTREE`, run backend commands from the former, and use the latter wherever this plan says “Run in frontend.”
- Every behavior change follows red-green TDD and every task ends with a focused commit.

## Repository and File Boundaries

Backend repository: `/Users/hyundonghoon/projects/rust/e2br3/e2br3`

- `crates/libs/lib-core/src/authorization/`: deterministic policy types, registry, fact definitions, snapshots, kernel, contexts, and permits.
- `crates/libs/lib-core/src/model/authorization/`: normalized storage repositories, snapshot loading, role administration, revisions, and audit persistence.
- `crates/libs/lib-core/src/model/store/`: typed PostgreSQL isolation/compliance context only.
- `crates/libs/lib-web/src/middleware/`: request snapshot extraction, typed guards, response snapshot headers, and error mapping.
- `crates/services/web-server/src/web/authorization/`: protected route bindings and contextual domain authorization adapters.
- `crates/services/web-server/src/web/rest/`: handlers reduced to extraction and domain-service calls; no direct policy predicates.
- `db/migrations/` and `db/bootstrap/01-safetydb-schema.sql`: normalized schema, backfill, triggers, constraints, RLS, and audit tables.
- `scripts/` and Rust examples: generated frontend authorization contract.
- `crates/services/web-server/tests/authz/`: cross-process, RLS, race, completeness, and security tests.

Frontend repository: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`

- `lib/auth/generated-authorization.ts`: generated Action IDs, PDF grant rows, endpoint metadata, and `CATALOG_HASH`.
- `lib/auth/action-access.ts`: eligibility and existing-resource allowed-action helpers.
- `lib/auth/policy-sync.ts`: opaque snapshot-token and validity-boundary synchronizer.
- `lib/contexts/AuthContext.tsx`: profile snapshot state, mutation pause, update-required state, and boundary refresh.
- `lib/api/client.ts`: response snapshot observation and authoritative error handling.
- `app/(protected)/admin/role*`: explicit-save custom-role editor and PDF account-context behavior.
- `__tests__/auth/` and `__tests__/rbac-contract/`: generated-contract, sync, static-policy, and role-editor tests.

---

### Task 1: Freeze the Legacy Behavior and Route Inventory

**Files:**
- Create: `crates/services/web-server/tests/authz/policy_kernel_characterization.rs`
- Create: `crates/services/web-server/tests/authz/protected_route_inventory.rs`
- Modify: `crates/services/web-server/tests/authz.rs`
- Create: `scripts/check_legacy_authorization_paths.sh`

**Interfaces:**
- Consumes: current Axum router, current `Ctx`, `has_permission`, `RequireAdmin`, `permission_contract.rs`, and test database helpers.
- Produces: `ProtectedRouteRecord { method, path, public }`, a checked-in characterization snapshot, and a static legacy-path report used as the removal baseline.

- [ ] **Step 1: Write failing characterization tests**

Add tests asserting that the inventory contains `/api/cases`, `/api/cases/{id}/review/toggle`, `/api/cases/export/xml`, `/api/import/xml`, `/api/users`, `/api/admin/permission-profiles`, and `/api/audit-logs`, and that every non-public `/api` route has one inventory record.

```rust
#[test]
fn protected_inventory_covers_representative_context_kinds() {
	let inventory = protected_route_inventory();
	for expected in [
		("GET", "/api/cases"),
		("POST", "/api/cases"),
		("POST", "/api/cases/{id}/review/toggle"),
		("POST", "/api/cases/export/xml"),
		("POST", "/api/import/xml"),
		("GET", "/api/audit-logs"),
	] {
		assert!(inventory.iter().any(|row| (row.method, row.path) == expected));
	}
}
```

- [ ] **Step 2: Run the tests and capture the failure**

Run: `cargo test -p web-server --test authz policy_kernel_characterization -- --test-threads=1`

Run: `cargo test -p web-server --test authz protected_route_inventory -- --test-threads=1`

Expected: FAIL because `protected_route_inventory` and the complete snapshot do not exist.

- [ ] **Step 3: Add the inventory extractor and legacy scan**

Implement route recording without changing authorization behavior. The scan must report direct calls to `has_permission`, `require_permission`, `ctx.is_admin`, `RequireAdmin`, `require_admin`, `can_access_user_admin`, `permission_contract`, `set_org_context`, and `Ctx::can_modify` under production Rust sources.

```bash
#!/usr/bin/env bash
set -euo pipefail
mode="${1:---report}"
matches="$(rg -n 'has_permission|require_permission|ctx\.is_admin|RequireAdmin|require_admin|can_access_user_admin|permission_contract|set_org_context|can_modify' \
  crates/libs crates/services/web-server/src || true)"
if [[ -n "$matches" ]]; then
  printf '%s\n' "$matches"
  [[ "$mode" != "--enforce-zero" ]]
fi
```

- [ ] **Step 4: Verify the characterization baseline**

Run: `cargo test -p web-server --test authz policy_kernel_characterization -- --test-threads=1`

Run: `cargo test -p web-server --test authz protected_route_inventory -- --test-threads=1`

Expected: PASS with every current route inventoried; the legacy scan intentionally prints the paths that later tasks must reduce to zero.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/policy_kernel_characterization.rs crates/services/web-server/tests/authz/protected_route_inventory.rs scripts/check_legacy_authorization_paths.sh
git commit -m "test: characterize legacy authorization surface"
```

### Task 2: Introduce the Typed Policy Registry

**Files:**
- Create: `crates/libs/lib-core/src/authorization/mod.rs`
- Create: `crates/libs/lib-core/src/authorization/ids.rs`
- Create: `crates/libs/lib-core/src/authorization/registry.rs`
- Create: `crates/libs/lib-core/src/authorization/definitions.rs`
- Create: `crates/libs/lib-core/src/authorization/tests.rs`
- Modify: `crates/libs/lib-core/src/lib.rs`

**Interfaces:**
- Produces: `GrantId`, `EntitlementId`, `ActionId`, `SubjectActionId`, `ContextActionId<C>`, `ContextKind`, `ProposalKind`, `ResourceKind`, `GrantDefinition`, `LegacyGrantAlias`, `ActionPolicy`, `AuthorizationFactDefinition`, `PolicyRegistry`, and `policy_registry()`.
- Stable built-in role UUIDs: platform administrator `00000000-0000-0000-0000-000000000101`, sponsor CRO `00000000-0000-0000-0000-000000000102`, sponsor company `00000000-0000-0000-0000-000000000103`, operational user `00000000-0000-0000-0000-000000000104`, internal service principal `00000000-0000-0000-0000-000000000105`.

- [ ] **Step 1: Write registry graph tests**

Test unique identifiers, unknown references, implication cycles, reserved-grant persistence rejection, CASE Review/Lock separation, missing Settings Read, reserved Report Due Mail rows, and immutable built-in UUID uniqueness.

```rust
#[test]
fn pdf_sensitive_grants_are_explicit() {
	let registry = policy_registry();
	assert_eq!(registry.grant("case.review").unwrap().availability, Availability::Implemented);
	assert_eq!(registry.grant("case.lock").unwrap().availability, Availability::Implemented);
	assert_eq!(registry.grant("email.report_due.read").unwrap().availability, Availability::Reserved);
	assert_eq!(registry.grant("email.report_due.send").unwrap().availability, Availability::Reserved);
	assert!(registry.grant("settings.read").is_none());
}
```

- [ ] **Step 2: Verify tests fail**

Run: `cargo test -p lib-core authorization::tests`

Expected: FAIL because the authorization module and registry do not exist.

- [ ] **Step 3: Implement identifiers and closed decision contexts**

Use private tuple fields and checked constructors. Define exactly these decision variants:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextKind {
	Collection(ResourceKind),
	Proposed(ProposalKind),
	Existing(ResourceKind),
	Parent { parent: ResourceKind, child: ResourceKind },
	ResourceSet(ResourceKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionStage {
	SubjectOnly,
	ContextRequired(ContextKind),
}
```

- [ ] **Step 4: Implement the PDF grants, entitlements, actions, identities, and fact definitions**

Keep registry construction deterministic, validate the whole graph in `PolicyRegistry::build`, and expose only the validated singleton from `policy_registry()`. Map `case.review.toggle` to `Existing(Case)`, `case.lock.toggle` to `Existing(Case)`, `audit_log.list` to `Collection(AuditLog)`, `case.create` to `Proposed(CaseCreate)`, and `case.export.xml_set` to `ResourceSet(Case)`.

- [ ] **Step 5: Verify registry tests pass**

Run: `cargo test -p lib-core authorization::tests`

Expected: PASS; malformed test registries fail with stable validation errors.

- [ ] **Step 6: Commit**

```bash
git add crates/libs/lib-core/src/lib.rs crates/libs/lib-core/src/authorization
git commit -m "feat: add typed authorization policy registry"
```

### Task 3: Generate the Registry-Owned Backend and Frontend Contract

**Files:**
- Create: `crates/libs/lib-core/examples/export_authorization_contract.rs`
- Create: `scripts/generate_frontend_authorization.sh`
- Create: `crates/libs/lib-core/tests/authorization_contract_snapshot.rs`
- Create in frontend: `lib/auth/generated-authorization.ts`
- Create in frontend: `__tests__/auth/generated-authorization.test.ts`
- Remove at final cutover: `scripts/generate_frontend_permissions.sh`
- Remove at final cutover: `scripts/generate_frontend_endpoint_permissions.sh`

**Interfaces:**
- Consumes: `policy_registry()`.
- Produces: `Action`, `ActionId`, `PDF_ROLE_PRIVILEGE_ROWS`, `CATALOG_HASH`, and canonical catalog JSON. Endpoint metadata is added from the route registry in Task 10, not guessed here.

- [ ] **Step 1: Write failing snapshot and frontend generated-contract tests**

```rust
#[test]
fn catalog_hash_is_stable_for_canonical_registry_json() {
	let first = export_contract(policy_registry()).unwrap();
	let second = export_contract(policy_registry()).unwrap();
	assert_eq!(first.catalog_hash, second.catalog_hash);
	assert_eq!(first.canonical_json, second.canonical_json);
}
```

```typescript
import { Action, CATALOG_HASH, PDF_ROLE_PRIVILEGE_ROWS } from "@/lib/auth/generated-authorization";

it("contains the PDF-sensitive action and grant contract", () => {
  expect(Action.CaseReviewToggle).toBe("case.review.toggle");
  expect(Action.CaseLockToggle).toBe("case.lock.toggle");
  expect(PDF_ROLE_PRIVILEGE_ROWS.find((row) => row.grantId === "settings.read")).toBeUndefined();
  expect(CATALOG_HASH).toMatch(/^[0-9a-f]{64}$/);
});
```

- [ ] **Step 2: Verify both tests fail**

Run: `cargo test -p lib-core --test authorization_contract_snapshot`

Run in frontend: `npm test -- --runInBand __tests__/auth/generated-authorization.test.ts`

Expected: FAIL because the exporter and generated TypeScript file do not exist.

- [ ] **Step 3: Implement canonical export and SHA-256 hash**

Sort grants, entitlements, actions, endpoints, and fact IDs before serialization. Use lowercase hexadecimal SHA-256 of the canonical UTF-8 JSON. The shell script accepts one frontend repository path and writes only `lib/auth/generated-authorization.ts`.

- [ ] **Step 4: Generate and verify the artifact**

Run: `./scripts/generate_frontend_authorization.sh "$RBAC_FRONTEND_WORKTREE"`

Run: `cargo test -p lib-core --test authorization_contract_snapshot`

Run in frontend: `npm test -- --runInBand __tests__/auth/generated-authorization.test.ts`

Expected: PASS and a second generation produces no diff.

- [ ] **Step 5: Commit once in each repository**

```bash
git add crates/libs/lib-core/examples/export_authorization_contract.rs crates/libs/lib-core/tests/authorization_contract_snapshot.rs scripts/generate_frontend_authorization.sh
git commit -m "feat: generate authorization contract"
git -C "$RBAC_FRONTEND_WORKTREE" add lib/auth/generated-authorization.ts __tests__/auth/generated-authorization.test.ts
git -C "$RBAC_FRONTEND_WORKTREE" commit -m "feat: consume generated authorization contract"
```

### Task 4: Add Normalized Authorization Storage and Deterministic Backfill

**Files:**
- Create: `db/migrations/20260720_authorization_kernel.sql`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Create: `crates/libs/lib-core/src/model/authorization/catalog_repo.rs`
- Create: `crates/libs/lib-core/src/model/authorization/migration_service.rs`
- Create: `crates/libs/lib-core/src/model/authorization/mod.rs`
- Modify: `crates/libs/lib-core/src/model/mod.rs`
- Modify: `crates/services/web-server/src/lib.rs`
- Create: `crates/services/web-server/tests/authz/authorization_test_support.rs`
- Create: `crates/services/web-server/tests/authz/authorization_storage.rs`
- Modify: `crates/services/web-server/tests/authz.rs`

**Interfaces:**
- Produces tables: `authorization_roles`, `authorization_grant_catalog`, `authorization_grant_role_classes`, `role_grants`, `user_role_assignments`, `organization_policy_state`, `principal_authorization_state`, `authorization_catalog_state`, and `authorization_migration_rejections`.
- Consumes fixed built-in UUIDs, grants, role classes, and one-time aliases directly from `policy_registry()`; no SQL file contains a second handwritten catalog.
- Produces `AuthorizationMigrationService::reconcile_and_backfill(&mut Transaction, &PolicyRegistry)`, invoked under a PostgreSQL advisory lock before protected routes are served.
- Test support produces `init_authorization_test_db`, `apply_authorization_migrations`, and typed SQL scalar helpers reused by Tasks 4–13.

- [ ] **Step 1: Write failing clean-bootstrap and upgraded-database tests**

Assert fixed built-ins, one assignment per membership, implemented-only grants, allowed grant/role-class pairs, mandatory revision rows, catalog hash match, deterministic alias translation, and rejection of an unknown active legacy role.

```rust
#[tokio::test]
async fn enabled_membership_has_one_normalized_role_assignment() -> Result<()> {
let mm = init_authorization_test_db().await?;
apply_authorization_migrations(&mm).await?;
let missing: i64 = scalar_i64(&mm, "SELECT count(*) FROM user_organization_memberships m LEFT JOIN user_role_assignments a USING (user_id, organization_id) WHERE m.active AND a.role_id IS NULL").await?;
	assert_eq!(missing, 0);
	Ok(())
}
```

- [ ] **Step 2: Verify storage tests fail**

Run: `cargo test -p web-server --test authz authorization_storage -- --test-threads=1`

Expected: FAIL because normalized tables are absent.

- [ ] **Step 3: Implement schema, registry reconciliation, constraints, and backfill**

Use SQL only for tables, keys, checks, and triggers. In one serializable Rust transaction, acquire the advisory lock, verify the stored catalog hash, upsert the registry's canonical grants and role-class pairs, seed fixed built-ins, translate legacy aliases declared by the registry, strip invalid non-CASE Review/Lock flags, leave reserved grants unassigned, map canonical built-in strings to fixed UUIDs, parse custom-role UUID strings, and create assignments. Produce a reconciliation row for every active legacy role comparing legacy and normalized effective access. If validation finds an unrecognized active row, roll back the main transaction, persist the rejection report in a separate restricted transaction, and refuse server readiness. Re-running the service with the same registry must be idempotent; a conflicting catalog hash must fail closed.

- [ ] **Step 4: Verify storage and bootstrap parity**

Run: `cargo test -p web-server --test authz authorization_storage -- --test-threads=1`

Expected: PASS for clean and upgraded schemas with the same built-ins, grants, and catalog hash.

- [ ] **Step 5: Commit**

```bash
git add db/migrations/20260720_authorization_kernel.sql db/bootstrap/01-safetydb-schema.sql crates/libs/lib-core/src/model/mod.rs crates/libs/lib-core/src/model/authorization crates/services/web-server/src/lib.rs crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/authorization_test_support.rs crates/services/web-server/tests/authz/authorization_storage.rs
git commit -m "feat: normalize authorization storage"
```

### Task 5: Make Authorization Revision Domains Executable

**Files:**
- Create: `db/migrations/20260720_authorization_revisions.sql`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `crates/libs/lib-core/src/model/authorization/mod.rs`
- Create: `crates/libs/lib-core/src/model/authorization/revision_repo.rs`
- Modify: `crates/libs/lib-core/src/model/mod.rs`
- Create: `crates/services/web-server/tests/authz/authorization_revisions.rs`
- Modify: `crates/services/web-server/tests/authz.rs`

**Interfaces:**
- Consumes: `AuthorizationFactDefinition` and normalized state rows.
- Produces: `RevisionRepository::load`, `RevisionRepository::lock`, trigger-verification query, and `PolicySnapshotVersion` internal revisions.

- [ ] **Step 1: Write failing revision tests**

Cover role/grant changes, shared sender/product/study definition changes, user active/access-window/scope/blind/active-sender changes, assignment/membership changes, missing state rows, and unrelated non-authorization updates.

```rust
#[tokio::test]
async fn principal_scope_change_advances_only_principal_revision() -> Result<()> {
	let before = revisions(&mm, user_id, organization_id).await?;
	update_user_sender_scope(&mm, user_id, "sender-a").await?;
	let after = revisions(&mm, user_id, organization_id).await?;
	assert_eq!(after.organization, before.organization);
	assert_eq!(after.principal, before.principal + 1);
	Ok(())
}
```

- [ ] **Step 2: Verify tests fail**

Run: `cargo test -p web-server --test authz authorization_revisions -- --test-threads=1`

Expected: FAIL because revision triggers and repository are absent.

- [ ] **Step 3: Implement fact-domain triggers and repository**

Generate trigger verification from registered `FactId` values. Organization-shared facts increment `organization_policy_state`; principal-owned facts increment `principal_authorization_state`; state rows are inserted atomically with organizations and memberships. `RevisionRepository::load` returns an error on missing rows.

- [ ] **Step 4: Verify revision tests pass**

Run: `cargo test -p web-server --test authz authorization_revisions -- --test-threads=1`

Expected: PASS with exactly one owning revision advanced for each fact mutation.

- [ ] **Step 5: Commit**

```bash
git add db/migrations/20260720_authorization_revisions.sql db/bootstrap/01-safetydb-schema.sql crates/libs/lib-core/src/model/mod.rs crates/libs/lib-core/src/model/authorization crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/authorization_revisions.rs
git commit -m "feat: version authorization fact domains"
```

### Task 6: Resolve Principals and Build One Request Snapshot

**Files:**
- Create: `crates/libs/lib-core/src/authorization/snapshot.rs`
- Create: `crates/libs/lib-core/src/model/authorization/snapshot_repo.rs`
- Create: `crates/libs/lib-core/src/model/authorization/principal_repo.rs`
- Modify: `crates/libs/lib-core/src/model/authorization/mod.rs`
- Modify: `crates/libs/lib-web/src/middleware/mw_auth.rs`
- Create: `crates/libs/lib-web/src/middleware/mw_authorization_snapshot.rs`
- Modify: `crates/libs/lib-web/src/middleware/mod.rs`
- Modify: `crates/services/web-server/src/lib.rs`
- Create: `crates/services/web-server/tests/authz/authorization_snapshot.rs`
- Modify: `crates/services/web-server/tests/authz.rs`

**Interfaces:**
- Produces: `IdentityTraits`, `PolicySnapshotVersion`, `RequestAuthorizationSnapshot`, `AuthorizationSnapshotW`, `SnapshotRepository::load_repeatable_read`, `evaluated_at`, and `authorization_valid_until`.
- Snapshot version fields: catalog hash, organization UUID, checked internal organization revision, checked internal principal revision.

- [ ] **Step 1: Write failing atomic-snapshot and identity tests**

Cover same-role/different-scope principals, fixed-UUID identity, custom admin-name spoofing, repeatable-read old-or-new behavior, missing rows, start-inclusive/end-exclusive boundaries, and no process-global snapshot reuse.

```rust
#[test]
fn custom_role_never_derives_platform_identity() {
	let facts = PrincipalFacts::custom(custom_role_id, "system_admin");
	assert!(!IdentityTraits::resolve(&facts).platform_admin());
}
```

- [ ] **Step 2: Verify snapshot tests fail**

Run: `cargo test -p web-server --test authz authorization_snapshot -- --test-threads=1`

Expected: FAIL because the snapshot repository and middleware extension are absent.

- [ ] **Step 3: Implement the immutable snapshot and repeatable-read loader**

Load membership, assignment, built-in UUID identity, role grants, principal scope facts, both revisions, and active organization in one repeatable-read transaction. Compute entitlements from the registry. Reject `now < access_start_at` and `now >= access_end_at`. Set `authorization_valid_until` to the earliest future access/token boundary.

- [ ] **Step 4: Attach the snapshot in middleware without changing decisions**

Keep legacy `CtxW` temporarily for observation comparison, but derive it from the same principal facts rather than a second database read. Add `AuthorizationSnapshotW` to request extensions and remove the standalone `PermissionProfileBmc::policy_version` middleware read.

- [ ] **Step 5: Verify snapshot tests pass**

Run: `cargo test -p web-server --test authz authorization_snapshot -- --test-threads=1`

Expected: PASS; a load error fails protected authentication and never falls back to cached permissions.

- [ ] **Step 6: Commit**

```bash
git add crates/libs/lib-core/src/authorization/snapshot.rs crates/libs/lib-core/src/model/authorization crates/libs/lib-web/src/middleware crates/services/web-server/src/lib.rs crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/authorization_snapshot.rs
git commit -m "feat: build request-local authorization snapshots"
```

### Task 7: Implement Eligibility, Contextual Reads, and Mutation Permits

**Files:**
- Create: `crates/libs/lib-core/src/authorization/context.rs`
- Create: `crates/libs/lib-core/src/authorization/decision.rs`
- Create: `crates/libs/lib-core/src/authorization/kernel.rs`
- Create: `crates/libs/lib-core/src/authorization/permit.rs`
- Modify: `crates/libs/lib-core/src/authorization/mod.rs`
- Create: `crates/libs/lib-core/tests/authorization_kernel.rs`

**Interfaces:**
- Produces:
  - `check_eligibility(ActionId, &RequestAuthorizationSnapshot) -> EligibilityDecision`
  - `authorize_subject(SubjectActionId, &RequestAuthorizationSnapshot) -> AuthorizationDecision`
  - `authorize_contextual_read(ContextActionId<C>, &RequestAuthorizationSnapshot, ContextSnapshot<'tx, C>) -> Result<AuthorizedRead<'tx, C>, Denial>`
  - `authorize_contextual_mutation(ContextActionId<C>, &RequestAuthorizationSnapshot, LockedMutationContext<'tx, C>) -> Result<AuthorizedMutation<'tx, C>, Denial>`

- [ ] **Step 1: Write compile-fail and decision tests**

Use `trybuild` as a dev dependency in `lib-core`. Prove that an Existing action cannot enter `authorize_subject`, a Case permit cannot authorize a User write, and a permit cannot escape its branded transaction closure. Add runtime tests for entitlement, identity, scope, lifecycle, parent, collection, proposal, and resource-set denials.

```rust
#[test]
fn case_review_requires_review_entitlement_and_compatible_state() {
	let denied = authorize_contextual_mutation(
		Action::CaseReviewToggle.typed(),
		&snapshot_without_review,
		locked_case_context("draft"),
	);
	assert_eq!(denied.unwrap_err().reason(), DenialReason::MissingEntitlement);
	let denied = authorize_contextual_mutation(
		Action::CaseReviewToggle.typed(),
		&snapshot_with_review,
		locked_case_context("locked"),
	);
	assert_eq!(denied.unwrap_err().reason(), DenialReason::ContextCondition);
}
```

- [ ] **Step 2: Verify tests fail**

Run: `cargo test -p lib-core --test authorization_kernel`

Expected: FAIL because kernel and permit types are absent.

- [ ] **Step 3: Implement structured decisions and transaction-branded permits**

Make permit constructors private to the kernel. Store action ID, principal ID, organization ID, target/context fingerprint, snapshot version, decision time, and an invariant transaction brand. Collection read permits carry the exact enforced scope/filter value consumed by repository queries.

- [ ] **Step 4: Verify kernel tests pass**

Run: `cargo test -p lib-core --test authorization_kernel`

Expected: PASS, including compile-fail fixtures.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/lib-core/Cargo.toml crates/libs/lib-core/src/authorization crates/libs/lib-core/tests/authorization_kernel.rs crates/libs/lib-core/tests/ui
git commit -m "feat: enforce typed authorization permits"
```

### Task 8: Replace Role-String RLS Context and Add Durable Authorization Audit

**Files:**
- Create: `db/migrations/20260720_authorization_isolation_audit.sql`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Modify: `crates/libs/lib-core/src/model/store/mod.rs`
- Create: `crates/libs/lib-core/src/model/authorization/audit_repo.rs`
- Create: `crates/libs/lib-web/src/middleware/mw_isolation_context.rs`
- Modify: `crates/libs/lib-web/src/middleware/mod.rs`
- Modify: `crates/services/web-server/src/lib.rs`
- Create: `crates/services/web-server/tests/authz/authorization_isolation.rs`
- Create: `crates/services/web-server/tests/authz/authorization_audit.rs`
- Modify: `crates/services/web-server/tests/authz.rs`

**Interfaces:**
- Produces: private `PlatformIsolationBypass`, `DatabaseIsolationContext`, `ServiceIsolationContext`, `set_isolation_context_dbx`, `AuthorizationAuditRepository::append`, and append-only `authorization_audit_events`.
- Removes at cutover: `set_org_context(UUID, VARCHAR)` and `app.current_user_role` RLS checks.

- [ ] **Step 1: Write failing isolation and audit failure tests**

Test ordinary/sponsor/custom/platform identities, spoofed names, cross-organization actions, missing context, connection reuse, internal service principal, denial after rollback, allowed mutation atomicity, read audit failure suppressing data, and denial audit failure preserving denial.

```rust
#[tokio::test]
async fn pooled_connection_does_not_retain_platform_bypass() -> Result<()> {
	read_cross_org_as_platform_admin(&mm).await?;
	let result = read_cross_org_without_context(&mm).await;
	assert!(result.is_err());
	Ok(())
}
```

- [ ] **Step 2: Verify tests fail**

Run: `cargo test -p web-server --test authz authorization_isolation -- --test-threads=1`

Run: `cargo test -p web-server --test authz authorization_audit -- --test-threads=1`

Expected: FAIL because RLS still reads role strings and the authorization audit sink is absent.

- [ ] **Step 3: Implement typed transaction-local isolation context**

Set principal UUID, organization UUID, platform isolation flag, and snapshot token through one private adapter. RLS may check only organization/ownership and the flag. Public request code cannot construct service or platform bypass types. Preserve append-only and no-update/no-delete audit constraints.

- [ ] **Step 4: Implement audit durability ordering**

Allowed mutations append authorization and business events inside the mutation transaction. Denied/stale mutations roll back first and append denial in a restricted transaction. Allowed reads append before response release; audit failure returns 500 without data. Denial audit failure emits a high-severity tracing event and still returns denial.

- [ ] **Step 5: Verify isolation and audit tests pass**

Run: `cargo test -p web-server --test authz authorization_isolation -- --test-threads=1`

Run: `cargo test -p web-server --test authz authorization_audit -- --test-threads=1`

Expected: PASS with no `app.current_user_role` dependency in new RLS definitions.

- [ ] **Step 6: Commit**

```bash
git add db/migrations/20260720_authorization_isolation_audit.sql db/bootstrap/01-safetydb-schema.sql crates/libs/lib-core/src/model/store/mod.rs crates/libs/lib-core/src/model/authorization/audit_repo.rs crates/libs/lib-web/src/middleware crates/services/web-server/src/lib.rs crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/authorization_isolation.rs crates/services/web-server/tests/authz/authorization_audit.rs
git commit -m "feat: type database isolation and authorization audit"
```

### Task 9: Expose Snapshot Versions and Profile Eligibility

**Files:**
- Modify: `crates/services/web-server/src/web/rest/user_rest/dto.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/handlers.rs`
- Modify: `crates/libs/lib-web/src/middleware/mw_res_map.rs`
- Modify: `crates/libs/lib-web/src/error.rs`
- Create: `crates/services/web-server/tests/authz/authorization_profile.rs`
- Modify: `crates/services/web-server/tests/authz.rs`

**Interfaces:**
- Produces DTOs: `PolicySnapshotVersionDto { catalogHash, organizationId, organizationRevision: String, principalRevision: String }`, `snapshotToken`, `eligibleActions`, and `authorizationValidUntil`.
- Produces header: `X-Authorization-Snapshot = base64url(canonical JSON)`.
- Produces errors: 403 `AUTHORIZATION_DENIED` and 409 `AUTHORIZATION_SNAPSHOT_STALE`.

- [ ] **Step 1: Write failing response-contract tests**

Test exact DTO fields, decimal strings above `Number.MAX_SAFE_INTEGER`, byte-identical profile/header tokens, organization mismatch, stale 409 without a false current header, and absence of raw role composition.

- [ ] **Step 2: Verify contract tests fail**

Run: `cargo test -p web-server --test authz authorization_profile -- --test-threads=1`

Expected: FAIL because the current profile returns raw permissions and numeric `policyVersion`.

- [ ] **Step 3: Implement canonical DTO/token/header and profile eligibility**

Use fixed JSON field order `catalogHash`, `organizationId`, `organizationRevision`, `principalRevision`; use `lib_utils::b64::b64u_encode`. Enumerate registered actions through `check_eligibility`; do not evaluate contextual conditions in the profile.

- [ ] **Step 4: Verify profile tests pass**

Run: `cargo test -p web-server --test authz authorization_profile -- --test-threads=1`

Expected: PASS; header and structured profile decode to identical values.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/rest/user_rest crates/libs/lib-web/src/middleware/mw_res_map.rs crates/libs/lib-web/src/error.rs crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/authorization_profile.rs
git commit -m "feat: expose authorization snapshot profile"
```

### Task 10: Register Every Protected Route with a Typed Action

**Files:**
- Create: `crates/services/web-server/src/web/authorization/mod.rs`
- Create: `crates/services/web-server/src/web/authorization/route.rs`
- Create: `crates/services/web-server/src/web/authorization/inventory.rs`
- Create: `crates/services/web-server/src/web/authorization/shadow.rs`
- Modify: `crates/services/web-server/src/web/mod.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/cases.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/misc.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/mod.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/presaves.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/submissions.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/users.rs`
- Modify: `crates/services/web-server/tests/authz/protected_route_inventory.rs`
- Create: `crates/services/web-server/tests/authz/action_binding_completeness.rs`
- Modify: `crates/services/web-server/tests/authz.rs`
- Create: `crates/services/web-server/examples/export_authorization_contract.rs`
- Modify: `scripts/generate_frontend_authorization.sh`
- Modify in frontend: `lib/auth/generated-authorization.ts`
- Create in frontend: `__tests__/auth/generated-endpoint-actions.test.ts`

**Interfaces:**
- Produces: `public_route`, `subject_route(SubjectActionId, MethodRouter)`, `context_route(ContextActionId<C>, MethodRouter)`, generated `ENDPOINT_ACTIONS`/OpenAPI/audit-name inventory, and `ShadowDecisionRecord { action, legacy, kernel, reason }`.
- Produces a canonical proof hash over the catalog hash plus the complete action-binding inventory. Re-evaluate every active migration reconciliation against observed legacy/kernel decisions, set it to `proven_equivalent` or `proven_different`, and invalidate prior proof whenever either its evidence hash or this proof hash changes.
- Context classification examples: `/cases` GET Collection(Case), `/cases` POST Proposed(CaseCreate), `/cases/{id}` Existing(Case), nested case rows Parent(Case, child), `/cases/export/xml` ResourceSet(Case), `/import/xml` Proposed(XmlImportBatch), `/audit-logs` Collection(AuditLog).

- [ ] **Step 1: Write failing completeness tests**

Assert every route from Task 1 is explicitly public or has exactly one typed action, action/path/method triples are unique, and contextual actions cannot bind through `subject_route`.

- [ ] **Step 2: Verify completeness tests fail**

Run: `cargo test -p web-server --test authz action_binding_completeness -- --test-threads=1`

Expected: FAIL because routes still use raw `.route` registration.

- [ ] **Step 3: Implement typed route builders and migrate route declarations**

Keep enforcement behavior unchanged in this task. Each contextual wrapper performs eligibility only and places the typed action binding in request extensions. Subject-only wrappers evaluate the final subject decision and compare it with the still-authoritative legacy result. Emit structured shadow records only in tests and non-production observation mode, with an explicit reviewed-difference classification; do not silently normalize a mismatch and never grant from a shadow decision.

- [ ] **Step 4: Verify route completeness**

Run: `cargo test -p web-server --test authz protected_route_inventory -- --test-threads=1`

Run: `cargo test -p web-server --test authz action_binding_completeness -- --test-threads=1`

Run: `./scripts/generate_frontend_authorization.sh "$RBAC_FRONTEND_WORKTREE"`

Run in frontend: `npm test -- --runInBand __tests__/auth/generated-endpoint-actions.test.ts`

Expected: PASS with 100% explicit route classification and no duplicate binding.

- [ ] **Step 5: Prove migration equivalence before cutover**

Require every active assignment reconciliation to carry the current catalog/action-binding proof hash. Persist `proven_equivalent` only when all relevant legacy and kernel decisions agree; persist reviewed differences as `proven_different`. Any missing route binding, changed evidence, changed proof hash, or unreviewed mismatch remains `pending_action_binding` and blocks cutover.

- [ ] **Step 6: Commit once in each repository**

```bash
git add crates/services/web-server/src/web/mod.rs crates/services/web-server/src/web/authorization crates/services/web-server/src/web/rest/routes crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/protected_route_inventory.rs crates/services/web-server/tests/authz/action_binding_completeness.rs crates/services/web-server/examples/export_authorization_contract.rs scripts/generate_frontend_authorization.sh
git commit -m "feat: bind protected routes to typed actions"
git -C "$RBAC_FRONTEND_WORKTREE" add lib/auth/generated-authorization.ts __tests__/auth/generated-endpoint-actions.test.ts
git -C "$RBAC_FRONTEND_WORKTREE" commit -m "feat: consume generated endpoint actions"
```

### Task 11: Cut Over CASE Lifecycle, Existing Resources, Parents, and Sets

**Files:**
- Create: `crates/services/web-server/src/web/authorization/case_context.rs`
- Modify: `crates/services/web-server/src/web/rest/case_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/case_export_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/cioms_export_rest/build.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/ae.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/common.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/dg.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/dh.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/direct.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/lb.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/portable_save.rs`
- Modify: `crates/services/web-server/src/web/rest/case_editor_rest/shell.rs`
- Modify: `crates/services/web-server/src/web/rest/patient_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/patient_sub_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/drug_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/drug_sub_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/reaction_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/test_result_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/narrative_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/narrative_sub_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/safety_report_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/safety_report_sub_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/case_identifiers_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/relatedness_assessment_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/drug_reaction_assessment_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/message_header_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/receiver_rest.rs`
- Modify: `crates/libs/lib-rest-core/src/utils/macro_utils.rs`
- Modify: `crates/libs/lib-core/src/model/case.rs`
- Create: `crates/services/web-server/tests/authz/contextual_case_authorization.rs`
- Create: `crates/services/web-server/tests/authz/contextual_case_races.rs`
- Create: `crates/services/web-server/tests/authz/contextual_case_test_support.rs`
- Modify: `crates/services/web-server/tests/authz.rs`

**Interfaces:**
- Consumes: `ContextActionId`, snapshot extension, case/parent context loaders, `AuthorizedRead`, and `AuthorizedMutation`.
- Produces transaction-owned CASE read, edit, review, lock, validation, delete, child-row, export, and Audit Trail decisions; CASE detail returns `allowedActions`.
- During cutover, each adapter records the legacy and kernel final outcomes before switching that action to kernel enforcement; every intentional difference must name the PDF rule that justifies it.
- Test support produces the paused-request barrier and database observation helpers used by the deterministic race tests; production code gets no test-only pause hook.

- [ ] **Step 1: Write failing CASE contextual and race tests**

Cover read vs edit, Review without Edit, Lock without Review, unlock, Audit Trail under reviewed/validated/locked state, parent/child scope, multi-case export all-or-nothing, lifecycle change between eligibility and lock, principal scope revision change, and no write on stale/deny.

```rust
#[tokio::test]
async fn review_rechecks_locked_case_state_in_write_transaction() -> Result<()> {
	let request = begin_review_request(&app, reviewer).await?;
	lock_case_as_other_request(&app, case_id).await?;
	let response = request.send().await?;
	assert_eq!(response.status(), StatusCode::CONFLICT);
	assert_eq!(case_status(&mm, case_id).await?, "locked");
	Ok(())
}
```

- [ ] **Step 2: Verify CASE tests fail**

Run: `cargo test -p web-server --test authz contextual_case_authorization -- --test-threads=1`

Run: `cargo test -p web-server --test authz contextual_case_races -- --test-threads=1`

Expected: FAIL because current handlers precheck separately from BMC transactions.

- [ ] **Step 3: Move CASE authorization into domain transactions**

Lock revision rows first, then target/parent/scope rows in stable order; compare revisions; build typed context; authorize; pass permit to repository operation; write audit; commit. Update generic subresource macros so every generated create/read/update/delete/restore path requires the corresponding parent-scoped permit.

- [ ] **Step 4: Return advisory existing-resource actions**

Compute `allowedActions` from the same read transaction and snapshot used for CASE detail. Keep Case Audit Trail independent of Review and Lock actions.

- [ ] **Step 5: Verify CASE and existing regression suites**

Run: `cargo test -p web-server --test authz contextual_case_authorization -- --test-threads=1`

Run: `cargo test -p web-server --test authz contextual_case_races -- --test-threads=1`

Run: `cargo test -p web-server --test authz rbac_cases -- --test-threads=1`

Run: `cargo test -p web-server --test authz rbac_subresources -- --test-threads=1`

Run: `cargo test -p web-server --test api case_contract_web -- --test-threads=1`

Run: `cargo test -p web-server --test api case_editor_contract_web -- --test-threads=1`

Run: `cargo test -p web-server --test api subresources_web -- --test-threads=1`

Expected: PASS with no direct CASE permission/admin predicates in production handlers.

- [ ] **Step 6: Commit**

```bash
git add crates/services/web-server/src/web/authorization/case_context.rs crates/services/web-server/src/web/rest crates/libs/lib-rest-core/src/utils/macro_utils.rs crates/libs/lib-core/src/model/case.rs crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/contextual_case_authorization.rs crates/services/web-server/tests/authz/contextual_case_races.rs crates/services/web-server/tests/authz/contextual_case_test_support.rs
git commit -m "feat: enforce contextual case authorization"
```

### Task 12: Cut Over Admin, Role, User, Info, Import, Export, Audit, and Terminology Actions

**Files:**
- Create: `crates/services/web-server/src/web/authorization/admin_context.rs`
- Create: `crates/services/web-server/src/web/authorization/transfer_context.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/dto.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/handlers.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/validation.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/views.rs`
- Modify: `crates/services/web-server/src/web/rest/permission_profile_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/organization_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/admin_settings_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/narrative.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/product.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/receiver.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/reporter.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/sender.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/shared.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/study.rs`
- Modify: `crates/services/web-server/src/web/rest/import_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/submission_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/audit_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/terminology_rest.rs`
- Create: `crates/services/web-server/tests/authz/contextual_admin_authorization.rs`
- Create: `crates/services/web-server/tests/authz/contextual_transfer_authorization.rs`
- Modify: `crates/services/web-server/tests/authz.rs`

**Interfaces:**
- Produces final Proposed/Existing/Collection/Parent/ResourceSet authorizers for users, role assignments, organizations, settings/notices, presaves, import/export/submission, audit, and terminology.
- During cutover, each adapter records the legacy and kernel final outcomes before switching that action to kernel enforcement; reviewed differences are checked in with their PDF rule and expiry task.
- `user.update.role_assignment` is separate from ordinary `user.update`; ADMIN grants do not create identity or implicit assignment authority.

- [ ] **Step 1: Write failing escalation and contextual tests**

Cover USER_CREATE without role-management, custom admin-name spoofing, cross-organization role/user updates, inactive/deleted role assignment, role self-escalation, list filtering, presave sender/product/study scope, XML import proposal destination, import history downloads, export/submission sets, and audit organization scope.

- [ ] **Step 2: Verify tests fail**

Run: `cargo test -p web-server --test authz contextual_admin_authorization -- --test-threads=1`

Run: `cargo test -p web-server --test authz contextual_transfer_authorization -- --test-threads=1`

Expected: FAIL while handlers still use `RequireAdmin`, `require_admin`, `ctx.is_admin`, or raw permissions.

- [ ] **Step 3: Implement contextual admin and transfer services**

Move validated payload/context loading and final authorization into domain transactions. Collection read permits carry mandatory organization and sender/product/study filters. Proposed XML import authorizes the destination and validated batch metadata before insertion. Resource-set export/submission rejects the whole set if any case is unauthorized.

- [ ] **Step 4: Verify targeted and existing suites**

Run: `cargo test -p web-server --test authz contextual_admin_authorization -- --test-threads=1`

Run: `cargo test -p web-server --test authz contextual_transfer_authorization -- --test-threads=1`

Run: `cargo test -p web-server --test authz rbac_users -- --test-threads=1`

Run: `cargo test -p web-server --test authz rbac_audit -- --test-threads=1`

Run: `cargo test -p web-server --test authz rbac_organizations -- --test-threads=1`

Run: `cargo test -p web-server --test api role_admin -- --test-threads=1`

Run: `cargo test -p web-server --test api scope_visibility_web -- --test-threads=1`

Run: `cargo test -p web-server --test api import_contract_web -- --test-threads=1`

Run: `cargo test -p web-server --test api import_history_web -- --test-threads=1`

Run: `cargo test -p web-server --test api submission_lifecycle_web -- --test-threads=1`

Run: `cargo test -p web-server --test api terminology_contract_web -- --test-threads=1`

Expected: PASS with no handler-level policy expressions in the migrated families.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/src/web/authorization crates/services/web-server/src/web/rest crates/services/web-server/tests/authz.rs crates/services/web-server/tests/authz/contextual_admin_authorization.rs crates/services/web-server/tests/authz/contextual_transfer_authorization.rs
git commit -m "feat: enforce contextual administration and transfer authorization"
```

### Task 13: Replace Permission Profiles with the Normalized Role Service

**Files:**
- Create: `crates/libs/lib-core/src/model/authorization/role_repo.rs`
- Create: `crates/libs/lib-core/src/model/authorization/role_service.rs`
- Modify: `crates/libs/lib-core/src/model/authorization/mod.rs`
- Create: `crates/services/web-server/src/web/rest/role_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/routes/users.rs`
- Modify: `crates/services/web-server/src/web/rest/mod.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/dto.rs`
- Create: `crates/services/web-server/tests/api/role_admin/normalized_roles_web.rs`
- Modify: `crates/services/web-server/tests/api/role_admin/mod.rs`

**Interfaces:**
- Produces canonical `/api/admin/roles` DTO fields: `id`, `displayName`, `description`, `roleClass`, `active`, `immutable`, `deletedAt`, `rowVersion`, `grants`, and server-projected PDF rows.
- Produces transactions: create, rename/update, replace grants, soft-delete, restore, and assign one role per membership.

- [ ] **Step 1: Write failing role-service tests**

Cover explicit grant replacement, reserved/unknown/alias rejection, built-in immutability, account-context visibility, 20-active-custom-role atomicity, active-assignment delete 409, restore, optimistic row-version conflict, display-name confirmation data, and one role per user/organization.

- [ ] **Step 2: Verify tests fail**

Run: `cargo test -p web-server --test api role_admin::normalized_roles_web -- --test-threads=1`

Expected: FAIL because role APIs still use `permission_profiles.privileges_json` and duplicate metadata.

- [ ] **Step 3: Implement normalized role repositories and service transactions**

Lock `organization_policy_state` before active-role counting. Validate grants through generated catalog tables. Reject built-in identity/class fields from public payloads. Return 409 with assignment count for an in-use role and return the canonical server DTO after every successful write.

Before disabling or deleting the legacy profile path, fail the cutover transaction unless every active assignment has a reconciliation row with `comparison_status = 'proven_equivalent'`, `equivalent = true`, and the current catalog/action-binding proof hash. Pending or different rows require an explicit reviewed disposition; startup reconciliation alone never treats them as equivalent.

- [ ] **Step 4: Verify role suites pass**

Run: `cargo test -p web-server --test api role_admin -- --test-threads=1`

Expected: PASS with no process-global dynamic-role refresh in role CRUD.

- [ ] **Step 5: Commit**

```bash
git add crates/libs/lib-core/src/model/authorization crates/services/web-server/src/web/rest/role_rest.rs crates/services/web-server/src/web/rest/routes/users.rs crates/services/web-server/src/web/rest/mod.rs crates/services/web-server/src/web/rest/user_rest/dto.rs crates/services/web-server/tests/api/role_admin
git commit -m "feat: serve normalized authorization roles"
```

### Task 14: Migrate the Frontend to Actions, Snapshot Identity, and Boundary Refresh

**Files in frontend repository:**
- Create: `lib/auth/action-access.ts`
- Replace: `lib/auth/policy-sync.ts`
- Create: `lib/auth/EligibleActionGate.tsx`
- Create: `lib/auth/ResourceActionGate.tsx`
- Modify: `lib/contexts/AuthContext.tsx`
- Modify: `lib/api/client.ts`
- Modify: `lib/types/api.ts`
- Modify: `lib/auth/routeAccess.ts`
- Modify: `lib/api/endpoints/admin.ts`
- Modify: `package.json`
- Create: `scripts/check-authorization-ast.mjs`
- Remove after call-site migration: `lib/auth/PermissionGate.tsx`
- Remove after call-site migration: `lib/auth/access-rules.ts`
- Remove after call-site migration: `lib/auth/admin-permissions.ts`
- Remove after call-site migration: `lib/auth/case-permissions.ts`
- Remove after call-site migration: `lib/auth/endpoint-contract.ts`
- Remove after call-site migration: `lib/auth/generated-endpoint-permissions.ts`
- Remove after call-site migration: `lib/auth/generated-permissions.ts`
- Remove after call-site migration: `lib/auth/permissions.ts`
- Remove after call-site migration: `lib/auth/roleAccess.ts`
- Modify: `__tests__/auth/policy-sync.test.ts`
- Create: `__tests__/auth/action-access.test.tsx`
- Create: `__tests__/auth/authorization-validity.test.tsx`

**Interfaces:**
- Consumes generated `ActionId` and `CATALOG_HASH`, profile `eligibleActions`, structured `policyVersion`, `snapshotToken`, `authorizationValidUntil`, and resource `allowedActions`.
- Produces `isEligibleForAction`, `canResourceAction`, `<EligibleActionGate>`, `<ResourceActionGate>`, `isPolicyRefreshing`, `isUpdateRequired`, and mutation-pause state.

- [ ] **Step 1: Rewrite tests first**

Test exact token mismatch refresh, organization mismatch reconciliation, catalog mismatch one-time cache-busted reload, persistent mismatch update-required state, deduplicated concurrent refresh, mutation pause, old-tab behavior, validity timer, focus/visibility refresh, 403 no retry, and 409 stale refresh without mutation retry.

```typescript
it("forces a document update when the backend catalog differs", async () => {
  const reload = jest.fn();
  const backendSnapshot = {
    catalogHash: "backend-hash",
    organizationId: currentSnapshot.organizationId,
    organizationRevision: currentSnapshot.organizationRevision,
    principalRevision: currentSnapshot.principalRevision,
  };
  const sync = createPolicySynchronizer({
    current: () => currentSnapshot,
    frontendCatalogHash: "frontend-hash",
    refreshProfile,
    reloadDocument: reload,
  });
  await sync.observe(backendSnapshot);
  expect(reload).toHaveBeenCalledTimes(1);
  expect(refreshProfile).not.toHaveBeenCalled();
});
```

- [ ] **Step 2: Verify frontend tests fail**

Run in frontend: `npm test -- --runInBand __tests__/auth/policy-sync.test.ts __tests__/auth/action-access.test.tsx __tests__/auth/authorization-validity.test.tsx`

Expected: FAIL because the frontend still treats policy version as an ordered number and exposes raw permissions.

- [ ] **Step 3: Implement snapshot identity and action gates**

Parse the opaque header without numeric coercion. Compare token/hash/organization strings. Store timer deadlines as RFC3339 instants. On catalog mismatch, set mutation pause, use `sessionStorage` to permit one cache-busted reload per observed hash pair, then show update-required/logout if the mismatch remains.

- [ ] **Step 4: Replace route and control authorization**

Routes/navigation use eligibility. Existing-resource controls use `allowedActions`. Proposed, collection, and set controls use eligibility only for visibility; backend denial remains authoritative. Remove `PermissionGate`, `permissions.ts`, and route role-name shortcuts after all call sites migrate. Add `check:authorization-ast`, implemented with the TypeScript compiler API, to reject production role-name predicates, raw permission-array authorization, summary booleans, and handwritten entitlement expressions while ignoring display-only role labels.

- [ ] **Step 5: Verify frontend auth tests and build**

Run in frontend: `npm test -- --runInBand __tests__/auth __tests__/rbac-contract`

Run in frontend: `npm run check:authorization-ast`

Run in frontend: `npm run build`

Expected: PASS with no TypeScript references to raw permission arrays for authorization.

- [ ] **Step 6: Commit in frontend**

```bash
git -C "$RBAC_FRONTEND_WORKTREE" add lib/auth lib/contexts/AuthContext.tsx lib/api/client.ts lib/api/endpoints/admin.ts lib/types/api.ts scripts/check-authorization-ast.mjs package.json __tests__/auth __tests__/rbac-contract
git -C "$RBAC_FRONTEND_WORKTREE" commit -m "feat: authorize frontend by generated actions"
```

### Task 15: Finish the PDF Role Editor Contract

**Files in frontend repository:**
- Modify: `app/(protected)/admin/role/hooks/useAdminRoles.ts`
- Modify: `app/(protected)/admin/role/components/AdminRolesPanel.tsx`
- Modify: `app/(protected)/admin/role/components/RoleCreateDialog.tsx`
- Modify: `app/(protected)/admin/role/model/adminRolesModel.ts`
- Modify: `app/(protected)/admin/role-privilege/hooks/useRolePrivilegeMatrix.ts`
- Modify: `app/(protected)/admin/role-privilege/components/RolePrivilegeMatrix.tsx`
- Modify: `app/(protected)/admin/role-privilege/model/rolePrivilegeModel.ts`
- Create: `__tests__/rbac-contract/role-editor-pdf-contract.test.tsx`

**Interfaces:**
- Consumes normalized role DTOs and generated PDF grant rows.
- Produces local dirty drafts, explicit Save, failure-preserved draft, canonical server replacement, read-only built-ins, strikethrough soft deletion, restore, display-name confirmation, assignment 409 handling, and UI advisory 20-role limit.

- [ ] **Step 1: Write failing PDF role-editor tests**

Cover checkbox/name edits without network calls, explicit Save, save failure preserving dirty state, successful server projection replacement, built-in non-editability, account-context visibility, display-name delete confirmation, soft-delete/restore rendering, assignment 409 message, and concurrent server-enforced limit error display.

- [ ] **Step 2: Verify tests fail**

Run in frontend: `npm test -- --runInBand __tests__/rbac-contract/role-editor-pdf-contract.test.tsx`

Expected: FAIL until all role UI state and DTO paths use the normalized contract.

- [ ] **Step 3: Implement the role editor behavior**

Maintain draft state by immutable role UUID, display the current server name in list/dialog/confirmation, never fall back to UUID in destructive copy, preserve failed drafts, and replace successful drafts with the response DTO. Render deleted roles with strikethrough and retained details; keep restore available when authorized.

- [ ] **Step 4: Verify role editor and full frontend suites**

Run in frontend: `npm test -- --runInBand __tests__/rbac-contract/role-editor-pdf-contract.test.tsx __tests__/rbac-contract`

Run in frontend: `npm run build`

Expected: PASS and the generated PDF row order is unchanged.

- [ ] **Step 5: Commit in frontend**

```bash
git -C "$RBAC_FRONTEND_WORKTREE" add app/'(protected)'/admin/role app/'(protected)'/admin/role-privilege __tests__/rbac-contract/role-editor-pdf-contract.test.tsx
git -C "$RBAC_FRONTEND_WORKTREE" commit -m "feat: complete PDF role editor behavior"
```

### Task 16: Remove Legacy Authorization and Prove Coordinated Cutover

**Files:**
- Remove: `crates/libs/lib-core/src/model/acs/dynamic_roles.rs`
- Remove: `crates/libs/lib-core/src/model/acs/menu_policy.rs`
- Remove: `crates/services/web-server/src/web/rest/permission_contract.rs`
- Remove: `crates/services/web-server/src/web/rest/permission_profile_rest.rs`
- Remove: `crates/services/web-server/examples/export_permission_contract.rs`
- Modify: `crates/libs/lib-core/src/model/acs/mod.rs`
- Modify: `crates/libs/lib-core/src/ctx/mod.rs`
- Modify: `crates/libs/lib-web/src/middleware/mw_permission.rs`
- Modify: `crates/libs/lib-rest-core/src/lib.rs`
- Modify: `crates/libs/lib-core/Cargo.toml`
- Create: `crates/libs/lib-core/tests/authorization_architecture.rs`
- Modify: `crates/services/web-server/src/web/rest/mod.rs`
- Create: `db/migrations/20260720_authorization_legacy_removal.sql`
- Modify: `db/bootstrap/01-safetydb-schema.sql`
- Create: `crates/services/web-server/tests/authz/authorization_cross_process.rs`
- Create: `crates/services/web-server/tests/authz/authorization_cutover.rs`
- Modify: `crates/services/web-server/tests/authz.rs`
- Create in frontend: `__tests__/auth/no-legacy-authorization.test.ts`
- Remove in frontend if not already removed in Task 14: `lib/auth/PermissionGate.tsx`
- Remove in frontend if not already removed in Task 14: `lib/auth/access-rules.ts`
- Remove in frontend if not already removed in Task 14: `lib/auth/admin-permissions.ts`
- Remove in frontend if not already removed in Task 14: `lib/auth/case-permissions.ts`
- Remove in frontend if not already removed in Task 14: `lib/auth/endpoint-contract.ts`
- Remove in frontend if not already removed in Task 14: `lib/auth/generated-endpoint-permissions.ts`
- Remove in frontend if not already removed in Task 14: `lib/auth/generated-permissions.ts`
- Remove in frontend if not already removed in Task 14: `lib/auth/permissions.ts`
- Remove in frontend if not already removed in Task 14: `lib/auth/roleAccess.ts`
- Remove in frontend: `__tests__/auth/generated-permissions.test.ts`
- Remove in frontend: `__tests__/auth/permissions.test.ts`
- Remove in frontend: `__tests__/rbac-contract/endpoint-manifest.test.ts`

**Interfaces:**
- Removes: `users.role`, `permission_profiles.privileges_json`, role summary booleans, duplicate API aliases, process-global dynamic roles, menu aliases, `RequireAdmin`, function admin gates, role-name `Ctx`, `Ctx::can_modify`, manual endpoint manifests, old role API calls, raw frontend permissions, and `app.current_user_role`.
- Produces: one Policy Registry, one normalized assignment source, one request snapshot, one typed kernel, generated frontend projections, a `syn`-based Rust architecture test, and a TypeScript-compiler-based frontend architecture check.

- [ ] **Step 1: Write failing final invariant tests**

Test two independently launched server processes, warm-cache role changes, exact snapshot-version/decision pairing, cross-principal isolation, missed-notification independence, catalog mismatch, clean/upgraded DB parity, no unexplained legacy/new shadow differences, and zero static legacy paths. Add `syn` and `walkdir` as `lib-core` dev dependencies; parse production Rust syntax to reject legacy gate calls, role-name decisions, raw authorization-fact reads outside approved repositories, and direct isolation-GUC writes.

- [ ] **Step 2: Verify final invariants fail while legacy code remains**

Run: `cargo test -p web-server --test authz authorization_cross_process -- --test-threads=1`

Run: `cargo test -p web-server --test authz authorization_cutover -- --test-threads=1`

Run: `./scripts/check_legacy_authorization_paths.sh --enforce-zero`

Expected: FAIL because legacy symbols, columns, and routes still exist.

- [ ] **Step 3: Remove legacy Rust, SQL, API, and frontend paths**

Make the legacy scan exit nonzero on any production occurrence. The destructive migration drops old columns/tables only after reconciliation and normalized contract checks succeed. Do not retain compatibility aliases after cutover.

- [ ] **Step 4: Regenerate contracts and prove no diff**

Run: `./scripts/generate_frontend_authorization.sh "$RBAC_FRONTEND_WORKTREE"`

Run in frontend: `git diff --exit-code -- lib/auth/generated-authorization.ts`

Expected: no generated drift in the frontend artifact; the Rust snapshot test proves the canonical backend export. Unrelated intentional source removals may still be unstaged.

- [ ] **Step 5: Run backend verification**

Run: `cargo fmt --all -- --check`

Run: `cargo test -p lib-core`

Run: `cargo test -p lib-web`

Run: `cargo test -p web-server --test authz -- --test-threads=1`

Run: `cargo test -p web-server --test api -- --test-threads=1`

Run: `./scripts/check_legacy_authorization_paths.sh --enforce-zero`

Expected: all commands exit 0 with zero test failures.

- [ ] **Step 6: Run frontend verification**

Run in frontend: `npm test -- --runInBand`

Run in frontend: `npm run check:authorization-ast`

Run in frontend: `npm run build`

Expected: all Jest suites pass and Next.js production build exits 0.

- [ ] **Step 7: Commit once in each repository**

```bash
git add crates db scripts
git commit -m "refactor: remove legacy RBAC architecture"
git -C "$RBAC_FRONTEND_WORKTREE" add app components lib scripts package.json __tests__
git -C "$RBAC_FRONTEND_WORKTREE" commit -m "refactor: remove legacy frontend authorization"
```

- [ ] **Step 8: Execute the coordinated deployment gate**

Require the backend catalog hash, migration `authorization_catalog_state.catalog_hash`, generated frontend `CATALOG_HASH`, and release manifest hash to be identical before deployment. Pause role-administration writes during final backfill/cutover. Abort deployment on migration rejections, generated diff, unexplained shadow decision difference, route incompleteness, or test failure. Preserve the pre-cutover database backup for rollback after legacy column removal.

## Final Verification Matrix

| Invariant | Proving task |
|---|---|
| PDF grant rows and reserved/absent behavior | 2, 3, 15 |
| Immutable built-in UUID identity | 2, 4, 6, 8 |
| One role per enabled membership | 4, 13 |
| No stale process authorization cache | 6, 16 |
| Atomic snapshot and compound version | 5, 6, 9, 16 |
| Typed Subject/Collection/Proposed/Existing/Parent/ResourceSet decisions | 2, 7, 10, 11, 12 |
| Read and mutation transaction permits | 7, 11, 12 |
| Review/Lock/Audit Trail independence | 11 |
| RLS isolation without role strings | 8, 16 |
| Durable allow/deny/stale audit | 8 |
| Grant role-class database enforcement | 4, 13 |
| Revision-state lifecycle and trigger completeness | 4, 5 |
| Decimal-string revisions and old-tab recovery | 9, 14 |
| Start-inclusive/end-exclusive access windows | 6, 14 |
| Explicit-save and soft-delete role UI | 13, 15 |
| Route and frontend completeness | 10, 14, 16 |
| Legacy duplicate/dead-code removal | 16 |
