# QVIS CIOMS Phase 4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align backend CIOMS PDF output with QVIS Phase 4 requirements for same-form portrait/landscape rendering, missing item prevention, and long-content continuation.

**Architecture:** Keep the existing hand-written PDF renderer. Use the landscape official-like renderer as the single form source, render it directly for landscape, and render it through a uniform PDF transform for portrait. Add continuation behavior in the same renderer layer so long field content is preserved without changing API contracts.

**Tech Stack:** Rust, Axum web-server tests, sqlx/PostgreSQL test database, direct PDF syntax via `PdfCanvas`.

---

### Task 1: Same-Form Portrait Rendering

**Files:**
- Modify: `crates/services/web-server/src/web/rest/cioms_export_rest.rs`

- [x] **Step 1: Add a failing unit test**

Add `cioms_portrait_pdf_uses_same_official_form_as_landscape` to assert that portrait keeps portrait MediaBox but renders landscape official labels such as `8-12 CHECK ALL APPROPRIATE TO ADVERSE`, `16. ROUTE(S) OF ADMINISTRATION`, `19. THERAPY DURATION`, and `21. DID REACTION REAPPEAR AFTER`.

- [x] **Step 2: Run the focused test**

Run: `cargo test -p web-server cioms_portrait_pdf_uses_same_official_form_as_landscape -- --nocapture`

Expected before implementation: FAIL because portrait uses compact labels.

- [x] **Step 3: Add PDF transform support**

Add `PdfCanvas::save_state`, `PdfCanvas::restore_state`, and `PdfCanvas::transform` using PDF `q`, `Q`, and `cm` operators.

- [x] **Step 4: Render landscape form inside portrait page**

Add `render_landscape_cioms_on_portrait_page` and call it from `build_cioms_pdf_with_options` when `settings.orientation == "Portrait"`.

- [x] **Step 5: Verify**

Run: `cargo test -p web-server cioms_portrait_pdf_ -- --nocapture`

Expected: PASS.

### Task 2: Missing Child Data Loading

**Files:**
- Modify: `crates/services/web-server/src/web/rest/cioms_export_rest.rs`

- [x] **Step 1: Reproduce through API contract**

Run: `cargo test -p web-server test_cioms_pdf_export -- --nocapture`

Expected before fix: FAIL because persisted route, indication, therapy date, or therapy duration is absent from PDF.

- [x] **Step 2: Apply RLS context to child loaders**

Change `load_dosages_by_case` and `load_indications_by_case` to accept `ctx`, begin a transaction, call `set_full_context_from_ctx_dbx`, commit on success, and rollback on error.

- [x] **Step 3: Treat nullable soft-delete as active unless true**

Use `deleted IS NOT TRUE` for CIOMS list loaders, dosage loader joins, and indication loader joins.

- [x] **Step 4: Verify API contract**

Run: `cargo test -p web-server test_cioms_pdf_export -- --nocapture`

Expected: PASS and PDF contains route, indication, therapy date, and therapy duration.

### Task 3: Continuation Page for Long Content

**Files:**
- Modify: `crates/services/web-server/src/web/rest/cioms_export_rest.rs`

- [x] **Step 1: Add a failing unit test**

Add a test that builds CIOMS data with a narrative long enough to exceed the first-page reaction box and asserts:

```rust
assert!(text.contains("/Count 2"), "{text}");
assert!(text.contains("CIOMS CONTINUATION"), "{text}");
assert!(text.contains("final overflow marker"), "{text}");
```

- [x] **Step 2: Return overflow from wrapped rendering**

Introduce a small helper that splits text into rendered lines and overflow lines without changing PDF escaping:

```rust
struct WrappedText {
    visible: Vec<String>,
    overflow: Vec<String>,
}
```

- [x] **Step 3: Collect overflow by CIOMS box**

Update long-text boxes for reaction narrative, concomitant drugs, medical history, manufacturer address, control number, and notation to push overflow into a `Vec<(String, String)>` with the CIOMS box label and remaining text.

- [x] **Step 4: Render a continuation page**

When overflow exists, emit a second page with title `CIOMS CONTINUATION`, case number, and labeled overflow sections. Preserve portrait/landscape page size matching the requested orientation.

- [x] **Step 5: Verify focused and broad tests**

Run:

```bash
cargo test -p web-server cioms_ -- --nocapture
cargo test -p web-server test_cioms_pdf_export -- --nocapture
cargo check -p web-server --tests --keep-going
```

Expected: PASS.
