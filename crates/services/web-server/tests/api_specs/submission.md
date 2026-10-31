# Submission API 명세와 검사 범위

대상 실행 단위는 `api_submission`이며 Case의 message-header·권한 검사는 `api_cases`, `api_administration`에도 있다. 아래는 현재 코드에 존재하는 assertion 명세다. 실행 결과는 **NOT_RUN**이다. 제출 테스트의 `start_mock_esg`/`configure_mock_as2_transport`는 기존 로컬 응답 대역이다. 이 테스트로 외부 FDA/MFDS 접수나 실제 AS2/Java 연동을 증명하지 않는다.

라우트 원본: [Case routes](../../src/web/rest/routes/cases.rs), [submission routes](../../src/web/rest/routes/submissions.rs). 요청·응답 원본: [submission_rest](../../src/web/rest/submission_rest.rs), [전자서명 입력](../../src/web/rest/compliance.rs).

<a id="submit"></a>
## 제출과 재시도 식별

| Method / path | 요청 | 구현 응답 / 검사하는 조건 |
|---|---|---|
| `POST /api/cases/{id}/submissions/fda` | JSON 최상위 `reason_for_change`, `e_signature`; 선택적 `x-idempotency-key` | 201 `{"data": SubmissionRecord}`. 같은 키의 반복·병렬 호출은 같은 submission ID와 목록 1개. 전송 결과 불명은 `dispatch_unknown`, 원격 ID·ACK null로 보존 |
| `POST /api/cases/{id}/submissions/mfds` | 같은 compliance 본문 | 201. 요청 URL의 기관이 Case appendices와 별도로 제출 기관을 결정. MFDS 로컬 AS2 대역 응답 ID 확인 |
| `GET /api/cases/{id}/submissions` | Case UUID | 200 `data.items`. 재시도 후 중복 없이 1개이고 불명 결과가 그대로 보임 |

`e_signature`는 `meaning`과 `password` 문자열이며, 변경 사유·의미·비밀번호는 공백만으로 구성될 수 없다. 비밀번호 재확인과 서명 저장은 위 compliance 구현을 따른다. 본문은 일반 CRUD의 `data` envelope가 아니다. 구현은 `submission.execute` 권한과 Case mutation 경계를 거쳐 서명을 기록하고 제출을 예약한 뒤 전송한다. `201` 자체는 규제기관 접수 성공을 뜻하지 않는다.

검사 근거:

- [idempotency](../api_submission/idempotency.rs): `test_submission_idempotency_key_reuses_submission_when_enabled`, `test_submission_idempotency_key_parallel_requests_single_submission` — 응답 ID 동등성·GET 목록 1개.
- [dispatch](../api_submission/dispatch.rs): `test_submission_esg_transport_sends_expected_headers_and_payload` — 대역으로 보내는 bearer/API-key와 비어 있지 않은 XML; `test_submission_esg_non_success_preserves_unknown_without_resend` — 502에도 재전송 없이 unknown·한 번의 전송·retry 없음·오류 보존; `test_submission_retains_unknown_when_as2_submitter_unreachable` — 미접속 상태·ACK1~4 null·동일 키 재사용.
- 같은 파일의 `test_submission_rejects_enabled_esg_without_base_url` — 400 및 누락 설정명. `test_submission_accepts_mfds_route_for_mfds_profile`, `test_submission_uses_request_authority_not_case_appendices` — MFDS 요청 기관 처리.
- [validation](../api_submission/validation.rs): `test_submission_rejects_case_validation_before_routing_or_dispatch` — 변경으로 캐시가 무효화되고 FDA/MFDS 둘 다 400 + validation issue; `test_unavailable_meddra_allows_save_and_export_but_blocks_submission` — 사전 버전 문제는 저장/수동 export를 허용해도 제출은 400. `test_submission_does_not_require_case_validated_status` — Case status 문자열만으로 사전 차단하지 않음.

