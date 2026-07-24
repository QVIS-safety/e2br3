# RBAC Single Authorization Design

## Objective

Make the reviewed QVIS UI specification the visible RBAC contract while
ensuring that every allow/deny decision is made by one authorization engine.
Remove parallel administrator gates, permission checks, and derived
administrator booleans instead of adding another policy layer.

This design covers four security and consistency goals:

1. `case.read` grants case viewing only and cannot grant export or user access.
2. User administration cannot become role administration or role assignment.
3. All administrator and permission decisions use one canonical authorization
   entry point.
4. The Role & Privilege screen and runtime behavior match the PDF contract.

## Reviewed PDF Contract

The authoritative source is
`QVIS Safety Database_UI Specification_18JUN2026_Updated.pdf`, especially
pages 8, 13, and 95.

The editable matrix contains these implemented rows in this order:

| Order | Menu | Type | Privilege | UI binding |
|---:|---|---|---|---|
| 1 | HOME | Notice | Read | `home_notice.can_read` |
| 2 | HOME | Notice | Edit | `home_notice.can_edit` |
| 3 | HOME | Workflow | Read | `home_workflow.can_read` |
| 4 | CASE | Case | Read | `case.can_read` |
| 5 | CASE | Case | Edit | `case.can_edit` |
| 6 | CASE | Workflow | Read | `case_workflow.can_read` |
| 7 | CASE | QC | Edit | `case.can_review` |
| 8 | CASE | Lock | Edit | `case.can_lock` |
| 9 | INFO | Case Info | Read | `info.can_read` |
| 10 | INFO | Case Info | Edit | `info.can_edit` |
| 11 | IMPORT | Import Files | Edit | `import.can_edit` |
| 12 | IMPORT | Import History | Read | `import.can_read` |
| 13 | EXPORT/SUBMISSION | Export/Submit | Edit | `export_submission.can_edit` |
| 14 | EXPORT/SUBMISSION | Export/Submit History | Read | `export_submission.can_read` |
| 15 | ADMIN | Admin | Read | `admin.can_read` |
| 16 | ADMIN | Admin | Edit | `admin.can_edit` |

The following rows remain visible but reserved and disabled until the Report
Due Mail feature exists:

| Order | Menu | Type | Privilege | UI binding |
|---:|---|---|---|---|
| 17 | E-mail | Report Due Mail | Read | `email_report_due.can_read` |
| 18 | E-mail | Report Due Mail | Send | `email_report_due.can_edit` |

There are no separate `users`, `roles`, `settings`, `audit`, or alias rows in
the PDF matrix. Legacy keys may be translated during migration, but cannot be
stored as independent current grants.

## Architecture

### One decision engine

The existing `authorization::kernel` is the only component allowed to decide
whether an operation is allowed. No second policy service is introduced.

The canonical flow is:

```text
authenticated request
  -> construct canonical action request
  -> authorization::kernel::authorize(ctx, action, scope)
       -> policy registry lookup
       -> canonical privilege evaluation
       -> built-in identity condition evaluation
       -> organization/resource scope evaluation
       -> allow or deny
  -> authorized permit
  -> construct database RLS context from that permit
  -> perform operation
```

Static endpoint extractors or middleware may remain as thin adapters. A thin
adapter may only translate an endpoint marker into a canonical action request
and delegate to the kernel. It cannot inspect roles, permissions, or resource
state itself.

Body-dependent operations authorize inside the handler after parsing the body.
An endpoint must use either the static adapter or the body-dependent call, not
both.

Existing resource handlers that still accept a legacy `Permission` use one
temporary compatibility entry point inside `authorization::kernel`. The
REST `require_permission` function only translates the kernel result to the
existing HTTP error. The kernel reads permissions compiled one-way from the
canonical registry grants; no second permission-to-action table is added.
Production web code cannot call `has_permission` directly. This keeps one
decision implementation while canonical typed actions replace legacy call
sites incrementally.

