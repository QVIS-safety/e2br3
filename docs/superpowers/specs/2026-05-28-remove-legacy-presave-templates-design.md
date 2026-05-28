# Remove Legacy Presave Templates Design

## Goal

Hard-remove the legacy generic `presave_templates` system from the backend. The canonical section presave BMCs and routes are now the source of truth. Existing legacy master data is intentionally discarded; no data migration from `presave_templates` or `presave_template_audits` is required.

## Scope

Remove:

- Public REST routes under `/api/presave-templates`.
- OpenAPI documentation for `/api/presave-templates`.
- `presave_template_rest.rs`.
- `lib-core` `presave_template` model module, including `PresaveTemplateBmc`, `PresaveTemplateAuditBmc`, and `PresaveEntityType`.
- Bootstrap schema for `presave_templates` and `presave_template_audits`.
- Bootstrap triggers, updated-at trigger wiring, RLS policies, and dev-db compatibility ALTER statements for those tables.
- Tests whose only purpose is proving legacy generic template behavior.

Keep:

- Canonical `/api/presaves/{section}` parent routes.
- Canonical child routes and `/details` graph routes.
- Existing `PRESAVE_TEMPLATE_*` permissions for now. They are stale names but still used as INFO presave permissions; permission renaming is a separate change.
- Canonical section table audit triggers. Field-level audit remains supported through canonical section audit infrastructure, not `presave_template_audits`.

## Architecture

The backend should no longer expose or compile any generic presave template BMC or route. Section-aware behavior must use the canonical BMCs:

- Sender: `SenderPresaveBmc`, `SenderPresaveGatewayBmc`, `SenderPresaveResponsiblePersonBmc`
- Receiver: `ReceiverPresaveBmc`, `ReceiverPresaveConsigneeBmc`
- Product: `ProductPresaveBmc` and product child BMCs
- Reporter: `ReporterPresaveBmc`
- Study: `StudyPresaveBmc` and study child BMCs
- Narrative: `NarrativePresaveBmc` and narrative child BMCs

The only legacy type still indirectly useful is the concept of a section key for scope checks. Since `PresaveEntityType` currently lives in the legacy model module, replace it with a local internal enum in `section_presave_rest.rs`, such as `PresaveScopeSection`, with only the sections that need assigned-scope checks: sender, product, and study.

## Runtime Replacements

`lib-rest-core` sender option loading currently reads sender identifiers from `presave_templates.data`. Replace that query with canonical sender data:

- Read direct sender identifiers from `sender_presave_gateways.sender_identifier`.
- Keep existing message-header fallback identifiers.
- Respect `sender_presaves.deleted = false`.
- Keep organization isolation.

`lib-core` XML import runtime currently finds the default sender using `PresaveTemplateBmc` and JSON fields. Replace it with canonical sender tables:

- Find a non-deleted `sender_presaves` row for the requested authority where `is_default = true`.
- Load its default or first gateway row for sender/routing identifiers.
- Load its default or first responsible person row for C.3.3 values.
- Map canonical sender fields into `c_helpers::SenderImport`.

## Database

Remove legacy objects from bootstrap scripts:

- `presave_templates`
- `presave_template_audits`
- all legacy indexes
- `audit_presave_templates`
- `audit_presave_templates_dedicated`
- `update_presave_templates_updated_at`
- RLS policies for both legacy tables
- dev-db ALTER/index compatibility statements for `presave_templates`

No drop migration is required in this spec because the user explicitly accepts rewriting/init cleanup and discarding legacy master data. The bootstrap scripts should create only canonical presave tables.

## Tests

Delete or rewrite tests that call `/api/presave-templates`.

Rewrite tests that still need coverage:

- Sender option tests should create canonical sender presaves plus gateway rows.
- XML/import default sender tests should seed canonical sender records.
- Scope tests should use canonical `/api/presaves/senders`, `/api/presaves/products`, and `/api/presaves/studies`.
- Add a route-negative test that `/api/presave-templates` returns `404 Not Found`.

Remove tests that only covered legacy JSON template behavior, such as generic entity type filtering, include-global template behavior, and template audit endpoint behavior.

## Non-Goals

- No frontend migration in this backend plan.
- No data migration from generic JSON rows.
- No permission rename from `PRESAVE_TEMPLATE_*`.
- No canonical audit endpoint redesign unless tests reveal that a backend call still depends on `/api/presave-templates/{id}/audit`.

## Verification

Run:

```bash
cargo fmt
cargo test -p lib-core
cargo test -p lib-rest-core
cargo test -p web-server --test api presave_contract_web -- --nocapture --test-threads=1
cargo test -p web-server --test api import_contract_web -- --nocapture --test-threads=1
cargo test -p web-server --test api scope_visibility_web -- --nocapture --test-threads=1
cargo test -p web-server --test api submission_lifecycle_web -- --nocapture --test-threads=1
```

Final repository scan must show no runtime references to:

- `PresaveTemplateBmc`
- `PresaveTemplateAuditBmc`
- `presave_template_rest`
- `/api/presave-templates`
- `presave_templates`
- `presave_template_audits`

Historical docs may still mention the old system.
