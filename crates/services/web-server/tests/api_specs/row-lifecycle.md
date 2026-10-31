# 반복행 삭제·재저장 API 계약

**기준:** backend `72be7fdbc24efda7975133bd125c104bd79e3628` 이후 이 변경. **실행: NOT_RUN.** 등록 route, handler/BMC, registry와 활성 assertion을 대조했다. 아래 status는 소스상 기대값이며 실제 실행 결과가 아니다. [검사 행렬](row-lifecycle.tsv)은 각 자원과 검사 축을 한 행씩 연결한다. [전체 API 색인](coverage.tsv)의 `asserted`만으로 이 행렬의 공백을 채우지 않는다.

## 전수조사 범위

| 범위 | 조사 단위 | 검사 축 |
|---|---|---|
| Editor | registry 12페이지, 반복행·하위 컬렉션·관련 singleton/value-array owner 31개 | ID 연결 후 재저장, 같은 PATCH 재전송, 생략/빈 배열, 삭제 후 조회·재저장, 반복 삭제, 명시 복원·재저장, 부모/소속/감사/rollback |
| RE utility | `Case PUT → Editor shell GET`의 수신자 배열 1개 | 부분 제거·같은 배열 재저장·전체 clear·다시 clear |
| Case·Presave | Case 자원 38개 + DELETE가 등록된 Presave 자원 13개 + graph 전용 자원 2개 | 삭제 status, 반복 삭제, 삭제/재저장/복원, 부모·자식, ID/count, graph replay, 소속, viewer, lock, 감사 사유, rollback |

Case·Presave 범위의 등록 DELETE 51개와 관련 mutation 184개를 색인과 대조했다. Editor의 N은 message-header singleton, SD는 sender/receiver singleton이므로 반복행 owner가 없지만 호출·소속 계약을 별도로 확인했다. 위 owner와 자원은 같은 DB 행을 서로 다른 API로 다룰 수 있어 합계 85개를 고유 DB 자원 수로 해석하면 안 된다.

행렬은 **807개 검사 축**이다. `covered`는 해당 축에 기존 직접 assertion이 있음, `added_not_run`은 이번 변경의 직접 assertion, `partial`/`partial_added_not_run`은 일부만 검사, `gap`/`missing`은 직접 근거 없음, `source_only`는 구현만 확인, `not_applicable`은 해당 API 형태에 적용되지 않음을 뜻한다. 모든 행의 실행 상태는 `NOT_RUN`이다. 결합 축이나 자원 수준의 evidence가 모든 세부 조건을 입증하지는 않는다.

## 요청 종류별 상태 계약