### Components retained

- `policy_registry`: the only policy data source for canonical PDF
  privileges, actions, identity conditions, and UI bindings. A PDF privilege
  is the stored grant; there is no separate entitlement layer.
- `authorization::kernel`: the only allow/deny implementation.
- `Ctx`: authenticated user ID, organization ID, legacy permission subject,
  and compliance context. It exposes no administrator predicate.
- Permit-bound RLS context builders: database isolation setup after
  authorization. They require an `AuthorizedRead` or `AuthorizedMutation`
  carrying the evaluated target organization, so they cannot run before the
  kernel decision.
- Generated frontend authorization contract: produced from the backend
  registry and never maintained as a handwritten second matrix.

### Duplicate decision paths removed

The following cannot remain as independent authorization implementations:

- `require_role_admin`
- direct authorization branches using `ctx.is_admin()` in REST handlers
- permission checks inside `user_admin_db_ctx`
- `USER_CREATE` or another permission interpreted as administrator identity
- `can_admin` or equivalent derived response fields
- endpoint middleware and handler checking the same operation independently
- the `Grant -> Entitlement -> Action` middle layer
- `Ctx::is_admin()` and any equivalent generic administrator predicate
- freely callable `user_admin_db_ctx`/`rls_ctx_for_user_admin` helpers that
  do not require a kernel permit

## Action and Scope Model

The registry owns the canonical action identifier for every protected
operation. User and role operations must remain distinct:

- user read/list
- user create
- user general-field update
- user delete/restore
- user role assignment
- permission-profile read
- permission-profile create/update/delete

The action request carries the organization and target resource when they are
known. Cross-organization access is denied by the kernel before an RLS context
is constructed.

Denial uses the existing normalized authentication/authorization response
shape:

- unauthenticated request: `401`
- authenticated but unauthorized action: `403`
- malformed or unknown matrix/alias input: `400`
- authorized request for an absent in-scope entity: `404`

The frontend may present a friendly message, but it cannot reinterpret a
denial as a different permission decision.

## Case Read Isolation

`case.read` must not compile through the generic `viewer_permissions()` role
bundle. It receives a dedicated, explicitly enumerated case-view permission
set.

The set includes only:

- case shell `Read` and `List`
- `Read` and `List` operations required to render case detail sections
- case-linked reference data that is strictly necessary to render those
  sections

It excludes:

- `XML_EXPORT` and `XML_EXPORT_READ`
- `USER_READ` and `USER_LIST`
- organization administration
- every `Create`, `Update`, `Delete`, `Approve`, `Lock`, `Import`, `Export`,
  and `Send` operation

New case-detail resources are not automatically included by category. Their
read/list permissions must be added explicitly with an API contract test.

## User Administration and Role Security

`admin.read` and `admin.edit` are operational grants, not administrator
identity.

`admin.read` may provide read access to the permitted Admin workspace data,
including users, organization metadata, settings, audit data, and terminology.
It does not grant permission-profile access.

`admin.edit` may add the following operational abilities:

- create a user with the baseline `user` role
- update non-role user fields
- delete or restore a user
- manage organization data allowed by the PDF Admin area
- update settings
- manage terminology

It does not grant:

- built-in administrator identity
- assignment or replacement of a user's role
- permission-profile creation, update, deletion, or privilege changes
- assignment of System Administrator or Sponsor Administrator

Requests that combine an allowed general user update with a forbidden `role`
change fail as a whole. The server does not silently apply the allowed subset.

Permission-profile management and role assignment require their own canonical
actions and a matching built-in System/Sponsor Administrator identity
condition. Possessing `USER_CREATE`, `USER_UPDATE`, or every operational Admin
permission cannot satisfy that identity condition.

## PDF Row Runtime Semantics

- HOME Notice Read exposes notices to the authenticated dashboard.
- HOME Notice Edit allows notice creation, update, and deletion and implies
  Notice Read.
