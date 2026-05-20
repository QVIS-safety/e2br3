# E2BR3 SafetyDB Production Readiness Plan

Complete roadmap for bringing the E2BR3 SafetyDB system (Next.js frontend + Rust backend) to production.

---

## Project Status

### Backend (Rust/Axum) - ✅ FEATURE-COMPLETE (INTEGRATION IN PROGRESS)

The backend APIs are implemented and stable for frontend integration. Ongoing work is mostly export fidelity and validator alignment.

**Completed Features:**
- Axum web framework with full REST API
- PostgreSQL database with E2B(R3) entities + audit + terminology tables
- Cookie-based auth (httpOnly cookie)
- RBAC permission system (4 roles: admin, manager, user, viewer)
- Row-Level Security (RLS) for organization isolation
- Complete audit trail with database triggers
- Full CRUD APIs for E2B(R3) entities and nested resources
- XML import/validate/export APIs
- Terminology search APIs (MedDRA, WHODrug, Countries, Code Lists)

### Frontend (Next.js 15) - ⏳ INTEGRATION IN PROGRESS

**Completed:**
- Next.js 15.5.9 with React 19 (upgraded from 14.2.5)
- Multi-step form wizard with all 8 E2B(R3) sections
- 7 reusable form components (production-ready)
- Complete type system (all 210 E2B(R3) data elements)
- Zod validation schemas for all form sections
- Proper E2B(R3) field numbering (C.1.1, D.1, E.i.1, etc.)
- Authentication flow with backend integration (login, logout, token refresh, 401 interceptor)
- Protected routes with automatic redirect
- API client connected to Rust backend (basic case flows)

**Currently Working On:**
- Connect form sections to backend subresource APIs

**Remaining Work:**
- Connect terminology autocompletes to backend
- Array field architecture (multiple reactions/drugs/tests/diagnoses)
- XML processing (import/export UX)
- Production hardening & deployment

---

## API Reference (For Frontend Integration)

**Base URL**: `http://localhost:8080`

**Response Shape**
```json
{ "data": { ... } }       // Single item
{ "data": [ ... ] }       // List
{ "result": { ... } }     // Auth endpoints
```

**Error Shape**
```json
{
  "error": {
    "message": "SERVICE_ERROR",
    "data": { "detail": "...", "req_uuid": "uuid" }
  }
}
```

### Authentication APIs

| Method | Endpoint | Description | Auth Required |
|--------|----------|-------------|---------------|
| Method | Endpoint | Description | Auth Required |
|--------|----------|-------------|---------------|
| `POST` | `/auth/v1/login` | Login with email/password | No |
| `POST` | `/auth/v1/logoff` | Logout and clear session | Yes |
| `POST` | `/auth/v1/refresh` | Refresh authentication token | Yes |

#### `POST /auth/v1/login`
```json
// Request
{ "email": "user@example.com", "pwd": "password123" }

// Response (200 OK) - Sets httpOnly cookie
{ "result": { "success": true } }
```

#### `POST /auth/v1/logoff`
```json
// Request
{ "logoff": true }

// Response (200 OK)
{ "result": { "logged_off": true } }
```

#### `POST /auth/v1/refresh`
```json
// Response (200 OK) - Refreshes httpOnly cookie
{ "data": { "expiresAt": "2026-01-26T15:30:00Z" } }
```

---

### User APIs

| Method | Endpoint | Description | Permission |
|--------|----------|-------------|------------|
| `GET` | `/api/users/me` | Get current user profile | Any authenticated |
| `GET` | `/api/users` | List users | User.List |
| `GET` | `/api/users/{id}` | Get user by ID | User.Read |
| `POST` | `/api/users` | Create user | User.Create (admin) |
| `PUT` | `/api/users/{id}` | Update user | User.Update (admin) |
| `DELETE` | `/api/users/{id}` | Delete user | User.Delete (admin) |

```json
// Response format
{
  "data": {
    "id": "uuid",
    "email": "user@example.com",
    "username": "johndoe",
    "role": "user",
    "organization_id": "uuid"
  }
}
```

---

### Case APIs (E2B(R3) Safety Reports)

| Method | Endpoint | Description | Permission |
|--------|----------|-------------|------------|
| `GET` | `/api/cases` | List cases | Case.List |
| `GET` | `/api/cases/{id}` | Get case | Case.Read |
| `POST` | `/api/cases` | Create case | Case.Create |
| `PUT` | `/api/cases/{id}` | Update case | Case.Update |
| `DELETE` | `/api/cases/{id}` | Delete case | Case.Delete (manager+) |

