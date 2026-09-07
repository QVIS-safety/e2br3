# Production Readiness / Goysop Audit

> Status: refreshed 2026-09-02 against `origin/dev` plus the pending case-query SQL branch. Remediated findings remain so old claims are not mistaken for current behavior.

## Executive verdict

The repository is not production-ready. The strongest remaining blockers are not stylistic:

- startup provisions known administrator/demo credentials;
- development database initialization is disabled by an opt-out flag;
- internal machine routes still authorize root-like operations with bearer secrets rather than signed requests or mTLS;
- operational telemetry has no documented durable sink, retention, or incident-response contract;
- transaction ownership can still be escaped by cloning `ModelManager`.

The authorization route inventory is active, so “everything is unauthenticated” is not a supported claim. The more accurate claim is: parts of the system have real authorization machinery, but the deployment, runtime, internal-service, transaction, and operational boundaries are not production-grade.

## Severity scale

- **P0** — stop production use; data loss or account compromise is plausible.
- **P1** — production release blocker; integrity, availability, or security boundary is unsafe.
- **P2** — serious maintainability, efficiency, or operational maturity debt.

## Confirmed production blockers

### P0 — Normal CD deployment recreates the database (remediated)

- [`.github/workflows/cd.yml`](../.github/workflows/cd.yml#L3) now starts production CD only after the `CI` workflow succeeds on `main`.
- The normal deploy command pins `RESET_DB=0`, `INCLUDE_SEED=0`, and `RELOAD_TERMINOLOGY=0` ([`cd.yml`](../.github/workflows/cd.yml#L146)).
- Destructive reset scripts still exist for deliberate maintenance, but they are no longer wired into the normal production deployment path.

### P0 — Known administrator credentials are embedded and reset at startup

- [`bootstrap.rs`](../crates/services/web-server/src/bootstrap.rs#L15) hardcodes the administrator email and password `welcome`.
- [`bootstrap.rs`](../crates/services/web-server/src/bootstrap.rs#L275) resets the matching user's password to that value on every startup.
The hardcoded credential still exists in the dev/test bootstrap code and must be replaced before any controlled bootstrap is needed in a deployed environment.

### P1 — Development database reset is an opt-out safety model

- [`main.rs`](../crates/services/web-server/src/main.rs#L33) calls the dev initializer during startup, with only the `SKIP_DEV_INIT=1` escape hatch.
- [`_dev_utils/mod.rs`](../crates/libs/lib-core/src/_dev_utils/mod.rs#L14) retains its `SKIP_DEV_INIT=1` test/local escape hatch.
- [`dev_db.rs`](../crates/libs/lib-core/src/_dev_utils/dev_db.rs#L10) contains hardcoded local database URLs and recreates the database.

The dev helper remains reachable to test code and should eventually move behind a dev-only feature.

### P1 — Internal routes expose root-like operations behind one shared token

- [`routes_internal.rs`](../crates/services/web-server/src/web/routes_internal.rs#L6) explicitly places internal machine routes outside user authentication.
- ACK validates `AS2_CALLBACK_TOKEN`/`x-callback-token`; reconcile and status now require the separate `SUBMISSION_RECONCILE_TOKEN`/`x-reconcile-token` ([`submission_rest.rs`](../crates/services/web-server/src/web/rest/submission_rest.rs#L549)).
- ACK mutation executes with `Ctx::root_ctx()` ([`ack.rs`](../crates/services/web-server/src/submission/ack.rs#L216)).
- Both secrets are still bearer tokens, but comparison uses a fixed-work byte comparison ([`submission_rest.rs`](../crates/services/web-server/src/web/rest/submission_rest.rs#L576)).

Each endpoint class now has its own bearer secret, so a leaked gateway callback token does not authorize reconciliation. There is still no request signature, replay protection, or mTLS boundary in this flow.

### P1 — Cookie authentication has no explicit CSRF/security-cookie contract (baseline remediated)

- [`token.rs`](../crates/libs/lib-web/src/utils/token.rs#L18) now sets `HttpOnly`, `SameSite=Lax`, and `Secure` automatically in production (or with `E2BR3_COOKIE_SECURE=1`).
- State-changing API requests with an auth cookie now reject mismatched/`null` `Origin` values; deployments behind a proxy can pin `E2BR3_PUBLIC_ORIGIN` ([`lib.rs`](../crates/services/web-server/src/lib.rs#L95)).
- The authentication cookie is refreshed on every request ([`mw_auth.rs`](../crates/libs/lib-web/src/middleware/mw_auth.rs#L172)).

State-changing APIs still use an ambient browser cookie, but the cookie and origin checks now form an explicit baseline contract. A full synchronizer-token CSRF scheme remains optional if cross-site embedding or multiple trusted browser origins become requirements.

### P1 — Reconcile workers use a fixed claim lease (remediated)

- Reconciliation now acquires a PostgreSQL transaction advisory lock per submission ([`reconcile.rs`](../crates/services/web-server/src/submission/reconcile.rs#L83)).
- Concurrent workers skip a held submission, and the lock is released with the transaction. The old five-minute duplicate-processing window no longer exists.

### P1 — Authentication token construction (remediated)

- [`token/mod.rs`](../crates/libs/lib-auth/src/token/mod.rs#L125) now uses HMAC-SHA256 and the MAC verifier for constant-time signature checks.
- The serialized token shape remains `ident.exp.signature`; rotating `SERVICE_TOKEN_KEY` or deploying this change invalidates old sessions, which is the intended session migration boundary.

This is now a standard MAC construction. Opaque server-side sessions remain a future option if revocation and fleet-wide session management become requirements.

### P1/P2 — ZIP/XML processing lacks aggregate resource limits (remediated)

- XML upload is capped at 50 MiB, each ZIP entry at 25 MiB, with 128 entries and 100 MiB expanded aggregate limits ([`import_rest.rs`](../crates/services/web-server/src/web/rest/import_rest.rs#L43)).
- Terminology upload allows 250 MiB and now bounds ZIP entry count, per-entry bytes, and aggregate expanded bytes ([`terminology_rest.rs`](../crates/services/web-server/src/web/rest/terminology_rest.rs#L26), [`terminology_import.rs`](../crates/libs/lib-core/src/model/terminology_import.rs#L43)).

The archive decompression path now has an explicit ceiling. The remaining work is to measure real dictionary sizes and tune the constants without silently raising them.

### P1 — Operational logging lacks a durable operations contract (partially remediated)

- Request completion now emits structured `info!`/`warn!` records with request ID, duration, status, user ID, and a query-free path ([`log/mod.rs`](../crates/libs/lib-web/src/log/mod.rs#L20)).
- Upstream gateway response bodies are no longer copied into client-facing errors ([`gateway.rs`](../crates/services/web-server/src/submission/gateway.rs#L75)).
- No durable sink, retention period, redaction review, alerting policy, or incident evidence workflow is documented.

### P1 — CI/CD is not a real release gate (remediated)

- CD is triggered by a successful `CI` workflow on `main`, not by a raw push ([`cd.yml`](../.github/workflows/cd.yml#L3)).
- The deploy job requires the publish job and deploys the immutable workflow SHA image ([`cd.yml`](../.github/workflows/cd.yml#L114)).
- The development deployment gate requires registry, formatting, clippy, and test jobs ([`ci.yml`](../.github/workflows/ci.yml#L195)).

## Architecture and code-structure review areas

The review covered these areas:

1. duplicated REST handler patterns and wrapper layers;
2. fallback paths that silently bypass transaction or authorization context;
3. oversized modules and functions that combine transport, policy, persistence, and side effects;
4. `ModelManager`/`Dbx` transaction ownership and clone semantics;
5. unbounded list queries, N+1 loops, repeated serialization/parsing, and blocking work inside async paths;
6. dead configuration, `allow(dead_code)`, TODO-driven safety behavior, and one-use abstractions;
7. startup/shutdown behavior, worker lifecycle, retry/fallback semantics, and failure recovery;
8. duplicate domain models, mapping code, and validation layers that disagree about the same input.

## Minimum remediation order

1. Remove runtime demo/admin bootstrap and dev initialization from production.
2. Isolate internal APIs with private networking plus signed or mTLS requests.
3. Define a durable, redacted telemetry and incident-response contract.
4. Replace implicit `ModelManager`/`Dbx` transaction ownership.
5. ~~Push the remaining legacy case-list scope and pagination into SQL.~~ Done.
6. Add token revocation/rotation semantics if immediate session revocation becomes a requirement.

`cargo check --workspace` passed during this review. That only establishes that the current tree compiles; it does not establish production safety.

## Second pass — architecture, code shape, fallbacks, and efficiency

### P1 — Startup defaults cross the development/production boundary (partially remediated)

- [`main.rs`](../crates/services/web-server/src/main.rs#L33) still starts the dev DB initializer and bootstrap path from the normal binary.
- [`_dev_utils/mod.rs`](../crates/libs/lib-core/src/_dev_utils/mod.rs#L14) still has a dev helper that is compiled into the binary and uses `SKIP_DEV_INIT=1` as an additional escape hatch.
- [`dev_db.rs`](../crates/libs/lib-core/src/_dev_utils/dev_db.rs#L10) uses hardcoded local credentials and recreates `app_db`.
- [`bootstrap.rs`](../crates/services/web-server/src/bootstrap.rs#L293) still resets dev bootstrap users' passwords to the static value `welcome` when explicitly invoked.
- [`user_rest/validation.rs`](../crates/services/web-server/src/web/rest/user_rest/validation.rs#L332) now rejects a missing/blank password instead of selecting a predictable credential.

The remaining gaps are compile-time isolation of dev helpers, explicit staging strictness, and replacement of the static dev credential.

### P2 — Missing domain identifiers are rendered as empty strings (remediated)

- Editor shell, lifecycle, and export paths now reject a missing or blank `safety_report_id`.
- The old empty-string fallbacks no longer hide this broken domain invariant.

### P2 — Case collection endpoints have inconsistent query shapes (remediated)

- The list-view now applies sender/product/study scope before ordering and bounded SQL pagination ([`case.rs`](../crates/libs/lib-core/src/model/case.rs#L921)).
- The previous 5,000-row projection, separate scope query, and in-memory pagination are gone.
- The dynamic case-query and list-view paths share the same SQL scope predicate.
- The legacy `GET /api/cases` path now applies the same scope predicate before bounded SQL pagination. Its per-case scope query is gone, and workflow settings are loaded once per non-empty response instead of repeatedly per case.
- The legacy list no longer loads `cases.raw_xml`, which was discarded before serialization; single-case and export reads retain the canonical XML payload.

The dynamic case-query endpoint now applies sender/product/study scope inside the candidate SQL and supports bounded `limit`/`offset` pagination ([`case_query_catalog_rest.rs`](../crates/services/web-server/src/web/rest/case_query_catalog_rest.rs#L205)). Calls that omit pagination retain the existing 5,000-result compatibility ceiling.

### P1/P2 — Case editor projections contain obvious N+1 loops (remediated)

- Parent medical-history/past-drug children and death-cause/autopsy children are loaded once per child family for all parent IDs ([`direct/dm.rs`](../crates/services/web-server/src/web/rest/case_editor_rest/direct/dm.rs#L1295)).
- The previous per-parent sequential database loops are gone.

### P2 — Bulk XML export remains synchronous, but has request and byte caps

- Single and bulk XML export now return only XML from one shared generation helper. The unused `Case` argument/return pair, full-payload clones, and redundant handler-level case reads are removed; authorization still checks each target before generation, and the XML serializer retains its own case read.
- Bulk XML export loads runtime settings once, at the first case's generation step, and reuses that snapshot for the request. Settings failures still record the failing case's export history; subsequent requests reload settings. E2B timestamp formatting now lives in `lib-utils::time`, and export filenames no longer accept an unused suffix switch.
- [`BulkXmlExportInput`](../crates/services/web-server/src/web/rest/case_export_rest.rs#L32) is capped at 100 unique cases and 100 MiB of uncompressed XML ([`case_export_rest.rs`](../crates/services/web-server/src/web/rest/case_export_rest.rs#L415)).
- [`case_export_rest.rs`](../crates/services/web-server/src/web/rest/case_export_rest.rs#L458) processes each case serially and performs repeated case, identifier, export, and history work.
- The entire ZIP is accumulated in a `Cursor<Vec<u8>>` before responding ([`case_export_rest.rs`](../crates/services/web-server/src/web/rest/case_export_rest.rs#L453)).

This is still a synchronous batch job hiding behind an HTTP request. A job queue, timeout, and streaming response are deferred until the capped path proves insufficient.

### P2 — Root fallback mixes API routing and static-file routing (remediated)

- [`lib.rs`](../crates/services/web-server/src/lib.rs#L79) sends every unmatched application route to the static file service.
- [`routes_static.rs`](../crates/libs/lib-web/src/routes/routes_static.rs#L13) is a generic `ServeDir` fallback.

API, internal, and login routers now own an explicit JSON `route_not_found` fallback. Static-file fallback remains only for non-API paths, so an API typo no longer enters the UI/static subsystem.

### P2 — OpenAPI declarations are documentation-only (removed)

The unused runtime router and generated declaration were deleted. Reintroduce an API schema only when a real consumer or compatibility gate requires it.

### P2 — Large files remain, but raw line count is no longer the main defect

Current large application modules include:

- `case_editor_rest/input_contract_fields.rs`: 3,113 lines;
- `lib-core/model/presave.rs`: 2,846 lines;
- `xml/export/sections/g.rs`: 2,638 lines;
- `case_editor_rest/input_contract_save.rs`: 1,794 lines;
- `case_editor_rest/direct/dm.rs`: 1,566 lines.

`case_editor_rest/direct.rs` is now a 231-line dispatcher over domain files, and `common.rs` is 678 lines. Crate-root dead-code suppression was removed; the remaining 25 allowances are confined to tests/examples. Further splitting is justified only where transport, policy, and persistence still change for different reasons—not to improve a line-count score.

### P2 — Transaction ownership is convention-based and has a silent non-transaction fallback

- [`ModelManager::clone`](../crates/libs/lib-core/src/model/mod.rs#L116) creates a fresh `Dbx` and therefore a fresh transaction holder over the same pool.
- [`Dbx`](../crates/libs/lib-core/src/model/store/dbx/mod.rs#L16) stores transaction state separately from the pool.
- When no holder is present, queries silently execute directly against the pool ([`dbx/mod.rs`](../crates/libs/lib-core/src/model/store/dbx/mod.rs#L161)).

Atomicity depends on every caller keeping the exact same manager/DBx instance and remembering the right transaction ceremony. A clone or helper can silently escape the transaction instead of failing loudly.

### P2 — Async export bridge (remediated)

[`xml/export/shared/postprocess.rs`](../crates/libs/xml/src/export/shared/postprocess.rs#L35) now loads a Send-safe database snapshot before parsing libxml. Section mutation is synchronous, so the export path no longer needs `spawn_blocking + block_on` wrappers in the library or HTTP callers.

### P2 — Repeated default pagination is a hidden payload and latency multiplier

The repository contains 50 uses of `ListOptions::default()` across 17 Rust files. The common list implementation defaults to 1,000 rows ([`base_uuid.rs`](../crates/libs/lib-core/src/model/base/base_uuid.rs#L428)), while generated REST handlers pass that default directly ([`macro_utils.rs`](../crates/libs/lib-rest-core/src/utils/macro_utils.rs#L80)). Detail endpoints should request explicit, bounded projections; “default list” is not a safe contract for nested clinical data.

## Second-pass priority order

1. Isolate dev initialization and known bootstrap credentials from deployed startup.
2. Replace `ModelManager`/`Dbx` transaction convention with an explicit transaction object passed through the operation.
3. Put export batches behind timeout/job boundaries only if the existing 100-case/100-MiB cap proves insufficient.
4. Split remaining large files only along real domain or ownership boundaries.

No new dependency is required. The next useful architectural cut is implicit transaction ownership.

### P1 — Regulatory payloads are copied into long-lived or unsafe debug locations (partially remediated)

- The parsed import snapshot no longer duplicates inbound XML; the canonical `cases.raw_xml` copy remains for round-trip/export behavior.
- Export validation no longer writes full XML to the process temporary directory or returns a debug path.
- The remaining concern is the intentional canonical raw XML retention and its lifecycle/access policy.

For clinical/regulatory payloads, “debug copy” is a data-retention decision. There is no visible lifecycle, encryption, cleanup, or access policy around these extra copies.

net: 0 new dependencies. The stale OpenAPI layer, static/API fallback overlap, editor child N+1 queries, and all case-list in-memory scope filters are gone. Remaining architecture debt is deployed dev/bootstrap startup, transaction ownership, and operational telemetry.
