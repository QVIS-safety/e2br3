# API Endpoints

**Scope**: Generated from server routes and REST handlers in `crates/services/web-server/src/web` and `crates/libs/lib-rest-core`.

**Auth**
- `/auth/v1/*` handles login, logoff, and refresh. These set or clear an auth cookie.
- `/api/*` endpoints require auth via cookie middleware and check permissions per handler.

**Request/Response Conventions**
- Create body: `ParamsForCreate<T>` → JSON `{ "data": T }`.
- Update body: `ParamsForUpdate<T>` → JSON `{ "data": T }`.
- List query: `ParamsList<F>` → query params `filters` and `list_options`.
- Success envelope: `DataRestResult<T>` → JSON `{ "data": T }`.
- Delete success: `204 No Content`.
- Case-scoped create endpoints override `case_id` from the path, even if provided in body.

---

**Auth**

| Method | Endpoint | Auth | Request Body | Response Body |
|---|---|---|---|---|
| POST | `/auth/v1/login` | No | `{ "email": string, "pwd": string }` | `{ "result": { "success": true } }` |
| POST | `/auth/v1/logoff` | No | `{ "logoff": bool }` | `{ "result": { "logged_off": bool } }` |
| POST | `/auth/v1/refresh` | Yes | none | `{ "data": { "expiresAt": string(RFC3339) } }` |

---

**Organizations**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/organizations` | `Organization.List` | none (query `ParamsList<OrganizationFilter>`) | `DataRestResult<Vec<Organization>>` |
| POST | `/api/organizations` | `Organization.Create` | `ParamsForCreate<OrganizationForCreate>` | `DataRestResult<Organization>` |
| GET | `/api/organizations/{id}` | `Organization.Read` | none | `DataRestResult<Organization>` |
| PUT | `/api/organizations/{id}` | `Organization.Update` | `ParamsForUpdate<OrganizationForUpdate>` | `DataRestResult<Organization>` |
| DELETE | `/api/organizations/{id}` | `Organization.Delete` | none | `204` |

---

**Users**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/users` | `User.List` | none (query `ParamsList<UserFilter>`) | `DataRestResult<Vec<User>>` |
| POST | `/api/users` | `User.Create` | `ParamsForCreate<UserForCreate>` | `DataRestResult<User>` |
| GET | `/api/users/{id}` | `User.Read` | none | `DataRestResult<User>` |
| PUT | `/api/users/{id}` | `User.Update` | `ParamsForUpdate<UserForUpdate>` | `DataRestResult<User>` |
| DELETE | `/api/users/{id}` | `User.Delete` | none | `204` |
| GET | `/api/users/me` | Authenticated | none | `DataRestResult<User>` |

---

