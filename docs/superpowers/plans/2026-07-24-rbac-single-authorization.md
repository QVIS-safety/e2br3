# RBAC Single Authorization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the existing authorization kernel the sole allow/deny engine, isolate Case Read from unrelated permissions, prevent operational Admin grants from assigning or managing roles, and preserve the reviewed 18-row PDF contract.

**Architecture:** Reuse `authorization::kernel`, `policy_registry`, and the request `AuthorizationSnapshotW`; do not create a second policy service. Typed REST adapters resolve canonical actions and delegate to the kernel. Existing resource handlers retain a thin `require_permission` compatibility adapter, but its decision moves into the same kernel and its permissions remain generated one-way from canonical grants. Database RLS-context builders run only after authorization and contain no permission or administrator checks.

**Tech Stack:** Rust 1.88, Axum, SQLx/PostgreSQL RLS, `lib-core` authorization registry/kernel, Jest/TypeScript, Next.js 15.

## Global Constraints

- The PDF at `/Users/hyundonghoon/Downloads/QVIS Safety Database_UI Specification_18JUN2026_Updated.pdf`, especially pages 8, 13, and 95, is the visible Role & Privilege contract.
- The matrix contains exactly 18 rows: rows 1-16 implemented and rows 17-18 reserved.
- Do not add a new policy engine, permission cache, role summary, salt, or compatibility write path.
- `Ctx::is_admin()` is identity metadata only; REST handlers cannot use it to allow or deny an operation.
- Static middleware and body-aware handlers must delegate to the same kernel and cannot authorize the same operation twice.
- `USER_CREATE`, `USER_UPDATE`, or any operational permission cannot create built-in administrator identity.
- Frontend row metadata is generated from the backend registry; do not create a handwritten mirror.
- Preserve unrelated user changes and the untracked `tmp/pdfs/qvis-ui-spec.txt`.

## Repository Map

Backend root:
`/Users/hyundonghoon/projects/rust/e2br3/e2br3`

Frontend root:
`/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/merge-local-dev`

Primary backend responsibilities:

- `crates/libs/lib-core/src/authorization/registry.rs`: canonical grants,
  entitlements, actions, identity conditions, and PDF bindings.
- `crates/libs/lib-core/src/authorization/kernel.rs`: sole policy decision
  implementation.
- `crates/libs/lib-core/src/model/acs/builtin_roles.rs`: explicit legacy
  permission sets produced from canonical entitlements.
- `crates/libs/lib-core/src/model/acs/registry_adapter.rs`: one-way adapter
  from stored menu flags to registry grants and legacy permissions.
- `crates/libs/lib-rest-core/src/authorization.rs`: thin REST-facing typed
  adapter; it may resolve actions and map errors but cannot decide policy.
- `crates/services/web-server/src/web/rest/user_rest/handlers.rs`: user
  operation orchestration after action authorization.
- `crates/services/web-server/src/web/rest/permission_profile_rest.rs`:
  permission-profile orchestration after action authorization.
- `crates/services/web-server/tests/authz/authorization_legacy_gate.rs`:
  structural guard against duplicate authorization.

Primary frontend responsibilities:

- `lib/auth/generated-authorization.ts`: generated backend contract.
- `lib/admin/roleConfig.ts`: projection of generated PDF rows.
- `app/(protected)/admin/role-privilege/model/rolePrivilegeModel.ts`: payload
  sanitization and matrix projection.
- `__tests__/role-privilege-rows.test.ts`: exact 18-row contract.
- `__tests__/integration/role-privilege-effective-access.live.test.ts`:
  live roundtrip and effective-access matrix.

---

### Task 1: Isolate Case Read From the Generic Viewer Bundle

**Files:**
- Modify: `crates/libs/lib-core/src/model/acs/builtin_roles.rs`
- Modify: `crates/libs/lib-core/src/model/acs/mod.rs`
- Modify: `crates/libs/lib-core/src/model/acs/registry_adapter.rs`
- Modify: `crates/libs/lib-core/tests/rbac_dynamic_roles/case_profile.rs`
- Modify: `crates/services/web-server/tests/api/role_admin/effective_access/case_web.rs`

**Interfaces:**
- Consumes: canonical entitlement `case.read`.
- Produces: `case_view_permissions() -> &'static [Permission]`, used only by
  the `case.read` entitlement adapter.

- [ ] **Step 1: Write the failing permission-boundary test**

Extend the existing `lib_core::model::acs` import with `USER_LIST`,
`USER_READ`, `XML_EXPORT`, and `XML_EXPORT_READ`, then add:

```rust
#[test]
#[serial]
fn case_read_never_grants_transfer_user_or_mutation_permissions() {
	let _registry = RegistryGuard::new();
	let role = "case_read_boundary";
	install_profile(role, profile("case", true, false, false, false));

	assert!(has_permission(role, CASE_READ));
	for denied in [
		XML_EXPORT,
		XML_EXPORT_READ,
		USER_READ,
		USER_LIST,
		CASE_CREATE,
		CASE_UPDATE,
		CASE_DELETE,
	] {
		assert!(
			!has_permission(role, denied),
			"case.read unexpectedly granted {denied}"
		);
	}
}
```

- [ ] **Step 2: Add a source contract that prohibits the generic bundle**

Add to the same test:

```rust
#[test]
fn case_read_uses_a_dedicated_permission_set() {
	let source = include_str!(
		"../../src/model/acs/registry_adapter.rs"
	);
	assert!(
		source.contains("\"case.read\" => case_view_permissions()"),
		"case.read must compile through its dedicated set"
	);
	assert!(
		!source.contains("\"case.read\" => viewer_permissions()"),
		"case.read must not inherit a generic role bundle"
	);
}
```

Run:

```bash
cargo test -p lib-core case_read_never_grants_transfer_user_or_mutation_permissions -- --nocapture
cargo test -p lib-core case_read_uses_a_dedicated_permission_set -- --nocapture
```

Expected: the permission-boundary test passes on the current dev baseline and
the dedicated-set test FAILS because `case.read` still calls
`viewer_permissions()`.

- [ ] **Step 3: Implement the dedicated permission set**

In `builtin_roles.rs`, rename the existing Case Read bundle and keep its
contents explicit:

```rust
permission_set! {
	CASE_VIEW_PERMISSIONS,
	case_view_permissions,
	CASE_PERMISSIONS => [Read, List],
	PATIENT_PERMISSIONS => [Read, List],
	PATIENT_IDENTIFIER_PERMISSIONS => [Read, List],
	DRUG_PERMISSIONS => [Read, List],
	DRUG_SUBSTANCE_PERMISSIONS => [Read, List],
	DRUG_DOSAGE_PERMISSIONS => [Read, List],
	DRUG_INDICATION_PERMISSIONS => [Read, List],
	DRUG_DEVICE_CHARACTERISTIC_PERMISSIONS => [Read, List],
	DRUG_REACTION_ASSESSMENT_PERMISSIONS => [Read, List],
	RELATEDNESS_ASSESSMENT_PERMISSIONS => [Read, List],
	DRUG_RECURRENCE_PERMISSIONS => [Read, List],
	REACTION_PERMISSIONS => [Read, List],
	TEST_RESULT_PERMISSIONS => [Read, List],
	NARRATIVE_PERMISSIONS => [Read, List],
	SENDER_DIAGNOSIS_PERMISSIONS => [Read, List],
	CASE_SUMMARY_PERMISSIONS => [Read, List],
	MESSAGE_HEADER_PERMISSIONS => [Read, List],
	SAFETY_REPORT_PERMISSIONS => [Read, List],
	SENDER_INFORMATION_PERMISSIONS => [Read, List],
	PRIMARY_SOURCE_PERMISSIONS => [Read, List],
	LITERATURE_REFERENCE_PERMISSIONS => [Read, List],
	STUDY_INFORMATION_PERMISSIONS => [Read, List],
	STUDY_REGISTRATION_PERMISSIONS => [Read, List],
	MEDICAL_HISTORY_PERMISSIONS => [Read, List],
	PAST_DRUG_PERMISSIONS => [Read, List],
	PATIENT_DEATH_PERMISSIONS => [Read, List],
	DEATH_CAUSE_PERMISSIONS => [Read, List],
	PARENT_INFORMATION_PERMISSIONS => [Read, List],
	PARENT_MEDICAL_HISTORY_PERMISSIONS => [Read, List],
	PARENT_PAST_DRUG_PERMISSIONS => [Read, List],
	CASE_IDENTIFIER_PERMISSIONS => [Read, List],
	RECEIVER_PERMISSIONS => [Read, List],
	PRESAVE_TEMPLATE_PERMISSIONS => [Read, List],
}
```