| 대상 | 삭제 요청 | 재저장·반복 요청 | 복원 |
|---|---|---|---|
| LR·DH 바깥 행 | `DELETE /editor/pages/{page}/rows/{id}` → 204 | 두 번째 DELETE도 204. 상세 GET은 `deleted:true`, 목록은 제외. ID가 있는 일반 PATCH는 숨김 유지 | PATCH의 해당 owner에 `id, deleted:false` → 200. 같은 ID·개수 유지 |
| AE·LB·DG 바깥 행 | 같은 DELETE → 204 | 두 번째 DELETE는 404. 상세/목록과 일반 PATCH 계약은 위와 같음 | PATCH 200 또는 `POST .../restore` 200. 반복 복원도 200 |
| RP·DM·SI·NR soft 자식 및 DG substance/dosage/indication/relatedness | owner 배열 항목의 `id, deleted:true`를 PATCH → 200 | 삭제 marker 재전송, 컬렉션 생략/빈 배열, 삭제 flag 없는 기존 ID 저장이 삭제를 취소하지 않음 | **명시한** `deleted:false`만 복원. 보강된 검사에서 새 GET·ID·개수를 확인 |
| DM parentInfo | singleton owner의 `id, deleted:true` → 200 | 부모 projection에서 제외 | 같은 ID로 복원하고 병력·과거약물 자식의 ID·개수 보존 |
| CI sourceDocuments | 기존 ID와 `deleted:true` PATCH → 200, 실제 행 제거 | 같은 ID 삭제·일반 수정·`deleted:false` 재전송 → 404, 재생성 없음 | hard delete이므로 복원 계약 없음 |
| DG assessment | `_deleteAssessment`로 부모와 연결 제거 | 삭제된 assessment ID 또는 하위 relatedness ID 재사용은 거절 | assessment는 hard delete. relatedness 자체 soft delete/복원과 구분 |
| DG fdaDevices 및 code 배열 | `fdaDevices` 전체 배열에서 항목 제외 또는 `[]` | 전체 치환 계약. 포함된 기존 ID는 유지하며 생략과 `[]` 의미가 다름 | 제거된 행에 soft restore를 보장하지 않음 |
| RE reviewReceivers | Case PUT의 `review_receivers_json` 배열에서 항목 제외 또는 `[]` | 2행→1행→같은 1행→0행→같은 0행 후 shell GET의 배열 완전 일치 | UUID 행 복원 개념 없음 |
| Presave 부모 6종 | sender/product/receiver/reporter/study/narrative DELETE → 204 | 두 번째 DELETE도 204, GET에서 같은 ID·deleted 확인 | 각 endpoint의 DTO 형태로 PATCH `deleted:false` → 200, 반복 PATCH·GET/list의 ID·개수 확인 |
| Presave hard 자식 | product substance·receiver consignee DELETE 또는 receiver route graph 삭제 | 이전 ID의 GET/DELETE 또는 graph 재사용 거절. 실패 graph의 다른 변경도 rollback | 새 생성은 새 ID. soft restore로 취급하지 않음 |
| Presave study 자식 4종 | registration/product/reporter/FDA IND의 graph PUT `id, deleted:true` | graph GET에 삭제 상태를 포함. 삭제 flag 생략 저장은 삭제 상태 유지 | `deleted:false` 복원 2회 후 매회 GET·같은 ID/count 확인, 이후 재삭제 |

표에서 합친 자원도 개별 evidence 범위는 행렬을 따른다. CI soft 자식 3종, 일부 DG 자식의 소속·감사·반복 삭제 등은 아직 부분 검사다. 일반 POST에 ID가 없으면 신규 행 생성이므로 같은 POST의 멱등성을 약속하지 않는다. HTTP 응답 유실 후 신규 POST 재전송, 동시 저장 충돌은 이번 소스 검사의 범위 밖이다.

## 확정 결함과 변경 계약

- **소속 확인:** RP와 DM 자식 및 parent/death singleton, SI studyInformation, SD sender의 명시 ID가 요청 Case/부모에 속하는지 변경 전에 확인한다. 동일 조직의 다른 Case도 404로 거절한다. legacy drug child 공통 6개 handler와 FDA code 경로도 Case→Drug→Device 관계를 확인하며 삭제된 부모 device 아래의 code 조작을 거절한다.
- **명시 복원:** 기존 soft 모델의 restore를 호출하지 않던 Editor 16종(자식과 DM parentInfo 포함)에 `deleted:false` 복원을 연결했다. 값 수정과 같은 transaction 안에서 처리한다. 삭제 flag 생략 시 자동 복원하지 않는다.
- **잘못된 ID:** 공통 UUID parser 19개 호출부는 잘못된 문자열/타입을 400으로 거절한다. 기존 생성 계약의 missing/null은 유지한다. malformed ID 때문에 새로운 행이 생기지 않아야 하며 거절 후 projection·감사 ID 목록 불변 검사를 추가했다.
- **Presave study 복원:** graph의 registration/product/reporter/FDA IND 4종이 명시 false를 저장 모델로 전달하도록 수정했다. 생략/null은 현재 상태를 보존하고 true는 삭제한다. 각 상태 뒤 새 GET과 복원 재호출의 ID/count를 검사한다.
- **값 배열 계약:** `patientInformation.raceCodes`는 UUID 자식 행이 아닌 문자열 배열이다. 잘못된 row identity 정의를 제거하고 기존 scalar 실행기로 배열 전체의 저장·clear·생략을 검사하도록 수정했다.

