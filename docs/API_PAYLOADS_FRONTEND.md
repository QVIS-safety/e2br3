# Frontend Payloads (Request/Response Examples)

This document shows **frontend-facing JSON shapes** for each endpoint, using example payloads with placeholder values. All REST endpoints use the `{ "data": ... }` envelope for create/update, and `{ "data": ... }` for success responses.

Notes:
- For case-scoped endpoints, the server **overrides** `case_id` from the URL path even if provided in the body.
- For list endpoints, the frontend typically sends query params only.
- Delete endpoints return `204 No Content`.

---

## Auth

### POST `/auth/v1/login`
```json
{ "email": "demo.user@example.com", "pwd": "welcome" }
```
Response
```json
{ "result": { "success": true } }
```

### POST `/auth/v1/logoff`
```json
{ "logoff": true }
```
Response
```json
{ "result": { "logged_off": true } }
```

### POST `/auth/v1/refresh`
(no body)
Response
```json
{ "data": { "expiresAt": "2026-02-05T12:34:56Z" } }
```

---

## Organizations

### POST `/api/organizations`
```json
{ "data": { "name": "Acme Pharma" } }
```
Response
```json
{ "data": { "id": "org-uuid", "name": "Acme Pharma", "created_at": "..." } }
```

### PUT `/api/organizations/{id}`
```json
{ "data": { "name": "Acme Pharma (Renamed)" } }
```
Response
```json
{ "data": { "id": "org-uuid", "name": "Acme Pharma (Renamed)", "updated_at": "..." } }
```

---

## Users

### POST `/api/users`
```json
{
  "data": {
    "organization_id": "org-uuid",
    "email": "user@example.com",
    "username": "user01",
    "pwd_clear": "TempPass123!",
    "role": "user"
  }
}
```
Response
```json
{ "data": { "id": "user-uuid", "email": "user@example.com", "role": "user" } }
```

### PUT `/api/users/{id}`
```json
{ "data": { "role": "manager" } }
```
Response
```json
{ "data": { "id": "user-uuid", "role": "manager" } }
```

### GET `/api/users/me`
Response
```json
{ "data": { "id": "user-uuid", "email": "user@example.com", "role": "user" } }
```

---

## Cases

### POST `/api/cases`
```json
{ "data": { "status": "draft", "safety_report_id": "SR-123" } }
```
Response
```json
{ "data": { "id": "case-uuid", "status": "draft", "safety_report_id": "SR-123" } }
```

### PUT `/api/cases/{id}`
```json
{ "data": { "status": "validated" } }
```
Response
```json
{ "data": { "id": "case-uuid", "status": "validated" } }
```

### POST `/api/cases/intake-check`
Use this before create-from-intake. The backend now returns a duplicate result even when the duplicate basis is incomplete, and surfaces that state through `basis_complete` plus `warnings`.

```json
{
  "data": {
    "safety_report_id": "INTAKE-123",
    "date_of_most_recent_information": [2024, 120],
    "report_type": "1",
    "patient_initials": "PT123",
    "dg_prd_key": "PRODUCT-001",
    "reaction_meddra_version": "27.0",
    "reaction_meddra_code": "10019211",
    "ae_start_date": [2024, 120]
  }
}
```
Response when basis is complete and duplicates were found:
```json
{
  "data": {
    "duplicate": true,
    "basis_complete": true,
    "warnings": [],
    "matches": [
      {
        "case_id": "case-uuid",
        "safety_report_id": "INTAKE-123",
        "version": 1,
        "status": "draft",
        "created_at": "2026-04-14 10:15:00 +00:00:00",
        "report_type": "1",
        "date_of_most_recent_information": [2024, 120],
        "reporter_organization": null,
        "sponsor_study_number": null,
        "patient_initials": "PT123",
        "investigation_number": null,
        "age_d2_2a": null,
        "sex_d5": null,
        "dg_prd_key": "PRODUCT-001",
        "reaction_meddra_version": "27.0",
        "reaction_meddra_code": "10019211",
        "ae_start_date": [2024, 120]
      }
    ]
  }
}
```
Response when basis is complete but warnings are still present:
```json
{
  "data": {
    "duplicate": false,
    "basis_complete": true,
    "warnings": [
      "Reaction MedDRA version is missing from duplicate check input"
    ],
    "matches": []
  }
}
```
Response when basis is incomplete but the UI should still let the user review and continue:
```json
{
  "data": {
    "duplicate": false,
    "basis_complete": false,
    "warnings": [
      "duplicate check basis is incomplete; resubmit with allow_duplicate_override=true after review",
      "Product ID is missing from duplicate check input",
      "Reaction MedDRA version is missing from duplicate check input"
    ],
    "matches": []
  }
}
```