In `model/acs/mod.rs`, replace the internal re-export:

```rust
pub(crate) use builtin_roles::{
	case_view_permissions, profile_edit_permissions,
};
```

Do not include `USER_PERMISSIONS`, `ORGANIZATION_PERMISSIONS`,
`XML_EXPORT_PERMISSIONS`, or any mutation selection.

Change `registry_adapter.rs`:

```rust
"case.read" => case_view_permissions(),
```

- [ ] **Step 4: Add real API negative checks**

Extend the Case effective-access test after creating a Case Read-only custom
user:

```rust
assert_get_status(&app, &custom_cookie, "/api/cases", StatusCode::OK).await?;
assert_get_status(
	&app,
	&custom_cookie,
	"/api/users",
	StatusCode::FORBIDDEN,
)
.await?;
assert_get_status(
	&app,
	&custom_cookie,
	"/api/exports/history",
	StatusCode::FORBIDDEN,
)
.await?;

let (status, value) = request_json(
	&app,
	"POST",
	&custom_cookie,
	"/api/cases/export/xml".to_string(),
	Some(json!({ "case_ids": [] })),
)
.await?;
assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
```

- [ ] **Step 5: Run the Case boundary tests**

Run:

```bash
cargo test -p lib-core case_read_ -- --nocapture
cargo test -p web-server test_role_privilege_matrix_update_grants_effective_case_access -- --nocapture
```

Expected: all focused tests PASS.

- [ ] **Step 6: Commit the Case Read isolation**

```bash
git add crates/libs/lib-core/src/model/acs/builtin_roles.rs \
  crates/libs/lib-core/src/model/acs/mod.rs \
  crates/libs/lib-core/src/model/acs/registry_adapter.rs \
  crates/libs/lib-core/tests/rbac_dynamic_roles/case_profile.rs \
  crates/services/web-server/tests/api/role_admin/effective_access/case_web.rs
git commit -m "fix: isolate case read permissions"
```

---

### Task 2: Make Registry Actions the Sole Identity and Entitlement Policy

**Files:**
- Modify: `crates/libs/lib-core/src/authorization/registry.rs`
- Modify: `crates/libs/lib-core/src/authorization/kernel.rs`
- Modify: `crates/libs/lib-core/src/authorization/tests.rs`
- Modify: `crates/libs/lib-core/tests/authorization_contract_snapshot.rs`

**Interfaces:**
- Consumes: existing canonical actions `user.*` and `role.*`.
- Produces: action policies where operational user actions are entitlement
  based, while role assignment and profile management are built-in
  identity-only.

- [ ] **Step 1: Write failing kernel tests for operational Admin versus identity**

Replace the existing
`identity_restrictions_do_not_treat_custom_roles_as_administrators` test with
`operational_user_actions_do_not_require_builtin_identity`, then add the
other three tests below:

```rust
#[test]
fn operational_user_actions_do_not_require_builtin_identity() {
	type Users = crate::authorization::Collection<
		crate::authorization::UserResource,
	>;
	let action = policy_registry()
		.context_action::<Users>("user.list")
		.unwrap();
	let custom = snapshot(&["user.read"], None);
	let permit = authorize_contextual_read(
		action,
		&custom,
		ContextSnapshot::new(evaluated(&custom, true)),
	);
	assert!(permit.is_ok());
}

#[test]
fn role_management_requires_builtin_identity_even_with_entitlement() {
	type Roles = crate::authorization::Collection<
		crate::authorization::RoleResource,
	>;
	let action = policy_registry()
		.context_action::<Roles>("role.list")
		.unwrap();
	let custom = snapshot(&["role.read"], None);
	let denial = authorize_contextual_read(
		action,
		&custom,
		ContextSnapshot::new(evaluated(&custom, true)),
	)
	.unwrap_err();
	assert_eq!(denial.reason(), DenialReason::IncompatibleIdentity);
}

#[test]
fn user_creation_with_role_assignment_requires_builtin_identity() {
	type NewUser = crate::authorization::Proposed<
		crate::authorization::UserCreateProposal,
	>;
	let action = policy_registry()
		.context_action::<NewUser>("user.create.role_assignment")
		.unwrap();
	let custom = snapshot(&["user.create"], None);
	let denial = authorize_contextual_mutation(
		action,
		&custom,
		LockedMutationContext::new(evaluated(&custom, true)),
	)
	.unwrap_err();
	assert_eq!(denial.reason(), DenialReason::IncompatibleIdentity);
}

#[test]
fn platform_administrator_can_target_a_selected_organization() {
	type Roles = crate::authorization::Collection<
		crate::authorization::RoleResource,
	>;
	let action = policy_registry()
		.context_action::<Roles>("role.list")
		.unwrap();
	let platform = snapshot(
		&[],
		Some(BuiltInIdentityKind::PlatformAdministrator),
	);
	let mut target = evaluated(&platform, true);
	target.organization_id = Some(Uuid::new_v4());
	let permit = authorize_contextual_read(
		action,
		&platform,
		ContextSnapshot::new(target),
	);
	assert!(permit.is_ok());
}
```

- [ ] **Step 2: Run the tests and verify the current mismatch**

Run:

```bash
cargo test -p lib-core operational_user_actions_do_not_require_builtin_identity -- --nocapture
cargo test -p lib-core role_management_requires_builtin_identity_even_with_entitlement -- --nocapture
```

Expected: the first test FAILS because current `user.list` requires an
administrator identity; the role test passes; the platform target test FAILS
on `SameOrganization`.

- [ ] **Step 3: Separate operational actions from identity-only actions**

In `canonical_actions()`:

```rust
action(
	"user.list",
	DecisionStage::ContextRequired(Collection(ResourceKind::User)),
	&["user.read"],
	&[],
	&[SameOrganization],
	PrivilegedRead,
),
action(
	"user.read",
	DecisionStage::ContextRequired(Existing(ResourceKind::User)),
	&["user.read"],
	&[],
	&[SameOrganization],
	PrivilegedRead,
),
action(
	"user.create",
	DecisionStage::ContextRequired(Proposed(ProposalKind::UserCreate)),
	&["user.create"],
	&[],
	&[SameOrganization],
	PrivilegedMutation,
),
action(
	"user.create.role_assignment",
	DecisionStage::ContextRequired(Proposed(ProposalKind::UserCreate)),
	&[],
	&administrators,
	&[SameOrganization],
	PrivilegedMutation,
),
action(
	"user.update",
	DecisionStage::ContextRequired(Existing(ResourceKind::User)),
	&["user.update"],
	&[],
	&[SameOrganization],
	PrivilegedMutation,
),
action(
	"user.delete",
	DecisionStage::ContextRequired(Existing(ResourceKind::User)),
	&["user.delete"],
	&[],
	&[SameOrganization],
	PrivilegedMutation,
),
```

