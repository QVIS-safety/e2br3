# Editor API 계약 명세

이 문서는 Editor API가 받아들이는 요청, 응답, 영속 상태와 감사 기록에 대한 검사 명세다. 테스트 실행 결과나 규제 적합성 인증서는 아니다. 필드별 입력·기대값의 원본은 [`registry/editor-contracts`](../../../../../registry/editor-contracts)에 있으며, 아래 계약은 실제 HTTP handler와 저장 모델을 대상으로 한다.

<a id="pages"></a>
## 요청과 응답

공통 경로를 `/api/cases/{caseId}/editor/pages/{page}`로 표기한다. `caseId`와 `rowId`는 해당 조직·Case에 속하는 UUID여야 한다. 인증 쿠키와 편집 권한이 필요하다. 변경 사유는 `x-e2br3-reason-for-change` 헤더로 전달한다.

| API | 적용 페이지 | 정상 응답 | 검사 대상 |
|---|---|---|---|
| `GET /{page}` | CI, RP, SD, SI, DM, NR | 200, `rows`에 페이지 데이터 | 저장값·clear 결과·자식 행 ID 재조회 |
| `PATCH /{page}` | CI, RP, SD, SI, DM, NR | 200, `rows`에 갱신 데이터 | 누락 처리 계약, 입력 제약, 부모·자식 소속 |
| `GET /{page}` | LR, DH, AE, LB, DG | 200, `rows.rows`에 목록 | 삭제 행 제외, 표시 열, 순서 |
| `GET /{page}?include_rows=true` | LR, DH, AE, LB, DG | 200, 목록과 `rows.rowDetails` | 키가 실제 행 ID이고 각 값이 해당 행 상세 GET 응답과 일치 |
| `POST /{page}/rows` | LR, DH, AE, LB, DG | 201, `rowId` | 신규 행 생성·미완성 draft 허용 범위·필수 입력 거절 |
| `GET /{page}/rows/{rowId}` | LR, DH, AE, LB, DG | 200, `data`에 상세 | 삭제된 행도 `deleted: true`로 조회 가능 |
| `PATCH /{page}/rows/{rowId}` | LR, DH, AE, LB, DG | 200, `data`에 상세 | 필드 변경, 하위 행 수정, `deleted: false`를 통한 복원과 편집의 원자성 |
| `DELETE /{page}/rows/{rowId}` | LR, DH, AE, LB, DG | 204, 본문 없음 | soft delete, 목록에서 제외, 자식 보존·연결 정리 |
| `POST /{page}/rows/{rowId}/restore` | AE, LB, DG | 전용 복원 handler | 라우트 존재. 이 파일군의 복원 검사는 PATCH 복원 경로를 기준으로 하며 POST 복원과 동일한 실행 증거로 취급하지 않는다. |
| `GET /api/cases/{caseId}/audit-trail` | Case 전체 | 200, `data` 배열 | 변경 항목·값·사유·반복행 위치 표시 |

PATCH/POST의 JSON 본문은 다음 형태다. `owner`와 각 필드의 실제 이름은 페이지 계약의 `patch.owner`, `payloadPath`를 따른다. 응답 이름은 `projectionPath`를 따르며 요청의 camelCase와 같다고 가정하지 않는다.

```json
{
  "authorities": ["ich"],
  "rows": {
    "owner": {"field": "value"}
  }
}
```

직접 페이지의 반복행 owner는 객체 배열이며, DG 같은 행 상세에서는 owner 내부의 하위 컬렉션이 배열이다. 기존 자식 행 수정에는 `rowIdentities`에 명시된 조회 ID를 넣는다. DG assessment는 `drugReactionAssessmentId`·`reactionId`를 사용하며, relatedness 자식이 있을 때만 그 자식의 `id`를 함께 보낸다. FDA·MFDS 필드는 해당 계약에 지정된 `authority`를 소문자로 전달한다. 임의 UUID나 운영 참조정보를 만든 값으로 실제 FK 저장을 통과시켰다고 간주하지 않는다.

## 페이지별 요청 owner와 계약 정의

