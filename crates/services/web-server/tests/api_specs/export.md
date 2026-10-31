# XML·CIOMS export API 명세와 검사 범위

실행 단위는 `xml`, 일부 회귀·권한 검사는 `api_submission`, `api_cases`, `api_administration`에 있다. 현재 소스의 assertion을 기록하며 실행 결과는 **NOT_RUN**이다. Import API는 [import](import.md), 제출 당시 원본 XML은 [submission](submission.md#history)에서 관리한다.

구현: [case_export_rest](../../src/web/rest/case_export_rest.rs), [CIOMS handler](../../src/web/rest/cioms_export_rest/build.rs), [라우트](../../src/web/rest/routes/cases.rs).

<a id="xml"></a>
## XML 생성

| Method / path | 요청 | 구현 응답·입력 경계 |
|---|---|---|
| `GET /api/cases/{id}/export/xml` | UUID; 필수 query `authority=ich|fda|mfds`, 선택적 `include_notation` bool | 200 XML attachment. 실제 저장값·message header를 읽어 생성. authority 누락/오류 및 필수 header/serialization 실패는 BadRequest |
| `POST /api/cases/export/xml` | `{"case_ids":["UUID"],"authority":"fda"}` (최상위 필드, `data` 없음) | 200 ZIP attachment. 구현은 중복 ID를 합치고 최대 100 Case·전체 XML 100 MiB, 빈 목록을 거절. 권한은 선택한 Case 집합 전체에 적용 |

기존 테스트가 직접 확인하는 조건:

- [XML gate](../xml/xml_import_export.rs)의 `test_fda_export_always_validates_even_when_env_unset`: 단건 GET·bulk POST 모두 batch receiver 누락을 400으로 거절하고 오류 문구를 보존한다. 해당 값을 수정해도 불완전한 FDA XML은 400 `export task failed`; 실패 이력도 보존한다. 함수명만으로 모든 FDA XSD 경계를 검사한다고 주장하지 않는다.
- [submission validation](../api_submission/validation.rs)의 `test_manual_export_is_not_blocked_by_workflow_status`: draft/reviewed/locked에서 단건 수동 export 200. `test_unavailable_meddra_allows_save_and_export_but_blocks_submission`: ICH/FDA/MFDS export에 저장된 미적재 MedDRA version 문자열이 남고 제출만 거절된다.
- [roundtrip profiles](../xml/roundtrip_profiles_web.rs)의 `test_roundtrip_fixtures_import_validate_export_revalidate`: 파일 기반 import→API export→재검증. fixture·XSD를 사용하는 범위이며 기관 수용 검증이 아니다.
- [XML import/export](../xml/xml_import_export.rs)의 `test_import_then_export_xml`, `test_import_update_dg_fields_then_export_contains_updates`: 저장/수정값의 XML 반영·내보내기 이력. 아래 조기 성공 조건 때문에 무조건 실행되는 증거로 읽지 않는다.
- [transfer permission](../api_administration/role_admin/effective_access/transfer_web.rs)의 `test_export_submission_matrix_privileges_grant_effective_xml_export_permission`: bulk export의 read-only 403, execute privilege로 403 경계 통과. `assert_ne!(403)`는 ZIP 성공이나 XML 내용의 증거가 아니다.

미검사 범위: bulk 100/101 경계·100 MiB 경계·중복 ID·권한이 섞인 여러 Case·ZIP 각 엔트리 완전성의 모든 조합을 위 테스트로 보장하지 않는다. 구현상 제한과 실제 assertion을 구분한다. XML 라이브러리의 직접 함수 호출 검사는 HTTP 인증·응답 계약을 대체하지 않는다.

<a id="history"></a>
## Export 이력·오류 다운로드

| Method / path | 구현 응답 | assertion |
|---|---|---|
| `GET /api/cases/{id}/exports/history` | 200 `data.items` | 실패 4건, 오류 상태·Case ID·사용자·파일명·오류 내용. 실패한 export 트랜잭션과 별도로 이력이 남음 |
| `GET /api/exports/history` | 200 `data.items` | 오류 이력 조회, custom profile 권한과 사용자 Case scope 필터 |
| `GET /api/exports/history/{id}/error.txt` | 200 `text/plain; charset=utf-8`, attachment | content type·파일명·본문이 저장된 오류 메시지와 일치 |

근거: [XML gate](../xml/xml_import_export.rs)의 `test_fda_export_always_validates_even_when_env_unset`; [scope histories](../api_cases/scope/histories.rs)의 전체 이력 사용자 범위 검사; 위 transfer permission 검사. 구현상 없는 이력·오류 내용 없는 이력의 BadRequest와 파일명 sanitization은 별도의 모든 입력 assertion이 없다. 오류 이력은 일반 변경 `audit_logs` old/new 검증과 같은 것으로 계산하지 않는다.

<a id="cioms"></a>
## CIOMS PDF

`GET /api/cases/{id}/export/cioms.pdf`는 선택적 query `include_notation`을 받아 Case·설정을 읽고 200 `application/pdf`, `{case_number}-cioms.pdf` attachment를 반환한다. `with_authorized_case_export`를 거친다. 현재 조사한 API 테스트에는 이 endpoint의 직접 HTTP assertion이 없다. 라우트 등록 확인과 PDF 생성 함수 단위 검사가 있더라도 API 응답·권한·필드·한글 font·페이지 잘림 검증 완료로 표시하지 않는다.

<a id="limits"></a>
## 실행 증거의 한계

- [xml_import_export](../xml/xml_import_export.rs)의 두 import/export 검사는 `E2BR3_EXAMPLES_DIR` 부재 시 `return Ok(())`로 끝날 수 있다. 테스트 이름의 `ok`만으로 fixture 경로 실행을 입증할 수 없다.
- [MFDS samples](../xml/mfds_import_samples_web.rs)의 `test_mfds_samples_import_and_validate`는 디렉터리·XML 파일이 없으면 성공 반환한다.
- [roundtrip/drug](../xml/roundtrip/drug.rs), [reaction](../xml/roundtrip/reaction.rs), [patient](../xml/roundtrip/patient.rs)의 일부 검사는 초기화 실패를 잡아 성공 반환한다. [report](../xml/roundtrip/report.rs), [narrative](../xml/roundtrip/narrative.rs) 등을 포함한 일부 XML 생성 오류 분기도 검사 후 성공으로 종료한다. 각 함수의 성공 경로를 실제 로그로 구별해야 한다.
- `xml`의 `fda_export_reads_persisted_sections_through_rls`, `fresh_full_build_validates_for_all_authorities` 등 직접 모델/XML 호출 검사는 저장·XML 계층 근거다. API method/path별 HTTP 검증으로 중복 집계하지 않는다.

이번 작업은 이 한계를 명세에 드러내는 작업이다. 기존 테스트의 skip/error 처리를 변경하거나 실행하지 않았다.