Keep `user.update.role_assignment` restricted to `administrators`. Make role
actions identity-only by removing unused entitlement requirements:

```rust
action(
	"role.create",
	DecisionStage::ContextRequired(Proposed(ProposalKind::RoleCreate)),
	&[],
	&administrators,
	&[SameOrganization],
	PrivilegedMutation,
),
```

Apply the same empty entitlement list to `role.list`, `role.read`,
`role.update`, `role.delete`, and `role.restore`. Apply it to
`user.update.role_assignment` as well.

Update the kernel's `SameOrganization` evaluation so the registry-declared
Platform Administrator identity may target a selected organization:

```rust
ContextCondition::SameOrganization
	if context.organization_id != Some(snapshot.organization_id())
		&& !snapshot.identity().is_platform_administrator() =>
{
	Some(DenialReason::SameOrganizationRequired)
}
```

Sponsor Administrator and custom-role snapshots remain same-organization only.

- [ ] **Step 4: Remove phantom role entitlements from Admin grants**

Change `admin.read` entitlements to remove `role.read`. Change `admin.edit`
entitlements to remove:

```rust
"user.role_assign",
"role.manage",
"role.assign",
```

Role actions remain available to built-in administrator identities through
their action policies, not through operational Admin grants.

- [ ] **Step 5: Add contract assertions**

In `authorization_contract_snapshot.rs`:

```rust
#[test]
fn pdf_admin_grants_do_not_encode_role_identity() {
	let registry = lib_core::authorization::policy_registry();
	let admin_read = registry.grant("admin.read").unwrap();
	let admin_edit = registry.grant("admin.edit").unwrap();
	for forbidden in ["role.read", "role.manage", "role.assign", "user.role_assign"] {
		assert!(!admin_read.entitlements.contains(
			&lib_core::authorization::EntitlementId::parse(forbidden).unwrap()
		));
		assert!(!admin_edit.entitlements.contains(
			&lib_core::authorization::EntitlementId::parse(forbidden).unwrap()
		));
	}
}
```

- [ ] **Step 6: Run registry/kernel tests**

Run:

```bash
cargo test -p lib-core authorization::kernel -- --nocapture
cargo test -p lib-core --test authorization_contract_snapshot -- --nocapture
cargo test -p lib-core rbac_dynamic_roles::administration_profile -- --nocapture
```

Expected: all tests PASS. Update old expectations that asserted phantom role
entitlements by deleting those exact entitlement assertions; retain every API
endpoint-denial assertion.

- [ ] **Step 7: Commit the policy separation**

```bash
git add crates/libs/lib-core/src/authorization/registry.rs \
  crates/libs/lib-core/src/authorization/kernel.rs \
  crates/libs/lib-core/src/authorization/tests.rs \
  crates/libs/lib-core/tests/authorization_contract_snapshot.rs
git commit -m "refactor: separate admin grants from role identity"
```

---

### Task 3: Add Thin REST Adapters Around the Existing Kernel

**Files:**
- Modify: `crates/libs/lib-core/src/authorization/context.rs`
- Create: `crates/libs/lib-rest-core/src/authorization.rs`
- Modify: `crates/libs/lib-rest-core/src/lib.rs`
- Test: `crates/libs/lib-rest-core/src/authorization.rs`

**Interfaces:**
- Consumes:
  `RequestAuthorizationSnapshot`, `EvaluatedContext`, registry typed-action
  lookup, and kernel typed authorization functions.
- Produces:
  `authorize_read<C>()` and `authorize_mutation<C>()`. These are thin typed
  adapters; the kernel remains the only decision implementation.

- [ ] **Step 1: Write adapter compile-contract tests before implementation**

Create `authorization.rs` with tests that require both typed adapters to exist.
Kernel behavior remains covered by `lib-core`; this test intentionally does
not manufacture a request snapshot outside `lib-core`:

```rust
#[cfg(test)]
mod tests {
	use super::*;
	use lib_core::authorization::{
		Collection, Proposed, UserCreateProposal, UserResource,
	};

	#[test]
	fn exports_typed_kernel_adapters() {
		let _read = authorize_read::<Collection<UserResource>>;
		let _mutation =
			authorize_mutation::<Proposed<UserCreateProposal>>;
	}
}
```

- [ ] **Step 2: Run the test and verify the adapter is absent**

Run:

```bash
cargo test -p lib-rest-core exports_typed_kernel_adapters -- --nocapture
```

Expected: compilation FAILS because `authorize_read` does not exist.

- [ ] **Step 3: Expose policy-neutral evaluated facts**

In `lib-core/src/authorization/context.rs`, make `EvaluatedContext` public
while keeping every field private outside `lib-core`. Add constructors that
default every unproven condition to `false`:

```rust
#[derive(Debug, Clone)]
pub struct EvaluatedContext {
	pub(crate) organization_id: Option<Uuid>,
	pub(crate) target_fingerprint: String,
	pub(crate) within_principal_scope: bool,
	pub(crate) lifecycle_compatible: bool,
	pub(crate) parent_authorized: bool,
	pub(crate) every_target_authorized: bool,
	pub(crate) enforced_scope_filter: Option<EnforcedScopeFilter>,
}

impl EvaluatedContext {
	pub fn target(
		organization_id: Option<Uuid>,
		fingerprint: impl Into<String>,
	) -> Self {
		Self {
			organization_id,
			target_fingerprint: fingerprint.into(),
			within_principal_scope: false,
			lifecycle_compatible: false,
			parent_authorized: false,
			every_target_authorized: false,
			enforced_scope_filter: None,
		}
	}
}
```

Make `ContextSnapshot::new()` and `LockedMutationContext::new()` public. Do
not add any constructor that defaults principal-scope, lifecycle, parent, or
target-set facts to `true`. This plan exposes only the organization-target
facts required by the user and role actions.

- [ ] **Step 4: Implement denial mapping and typed delegation**

Implement:

```rust
use crate::{Error, Result};
use lib_core::authorization::{
	authorize_contextual_mutation, authorize_contextual_read, policy_registry,
	AuthorizationContext, AuthorizedMutation, AuthorizedRead, ContextSnapshot,
	EvaluatedContext, LockedMutationContext, RequestAuthorizationSnapshot,
};

fn denial(action: &str) -> Error {
	Error::PermissionDenied {
		required_permission: action.to_string(),
	}
}

pub fn authorize_read<'tx, C: AuthorizationContext>(
	snapshot: &RequestAuthorizationSnapshot,
	action_id: &str,
	evaluated: EvaluatedContext,
) -> Result<AuthorizedRead<'tx, C>> {
	let action = policy_registry()
		.context_action::<C>(action_id)
		.ok_or_else(|| denial(action_id))?;
	authorize_contextual_read(
		action,
		snapshot,
		ContextSnapshot::new(evaluated),
	)
	.map_err(|_| denial(action_id))
}

pub fn authorize_mutation<'tx, C: AuthorizationContext>(
	snapshot: &RequestAuthorizationSnapshot,
	action_id: &str,
	evaluated: EvaluatedContext,
) -> Result<AuthorizedMutation<'tx, C>> {
	let action = policy_registry()
		.context_action::<C>(action_id)
		.ok_or_else(|| denial(action_id))?;
	authorize_contextual_mutation(
		action,
		snapshot,
		LockedMutationContext::new(evaluated),
	)
	.map_err(|_| denial(action_id))
}
```

Export it from `lib.rs`:

```rust
pub mod authorization;
pub use authorization::{authorize_mutation, authorize_read};
```

- [ ] **Step 5: Add the organization-target constructor**

Keep constructors mechanical and policy-free:

```rust
pub fn same_organization_context(
	organization_id: Uuid,
	fingerprint: impl Into<String>,
) -> EvaluatedContext {
	EvaluatedContext::target(Some(organization_id), fingerprint)
}
```

This function reports only target organization and fingerprint. It must not
inspect roles, permissions, action IDs, principal scope, lifecycle, parent
authorization, or a target set.

- [ ] **Step 6: Run adapter tests and formatting**

Run:

```bash
cargo fmt --all -- --check
cargo test -p lib-rest-core authorization -- --nocapture
```

Expected: all tests PASS.

- [ ] **Step 7: Commit the thin adapters**

```bash
git add crates/libs/lib-core/src/authorization/context.rs \
  crates/libs/lib-rest-core/src/authorization.rs \
  crates/libs/lib-rest-core/src/lib.rs \
git commit -m "refactor: delegate REST authorization to kernel"
```

---

### Task 4: Convert User Administration to One Action Check

**Files:**
- Modify: `crates/services/web-server/src/web/rest/user_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/handlers.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/validation.rs`
- Modify: `crates/libs/lib-rest-core/src/lib.rs`
- Modify: `crates/services/web-server/tests/api/role_admin/effective_access/administration_web.rs`
- Modify: `crates/services/web-server/tests/authz/authorization_legacy_gate.rs`

**Interfaces:**
- Consumes: `AuthorizationSnapshotW`, `authorize_read`,
  `authorize_mutation`, and canonical user actions.
- Produces: `user_admin_rls_ctx()` with no policy logic; each user endpoint
  authorizes exactly one canonical action.

- [ ] **Step 1: Strengthen the structural test**

Replace the old call-count test with:

```rust
#[test]
fn user_handlers_have_no_parallel_authorization_gate() {
	let root = workspace_root();
	let handlers = fs::read_to_string(
		root.join("crates/services/web-server/src/web/rest/user_rest/handlers.rs"),
	)
	.unwrap();
	let rest_core = fs::read_to_string(
		root.join("crates/libs/lib-rest-core/src/lib.rs"),
	)
	.unwrap();

	assert!(!handlers.contains("ctx.is_admin()"));
	assert!(!handlers.contains("user_admin_db_ctx("));
	assert!(!rest_core.contains("pub fn require_role_admin"));
	assert!(!rest_core.contains("pub fn user_admin_db_ctx"));
	assert!(handlers.contains("authorize_read::<"));
	assert!(handlers.contains("authorize_mutation::<"));
}
```

- [ ] **Step 2: Run the structural test and verify failure**

Run:

```bash
cargo test -p web-server user_handlers_have_no_parallel_authorization_gate -- --nocapture
```

Expected: FAIL because current handlers use `user_admin_db_ctx` and direct
`ctx.is_admin()`.

- [ ] **Step 3: Replace the RLS helper with a policy-free builder**

In `lib-rest-core/src/lib.rs`, delete `require_role_admin` and
`user_admin_db_ctx`. Add:

```rust
use lib_core::ctx::ROLE_SYSTEM_ADMIN;

pub fn user_admin_rls_ctx(
	ctx: &Ctx,
	organization_id: Uuid,
) -> Result<Ctx> {
	if ctx.is_system_admin() {
		let scoped = Ctx::new(
			ctx.user_id(),
			organization_id,
			ROLE_SYSTEM_ADMIN.to_string(),
		)
		.map_err(|_| Error::AccessDenied {
			required_role: "valid organization context".to_string(),
		})?
		.with_compliance(
			ctx.change_reason().map(ToString::to_string),
			ctx.e_signature_id(),
		)
		.with_change_category(
			ctx.change_category().map(ToString::to_string),
		);
		return Ok(scoped);
	}
	Ok(ctx.clone())
}
```

This helper must not call `has_permission`, `require_permission`,
`ctx.is_admin()`, or the authorization kernel. The
`ctx.is_system_admin()` branch selects a target-organization database context;
it does not authorize the request. Sponsor and custom-role contexts preserve
the typed isolation already loaded from `AuthorizationSnapshotW`.

- [ ] **Step 4: Add the request snapshot to user handlers**

Add `AuthorizationSnapshotW` to these administrator handlers and map each to
one action:

```text
POST /api/users      -> user.create or user.create.role_assignment
GET /api/users       -> user.list
GET /api/users/{id}  -> user.read
PUT /api/users/{id}  -> user.update or user.update.role_assignment
DELETE /api/users/{id} -> user.delete
```

The self-profile, password, organization-selection, routing, and workflow
user-option handlers retain their existing behavior. Their legacy
`require_permission` call, where present, delegates to the kernel after
Task 6.

For the five administrator handlers add:

```rust
snapshot: AuthorizationSnapshotW,
```

Use these typed contexts:

```rust
type Users = Collection<UserResource>;
type ExistingUser = Existing<UserResource>;
type NewUser = Proposed<UserCreateProposal>;
```

List example:

```rust
let organization_id = ctx.organization_id();
let evaluated = same_organization_context(
	organization_id,
	"users:list",
);
authorize_read::<Users>(&snapshot, "user.list", evaluated)?;
let db_ctx = user_admin_rls_ctx(&ctx, organization_id)?;
```

Create example:

```rust
let role = normalize_user_role(data.role.clone());
let action_id = match role.as_deref() {
	None | Some(ROLE_USER) => "user.create",
	Some(_) => "user.create.role_assignment",
};
let organization_id = if ctx.is_system_admin() {
	data.organization_id.ok_or_else(|| Error::BadRequest {
		message: "organization_id is required".to_string(),
	})?
} else {
	ctx.organization_id()
};
if organization_id.is_nil() {
	return Err(Error::BadRequest {
		message: "organization context is required".to_string(),
	});
}
let evaluated = same_organization_context(
	organization_id,
	format!("user:create:{organization_id}"),
);
authorize_mutation::<NewUser>(&snapshot, action_id, evaluated)?;
let db_ctx = user_admin_rls_ctx(&ctx, organization_id)?;
```

Update chooses exactly one action:

```rust
let existing: User = UserBmc::get(&ctx, &mm, id).await?;
let action_id = if data.role.is_some() {
	"user.update.role_assignment"
} else {
	"user.update"
};
authorize_mutation::<ExistingUser>(
	&snapshot,
	action_id,
	same_organization_context(
		existing.organization_id,
		format!("user:{id}"),
	),
)?;
let db_ctx = user_admin_rls_ctx(&ctx, existing.organization_id)?;
```

For a Platform Administrator, the request `ctx` carries the snapshot-derived
platform RLS bypass and may load the target solely to evaluate its
organization. For every other identity the request `ctx` remains
tenant-scoped. Do not return target data before authorization and do not authorize both
`user.update` and `user.update.role_assignment`.

- [ ] **Step 5: Remove direct identity branches**

Delete this handler authorization branch:

```rust
if !ctx.is_admin() && data.role.is_some() {
	return Err(Error::PermissionDenied {
		required_permission: "built-in administrator role assignment"
			.to_string(),
	});
}
```

Delete `validate_create_role_selection()` from `validation.rs` and delete its
handler call. The role value is not itself trusted: the action selection above
sends every non-baseline role through `user.create.role_assignment`, which
requires a built-in administrator identity.

The action policy now denies custom roles before these existing, deny-only
domain invariants run:

```rust
validate_existing_sponsor_admin_mutation(&ctx, &existing)?;
validate_sponsor_admin_assignment_authority(&ctx, role.as_deref())?;
validate_sponsor_admin_role_for_org(
	&db_ctx,
	&mm,
	existing.organization_id,
	role.as_deref(),
).await?;
validate_single_sponsor_admin_for_org(
	&db_ctx,
	&mm,
	existing.organization_id,
	role.as_deref(),
	Some(id),
).await?;
```