**Cases**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/cases` | `Case.List` | none (query `ParamsList<CaseFilter>`) | `DataRestResult<Vec<Case>>` |
| POST | `/api/cases` | `Case.Create` | `ParamsForCreate<CaseForCreate>` | `DataRestResult<Case>` |
| GET | `/api/cases/{id}` | `Case.Read` | none | `DataRestResult<Case>` |
| PUT | `/api/cases/{id}` | `Case.Update` | `ParamsForUpdate<CaseForUpdate>` | `DataRestResult<Case>` |
| DELETE | `/api/cases/{id}` | `Case.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/export/xml` | `XmlExport.Export` | optional query `profile=ich|fda|mfds` | `application/xml` body |
| GET | `/api/cases/{case_id}/versions` | `AuditLog.List` | none | `DataRestResult<Vec<CaseVersion>>` |
| GET | `/api/cases/{case_id}/validation` | `Case.Read` | optional query `profile=fda|mfds` | `DataRestResult<CaseValidationReport>` |

---

**Case Singletons**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/cases/{case_id}/patient` | `Patient.Read` | none | `DataRestResult<PatientInformation>` |
| POST | `/api/cases/{case_id}/patient` | `Patient.Create` | `ParamsForCreate<PatientInformationForCreate>` | `DataRestResult<PatientInformation>` |
| PUT | `/api/cases/{case_id}/patient` | `Patient.Update` | `ParamsForUpdate<PatientInformationForUpdate>` | `DataRestResult<PatientInformation>` |
| DELETE | `/api/cases/{case_id}/patient` | `Patient.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/message-header` | `MessageHeader.Read` | none | `DataRestResult<MessageHeader>` |
| POST | `/api/cases/{case_id}/message-header` | `MessageHeader.Create` | `ParamsForCreate<MessageHeaderForCreate>` | `DataRestResult<MessageHeader>` |
| PUT | `/api/cases/{case_id}/message-header` | `MessageHeader.Update` | `ParamsForUpdate<MessageHeaderForUpdate>` | `DataRestResult<MessageHeader>` |
| DELETE | `/api/cases/{case_id}/message-header` | `MessageHeader.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/receiver` | `Receiver.Read` | none | `DataRestResult<ReceiverInformation>` |
| POST | `/api/cases/{case_id}/receiver` | `Receiver.Create` | `ParamsForCreate<ReceiverInformationForCreate>` | `DataRestResult<ReceiverInformation>` |
| PUT | `/api/cases/{case_id}/receiver` | `Receiver.Update` | `ParamsForUpdate<ReceiverInformationForUpdate>` | `DataRestResult<ReceiverInformation>` |
| DELETE | `/api/cases/{case_id}/receiver` | `Receiver.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/safety-report` | `SafetyReport.Read` | none | `DataRestResult<SafetyReportIdentification>` |
| POST | `/api/cases/{case_id}/safety-report` | `SafetyReport.Create` | `ParamsForCreate<SafetyReportIdentificationForCreate>` | `DataRestResult<SafetyReportIdentification>` |
| PUT | `/api/cases/{case_id}/safety-report` | `SafetyReport.Update` | `ParamsForUpdate<SafetyReportIdentificationForUpdate>` | `DataRestResult<SafetyReportIdentification>` |
| DELETE | `/api/cases/{case_id}/safety-report` | `SafetyReport.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/narrative` | `Narrative.Read` | none | `DataRestResult<NarrativeInformation>` |
| POST | `/api/cases/{case_id}/narrative` | `Narrative.Create` | `ParamsForCreate<NarrativeInformationForCreate>` | `DataRestResult<NarrativeInformation>` |
| PUT | `/api/cases/{case_id}/narrative` | `Narrative.Update` | `ParamsForUpdate<NarrativeInformationForUpdate>` | `DataRestResult<NarrativeInformation>` |
| DELETE | `/api/cases/{case_id}/narrative` | `Narrative.Delete` | none | `204` |

---

