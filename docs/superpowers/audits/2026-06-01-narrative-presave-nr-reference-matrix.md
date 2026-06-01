# Narrative Presave -> NR Reference Matrix

Scope: Narrative Presave -> case NR/H narrative section.

Reference routes/evidence:

- ICH: `/en/NA/case/786994/detail/NR`
- FDA/US: `/en/US/case/786994/detail/NR`
- MFDS/KR: `/en/KR/case/786994/detail/NR`
- Captured in `docs/superpowers/specs/2026-05-30-nr-reference-ui-gap-design.md`.

Local modules compared:

- Backend presave model: `crates/libs/lib-core/src/model/presave.rs`
- Backend presave REST: `crates/services/web-server/src/web/rest/section_presave_rest.rs`
- Backend presave schema: `db/bootstrap/01-safetydb-schema.sql`
- Backend case target model/schema: `crates/libs/lib-core/src/model/narrative.rs`, `db/bootstrap/08-narrative-information.sql`
- Frontend presave form/types/mappers: `components/presave/NarrativeForm.tsx`, `lib/types/presave.ts`, `lib/schemas/presave.ts`, `lib/presave/canonicalMappers.ts`, `lib/presave/canonicalWriteMappers.ts`, `lib/hooks/usePresaveTemplates.ts`
- Frontend case target: `components/case-form/sections/SectionH.tsx`

Categories:

- `referenceImportedToCase`: reference-visible NR/H case field and local narrative presave import should populate the case section.
- `referencePreserveOnly`: local presave authoring or compatibility field that may remain on the presave but must not populate the NR/H case section.
- `localSystemOnly`: local identity, metadata, row IDs, soft delete, sequence, linkage, audit, or persistence mechanics.
- `removed`: fake, redundant, legacy, or unsupported field that should not remain in presave API/form/schema paths.

