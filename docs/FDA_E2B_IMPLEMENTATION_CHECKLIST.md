# FDA E2B(R3) Implementation Checklist (Draft)

Goal: Close FDA-required gaps identified in `docs/FDA_E2B_MAPPING.md` and make exports pass FDA validation.

## 1) FDA-specific required fields (DB + API + export + validator)
- [x] Add FDA.C.1.7.1 Local Criteria Report Type (DB + API + export + rules).
- [x] Add FDA.C.1.12 Combination Product Report Indicator (DB + API + export + rules).
- [x] Add FDA.D.11.r.1 Patient Race Code (DB + API + export + rules).
- [x] Add FDA.D.12 Patient Ethnicity Code (DB + API + export + rules).
- [x] Add FDA.E.i.3.2h Required Intervention (DB + API + export + rules).
- [x] Add FDA.G.k.10a Additional Information on Drug (coded) (DB + API + export + rules).

## 2) Missing REST coverage for existing DB fields
### Reaction (E)
- [ ] Expose `reactions.reaction_language`, `term_highlighted`, `criteria_disabling`,
      `criteria_congenital_anomaly`, `criteria_other_medically_important`,
      `duration_value`, `duration_unit`, `medical_confirmation`, `country_code`.

### Tests (F)
- [x] Expose `test_results.test_meddra_code`, `test_meddra_version`,
      `test_result_code`, `result_unstructured`, `normal_low_value`,
      `more_info_available`.

### Drug (G)
- [ ] Expose `drug_information.mpid/phpid` + version fields.
- [ ] Expose `drug_information.obtain_drug_country`, `manufacturer_country`,
      `rechallenge`, `dosage_text` (drug-level).
- [ ] Expose `dosage_information.dose_form_termid` + version,
      `parent_route_termid` + version (update struct).

### Patient (D)
- [x] Add REST for patient identifiers (`patient_identifiers`) with case-scoped
      single-item read/update/delete behavior.

## 3) Exporter gaps (fields stored but not emitted)
### Reaction (E)
- [x] Export `serious` (E.i.3.1) and any FDA-required intervention once modeled.

### Tests (F)
- [x] Export `test_name` (free text) when MedDRA is absent.
- [x] Export `test_result_code`, `result_unstructured`.
- [ ] Export `normal_low_value` when high and low both present (currently uses high else low).

### Drug (G)
- [x] Export `brand_name`, `batch_lot_number`.
- [x] Export drug-reaction assessment (G.k.9.i) beyond relatedness
      (time interval + reaction link + recur flag).
- [x] Export full drug recurrence details (action/MedDRA), not just `reaction_recurred`.

### Narrative (H)
- [x] Export `case_summary_information.summary_type`.
- [x] Support multiple summaries (currently first only).

## 4) FDA business rules (validator)
- [x] Implement FDA conditional-required logic from Excel (e.g., C.1.7.1, C.1.12).
- [x] Add FDA-required checks for reporter email and sender fields.
- [x] Add code-list validation for FDA-specific coded fields (race/ethnicity, intervention, etc.).

## 5) Data realism for FDA validation
- [ ] Replace placeholder values with real values (no "D.10.3", "G.k.4.r.6a", etc.).
- [ ] Ensure dates follow FDA/HL7 patterns and required elements appear in order.

---

Owner notes:
- Mapping source: `docs/FDA_E2B_MAPPING.md`.
- References live in `docs/refs/`.