- HOME Workflow Read allows the user's assigned workflow/case queue to be
  viewed without granting case mutation.
- CASE Case Read follows the isolated case-view contract above.
- CASE Case Edit grants case authoring and implies Case Read.
- CASE Workflow Read exposes workflow state without granting workflow
  transition mutation.
- CASE QC Edit grants only review/QC transitions.
- CASE Lock Edit grants only lock and unlock transitions.
- INFO Read/Edit control the Case Info surface independently.
- IMPORT Edit executes imports; IMPORT History Read views import history.
- EXPORT/SUBMISSION Edit executes export/submission; its History Read row
  views history and associated downloads.
- ADMIN Read/Edit follow the operational limits above.
- E-mail rows compile to no runtime permission while reserved.

Reserved rows are disabled in the frontend, removed from outgoing payloads,
and ignored by backend normalization. They cannot grant
`EMAIL_NOTIFICATION_SEND`.

## Frontend Contract

The frontend consumes generated rows from the backend registry.

- `ROLE_PRIVILEGE_ROWS` is a projection of the generated contract.
- No handwritten mirror of row order, labels, availability, or UI binding is
  introduced.
- Unsupported aliases are removed before sending and rejected by the backend
  if submitted directly.
- Inactive roles remain available in history but do not appear in assignment
  selectors or editable privilege columns.
- Built-in role metadata is display information only and never substitutes
  for endpoint permissions.

## Migration

Legacy menu keys are accepted only by the one-way migration translator.
Migration produces canonical grants or no grants:

- aliases map to their canonical current row
- removed rows produce no grant
- reserved E-mail rows produce no runtime grant
- inactive historical roles do not block reconciliation
- active invalid roles fail closed and produce a migration rejection

Successful reconciliation resolves obsolete rejection records. Runtime writes
cannot recreate alias rows.

## Verification

### Structural tests

Source-level architecture tests fail when REST authorization code introduces:

- direct `ctx.is_admin()` allow/deny branches
- `require_role_admin`
- a `can_admin` response field
- permission-to-identity derivation
- a second endpoint-level check for an already authorized operation

Thin adapters must be tested to prove that they delegate to the kernel and
contain no independent role or permission logic.

### Policy tests

- The generated PDF contract contains exactly 18 ordered rows.
- Rows 1-16 are implemented; rows 17-18 are reserved.
- Every implemented row compiles to its expected permissions.
- No row compiles to an unrelated permission.
- Case Read explicitly excludes export, user, and mutation permissions.
- Admin Edit explicitly excludes role assignment and permission-profile
  management.

### API integration tests

For a custom role containing only Case Read:

- case list and detail reads succeed
- XML export execution and history fail with `403`
- user list/read fail with `403`
- case create/update/delete/review/lock fail with `403`

For a custom role containing Admin Read/Edit:

- permitted user general-field operations behave according to the grant
- role changes fail with `403`
- permission-profile CRUD fails with `403`
- built-in role assignment fails with `403`
- cross-organization access fails

Notice, workflow, info, import, submission, QC, and lock rows receive matching
positive and negative API tests.

### Frontend and browser E2E

- save and reload all 18 matrix rows
- confirm reserved rows are disabled and never sent
- confirm inactive roles are excluded from assignment and editing
- authenticate users with minimal Case Read and Admin Read/Edit roles
- verify visible navigation and real API outcomes
- attempt role escalation and confirm denial
- restart the backend and confirm authorization reconciliation completes
  without legacy-runtime fallback

Temporary E2E users and roles are deactivated or removed after verification.

## Definition of Done

- The policy registry and kernel are the only policy data and decision
  implementation.
- Thin adapters contain no authorization logic.
- REST handlers contain no direct administrator allow/deny branches.
- Case Read cannot access exports or users.
- Operational Admin permissions cannot assign or manage roles.
- The frontend shows the exact PDF rows and reserved state.
- Contract, unit, API integration, roundtrip, browser E2E, and restart checks
  all pass against the same build.
