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

### Current state — VERIFIED IMPLEMENTED
The original "no path to pass a user-entered reason" gap is **false**. End to end works:
- Reason path: auth middleware accepts `x-e2br3-reason-for-change` and stores it on `Ctx` — `crates/libs/lib-web/src/middleware/mw_auth.rs:42`.
- Category path: auth middleware accepts `x-e2br3-change-category` and stores it on `Ctx`; DB context writes it to `app.change_category`.
- Presave updates pass `Ctx` into `base_uuid::update`, which calls `set_full_context_from_ctx_dbx` before the UPDATE fires the trigger — `crates/libs/lib-core/src/model/base/base_uuid.rs:134`.
- The DB trigger computes per-field `changed_fields` and writes `reason_for_change` + `change_category` into `audit_logs`.
- Per-record audit read path exists: `AuditLogBmc::list_by_record` — `crates/libs/lib-core/src/model/audit.rs:357`.
- Existing web test for INFO presave reason/category capture: `crates/services/web-server/tests/api/presave/sender_web.rs:157`.
- `cargo test -p lib-core section_presave` → 14 passed.
- `cargo test -p web-server info_update_audit_reason_records_sender_presave_reason` → 1 targeted test passed.

### Remaining frontend work
The backend now stores category separately. The frontend should send:
- `x-e2br3-change-category`: one of Input Error / New Data / Edited Data / Others.
- `x-e2br3-reason-for-change`: the free-text reason.

The Audit Trail read view (No/Date-Time/User/Item/Value/Notation/Reason) is served by `AuditLogBmc::list_by_record` + `changed_fields`; map `change_category` and `reason_for_change` as needed in the frontend.

### Verify (needs DB)
Edit with both headers → `audit_logs` row has `reason_for_change` and `change_category`.

### Risk / size
Small. Reason/diff/read all already work; category was added as an additive column/header path.
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