These checks may reject an already-authorized request because of target-role,
organization-type, or singleton invariants. They must remain after the
kernel call and must never turn a kernel denial into success.

For `current_user_menu_privileges`, replace:

```rust
if !built_in.is_empty() || ctx.is_admin()
```

with:

```rust
if !built_in.is_empty()
```

because built-in metadata already determines whether a built-in matrix exists.

- [ ] **Step 6: Add operational Admin and escalation integration tests**

Extend `administration_web.rs`:

```rust
let (baseline_create_status, baseline_create_body) = request_json(
	&app,
	"POST",
	&custom_cookie,
	"/api/users".to_string(),
	Some(json!({
		"data": {
			"email": "rbac_admin_edit_baseline@example.com",
			"username": "rbac_admin_edit_baseline",
			"role": "user"
		}
	})),
)
.await?;
assert_eq!(
	baseline_create_status,
	StatusCode::CREATED,
	"{baseline_create_body:?}",
);

let (assigned_create_status, assigned_create_body) = request_json(
	&app,
	"POST",
	&custom_cookie,
	"/api/users".to_string(),
	Some(json!({
		"data": {
			"email": "rbac_admin_edit_assigned@example.com",
			"username": "rbac_admin_edit_assigned",
			"role": profile_id
		}
	})),
)
.await?;
assert_eq!(
	assigned_create_status,
	StatusCode::FORBIDDEN,
	"{assigned_create_body:?}",
);

assert_eq!(
	request_json(
		&app,
		"PUT",
		&custom_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({ "data": { "comments": "allowed" } })),
	)
	.await?
	.0,
	StatusCode::OK,
);

for role in [&profile_id, "sponsor_admin_cro", "system_admin"] {
	let (status, value) = request_json(
		&app,
		"PUT",
		&custom_cookie,
		format!("/api/users/{}", seed.viewer.id),
		Some(json!({ "data": { "role": role } })),
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
}
```

Add a mixed update and verify it fails atomically:

```rust
let (status, _) = request_json(
	&app,
	"PUT",
	&custom_cookie,
	format!("/api/users/{}", seed.viewer.id),
	Some(json!({
		"data": {
			"comments": "must not persist",
			"role": profile_id
		}
	})),
)
.await?;
assert_eq!(status, StatusCode::FORBIDDEN);
```

Fetch the user as the built-in admin and assert the comment did not change.

- [ ] **Step 7: Run user authorization tests**

Run:

```bash
cargo test -p web-server user_handlers_have_no_parallel_authorization_gate -- --nocapture
cargo test -p web-server test_admin_edit_cannot_manage_roles_or_assign_roles -- --nocapture
cargo test -p web-server test_admin_matrix_privileges_grant_user_operations_but_not_role_identity -- --nocapture
```

Expected: all tests PASS.

- [ ] **Step 8: Commit user-handler conversion**

```bash
git add crates/services/web-server/src/web/rest/user_rest.rs \
  crates/services/web-server/src/web/rest/user_rest/handlers.rs \
  crates/services/web-server/src/web/rest/user_rest/validation.rs \
  crates/libs/lib-rest-core/src/lib.rs \
  crates/services/web-server/tests/api/role_admin/effective_access/administration_web.rs \
  crates/services/web-server/tests/authz/authorization_legacy_gate.rs
git commit -m "refactor: authorize user administration through kernel"
```

---

### Task 5: Convert Permission-Profile Management to Identity Actions

**Files:**
- Modify: `crates/services/web-server/src/web/rest/permission_profile_rest.rs`
- Modify: `crates/services/web-server/tests/authz/rbac_users/permission_profiles_web.rs`
- Modify: `crates/services/web-server/tests/authz/authorization_legacy_gate.rs`

**Interfaces:**
- Consumes: typed role actions from Task 2 and thin adapters from Task 3.
- Produces: permission-profile endpoints with one kernel decision and
  organization-scoped RLS context.

- [ ] **Step 1: Add the structural failure**

Add:

```rust
#[test]
fn permission_profile_handlers_delegate_only_to_kernel() {
	let source = fs::read_to_string(
		workspace_root().join(
			"crates/services/web-server/src/web/rest/permission_profile_rest.rs",
		),
	)
	.unwrap();
	assert!(!source.contains("require_role_admin"));
	assert!(!source.contains("ctx.is_admin()"));
	assert!(source.contains("authorize_read::<"));
	assert!(source.contains("authorize_mutation::<"));
}
```

Run:

```bash
cargo test -p web-server permission_profile_handlers_delegate_only_to_kernel -- --nocapture
```

Expected: FAIL on current `require_role_admin`.

- [ ] **Step 2: Split target-organization validation from authorization**

Rename `permission_profile_ctx` to
`permission_profile_organization_context`. It performs only:

- system-admin target organization selection
- active organization/type validation
- scoped `Ctx` construction

Remove `require_role_admin(ctx)?`.

- [ ] **Step 3: Add one action check to every endpoint**

Add `AuthorizationSnapshotW` to each permission-profile handler.

Use:

```rust
type Roles = Collection<RoleResource>;
type ExistingRole = Existing<RoleResource>;
type NewRole = Proposed<RoleCreateProposal>;
```

Map endpoints:

```text
GET collection -> role.list / authorize_read::<Roles>
GET item       -> role.read / authorize_read::<ExistingRole>
POST           -> role.create / authorize_mutation::<NewRole>
PUT active=true -> role.restore / authorize_mutation::<ExistingRole>
other PUT      -> role.update / authorize_mutation::<ExistingRole>
DELETE         -> role.delete / authorize_mutation::<ExistingRole>
```

Example:

```rust
let scoped_ctx = permission_profile_organization_context(
	&request_ctx,
	&mm,
	&scope,
).await?;
authorize_read::<Roles>(
	&snapshot,
	"role.list",
	same_organization_context(
		scoped_ctx.organization_id(),
		"roles:list",
	),
)?;
```

The update handler selects exactly one action from the request body before
loading or mutating the profile:

```rust
let action_id = if params.data.active == Some(true) {
	"role.restore"
} else {
	"role.update"
};
authorize_mutation::<ExistingRole>(
	&snapshot,
	action_id,
	same_organization_context(
		scoped_ctx.organization_id(),
		format!("role:{id}"),
	),
)?;
```

An already-active role sent with `active=true` is harmlessly checked against
the stricter restore action; the handler never performs both decisions.

- [ ] **Step 4: Verify operational Admin remains denied**

Use the existing custom Admin Edit test and assert all profile operations are
forbidden:

```rust
for method in ["GET", "POST"] {
	let (status, value) = request_json(
		&app,
		method,
		&custom_cookie,
		"/api/admin/permission-profiles".to_string(),
		if method == "POST" {
			Some(json!({ "data": { "name": "forbidden", "privileges": [] } }))
		} else {
			None
		},
	)
	.await?;
	assert_eq!(status, StatusCode::FORBIDDEN, "{value:?}");
}
```

Retain the system-admin target-organization test and sponsor-admin
same-organization tests.

- [ ] **Step 5: Run role authorization tests**

Run:

```bash
cargo test -p web-server permission_profile_handlers_delegate_only_to_kernel -- --nocapture
cargo test -p web-server test_system_admin_permission_profiles_require_target_organization -- --nocapture
cargo test -p web-server test_permission_profiles_are_scoped_by_organization_for_sponsor_admins -- --nocapture
cargo test -p web-server test_admin_edit_cannot_manage_roles_or_assign_roles -- --nocapture
```

Expected: all tests PASS.

- [ ] **Step 6: Commit permission-profile conversion**