**Case Collections**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/cases/{case_id}/reactions` | `Reaction.List` | none | `DataRestResult<Vec<Reaction>>` |
| POST | `/api/cases/{case_id}/reactions` | `Reaction.Create` | `ParamsForCreate<ReactionForCreate>` | `DataRestResult<Reaction>` |
| GET | `/api/cases/{case_id}/reactions/{id}` | `Reaction.Read` | none | `DataRestResult<Reaction>` |
| PUT | `/api/cases/{case_id}/reactions/{id}` | `Reaction.Update` | `ParamsForUpdate<ReactionForUpdate>` | `DataRestResult<Reaction>` |
| DELETE | `/api/cases/{case_id}/reactions/{id}` | `Reaction.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/drugs` | `Drug.List` | none | `DataRestResult<Vec<DrugInformation>>` |
| POST | `/api/cases/{case_id}/drugs` | `Drug.Create` | `ParamsForCreate<DrugInformationForCreate>` | `DataRestResult<DrugInformation>` |
| GET | `/api/cases/{case_id}/drugs/{id}` | `Drug.Read` | none | `DataRestResult<DrugInformation>` |
| PUT | `/api/cases/{case_id}/drugs/{id}` | `Drug.Update` | `ParamsForUpdate<DrugInformationForUpdate>` | `DataRestResult<DrugInformation>` |
| DELETE | `/api/cases/{case_id}/drugs/{id}` | `Drug.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/test-results` | `TestResult.List` | none | `DataRestResult<Vec<TestResult>>` |
| POST | `/api/cases/{case_id}/test-results` | `TestResult.Create` | `ParamsForCreate<TestResultForCreate>` | `DataRestResult<TestResult>` |
| GET | `/api/cases/{case_id}/test-results/{id}` | `TestResult.Read` | none | `DataRestResult<TestResult>` |
| PUT | `/api/cases/{case_id}/test-results/{id}` | `TestResult.Update` | `ParamsForUpdate<TestResultForUpdate>` | `DataRestResult<TestResult>` |
| DELETE | `/api/cases/{case_id}/test-results/{id}` | `TestResult.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/other-identifiers` | `CaseIdentifier.List` | none | `DataRestResult<Vec<OtherCaseIdentifier>>` |
| POST | `/api/cases/{case_id}/other-identifiers` | `CaseIdentifier.Create` | `ParamsForCreate<OtherCaseIdentifierForCreate>` | `DataRestResult<OtherCaseIdentifier>` |
| GET | `/api/cases/{case_id}/other-identifiers/{id}` | `CaseIdentifier.Read` | none | `DataRestResult<OtherCaseIdentifier>` |
| PUT | `/api/cases/{case_id}/other-identifiers/{id}` | `CaseIdentifier.Update` | `ParamsForUpdate<OtherCaseIdentifierForUpdate>` | `DataRestResult<OtherCaseIdentifier>` |
| DELETE | `/api/cases/{case_id}/other-identifiers/{id}` | `CaseIdentifier.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/linked-reports` | `CaseIdentifier.List` | none | `DataRestResult<Vec<LinkedReportNumber>>` |
| POST | `/api/cases/{case_id}/linked-reports` | `CaseIdentifier.Create` | `ParamsForCreate<LinkedReportNumberForCreate>` | `DataRestResult<LinkedReportNumber>` |
| GET | `/api/cases/{case_id}/linked-reports/{id}` | `CaseIdentifier.Read` | none | `DataRestResult<LinkedReportNumber>` |
| PUT | `/api/cases/{case_id}/linked-reports/{id}` | `CaseIdentifier.Update` | `ParamsForUpdate<LinkedReportNumberForUpdate>` | `DataRestResult<LinkedReportNumber>` |
| DELETE | `/api/cases/{case_id}/linked-reports/{id}` | `CaseIdentifier.Delete` | none | `204` |

---

**Patient Sub-Resources**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/cases/{case_id}/patient/identifiers` | `PatientIdentifier.List` | none | `DataRestResult<Vec<PatientIdentifier>>` |
| POST | `/api/cases/{case_id}/patient/identifiers` | `PatientIdentifier.Create` | `ParamsForCreate<PatientIdentifierForCreate>` | `DataRestResult<PatientIdentifier>` |
| GET | `/api/cases/{case_id}/patient/identifiers/{id}` | `PatientIdentifier.Read` | none | `DataRestResult<PatientIdentifier>` |
| PUT | `/api/cases/{case_id}/patient/identifiers/{id}` | `PatientIdentifier.Update` | `ParamsForUpdate<PatientIdentifierForUpdate>` | `DataRestResult<PatientIdentifier>` |
| DELETE | `/api/cases/{case_id}/patient/identifiers/{id}` | `PatientIdentifier.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/patient/medical-history` | `MedicalHistory.List` | none | `DataRestResult<Vec<MedicalHistoryEpisode>>` |
| POST | `/api/cases/{case_id}/patient/medical-history` | `MedicalHistory.Create` | `ParamsForCreate<MedicalHistoryEpisodeForCreate>` | `DataRestResult<MedicalHistoryEpisode>` |
| GET | `/api/cases/{case_id}/patient/medical-history/{id}` | `MedicalHistory.Read` | none | `DataRestResult<MedicalHistoryEpisode>` |
| PUT | `/api/cases/{case_id}/patient/medical-history/{id}` | `MedicalHistory.Update` | `ParamsForUpdate<MedicalHistoryEpisodeForUpdate>` | `DataRestResult<MedicalHistoryEpisode>` |
| DELETE | `/api/cases/{case_id}/patient/medical-history/{id}` | `MedicalHistory.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/patient/past-drugs` | `PastDrug.List` | none | `DataRestResult<Vec<PastDrugHistory>>` |
| POST | `/api/cases/{case_id}/patient/past-drugs` | `PastDrug.Create` | `ParamsForCreate<PastDrugHistoryForCreate>` | `DataRestResult<PastDrugHistory>` |
| GET | `/api/cases/{case_id}/patient/past-drugs/{id}` | `PastDrug.Read` | none | `DataRestResult<PastDrugHistory>` |
| PUT | `/api/cases/{case_id}/patient/past-drugs/{id}` | `PastDrug.Update` | `ParamsForUpdate<PastDrugHistoryForUpdate>` | `DataRestResult<PastDrugHistory>` |
| DELETE | `/api/cases/{case_id}/patient/past-drugs/{id}` | `PastDrug.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/patient/death-info` | `PatientDeath.List` | none | `DataRestResult<Vec<PatientDeathInformation>>` |
| POST | `/api/cases/{case_id}/patient/death-info` | `PatientDeath.Create` | `ParamsForCreate<PatientDeathInformationForCreate>` | `DataRestResult<PatientDeathInformation>` |
| GET | `/api/cases/{case_id}/patient/death-info/{id}` | `PatientDeath.Read` | none | `DataRestResult<PatientDeathInformation>` |
| PUT | `/api/cases/{case_id}/patient/death-info/{id}` | `PatientDeath.Update` | `ParamsForUpdate<PatientDeathInformationForUpdate>` | `DataRestResult<PatientDeathInformation>` |
| DELETE | `/api/cases/{case_id}/patient/death-info/{id}` | `PatientDeath.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes` | `DeathCause.List` | none | `DataRestResult<Vec<ReportedCauseOfDeath>>` |
| POST | `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes` | `DeathCause.Create` | `ParamsForCreate<ReportedCauseOfDeathForCreate>` | `DataRestResult<ReportedCauseOfDeath>` |
| GET | `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}` | `DeathCause.Read` | none | `DataRestResult<ReportedCauseOfDeath>` |
| PUT | `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}` | `DeathCause.Update` | `ParamsForUpdate<ReportedCauseOfDeathForUpdate>` | `DataRestResult<ReportedCauseOfDeath>` |
| DELETE | `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}` | `DeathCause.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes` | `DeathCause.List` | none | `DataRestResult<Vec<AutopsyCauseOfDeath>>` |
| POST | `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes` | `DeathCause.Create` | `ParamsForCreate<AutopsyCauseOfDeathForCreate>` | `DataRestResult<AutopsyCauseOfDeath>` |
| GET | `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}` | `DeathCause.Read` | none | `DataRestResult<AutopsyCauseOfDeath>` |
| PUT | `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}` | `DeathCause.Update` | `ParamsForUpdate<AutopsyCauseOfDeathForUpdate>` | `DataRestResult<AutopsyCauseOfDeath>` |
| DELETE | `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}` | `DeathCause.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/patient/parents` | `ParentInformation.List` | none | `DataRestResult<Vec<ParentInformation>>` |
| POST | `/api/cases/{case_id}/patient/parents` | `ParentInformation.Create` | `ParamsForCreate<ParentInformationForCreate>` | `DataRestResult<ParentInformation>` |
| GET | `/api/cases/{case_id}/patient/parents/{id}` | `ParentInformation.Read` | none | `DataRestResult<ParentInformation>` |
| PUT | `/api/cases/{case_id}/patient/parents/{id}` | `ParentInformation.Update` | `ParamsForUpdate<ParentInformationForUpdate>` | `DataRestResult<ParentInformation>` |
| DELETE | `/api/cases/{case_id}/patient/parents/{id}` | `ParentInformation.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history` | `ParentMedicalHistory.List` | none | `DataRestResult<Vec<ParentMedicalHistory>>` |
| POST | `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history` | `ParentMedicalHistory.Create` | `ParamsForCreate<ParentMedicalHistoryForCreate>` | `DataRestResult<ParentMedicalHistory>` |
| GET | `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}` | `ParentMedicalHistory.Read` | none | `DataRestResult<ParentMedicalHistory>` |
| PUT | `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}` | `ParentMedicalHistory.Update` | `ParamsForUpdate<ParentMedicalHistoryForUpdate>` | `DataRestResult<ParentMedicalHistory>` |
| DELETE | `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}` | `ParentMedicalHistory.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs` | `ParentPastDrug.List` | none | `DataRestResult<Vec<ParentPastDrugHistory>>` |
| POST | `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs` | `ParentPastDrug.Create` | `ParamsForCreate<ParentPastDrugHistoryForCreate>` | `DataRestResult<ParentPastDrugHistory>` |
| GET | `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}` | `ParentPastDrug.Read` | none | `DataRestResult<ParentPastDrugHistory>` |
| PUT | `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}` | `ParentPastDrug.Update` | `ParamsForUpdate<ParentPastDrugHistoryForUpdate>` | `DataRestResult<ParentPastDrugHistory>` |
| DELETE | `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}` | `ParentPastDrug.Delete` | none | `204` |

