# RBAC Privilege Roundtrip Design

## Objective

Make the existing PDF-aligned Role & Privilege selections control both backend authorization and frontend visibility without introducing another RBAC concept or duplicating the permission catalog.

## Constraints

- `AdminMenuPrivilege[]` remains the only user-facing menu privilege model.
- No `roleGrants`, capability mirror, additional permission table, cache, or client-maintained privilege catalog is introduced.
- The existing low-level `permissions` array remains available only for API/action authorization.
- A user has exactly one active role in an organization.
- Reserved Email rows remain visible but disabled and are never persisted.
- System-organization custom roles are forbidden because operational users cannot belong to the system organization.

## Current Failure

The Role & Privilege matrix persists `AdminMenuPrivilege[]`, but the authenticated frontend discards that information and reconstructs menu access from expanded low-level permissions. A `case.read` privilege expands to supporting permissions such as `PresaveTemplate.Read` and `Organization.Read`. The frontend then incorrectly interprets those dependencies as INFO and ADMIN menu access.

System administrators can also create custom profiles in the system organization even though user provisioning requires a non-system organization. Those profiles cannot be assigned to a usable operational account.

## Architecture

### Authenticated privilege contract

`GET /api/users/me/profile` returns the assigned role's existing normalized `AdminMenuPrivilege[]` as `privileges` alongside the existing `permissions` array.

- Custom roles use their stored, backend-normalized `permission_profiles.privileges_json`.
- Sponsor administrator built-ins use the same built-in privilege construction already used by the Role & Privilege API.
- The platform system administrator receives only the administration privileges required by its platform UI; it does not receive operational CASE, INFO, Import, or Submission privileges.
- Missing, inactive, cross-organization, or ambiguous role assignments fail closed and return no operational privileges.

The backend remains authoritative. The frontend does not reconstruct privileges from low-level permissions.

### Frontend consumption

`AuthContext` stores both:

- `permissions`: low-level action permissions used by action-specific guards;
- `privileges`: normalized menu privileges used by navigation and screen visibility.

Sidebar visibility, Dashboard quick actions, and ADMIN section tabs consume small shared helpers that answer read/edit/review/lock checks against `privileges`. `access-rules.ts` no longer maps low-level permissions back to menus.

The Role & Privilege editor continues to use the generated PDF rows and its existing payload sanitizer. No additional frontend authorization table is added.

### Organization targeting

Custom roles are always owned by a non-system organization.

- Sponsor administrators are fixed to their authenticated organization.
- System administrators must select a target active CRO or Company organization before listing, creating, updating, or deleting custom roles and before creating a user with one of those roles.
- System-admin permission-profile requests carry `organizationId`; sponsor-admin requests ignore any caller-supplied organization and use the authenticated organization.
- The backend validates that the selected organization exists, is active, is not the system organization, and owns the custom role.
- The same selected organization is included when the system administrator creates the user, closing the role-to-user roundtrip.

No global custom role is introduced.

## Error Handling

- System-admin role operations without `organizationId` return `400 organization_id is required`.
- System organization, inactive organization, and cross-organization role IDs are rejected before mutation.
- Sponsor administrators cannot use `organizationId` to escape their organization.
- Frontend role and user forms remain disabled until a valid target organization is selected.
- Authorization failures remain server-enforced even if the frontend is bypassed.

## Testing

### Backend

- Current-user profile returns the exact normalized privileges for a custom role.
- `case.read` does not produce INFO or ADMIN menu privileges.
- Built-in role privileges are returned consistently with the Role & Privilege API.
- System-admin custom-role CRUD requires and scopes to a non-system organization.
- User creation with the selected organization and role creates one membership and one active role assignment.
- Startup authorization reconciliation creates the system membership before assigning the system administrator role.

### Frontend

- Sidebar and Dashboard use `privileges`, not expanded `permissions`.
- CASE Read exposes CASE and hides INFO, ADMIN, Import, Submission, and edit-only actions.
- ADMIN Read and Edit expose only their permitted ADMIN surfaces.
- Reserved Email rows remain disabled and are absent from update payloads.
- System-admin role and user forms share the selected target organization.
- Existing generated-row and matrix roundtrip tests remain green; stale matrix expectations are updated to the generated PDF contract.

### E2E acceptance

Run backend and frontend against a dedicated database, then verify:

1. System administrator selects a sponsor organization.
2. A custom CASE Read role is created and saved.
3. A user is created in the same organization with that role.
4. The user logs in and sees CASE but not INFO, ADMIN, Import, or Submission.
5. Case listing succeeds, while user administration and case mutation are denied.
6. Editing the role to add or remove privileges is reflected after policy refresh and relogin/refresh.
