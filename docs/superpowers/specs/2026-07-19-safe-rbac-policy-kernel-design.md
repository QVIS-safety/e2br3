# Safe RBAC Policy Kernel Design

**Date:** 2026-07-19

**Last reviewed:** 2026-07-20

**Status:** Revised after systematic debugging and code review; pending final user approval
**Supersedes:** The architectural portions of `2026-07-14-rbac-architecture-design.md` and `2026-07-15-unified-rbac-contract-design.md`. The implemented PDF behavior in `2026-07-18-pdf-rbac-compliance-design.md` remains the product contract until migrated to this model.

## Source of Truth and Decision Order

The product-level source of truth is `QVIS Safety Database_UI Specification_18JUN2026_Updated.pdf`, especially the Role & Privilege and lifecycle annotations on PDF pages 7, 8, 41, 94, and 95. When code, stored profiles, generated artifacts, tests, or older design documents disagree with the PDF, the PDF wins.

The runtime source of truth is the backend Policy Registry defined by this design. The registry encodes the approved PDF behavior as executable grant and action definitions. The database stores role assignments and selected canonical grants; it does not redefine what a grant means. Frontend authorization artifacts are generated projections of the registry and do not contain independently maintained permission logic.

The resulting authority order is:

1. the PDF defines intended product behavior;
2. the reviewed Policy Registry encodes that behavior;
3. the database stores which canonical grants and roles are assigned;
4. one request-scoped Policy Kernel evaluates the registry and stored assignments;
5. generated frontend artifacts render the result but never redefine it.

## Goal

Replace the current collection of menu flags, role-name predicates, summary booleans, process-local permission caches, manually maintained endpoint contracts, and frontend permission combinations with one executable authorization model.

For a given principal and authorization snapshot version:

- every backend instance must make the same subject-level authorization decision;
- the current-user profile and backend route guards must use the exact same snapshot;
- every protected endpoint must be bound to exactly one registered action;
- every frontend authorization decision must reference a registered action rather than reconstructing its permission requirements;
- unknown, reserved, aliased, inactive, or deleted grants must fail closed;
- the PDF Role & Privilege rows must be generated from the same grant definitions that produce effective access.

This is an architectural replacement, not a continuation of individual RBAC patches.

## Confirmed Root Causes

### Policy representations are duplicated

The current system separately maintains PDF-facing frontend rows, `privileges_json`, backend normalization, `MENU_POLICIES`, granular handler checks, a partial endpoint manifest, frontend route rules, frontend action rules, role metadata summaries, and selected role-name checks. The generated permission catalog unifies identifiers only; it does not make these policy decisions executable from one declaration.

At the time of this design, the backend has roughly 236 REST route declarations and 361 direct `require_permission` calls, while `ENDPOINT_PERMISSION_CONTRACTS` contains only 20 entries. A contract test can pass while an unregistered endpoint or UI control remains inconsistent.

### Policy version and permission state are not atomic

Dynamic role permissions are stored in an unversioned process-global `OnceLock<RwLock<HashMap<...>>>`. Role mutation refreshes only the process that handled the mutation, while every process reads the latest database policy version. One process can therefore return stale permissions paired with the latest version and can authorize requests with that stale permission set indefinitely.

The existing policy-version tests prove only that a counter increases. They do not prove that a version identifies one immutable permission snapshot or that two server processes converge before reporting that version.

### Identity, assignment, and authorization are conflated

`users.role` stores either a built-in identity label or a custom role UUID in one string. `ctx.is_admin()`, `USER_CREATE`, menu-derived `can_admin`, `sponsor_admin_capable`, and frontend `isAdminRole` checks assign different meanings to the word “admin.” This allows metadata and UI decisions to disagree with endpoint enforcement even when privilege escalation is blocked at a specific handler.

### The database contains a second business-authorization engine

Audit-log RLS directly inspects built-in role strings and `privileges_json`. It does not evaluate the backend permission catalog. The REST layer and PostgreSQL can therefore evaluate different policy representations for the same request.

### Tests validate copies rather than invariants

Most current tests verify a local mapping, response shape, or manually inventoried contract. They do not require every route to be registered, do not execute two independent server processes, and do not prove that the frontend action, profile snapshot, handler guard, and database behavior came from one policy definition.

## Chosen Approach

Implement an internal, typed Policy Kernel in Rust. Do not continue extending the current JSON menu-policy adapter, and do not introduce an external policy service such as OPA or Cedar during this migration.

The internal kernel is preferred because:

- the policy is currently bounded and closely tied to typed Rust domain state;
- lifecycle, organization, sender, product, study, and blind-data constraints already live in the application;
- an external engine would add deployment and policy-language complexity before the policy boundaries are clean;
- the registry can later be exported to or replaced by an external engine without changing callers, because all callers depend on the typed eligibility and global/resource authorization interfaces defined below.

## Canonical Model

### Identifiers

The model has three distinct stable identifiers:

1. **GrantId** — a configurable Role & Privilege unit derived from a PDF row, such as `case.read`, `case.review`, or `submission.execute`.
2. **EntitlementId** — an atomic subject capability, replacing the current use of free-standing `Permission(Resource, Action)` values as the frontend contract. Existing permission constants are migrated to typed entitlement identifiers.
3. **ActionId** — a protected product operation, such as `case.review.toggle`, `role_profile.update`, or `audit_log.list`.

A grant expands to entitlements. An action declares the entitlements and conditions required to perform it. Roles store grants, not menu booleans and not actions.

These layers are intentionally distinct:

- grants are stable administrator-facing choices;
- entitlements allow multiple grants or built-in roles to share atomic capabilities;
- actions bind authorization directly to real operations.

All three are declared in one backend Policy Registry and validated as one graph at startup and in CI.

### GrantDefinition