공백: 외부 규제기관 ACK 수용, 실제 transport 전 구간, 키의 모든 충돌/사용자·기관 조합, 서명 실패 후 모든 테이블·감사의 원자적 불변을 이 목록만으로 보증하지 않는다. [external](../api_submission/external.rs)의 `test_real_java_submitter_integration_mfds`는 `#[ignore]`이며 일반 CI 실행 증거가 아니다.

<a id="history"></a>
## 제출 이력·상태·원본 다운로드

| Method / path | 구현 응답 | 실제 assertion / 공백 |
|---|---|---|
| `GET /api/submissions/history` | 200 `data.items` | 최신 ACK 시간·이벤트, 결과, ICSR 수, 원본 파일 URL; 사용자 scope 필터 |
| `GET /api/submissions/{id}` | 200 `data` submission | callback 후 `ack3_received` 재조회 |
| `GET /api/submissions/{id}/events` | 200 `data.items` | 생성·ACK·상태변경 이벤트, 중복 callback의 `ack_duplicate_ignored` |
| `GET /api/submissions/{id}/dispatch-state` | 200 `data.state` | 불명 전송의 retry 없음·오류 메시지. callback 후 HTTP 200을 요구하고 attempt_count=1, last_attempt_at·terminal_at 존재를 확인 |
| `GET /api/submissions/{id}/acks/{level}/download` | 200 `text/plain` | ACK2 텍스트의 submission/level/code/message. 모든 level과 없는 ACK의 오류는 별도 공백 |
| `GET /api/submissions/{id}/xml` | 200 저장된 원본 XML bytes | 저장 원본과 byte 동등, 무인증 비성공·타 조직 404; DB 원본 변경 시도 거절. 현재 Case를 다시 export한 결과와 혼동하지 않음 |

[history](../api_submission/history.rs)의 `test_submission_history_includes_latest_ack_time_and_event`, `test_submission_ack_can_be_downloaded_as_text`; [callbacks](../api_submission/callbacks.rs)의 `test_internal_ack_callback_updates_submission_by_remote_id`, `test_internal_ack_callback_duplicate_payload_is_idempotent`; [dispatch](../api_submission/dispatch.rs)의 `test_submission_retains_unknown_when_as2_submitter_unreachable`가 해당 근거다. 이력 scope는 [histories](../api_cases/scope/histories.rs)의 실제 endpoint loop도 참조한다.

원본 XML 보존 검사는 HTTP 다운로드와 DB 변경 거절을 함께 확인하지만, 모든 이력 API에 동일한 권한·감사·파일 헤더 assertion이 있다는 뜻은 아니다. 다운로드 audit 기록의 모든 old/new·사용자·사유를 이 API군에서 직접 비교하지 않는다.

<a id="receiver"></a>
## 수신처와 메시지 헤더

| Method / path | 요청 / 응답 | 검사 범위 |
|---|---|---|
| `GET /api/submissions/receiver-options` | 필수 query `authority=fda|mfds`; 200 `data.items` | FDA Postmarket 식별자와 MFDS 5개 조건/수신처 식별자. 잘못된 authority의 구현 400은 소스 조건이며 전 값 테스트가 아님 |
| `PUT /api/cases/{id}/submission-receiver` | `{"data":{"authority":"mfds","receiver_label":"MFDS(KR)"}}`; 200 `data` MessageHeader | draft/reviewed/validated/locked에서 기관·수신처 적용, 메시지/배치 번호·sender·date 재조회; execute 권한 |
| `POST /api/cases/{case_id}/message-header` | `data`에 header 필드; 신규 201, 기존 200 | 중복 POST가 기존 ID·기존 값 보존 |
| `GET /api/cases/{case_id}/message-header` | 200 `data`; 미생성 404 | POST 및 수신처 적용 후 재조회 |
| `PUT /api/cases/{case_id}/message-header` | `data` 변경 필드; 200 `data` | XML 전제조건을 수정하는 경로 및 intake 회귀의 일부 assertion; 모든 헤더 필드 clear/감사 계약은 아님 |
| `DELETE /api/cases/{case_id}/message-header` | 구현상 204 | 등록 확인 외 직접 삭제 assertion은 찾지 못함. 자동으로 다른 singleton DELETE 검사에 포함하지 않음 |

