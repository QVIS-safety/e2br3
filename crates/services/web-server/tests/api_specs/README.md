# API 계약 명세와 테스트 커버리지

현재 프로그램에 등록한 **백엔드 436개 + 프런트 자체 1개 = 437개 HTTP operation**의 명세다. 요청·응답 필드, 저장·삭제·복원, 권한·잠금·감사, 오류, 직접 검사 코드와 남은 공백을 연결한다. API 경로의 중복·누락과 테스트가 있다는 사실을 실제 실행 성공과 구분한다.

**기준:** 2026-10-31, backend `develop`의 `ef334e934a7dc381083fbb340931cad2a2faea0d` 이후 이 변경. frontend `344049025f4d3a8adba73b7db5cf644f18978412`. **실행: NOT_RUN.** 사용자 지시에 따라 테스트·빌드·DB·API·UI를 실행하지 않았다. status와 상태 변화는 구현 및 assertion에서 읽은 계약이며 PASS 결과가 아니다.

## 명세 구성

| 자료 | 확인하는 내용 |
|---|---|
| [surfaces.md](surfaces.md) | 두 저장소의 API 경계, 프런트 validator proxy 1개, 프록시 중복 제외와 외부 AS2 경계 |
| [api-contract-manifest.json](api-contract-manifest.json) | 백엔드 모든 method/path → route·handler 또는 macro → 요청 extractor·반환형 → 중첩 Rust 타입·필드·serde 속성·소스 위치/해시 |
| 아래 API군별 문서 | 사람이 읽는 입력·응답·오류·저장·삭제·권한·감사 의미; 동적 JSON의 의미는 타입 목록만으로 추론하지 않음 |
| [coverage.tsv](coverage.tsv) | 백엔드 operation별 실제 HTTP assertion 근거와 검사 공백 |
| [row-lifecycle.md](row-lifecycle.md), [row-lifecycle.tsv](row-lifecycle.tsv) | Editor 31 owner·RE 배열·Case 38·Presave 15 자원의 807개 lifecycle 검사 축 |
| [file-structure.md](file-structure.md), [file-structure.tsv](file-structure.tsv) | 모든 API 관련 테스트 파일의 소속·책임·실행 단위·등록 수 |

JSON manifest는 OpenAPI나 추정 schema가 아니다. `Option`, wrapper, generic 타입과 serde 속성을 원형대로 보존하며, missing/null/clear/default 의미를 Rust 타입만 보고 만들어내지 않는다. 동적 `Value`, `Response`, multipart/파일 전송의 실제 shape와 오류는 handler 소스와 해당 API군 문서를 함께 읽는다.

## 전체 색인

| API군 | 명세 | 등록 operation | 동작 assertion | 권한만 | 등록만 | 근거 없음 |
|---|---|---:|---:|---:|---:|---:|
| Case CRUD·조회·workflow·하위 자원 | [cases.md](cases.md) | 231 | 227 | 1 | 3 | 0 |
| Editor 페이지·행·필드·삭제/복원 | [editor.md](editor.md) | 55 | 41 | 0 | 14 | 0 |
| Presave 부모·자식·graph | [presave.md](presave.md) | 73 | 70 | 0 | 3 | 0 |
| XML import·validation·history | [import.md](import.md) | 4 | 3 | 0 | 1 | 0 |
| XML/CIOMS export·history | [export.md](export.md) | 6 | 6 | 0 | 0 | 0 |
| 제출·receiver·header·ACK·reconcile | [submission.md](submission.md) | 18 | 18 | 0 | 0 | 0 |
| 사전 검색·import·release | [terminology.md](terminology.md) | 15 | 12 | 0 | 3 | 0 |
| 사용자·조직·역할·설정·감사·인증·health | [administration.md](administration.md) | 34 | 29 | 4 | 0 | 1 |
| **백엔드 합계** | [coverage.tsv](coverage.tsv) | **436** | **406** | **5** | **24** | **1** |
| 프런트 자체 validator proxy | [surfaces.md](surfaces.md) | 1 | 0 | 0 | 0 | 1 |

