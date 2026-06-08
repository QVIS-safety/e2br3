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

The new architecture keeps export history and XSD/basic validation. Receiver/business readiness must be owned by the existing case BMC validation layer before XML generation. `validate_e2b_xml_business` is not part of the new exporter architecture and must not be used as an export gate.

### Schema Evidence

The current FDA and MFDS example XML instances both use:

- root message: `MCCI_IN200100UV01`
- ICSR payload interaction: `PORR_IN049016UV`

For the first redesign scope, exporter support must target this message pair. The complete schema bundle must still be deployed because `MCCI_IN200100UV01.xsd` depends on many core and multicache schema files.

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

Required fields:

- authority: `ich`, `fda`, or `mfds`
- sender identifier used in the E2B message header
- receiver identifier used in the E2B message header
- receiver label, such as `MFDS(KR)`
- root element: `MCCI_IN200100UV01`
- case element: `PORR_IN049016UV`
- schema entrypoint: `MCCI_IN200100UV01.xsd`
- filename policy: `<receiver-label>_<batch-or-export-no>.<timestamp>.xml`
- export mode: `single_case` or `submission_batch`

The profile is selected by the export/submission workflow. It is not inferred from dirty flags.

The first implementation must reject profiles that request any root or payload other than `MCCI_IN200100UV01` and `PORR_IN049016UV`. It must not silently fall back to FDA sample XML or another schema.

### CaseSnapshotLoader

Loads the complete current saved case state for one or more case IDs.

The loader must read from persisted BMC/model tables, including the records needed by the canonical export builder:

- `cases`
- `message_headers`
- `safety_report_identification`
- sender/receiver/reporting entities used by C and N sections
- patient and parent entities used by D section
- reactions used by E section
- tests used by F section
- drugs, substances, indications, dosages, and relatedness used by G section
- narrative, sender diagnoses, literature/case summaries used by H section

The loader must not read these fields as export inputs:

- `cases.raw_xml`
- `cases.dirty_c`
- `cases.dirty_d`
- `cases.dirty_e`
- `cases.dirty_f`
- `cases.dirty_g`
- `cases.dirty_h`
- frontend form dirty state
- import patch state

If the first implementation cannot map a required persisted BMC field into the canonical model, it must return a mapping error. It must not copy missing XML from `raw_xml`.

The first implementation may use existing BMC tables and models internally, but the public exporter boundary must be a stable snapshot contract, not raw table access from XML writer code.

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

Writes full XML for the supported message pair. The first implementation supports exactly:

```xml
<MCCI_IN200100UV01 ITSVersion="XML_1.0">
  ...
  <PORR_IN049016UV>
    ...
  </PORR_IN049016UV>
</MCCI_IN200100UV01>
```

The writer must set the HL7 namespace expected by the schema and must include:

```text
xsi:schemaLocation="urn:hl7-org:v3 MCCI_IN200100UV01.xsd"
```

That `schemaLocation` value is only a hint in the XML. Runtime validation must use the deployed local schema bundle, not a schema fetched from the XML.

The writer owns XML element order and structural placement. It must not serialize arbitrary maps or rely on incidental struct field order.

Writer modules must be organized by E2B sections and XML responsibility:

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

Post-write validation must run:

- XML byte-size limit check,
- XML well-formedness parse,
- root element allow-list check,
- `ITSVersion="XML_1.0"` check when configured,
- XSD validation using `MCCI_IN200100UV01.xsd` from the local schema bundle.

Post-write validation must not run `validate_e2b_xml_business`.

Runtime schema path must be configured, for example:

```text
E2BR3_SCHEMAS_DIR=/app/schemas
```

The runtime path must contain:

```text
/app/schemas/coreschemas
/app/schemas/multicacheschemas
```

The source evidence copy may remain in `docs/exporter/schema`, but production must use a stable deployed read-only schema path.

### BMC Receiver/Business Validation

Existing case BMC validation must run before XML generation.

Pre-write validation must call the case validation engine for the selected export authority:

```text
validate_case_for_authority(ctx, mm, case_id, profile.authority)
```

For a batch export, this must run for every selected case ID before any XML artifact is generated.

The exporter must block XML generation when the returned `CaseValidationReport` contains blocking issues. The export result must preserve the authority, case ID, blocking count, and issue list needed to show the user why export was blocked.

This pre-write gate owns:

- authority-specific case readiness rules,
- required regulatory fields,
- code-list and value constraints,
- existing FDA/MFDS/ICH export checks.

The XML writer must not duplicate these rules. It must consume a case snapshot that has already passed the receiver/business gate.

The new exporter must not call `validate_e2b_xml_business`. Post-write validation is limited to XML well-formedness, supported root/message checks, and XSD compliance.

### ExportArtifact

Every successful generated XML must have an artifact record. The artifact record must store:

- artifact ID,
- export mode: `single_case` or `submission_batch`,
- case IDs included in the XML,
- authority,
- sender identifier,
- receiver identifier,
- receiver label,
- root element: `MCCI_IN200100UV01`,
- case element: `PORR_IN049016UV`,
- schema entrypoint: `MCCI_IN200100UV01.xsd`,
- batch number or export number,
- file name,
- file MIME type: `application/xml` or another explicit XML download type,
- file size in bytes,
- file content location or blob key,
- XSD/basic validation status,
- validation errors when generation failed,
- created by user ID,
- created at timestamp.