Frontend behavior:
- if `duplicate === true`, show matching cases and block create
- if `basis_complete === false`, show the warnings and require explicit confirmation before create
- do not treat `warnings.length > 0` as equivalent to `basis_complete === false`
- interpret `basis_complete` as “enough duplicate-basis input exists to trust the check”
- interpret `warnings` as a mixed informational list: some warnings indicate incomplete basis, while others are non-blocking missing-field notes
- treat null-flavor placeholders such as `NI`, `UNK`, `ASKU`, `NASK`, `MSK` as missing values in the intake UI as well

### POST `/api/cases/from-intake`
```json
{
  "data": {
    "safety_report_id": "INTAKE-123",
    "date_of_most_recent_information": [2024, 120],
    "report_type": "1",
    "validation_profile": "fda",
    "patient_initials": "PT123",
    "dg_prd_key": "PRODUCT-001",
    "reaction_meddra_version": "27.0",
    "reaction_meddra_code": "10019211",
    "ae_start_date": [2024, 120]
  }
}
```
Response
```json
{
  "data": {
    "case_id": "case-uuid",
    "safety_report_id": "INTAKE-123",
    "version": 1
  }
}
```

When the duplicate basis is incomplete, the frontend may resubmit with `allow_duplicate_override: true` after the user confirms:
```json
{
  "data": {
    "safety_report_id": "INTAKE-123",
    "date_of_most_recent_information": [2024, 120],
    "report_type": "1",
    "validation_profile": "fda",
    "patient_initials": "PT123",
    "allow_duplicate_override": true
  }
}
```

When duplicate matches exist, `allow_duplicate_override` does not change the result. Duplicate hits are always blocked at create-from-intake time.

Example error for duplicate hit:
```json
{
  "error": {
    "message": "SERVICE_ERROR",
    "data": {
      "detail": "duplicate case detected; create is blocked when intake check finds duplicates",
      "req_uuid": "uuid"
    }
  }
}
```

Example error for incomplete duplicate basis:
```json
{
  "error": {
    "message": "SERVICE_ERROR",
    "data": {
      "detail": "duplicate check basis is incomplete; resubmit with allow_duplicate_override=true after review",
      "req_uuid": "uuid"
    }
  }
}
```

### GET `/api/cases/{case_id}/validation?profile=mfds`
Query:
- `profile` optional: `fda` or `mfds`
- If omitted, backend infers profile from message header batch receiver (contains `MFDS` -> `mfds`, otherwise `fda`).

Response
```json
{
  "data": {
    "profile": "mfds",
    "case_id": "case-uuid",
    "ok": false,
    "blocking_count": 1,
    "non_blocking_count": 2,
    "issues": [
      {
        "code": "MFDS.C.3.1.KR.1.REQUIRED",
        "message": "MFDS requires [C.3.1.KR.1] when sender type is health professional.",
        "path": "senderInformation.senderTypeKr1",
        "section": "sender",
        "blocking": true
      },
      {
        "code": "ICH.G.k.2.2.REQUIRED",
        "message": "[G.k.2.2] is required.",
        "path": "drugs.0.medicinalProduct",
        "section": "drugs",
        "blocking": false
      }
    ]
  }
}
```

---

## Case Singletons