Each PDF-facing grant definition contains:

```text
GrantDefinition {
    id: GrantId,
    pdf_menu,
    pdf_type,
    pdf_privilege,
    availability: Implemented | Reserved,
    implied_grants,
    entitlements,
    assignable_role_classes,
}
```

Rules:

- each visible PDF matrix row maps to exactly one `GrantId`;
- `Edit` may imply `Read` only when the PDF contract explicitly says so;
- CASE Review and Lock are separate grants and separate entitlements;
- a reserved grant is visible only when required by the PDF, is disabled, and cannot be persisted or assigned;
- an unknown grant is rejected;
- aliases are accepted only by the one-time migration translator and are never accepted by the steady-state API;
- no grant definition may reference an unregistered entitlement.

The generated PDF projection explicitly preserves these reviewed rows and semantics:

- HOME / Notice / Read and Edit;
- HOME / Workflow / Read;
- CASE / Case / Read and Edit;
- CASE / Workflow / Read;
- CASE / QC / Edit;
- CASE / Lock / Edit;
- INFO / Case Info / Read and Edit;
- IMPORT / Import Files / Edit;
- IMPORT / Import History / Read, including downloads and in-page history interactions;
- EXPORT/SUBMISSION / Export/Submit / Edit;
- EXPORT/SUBMISSION / Export/Submit History / Read, including downloads and in-page history interactions;
- ADMIN / Admin / Read and Edit as retained by PDF page 8;
- E-mail / Report Due Mail / Read;
- E-mail / Report Due Mail / Send.

The PDF retains both Report Due Mail rows but the scheduled-mail consuming feature is absent. Both rows therefore remain visible with `Reserved` availability, disabled controls, and no persisted assignment until real read/send actions exist. Settings / Read is not reintroduced. This representation is honest about implementation state without deleting a PDF-required row.

ADMIN / Admin / Read expands only to registered administration list/read entitlements. ADMIN / Admin / Edit expands only to explicitly registered administration mutation entitlements and their required read prerequisites. Neither grant creates a platform/sponsor identity trait, bypasses organization scope, or implicitly permits role assignment. Role-profile and role-assignment actions remain separately declared policies.

### ActionPolicy

Each protected operation contains:

```text
ActionPolicy {
    id: ActionId,
    decision_stage: SubjectOnly | ContextRequired(ContextKind),
    entitlement_rule: AllOf | AnyOf,
    identity_conditions,
    scope_conditions,
    context_conditions,
    audit_classification,
}
```

`ContextKind` is a closed typed set:

```text
ContextKind {
    Collection(ResourceKind),
    Proposed(ProposalKind),
    Existing(ResourceKind),
    Parent { parent: ResourceKind, child: ResourceKind },
    ResourceSet(ResourceKind),
}
```

The distinctions are authorization-relevant:

- `SubjectOnly` has no target-dependent fact and can be decided from the request snapshot alone;
- `Collection` covers list/search/history operations whose organization, filter, and scope projection affect the rows that may be returned;
- `Proposed` covers typed single or batch create/import proposals whose validated payload and destination scope must be checked before insertion;
- `Existing` covers one already stored target, including target-derived downloads;
- `Parent` covers a child operation authorized through a case or other owning record;
- `ResourceSet` covers export, submission, and other multi-target operations. Mutations lock every target in a stable order; reads use one consistent database snapshot.

Adding a new context kind requires a Policy Kernel type and route/domain binding in the same registry change. It may not be simulated with a handler-specific boolean or downgraded to `SubjectOnly`.

Examples:

- `case.review.toggle` is `Existing(Case)` and requires the CASE review entitlement and a review-compatible lifecycle state;
- `case.lock.toggle` is `Existing(Case)` and requires the CASE lock entitlement and a lock-compatible lifecycle state;
- `role_profile.update` is `Existing(Role)` and requires the role-management entitlement, an allowed administrator identity class, and organization scope over the target role;
- `user.update.role_assignment` is `Existing(User)` and is distinct from ordinary `user.update` with a stronger action policy;
- `audit_log.list` is `Collection(AuditLog)` and requires the audit-read entitlement and organization scope;
- `case.create` is `Proposed(CaseCreate)`, while multi-case XML export is `ResourceSet(Case)`.

Handlers never call `has_permission` or `ctx.is_admin` directly. Subject-only actions use the subject authorization path. Context-required actions require a typed context matching their declared `ContextKind` and cannot be registered as or silently downgraded to subject-only actions.

### Identity and scope

Identity facts are separate from entitlements:

```text
IdentityTraits {
    platform_admin,
    sponsor_admin_cro,
    sponsor_admin_company,
    operational_user,
}
```

Identity traits may constrain an action but do not by themselves act as generic “admin access.” Privileged traits are derived only by matching the assigned role UUID against the immutable built-in role UUID map compiled into the reviewed Policy Registry. Display names, `stable_key`, mutable database `kind`, grants, and user-supplied strings never produce a privileged identity trait. A custom role can never produce `platform_admin` or sponsor-administrator identity, even if its name, metadata, or grants resemble a built-in role.

The principal resolver is the only component that performs the UUID-to-typed-built-in-role lookup. Legacy role strings may be translated only by the one-time migration; steady-state runtime code does not compare role names. Public role create/update APIs cannot set built-in UUIDs, `stable_key`, role class, immutability, or identity traits. Database constraints and triggers reject a non-registry role carrying a privileged built-in identity class and reject mutation of the identity class or UUID of a registry built-in role.

Organization, sender, product, study, active-sender, access-window, and blind-data rules are typed scope conditions. They are not encoded as permissions or role names. This preserves the existing business scoping model while separating it from RBAC.

## Storage Model

Introduce normalized authorization tables:

```text
authorization_roles
    id UUID primary key
    organization_id UUID nullable for platform-defined roles
    stable_key TEXT unique where applicable
    display_name TEXT
    description TEXT
    role_class TEXT
    active BOOLEAN
    immutable BOOLEAN
    deleted_at TIMESTAMPTZ nullable
    row_version BIGINT

role_grants
    role_id UUID references authorization_roles
    grant_id TEXT references authorization_grant_catalog
    primary key (role_id, grant_id)

authorization_grant_catalog
    grant_id TEXT primary key
    availability TEXT
    catalog_hash TEXT

authorization_grant_role_classes
    grant_id TEXT references authorization_grant_catalog
    role_class TEXT
    catalog_hash TEXT
    primary key (grant_id, role_class)

user_role_assignments
    user_id UUID
    organization_id UUID
    role_id UUID references authorization_roles
    active BOOLEAN
    row_version BIGINT
    primary key (user_id, organization_id)

organization_policy_state
    organization_id UUID primary key references organizations
    revision BIGINT not null default 1
    updated_at TIMESTAMPTZ

principal_authorization_state
    user_id UUID
    organization_id UUID
    revision BIGINT not null default 1
    updated_at TIMESTAMPTZ
    primary key (user_id, organization_id)
    foreign key (user_id, organization_id) references user_organization_memberships

authorization_catalog_state
    singleton BOOLEAN primary key
    catalog_hash TEXT
    applied_at TIMESTAMPTZ
```

Built-in roles use fixed UUIDs declared in the Policy Registry and are represented in the same role and grant tables as custom roles. `role_class` and grant availability use constrained database values rather than free text. Role class exists only for grant-assignability and administration validation; it never derives an identity trait. Their definitions, `authorization_grant_catalog`, and `authorization_grant_role_classes` rows are seeded and reconciled by versioned database migrations generated from the reviewed Policy Registry. They are immutable through public role-administration APIs. `role_grants.grant_id` has a foreign key to the generated catalog projection. Database constraint triggers reject a `role_grants` insert or update unless the referenced catalog row has `Implemented` availability and a generated `(grant_id, role_class)` assignment row exists. Assignment constraints additionally require the target role to be a platform built-in or belong to the same organization as the assignment.

There is at most one persisted role-assignment row per `(user_id, organization_id)`, and every enabled membership has exactly one active role assignment. PDF pages 12 and 97 permit multiple roles to be selected for a workflow-status rule; they do not define multiple simultaneous RBAC roles for one user. Workflow configuration therefore stores its own many-role rule and does not change `user_role_assignments` cardinality.

An organization-policy row is created in the same transaction as every organization and is protected by a foreign key. A principal-policy row is created in the same transaction as every user-organization membership and is removed with that membership. Backfill creates and verifies both state-row classes before normalized authorization becomes readable. Snapshot construction fails closed on a missing row; it never substitutes revision zero.

The database remains the assignment-state authority. The Rust registry remains the semantic authority for what each canonical grant and action means. Startup fails if stored non-reserved `grant_id` values are absent from the deployed registry or if the deployed catalog hash does not match the migration metadata.

The following legacy fields are removed after cutover:

- `users.role`;
- `permission_profiles.privileges_json`;
- `can_view`, `can_review`, `can_lock`, and `can_admin`;
- `built_in`/`is_builtin`, `editable`/`is_editable`, and equivalent duplicate response aliases;
- `sponsor_admin_capable` and derived `is_sponsor_admin` authorization summaries.

Role metadata is produced by one role projection service from `authorization_roles`, typed identity traits, and the Policy Registry. Metadata never independently derives authorization.

## Policy Revision and Multi-Instance Consistency

Every mutable authorization fact belongs to exactly one invalidation domain:

- catalog-wide built-in policy changes replace the catalog hash through a matching migration;
- organization-shared facts, including a custom role or a scope definition that affects multiple principals, increment the organization revision;
- principal-owned facts, including active state, role assignment, membership, assigned sender/product/study scope, blind-data access, active sender, and access window, increment that principal’s revision.

Database triggers update the owning revision in the same transaction. The Policy Registry contains `AuthorizationFactDefinition` declarations identifying each table/column source and its invalidation domain. Those declarations generate the trigger-verification inventory, and Snapshot Repository loaders must reference a registered `FactId`; raw authorization-fact reads outside those loaders fail static checks. Migration tests fail when a registered fact lacks its trigger. This is invalidation topology, not a second permission mapping. A shared mutation increments one organization revision rather than attempting an error-prone fan-out to every principal.

Built-in policy changes change the Policy Registry catalog hash and require a matching database migration. The effective snapshot version is:

```text
PolicySnapshotVersion {
    catalog_hash,
    organization_id,
    organization_revision,
    principal_revision,
}
```

The public version DTO is exactly:

```text
PolicySnapshotVersionDto {
    catalogHash,
    organizationId,
    organizationRevision: DecimalString,
    principalRevision: DecimalString,
}
```

Internal revisions remain checked 64-bit integers. Public revisions are canonical unsigned decimal strings with no sign or leading zero, preventing JavaScript precision loss. The backend also emits `X-Authorization-Snapshot`, an opaque token defined as base64url-encoded canonical JSON of that DTO, with fixed field names and ordering and no padding. The authenticated profile returns both the structured DTO and the identical token. Clients compare token identity and exact strings; they never coerce a policy version to a number or infer ordering between compound versions.

The global singleton `rbac_policy_state` and unversioned process-global dynamic-role map are removed.

### Request snapshot algorithm

Authentication middleware creates one request-local snapshot before the handler. The snapshot is never stored as a process-global or cross-request user cache:

1. validate the authentication token and obtain only its stable principal identifier;
2. start a repeatable-read transaction;
3. in that transaction, resolve the active organization, membership, identity traits, active role assignment, principal scope facts, organization revision, principal revision, and role grants;
4. compile the role’s entitlements from those role grants and the deployed registry;
5. create an immutable `RequestAuthorizationSnapshot` containing the principal facts, compiled entitlements, exact snapshot version, `evaluated_at`, and `authorization_valid_until`;
6. commit the read transaction;
7. attach that snapshot to the request;
8. use that same object for route authorization, handler checks, profile serialization, response version headers, contextual action projection, and authorization audit events;
9. discard the snapshot when the request ends.

If a concurrent policy mutation commits after step 3, the request consistently uses the earlier revision. The next request observes the new revision. A response must never pair permissions or allowed actions from one revision with a different version.

Access windows use the half-open interval `[access_start_at, access_end_at)`: start is inclusive and end is exclusive. At the exact end instant access is denied. The API accepts offset-bearing instants, not ambiguous date-only values. A date-only UI that intends to include the whole displayed end day converts the following local midnight with its explicit configured display timezone and sends that offset-bearing exclusive instant; the backend rejects a missing offset. Authentication expiry is also exclusive.

`authorization_valid_until` is the earliest future access-window boundary, authentication expiry, or other principal-level time boundary that can change the current eligibility decision. Subject eligibility includes entitlement, identity, active membership, organization scope, and principal-level time/scope conditions evaluated at `evaluated_at`; only conditions requiring contextual data are deferred. A future access start can therefore be the validity boundary of a currently ineligible profile, just as an access end can be the boundary of an eligible one. At `authorization_valid_until`, the previous profile is already invalid for authorization and the frontend refreshes before enabling another mutation.

The initial implementation does not cache complete authorization snapshots. If profiling later demonstrates a need, a process cache may store only the role-level entitlement compilation keyed by `(catalog_hash, organization_id, organization_revision, role_id)`. Principal identity, assignment, scope, access windows, and final decisions are never shared across users or requests. Time-dependent conditions are evaluated against the current request time. Crossing a time boundary need not increment a database revision because `authorization_valid_until` drives client refresh and every backend request evaluates its own current time. Cache invalidation notifications or Redis may improve hit rate, but correctness never depends on receiving a notification. Load failure fails closed for protected actions.

## Backend Components

### Policy Registry

Owns typed declarations for grants, entitlements, actions, built-in roles, PDF labels, availability, authorization fact IDs/invalidation domains, and policy graph validation. It does not access the database and is deterministic for a given build. Database migrations and Snapshot Repository loaders consume generated fact identifiers; they do not maintain a second table-to-revision map.

### Principal Resolver

Loads user identity, organization membership, active role assignment, and typed scope facts. It is the only component permitted to translate legacy identity labels during migration.

### Snapshot Repository

Reads every mutable principal fact, normalized assignment, role grant, and both revisions in one transaction. It creates a request-local immutable snapshot and does not make authorization decisions. The initial implementation has no cross-request snapshot cache.

### Policy Kernel

Exposes separate typed decision interfaces:

```text
check_eligibility(action_id, snapshot) -> EligibilityDecision
authorize_subject(subject_action_id, snapshot) -> AuthorizationDecision
authorize_contextual_read(context_action_id<C>, snapshot, ContextSnapshot<'tx, C>) -> AuthorizedRead<'tx, C> | Denial
authorize_contextual_mutation(context_action_id<C>, snapshot, LockedMutationContext<'tx, C>) -> AuthorizedMutation<'tx, C> | Denial
```

`SubjectActionId` and `ContextActionId<ContextKind>` are generated typed projections of `ActionPolicy.decision_stage`; callers cannot pass a contextual action to `authorize_subject`, and a mismatched context kind does not type-check. A contextual read snapshot and `AuthorizedRead<'tx, C>` are tied to the consistent database read transaction used to build the returned projection. Protected projection/query APIs require that read permit; collection permits carry the enforced scope/filter so the query cannot silently omit it. A `LockedMutationContext<'tx, C>` and resulting mutation permit are tied to the mutation transaction lifetime, and every protected repository write API requires the correctly typed permit. A permit cannot be stored, changed to another target ID, or reused after commit. Context loaders expose only the minimum fields required for authorization before a read permit exists. The kernel evaluates entitlement, identity, scope, and contextual conditions and returns structured allow or deny decisions. It has no HTTP or frontend concerns.

### Protected route registration

Every protected route is registered through a wrapper that requires a typed subject-only or contextual action binding. A subject-only wrapper performs the final decision before body extraction. A contextual wrapper may reject subject-ineligible requests before body extraction, but that result is only an optimization and never marks the contextual action authorized. After validated body/query extraction and contextual loading, the domain service performs the final typed decision before reading protected data or mutating state. Registry generation and startup/CI validation fail if a contextual action lacks its matching final authorizer or is bound through the subject-only path.

Create and import operations authorize a validated `Proposed` context inside their mutation transaction before insertion. Parent-scoped child creates lock or consistently read the declared parent as required by the operation. Collection queries authorize the requested collection scope and then rely on the same transaction’s RLS/filter projection. Resource-set operations load all target IDs, reject mixed or unauthorized targets, and use a stable target lock order for mutations. Empty sets still require the declared collection/destination context and do not become subject-only actions.

Every contextual mutation is owned by a domain-service transaction with this mandatory order:

1. begin the mutation transaction;
2. lock the relevant organization/principal revision rows, existing target/parent rows, and mutable scope rows in the documented global lock order; proposed creates lock their destination scope and any serialization key required by a limit or uniqueness rule;
3. compare the locked revisions with the request snapshot; if they differ, return `AUTHORIZATION_SNAPSHOT_STALE` without writing so the client refreshes instead of authorizing from mixed facts;
4. load the latest proposed, existing, parent, or resource-set state and required scope facts into the matching `LockedMutationContext<'tx, C>`;
5. call `authorize_contextual_mutation` and obtain a transaction-bound `AuthorizedMutation<'tx, C>` permit;
6. pass that permit to the repository mutation, then write the business audit record;
7. commit; on stale snapshot, denial, or load failure, roll back without a write.