[authorization.md](authorization.md)는 인증 cookie, permit/RLS, CSRF와 공통 오류의 횡단 계약이다. 별도 operation으로 중복 계산하지 않는다. 백엔드 분모는 보호 `/api` 429개, `/auth/v1` 3개, `/internal` 3개, `/health` 1개다. 보호 경로는 [snapshot](../authz/protected_route_inventory.snapshot)과 일치한다. 자동 HEAD/OPTIONS·정적 파일·fallback·미등록 legacy URL은 명시 API 분모에서 제외한다.

## 증거 판정

| 상태 | 실제 근거 | 보장하지 않는 것 |
|---|---|---|
| `asserted` | 해당 요청의 status/body/저장/감사 중 직접 assertion. 새 조회로 ID·값을 비교하는 교차 API 검사 포함 | 모든 필드·오류·권한·경계·감사, 현재 실행 PASS |
| `authorization_only` | 해당 HTTP 요청의 권한 거절·조직 가시성 assertion | 정상 응답 필드와 업무 동작 |
| `registration_only` | 활성 route inventory가 method/path 등록 확인 | HTTP 동작·저장·권한 검사 |
| `uncovered` | 구현은 있으나 위 근거 없음 | 구현 결함이라는 판정 |

**406은 전체 계약 검증을 끝낸 API 수가 아니다.** 하나의 동작을 검사하는 경로도 포함한다. 각 operation의 `gaps`, 명세, 807개 lifecycle 행렬을 함께 확인한다. 같은 BMC나 공통 guard를 쓴다는 이유로 호출하지 않은 HTTP method에 검사 근거를 옮기지 않았다. 준비용 POST만 호출한 Presave study 자식 3개는 계속 등록 검사로 남겼다.

`coverage.tsv`의 `method/path`는 router 원형, `spec`은 문서와 anchor, `evidence`는 `web-server` 기준 `tests/파일.rs::함수`(복수는 `;`), `gaps`는 직접 검사하지 않는 범위다. `evidence`는 근거 색인이며 모든 테스트의 역색인은 아니다. ignored/컴파일 제외·성공 조기 반환을 직접 행동 근거로 올리지 않는다.

## 이번 보강과 파일 책임

- CI soft 자식 3종: 삭제·재삭제·일반 재저장 비복원·명시 복원·동일 저장, fresh GET의 ID/count/형제 보존, 삭제 감사와 거절 rollback을 [ci_rows.rs](../api_editor/relations/ci_rows.rs)에 추가했다.
- Editor 자원별 기존 assertion: viewer·다른 조직·Case 잠금의 update/delete/restore 거절 후 저장·감사 불변, soft transition/no-op 감사, source-document/assessment hard DELETE 감사, FDA device/code 3종 clear·재호출을 보강했다.
- Case legacy 자원 38개: [subresources](../api_cases/subresources)의 drug/patient/report/general lifecycle 파일에 구분했다. 감사, 정책 거절, singleton, 행 흐름 helper도 각각 분리한다. source-document에는 등록되지 않은 item GET을 만들지 않고 collection 조회로 검사한다. Case ID만 받는 singleton에는 존재하지 않는 “다른 부모 ID” 거절을 요구하지 않는다.
- Presave 15종: [lifecycle_matrix_web.rs](../api_presave/lifecycle_matrix_web.rs), [authorization_web.rs](../api_presave/authorization_web.rs), [audit_web.rs](../api_presave/audit_web.rs)로 lifecycle·권한·감사 책임을 분리했다. Case lock은 Presave 계약에 없어 N/A다.
- 실제 결함 수정: Drug·Reaction·TestResult legacy PUT의 삭제행 일반 수정 후 404 경로를 차단했다. Reaction의 공개 `deleted:true/false` 삭제·복원 PUT은 성공 응답과 같은 ID를 유지한다. Editor DG의 삭제행 PATCH/복원 허용 경로는 별도로 유지하며, 변경값을 보낸 거절 요청의 감사 불변과 복원 후 원값을 검사한다.
- RE 수신자 배열: [review_receivers.rs](../api_cases/contract/review_receivers.rs)에 2→1→동일1→[]→동일[]와 Case GET/shell·실제 JSON 감사·거절 불변을 연결했다.