| 페이지 | `rows`의 owner | 필드 계약 | 추가 동작 검사 | 정의 / 실행 등록 |
|---|---|---:|---:|---|
| CI | `case`, `documentsHeldBySender`, `linkedReports`, `otherCaseIdentifiers`, `safetyReportIdentification`, `sourceDocuments` | 26 | 4 | [JSON](../../../../../registry/editor-contracts/ci.json) / [등록](../api_editor/pages/ci.generated.rs) |
| RP | `primarySources` | 29 | 2 | [JSON](../../../../../registry/editor-contracts/rp.json) / [등록](../api_editor/pages/rp.generated.rs) |
| SD | `receiverInformation`, `senderInformation` | 27 | 0 | [JSON](../../../../../registry/editor-contracts/sd.json) / [등록](../api_editor/pages/sd.generated.rs) |
| LR | `literatureReference` | 3 | 3 | [JSON](../../../../../registry/editor-contracts/lr.json) / [등록](../api_editor/pages/lr.generated.rs) |
| SI | `studyInformation`, `studyRegistrationNumbers` | 14 | 5 | [JSON](../../../../../registry/editor-contracts/si.json) / [등록](../api_editor/pages/si.generated.rs) |
| DM | `autopsyCauses`, `deathInfo`, `medicalHistoryEpisodes`, `parentInfo`, `parentMedicalHistory`, `parentPastDrugs`, `patientIdentifiers`, `patientInformation`, `reportedCauses` | 87 | 6 | [JSON](../../../../../registry/editor-contracts/dm.json) / [등록](../api_editor/pages/dm.generated.rs) |
| NR | `caseSummaryInformation`, `narrative`, `senderDiagnoses` | 8 | 4 | [JSON](../../../../../registry/editor-contracts/nr.json) / [등록](../api_editor/pages/nr.generated.rs) |
| AE | `reaction` | 49 | 3 | [JSON](../../../../../registry/editor-contracts/ae.json) / [등록](../api_editor/pages/ae.generated.rs) |
| LB | `testResult` | 14 | 6 | [JSON](../../../../../registry/editor-contracts/lb.json) / [등록](../api_editor/pages/lb.generated.rs) |
| DG | `drug` | 92 | 13 | [JSON](../../../../../registry/editor-contracts/dg.json) / [등록](../api_editor/pages/dg.generated.rs) |
| DH | `pastDrugHistory` | 16 | 4 | [JSON](../../../../../registry/editor-contracts/dh.json) / [등록](../api_editor/pages/dh.generated.rs) |

위 숫자는 등록된 API 검사 단위다. canonical UI 필드 360개, 일반 editor scalar 등록 365개와 N header 등록 8개, 관계 필드의 별도 API 검사 수는 서로 다른 집계다.

## 필드 저장 계약

각 `fields[]` 항목은 다음 사항을 명시한다.

| 속성 | API 기대 결과 |
|---|---|
| `code`, `authority` | 검사할 E2B/기관 확장 필드 식별자 |
| `patch.owner`, `payloadPath` | `rows` 안에서 보낼 실제 요청 위치 |
| `roundTripValue`, 선택적 `fixture`·`rowsFixture` | 저장할 값과 필요한 부모·동반 필드 |
| `projectionPath`, 선택적 `projectionValue` | 새 GET에서 읽을 위치와 값. 변환이 없으면 저장 입력과 같음 |
| `persistence.clear` | 명시적 clear 입력과 저장 기대값, 또는 422 및 `ruleCode` |
| `persistence.blank` | 빈 문자열의 저장/거절 결과. 비문자열 입력은 이유를 적어 적용 제외 |
| `persistence.omission` | 필드 누락 시 preserve, replacement에 의한 값 제거, 또는 명시된 400 오류 |
| `persistence.audit` | 실제 감사 대상 테이블·열·레코드 ID 위치 |
| `constraint` | 기존 input contract의 잘못된 값, 오류 코드, 가능한 오류 경로 |
| `nullFlavorPartnerCode` | 값/NF 동반 필드. 전환과 결합 동작은 `fieldChecks`에 별도로 연결 |

공통 필드 검사는 저장 → GET, 속성 누락 → GET, 저장값 설정 → clear → GET, 저장값 설정 → blank → GET, 잘못된 입력 거절 → GET을 수행한다. 성공 시 Case의 기존 status가 바뀌지 않아야 한다. 거절 시 페이지의 영속 projection이 직전 상태와 같아야 한다. 반복행의 기존 ID를 요청에 다시 사용하고 감사 레코드 ID가 교체되지 않았는지도 확인한다.

