# Presave Lifecycle Delete Guard Design

## Goal

Unify every presave logical and physical deletion path behind one lifecycle boundary so that REST `DELETE`, REST `PUT` with `deleted=true`, internal jobs, and direct BMC callers cannot apply different reference policies.

## Scope

This change covers Sender, Receiver, Product, Study, Reporter, and Narrative presaves. It consolidates deletion guards, converts in-memory relationship scans to SQL `EXISTS`, and makes guard evaluation plus mutation atomic. It does not normalize the existing user scope JSON/string columns into relationship tables.

## Current Problem

Deletion is currently represented by two public operations: REST `DELETE` and update payloads containing `deleted=true`. Case-reference checks primarily live in REST handlers, while presave-to-presave checks and some user-scope checks live in individual BMCs. Consequently, the same state transition can have different guards depending on its entry point. Reporter, Narrative, Study, and Sender have paths where `deleted=true` can bypass a case-reference check. Direct BMC callers can also bypass policies that exist only at the REST layer.

Several BMC guards load all candidate rows and scan them in Rust. Guard checks and the subsequent mutation are separate database operations, which permits a reference to be created between the check and the mutation.

## Architecture

Create a focused presave lifecycle module in `lib-core`. It owns the domain commands that deactivate or physically delete a presave:

```rust
pub enum PresaveKind {
	Sender,
	Receiver,
	Product,
	Study,
	Reporter,
	Narrative,
}

pub struct PresaveLifecycleService;

impl PresaveLifecycleService {
	pub async fn archive(
		ctx: &Ctx,
		mm: &ModelManager,
		kind: PresaveKind,
		id: Uuid,
	) -> Result<()>;

	pub async fn hard_delete(
		ctx: &Ctx,
		mm: &ModelManager,
		kind: PresaveKind,
		id: Uuid,
	) -> Result<()>;
}
```

`archive` and `hard_delete` execute the same dependency policy. The only difference is the final mutation. REST remains responsible for HTTP permission checks, scope visibility, DTO parsing, and translating model conflicts into HTTP 409. The lifecycle service owns the invariant that a used or assigned presave cannot be archived or deleted.

Existing public BMC update methods must not allow callers to set `deleted=true` directly. REST update handlers route such requests to `archive`; normal field updates continue through the BMC. Existing BMC `delete` methods delegate to `hard_delete`, preserving their public signatures while removing the bypass.

## Dependency Policy

Each presave kind has an explicit dependency specification:

| Kind | Case reference | Presave reference | User assignment |
|---|---|---|---|
| Sender | `sender_information.source_sender_presave_id` | active `product_presaves.sender_presave_id` | `access_sender_ids` |
| Receiver | none | active Product receiver link | none |
| Product | `drug_information.source_product_presave_id` | active Study parent or Study product child link | `access_product_ids` |
| Study | `study_information.source_study_presave_id` | none | `access_study_ids` |
| Reporter | `primary_sources.source_reporter_presave_id` | active Study reporter child link | none |
| Narrative | `narrative_information.source_narrative_presave_id` | none | none |

The Product-to-Study guard covers both `study_presaves.product_presave_id` and active `study_presave_products.product_presave_id` rows.

Receiver compatibility checks use `product_presaves.receiver_presave_id = receiver.id`. For legacy products whose UUID link is null only, the guard also compares normalized `original_manufacturer` with the receiver organization name. A populated, different UUID is never overridden by the legacy name fallback.

All case queries include organization isolation. Presave-to-presave queries rely on UUID references and organization-aware RLS/context, with explicit organization predicates where the child table provides the organization column through its parent.

## Transaction and Concurrency

The lifecycle service opens one transaction, installs the full request context on that transaction, locks the target presave row with `SELECT ... FOR UPDATE`, evaluates all dependency queries with `SELECT EXISTS`, and performs the final mutation before commit.

Target-row locking serializes competing lifecycle operations. Reference creation paths must lock the referenced presave row before accepting a new reference. The implementation will extend existing assignment validation to acquire a compatible row lock in the same transaction where possible. This closes the check-then-reference race without relying on process-local locks.

If a reference appears or remains visible during the transaction, the lifecycle service returns `model::Error::Conflict` with a stable entity-specific message. Any query or mutation failure rolls back the transaction.

## REST Data Flow

Logical deletion through either API shape follows the same path:

```text
REST permission and scope check
  -> PresaveLifecycleService::archive
       -> begin transaction and install context
       -> lock target
       -> case EXISTS check
       -> presave dependency EXISTS check
       -> user assignment EXISTS check
       -> UPDATE deleted = true
       -> commit
  -> HTTP success or 409 Conflict
```

For `PUT` payloads that combine `deleted=true` with other field changes, deletion is treated as an archive command. Other field changes are rejected with `BadRequest` rather than being silently applied or ignored. This keeps archive semantics deterministic.

## Error Contract

- Missing or invisible target: existing model not-found behavior.
- Insufficient REST permission: existing permission-denied response.
- Referenced by a case, another presave, or an active user assignment: HTTP 409 through `model::Error::Conflict`.
- `deleted=true` combined with other mutations: HTTP 400.
- Database or transaction failure: existing internal error mapping with rollback.
- Re-archiving an already archived, otherwise unreferenced row: idempotent success.

## Testing

Tests are written before production changes and cover the public behavior and service boundary.

For each presave kind:

- REST `DELETE` rejects a case-linked record where applicable.
- REST `PUT deleted=true` rejects the same record.
- An unreferenced record can be archived.
- A direct BMC physical delete cannot bypass the lifecycle guard.
- Conflict status and message remain stable.

Relationship-specific tests cover Sender-to-Product, Receiver-to-Product UUID, Receiver legacy null-UUID fallback, Product-to-Study parent and child links, Reporter-to-Study reporter links, and active user assignments. A concurrency test holds a lifecycle target lock while attempting reference creation and verifies that only one valid final state can commit.

Regression verification includes the complete `section_presave` model suite, presave REST authorization tests, formatting, and compilation.

## Migration and Compatibility

No data migration is required. Existing endpoints and successful response shapes remain unchanged. Conflict behavior becomes stricter only for paths that previously bypassed an established delete guard.

The Receiver name fallback remains temporarily for legacy data but is narrowed to Product rows with no UUID receiver link. Its eventual removal requires a separate data backfill and is outside this scope.

## Non-Goals

- Normalizing `users.access_*_ids` into join tables.
- Changing frontend delete UX or endpoint shapes.
- Introducing a generic global dependency graph framework.
- Removing Receiver legacy compatibility before a data backfill.
- Changing restore behavior.