기존 데이터를 수정하거나 DB migration을 추가하지 않았다. 위 정책 변경으로 이전에 통과하던 다른 Case의 행 접근, 삭제된 부모 아래의 legacy code 접근, malformed UUID 요청은 거절된다.

## 테스트 파일별 책임

| 위치 | 책임 |
|---|---|
| [relations.rs](../api_editor/relations.rs) | 바깥 행 5종의 동일 저장·삭제·복원 재호출, 상세/목록·값·ID·감사 비교; 하위 행 PATCH 후 새 GET 공통 비교 |
| [rp_rows.rs](../api_editor/relations/rp_rows.rs), [dm_rows.rs](../api_editor/relations/dm_rows.rs), [si_rows.rs](../api_editor/relations/si_rows.rs), [nr_rows.rs](../api_editor/relations/nr_rows.rs), [dg_rows.rs](../api_editor/relations/dg_rows.rs) | 페이지별 하위 행 삭제·숨김·복원·재저장, malformed ID 거절 |
| [scope.rs](../api_editor/relations/scope.rs) | RP/DM/SI/SD의 외부 ID 변경 거절과 양쪽 Case 재조회·감사 불변 |
| [documents.rs](../api_editor/relations/documents.rs), [relatedness.rs](../api_editor/relations/relatedness.rs) | hard delete 재호출·재생성 방지, relatedness soft 복원과 assessment hard 삭제 구분 |
| [drug_scope.rs](../api_cases/subresources/drug_scope.rs) | legacy drug child 5종과 FDA code의 소속·삭제된 부모 경계 |
| [review_receivers.rs](../api_cases/contract/review_receivers.rs) | RE 배열 부분 삭제·동일 저장·전체 clear |
| [api_presave](../api_presave) | 기존 부모 CRUD matrix와 product/receiver/study 책임별 graph·hard child 검사 |

생성 파일은 위 수기 assertion을 실행하도록 연결하는 registry 등록이다. 신규 동작 8개를 포함해 Editor **373 scalar + 58 behavior = 431개**가 등록된다. Rust 선언 수, canonical UI 필드 360개, HTTP operation 436개와는 다른 집계다. 시험 성공 수가 아니다.

## 프런트 호출 연결과 남은 검사

frontend `344049025f4d3a8adba73b7db5cf644f18978412`의 저장 hook·mapper·endpoint와 기존 격리 검사를 정적으로 대조했다. 저장된 바깥 행 삭제는 DELETE, 그 외 기존 행은 PATCH, 신규 행은 POST다. 생성 응답의 ID를 받은 뒤 재조회가 실패하면 다음 저장은 기존 ID PATCH로 전환한다. 삭제 성공 뒤 form marker를 정리하고 복원은 `deleted:false` PATCH로 보낸다. 전용 POST restore client 함수는 없다.

이 호출 선택·hook 검사는 HTTP/DB의 실제 저장 결과 증거로 계산하지 않는다. 프런트 소스와 테스트는 이번 변경에서 수정하지 않았다.

남은 우선순위는 다음과 같다.

1. CI soft 자식 3종과 legacy Case의 부분/미검사 자원에 삭제→GET/list→일반 재저장→명시 복원→재저장 검사를 채운다.
2. 각 자식별 부모 삭제/복원, 다른 Case·조직, viewer, 잠금 거절을 분리하고 실패 전후 데이터·감사 불변을 확인한다. 공통 guard가 있다고 개별 HTTP assertion이 있는 것으로 표시하지 않는다.
3. direct soft 자식의 정상 삭제·복원 감사 old/new·사용자·사유와 no-op 감사를 보강한다. 현재의 malformed/foreign ID 거절 감사 불변 검사로 이를 대신하지 않는다.
4. 실제 실행이 허용되면 같은 commit의 선택된 테스트 목록·환경·결과를 보관한다. 이후 응답 유실·동시 재저장 검사를 별도 계약으로 정한다.

이 문서와 행렬은 검사 범위와 공백을 추적하는 명세다. 실행 로그나 데이터 무결성 인증서를 대신하지 않는다.