| field | reference evidence | local frontend | local backend/BMC | case target | category | action |
|---|---|---|---|---|---|---|
| narrative_presaves.id / id | No reference case field; local row identity only. | `NarrativePresaveData.id` used for edit/audit. | `NarrativePresave.id` primary key. | None. | `localSystemOnly` | Keep for identity and audit. |
| narrative_presaves.organization_id | No reference case field; tenant scope only. | Not user-editable. | `NarrativePresave.organization_id`. | None. | `localSystemOnly` | Keep. |
| narrative_presaves.name / name | Presave list identity, not NR/H case content. | Presave template name outside section import payload. | Required `NarrativePresaveForCreate.name`. | None. | `localSystemOnly` | Keep as template identity; do not import. |
| narrative_presaves.comments / comments | Presave management metadata, not NR/H case content. | Presave comments/description metadata. | `comments` on parent presave. | None. | `referencePreserveOnly` | Keep on presave; do not import. |
| narrative_presaves.deleted / deleted | No reference case field; local soft-delete state. | `deleted` flag in presave data/form. | `deleted` parent column. | None. | `localSystemOnly` | Keep for soft delete; do not import. |
| narrative_presaves.case_narrative / caseNarrative | Reference NR shows H.1 Case Narrative for ICH/FDA/MFDS. | `NarrativeForm`, canonical mapper/write mapper, and `SectionH.handleImport`. | Parent presave field exists. | `narrative.caseNarrative` / `narrative_information.case_narrative`. | `referenceImportedToCase` | Import into H.1 and prove with actual `SectionH` import test. |
| narrative_presaves.case_narrative_notation / caseNarrativeNotation | No NR/H case target in reference evidence. | `NarrativeForm` renders Notation for Auto Narrative; mappers preserve it. | Parent presave field exists. | None. | `referencePreserveOnly` | Keep as presave authoring metadata; do not import. |
| narrative_presaves.case_summary / caseSummary | Reference NR case target is repeatable H.5.r summary text, not parent-level case summary. | `NarrativePresaveData.caseSummary`, write mapper sends `case_summary`, `SectionH.handleImport` imports to first summary text. | No current parent BMC/schema field. | `caseSummaryInformation[0].summaryText` if preserved. | `removed` | Remove parent-level presave API/form/schema path; use `case_summaries[].summary_text` for H.5.r. |
| narrative_presaves.additional_information / additionalInformation | Reference NR shows Additional Information (NR_SPONSOR) for ICH/FDA/MFDS. | Types/mappers mostly present; schema/form coverage incomplete. | Missing from `NarrativePresave` parent model/schema. | `narrative.additionalInformation` / `narrative_information.additional_information`. | `referenceImportedToCase` | Add backend presave storage/DTO/schema and form/schema tests; import into NR_SPONSOR. |
| narrative_presaves.reporter_comments / reporterComments | Reference NR shows H.2 Reporter's comments. | `NarrativeForm`, canonical mapper/write mapper, and `SectionH.handleImport`. | Parent presave field exists. | `narrative.reporterComments` / `narrative_information.reporter_comments`. | `referenceImportedToCase` | Import into H.2 and prove with actual `SectionH` import test. |
| narrative_presaves.sender_comments / senderComments | Reference NR shows H.4 Sender's comments. | `NarrativeForm`, canonical mapper/write mapper, and `SectionH.handleImport`. | Parent presave field exists. | `narrative.senderComments` / `narrative_information.sender_comments`. | `referenceImportedToCase` | Import into H.4 and prove with actual `SectionH` import test. |
| narrative_presave_sender_diagnoses.id / senderDiagnoses[].id | No reference case field; child row identity only. | Child row id used for edit/audit/delete. | Child primary key. | None. | `localSystemOnly` | Keep for identity and updates. |
| narrative_presave_sender_diagnoses.narrative_presave_id | No reference case field; child parent FK only. | Not user-editable. | Child FK. | None. | `localSystemOnly` | Keep. |
| narrative_presave_sender_diagnoses.sequence_number / senderDiagnoses[].sequenceNumber | Reference H.3.r is repeatable; local order mechanic. | Child row order. | Child `sequence_number`. | `narrative.senderDiagnoses[].sequenceNumber` / `sender_diagnoses.sequence_number`. | `localSystemOnly` | Keep to preserve repeat order; import as row order. |
| narrative_presave_sender_diagnoses.diagnosis_meddra_version / senderDiagnoses[].diagnosisMeddraVersion | Reference NR shows H.3.r.1a MedDRA Version. | Child row field and mapper. | Child field exists. | `narrative.senderDiagnoses[].diagnosisMeddraVersion` / `sender_diagnoses.diagnosis_meddra_version`. | `referenceImportedToCase` | Import all non-deleted diagnosis rows. |
| narrative_presave_sender_diagnoses.diagnosis_meddra_code / senderDiagnoses[].diagnosisMeddraCode | Reference NR shows H.3.r.1b diagnosis MedDRA code. | Child row field and mapper. | Child field exists. | `narrative.senderDiagnoses[].diagnosisMeddraCode` / `sender_diagnoses.diagnosis_meddra_code`. | `referenceImportedToCase` | Import all non-deleted diagnosis rows. |
| narrative_presave_sender_diagnoses.deleted / senderDiagnoses[].deleted/_delete | No reference case field; local soft-delete state. | Child soft delete/cancel support. | Child `deleted` column. | None. | `localSystemOnly` | Keep for presave graph edits; do not import deleted rows. |
| narrative_presave_case_summaries.id / caseSummaries[].id | No reference case field; child row identity only. | Child row id in type/mappers. | Child primary key. | None. | `localSystemOnly` | Keep for identity and updates. |
| narrative_presave_case_summaries.narrative_presave_id | No reference case field; child parent FK only. | Not user-editable. | Child FK. | None. | `localSystemOnly` | Keep. |
| narrative_presave_case_summaries.sequence_number / caseSummaries[].sequenceNumber | Reference H.5.r is repeatable; local order mechanic. | Child row order in type/mappers. | Child `sequence_number`. | `caseSummaryInformation[].sequenceNumber` / `case_summary_information.sequence_number`. | `localSystemOnly` | Keep to preserve repeat order; import as row order. |
| narrative_presave_case_summaries.summary_type / caseSummaries[].summaryType | Reference NR does not show Summary Type. Existing NR spec keeps backend summary type only for backward compatibility. | Type/mappers preserve it; should not be visible/import-driving. | Child field exists. | Backend case compatibility field only. | `referencePreserveOnly` | Keep for compatibility; do not show or require for import. |
| narrative_presave_case_summaries.language_code / caseSummaries[].languageCode | Reference NR shows H.5.r.1b Language. | Type/mappers preserve it. | Child field exists. | `caseSummaryInformation[].languageCode` / `case_summary_information.language_code`. | `referenceImportedToCase` | Import all non-deleted summary rows with language/text content. |
| narrative_presave_case_summaries.summary_text / caseSummaries[].summaryText | Reference NR shows H.5.r.1a Case summary and Reporter's Comments. | Type/mappers preserve it; `caseSummary` compatibility currently duplicates it. | Child field exists. | `caseSummaryInformation[].summaryText` / `case_summary_information.summary_text`. | `referenceImportedToCase` | Import all non-deleted summary rows with language/text content. |
| narrative_presave_case_summaries.caseSummary / caseSummaries[].caseSummary | Compatibility alias for `summaryText`; no distinct reference field. | Type/mappers expose alias. | No distinct backend column. | Same as `summaryText` if accepted. | `removed` | Remove distinct alias from saved payload/form expectations; use `summaryText`. |
| narrative_presave_case_summaries.deleted / caseSummaries[].deleted/_delete | No reference case field; local soft-delete state. | Child soft delete support in types/mappers. | Child `deleted` column. | None. | `localSystemOnly` | Keep for presave graph edits; do not import deleted rows. |
| created_at / updated_at / created_by / updated_by | No reference case field; local audit mechanics. | Not editable. | Parent and child audit columns. | None. | `localSystemOnly` | Keep. |

Coverage check:

- total reference fields: 9 (`H.1`, `H.2`, `H.3.r.1a`, `H.3.r.1b`, `H.4`, `H.5.r.1a`, `H.5.r.1b`, `NR_SPONSOR`, repeatable row ordering)
- total local fields: 27 rows in this matrix
- total categorized rows: 27
- zero uncategorized fields
- zero ambiguous fields
