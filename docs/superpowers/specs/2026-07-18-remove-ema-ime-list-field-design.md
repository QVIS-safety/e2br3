# Remove EMA IME List Field Design

**Date:** 2026-07-18

## Goal

Remove the app-local reaction field `E.local.includedInEmaImeList` from the
database, backend API, case editor, XML import/export, and field registry.

## Decision

The field is not editable, displayed, validated, searched, or calculated from
an EMA IME list. Its only current behavior is preserving a value received from
legacy XML or API input and returning that value during later case edits or XML
exports. Legacy round-trip compatibility is explicitly out of scope, so the
field has no remaining product consumer.

The removal is end-to-end. Keeping only a database or transport representation
would recreate an orphan field without restoring any business behavior.

## Removed Contract

The following representations are deleted together:

- PostgreSQL `reactions.included_in_ema_ime_list`
- Rust `Reaction`, create, and update model members and SQL bindings
- REST/OpenAPI `included_in_ema_ime_list` request and response members
- Frontend `includedInEmaImeList` types, detail-load mapping, and save payload
- XML extension observation `AE_IME_LIST` import and export
- Registry row `E.local.includedInEmaImeList`
- Tests and fixtures whose only purpose is preserving the removed contract

Other reaction fields, including `expectedness`, `severity`, and MFDS device
reaction extensions, remain unchanged.

## Database and Deployment

Fresh databases omit the column from `db/bootstrap/05-reactions.sql`. Existing
databases apply an idempotent migration using
`ALTER TABLE reactions DROP COLUMN IF EXISTS included_in_ema_ime_list`.
Existing values are intentionally discarded and are not archived or copied.

Frontend and backend may be deployed together. Deploying the frontend first is
also safe because the backend currently accepts an omitted optional field.
After the backend deployment, old clients that still send the removed REST
member are not guaranteed compatibility.

## Testing

Regression coverage must prove:

1. Backend model, bootstrap SQL, migration target, OpenAPI, and registry no
   longer expose the field.
2. XML import ignores `AE_IME_LIST` and XML export never emits it.
3. Frontend case-detail loading and reaction save payloads no longer carry the
   field.
4. Existing reaction API, XML, registry, frontend case-save, and TypeScript
   checks remain green.
5. A repository-wide reference scan contains no production references to
   `includedInEmaImeList`, `included_in_ema_ime_list`, or `AE_IME_LIST` after
   removal; migration history is the sole allowed snake-case occurrence.

Implementation follows red-green TDD by first extending removal-contract tests
that fail against the current field inventory, then deleting the production
contract and updating obsolete positive round-trip fixtures.

## Out of Scope

- Adding EMA IME terminology lookup or automatic calculation
- Replacing the field with a visible form control
- Preserving or migrating existing values
- Removing adjacent reaction extensions
