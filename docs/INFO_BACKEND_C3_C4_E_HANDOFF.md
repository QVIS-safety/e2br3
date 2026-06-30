# INFO Backend Handoff — C3 (Audit), C4 (Report-due Mail), E (ACL)

Spec + implementation plan for another thread. Source: QVIS Safety Database
UI Specification (18JUN2026), INFO section (pp.20–37). This covers the three
remaining INFO backend workstreams that depend on DB / mail infra / policy
and were not implementable in the originating session.

## Context (already done, do not redo)
- `report_due.rs` — C1 receiver report-due classification + due-date calc (pure, tested).
- `narrative_template.rs` — C2 `{E2B.CODE}` substitution engine (pure, tested).
- `ReporterPresave.{reporter_name_null_flavor, reporter_address_null_flavor, qualification_null_flavor}` — A1.
- `ReporterPresave.qualification_kr1` (C.2.r.4.KR.1) — A2 (restored; `section_presave.rs` guard test now asserts it MUST exist).
- B1/B2 (duplicate + required-field guards) already implemented in each presave BMC (`validate_identity` + `ensure_unique_identity`).
- B3 in-use delete guards: presave→presave (BMC), presave→Case (`*_used_by_cases` in `section_presave_rest.rs`), presave→Admin-User via `access_*_ids` (BMC `id_scope_contains` / `any_user_scope_contains`).
- Reusable INFO records = `*Presave` models in `crates/libs/lib-core/src/model/presave.rs` (NOT the per-case BMC models). presave uses modql `Fields` + `base_uuid` auto CRUD (no positional-bind SQL).

---

## C3 — Audit Trail + Change Reason  (REVISED — mostly already implemented)

### Spec (Slide#23, applies to ALL INFO tabs on Edit)
On editing an existing INFO record, every changed field is recorded in an
Audit Trail with columns: **No / Date-Time / User / Item / Value / Notation / Reason**.
On save, a Change Confirmation popup collects a reason category
(Input Error / New Data / Edited Data / Others) + free-text; that reason
appears in the Audit Trail `Reason` column.

### Current state — VERIFIED ALREADY DONE (do not re-implement)
The original "no path to pass a user-entered reason" gap is **false**. End to end already works:
- Reason path: auth middleware accepts `x-e2br3-reason-for-change` and stores it on `Ctx` — `crates/libs/lib-web/src/middleware/mw_auth.rs:42`.
- Presave updates pass `Ctx` into `base_uuid::update`, which calls `set_full_context_from_ctx_dbx` before the UPDATE fires the trigger — `crates/libs/lib-core/src/model/base/base_uuid.rs:134`.
- The DB trigger computes per-field `changed_fields` and writes `reason_for_change` into `audit_logs` — `db/bootstrap/10-triggers.sql:262` (and the other INSERTs at 246/272/289/...).
- Per-record audit read path exists: `AuditLogBmc::list_by_record` — `crates/libs/lib-core/src/model/audit.rs:357`.
- Existing web test for INFO presave reason capture: `crates/services/web-server/tests/api/presave/sender_web.rs:157`.
- `cargo test -p lib-core section_presave` → 14 passed.
  (`cargo test -p web-server test_update_sender_presave_records_reason_for_change` did not compile due to **unrelated** pre-existing errors in `case_rest.rs` / `cioms_export_rest.rs` — not this feature.)

### Remaining real gap (the only one verified)
Backend stores a **single `reason_for_change` string**, NOT a separate
`change_category` enum/column. `change_category` does not exist anywhere in
`crates/` or `db/`. So the spec's 4-way category (Input Error / New Data /
Edited Data / Others) is not stored as a distinct field today.

### Revised plan — confirm contract, then (optionally) add category
1. **Confirm the UI contract**: does the frontend send category + free-text combined into one `x-e2br3-reason-for-change` string (e.g. `"Input Error: typo"`), or must they be stored separately? This is a product/UX decision.
2. **If combined string is acceptable** → C3 is effectively complete; only the frontend needs to format `category: text` into the header and parse it back for display. No backend change.
3. **If a distinct category is required** → add `change_category` to `audit_logs` (column) + populate it from a second header (e.g. `x-e2br3-change-category`) carried on `Ctx` alongside the existing reason, written by the same trigger path. Small additive change mirroring the existing `reason_for_change` plumbing.
4. The Audit Trail read view (No/Date-Time/User/Item/Value/Notation/Reason) is served by `AuditLogBmc::list_by_record` + `changed_fields`; map those to the table columns in the frontend.