`null`, `""`, 누락은 서로 다른 입력이다. CI의 비nullable 자식 텍스트는 null을 빈 문자열로 저장한다. LB `testName`은 draft에서 빈 문자열을 허용한다. `safetyReportId`, DG `drugCharacterization`·`medicinalProduct`, NR `caseNarrative`의 명시적 null/blank는 `422 / INPUT.NULL.REJECTED`다. FDA device code의 빈 값 거절은 각 input contract의 별도 규칙을 따른다. FDA device 목록은 replacement다. device scalar 누락은 null로 저장하며, NF가 아닌 문자열의 blank는 빈 문자열로 저장한다. 하위 code 객체에서 `valueCode`를 누락하면 `400 / INVALID_REQUEST`, detail `Invalid FDA device information`이며 기존 데이터는 보존된다. `fdaDevices` 자체를 보내지 않는 경우와 device 객체 내부의 필드를 생략하는 경우를 구분한다. 구체적인 예외는 필드 JSON에 명시하며 공통 기본값으로 덮지 않는다.

결합 필드 계약은 [fields/coupled_clear.rs](../api_editor/fields/coupled_clear.rs), 값/NF 전환은 [fields/null_flavor.rs](../api_editor/fields/null_flavor.rs), 필수값 거절과 변경 원자성은 [fields/required_values.rs](../api_editor/fields/required_values.rs)에 있다. 예를 들어 LB 수치 결과 clear는 qualifier도 지우지만 자유 텍스트를 보존해야 한다. DG PHPID의 null 처리와 blank 처리에서 동반 version 보존 여부도 구분한다.

## 반복행 및 부모·자식 계약

- **ID 보존:** 기존 행·하위 코드 수정에서 `id`, `created_at`, 변경하지 않은 형제 행을 보존한다. 새 행에는 다른 ID와 충돌하지 않는 sequence가 필요하다.
- **목록과 상세:** 목록 표시 열의 값이 상세 저장값과 일치해야 한다. batch 상세는 개별 상세 GET과 동일해야 한다. LR/DH/RP의 sequence 정렬 계약은 `collections` 등록을 따른다.
- **빈 입력:** 일반 컬렉션의 `[]`는 미완성 부모를 만들지 않는다. `[{}]`의 draft 생성 허용 여부는 컬렉션별 검사에 명시한다. FDA `fdaDevices`처럼 전체 목록을 교체하는 컬렉션에서 `[]`는 기존 행 제거를 뜻하므로 일반 목록과 구분한다.
- **소속·중복:** 중복 자식 ID, 다른 부모/Case의 자식 ID는 허용하지 않는다. FDA device 중복 ID와 중복 code ID는 400, 다른 drug의 device ID는 404, 다른 device의 code ID는 400이다. 거절 후 기존 자식 목록이 바뀌지 않아야 한다.
- **부모 생성:** SI 등록번호, DM 병력·사망원인·부모 정보, NR 요약·진단, DH 과거약물을 먼저 저장할 때 필요한 부모를 생성하고 이후 편집에서 같은 부모 ID 및 이미 저장한 값을 보존한다.
- **삭제·복원:** 삭제된 행은 활성 목록에서 제외되고 상세에는 삭제 상태로 남는다. PATCH 복원과 값 수정은 함께 성공하거나 함께 취소되어야 한다. LR/DH sequence 충돌, AE/LB/DG 잘못된 복원 입력은 이전 삭제 상태와 내용을 보존한다. 다른 Case ID로 접근하면 404, viewer의 복원 변경은 403이다.
- **연결 관계:** DG relatedness 하위 항목 삭제는 형제·부모를 보존하고 부모 assessment 삭제는 해당 연결을 제거한다. DG 행 삭제/복원은 자식 데이터와 식별자의 계약을 지킨다.

구체적 요청 본문과 상태별 기대값은 [rows](../api_editor/rows), [relations](../api_editor/relations)의 검사와 각 페이지 JSON의 `collections`, `relations` 등록으로 연결된다. 검사가 없는 API 동작을 이 명세만으로 검증 완료 처리하지 않는다.

## 감사 계약

[공통 감사 검사](../api_editor/audit.rs)는 필드 저장·clear와 동일한 HTTP 변경을 관찰한다. 감사 검증을 위해 동일 저장을 한 번 더 실행하지 않는다.

1. API 변경 전후 실제 DB 저장값을 읽고 신규 `audit_logs`를 식별한다.
2. 변경 대상의 table, record ID, column이 맞고 최초 old value와 최종 new value가 실제 저장 상태와 같아야 한다.
3. 변경 사용자, 조직, 전달한 변경 사유가 일치해야 한다.
4. 값이 바뀌지 않은 요청은 해당 필드의 거짓 변경 기록을 남기지 않아야 한다.
5. 거절된 입력은 Case 및 자식의 신규 감사 변경 기록을 남기지 않아야 한다.
6. 삭제·복원 검사도 같은 변경 요청에서 `deleted`의 false→true→false 감사 전환과 레코드 ID·사용자·조직·사유를 확인한다.
7. [감사 조회 API 검사](../api_editor/audit/projection.rs)는 FDA device usage/lot의 `item`, `value`, `reason`, `rowNo1/2/3`를 검증한다. DB 감사 검사 전체를 화면 표시 항목 전체의 검증으로 확대 해석하지 않는다.

