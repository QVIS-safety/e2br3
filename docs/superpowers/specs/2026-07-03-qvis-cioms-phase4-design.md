# QVIS CIOMS Phase 4 Design

## Source

- `/Users/hyundonghoon/Downloads/QVIS Safety Database_UI Specification_18JUN2026_Updated.pdf`, pages 82-90.
- Relevant requirements in those pages:
  - Export CIOMS.
  - Landscape and portrait output must use the same form.
  - Output items must be corrected where they are missing or printed differently from the delivered form.
  - When content is long, it must continue on a back/continuation page instead of disappearing.

## Current Implementation

- Backend CIOMS PDF generation lives in `crates/services/web-server/src/web/rest/cioms_export_rest.rs`.
- The exporter builds PDF syntax directly with `PdfCanvas`; there is no external PDF template or fillable CIOMS form file in the local workspace.
- The landscape renderer is the official-like form renderer used by the current tests.
- Before Phase 4, portrait used a separate compact layout, so portrait and landscape did not share the same form.
- CIOMS child row loading for dosage and indication used direct SQL without RLS context and nullable soft-delete handling, so route, indication, therapy dates, and duration could be omitted even when case data existed.

## Phase 4 Scope

### Issue 12: Portrait/Landscape Same Form

Portrait must preserve a portrait PDF page size, but the form content must be the same official-like layout as landscape. The backend should render the landscape form into the portrait page with a uniform scale transform instead of maintaining a separate compact portrait form.

### Issues 13-14: Missing Items / Form Mismatch

The exporter must not drop mapped CIOMS child data because of backend loading mistakes. Dosage and indication rows must load under the same auth/RLS context as the parent case data and must treat `deleted IS NOT TRUE` as active.

Full field-by-field parity against the delivered fillable CIOMS form remains limited until that exact form file is available locally.

### Issue 15: Long Content Continuation

Current text wrapping truncates after `max_lines`. Long narrative, concomitant drugs, history, manufacturer address, and control-number content must not disappear. In this phase, the backend should add a deterministic continuation page that captures overflow text after the primary CIOMS form page.

## Non-Goals

- Do not redesign the frontend export workflow in this backend phase.
- Do not replace the hand-written PDF renderer with a new PDF crate unless the existing renderer cannot support continuation pages.
- Do not claim full regulatory CIOMS parity without the delivered fillable CIOMS reference file.

## Acceptance Criteria

- Portrait CIOMS PDF has `/MediaBox [0 0 595 842]` and contains the same official-like form labels as landscape.
- Landscape CIOMS PDF keeps `/MediaBox [0 0 842 595]`.
- API export contract includes route, indication, therapy date, and therapy duration from persisted child rows.
- Long CIOMS text that exceeds first-page boxes appears on a continuation page.
- `cargo test -p web-server cioms_ -- --nocapture` passes.
- `cargo test -p web-server test_cioms_pdf_export -- --nocapture` passes.
- `cargo check -p web-server --tests --keep-going` passes.