The artifact record must not store dirty flags as export criteria. If dirty flag values are kept for audit/debug, they must be explicitly marked as diagnostic metadata and must not be read by the exporter.

For single-case direct XML export, the artifact may be returned immediately. For submission-style export, the artifact must be attached to submission history and downloaded from history.

## Data Flow

### Direct Single-Case Export

```text
User requests XML export for one case
  -> choose authority/profile
  -> load complete case snapshot
  -> run BMC receiver/business validation
     -> if blocking issues exist: stop, record failed export result, do not write XML
  -> build canonical E2B model
     -> if mapping gap exists: stop, record mapping error
  -> write full XML
  -> validate XML structure against XSD/basic checks
     -> if validation fails: record failed artifact/error, do not return file
  -> record export history
  -> return XML download
```

### Submission Batch Export

```text
User selects sender and receiver
  -> create/check submission batch
  -> user selects case IDs
  -> submit batch
  -> run BMC receiver/business validation for each selected case
     -> if any selected case has blocking issues: stop, show per-case errors
  -> generate one XML artifact
     -> if XML generation or XSD validation fails: record failed batch artifact/error
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

Dirty flags may be used only for:

- deciding whether an edit form has unsaved changes,
- building a PATCH payload during a save,
- clearing edit-session state after save,
- audit/debug metadata outside exporter decisions.

Dirty flags must not be used for:

- skipping clean sections,
- patching only dirty sections,
- returning raw XML just because no section is dirty,
- treating imported XML provenance as the export source,
- selecting which E2B sections appear in XML,
- selecting which BMC tables are loaded,
- deciding whether validation runs,
- deciding whether a case is exportable.

Implementation check: changing only `cases.dirty_c` through `cases.dirty_h` on an otherwise identical case must not change exported XML bytes, except for timestamp/message-number fields intentionally generated at export time.

## Error Handling

Exporter errors must be typed and actionable:

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

Errors returned to users must avoid raw internal paths unless explicitly in debug/test mode. Internal logs may include validation details and debug artifact IDs.

## Migration Plan

### Phase 1: Add New Export Path Beside Legacy

Introduce a new exporter module without deleting the legacy path.

Target:

```text
crates/libs/lib-core/src/xml/export_v2
```

The implementation may choose a different module name only if it preserves a separate new-exporter entrypoint that does not share legacy dirty-patch code.

Add:

- `ExportProfile`
- `CaseSnapshot`
- `CanonicalE2bExportCase`
- `SchemaAwareXmlWriter`
- `ExportArtifact`

Exit criteria:

- new entrypoint accepts `ExportProfile` plus case ID(s),
- new entrypoint loads a complete snapshot without reading `raw_xml` or dirty flags,
- tests prove dirty flag changes do not alter the canonical model,
- legacy exporter remains callable only through the old entrypoint.

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

Exit criteria:

- unsupported root/payload combinations return an explicit unsupported-interaction error,
- MFDS and FDA profiles both use `MCCI_IN200100UV01` plus `PORR_IN049016UV`,
- XML writer no longer uses `docs/exporter/fda/FAERS2022Scenario1.xml` as a production skeleton,
- generated XML passes XSD/basic validation from the deployed schema path.

### Phase 3: Switch Direct Export Endpoint

Change single-case XML export to call the new full-snapshot exporter.

The old path may remain only behind an explicit compatibility flag, and that flag must be disabled by default.

Exit criteria:

- default single-case export no longer calls `try_fast_path_export`,
- default single-case export no longer calls `try_fresh_section_export`,
- default single-case export no longer calls `apply_dirty_sections_from_db`,
- default single-case export does not return `cases.raw_xml`,
- default single-case export does not call `validate_e2b_xml_business`,
- failed BMC receiver/business validation blocks XML writing before the writer is invoked.

### Phase 4: Add Submission Artifact Flow

Add or align a submission-style workflow:

- sender/receiver profile
- selected case IDs
- batch/export record
- generated XML artifact
- history row
- download endpoint

This can be implemented after the direct single-case path is stable.

Exit criteria:

- submission request stores sender, receiver, authority, selected case IDs, and batch/export number,
- batch submit validates every selected case before XML generation,
- successful submit creates an artifact and a history row,
- Data File download reads the generated artifact by artifact/file key,
- download filename follows `<receiver-label>_<batch-or-export-no>.<timestamp>.xml`,
- failed validation or XML generation creates a failed history/artifact state with inspectable errors.

### Phase 5: Retire Legacy Dirty Export

Remove or quarantine:

- `try_fast_path_export`
- `try_fresh_section_export`
- `apply_dirty_sections_from_db`
- raw XML fallback as normal export source
- FDA sample skeleton as production export base

Keep importer roundtrip tests only where they explicitly test import/export compatibility, not production exporter semantics.

Exit criteria:

- production export code has no call path that uses `cases.dirty_c` through `cases.dirty_h`,
- production export code has no call path that uses `cases.raw_xml` as the XML source,
- production export code has no call path that uses FDA examples as XML skeletons,
- legacy patch exporter tests are either removed or renamed as importer/roundtrip compatibility tests,
- all direct and submission export tests pass through the new full-snapshot exporter.

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
3. Decision required: export validation runs synchronously for all exports or moves to a job for large batches.
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