A handler-level precheck followed by a separately transactional mutation is prohibited. Contextual reads use one consistent read transaction or snapshot to load the declared context, perform the final decision, and build the returned projection. No protected collection or resource data is returned before that final decision. Resource `allowedActions` are an advisory UI projection of that read snapshot; every later mutation reauthorizes against the latest locked context and never trusts a client-supplied or previously returned action list.

The route registry generates:

- endpoint/action inventory;
- OpenAPI authorization metadata;
- frontend endpoint action IDs;
- audit action names;
- completeness tests.

There is no independently maintained `permission_contract.rs` permission list after migration.

### Role administration service

Owns role create, update, soft-delete, restore, and grant replacement transactions. It validates canonical grants, rejects reserved grants, enforces immutable built-in roles and the 20-active-custom-role limit, and relies on database revision triggers. Create and restore lock the organization policy-state row before counting and writing, making the limit atomic across concurrent requests. Soft-delete returns HTTP 409 while active user assignments reference the role; administrators must reassign those users first. It does not update a process-global cache.

The role projection applies the PDF account-context visibility rule: a sponsor administrator sees only the sponsor administrator type assigned to that account plus its custom roles, while unrelated global built-in administrator roles are omitted.

### Role editor behavior

The role-management UI follows PDF pages 94–95 as an explicit persistence contract:

- changing a custom-role checkbox or display name edits only a local dirty draft; only explicit Save persists it;
- a failed Save preserves the dirty draft and reports the server error, while a successful Save replaces the draft with the canonical server response;
- built-in-role grants and identity metadata are visible according to account context but are not editable;
- a soft-deleted custom role remains visible with strikethrough styling and its details intact, and can be restored;
- deletion while active assignments exist returns HTTP 409 and requires those users to be reassigned first;
- role list, create dialog, edit dialog, audit labels, and delete confirmation use the current display name; destructive confirmation never substitutes a UUID for the human-visible name;
- the 20-active-custom-role limit is enforced atomically in the role creation/restoration transaction, not only in the UI.

## Frontend Contract

The backend generator produces:

- typed `ActionId` constants;
- the PDF Role & Privilege row projection with `GrantId`, labels, order, and availability;
- endpoint-to-action metadata for API client diagnostics;
- the Policy Registry catalog hash.

The authenticated profile returns subject-level `eligibleActions`, the exact structured `PolicySnapshotVersion`, the matching opaque snapshot token, and `authorizationValidUntil`. An eligible action has passed all principal-level entitlement, identity, membership, scope, and time checks but is not a final authorization result when the action requires contextual data. The profile does not require the frontend to reconstruct action permission expressions.

Frontend authorization has one public boundary:

```text
isEligibleForAction(actionId)
canResourceAction(resource, actionId)
<EligibleActionGate action={actionId}>
<ResourceActionGate action={actionId} allowedActions={resource.allowedActions}>
```

Routes and global navigation use `eligibleActions`. Existing-resource screens use the target resource’s advisory `allowedActions`; they must not treat subject eligibility as final contextual authorization. Create, import, collection, and resource-set controls may use eligibility for visibility, but only the backend can finalize their proposed/filter/set context. Direct permission strings, permission arrays, `roleMeta.canAdmin`, summary booleans, and role-name authorization checks are prohibited by a syntax-aware static rule.

Resource responses expose final `allowedActions` for state- or scope-sensitive operations after loading the target. CASE detail therefore returns allowed lifecycle actions calculated from the same request snapshot, current request time, user scope, and case state. The frontend does not duplicate the lifecycle authorization expression; it uses the returned action set while still rendering ordinary domain state. Review and Lock never remove the independent Case Audit Trail read action, matching PDF page 41.

The generated frontend artifact embeds the build-time `CATALOG_HASH`. Snapshot synchronization uses these fail-closed rules:

- when catalog hash and organization match but the token or either revision differs, perform one deduplicated profile refresh and pause new mutations until it finishes;
- when `organizationId` differs, refresh the profile and reconcile the active organization before permitting a mutation;
- when `catalogHash` differs from the loaded frontend build, enter an update-required state, pause mutations, and force one cache-busted full-document reload; if the mismatch remains, show update-required/logout rather than trying to repair it with a profile refresh;
- schedule one deduplicated profile refresh at `authorizationValidUntil`, and on focus/visibility refresh if that boundary has passed; transition to logged-out or access-disabled state if the refreshed session is no longer valid;
- treat a 403 as an authoritative denial and never automatically retry the mutation.

Compound versions have identity, not ordinal “newer than” semantics. This explicitly handles an old browser tab after a deployment and an access-window expiry without a database revision change.

## Database Security Boundary

PostgreSQL RLS continues to enforce organization and record ownership isolation. It does not inspect `privileges_json`, PDF menu keys, frontend concepts, or free-form role strings.

Each database transaction receives one typed isolation context derived from the same request snapshot:

```text
DatabaseIsolationContext {
    principal_id,
    organization_id,
    platform_isolation_bypass,
    snapshot_version,
}
```

`platform_isolation_bypass` is true only when the principal resolver matched the assigned role UUID to the registry-declared immutable platform-administrator UUID. Sponsor administrators and custom roles never receive it. The database adapter exposes only `set_isolation_context(&RequestAuthorizationSnapshot)`; direct writes to authorization-isolation GUCs, legacy `set_org_context(role_string)`, and construction of the private bypass newtype outside the principal-resolver module are prohibited by syntax-aware checks. Unrelated authentication and compliance context setters remain separately typed. Transaction-pool tests prove that the isolation context is transaction-local and is cleared before connection reuse.