## 테스트 연결 및 실행 증거

N header의 8개 항목도 [N registry](../../../../../registry/editor-contracts/n.json)에서 [실행 등록](../api_editor/pages/n.generated.rs)을 생성한다. [N 검사](../api_editor/fields/message_header.rs)는 `/editor/pages/N`을 만들지 않고 실제 `POST/PUT/GET /api/cases/{id}/message-header`를 사용한다. 정상 저장·GET·감사 변경과 잘못된 값 거절 후 header·감사 불변을 확인한다. N.1.2는 POST에서 파생되어 PUT 거절만 검사하고, N.1.5는 REST 날짜 DTO의 배열 표현으로 저장하며 잘못된 문자열은 역직렬화 단계의422로 검사한다. N에는 일반 editor clear/blank/omission 검사를 적용했다고 주장하지 않는다.

`pages/<page>.generated.rs`는 페이지 계약에서 만든 테스트 등록 파일이다. 필드 기대값과 HTTP 동작을 복제하지 않는다. Cargo target은 `api_editor` 하나이며 이름은 `case_editor_contract_web::<page>::fields|rows|relations|audit::<검사 ID>`다. 필드별 실행 단위 안에서 저장·clear·누락·거절·감사를 함께 확인한다.

CI의 실행 목록과 결과 gate는 같은 페이지 계약에서 기대 테스트 집합을 만든다. 선택한 범위에서 누락, ignored, 실패, 중복 실행 결과가 있으면 gate가 실패한다. 페이지 변경은 해당 페이지, 책임별 검증기 변경은 해당 책임, 공통 검증기·production 의존 변경은 영향을 받는 전체 범위로 넓힌다.

DG의 `G.k.9.i.1`, `G.k.local.sourceProductPresaveId`는 실제 참조 레코드가 필요한 연결 필드다. 일반 scalar 검사에서 `execution.blocked`로 남기고 registry의 `automatedTests`가 지정한 relatedness/product 연결 API 검사를 별도로 요구한다. 이를 일반 scalar 검사 성공 수에 포함하지 않는다.

이 개편에서는 사용자 지시에 따라 Rust/API/DB/Jest 테스트와 빌드를 실행하지 않는다. 생성물·명세·참조의 정적 일치 여부는 실행 통과 증거와 구별한다. 실제 CI 실행 로그가 나오기 전에는 이 명세로 새 검사의 통과나 완전한 데이터 무결성을 주장할 수 없다.

<a id="legacy"></a>
## Shell과 이전 조회 경로

`GET /api/cases/{case_id}/editor/shell`은 `data` wrapper 없이 `CaseEditorShellDto`를 반환한다. [workflow](../api_cases/workflow.rs)의 `test_workflow_transition_updates_case_and_persists_event`가 `workflowDueAt`, [review receivers](../api_cases/contract/review_receivers.rs)의 검사가 `reviewReceivers`와 `messageHeader.messageReceiverIdentifier`를 직접 비교한다. shell 전체 필드·모든 권한 조합의 검사는 아니다. 구현은 [shell handler](../../src/web/rest/case_editor_rest/shell.rs)를 따른다.

현재 router에는 이전 GET 조회 경로도 남아 있다: `/editor/CI`, `/editor/RP`, `/editor/SD`, `/editor/SI`, `/editor/DM`, `/editor/NR`, `/editor/AE/list`, `/editor/AE/{reaction_id}`, `/editor/LB/list`, `/editor/LB/{test_result_id}`, `/editor/DG/list`, `/editor/DG/{drug_id}`, `/editor/DH/list`, `/editor/DH/{past_drug_id}`. 이들은 `/api/cases/{case_id}` 아래에 있으며 pages projection과 별도 method/path다. [등록 소스](../../src/web/rest/routes/cases.rs)와 [DTO](../../src/web/rest/case_editor_dto.rs)는 존재하지만, 조사한 테스트의 직접 HTTP assertion은 찾지 못했다. pages GET 테스트를 이 경로의 실행 증거로 옮겨 계산하지 않는다.

AE/LB/DG의 별도 POST restore 세 경로 역시 등록은 있지만 현재 부모 복원 assertion은 PATCH 경로에 있다. 이 차이는 [전체 커버리지 목록](coverage.tsv)에 그대로 남긴다.
