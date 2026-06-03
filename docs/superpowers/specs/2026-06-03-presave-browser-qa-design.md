# Presave Browser QA Design

Date: 2026-06-03

## Goal

Verify the priority presave workflows through the local frontend browser:

1. Product presave creation for FDA and MFDS, then import into case edit `DG/G.k`.
2. Receiver presave creation for FDA and MFDS, then use in Submission receiver routing.

Created QA records should remain in the local database for inspection. Each record will use a unique `QA-...` name and distinctive field values.

## Scope

This is a browser-first QA pass. The frontend is the source of truth for the workflow steps. API or database checks are allowed only as spot checks when the UI does not clearly show whether a value was saved, imported, or persisted.

Out of scope:

- Cleaning up created QA records.
- Refactoring presave or submission code during the design phase.
- Full XML/export correctness review unless the submission UI cannot validate receiver routing without it.

## Product Presave Flow

Create two Product INFO records:

- `QA-FDA-PRODUCT-<timestamp>`
- `QA-MFDS-PRODUCT-<timestamp>`

The FDA-oriented record should include a distinctive product ID/name, linked sender, and substance row. The MFDS-oriented record should include the same core identity pattern plus MFDS-specific product/device/coding fields exposed by the Product form.

Then test case edit import:

1. Open or create a case in an FDA/MFDS authority context.
2. Navigate to `DG/G.k`.
3. Use `Import Template`.
4. Select each QA Product record.
5. Verify the drug editor fields populate with the distinctive values.
6. Save, reload, and verify the imported values persist.

## Receiver Presave Flow

Create two Receiver INFO records:

- `QA-FDA-RECEIVER-<timestamp>`
- `QA-MFDS-RECEIVER-<timestamp>`

Each record should include a distinctive receiver organization, receiver ID, routing identifiers where the UI exposes them, and timeline values.

Then test submission routing:

1. Open the Submission page.
2. Select a case eligible for FDA/MFDS submission routing.
3. Select the QA Receiver presave for the target authority.
4. Verify routing fields derive from the selected receiver template.
5. Repeat for FDA and MFDS.

## Evidence To Report

The QA result should include:

- Unique names of all created presave records.
- Browser routes visited.
- Pass/fail result for creation, list visibility, detail mapping, import picker visibility, field population, save, reload persistence, and submission routing.
- Any failing field names and the suspected layer: creation form, list/detail mapper, import picker, case-form mapper, submission routing, or backend persistence.

## Error Handling

If a browser action fails, first capture the visible UI state and console/network evidence. Then use targeted API or database checks to determine whether the data exists and whether the frontend is reading the expected endpoint.

If a prior dirty local state blocks a clean test, keep the created QA records but use fresh timestamped names and a fresh or clearly identified case.