RLS checks only the organization ID, record ownership relationships, and this typed platform isolation flag. The flag is an isolation identity input, not a business-action permission. Cross-organization platform-administrator operations still require their registered Policy Kernel action; the flag only prevents RLS from contradicting an already authorized cross-organization operation. Integration tests cover ordinary users, both sponsor-administrator classes, custom roles whose names mimic administrators, platform administrators, missing context, and reused pooled connections.

Background jobs and migrations do not recreate `root_ctx` with a role string. They use a separately registered immutable service-principal UUID, explicit internal `ActionId`, and typed `ServiceIsolationContext`; public request code cannot construct that context. Internal bypass usage is included in the route/job action inventory and audit stream.

Business action authorization is performed by the Policy Kernel before the operation. This migration does not add database-level business authorization. Any future requirement for a database business-permission check needs a separate reviewed design and may not introduce a second mapping from roles or menu flags to permissions.

Audit-log RLS no longer derives read access from roles or grants. Organization isolation remains in RLS, while the registered audit read action is finalized by the Policy Kernel before the query. Append-only and no-update/no-delete protections remain database-enforced.

Database constraints provide defense in depth for:

- role and assignment referential integrity;
- immutable built-in role mutation rejection;
- organization-consistent role assignment;
- unique active assignment per user and organization;
- soft-deleted/inactive role assignment rejection;
- canonical grant validation through migration metadata;
- implemented-grant enforcement through the generated grant catalog foreign key;
- grant-to-role-class enforcement through `authorization_grant_role_classes`;
- mandatory organization/principal policy-state rows;
- monotonic organization revision changes;
- monotonic principal revision changes for assignment, membership, and scope mutations.

## Error and Audit Contract

Authorization denial returns HTTP 403 with:

```json
{
  "error": {
    "code": "AUTHORIZATION_DENIED",
    "actionId": "case.review.toggle",
    "policyVersion": {
      "catalogHash": "...",
      "organizationId": "...",
      "organizationRevision": "42",
      "principalRevision": "7"
    },
    "snapshotToken": "...",
    "requestId": "..."
  }
}
```

The response does not expose internal role composition or unrelated entitlements. Invalid or unknown action IDs are server configuration errors and fail closed. Unknown, aliased, or reserved grant IDs in public administration requests return HTTP 400 with a stable error code. A locked revision mismatch returns HTTP 409 `AUTHORIZATION_SNAPSHOT_STALE`, performs no write, and causes the client’s normal deduplicated profile refresh; the original mutation is not automatically retried. It must not emit a header that falsely pairs the request’s stale decisions with the observed locked revisions. Concurrent role updates and role-limit conflicts also return HTTP 409.

Authorization audit events record principal, organization, role ID, action ID, allow/deny result, snapshot version, target identifier when available, and request ID. They exclude sensitive payload data. Business change audit records remain separate and reference the same request and action IDs.

Audit durability follows the decision type:

- an allowed mutation writes its authorization event and business change audit inside the mutation transaction, so neither can survive without the protected write;
- a denied or stale mutation rolls back the mutation transaction first, then synchronously appends the denial/stale event through a separate restricted append-only audit transaction;
- an allowed contextual read appends its authorization event before releasing protected response data; if that append fails, the server returns HTTP 500 without the data;
- if a denial audit append fails, the denial remains authoritative, no protected operation is performed, and a high-severity operational event is emitted because returning an allow or retry would be less safe;
- subject-only decisions use the same allowed/denied audit rules according to whether they guard a read or mutation.

The audit writer accepts a completed structured decision, not roles, grants, or permission expressions. It cannot make or change an authorization decision. Tests force each audit-write failure mode and verify that no protected mutation or response data escapes.

## Migration Strategy

The frontend, backend, and database are deployed as one coordinated migration. Mixed old/new authorization contracts are not supported after cutover.

### Phase 0: Characterization and invariant tests

Before behavior changes, add failing tests that demonstrate:

- stale cross-process dynamic-role authorization;
- profile/version mismatch;
- unregistered protected routes;
- role-name authorization paths;
- PDF row, backend action, and frontend control drift;
- audit RLS disagreement with application authorization.

Create a complete inventory of protected endpoints and frontend actions. This inventory is diagnostic input, not a new manually maintained contract.

### Phase 1: Policy Registry and kernel in observation mode

Add the typed registry, graph validation, principal resolver, snapshot repository, and kernel without changing production allow/deny outcomes. Existing handlers remain authoritative temporarily. The new kernel records comparison metrics in tests and non-production environments only; it must not grant access based on a shadow decision.

All differences are classified against the PDF before proceeding. The target decision is updated only by changing the registry or fixing the principal/scope input, never by adding endpoint-specific exceptions.

### Phase 2: Normalized storage and one-way backfill

Create normalized tables, seed built-in roles, translate canonical legacy profiles, and create user-role assignments. The migration:

- maps supported aliases to one canonical grant;
- strips non-CASE Review/Lock flags;
- rejects or quarantines unrecognized active data instead of guessing;
- marks reserved features unassigned;
- generates grant-to-role-class catalog rows and rejects an assignment absent from that projection;
- creates one organization policy-state row per organization and one principal policy-state row per user-organization membership, then verifies exact cardinality before cutover;
- installs and verifies every authorization-fact revision trigger before normalized snapshots are enabled;
- produces a reconciliation report comparing legacy and new effective access for every active role.

Legacy and new data may be read in shadow comparison during this phase. There is no indefinite dual-write architecture. Administrative writes are paused during the final backfill/cutover transaction or routed exclusively through the new service once normalized tables become writable.

### Phase 3: Backend action cutover