**Query Parameters:**
```
?filters[status]=draft
&list_options[limit]=20
&list_options[offset]=0
&list_options[order_by]=ctime
&list_options[order_dir]=desc
```

---

### Case Subresource APIs

All nested under `/api/cases/{case_id}/...`

| Resource | Type | Endpoint | E2B Section |
|----------|------|----------|-------------|
| Patient | Singleton | `/patient` | D |
| Reactions | Collection | `/reactions` | E |
| Drugs | Collection | `/drugs` | G |
| Test Results | Collection | `/test-results` | F |
| Narrative | Singleton | `/narrative` | H |
| Message Header | Singleton | `/message-header` | N |
| Safety Report | Singleton | `/safety-report` | C |
| Receiver | Singleton | `/receiver` | A |
| Other Identifiers | Collection | `/other-identifiers` | C.1.9.r |
| Linked Reports | Collection | `/linked-reports` | C.1.10.r |
| Case Versions | Read-only | `/versions` | Audit |

**Drug Sub-resources** (nested under `/drugs/{drug_id}/...`):
| Resource | Endpoint | E2B Section |
|----------|----------|-------------|
| Active Substances | `/active-substances` | G.k.2.3.r |
| Dosages | `/dosages` | G.k.4.r |
| Indications | `/indications` | G.k.6.r |
| Reaction Assessments | `/reaction-assessments` | G.k.9.i |
| Relatedness Assessments | `/reaction-assessments/{assessment_id}/relatedness` | G.k.9.i.2.r |
| Recurrences | `/recurrences` | G.k.8.r |

**Patient Sub-resources**:
| Resource | Endpoint | E2B Section |
|----------|----------|-------------|
| Patient Identifiers | `/patient/identifiers` | D.1.1.x |
| Medical History Episodes | `/patient/medical-history` | D.7.1.r |
| Past Drug History | `/patient/past-drugs` | D.8.r |
| Death Info | `/patient/death-info` | D.9 |
| Reported Causes | `/patient/death-info/{death_info_id}/reported-causes` | D.9.2.r |
| Autopsy Causes | `/patient/death-info/{death_info_id}/autopsy-causes` | D.9.4.r |
| Parents | `/patient/parents` | D.10 |

**Parent Sub-resources** (nested under `/patient/parent/{parent_id}/...`):
| Resource | Endpoint | E2B Section |
|----------|----------|-------------|
| Medical History | `/medical-history` | D.10.7.1.r |
| Past Drug History | `/past-drugs` | D.10.8.r |

**Safety Report Sub-resources** (nested under `/safety-report/...`):
| Resource | Endpoint | E2B Section |
|----------|----------|-------------|
| Senders | `/safety-report/senders` | C.3 |
| Primary Sources | `/safety-report/primary-sources` | C.2.r |
| Literature References | `/safety-report/literature` | C.4.r |
| Studies | `/safety-report/studies` | C.5 |
| Study Registrations | `/safety-report/studies/{study_id}/registrations` | C.5.1.r |

**CRUD Pattern for Collections:**
- `GET /api/cases/{case_id}/{resource}` - List
- `GET /api/cases/{case_id}/{resource}/{id}` - Get one
- `POST /api/cases/{case_id}/{resource}` - Create
- `PUT /api/cases/{case_id}/{resource}/{id}` - Update
- `DELETE /api/cases/{case_id}/{resource}/{id}` - Delete

**CRUD Pattern for Singletons:**
- `GET /api/cases/{case_id}/{resource}` - Get
- `POST /api/cases/{case_id}/{resource}` - Create
- `PUT /api/cases/{case_id}/{resource}` - Update
- `DELETE /api/cases/{case_id}/{resource}` - Delete

---

### Organization APIs

| Method | Endpoint | Description | Permission |
|--------|----------|-------------|------------|
| `GET` | `/api/organizations` | List | Org.List |
| `GET` | `/api/organizations/{id}` | Get | Org.Read |
| `POST` | `/api/organizations` | Create | Org.Create (admin) |
| `PUT` | `/api/organizations/{id}` | Update | Org.Update (admin) |
| `DELETE` | `/api/organizations/{id}` | Delete | Org.Delete (admin) |

---