### Files (only if category column is required)
`audit_logs` schema (`db/bootstrap/`), `10-triggers.sql`, `mw_auth.rs` (second header → Ctx), `dev_db.rs`, tests.

### Verify (needs DB)
Edit with header → `audit_logs` row has `reason_for_change` (already passing). If adding category: row also has `change_category`.

### Risk / size
Small. Reason/diff/read all already work; the only possible work is one
additive `change_category` column IF the UI requires a separate field.
Do NOT rebuild the audit/diff/read machinery.

---

## C4 — Report-due Notification Mail

### Spec (p.25, INFO > Receiver)
The case's담당 User (the user with Sender/Product/Study access for that case)
receives an email alert as the Report-due date approaches. Report-due date is
auto-filled on the case RE page (1st pass) and editable.

### Current state
- C1 logic done (`report_due.rs`). No persistence of computed due dates, no mail, no scheduler.

### Plan
1. Confirm infra: mail transport (SMTP/SES) + scheduler (cron/batch). If absent, build/secure first.
2. Persist per-(case × receiver) report-due date (compute via C1 on RE-page entry; allow manual edit).
3. Determine the case's responsible user(s) (ties into E / access scopes `access_*_ids`).
4. Daily batch: scan due dates within N-day window → email the responsible users.
5. Idempotency: record sent notifications to avoid duplicates per due.

### Files
new scheduler/job module, mail client, due-date persistence model, `report_due` wiring, config.

### Verify (needs infra)
due calc → recipient selection → send trigger (integration/E2E with mail mocked).

### Risk / size
Medium–Large. **Highest infra dependency.** Recipient rule couples to E (ACL).

---

## E1 / E2 — ACL (Sender count limit / Product-Sender权)

### Spec
- **E1 (p.20):** A company granted **Sponsor Administrator (CRO)** can register **multiple** Sender records; a company with **Sponsor Administrator (Pharmaceutical Company)** can register **only one** Sender.
- **E2 (p.27):** On a Product record, adding the **Sender** field is allowed **only for CRO-privileged companies**.

### Current state
- ACL models exist under `crates/libs/lib-core/src/model/acs/`. Need to confirm how "CRO vs Pharma" is represented (role / capability / org type).

### Plan
1. Inspect `model/acs/permission.rs` etc.: is the CRO-vs-Pharma distinction already modeled?
2. **DECISION NEEDED (business):** define how a CRO-privileged org is identified (org-type column? capability? role assignment).
3. E1: on `SenderPresaveBmc::create`, if the org is non-CRO, count existing (non-deleted) sender presaves; reject if it would exceed 1.
4. E2: on `ProductPresaveBmc::create`/`update`, allow setting `sender_presave_id` only for CRO orgs.
5. Clear 403 / policy-violation errors.

### Files
`model/presave.rs` (create guards), `model/acs/`, permission helper, tests.

### Verify (needs DB + ACL ctx)
CRO/Pharma scenarios per rule.

### Risk / size
Medium **after** the CRO-identification policy is decided. Blocked without that decision.

---

## Sequencing
```
Pre-reqs (external):
  - E: business decision on "how to identify a CRO-privileged org"
  - C4: mail + scheduler infra

Implementation order (once DB available):
  1. C3  (audit reason)   — most self-contained; reuse existing audit
  2. E1/E2 (ACL)          — after CRO policy decided; presave create guards
  3. C4  (report-due mail) — needs C1 (done) + responsible-user rule (E) + infra; last
```

## Cross-cutting notes for the implementer
- All three are runtime-verifiable only with a DB (+ mail infra for C4). Compile-check is the only static gate.
- Keep changes in the **presave layer**; never touch the per-case BMC models for INFO work.
- presave auto-CRUD is modql `Fields`-derived — adding a column = struct field + DB column + dev_db migration; no positional binds.
- Reason/category (C3) and responsible-user (C4) interact with `Ctx` (user/org) and `access_*_ids` — reuse the B3 `id_scope_contains` helper where helpful.