Bind every protected route to an `ActionId`, switch subject-only handlers to final subject authorization, and bind every collection, proposed, existing, parent, and resource-set operation to its matching contextual authorizer. Move every contextual mutation into the transaction-bound lock → revision-check → final-authorization → write sequence. Switch profile and response headers to the same request snapshot and remove the old dynamic-role registry from runtime decisions.

Cutover is blocked until route completeness is 100%, cross-process tests pass, and the legacy/new reconciliation report has no unexplained differences.

### Phase 4: Frontend action cutover

Generate Action IDs and PDF grant rows, migrate routes and controls to `isEligibleForAction` or `canResourceAction` as appropriate, consume resource-level allowed actions, and remove handwritten permission and role-name authorization logic.

The PDF matrix remains visually unchanged except where the current UI contradicts the approved PDF. E-mail / Report Due Mail / Read and Send remain visible and disabled until their real actions exist; Settings / Read remains absent.

### Phase 5: Legacy removal

Remove legacy columns, JSON normalization, menu aliases, summary fields, duplicate response aliases, `RequireAdmin`/`require_admin` variants, role-string `root_ctx`, `Ctx::can_modify`, old role endpoints, manual endpoint permission manifests, and obsolete tests. Remove direct audit RLS interpretation of legacy RBAC data and replace `set_org_context(role_string)` with the typed isolation-context adapter.

## Verification Strategy

### Registry and policy graph

- every `GrantId`, `EntitlementId`, and `ActionId` is unique and canonical;
- every implemented PDF row maps to one grant and every visible row has the correct order and label;
- reserved grants cannot appear in stored assignments;
- every entitlement referenced by a grant or action exists;
- every action is reachable from at least one approved role or is explicitly marked internal;
- implication cycles and unknown references fail registry construction.

### Route and frontend completeness

- every authenticated protected route has exactly one registered action;
- every action declares `SubjectOnly` or one concrete `ContextKind`, and every contextual action has exactly one matching final-authorizer binding;
- characterization covers collection, proposed create/import, existing resource/download, parent-scoped child, and resource-set export/submission routes so none are reclassified as subject-only for convenience;
- public routes are explicitly marked public;
- no handler or middleware directly calls legacy permission/admin predicates;
- every frontend route, menu, control, and mutation uses a registered Action ID;
- generated files are regenerated in CI and fail on diff;
- syntax-aware checks reject direct role-name and permission-expression authorization.
- contextual operations cannot be registered through the subject-only wrapper; protected repository projection/query and mutation APIs cannot be called without the correctly typed transaction-bound read or mutation permit.

### Snapshot consistency

- launch two independent server processes against one test database;
- establish authorization through both processes, mutate a role through process A, and authorize through process B; if the optional role-compilation cache is later enabled, exercise the same test with that cache warm;
- process B may use the old revision only if it reports the old revision; once it reports the new revision it must use the new decision;
- profile, route guard, handler, resource allowed actions, response header, and audit event must carry the same version;
- profile and response header contain byte-identical opaque snapshot tokens whose decoded DTO includes catalog hash, organization ID, organization revision, and principal revision;
- public revisions round-trip as canonical decimal strings above JavaScript’s safe-integer limit without loss or numeric coercion;
- two users with the same organization and role but different sender/product/study/blind scopes must never share principal facts or final decisions;
- assignment or scope mutation between authentication and snapshot creation must produce either the complete earlier snapshot or the complete later snapshot, never a mixture;
- user membership, role assignment, scope, blind access, active sender, and access-window mutations must advance principal revision;
- fake-clock tests prove start-inclusive/end-exclusive behavior immediately before, exactly at, and immediately after each access-window boundary; the backend decision and frontend boundary refresh change without waiting for a policy mutation or cache invalidation even when revisions are unchanged;
- an old browser tab whose embedded catalog hash differs must pause mutation and require a full-document update rather than repeatedly refreshing the profile;
- restart, cache loss, missed notifications, and concurrent mutation tests must preserve the invariant;
- snapshot-load failure must deny protected actions without falling back to built-in or stale permissions.

### Database boundary and audit durability

- ordinary users, sponsor administrators, and custom roles cannot set or obtain platform isolation bypass, including when a custom display name or stable key resembles a platform administrator;
- platform-administrator cross-organization operations require both a registered allowed action and the typed isolation context;
- missing, leaked, or reused pooled-connection isolation context fails closed;
- every mutable authorization fact appears in exactly one invalidation domain and every database source has the required revision trigger;
- organization and membership creation atomically create their required policy-state rows; clean bootstrap and backfill contain no missing state rows;
- database writes reject implemented grants assigned to an unapproved role class;
- forced authorization-audit failures never release protected read data or commit a protected mutation; denial-audit failure still returns denial and emits a high-severity operational event.

### Security properties

- user editing cannot grant role-management or role-assignment actions;
- read grants cannot perform execute, update, review, lock, or export actions unless explicitly granted by the PDF;
- role self-escalation, assignment to inactive/deleted roles, cross-organization assignment, and built-in role modification are rejected;
- a custom role with a built-in display name, stable key, role-class payload, or equivalent grants never acquires a privileged identity trait; only registry-declared built-in UUIDs resolve to those traits;
- soft-delete of a role with active assignments returns HTTP 409 until those users are reassigned;
- CASE Review, Lock, ordinary Edit, and raw status mutation remain independently enforced;
- Case Audit Trail remains readable while the case is reviewed, validated, or locked when the user has the audit read action;
- organization and sender/product/study/blind scopes are applied independently of RBAC grants;
- concurrent tests that reassign scope or change target, parent, collection destination, or resource-set state between early eligibility and mutation prove that the final decision uses the latest locked context in the same transaction and performs no write on denial;
- RLS and application authorization cannot consult different policy representations.

### PDF role-management behavior