```bash
git add crates/services/web-server/src/web/rest/permission_profile_rest.rs \
  crates/services/web-server/tests/authz/rbac_users/permission_profiles_web.rs \
  crates/services/web-server/tests/authz/authorization_legacy_gate.rs
git commit -m "refactor: authorize role management through kernel"
```

---

### Task 6: Remove Legacy Permission Decision Implementations

**Files:**
- Modify: `crates/libs/lib-core/src/authorization/kernel.rs`
- Modify: `crates/libs/lib-rest-core/src/lib.rs`
- Delete: `crates/libs/lib-web/src/middleware/mw_permission.rs`
- Modify: `crates/libs/lib-web/src/middleware/mod.rs`
- Modify: `crates/services/web-server/src/web/rest/admin_settings_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/audit_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/case_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/user_rest/handlers.rs`
- Modify: `crates/services/web-server/src/web/rest/terminology_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/import_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/case_export_rest.rs`
- Modify: `crates/services/web-server/src/web/rest/section_presave_rest/shared.rs`
- Modify: `crates/services/web-server/tests/authz/authorization_legacy_gate.rs`

**Interfaces:**
- Consumes: legacy permissions compiled one-way from canonical registry
  entitlements.
- Produces:
  `authorization::kernel::legacy_permission_allowed(subject, permission)`,
  `authorization::kernel::any_legacy_permission_allowed(subject, permissions)`,
  `authorization::kernel::administrator_identity(ctx)`, and thin REST
  adapters. No web-layer code decides permissions or derives administrator
  status directly.

- [ ] **Step 1: Write failing kernel compatibility tests**

Add to the kernel tests:

```rust
#[test]
fn legacy_permission_compatibility_is_decided_inside_the_kernel() {
	use crate::ctx::ROLE_USER;
	use crate::model::acs::{CASE_APPROVE, CASE_READ};

	assert!(legacy_permission_allowed(ROLE_USER, CASE_READ));
	assert!(!legacy_permission_allowed(ROLE_USER, CASE_APPROVE));
	assert!(any_legacy_permission_allowed(
		ROLE_USER,
		&[CASE_APPROVE, CASE_READ],
	));
}

#[test]
fn administrator_identity_has_one_kernel_derivation() {
	use crate::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO, ROLE_USER};

	let sponsor = Ctx::new(
		Uuid::new_v4(),
		Uuid::new_v4(),
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	).unwrap();
	let user = Ctx::new(
		Uuid::new_v4(),
		Uuid::new_v4(),
		ROLE_USER.to_string(),
	).unwrap();
	assert!(administrator_identity(&sponsor));
	assert!(!administrator_identity(&user));
}
```

Run:

```bash
cargo test -p lib-core legacy_permission_compatibility_is_decided_inside_the_kernel -- --nocapture
```

Expected: compilation FAILS because the two kernel functions do not exist.

- [ ] **Step 2: Add structural guards before moving the decision**

Add:

```rust
fn rust_sources(root: &Path) -> Vec<PathBuf> {
	let mut pending = vec![root.to_path_buf()];
	let mut sources = Vec::new();
	while let Some(path) = pending.pop() {
		for entry in fs::read_dir(path).unwrap() {
			let path = entry.unwrap().path();
			if path.is_dir() {
				if path.file_name().and_then(|name| name.to_str())
					!= Some("tests")
				{
					pending.push(path);
				}
			} else if path.extension().and_then(|value| value.to_str())
				== Some("rs")
			{
				sources.push(path);
			}
		}
	}
	sources
}

#[test]
fn production_permission_checks_delegate_to_the_kernel() {
	let root = workspace_root();
	let forbidden = [
		"crates/libs/lib-rest-core/src",
		"crates/libs/lib-web/src",
		"crates/services/web-server/src",
	];
	for relative in forbidden {
		let path = root.join(relative);
		for source in rust_sources(&path) {
			let text = fs::read_to_string(&source).unwrap();
			assert!(
				!text.contains("has_permission(")
					&& !text.contains("has_any_permission(")
					&& !text.contains("has_all_permissions(")
					&& !text.contains(".is_admin()"),
				"direct authorization decision remains in {}",
				source.display(),
			);
		}
	}

	let rest_core = fs::read_to_string(
		root.join("crates/libs/lib-rest-core/src/lib.rs"),
	)
	.unwrap();
	assert!(rest_core.contains(
		"authorization::legacy_permission_allowed"
	));
	assert!(rest_core.contains(
		"authorization::any_legacy_permission_allowed"
	));
	assert!(rest_core.contains(
		"authorization::administrator_identity"
	));
}
```

Import `std::path::{Path, PathBuf}` in the test file. The collector excludes
test directories; the guard targets production sources only.

Also retain the extractor deletion guard:

```rust
#[test]
fn web_permission_adapters_do_not_decide_policy() {
	let root = workspace_root();
	assert!(
		!root.join(
			"crates/libs/lib-web/src/middleware/mw_permission.rs"
		).exists()
	);
	let modules = fs::read_to_string(
		root.join("crates/libs/lib-web/src/middleware/mod.rs"),
	)
	.unwrap();
	assert!(!modules.contains("mw_permission"));
}
```

- [ ] **Step 3: Move legacy compatibility decisions into the kernel**

Add to `authorization/kernel.rs`:

```rust
pub fn legacy_permission_allowed(
	subject: &str,
	permission: crate::model::acs::Permission,
) -> bool {
	crate::model::acs::has_permission(subject, permission)
}

pub fn any_legacy_permission_allowed(
	subject: &str,
	permissions: &[crate::model::acs::Permission],
) -> bool {
	permissions
		.iter()
		.copied()
		.any(|permission| legacy_permission_allowed(subject, permission))
}

pub fn administrator_identity(ctx: &crate::ctx::Ctx) -> bool {
	ctx.is_admin()
}
```

These are migration entry points inside the sole decision engine. They read
only legacy permissions already compiled from canonical registry grants; do
not add another permission-to-action table.

- [ ] **Step 4: Make REST permission helpers thin kernel adapters**

Replace `require_permission` and add the two non-throwing/any-of adapters:

```rust
pub fn permission_allowed(ctx: &Ctx, permission: Permission) -> bool {
	lib_core::authorization::legacy_permission_allowed(
		ctx.permission_subject(),
		permission,
	)
}

pub fn administrator_scope_bypass(ctx: &Ctx) -> bool {
	lib_core::authorization::administrator_identity(ctx)
}

pub fn require_permission(ctx: &Ctx, permission: Permission) -> Result<()> {
	if permission_allowed(ctx, permission) {
		return Ok(());
	}
	Err(Error::PermissionDenied {
		required_permission: format!("{permission}"),
	})
}

pub fn require_any_permission(
	ctx: &Ctx,
	permissions: &[Permission],
) -> Result<()> {
	if lib_core::authorization::any_legacy_permission_allowed(
		ctx.permission_subject(),
		permissions,
	) {
		return Ok(());
	}
	Err(Error::PermissionDenied {
		required_permission: permissions
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.join(" or "),
	})
}
```

These functions translate kernel results only; they cannot call
`has_permission`, inspect a role label, or special-case administrators.

- [ ] **Step 5: Remove direct web decisions and the duplicate extractor**

Replace the four production direct-check sites:

```text
admin_settings_rest.rs notice filtering -> permission_allowed
audit_rest.rs audit gate                -> require_permission
case_rest.rs write-grade any-of gate    -> require_any_permission
user_rest/handlers.rs response list     -> permission_allowed
lib-rest-core case scope bypass         -> administrator_scope_bypass
section_presave_rest/shared.rs bypass   -> administrator_scope_bypass
import_rest.rs history scope branches   -> administrator_scope_bypass
```

