# API 계약 명세와 테스트 커버리지

이 디렉터리는 등록된 API의 **요청 → 응답·저장 상태·감사·권한 계약 → 실제 테스트 assertion → 남은 공백**을 연결한다. [coverage.tsv](coverage.tsv)가 method/path별 색인이고, 아래 문서가 API군별 명세다. 테스트 이동 경로나 중복 제거 이력은 명세에 포함하지 않는다.

**검토 기준:** 2026-10-31, backend `develop`의 구현 commit `abee1ce396433ecc01f972e2a418b33e585ca278`. **실행 상태: NOT_RUN.** 사용자 지시에 따라 이번 작업에서는 테스트·빌드·운영 API·DB를 실행하지 않았다. 본문의 성공 status와 상태 변화는 소스상 계약이며, 통과 결과가 아니다. 예시 JSON은 구조 설명이며 운영 응답이나 실행 결과가 아니다.

## 전체 색인

| API군 | 명세 | 등록 operation | 동작 assertion 있음 | 권한만 assertion | 등록만 검사 | 검사 근거 없음 |
|---|---|---:|---:|---:|---:|---:|
| Case CRUD·조회·workflow·하위 리소스 | [cases.md](cases.md) | 231 | 111 | 1 | 119 | 0 |
| Editor 페이지·행·필드·삭제/복원·감사 | [editor.md](editor.md) | 55 | 38 | 0 | 17 | 0 |
| Presave 부모·자식·graph | [presave.md](presave.md) | 73 | 46 | 1 | 26 | 0 |
| XML import·validation·history | [import.md](import.md) | 4 | 3 | 0 | 1 | 0 |
| XML/CIOMS export·history | [export.md](export.md) | 6 | 6 | 0 | 0 | 0 |
| 제출·receiver·header·ACK·reconcile | [submission.md](submission.md) | 18 | 17 | 0 | 1 | 0 |
| 사전 검색·import·release | [terminology.md](terminology.md) | 15 | 12 | 0 | 3 | 0 |
| 사용자·조직·역할·설정·감사·인증·health | [administration.md](administration.md) | 34 | 29 | 4 | 0 | 1 |
| **합계** | [coverage.tsv](coverage.tsv) | **436** | **262** | **6** | **167** | **1** |

[authorization.md](authorization.md)는 인증 cookie, authorization snapshot, permit/RLS, CSRF와 오류 매핑의 횡단 계약이다. 같은 API를 별도 operation으로 중복 계산하지 않는다.

분모 436개는 현재 router에 명시적으로 등록한 **HTTP method + path**다. 보호 `/api` 429개는 [기존 snapshot](../authz/protected_route_inventory.snapshot) 및 [라우트 원본](../../src/web/rest/routes)과 일치한다. 여기에 `/auth/v1` 3개, `/internal` 3개, `/health` 1개를 더했다. 자동 HEAD/OPTIONS, 정적 파일 제공, fallback, 미등록 legacy URL은 분모에 넣지 않는다. 예를 들어 삭제된 Presave legacy URL의 404 검사는 [Presave 명세](presave.md)에 별도로 남겼다.

## 판정 읽는 법

| `status` | 필요한 근거 | 뜻하지 않는 것 |
|---|---|---|
| `asserted` | 활성 테스트의 해당 HTTP 요청과 성공/오류 status, body 또는 상태 변화 assertion. 저장 후 다른 조회 API/DB에서 결과를 직접 확인하는 경우 포함 | 모든 필드·오류·권한·경계·감사가 검사됨, 현재 실행 PASS |
| `authorization_only` | 실제 HTTP 요청의 권한 거절 또는 조직 가시성 assertion | 정상 업무 기능과 응답 schema 검증 |
| `registration_only` | 활성 `protected_inventory_matches_checked_in_snapshot`의 method/path 등록 검사 | 실제 HTTP 호출, 인증·인가 또는 저장 동작 검증 |
| `uncovered` | 등록 소스는 있지만 위 세 종류의 활성 근거를 찾지 못함 | 구현이 반드시 고장났다는 뜻 |