근거: [headers](../api_submission/headers.rs)의 `test_submission_receiver_options_list_defaults_by_authority`, `test_submission_receiver_selection_updates_message_header_n_identifiers`; [singleton](../api_cases/subresources/singleton.rs)의 `test_singleton_post_endpoints_are_idempotent`; [XML gate](../xml/import_export/rejections.rs)의 `test_manual_export_rejects_missing_headers_and_records_errors`; [transfer permissions](../api_administration/role_admin/effective_access/transfer_web.rs)의 제출 수신처 변경 검사.

`concurrent_authority_header_preparation_returns_request_scoped_values`는 같은 Case에 FDA/MFDS HTTP PUT을 `tokio::join!`으로 보내고, 두 응답의 200과 기관별 sender/receiver를 각각 확인한다. 응답이 다른 요청의 기관 값으로 섞이지 않는 검사이며, 모든 동시 변경 조합이나 최종 저장 순서까지 보장하지 않는다.

<a id="internal"></a>
## 내부 ACK·reconcile API

이 경로는 일반 사용자 쿠키 middleware 대신 명시적인 machine token을 검사한다. `x-callback-token`은 `AS2_CALLBACK_TOKEN`, `x-reconcile-token`은 `SUBMISSION_RECONCILE_TOKEN`과 비교한다. 실제 토큰 값은 명세에 넣지 않는다.

| Method / path | 요청 | 응답·검사 |
|---|---|---|
| `POST /internal/submissions/callbacks/ack` | 최상위 `remote_submission_id`, `ack_level`, `success`, 선택적 `ack_code`, `ack_message` 및 callback header | 200 `data` submission; ACK3 상태·재조회·이벤트. 같은 payload 재호출 시 중복 이벤트. 토큰 누락/오류 400. `ack_level` 누락 검사는 400 또는 422를 허용하므로 단일 오류코드 보증이 아님 |
| `POST /internal/submissions/reconcile` | reconcile header; 선택적 JSON `limit` (구현 기본 25) | 200 `data.result`의 attempted/processed ID, 빈 결과·인증, HTTP reconcile 뒤 ESG 중복 전송 방지. legacy/AS2는 아래 service 검사와 구분 |
| `GET /internal/submissions/reconcile/status` | reconcile header | 200 `data.status`; total_runs 증가와 total_attempted 타입. 프로세스 재시작 이후 누적 보존 검사는 아님 |

근거: [callbacks](../api_submission/callbacks.rs)의 `test_internal_submission_endpoints_require_valid_callback_token`, `test_internal_ack_callback_rejects_malformed_payload`, 앞의 update/duplicate 검사; [reconcile](../api_submission/reconcile.rs)의 `test_internal_reconcile_endpoint_auth_and_empty_result`, `test_internal_reconcile_status_accumulates_runtime_counters`, `test_internal_reconcile_does_not_resend_esg_attempt`, `test_internal_reconcile_transport_change_does_not_resend_esg_attempt`, `test_as2_pending_reconciles_without_resending_and_ignores_late_ack`.

공백: 토큰 미설정·비UTF8·빈 문자열 모든 조합, 모든 ACK 전이/역순/기관별 signature/MIC 검증은 이 HTTP 명세의 통과 증거가 없다. `test_reconcile_blocks_legacy_attempts_without_reexport_or_resend`는 직접 service 함수 검사다. `test_as2_pending_reconciles_without_resending_and_ignores_late_ack`도 reconcile과 late ACK는 직접 service 함수를 호출하고 GET 결과만 HTTP로 관찰한다. 이를 reconcile/callback POST의 해당 분기 검증으로 계산하지 않는다. callback의 dispatch-state 비200 성공 처리 분기를 제거했다. 실행 결과는 NOT_RUN이다.
