# XML·CIOMS export API 명세와 검사 범위

현재 등록된 export 경로는 XML 생성 2개, export 이력 3개, CIOMS 1개로 모두 6개다. 실행 단위는 `xml`, 일부 회귀·권한 검사는 `api_submission`, `api_cases`, `api_administration`에 있다. 현재 소스의 assertion을 기록하며 실행 결과는 **NOT_RUN**이다. Import API는 [import](import.md), 제출 당시 원본 XML은 [submission](submission.md#history)에서 관리한다.

구현: [case_export_rest](../../src/web/rest/case_export_rest.rs), [CIOMS handler](../../src/web/rest/cioms_export_rest/build.rs), [라우트](../../src/web/rest/routes/cases.rs).

<a id="xml"></a>
## XML 생성

| Method / path | 요청 | 구현 응답·입력 경계 |
|---|---|---|
| `GET /api/cases/{id}/export/xml` | UUID; 필수 query `authority=ich|fda|mfds`, 선택적 `include_notation` bool | 200 `application/xml` attachment. 실제 저장값·message header를 읽어 생성. authority 누락/오류 및 필수 header/serialization 실패와 생성된 N envelope의 기관별 수신처·송신처·일치 규칙 위반은 BadRequest |
| `POST /api/cases/export/xml` | `{"case_ids":["UUID"],"authority":"fda"}` (최상위 필드, `data` 없음; 두 필드 모두 의미상 필수) | 200 `application/zip` attachment. 구현은 중복 ID를 합치고 최대 100 Case·전체 비압축 XML 100 MiB, 빈 목록을 거절. 권한은 중복 제거한 Case 집합 전체에 적용 |

기존 테스트가 직접 확인하는 조건:

- [N 다운로드 검사](../xml/import_export/rejections.rs)의 `test_manual_export_checks_n_routes_without_blocking_draft_cases`: 잘못된 FDA/MFDS 수신처는 단건·ZIP 모두400, 실패 이력4건을 보존한다. 정상 ICH/FDA/MFDS header는 Case business 오류가 있는 draft도 다운로드하며 ZIP 내용도 확인한다. N 검사만 공유하며 전체 제출용 Case/XSD 검사를 다운로드에 강제하지 않는다. FDA 수신처 허용값은 `registry/sources/fda-core-regional-data-elements-v1.csv`의 N.1.4/N.2.r.3, 조합은 rejection rules의 R0004~R0007과 대조한다.
- [XML gate](../xml/import_export/rejections.rs)의 `test_manual_export_rejects_missing_headers_and_records_errors`: 단건 GET·bulk POST 모두 batch receiver 누락을 400으로 거절하고 오류 문구를 보존한다. 해당 값을 수정해도 불완전한 FDA XML은 400 `export task failed`; 실패 이력도 보존한다. 모든 FDA XSD 경계를 검사한다고 주장하지 않는다.
- [submission validation](../api_submission/validation.rs)의 `test_manual_export_is_not_blocked_by_workflow_status`: draft/reviewed/locked에서 단건 수동 export 200. `test_unavailable_meddra_allows_save_and_export_but_blocks_submission`: ICH/FDA/MFDS export에 저장된 미적재 MedDRA version 문자열이 남고 제출만 거절된다.
- [roundtrip profiles](../xml/roundtrip_profiles_web.rs)의 `test_roundtrip_fixtures_import_validate_export_revalidate`: FDA 원본 fixture 3개를 ICH/FDA/MFDS profile로 import→API export→XSD·export-rule 재검증한다. 업무 validation의 ok=false만으로 export를 생략하지 않는다. 이것을 MFDS 공식 원본 샘플이나 기관 수용 검증으로 간주하지 않는다.
- [다운로드](../xml/import_export/downloads.rs)의 `test_import_then_export_xml`: 단건 XML, FDA/MFDS notation, 중복 Case ID를 제거한 ZIP 2개 엔트리의 파일명·본문, 거절 응답과 성공/실패 이력. [약물 수정 반영](../xml/import_export/drug_updates.rs)의 `test_import_update_dg_fields_then_export_contains_updates`: 저장/수정값의 XML 반영.
- [transfer permission](../api_administration/role_admin/effective_access/transfer_web.rs)의 `test_export_submission_matrix_privileges_grant_effective_xml_export_permission`: bulk export의 read-only 403, execute privilege로 403 경계 통과. `assert_ne!(403)`는 ZIP 성공이나 XML 내용의 증거가 아니다.

미검사 범위: bulk 100/101 경계·100 MiB 경계·권한이 섞인 여러 Case·ZIP 각 엔트리 완전성의 모든 조합을 위 테스트로 보장하지 않는다. 구현상 제한과 실제 assertion을 구분한다. XML 라이브러리의 직접 함수 호출 검사는 HTTP 인증·응답 계약을 대체하지 않는다.

<a id="history"></a>
## Export 이력·오류 다운로드

| Method / path | 구현 응답 | assertion |
|---|---|---|
| `GET /api/cases/{id}/exports/history` | 200 `data.items` | 실패 4건, 오류 상태·Case ID·사용자·파일명·오류 내용. 실패한 export 트랜잭션과 별도로 이력이 남음 |
| `GET /api/exports/history` | 200 `data.items` | 오류 이력 조회, custom profile 권한과 사용자 Case scope 필터 |
| `GET /api/exports/history/{id}/error.txt` | 200 `text/plain; charset=utf-8`, attachment | content type·파일명·본문이 저장된 오류 메시지와 일치 |

근거: [XML gate](../xml/import_export/rejections.rs)의 `test_manual_export_rejects_missing_headers_and_records_errors`; [scope histories](../api_cases/scope/histories.rs)의 전체 이력 사용자 범위 검사; 위 transfer permission 검사. 구현상 없는 이력·오류 내용 없는 이력의 BadRequest와 파일명 sanitization은 별도의 모든 입력 assertion이 없다. 단건·bulk 생성에서 history 기록 실패는 warning만 남기고 성공 다운로드를 유지하는 best-effort 구현이며, 이 실패 분기는 HTTP assertion이 없다. 오류 이력은 일반 변경 `audit_logs` old/new 검증과 같은 것으로 계산하지 않는다.

<a id="cioms"></a>
## CIOMS PDF

`GET /api/cases/{id}/export/cioms.pdf`는 선택적 query `include_notation`을 받아 Case·설정을 읽고 200 `application/pdf`, `{case_number}-cioms.pdf` attachment를 반환한다. `with_authorized_case_export`를 거친다. [cioms_export.rs](../api_cases/cioms_export.rs)의 활성 HTTP 검사는 실제 case·reaction·suspect drug·dechallenge assessment를 만든 뒤 200, content type, attachment 파일명, PDF header/EOF/xref/page tree와 case 번호 byte 포함을 확인한다. 별도 검사에서 viewer 403, 다른 조직 case 403, 없는 case 404를 단언한다.

이는 PDF parser나 화면 렌더링 검사가 아니다. 전체 필드 매핑·한글 font 렌더링·페이지 잘림·notation 옵션·dechallenge 오류 HTTP 응답은 미단언이다. font는 [font.rs](../../src/web/rest/cioms_export_rest/font.rs)가 저장소의 NotoSansCJKkr-Regular.otf를 `include_bytes!`로 포함하며 템플릿은 소스 상수다. 필수 자산 누락은 build 오류이며 테스트를 skip하거나 성공 대역으로 바꾸지 않는다.

<a id="limits"></a>
## 실행 조건과 증거의 한계

- `xml/import_export/`는 `downloads.rs`(단건·ZIP·notation·권한·이력), `nullflavor.rs`(import 후 E/F 재조회), `drug_updates.rs`(약물 수정의 XML 반영), `rejections.rs`(필수값 오류·실패 이력·오류 다운로드)로 나뉜다. 실행 단위는 모두 기존 `xml`이며 Cargo target을 늘리지 않는다.
- 공통 HTTP 준비·경로·반응 언어·message header는 [xml/support.rs](../xml/support.rs)에서 공유한다. field roundtrip의 실제 device 준비는 그 모듈에 남긴다. validator가 읽지 않는 legacy device-characteristics를 쓰던 별도 준비는 수동 export와 무관해 제거했다.
- fixture 누락, setup/import/export 실패, malformed 필수 응답은 성공으로 끝내지 않는다. 감사 실패/deadlock의 테스트 내부 재시도를 제거했다. 수동 export에 불필요한 `mark-validated` 준비도 제거했다. 제출 검증과 export의 필수값/XSD 검사는 각자의 책임에 남는다.
- [CI/SI roundtrip](../xml/roundtrip/report.rs)은 study type의 XML 반영을 확인한다. N.2.r.1/N.2.r.4는 C.1.1/C.1.2에서 파생되므로 별도 message-header의 legacy number/date를 export 기대값으로 사용하지 않는다.
- [MFDS samples](../xml/mfds_import_samples_web.rs)는 공식 원본 fixture가 필요하며 디렉터리 부재/빈 목록도 실패다. [Import 명세](import.md)에 공식 첨부 URL·SHA-256·CI checkout 경로를 기록했다.
- `xml`의 `fda_export_reads_persisted_sections_through_rls`, `fresh_full_build_validates_for_all_authorities` 등 직접 모델/XML 호출 검사는 저장·XML 계층 근거다. API method/path별 HTTP 검증으로 중복 집계하지 않는다.

위 변경의 실행 결과는 **NOT_RUN**이다. 조기 성공 제거는 검사 코드의 신뢰성을 개선하지만, 실제 XML·DB·기관 검증 통과를 증명하지 않는다.
