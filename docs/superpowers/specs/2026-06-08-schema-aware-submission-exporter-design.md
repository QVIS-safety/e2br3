# Schema-Aware Submission Exporter Design

Date: 2026-06-08

## Purpose

Replace the legacy XML exporter architecture with a submission-profile-driven exporter that generates complete E2B(R3) XML from the current saved case state.

The exporter must no longer use case dirty flags, raw imported XML, or section patch state to decide what XML exists. Dirty flags may remain useful for edit-session PATCH behavior, but they are not export semantics.

## Evidence

### Local Code Evidence

The current exporter in `crates/libs/lib-core/src/xml/export.rs` still branches on `dirty_c` through `dirty_h`. It can:

- return raw imported XML when no section is dirty,
- patch only dirty sections into raw XML,
- build from an FDA sample skeleton when raw XML is absent.

That architecture couples export output to importer/edit history instead of complete current case state.

The web export path in `crates/services/web-server/src/web/rest/case_export_rest.rs` currently performs post-generation validation:

- XSD/basic validation through `validate_e2b_xml`,
- XML business validation through `validate_e2b_xml_business`,
- export history recording.

The new architecture should keep export history and XSD/basic validation. Receiver/business readiness must be owned by the existing case BMC validation layer before XML generation. `validate_e2b_xml_business` is not part of the new exporter architecture and should not be used as an export gate.

### Schema Evidence

The current FDA and MFDS example XML instances both use:

- root message: `MCCI_IN200100UV01`
- ICSR payload interaction: `PORR_IN049016UV`

For the first redesign scope, exporter support should target this message pair. The complete schema bundle must still be deployed because `MCCI_IN200100UV01.xsd` depends on many core and multicache schema files.

### CubeSafety Workflow Evidence

The CubeSafety workflow was inspected through normal logged-in browser UI behavior only. No curl, endpoint replay, or direct HTTP probing was used.

Observed flow:

1. Open `SUB > SUBMISSION`.
2. Select `Sender`.
3. Select `Receiver`, such as `MFDS(KR)`.
4. Click submission action.
5. CubeSafety calls a submission check workflow and navigates to a batch checklist route.
6. Select one or more cases into the submission batch.
7. Confirm submit.
8. CubeSafety creates a submission history row.
9. The history row exposes a Data File download button.
10. Pressing that button fetches a generated XML file and creates a browser blob download.

Observed endpoint shapes from passive browser resource timing:

- `GET /safety/1.1/sponsors/{sponsorKey}/receivers/options?Submission...`
- `POST /safety/1.1/sponsors/{sponsorKey}/submission/check...`
- `GET /safety/1.1/sponsors/{sponsorKey}/submission/cases?batchKey=...`
- `POST /safety/1.0/sponsors/{sponsorKey}/submission/submit...`
- `GET /safety/1.0/sponsors/{sponsorKey}/submission/batch-history...`
- `GET /safety/1.0/sponsors/{sponsorKey}/files/{fileKey}...`

The browser-created file name followed this shape:

```text
<receiver>_<batch-no>.<timestamp>.xml
```

This supports a submission/export job model, not a dirty-section XML patch model.

## Design Goals

- Generate XML from a complete saved case snapshot.
- Make receiver/sender/submission profile explicit.
- Support `MCCI_IN200100UV01` plus `PORR_IN049016UV` first.
- Reuse existing BMC receiver/business validation before XML generation.
- Reuse XSD/basic XML validation after XML generation.
- Keep schema files available from a deployed runtime schema directory.
- Record export/submission history and expose generated XML artifacts.
- Keep importer/edit dirty flags outside exporter semantics.

## Non-Goals

- Do not generate the entire exporter directly from XSD.
- Do not support every XSD interaction in the first pass.
- Do not remove UI dirty tracking used for edit-session PATCH/save behavior.
- Do not use raw imported XML as the canonical export source.
- Do not patch only selected XML sections during export.

## Target Architecture