### Terminology APIs

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/terminology/meddra?q={query}` | Search MedDRA terms |
| `GET` | `/api/terminology/whodrug?q={query}` | Search WHODrug products |
| `GET` | `/api/terminology/countries` | List ISO country codes |
| `GET` | `/api/terminology/code-lists?list={name}` | Get E2B(R3) code list |

**Available Code Lists (current backend):**
- `reaction_outcome` (E.i.7)
- `seriousness_criteria` (E.i.3.2)
- `drug_characterization` (G.k.1)
- `action_taken` (G.k.7)
- `dosage_form` (G.k.4.r.9.1)
- `route_of_administration` (G.k.4.r.10.1)
- `age_group` (D.2.3)
- `sex` (D.5)
- `report_type` (C.1.3)
- `sender_type` (C.3.1)

---

### Import/Export APIs

| Method | Endpoint | Description | Auth Required |
|--------|----------|-------------|---------------|
| `POST` | `/api/import/xml/validate` | Validate XML against XSD + rules | Yes |
| `POST` | `/api/import/xml` | Import XML to case | Yes |
| `GET` | `/api/cases/{case_id}/export/xml` | Export XML | Yes |

### Audit APIs

| Method | Endpoint | Description | Permission |
|--------|----------|-------------|------------|
| `GET` | `/api/audit-logs` | List all logs | AuditLog.List (admin/manager) |
| `GET` | `/api/audit-logs/by-record/{table}/{id}` | Logs for record | AuditLog.List |

---

---

## Notes For Frontend Integration
- All endpoints use cookie auth; do not attach bearer tokens.
- Most nested resources require `case_id` in the POST body.
- Collections are ordered by `sequence_number` where applicable.

---

## API Examples (Frontend Wiring Reference)

### Auth
```json
POST /auth/v1/login
{ "email": "demo.cro.admin@example.com", "pwd": "welcome" }
```

### Create Case
```json
POST /api/cases
{ "data": { "status": "draft" } }
```

### Update Case (status)
```json
PUT /api/cases/{id}
{ "data": { "status": "validated" } }
```

### Message Header (create/update)
```json
POST /api/cases/{case_id}/message-header
{
  "data": {
    "case_id": "{case_id}",
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

### Safety Report (create/update)
```json
POST /api/cases/{case_id}/safety-report
{
  "data": {
    "case_id": "{case_id}",
    "transmission_date": [2024, 15],
    "report_type": "1",
    "date_first_received_from_source": [2024, 10],
    "date_of_most_recent_information": [2024, 15],
    "fulfil_expedited_criteria": true
  }
}
```

### Patient (create/update)
```json
POST /api/cases/{case_id}/patient
{ "data": { "case_id": "{case_id}", "patient_initials": "PT", "sex": "2" } }

PUT /api/cases/{case_id}/patient
{ "data": { "weight_kg": 70.5, "race_code": "2106-3" } }
```

### Reaction (create/update)
```json
POST /api/cases/{case_id}/reactions
{ "data": { "case_id": "{case_id}", "sequence_number": 1, "primary_source_reaction": "Headache", "serious": false, "outcome": "1" } }

PUT /api/cases/{case_id}/reactions/{id}
{ "data": { "outcome": "2" } }
```

### Drug (create/update)
```json
POST /api/cases/{case_id}/drugs
{ "data": { "case_id": "{case_id}", "sequence_number": 1, "drug_characterization": "1", "medicinal_product": "Drug A" } }

PUT /api/cases/{case_id}/drugs/{id}
{ "data": { "medicinal_product": "Drug A Updated" } }
```

### Test Result (create/update)
```json
POST /api/cases/{case_id}/test-results
{ "data": { "case_id": "{case_id}", "sequence_number": 1, "test_name": "Baseline Test" } }

PUT /api/cases/{case_id}/test-results/{id}
{ "data": { "test_result_value": "111", "test_result_unit": "mg/dL", "result_unstructured": "Test unstructured" } }
```

### Narrative (create/update)
```json
POST /api/cases/{case_id}/narrative
{ "data": { "case_id": "{case_id}", "case_narrative": "Case narrative" } }

PUT /api/cases/{case_id}/narrative
{ "data": { "sender_comments": "Sender comments updated" } }
```

### Other Identifiers (create/update)
```json
POST /api/cases/{case_id}/other-identifiers
{ "data": { "case_id": "{case_id}", "sequence_number": 1, "source_of_identifier": "SRC", "case_identifier": "CASE-123" } }
```

### Linked Reports (create/update)
```json
POST /api/cases/{case_id}/linked-reports
{ "data": { "case_id": "{case_id}", "sequence_number": 1, "linked_report_number": "LINK-1" } }
```

### Import XML (multipart)
```
POST /api/import/xml
Content-Type: multipart/form-data; boundary=...
file=@case.xml
```

### Export XML
```
GET /api/cases/{case_id}/export/xml
```

**Error Types:**
| Type | HTTP | Description |
|------|------|-------------|
| `LOGIN_FAIL` | 401 | Invalid credentials |
| `AUTH_REQUIRED` | 401 | Not authenticated |
| `PERMISSION_DENIED` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_ERROR` | 400 | Invalid request data |

---

### RBAC Permission Matrix

| Resource | Action | Admin | Manager | User | Viewer |
|----------|--------|:-----:|:-------:|:----:|:------:|
| **Case** | Create | ✅ | ✅ | ✅ | ❌ |
| | Read | ✅ | ✅ | ✅ | ✅ |
| | Update | ✅ | ✅ | ✅ | ❌ |
| | Delete | ✅ | ✅ | ❌ | ❌ |
| **User** | Create | ✅ | ❌ | ❌ | ❌ |
| | Read | ✅ | ✅ | ✅ | ✅ |
| | Update | ✅ | ❌ | ❌ | ❌ |
| | Delete | ✅ | ❌ | ❌ | ❌ |
| **Organization** | CRUD | ✅ | ❌ | ❌ | ❌ |
| **AuditLog** | Read/List | ✅ | ✅ | ❌ | ❌ |
| **Subresources** | CRUD | ✅ | ✅ | ✅ | Read |

**Note:** Row-Level Security (RLS) filters data by organization. Non-admins only see their org's data.

---

## Frontend Integration Checklist

### ✅ Phase 1: Authentication (COMPLETE)
- [x] Login form → `POST /auth/v1/login`
- [x] Token refresh (14-min intervals) → `POST /auth/v1/refresh`
- [x] Logout → `POST /auth/v1/logoff`
- [x] Get current user on load → `GET /api/users/me`
- [x] 401 interceptor for automatic redirect

### ✅ Phase 2: Case Management (COMPLETE)
- [x] Case list → `GET /api/cases`
- [x] Create case → `POST /api/cases`
- [x] Case detail → `GET /api/cases/{id}`
- [x] Update case → `PUT /api/cases/{id}`
- [x] Delete case → `DELETE /api/cases/{id}`

### ✅ Phase 2.5: E2B(R3) Form Structure (COMPLETE)
- [x] 8-Section Form Wizard with all E2B(R3) sections
- [x] Complete type system for all 210 E2B(R3) data elements
- [x] Zod validation schemas for all form sections
- [x] Proper E2B(R3) field numbering (C.1.1, D.1, E.i.1, etc.)
- [x] Section labeling compliance

### 🚧 Phase 3: Form Sections Backend (CURRENT)
- [ ] Safety Report → `/api/cases/{id}/safety-report` (Section C)
- [ ] Patient → `/api/cases/{id}/patient` (Section D)
- [ ] Reactions → `/api/cases/{id}/reactions` (Section E)
- [ ] Test Results → `/api/cases/{id}/test-results` (Section F)
- [ ] Drugs → `/api/cases/{id}/drugs` (Section G)
- [ ] Narrative → `/api/cases/{id}/narrative` (Section H)
- [ ] Message Header → `/api/cases/{id}/message-header`

### Phase 4: Terminology
- [ ] MedDRA autocomplete → `GET /api/terminology/meddra?q=`
- [ ] WHODrug autocomplete → `GET /api/terminology/whodrug?q=`
- [ ] Country list → `GET /api/terminology/countries`
- [ ] Code lists → `GET /api/terminology/code-lists?list=`

### Phase 5: Array Field Architecture
- [ ] Refactor form to use dynamic arrays with useFieldArray
- [ ] Add "Add/Remove" buttons for repeating sections
- [ ] Handle drug-reaction relationships (G.k.9.i)

### Phase 6: Admin Features
- [ ] User management (admin only)
- [ ] Organization management (admin only)
- [ ] Audit log viewer (admin/manager)

---

## Future Phases (Not Yet Implemented)

### Phase 7: XML Processing
- [ ] E2B(R3) XML export
- [ ] E2B(R3) XML import with validation
- [ ] XSD schema validation
- [ ] ICH example support

### Phase 8: Production Hardening
- [ ] Error boundaries and graceful error handling
- [ ] Loading skeletons and optimistic UI
- [ ] Offline support / draft persistence (localStorage)
- [ ] Accessibility audit (WCAG 2.1 AA compliance)
- [ ] CSRF protection
- [ ] Rate limiting
- [ ] Performance optimization
- [ ] Deployment configuration

---

*Last Updated: 2026-02-05*