### POST `/api/cases/{case_id}/message-header`
```json
{
  "data": {
    "case_id": "case-uuid",
    "message_date": "20240101120000",
    "message_date_format": "204",
    "message_format_release": "2.0",
    "message_format_version": "2.1",
    "message_number": "MSG-123",
    "message_receiver_identifier": "CDER",
    "message_sender_identifier": "DSJP",
    "message_type": "ichicsr"
  }
}
```
Response
```json
{ "data": { "id": "msg-uuid", "case_id": "case-uuid" } }
```

### POST `/api/cases/{case_id}/safety-report`
```json
{
  "data": {
    "case_id": "case-uuid",
    "transmission_date": [2024, 15],
    "report_type": "1",
    "date_first_received_from_source": [2024, 10],
    "date_of_most_recent_information": [2024, 15],
    "fulfil_expedited_criteria": true
  }
}
```
Response
```json
{ "data": { "id": "safety-uuid", "case_id": "case-uuid" } }
```

### POST `/api/cases/{case_id}/patient`
```json
{ "data": { "case_id": "case-uuid", "patient_initials": "PT", "sex": "2" } }
```
Response
```json
{ "data": { "id": "patient-uuid", "case_id": "case-uuid" } }
```

### POST `/api/cases/{case_id}/receiver`
```json
{ "data": { "case_id": "case-uuid", "receiver_type": "1", "receiver_identifier": "FDA" } }
```
Response
```json
{ "data": { "id": "receiver-uuid", "case_id": "case-uuid" } }
```

### POST `/api/cases/{case_id}/narrative`
```json
{ "data": { "case_id": "case-uuid", "case_summary": "Patient experienced..." } }
```
Response
```json
{ "data": { "id": "narrative-uuid", "case_id": "case-uuid" } }
```

---

## Case Collections

### POST `/api/cases/{case_id}/reactions`
```json
{ "data": { "case_id": "case-uuid", "primary_source_reaction": "Headache", "reaction_meddra_code": "10019211" } }
```
Response
```json
{ "data": { "id": "reaction-uuid", "case_id": "case-uuid" } }
```

### POST `/api/cases/{case_id}/drugs`
```json
{ "data": { "case_id": "case-uuid", "drug_name": "Drug A", "drug_characterization": "1" } }
```
Response
```json
{ "data": { "id": "drug-uuid", "case_id": "case-uuid" } }
```

### POST `/api/cases/{case_id}/test-results`
```json
{ "data": { "case_id": "case-uuid", "test_name": "ALT", "test_result_value": "42", "test_result_unit": "U/L" } }
```
Response
```json
{ "data": { "id": "test-uuid", "case_id": "case-uuid" } }
```

### POST `/api/cases/{case_id}/other-identifiers`
```json
{ "data": { "case_id": "case-uuid", "identifier_value": "ALT-123", "identifier_type_code": "2" } }
```
Response
```json
{ "data": { "id": "other-id-uuid", "case_id": "case-uuid" } }
```

### POST `/api/cases/{case_id}/linked-reports`
```json
{ "data": { "case_id": "case-uuid", "report_number": "LR-001" } }
```
Response
```json
{ "data": { "id": "linked-uuid", "case_id": "case-uuid" } }
```

---

## Patient Sub-Resources

### POST `/api/cases/{case_id}/patient/identifiers`
```json
{ "data": { "patient_id": "patient-uuid", "identifier_value": "MRN-123", "identifier_type_code": "1" } }
```

### POST `/api/cases/{case_id}/patient/medical-history`
```json
{ "data": { "patient_id": "patient-uuid", "meddra_code": "10012345", "meddra_version": "26.0" } }
```

### POST `/api/cases/{case_id}/patient/past-drugs`
```json
{ "data": { "patient_id": "patient-uuid", "drug_name": "Drug B", "indication": "Hypertension" } }
```

### POST `/api/cases/{case_id}/patient/death-info`
```json
{ "data": { "patient_id": "patient-uuid", "date_of_death": "20240101" } }
```

### POST `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes`
```json
{ "data": { "death_information_id": "death-uuid", "meddra_code": "10020241" } }
```

### POST `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes`
```json
{ "data": { "death_information_id": "death-uuid", "meddra_code": "10020241" } }
```