**262개는 전체 계약을 검증한 API 수가 아니다.** 성공 status 하나만 확인하는 경로도 포함된다. 각 행의 `gaps`와 연결된 명세에서 실제 assertion 범위를 확인해야 한다. 167개는 행동 검사가 없고, 6개는 권한 검사만 있다. 행·분기 커버리지 비율이나 테스트 통과율로 환산하지 않는다.

`coverage.tsv` 열은 다음과 같다.

| 열 | 의미 |
|---|---|
| `method`, `path` | router에 등록된 그대로의 operation 식별자. `{id}` 등 parameter 이름도 유지 |
| `status` | 위 네 가지 근거 분류 |
| `spec` | 이 디렉터리의 명세 파일과 명시적 section anchor |
| `evidence` | `web-server` 기준 `tests/파일.rs::테스트함수`; 여러 근거는 `;`로 구분. 근거가 없으면 `-` |
| `gaps` | 확인하지 않는 동작·필드·권한·경계. 전체 가능한 오류 조합의 목록은 아님 |

`evidence`는 분류를 뒷받침하는 직접 근거다. 한 operation을 건드리는 모든 테스트의 역색인은 아니다. 공통 setup이 요청에 성공했다는 사실만으로 해당 API의 계약 검증으로 올리지 않았다. 반대로 생성 응답과 영속 결과를 명시적으로 비교하는 교차 API 검사는 근거로 연결했다. 서로 다른 URL이나 method는 같은 저장 모델을 사용하더라도 별도로 판정한다.

## 테스트 구조와 연결

| 실행 단위 | 연결 명세 |
|---|---|
| `api_cases` | Case, 일부 설정·Presave scope·Editor shell |
| `api_editor` | Editor, legacy resource 생성의 명시적 readback, audit projection |
| `api_presave` | Presave |
| `api_import` | Import |
| `api_submission` | Submission, 일부 Case validation·저장 |
| `api_terminology` | Terminology |
| `api_administration` | Administration, Authorization |
| `authz` | HTTP 인증·권한·RLS, 보호 route inventory |
| `xml` | Export와 Import 관련 HTTP 검사 및 별도 XML 변환 검사 |

`tests/<target>.rs`와 같은 이름의 디렉터리가 기존 Cargo 실행 단위다. 이 명세 디렉터리는 실행 단위를 새로 만들지 않는다. 순수 모델/DB 검사는 `lib-core`, XML 변환·XSD 검사는 `xml`, 공통 fixture/helper와 isolation guard는 각각의 책임을 유지한다. 직접 HTTP assertion이 없는 모델·helper 검사를 특정 API의 실행 근거로 계산하지 않는다.

Editor의 필드별 계약 원본은 [registry/editor-contracts](../../../../../registry/editor-contracts)이고, `pages/*.generated.rs`는 실행 등록이다. 명세는 [fields](../api_editor/fields), [rows](../api_editor/rows), [relations](../api_editor/relations), [audit](../api_editor/audit)의 검증과 연결된다. scalar 365개 + 동작 50개 = 415개 등록, canonical UI 필드 360개, Editor operation 55개는 서로 다른 집계다.

## 먼저 보강할 공백