In `terminology_rest.rs`, `import_rest.rs`, and `case_export_rest.rs`, remove
every `RequirePermission<_>` argument and its imports. Keep exactly one
existing inline `require_permission()` call per endpoint; it now delegates to
the kernel.

Delete `mw_permission.rs`, remove this line from `middleware/mod.rs`:

```rust
pub mod mw_permission;
```

Do not delete legacy permission constants from `lib-core`; they remain the
storage/runtime compatibility target compiled from canonical entitlements.

- [ ] **Step 6: Run kernel, structural, and web tests**

Run:

```bash
cargo test -p lib-core legacy_permission_compatibility_is_decided_inside_the_kernel -- --nocapture
cargo test -p web-server --test authz authorization_legacy_gate -- --nocapture
cargo test -p lib-web -- --nocapture
cargo check -p web-server
```

Expected: structural tests PASS and the server compiles with no raw
web-layer decision implementation.

- [ ] **Step 7: Commit legacy-gate removal**

Stage the explicit converted files:

```bash
git add crates/libs/lib-core/src/authorization/kernel.rs \
  crates/libs/lib-rest-core/src/lib.rs \
  crates/libs/lib-web/src/middleware/mod.rs \
  crates/services/web-server/src/web/rest/admin_settings_rest.rs \
  crates/services/web-server/src/web/rest/audit_rest.rs \
  crates/services/web-server/src/web/rest/case_rest.rs \
  crates/services/web-server/src/web/rest/user_rest/handlers.rs \
  crates/services/web-server/src/web/rest/terminology_rest.rs \
  crates/services/web-server/src/web/rest/import_rest.rs \
  crates/services/web-server/src/web/rest/case_export_rest.rs \
  crates/services/web-server/src/web/rest/section_presave_rest/shared.rs \
  crates/services/web-server/tests/authz/authorization_legacy_gate.rs
git add -u crates/libs/lib-web/src/middleware/mw_permission.rs
git commit -m "refactor: remove duplicate permission gates"
```

---

### Task 7: Lock the PDF Matrix and Reserved E-mail Behavior

**Files:**
- Modify: `crates/libs/lib-core/src/authorization/registry.rs`
- Modify: `crates/libs/lib-core/src/model/acs/registry_adapter.rs`
- Modify: `crates/libs/lib-core/src/model/acs/tests.rs`
- Modify: `crates/libs/lib-core/src/model/authorization/migration_service.rs`
- Modify: `crates/libs/lib-core/tests/authorization_contract_snapshot.rs`
- Modify: `crates/services/web-server/tests/api/role_admin/effective_access/dashboard_web.rs`
- Modify: `crates/services/web-server/tests/api/role_admin/effective_access/persistence_web.rs`
- Regenerate: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/merge-local-dev/lib/auth/generated-authorization.ts`
- Modify: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/merge-local-dev/__tests__/role-privilege-rows.test.ts`

**Interfaces:**
- Consumes: backend PDF registry contract.
- Produces: one generated frontend matrix with exactly 18 rows.

- [ ] **Step 1: Add the exact backend row test**

Add:

```rust
#[test]
fn pdf_matrix_is_exactly_the_reviewed_eighteen_rows() {
	let mut rows = lib_core::authorization::policy_registry()
		.grants()
		.map(|grant| (
			grant.pdf_order,
			grant.pdf_menu.as_str(),
			grant.pdf_type.as_str(),
			grant.pdf_privilege.as_str(),
			grant.availability,
		))
		.collect::<Vec<_>>();
	rows.sort_by_key(|row| row.0);
	assert_eq!(rows.len(), 18);
	assert_eq!(rows[0].1, "HOME");
	assert_eq!(rows[0].2, "Notice");
	assert_eq!(rows[15].1, "ADMIN");
	assert_eq!(rows[16].1, "E-mail");
	assert_eq!(rows[17].1, "E-mail");
	assert!(rows[..16].iter().all(
		|row| row.4 == Availability::Implemented
	));
	assert!(rows[16..].iter().all(
		|row| row.4 == Availability::Reserved
	));
}
```

- [ ] **Step 2: Reject aliases on current writes while preserving migration**

In `normalize_menu_privileges()`, remove the `has_alias` acceptance and the
`legacy_alias()` fallback. Current input must match a direct generated
`grant.ui_binding.menu_key`; otherwise return `UnknownMenu`.

Replace the current alias-normalization unit assertion with:

```rust
#[test]
fn current_menu_writes_reject_legacy_aliases() {
	let error = normalize_menu_privileges(&[AdminMenuPrivilege {
		menu_key: "users".to_string(),
		can_read: true,
		can_edit: false,
		can_review: false,
		can_lock: false,
	}])
	.unwrap_err();
	assert_eq!(
		error,
		PrivilegeAdapterError::UnknownMenu {
			menu_key: "users".to_string(),
		},
	);
}
```

Add a migration-service test proving the one-way translator still accepts a
known historical alias and returns only its canonical grant:

```rust
#[test]
fn legacy_aliases_exist_only_in_the_migration_translator() {
	let grants = grants_for_legacy_privileges(
		crate::authorization::policy_registry(),
		&[AdminMenuPrivilege {
			menu_key: "export".to_string(),
			can_read: true,
			can_edit: false,
			can_review: false,
			can_lock: false,
		}],
	)
	.unwrap();
	assert_eq!(
		grants,
		BTreeSet::from(["submission.history.read".to_string()]),
	);
}
```

Extend `persistence_web.rs` with a direct current-write rejection:

```rust
let (status, value) = request_json(
	&app,
	"PUT",
	&admin_cookie,
	format!("/api/admin/permission-profiles/{profile_id}"),
	Some(json!({
		"data": {
			"privileges": [{
				"menu_key": "export",
				"can_read": true,
				"can_edit": false,
				"can_review": false,
				"can_lock": false
			}]
		}
	})),
)
.await?;
assert_eq!(status, StatusCode::BAD_REQUEST, "{value:?}");
```

- [ ] **Step 3: Add reserved-row normalization and execution tests**

Rename the existing test to
`test_report_due_mail_rows_are_removed_and_grant_nothing_while_reserved`.
Assert that a direct payload containing only:

```json
{
  "menu_key": "email_report_due",
  "can_read": true,
  "can_edit": true,
  "can_review": false,
  "can_lock": false
}
```

returns a profile with no operational E-mail grant and:

```rust
let (status, profiles) = request_json(
	&app,
	"GET",
	&admin_cookie,
	"/api/admin/permission-profiles".to_string(),
	None,
)
.await?;
assert_eq!(status, StatusCode::OK, "{profiles:?}");
let saved_profile = profiles
	.as_array()
	.and_then(|rows| rows.iter().find(|row| row["id"] == profile_id))
	.ok_or("saved profile not found")?;
assert!(!saved_profile["privileges"]
	.as_array()
	.ok_or("privileges must be an array")?
	.iter()
	.any(|row| row["menu_key"] == "email_report_due"));
assert!(!has_permission(&profile_id, EMAIL_NOTIFICATION_SEND));
```

The backend must normalize the reserved row away; the generated grant set and
effective permissions must both be empty.

- [ ] **Step 4: Verify Notice and Admin semantics**

Retain and strengthen:

```rust
assert!(has_permission(&notice_role, DASHBOARD_NOTICE_READ));
assert!(has_permission(&notice_role, DASHBOARD_NOTICE_UPDATE));
assert!(!has_permission(&notice_role, SETTINGS_UPDATE));
```

For Admin Edit:

```rust
assert!(has_permission(&admin_role, USER_CREATE));
assert!(has_permission(&admin_role, USER_UPDATE));
```