---

**Narrative Sub-Resources**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/cases/{case_id}/narrative/sender-diagnoses` | `SenderDiagnosis.List` | none | `DataRestResult<Vec<SenderDiagnosis>>` |
| POST | `/api/cases/{case_id}/narrative/sender-diagnoses` | `SenderDiagnosis.Create` | `ParamsForCreate<SenderDiagnosisForCreate>` | `DataRestResult<SenderDiagnosis>` |
| GET | `/api/cases/{case_id}/narrative/sender-diagnoses/{id}` | `SenderDiagnosis.Read` | none | `DataRestResult<SenderDiagnosis>` |
| PUT | `/api/cases/{case_id}/narrative/sender-diagnoses/{id}` | `SenderDiagnosis.Update` | `ParamsForUpdate<SenderDiagnosisForUpdate>` | `DataRestResult<SenderDiagnosis>` |
| DELETE | `/api/cases/{case_id}/narrative/sender-diagnoses/{id}` | `SenderDiagnosis.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/narrative/summaries` | `CaseSummary.List` | none | `DataRestResult<Vec<CaseSummaryInformation>>` |
| POST | `/api/cases/{case_id}/narrative/summaries` | `CaseSummary.Create` | `ParamsForCreate<CaseSummaryInformationForCreate>` | `DataRestResult<CaseSummaryInformation>` |
| GET | `/api/cases/{case_id}/narrative/summaries/{id}` | `CaseSummary.Read` | none | `DataRestResult<CaseSummaryInformation>` |
| PUT | `/api/cases/{case_id}/narrative/summaries/{id}` | `CaseSummary.Update` | `ParamsForUpdate<CaseSummaryInformationForUpdate>` | `DataRestResult<CaseSummaryInformation>` |
| DELETE | `/api/cases/{case_id}/narrative/summaries/{id}` | `CaseSummary.Delete` | none | `204` |

---

**Safety Report Sub-Resources**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/cases/{case_id}/safety-report/senders` | `SenderInformation.List` | none | `DataRestResult<Vec<SenderInformation>>` |
| POST | `/api/cases/{case_id}/safety-report/senders` | `SenderInformation.Create` | `ParamsForCreate<SenderInformationForCreate>` | `DataRestResult<SenderInformation>` |
| GET | `/api/cases/{case_id}/safety-report/senders/{id}` | `SenderInformation.Read` | none | `DataRestResult<SenderInformation>` |
| PUT | `/api/cases/{case_id}/safety-report/senders/{id}` | `SenderInformation.Update` | `ParamsForUpdate<SenderInformationForUpdate>` | `DataRestResult<SenderInformation>` |
| DELETE | `/api/cases/{case_id}/safety-report/senders/{id}` | `SenderInformation.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/safety-report/primary-sources` | `PrimarySource.List` | none | `DataRestResult<Vec<PrimarySource>>` |
| POST | `/api/cases/{case_id}/safety-report/primary-sources` | `PrimarySource.Create` | `ParamsForCreate<PrimarySourceForCreate>` | `DataRestResult<PrimarySource>` |
| GET | `/api/cases/{case_id}/safety-report/primary-sources/{id}` | `PrimarySource.Read` | none | `DataRestResult<PrimarySource>` |
| PUT | `/api/cases/{case_id}/safety-report/primary-sources/{id}` | `PrimarySource.Update` | `ParamsForUpdate<PrimarySourceForUpdate>` | `DataRestResult<PrimarySource>` |
| DELETE | `/api/cases/{case_id}/safety-report/primary-sources/{id}` | `PrimarySource.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/safety-report/literature` | `LiteratureReference.List` | none | `DataRestResult<Vec<LiteratureReference>>` |
| POST | `/api/cases/{case_id}/safety-report/literature` | `LiteratureReference.Create` | `ParamsForCreate<LiteratureReferenceForCreate>` | `DataRestResult<LiteratureReference>` |
| GET | `/api/cases/{case_id}/safety-report/literature/{id}` | `LiteratureReference.Read` | none | `DataRestResult<LiteratureReference>` |
| PUT | `/api/cases/{case_id}/safety-report/literature/{id}` | `LiteratureReference.Update` | `ParamsForUpdate<LiteratureReferenceForUpdate>` | `DataRestResult<LiteratureReference>` |
| DELETE | `/api/cases/{case_id}/safety-report/literature/{id}` | `LiteratureReference.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/safety-report/studies` | `StudyInformation.List` | none | `DataRestResult<Vec<StudyInformation>>` |
| POST | `/api/cases/{case_id}/safety-report/studies` | `StudyInformation.Create` | `ParamsForCreate<StudyInformationForCreate>` | `DataRestResult<StudyInformation>` |
| GET | `/api/cases/{case_id}/safety-report/studies/{id}` | `StudyInformation.Read` | none | `DataRestResult<StudyInformation>` |
| PUT | `/api/cases/{case_id}/safety-report/studies/{id}` | `StudyInformation.Update` | `ParamsForUpdate<StudyInformationForUpdate>` | `DataRestResult<StudyInformation>` |
| DELETE | `/api/cases/{case_id}/safety-report/studies/{id}` | `StudyInformation.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations` | `StudyRegistration.List` | none | `DataRestResult<Vec<StudyRegistrationNumber>>` |
| POST | `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations` | `StudyRegistration.Create` | `ParamsForCreate<StudyRegistrationNumberForCreate>` | `DataRestResult<StudyRegistrationNumber>` |
| GET | `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}` | `StudyRegistration.Read` | none | `DataRestResult<StudyRegistrationNumber>` |
| PUT | `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}` | `StudyRegistration.Update` | `ParamsForUpdate<StudyRegistrationNumberForUpdate>` | `DataRestResult<StudyRegistrationNumber>` |
| DELETE | `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}` | `StudyRegistration.Delete` | none | `204` |