- custom-role checkbox and name changes remain local until explicit Save;
- failed Save retains a dirty draft and successful Save replaces it with the server projection;
- built-in controls are read-only and account-context visibility matches the PDF;
- soft-deleted roles remain visible with details and strikethrough styling and can be restored;
- delete confirmation and list/dialog labels use the current display name;
- concurrent create/restore requests cannot exceed 20 active custom roles.

### Migration

- backfill is deterministic and idempotent;
- every legacy active role has an explicit reconciliation result;
- aliases never remain in normalized storage;
- rollback before cutover restores the legacy runtime; after destructive legacy removal, rollback uses the release database backup rather than dual runtime logic;
- clean-database bootstrap and upgraded production-like database tests produce the same built-in roles, grants, and catalog hash.

## Mapping of Known Issues to Architectural Removal

| Known issue | Architectural resolution |
|---|---|
| `capabilities` reverse mapping | Already removed; no replacement summary model is introduced |
| menu-to-permission expansion drift | Grant definitions and actions live in one validated registry |
| permission catalog drift | Entitlements, grants, and actions are exported from the same registry |
| `RequireAdmin` and function gate duplication | All protected operations call the Policy Kernel by Action ID |
| `ctx.is_admin`, `USER_CREATE`, and metadata disagreement | Identity traits and action entitlements are separate typed inputs |
| duplicate `can_admin` derivation | Summary authorization fields are deleted |
| built-in metadata duplication | One role projection service reads normalized roles and registry metadata |
| duplicate API response aliases | New DTOs expose one canonical field per concept |
| menu aliases stored separately | Aliases exist only in the one-time migration translator |
| frontend/backend privilege-table mismatch | PDF matrix rows are generated from `GrantDefinition` |
| `Ctx::can_modify` and old role API | Removed during legacy deletion |
| partial endpoint contract | Route action binding is executable and mandatory |
| process-local stale cache | Request-local snapshots replace global mutable state; only role compilation may be cached later |
| DB RLS reads `privileges_json` | RLS is restricted to organization and record isolation |
| incomplete E-mail representation | Both PDF Report Due Mail rows are explicit, reserved, disabled, and unassignable until implemented |
| custom-role identity spoofing | Privileged identity derives only from registry-declared immutable built-in UUIDs |
| resource authorization/write race | Final authorization and mutation share locked state and one transaction-bound permit |
| numeric policy-version drift | Canonical compound DTO and opaque identity token replace ordinal comparison |
| access-window UI staleness | Snapshot validity boundary schedules profile refresh while backend reevaluates every request |
| single-resource authorization assumption | Typed contextual authorization covers collection, proposed, existing, parent, and resource-set operations |
| RLS role-string admin bypass | Typed snapshot-derived isolation context replaces `app.current_user_role` |
| unenforceable grant role classes | Generated `authorization_grant_role_classes` makes the database constraint executable |
| authorization denial audit rollback | Denials append only after mutation rollback through a restricted audit transaction |
| missing revision state rows | Organization/membership lifecycle atomically owns required state rows and backfill verifies them |
| JavaScript revision precision | Public 64-bit revisions serialize as canonical decimal strings |
| ambiguous access-window expiry | Start-inclusive/end-exclusive semantics define the exact refresh and denial instant |

## Scope Boundaries

This design does not implement the absent RE scheduled-mail system, replace the existing workflow model, redesign sender/product/study/blind-data semantics, or introduce an external authorization service. It changes how existing identity, scope, role, grant, and lifecycle facts are represented and evaluated for authorization.

The migration may change database schema and frontend/backend API contracts because the repositories are deployed together. Compatibility adapters are temporary migration tools and are deleted at cutover; they are not permanent policy sources.

## Completion Criteria

The architecture migration is complete only when:

1. the approved PDF matrix is generated from canonical backend grant definitions;
2. all protected backend routes are bound to registered Action IDs;
3. handlers, profile responses, headers, contextual allowed actions, and audit events use one request-local snapshot created from a single repeatable-read transaction, with the exact structured version and matching opaque token;
4. two independent backend processes cannot report the same snapshot version with different authorization decisions;
5. same-role users with different scopes cannot share principal facts, and all principal authorization mutations advance principal revision;
6. frontend production code contains no authorization role-name checks or handwritten entitlement expressions, distinguishes subject eligibility from final resource authorization, refreshes at time boundaries, and fails closed on catalog-hash mismatch;
7. normalized roles, grants, assignments, grant catalog, organization revision, and principal revision replace legacy role strings and `privileges_json`;
8. DB RLS no longer interprets legacy RBAC representations;
9. PDF Report Due Mail Read/Send rows, built-in role visibility, explicit-save role editing, dirty-draft failure behavior, role display names, soft-delete/restore rules, Review/Lock, and Audit Trail behavior are explicitly preserved;
10. all known duplicate gates, summary fields, aliases, manual endpoint contracts, dead RBAC code, and old role calls are removed;
11. the full registry, migration, backend, frontend, cross-process, cross-principal, security, and PDF conformance suites pass from clean environments; and
12. an unexplained legacy/new decision difference blocks deployment rather than being accepted as a compatibility exception;
13. privileged identity traits are derivable only from registry-declared immutable built-in UUIDs; and
14. every contextual read uses a typed permit tied to the consistent transaction that loads its context and returned projection;
15. every contextual mutation performs final authorization against its locked proposed, existing, parent, destination, or resource-set state inside the same transaction as the write;
16. RLS consumes only typed snapshot-derived isolation context and no role/grant policy representation;
17. every authorization fact has one tested revision domain and every required policy-state row has transactional lifecycle ownership;
18. authorization audit failure cannot expose read data, commit a write, or convert a denial to an allow; and
19. access-window, public revision serialization, and all contextual action kinds have boundary and conformance tests.