1. **저장·삭제·복원:** Case 하위 리소스의 개별 item mutation, Presave direct child CRUD의 빈 method. 같은 변경 요청에서 저장/clear, ID·부모 소속, 거절 후 상태 불변, 삭제/복원과 감사 기록을 확인해야 한다. Editor PATCH 검사만으로 legacy CRUD 검증을 대신하지 않는다.
2. **감사·권한 경계:** 손상된 감사 chain의 HTTP 보고, 조직 목록·수정·삭제의 정상 경로, workflow 사용자 옵션의 성공 응답, 실제 요청을 통한 CSRF 검사. SQL hash-chain 검사나 origin helper unit만으로 이 경계를 입증할 수 없다.
3. **입출력 경계:** `/api/import/xml/validate`, CIOMS PDF의 전체 필드·한글 렌더링·페이지 잘림. ZIP/request 상한, 부분 성공 결과와 오류 이력, 모든 권한·조직 조합도 추가 근거가 필요하다. MedDRA는 reader의 250 MiB와 현재 router의 2 MiB body 기본 제한이 다르므로 [명세](terminology.md#imports)의 한도를 구분해서 봐야 한다.
4. **등록만 남은 조회:** case-query catalog/search alias·link-options, 이전 Editor GET 14개와 별도 POST restore 3개, 사전 검색 3개. 사용 중인 API의 검사 보강과 미사용 API 정리는 호출자를 확인한 뒤 별도 작업으로 결정한다.
5. **실행 증거의 빈틈:** ignored Java 연동과 외부 fixture 준비, 실행 로그가 필요한 XML 검사. Import의 컴파일 제외 3개는 중복 1개 삭제·필요 2개 활성화로 정리했고 XML 조기 성공 분기를 제거했다. [Import](import.md), [Export](export.md), [Submission](submission.md)의 제한을 유지하며, 대역 전송 성공을 기관 수용 증거로 사용하지 않는다. `/health`도 직접 HTTP 검사가 없다.

## 실제 화면 호출을 기준으로 한 다음 순서

frontend `d4eee87e55cf955058fc70e09ea1559cc90b9419`의 endpoint 정의와 app/component/hook 소비자를 대조했다. helper 선언만으로 사용 중이라고 판정하지 않았다.

| 우선 대상 | 실제 소비자 | 남은 검사 |
|---|---|---|
| field-notation GET/PUT/DELETE | `E2BFormField.tsx`, `field-notations.ts` | 정상 lifecycle·ID·감사 action·viewer 거절 후 불변 추가. recordId 존재·case/org 소속 검증과 입력 경계는 남음 |
| CIOMS PDF GET | submission 화면과 Editor action, `xml.ts` | HTTP 권한·content type·파일명·PDF byte 구조·case 번호 추가. 전체 필드·한글·잘림·notation 옵션은 남음 |
| Case query catalog·link-options | submission 화면, `useCaseLinkOptions.ts` | 응답 필드·사용자 scope·검색 필터 |
| Presave study 조회·삭제 | `usePresaveTemplates.ts`, `useCanonicalPresaveBase.ts`, `InfoPresaveListRoute.tsx` | 실제 DELETE와 하위 관계·권한·감사 |
| 사전 검색 3개 | active substance·device problem dialog·UCUM hook | 실제 선택에 쓰이는 검색 결과와 빈/잘못된 질의 |

import error download는 import 화면에서 사용하므로 비활성 검사를 이번에 복구했다. 반면 `/api/import/xml/validate`, atomic restore, 상당수 legacy item mutation은 현재 frontend에서 실행 소비자를 확인하지 못했다. 외부 client나 동적 경로가 없다는 증거는 아니므로 삭제하지 않는다. 감사 무결성 API는 frontend 호출 여부와 별개로 정상 report와 viewer 403 검사를 추가했다. 손상된 chain의 HTTP 보고 분기는 남는다.

구조 검토에서는 import의 요청/이력/설정, XML의 다운로드/재조회/수정/오류 책임을 분리하고 복제 준비 함수를 공용화했다. Presave의 aggregate CRUD, Case validation helper, XML의 reaction 검사는 줄 수만으로 쪼개지 않았다. `api_cases/contract/query.rs`는 검색 결과 계약, `list_view.rs`는 목록 투영·페이지 처리를 담당한다. 기존 `case_query_web.rs`라는 별도 파일은 없다. 감사 chain의 순수 SQL 동시성 검사는 `lib-core/tests/database/audit_chain.rs`, 감사 HTTP 검사는 `api_administration/audit_web.rs`에 둔다. authz 5개 파일의 동일 Case 생성 helper는 `authz/support.rs` 하나로 모았다. 호출되지 않는 validation helper 30개와 scope helper 5개, 죽은 context 필드 2개 및 이를 숨기던 dead_code 허용을 제거했다. 실제 테스트 선언은 삭제하지 않았다.

## 파일 구조 전수 검토에서 남은 항목

9개 실행 단위(`api_cases`, `api_editor`, `api_presave`, `api_import`, `api_submission`, `api_terminology`, `api_administration`, `authz`, `xml`)의 모든 하위 Rust 파일과 root 모듈 연결을 확인했다. 미등록 파일은 없지만 **책임 분리가 전부 끝난 상태는 아니다.** 다음은 검사 부재와 구별되는 배치 문제다.

| 파일 | 확인한 문제 | 다음 정리 위치 |
|---|---|---|
| [rbac_users/create_validation_web.rs](../authz/rbac_users/create_validation_web.rs), [update_delete_web.rs](../authz/rbac_users/update_delete_web.rs) | 권한 거부와 일반 사용자 생성·입력·날짜·비밀번호 계약이 혼재 | 권한 정책은 authz, 일반 사용자 API는 api_administration |
| [update_delete_web.rs](../authz/rbac_users/update_delete_web.rs)의 `test_expired_user_is_soft_deactivated` | HTTP 없이 UserBmc update/deactivate/get만 검사 | lib-core 사용자 모델/DB 실행 단위; web 전용 seed 의존을 먼저 대체 |
| [rls_web.rs](../authz/rls_web.rs)의 `device_rows_are_tenant_scoped_and_targeted_export_ignores_org_volume` | 직접 BMC tenant scope와 XML targeted export를 한 검사에서 단언 | lib-core RLS와 XML export 계약 분리; 기존 양쪽 assertion 보존 필요 |
| [api_cases/settings.rs](../api_cases/settings.rs) | admin/settings만 검사하는 5개가 Case target에 남음 | api_administration/admin_settings; Case workflow/config와 case-number downstream 검사는 유지 |
| [auth_mw_audit.rs](../authz/auth_mw_audit.rs) | 로그인·middleware와 Case CRUD 감사 검사가 같은 파일 | Case 감사 검사를 해당 계약 파일로 이동; viewer 거절 단순 검사는 기존 권한 matrix와 대조 |

제출 재처리는 `reconcile/web.rs`의 내부 HTTP 계약과 `reconcile/service.rs`의 worker 계약으로 나눴다. worker 구현은 web-server 소유이므로 lib-core로 옮기지 않는다. HTTP 준비 호출만 존재하는 service 검사는 API 직접 근거로 올리지 않는다. Editor의 generated 등록과 수기 검사 구현, scope 정책의 여러 endpoint 비교, Presave aggregate CRUD, XML parser/serializer 검사는 책임이 이어져 있어 줄 수만으로 나누지 않는다.

## 변경 시 유지할 기준

API를 추가·변경·삭제하면 router/DTO와 해당 명세, `coverage.tsv`, 실제 assertion을 함께 검토한다. route 이름이 같아도 body·상태·권한 변화가 있으면 명세를 갱신한다. 분류를 올릴 때는 helper나 테스트 이름이 아니라 실제 요청과 assertion을 연결하고, ignored/컴파일 제외/조기 성공 반환 여부를 확인한다.

이번 통합에서는 GPT-5.6 Sol 하위 에이전트가 Case·Presave·Import·Terminology·Administration·Authorization을 작성·교차 검토했고, 상위 에이전트가 router, handler/DTO, 테스트 본문과 연결을 대조했다. Editor·Submission·Export도 같은 기준으로 합쳤다. 전체 operation 중복/누락, 활성 테스트 함수·등록, 문서 anchor·상대 링크는 정적으로 확인했다.

이 자료는 감사 시 **무엇을 검사하도록 구현했는지와 공백**을 추적하는 근거다. 실제 통과를 입증하려면 같은 commit의 CI 실행 로그, 선택된 테스트 목록과 결과, 실행 환경·의존 데이터 버전이 추가로 필요하다. 현재 자료만으로 규제 적합성이나 완전한 데이터 무결성을 인증하지 않는다.

명세 `.md`와 `coverage.tsv`만 바뀌면 CI의 Rust 검사 선택에서 제외된다. 같은 디렉터리라도 실행 소스 변경은 제외하지 않으며, 실제 테스트/production 변경은 기존 영향 범위 선택을 따른다.