Editor는 **scalar 373 + behavior 60 = 433개**를 registry에서 등록한다. canonical UI 필드 360개, HTTP operation 55개, 직접 Rust test 선언 수와 다른 집계다. [generated 파일](../api_editor/pages)은 수기 verifier 실행 등록이고, 필드별 [registry](../../../../../registry/editor-contracts)는 계약 원본이다. 같은 검사를 중복 구현한 것이 아니다.

기존 9개 실행 단위 `api_cases`, `api_editor`, `api_presave`, `api_import`, `api_submission`, `api_terminology`, `api_administration`, `authz`, `xml`을 유지한다. 모델/DB assertion은 `lib-core`, HTTP는 `web-server`, XML 변환/XSD는 해당 target에 둔다. 이 명세 디렉터리는 새 Cargo target을 만들지 않는다.

## 남은 공백

1. 등록만 검사하는 24개: Case query catalog/search alias/link-options 3개, legacy Editor GET 14개, Presave study 자식 생성 3개, import validate 1개, terminology 검색 3개. health와 프런트 proxy는 직접 HTTP 검사 근거가 없다.
2. 권한만 검사하는 5개: Case versions 1개와 administration 4개. 정상 결과의 필드·정렬·경계 검사는 별도다.
3. lifecycle의 결합 축: 모든 부모·자식 cascade 조합, 생성 권한의 모든 역할/scope, 동시 변경·응답 유실 후 새 POST 재시도, 전체 필드의 제약 조합은 이 검사로 입증하지 않는다. 신규 POST를 멱등적이라고 명세화하지 않는다.
4. 감사 chain 손상 HTTP 보고, CIOMS 전체 필드·한글·잘림, 외부 XML fixture/Java/전송·기관 수용 증거는 해당 명세의 공백으로 남아 있다. 정상 데이터의 감사 assertion을 손상 탐지·전자서명·규제 준수 인증으로 바꾸지 않는다.
5. 이번 변경의 실행 결과는 전부 NOT_RUN이다. 동일 commit의 선택된 테스트 목록·로그·환경/의존 데이터 버전이 있어야 실행 통과를 입증한다.

## 변경 시 정적 검증

API·DTO·테스트 근거가 바뀌면 사람이 해당 명세와 coverage를 검토한 뒤 아래 명령으로 manifest를 갱신하고 diff를 확인한다.

```sh
python3 crates/services/web-server/tests/api_specs/check_api_contract_manifest.py --write
python3 crates/services/web-server/tests/api_specs/check_api_contract_manifest.py
```

검사기는 route/snapshot/coverage 집합, 중복·미해결 handler/type, 문서 anchor·활성 evidence, 원본 해시 및 생성 결과의 일치를 검사한다. 자동 생성은 문서의 의미를 대신 검토하지 않는다. CI의 `Test Scope`에서 이 정적 검사를 먼저 수행한다. 명세 `.md/.tsv`와 위 manifest/checker만 바뀌면 Rust 테스트·Clippy 선택은 늘어나지 않는다. 다른 실행 소스를 같은 디렉터리에 넣어 제외시키지 않는다.

GPT-5.6 Sol 작업을 상위에서 직접 대조했고, 잘못된 RLS status, 필수 DTO 필드 누락, 미등록 GET 호출, singleton 소속 거절 가정 등을 고쳤다. 이 자료의 목적은 **명세 누락과 검사 공백을 보이게 하는 것**이며 실행 성공 또는 데이터 무결성 인증서를 대신하지 않는다.
