# Canonical Presave Details Graph Design

## Context

The backend now has separated canonical presave models and tables for sender, receiver, product, reporter, study, and narrative presaves. It also has parent and child REST routes for these canonical resources. The legacy `/api/presave-templates` route remains for compatibility, and the frontend still uses `usePresaveTemplates` as a mixed legacy/canonical hook.

The frontend should move to a canonical architecture instead of expanding `usePresaveTemplates`. The new architecture introduces `CanonicalPresave<TData>`, section-specific hooks, and details graph endpoints for sections with child rows.

## Goals

- Represent canonical presaves in the frontend with `CanonicalPresave<TData>`, not `PresaveTemplate<TData>`.
- Keep `PresaveTemplate<TData>` only for legacy `/api/presave-templates` compatibility.
- Add backend graph endpoints for section detail/edit workflows.
- Keep parent and child routes as canonical CRUD surfaces.
- Avoid frontend orchestration of multi-child saves for normal detail forms.
- Migrate section by section without breaking existing callers.

## Non-Goals

- Do not remove `/api/presave-templates` in the first implementation stage.
- Do not migrate every frontend caller in one large change.
- Do not add graph details endpoints for reporter unless reporter later gains child rows.
- Do not use implicit child deletion based on omitted arrays.

## Backend Route Shape

Canonical parent routes remain:

```text
GET    /api/presaves/senders
POST   /api/presaves/senders
GET    /api/presaves/senders/{id}
PATCH  /api/presaves/senders/{id}
DELETE /api/presaves/senders/{id}

GET    /api/presaves/receivers
POST   /api/presaves/receivers
GET    /api/presaves/receivers/{id}
PATCH  /api/presaves/receivers/{id}
DELETE /api/presaves/receivers/{id}

GET    /api/presaves/products
POST   /api/presaves/products
GET    /api/presaves/products/{id}
PATCH  /api/presaves/products/{id}
DELETE /api/presaves/products/{id}

GET    /api/presaves/reporters
POST   /api/presaves/reporters
GET    /api/presaves/reporters/{id}
PATCH  /api/presaves/reporters/{id}
DELETE /api/presaves/reporters/{id}

GET    /api/presaves/studies
POST   /api/presaves/studies
GET    /api/presaves/studies/{id}
PATCH  /api/presaves/studies/{id}
DELETE /api/presaves/studies/{id}

GET    /api/presaves/narratives
POST   /api/presaves/narratives
GET    /api/presaves/narratives/{id}
PATCH  /api/presaves/narratives/{id}
DELETE /api/presaves/narratives/{id}
```

Canonical child routes remain:

```text
/api/presaves/senders/{id}/gateways
/api/presaves/senders/{id}/responsible-persons
/api/presaves/receivers/{id}/consignees
/api/presaves/products/{id}/substances
/api/presaves/products/{id}/fda-cross-reported-inds
/api/presaves/products/{id}/mfds-regional-items
/api/presaves/studies/{id}/registration-numbers
/api/presaves/studies/{id}/fda-cross-reported-inds
/api/presaves/narratives/{id}/sender-diagnoses
/api/presaves/narratives/{id}/case-summaries
```

Details graph endpoints are added for sections with child rows:

```text
GET /api/presaves/senders/{id}/details
PUT /api/presaves/senders/{id}/details

GET /api/presaves/receivers/{id}/details
PUT /api/presaves/receivers/{id}/details

GET /api/presaves/products/{id}/details
PUT /api/presaves/products/{id}/details

GET /api/presaves/studies/{id}/details
PUT /api/presaves/studies/{id}/details

GET /api/presaves/narratives/{id}/details
PUT /api/presaves/narratives/{id}/details
```

Reporter does not receive a details route initially because it has no child rows.

## Details Graph Semantics

`GET /details` loads the parent row and all canonical child collections for the section. It returns a single graph shaped for detail/edit screens.

`PUT /details` updates the parent and applies explicit child operations. Missing child arrays are no-ops. Empty child arrays are also no-ops. There is no replace-entire-collection behavior in this design.

Child save rules:

```text
child with id and _delete: true -> delete or soft-delete
child with id                   -> update
child without id                -> create
omitted child array              -> no-op
empty child array                -> no-op
```

The request delete marker is `_delete: true`. The response may expose `deleted: boolean` only where the underlying model has a deleted field. Storage behavior is backend-internal: child tables with a deleted column can soft-delete; child tables without one may hard-delete.

Invalid graph operations should fail clearly:

- `_delete: true` without `id` is invalid.
- Child row `id` that belongs to another parent is invalid.
- Child row `id` that belongs to another organization is invalid through normal org isolation.
- Authority-specific child constraints, such as FDA cross-reported IND parent authority, remain enforced by the BMC layer.