```text
Export or submission request
  -> ExportProfile
  -> CaseSnapshotLoader
  -> BMC receiver/business validation
  -> CanonicalE2bExportBuilder
  -> SchemaAwareXmlWriter
  -> XSD validation
  -> ExportArtifact
  -> Export/submission history
  -> Download
```

## Components

### ExportProfile

The export profile represents the receiver/submission envelope.

Fields:

- authority: `ich`, `fda`, or `mfds`
- sender identifier
- receiver identifier
- receiver label, such as `MFDS(KR)`
- root element: `MCCI_IN200100UV01`
- case element: `PORR_IN049016UV`
- schema entrypoint: `MCCI_IN200100UV01.xsd`
- filename policy
- authority-specific export options

The profile is selected by the export/submission workflow. It is not inferred from dirty flags.

### CaseSnapshotLoader

Loads the complete current saved case state for one or more case IDs.

Rules:

- Read accepted persisted case data.
- Do not read frontend dirty state.
- Do not decide output from `dirty_c` through `dirty_h`.
- Do not return raw imported XML as the export source.

This loader can initially use existing BMC tables and models. It should provide a stable snapshot contract to the exporter.

### CanonicalE2bExportBuilder

Maps the case snapshot into an internal E2B export model.

The canonical model is not XML. It is a structured representation of the exportable case:

- message/header data
- sender and receiver data
- case identifiers
- patient
- reactions
- drugs/products
- tests
- reporters
- literature
- narrative
- authority/regional fields

Mapping gaps must produce explicit exporter errors. Silent omission is not acceptable for required or known-mapped fields.

### SchemaAwareXmlWriter

Writes full XML for the supported message pair:

```xml
<MCCI_IN200100UV01>
  ...
  <PORR_IN049016UV>
    ...
  </PORR_IN049016UV>
</MCCI_IN200100UV01>
```

The writer owns XML element order and structural placement. It must not serialize arbitrary maps or rely on incidental struct field order.

Writer modules should be organized by E2B sections and XML responsibility:

- message wrapper/header
- C safety report/header data
- D patient
- E reactions
- F tests
- G drugs/products
- H narrative/literature
- N receiver/submission envelope fields

### XSD Validation

The generated XML must be validated against the deployed schema bundle.

Runtime schema path should be configured, for example:

```text
E2BR3_SCHEMAS_DIR=/app/schemas
```

The runtime path should contain:

```text
/app/schemas/coreschemas
/app/schemas/multicacheschemas
```

The source evidence copy can remain in `docs/exporter/schema`, but production must use a stable deployed read-only schema path.

### BMC Receiver/Business Validation

Existing case BMC validation should run before XML generation.

Responsibilities:

- authority-specific case readiness rules,
- required regulatory fields,
- code-list and value constraints,
- existing FDA/MFDS/ICH export checks.

The XML writer must not duplicate these rules. It should consume a case snapshot that has already passed the receiver/business gate.

The new exporter must not call `validate_e2b_xml_business`. Post-write validation is limited to XML well-formedness, supported root/message checks, and XSD compliance.

### ExportArtifact

The exporter should return or persist a generated artifact record:

- artifact ID
- case IDs
- authority
- sender
- receiver
- batch number or export number
- file name
- file content location
- validation status
- errors and warnings
- created by
- created at

For single-case direct XML export, the artifact can be returned immediately. For submission-style export, the artifact should be attached to submission history and downloaded from history.

## Data Flow

### Direct Single-Case Export

```text
User requests XML export for one case
  -> choose authority/profile
  -> load complete case snapshot
  -> run BMC receiver/business validation
  -> build canonical E2B model
  -> write full XML
  -> validate XML structure against XSD/basic checks
  -> record export history
  -> return XML download
```

### Submission Batch Export

```text
User selects sender and receiver
  -> create/check submission batch
  -> user selects case IDs
  -> submit batch
  -> generate one XML artifact
  -> record submission history
  -> user downloads Data File from history
```

This matches the CubeSafety workflow shape.

## Dirty Flag Boundary

Dirty flags may remain in the edit pipeline as transient patch masks:

```text
form loaded
  -> user edits fields
  -> dirty fields determine PATCH payload
  -> save applies patch
  -> dirty state is cleared
```

Exporter rule:

```text
Dirty flags must not affect XML generation.
```

The exporter must not:

- skip clean sections,
- patch only dirty sections,
- return raw XML just because no section is dirty,
- treat imported XML provenance as the export source.

## Error Handling

Exporter errors should be typed and actionable:

- missing export profile,
- unsupported authority,
- unsupported message interaction,
- case snapshot load failure,
- missing required canonical field,
- mapping failure,
- XML writer failure,
- XSD validation failure,
- BMC receiver/business validation failure,
- artifact persistence failure.

Errors returned to users should avoid raw internal paths unless explicitly in debug/test mode. Internal logs may include validation details and debug artifact IDs.

## Migration Plan

### Phase 1: Add New Export Path Beside Legacy

Introduce a new exporter module without deleting the legacy path.

Target:

```text
xml/export_v2
```

or equivalent naming aligned with local conventions.

Add:

- `ExportProfile`
- `CaseSnapshot`
- `CanonicalE2bExportCase`
- `SchemaAwareXmlWriter`
- `ExportArtifact`

### Phase 2: Support MFDS/FDA Primary XML

Support:

- `MCCI_IN200100UV01`
- `PORR_IN049016UV`
- MFDS profile
- FDA profile

Use the sample instances under:

- `docs/exporter/mfds`
- `docs/exporter/fda`

as structural regression evidence.

### Phase 3: Switch Direct Export Endpoint

Change single-case XML export to call the new full-snapshot exporter.

Keep old path only behind an explicit compatibility flag if needed.

### Phase 4: Add Submission Artifact Flow

Add or align a submission-style workflow:

- sender/receiver profile
- selected case IDs
- batch/export record
- generated XML artifact
- history row
- download endpoint

This can be implemented after the direct single-case path is stable.

### Phase 5: Retire Legacy Dirty Export

Remove or quarantine:

- `try_fast_path_export`
- `try_fresh_section_export`
- `apply_dirty_sections_from_db`
- raw XML fallback as normal export source
- FDA sample skeleton as production export base

Keep importer roundtrip tests only where they explicitly test import/export compatibility, not production exporter semantics.

## Testing Strategy

### Unit Tests

- `ExportProfile` construction for FDA and MFDS.
- snapshot-to-canonical mapping for required sections.
- XML writer ordering for key MCCI/PORR areas.
- error behavior for missing required fields.

### Integration Tests

- single-case export returns XML from complete saved case state.
- exporter ignores dirty flags.
- changing a persisted case field changes exported XML even if no dirty flag is set.
- raw imported XML is not returned as the export source.
- export fails before XML writing when BMC receiver/business validation fails.
- generated XML passes XSD validation.
- generated XML export does not call `validate_e2b_xml_business`.

### Regression Tests

- FDA sample export structure remains compatible with `docs/exporter/fda`.
- MFDS sample structure remains compatible with `docs/exporter/mfds`.
- schema files resolve from runtime schema path.

### CubeSafety-Parity Tests

When submission workflow exists locally:

- select sender and receiver,
- create/check batch,
- select cases,
- submit,
- create history row,
- download XML artifact.

## Open Decisions

1. Whether direct single-case XML export remains as a first-class workflow or becomes a shortcut over the submission artifact model.
2. Whether generated XML artifacts are stored in database, filesystem/object storage, or both.
3. Whether export validation should always run synchronously or become job-based for large batches.
4. Whether old roundtrip raw XML behavior is kept only for test utilities or removed entirely.

## Acceptance Criteria

- New exporter can generate MFDS and FDA XML from complete saved case state.
- Dirty flags do not affect XML output.
- Export profile controls sender, receiver, authority, root schema, and file naming.
- Existing BMC receiver/business validation runs before XML generation.
- Generated XML validates against deployed XSD schema bundle.
- Export history records success and failure.
- Submission-style flow can create a downloadable XML artifact from selected case IDs.
- Legacy raw XML patch exporter is no longer the default production path.