Role-management denial is verified through the actual permission-profile API
in Task 5; no derived `can_manage_roles` helper or response field is added.

- [ ] **Step 5: Regenerate frontend authorization**

From the backend root:

```bash
./scripts/generate_frontend_authorization.sh \
  /Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/merge-local-dev
```

Expected: `lib/auth/generated-authorization.ts` contains 18
`PDF_ROLE_PRIVILEGE_ROWS` entries, with the final two marked `reserved`.

- [ ] **Step 6: Add exact frontend assertions**

In `role-privilege-rows.test.ts`:

```typescript
expect(ROLE_PRIVILEGE_ROWS).toHaveLength(18);
expect(
  ROLE_PRIVILEGE_ROWS.slice(0, 16).every((row) => !row.disabled),
).toBe(true);
expect(
  ROLE_PRIVILEGE_ROWS.slice(16).map((row) => ({
    menu: row.menu,
    type: row.type,
    privilege: row.privilege,
    disabled: row.disabled,
  })),
).toEqual([
  {
    menu: "E-mail",
    type: "Report Due Mail",
    privilege: "Read",
    disabled: true,
  },
  {
    menu: "E-mail",
    type: "Report Due Mail",
    privilege: "Send",
    disabled: true,
  },
]);
expect(ROLE_PRIVILEGE_ROWS.some((row) => row.menuKey === "roles")).toBe(false);
```

- [ ] **Step 7: Run backend and frontend contract tests**

Backend:

```bash
cargo test -p lib-core --test authorization_contract_snapshot -- --nocapture
cargo test -p lib-core current_menu_writes_reject_legacy_aliases -- --nocapture
cargo test -p lib-core legacy_aliases_exist_only_in_the_migration_translator -- --nocapture
cargo test -p web-server test_home_notice_matrix_privileges_surface_in_current_user_capabilities -- --nocapture
cargo test -p web-server test_report_due_mail_rows_are_removed_and_grant_nothing_while_reserved -- --nocapture
```

Frontend:

```bash
npm test -- --runInBand __tests__/role-privilege-rows.test.ts \
  __tests__/admin-role-contract.test.ts
npx tsc --noEmit
```

Expected: all tests PASS.

- [ ] **Step 8: Commit backend contract changes**

From the backend root:

```bash
git add crates/libs/lib-core/src/authorization/registry.rs \
  crates/libs/lib-core/src/model/acs/registry_adapter.rs \
  crates/libs/lib-core/src/model/acs/tests.rs \
  crates/libs/lib-core/src/model/authorization/migration_service.rs \
  crates/libs/lib-core/tests/authorization_contract_snapshot.rs \
  crates/services/web-server/tests/api/role_admin/effective_access/dashboard_web.rs \
  crates/services/web-server/tests/api/role_admin/effective_access/persistence_web.rs
git commit -m "test: lock reviewed RBAC matrix contract"
```

- [ ] **Step 9: Commit generated frontend contract**

From the frontend root:

```bash
git add lib/auth/generated-authorization.ts \
  __tests__/role-privilege-rows.test.ts
git commit -m "test: lock generated RBAC matrix contract"
```

---

### Task 8: Run Full Roundtrip, Browser E2E, and Restart Verification

**Files:**
- Modify only if a test exposes a defect:
  `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend/.worktrees/merge-local-dev/__tests__/integration/role-privilege-effective-access.live.test.ts`
- No production file changes are expected in this task.

**Interfaces:**
- Consumes: completed backend and frontend builds from Tasks 1-7.
- Produces: verified dev-ready commits and cleaned E2E fixtures.

- [ ] **Step 1: Run formatting and focused backend suites**

```bash
cargo fmt --all -- --check
cargo test -p lib-core authorization -- --nocapture
cargo test -p lib-core rbac_dynamic_roles -- --nocapture
cargo test -p web-server --test authz authorization_legacy_gate -- --nocapture
cargo test -p web-server role_admin -- --nocapture
```

Expected: all commands exit `0`.

- [ ] **Step 2: Run frontend unit and type checks**

```bash
npm test -- --runInBand \
  __tests__/role-privilege-rows.test.ts \
  __tests__/admin-role-contract.test.ts \
  __tests__/rbac-contract/admin-actions.test.tsx \
  __tests__/rbac-contract/endpoint-manifest.test.ts
npx tsc --noEmit
```

Expected: all tests and type checking PASS.

- [ ] **Step 3: Start isolated backend and frontend**

Backend:

```bash
SERVICE_BIND_ADDR=127.0.0.1:8082 \
RUST_LOG=web_server=info,lib_core=info \
cargo run -p web-server
```

Frontend:

```bash
API_PROXY_TARGET=http://127.0.0.1:8082 \
npx next dev -p 3002
```

Expected:

```text
authorization storage reconciled
LISTENING - Ok(127.0.0.1:8082)
```

No `continuing with the active legacy RBAC runtime` warning is allowed.

- [ ] **Step 4: Run the live RBAC matrix**

From the frontend root:

```bash
RBAC_TEST_BASE_URL=http://127.0.0.1:8082 \
RBAC_TEST_ADMIN_EMAIL=hdh4063@gmail.com \
RBAC_TEST_ADMIN_PASSWORD=welcome \
npm run test:rbac-integration
```

Expected: every PDF row contract passes with zero skipped checks.

- [ ] **Step 5: Run browser verification**

Using the in-app browser against `http://127.0.0.1:3002`:

1. Sign in as the system administrator.
2. Select Demo CRO Organization.
3. Open Role & Privilege.
4. Confirm exactly 18 rows.
5. Confirm only active roles appear as editable columns.
6. Confirm E-mail Read/Send are disabled.
7. Create a Case Read-only role and user.
8. Confirm Case pages load while Users and Export/Submission operations are
   denied.
9. Create an Admin Edit role and user.
10. Confirm general user-field update succeeds.
11. Confirm role assignment and Permission Profile CRUD fail.

- [ ] **Step 6: Clean E2E fixtures**

Deactivate every temporary user and role created by the live suite or browser
run using their exact IDs. Verify:

```sql
SELECT email, active
FROM users
WHERE email LIKE 'rbac_%@example.com'
ORDER BY email;
```

Only historical inactive E2E users may remain.

- [ ] **Step 7: Restart and verify normalized storage**

Stop only the isolated port `8082` process, restart the same backend command,
and verify:

```text
authorization storage reconciled
```

The log must not contain:

```text
authorization normalization rejected
```

- [ ] **Step 8: Review diffs and commit any test-only correction**

If the live test required a correction:

```bash
git add __tests__/integration/role-privilege-effective-access.live.test.ts
git commit -m "test: cover single-engine RBAC roundtrip"
```

If no file changed, do not create an empty commit.

- [ ] **Step 9: Confirm repository state before integration**

Backend:

```bash
git status --short
git log --oneline origin/dev..HEAD
```

Frontend:

```bash
git status --short
git log --oneline origin/dev..HEAD
```

Expected: only the known untracked PDF extraction remains outside committed
work, and every implementation commit is listed above `origin/dev`.

---

## Final Completion Gate

Do not merge or push until all of the following are true:

- Case Read API tests prove user and export access are denied.
- Operational Admin roles can update allowed user fields but cannot assign or
  manage roles.
- `authorization::kernel` is the only allow/deny implementation.
- Thin adapters contain no `has_permission`, administrator special case, or
  independent organization decision.
- User and permission-profile handlers authorize each operation exactly once.
- The generated frontend contract contains exactly 18 PDF rows.
- Reserved E-mail rows are disabled and grant no runtime permission.
- Backend focused tests, frontend focused tests, TypeScript, live RBAC
  roundtrip, browser E2E, and clean restart all pass.
- Temporary E2E fixtures are inactive or removed.