---

**Drug Sub-Resources**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/cases/{case_id}/drugs/{drug_id}/active-substances` | `DrugSubstance.List` | none | `DataRestResult<Vec<DrugActiveSubstance>>` |
| POST | `/api/cases/{case_id}/drugs/{drug_id}/active-substances` | `DrugSubstance.Create` | `ParamsForCreate<DrugActiveSubstanceForCreate>` | `DataRestResult<DrugActiveSubstance>` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/active-substances/{id}` | `DrugSubstance.Read` | none | `DataRestResult<DrugActiveSubstance>` |
| PUT | `/api/cases/{case_id}/drugs/{drug_id}/active-substances/{id}` | `DrugSubstance.Update` | `ParamsForUpdate<DrugActiveSubstanceForUpdate>` | `DataRestResult<DrugActiveSubstance>` |
| DELETE | `/api/cases/{case_id}/drugs/{drug_id}/active-substances/{id}` | `DrugSubstance.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/dosages` | `DrugDosage.List` | none | `DataRestResult<Vec<DosageInformation>>` |
| POST | `/api/cases/{case_id}/drugs/{drug_id}/dosages` | `DrugDosage.Create` | `ParamsForCreate<DosageInformationForCreate>` | `DataRestResult<DosageInformation>` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/dosages/{id}` | `DrugDosage.Read` | none | `DataRestResult<DosageInformation>` |
| PUT | `/api/cases/{case_id}/drugs/{drug_id}/dosages/{id}` | `DrugDosage.Update` | `ParamsForUpdate<DosageInformationForUpdate>` | `DataRestResult<DosageInformation>` |
| DELETE | `/api/cases/{case_id}/drugs/{drug_id}/dosages/{id}` | `DrugDosage.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/indications` | `DrugIndication.List` | none | `DataRestResult<Vec<DrugIndication>>` |
| POST | `/api/cases/{case_id}/drugs/{drug_id}/indications` | `DrugIndication.Create` | `ParamsForCreate<DrugIndicationForCreate>` | `DataRestResult<DrugIndication>` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/indications/{id}` | `DrugIndication.Read` | none | `DataRestResult<DrugIndication>` |
| PUT | `/api/cases/{case_id}/drugs/{drug_id}/indications/{id}` | `DrugIndication.Update` | `ParamsForUpdate<DrugIndicationForUpdate>` | `DataRestResult<DrugIndication>` |
| DELETE | `/api/cases/{case_id}/drugs/{drug_id}/indications/{id}` | `DrugIndication.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments` | `DrugReactionAssessment.List` | none | `DataRestResult<Vec<DrugReactionAssessment>>` |
| POST | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments` | `DrugReactionAssessment.Create` | `ParamsForCreate<DrugReactionAssessmentForCreate>` | `DataRestResult<DrugReactionAssessment>` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{id}` | `DrugReactionAssessment.Read` | none | `DataRestResult<DrugReactionAssessment>` |
| PUT | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{id}` | `DrugReactionAssessment.Update` | `ParamsForUpdate<DrugReactionAssessmentForUpdate>` | `DataRestResult<DrugReactionAssessment>` |
| DELETE | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{id}` | `DrugReactionAssessment.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness` | `RelatednessAssessment.List` | none | `DataRestResult<Vec<RelatednessAssessment>>` |
| POST | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness` | `RelatednessAssessment.Create` | `ParamsForCreate<RelatednessAssessmentForCreate>` | `DataRestResult<RelatednessAssessment>` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}` | `RelatednessAssessment.Read` | none | `DataRestResult<RelatednessAssessment>` |
| PUT | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}` | `RelatednessAssessment.Update` | `ParamsForUpdate<RelatednessAssessmentForUpdate>` | `DataRestResult<RelatednessAssessment>` |
| DELETE | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}` | `RelatednessAssessment.Delete` | none | `204` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/recurrences` | `DrugRecurrence.List` | none | `DataRestResult<Vec<DrugRecurrenceInformation>>` |
| POST | `/api/cases/{case_id}/drugs/{drug_id}/recurrences` | `DrugRecurrence.Create` | `ParamsForCreate<DrugRecurrenceInformationForCreate>` | `DataRestResult<DrugRecurrenceInformation>` |
| GET | `/api/cases/{case_id}/drugs/{drug_id}/recurrences/{id}` | `DrugRecurrence.Read` | none | `DataRestResult<DrugRecurrenceInformation>` |
| PUT | `/api/cases/{case_id}/drugs/{drug_id}/recurrences/{id}` | `DrugRecurrence.Update` | `ParamsForUpdate<DrugRecurrenceInformationForUpdate>` | `DataRestResult<DrugRecurrenceInformation>` |
| DELETE | `/api/cases/{case_id}/drugs/{drug_id}/recurrences/{id}` | `DrugRecurrence.Delete` | none | `204` |

---

**Terminology**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/terminology/meddra?q={term}&limit={count}&version={version}` | `Terminology.Read` | none | `DataRestResult<Vec<MeddraTerm>>` |
| GET | `/api/terminology/whodrug?q={term}&limit={count}` | `Terminology.Read` | none | `DataRestResult<Vec<WhodrugProduct>>` |
| POST | `/api/terminology/import/meddra?version={ver}&language={lang}&dry_run={bool}` | `Terminology.Import` | `multipart/form-data` field `file` (zip with `llt.asc`,`mdhier.asc`) | `DataRestResult<TerminologyImportResult>` (`dry_run=false` stages release as `validated`) |
| POST | `/api/terminology/import/whodrug?version={ver}&language={lang}&dry_run={bool}` | `Terminology.Import` | `multipart/form-data` field `file` (csv/tsv/txt or zip containing one) | `DataRestResult<TerminologyImportResult>` (`dry_run=false` stages release as `validated`) |
| GET | `/api/terminology/releases?dictionary={dict}&language={lang}` | `Terminology.Read` | none | `DataRestResult<Vec<TerminologyReleaseRow>>` |
| POST | `/api/terminology/releases/{dictionary}/{version}/approve?language={lang}&note={text}` | `Terminology.Approve` | none | `DataRestResult<TerminologyReleaseRow>` |
| POST | `/api/terminology/releases/{dictionary}/{version}/activate?language={lang}` | `Terminology.Approve` | none | `DataRestResult<TerminologyReleaseRow>` |
| POST | `/api/terminology/releases/{dictionary}/{version}/rollback?language={lang}` | `Terminology.Approve` | none | `DataRestResult<TerminologyReleaseRow>` |
| GET | `/api/terminology/countries` | `Terminology.Read` | none | `DataRestResult<Vec<IsoCountry>>` |
| GET | `/api/terminology/code-lists?list_name={name}` | `Terminology.Read` | none | `DataRestResult<Vec<E2bCodeList>>` |

---

**Import/Export**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| POST | `/api/import/xml/validate` | `XmlImport.Import` | `multipart/form-data` field `file` or `xml` | `DataRestResult<XmlValidationReport>` |
| POST | `/api/import/xml` | `XmlImport.Import` | `multipart/form-data` field `file` or `xml` | `DataRestResult<XmlImportResult>` |

---

**Audit Logs**

| Method | Endpoint | Permission | Request Body | Response Body |
|---|---|---|---|---|
| GET | `/api/audit-logs` | `AuditLog.List` | none (query `ParamsList<AuditLogFilter>`) | `DataRestResult<Vec<AuditLog>>` |
| GET | `/api/audit-logs/by-record/{table_name}/{record_id}` | `AuditLog.List` | none | `DataRestResult<Vec<AuditLog>>` |