## Frontend Types

Add a canonical type that is separate from legacy template terminology:

```ts
export type PresaveSection =
  | "sender"
  | "receiver"
  | "product"
  | "reporter"
  | "study"
  | "narrative";

export interface CanonicalPresave<TData> {
  id: string;
  organizationId: string;
  authority: string;
  name: string;
  description?: string | null;
  comments?: string | null;
  isGlobal: boolean;
  canEdit: boolean;
  createdBy?: string | null;
  createdAt?: string | null;
  updatedAt?: string | null;
  deleted?: boolean;
  data: TData;
}
```

`PresaveTemplate<TData>` remains only for legacy callers backed by `/api/presave-templates`.

Section data types should preserve the domain language already used by forms, but their persistence mappers must align with canonical backend fields. Narrative needs an explicit case-summary collection because the backend canonical model has `narrative_presave_case_summaries`; a single scalar `caseSummary` is not enough for the canonical graph.

## Frontend Hook Architecture

Create section-specific hooks:

```text
useSenderPresaves
useReceiverPresaves
useProductPresaves
useReporterPresaves
useStudyPresaves
useNarrativePresaves
```

Each hook owns:

- parent list loading
- parent create/update/delete
- details loading where the section has children
- details graph save where the section has children
- section-specific mapping between backend records and `CanonicalPresave<TData>`

Generic screens may use a thin dispatcher:

```text
useCanonicalPresaves(section)
```

The dispatcher delegates to section hooks. It must not contain route-specific child persistence logic.

`usePresaveTemplates` remains as the legacy compatibility hook until all callers are migrated.

## Frontend Data Flow

List screens call parent list endpoints:

```text
GET /api/presaves/{section}?authority={authority}
```

Detail/edit screens call details endpoints for graph sections:

```text
GET /api/presaves/{section}/{id}/details
PUT /api/presaves/{section}/{id}/details
```

Reporter detail/edit screens call the parent endpoint:

```text
GET   /api/presaves/reporters/{id}
PATCH /api/presaves/reporters/{id}
```

Initial create flow should remain parent-first:

```text
POST /api/presaves/{section}
PUT  /api/presaves/{section}/{newId}/details
```

Graph create, such as `POST /api/presaves/{section}/details`, is deferred until there is a concrete UI need.

## Backend Implementation Notes

Each details handler should be small and explicit:

- load parent through the existing parent BMC
- load child rows through existing child BMC list APIs or scoped SQL helpers
- assemble a section-specific details DTO
- on `PUT`, update parent first, then apply child creates, updates, and explicit deletes
- execute graph save in a transaction where available
- reuse existing authorization, organization isolation, and BMC validation

The graph layer should not bypass parent-child scope checks. If the existing child BMC methods do not enforce enough parent scope, details handlers must verify child ownership before update/delete.

## Testing Strategy

Backend API contract tests should cover each details graph section:

- `GET /details` returns parent and all child collections.
- `PUT /details` updates parent fields.
- `PUT /details` creates child rows without ids.
- `PUT /details` updates child rows with ids.
- `PUT /details` deletes only child rows marked `_delete: true`.
- Omitted child arrays do not delete anything.
- Empty child arrays do not delete anything.
- `_delete: true` without `id` fails.
- Child ids from another parent fail.
- Authority-specific constraints still fail where applicable.

Frontend tests should cover:

- canonical mappers for each section
- section hook list/detail/save behavior
- dispatcher delegation
- migrated info list/detail screens
- narrative case summaries are loaded and saved through the canonical graph

## Migration Plan

1. Add backend details DTOs, handlers, routes, and contract tests.
2. Add `CanonicalPresave<TData>` and canonical section graph types.
3. Add shared canonical API helpers for parent list, parent CRUD, details load, and details save.
4. Add section hooks.
5. Migrate `InfoPresaveListRoute` and `InfoPresaveDetailRoute` to `useCanonicalPresaves`.
6. Migrate specific consumers in small follow-up changes: study product pickers, sender pickers, submission page, admin pages, SectionC3, and case duplication check.
7. Keep `usePresaveTemplates` for remaining legacy callers during the migration.
8. Remove `usePresaveTemplates` and `/api/presave-templates` after call sites and tests no longer depend on them.

## Open Decisions Resolved

- Use explicit `_delete: true` for graph child deletion.
- Do not use omitted arrays or empty arrays as collection replacement.
- Do not add reporter details endpoint initially.
- Use `CanonicalPresave<TData>` rather than extending `PresaveTemplate<TData>`.
- Keep parent and child canonical routes even after adding graph endpoints.