### POST `/api/cases/{case_id}/patient/parents`
```json
{ "data": { "patient_id": "patient-uuid", "parent_identification": "MOTHER", "sex": "2" } }
```

### POST `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history`
```json
{ "data": { "parent_id": "parent-uuid", "meddra_code": "10012345", "meddra_version": "26.0" } }
```

### POST `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs`
```json
{ "data": { "parent_id": "parent-uuid", "drug_name": "Drug C" } }
```

---

## Narrative Sub-Resources

### POST `/api/cases/{case_id}/narrative/sender-diagnoses`
```json
{ "data": { "narrative_id": "narrative-uuid", "diagnosis": "Diabetes" } }
```

### POST `/api/cases/{case_id}/narrative/summaries`
```json
{ "data": { "narrative_id": "narrative-uuid", "summary": "Case summary text" } }
```

---

## Safety Report Sub-Resources

### POST `/api/cases/{case_id}/safety-report/senders`
```json
{ "data": { "case_id": "case-uuid", "sender_type": "1", "sender_identifier": "SPONSOR" } }
```

### POST `/api/cases/{case_id}/safety-report/primary-sources`
```json
{ "data": { "case_id": "case-uuid", "reporter_first_name": "John", "reporter_last_name": "Doe" } }
```

### POST `/api/cases/{case_id}/safety-report/literature`
```json
{ "data": { "case_id": "case-uuid", "citation": "PMID:12345678" } }
```

### POST `/api/cases/{case_id}/safety-report/studies`
```json
{ "data": { "case_id": "case-uuid", "study_name": "Study 001", "study_type": "1" } }
```

### POST `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations`
```json
{ "data": { "study_id": "study-uuid", "registration_number": "NCT01234567", "country_code": "US" } }
```

---

## Drug Sub-Resources

### POST `/api/cases/{case_id}/drugs/{drug_id}/active-substances`
```json
{ "data": { "drug_id": "drug-uuid", "substance_name": "Substance X" } }
```

### POST `/api/cases/{case_id}/drugs/{drug_id}/dosages`
```json
{ "data": { "drug_id": "drug-uuid", "dose_number": "50", "dose_unit": "mg", "route_of_administration": "ORAL" } }
```

### POST `/api/cases/{case_id}/drugs/{drug_id}/indications`
```json
{ "data": { "drug_id": "drug-uuid", "indication": "Pain" } }
```

### POST `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments`
```json
{ "data": { "drug_id": "drug-uuid", "reaction_id": "reaction-uuid", "assessment_method": "1" } }
```

### POST `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness`
```json
{ "data": { "drug_reaction_assessment_id": "assess-uuid", "relatedness": "1" } }
```

### POST `/api/cases/{case_id}/drugs/{drug_id}/recurrences`
```json
{ "data": { "drug_id": "drug-uuid", "recurrence": "1" } }
```

---

## Terminology (query params only)

### GET `/api/terminology/meddra`
```
/ api/terminology/meddra?q=headache&limit=20&version=26.0
```

### GET `/api/terminology/whodrug`
```
/ api/terminology/whodrug?q=aspirin&limit=20
```

### GET `/api/terminology/countries`
(no params)

### GET `/api/terminology/code-lists`
```
/ api/terminology/code-lists?list_name=reaction_outcome
```

---

## Import

### POST `/api/import/xml/validate`
`multipart/form-data` with `file` or `xml` field containing XML.

Response
```json
{ "data": { "ok": true, "errors": [], "root_element": "..." } }
```

### POST `/api/import/xml`
`multipart/form-data` with `file` or `xml` field containing XML.

Response
```json
{ "data": { "case_id": "case-uuid", "case_version": 1, "xml_key": null, "parsed_json_id": "version-uuid" } }
```

---

## Audit Logs

### GET `/api/audit-logs`
Query params example:
```
/ api/audit-logs?filters=[{"field":"table_name","op":"eq","value":"cases"}]&list_options={"limit":50}
```

### GET `/api/audit-logs/by-record/{table_name}/{record_id}`
(no body)
