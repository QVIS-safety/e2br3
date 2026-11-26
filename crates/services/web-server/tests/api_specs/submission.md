# Submission API 명세와 검사 범위

현재 등록 범위는 `/api` 제출 9개와 `/internal` machine-token 경로 3개, 모두 12개다. 대상 실행 단위는 `api_submission`이다. 아래는 현재 코드에 존재하는 assertion 명세다. 실행 결과는 **NOT_RUN**이다. 제출 테스트의 `start_mock_esg`/`configure_mock_as2_transport`는 기존 로컬 응답 대역이다. 이 테스트로 외부 FDA/MFDS 접수나 실제 AS2/Java 연동을 증명하지 않는다.

라우트 원본: [Case routes](../../src/web/rest/routes/cases.rs), [submission routes](../../src/web/rest/routes/submissions.rs). 요청·응답 원본: [submission_rest](../../src/web/rest/submission_rest.rs), [전자서명 입력](../../src/web/rest/compliance.rs).

<a id="submit"></a>
## 제출과 재시도 식별

| Method / path | 요청 | 구현 응답 / 검사하는 조건 |
|---|---|---|
| `POST /api/cases/{id}/submissions/fda` | JSON 최상위 `reason_for_change`, `e_signature`; 선택적 `x-idempotency-key` | 201 `{"data": SubmissionRecord}`. 같은 키의 반복·병렬 호출은 같은 submission ID와 목록 1개. 전송 결과 불명은 `dispatch_unknown`, 원격 ID·ACK null로 보존 |
| `POST /api/cases/{id}/submissions/mfds` | 같은 compliance 본문 | 201. 요청 URL의 기관이 Case appendices와 별도로 제출 기관을 결정. MFDS 로컬 AS2 대역 응답 ID 확인 |
| `GET /api/cases/{id}/submissions` | Case UUID | 200 `data.items`. 재시도 후 중복 없이 1개이고 불명 결과가 그대로 보임 |

`e_signature`는 `meaning`과 `password` 문자열이며, body 자체와 `reason_for_change`, `e_signature.meaning`, `e_signature.password`는 모두 필수이고 세 문자열은 공백만으로 구성될 수 없다. 비밀번호 재확인과 서명 저장은 위 compliance 구현을 따른다. 본문은 일반 CRUD의 `data` envelope가 아니다. `x-idempotency-key`가 있으면 UTF-8이어야 하며 저장 전 양끝 공백을 제거한다. 구현은 `submission.execute` 권한과 Case mutation 경계를 거쳐 서명을 기록하고 제출을 예약한 뒤 전송한다. `201` 자체는 규제기관 접수 성공을 뜻하지 않는다.

검사 근거:

- [idempotency](../api_submission/idempotency.rs): `test_submission_idempotency_key_reuses_submission_when_enabled`, `test_submission_idempotency_key_parallel_requests_single_submission` — 응답 ID 동등성·GET 목록 1개.
- [dispatch](../api_submission/dispatch.rs): `test_submission_esg_transport_sends_expected_headers_and_payload` — 대역으로 보내는 bearer/API-key와 비어 있지 않은 XML; `test_submission_esg_non_success_preserves_unknown_without_resend` — 502에도 재전송 없이 unknown·한 번의 전송·retry 없음·오류 보존; `test_submission_records_failure_when_as2_submitter_unreachable` — 연결 실패 시 dispatch_failed·Case 상태 복원·재시도 없음·ACK1~4 null·동일 키 재사용.
- 같은 파일의 `test_submission_rejects_enabled_esg_without_base_url` — 400 및 누락 설정명. `test_submission_accepts_mfds_route_for_mfds_profile`, `test_submission_uses_request_authority_not_case_appendices` — MFDS 요청 기관 처리.
- [validation](../api_submission/validation.rs): `test_submission_rejects_case_validation_before_routing_or_dispatch` — 변경으로 캐시가 무효화되고 FDA/MFDS 둘 다 400 + validation issue; `test_unavailable_meddra_allows_save_and_export_but_blocks_submission` — 사전 버전 문제는 저장·수동 export를 허용하지만 제출은 400. `test_submission_does_not_require_case_validated_status` — Case status 문자열만으로 사전 차단하지 않음.

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

[history](../api_submission/history.rs)의 `test_submission_history_includes_latest_ack_time_and_event`, `test_submission_ack_can_be_downloaded_as_text`; [callbacks](../api_submission/callbacks.rs)의 `test_internal_ack_callback_updates_submission_by_remote_id`, `test_internal_ack_callback_duplicate_payload_is_idempotent`; [dispatch](../api_submission/dispatch.rs)의 `test_submission_records_failure_when_as2_submitter_unreachable`가 해당 근거다. 이력 scope는 [histories](../api_cases/scope/histories.rs)의 실제 endpoint loop도 참조한다.

원본 XML 보존 검사는 HTTP 다운로드와 DB 변경 거절을 함께 확인하지만, 모든 이력 API에 동일한 권한·감사·파일 헤더 assertion이 있다는 뜻은 아니다. 다운로드 audit 기록의 모든 old/new·사용자·사유를 이 API군에서 직접 비교하지 않는다.

<a id="receiver"></a>
## 수신처와 메시지 헤더

Case 초안은 Section N을 저장하지 않는다. export와 submission은 Case의 보고유형, Case Sender가 참조한 Sender presave의 기관별 기본 Sender ID, Product presave가 연결한 Receiver presave의 정확히 한 개 route를 사용해 N을 요청 단위로 생성한다. route가 없거나 둘 이상이면 dispatch 전에 실패한다. 생성한 N은 dictionary 계약과 최종 XML 검증을 통과해야 하며 제출 XML은 `case_submissions.submitted_xml`에 보존한다. 가져온 XML의 N은 원본 `cases.raw_xml`에만 남고 Case 편집 필드로 투영하지 않는다.

근거: [headers](../api_submission/headers.rs)의 `outbound_header_is_transient_and_uses_receiver_presave_route`는 실제 격리 DB에서 presave route로 N을 생성하고 XML 검증을 통과하며 `message_headers` 테이블이 없는 것을 확인한다.

<a id="internal"></a>
## 내부 ACK·reconcile API

이 경로는 일반 사용자 쿠키 middleware 대신 명시적인 machine token을 검사한다. `x-callback-token`은 `AS2_CALLBACK_TOKEN`, `x-reconcile-token`은 `SUBMISSION_RECONCILE_TOKEN`과 비교한다. 실제 토큰 값은 명세에 넣지 않는다.

| Method / path | 요청 | 응답·검사 |
|---|---|---|
| `POST /internal/submissions/callbacks/ack` | 최상위 `remote_submission_id`, `ack_level`, `success`, 선택적 `ack_code`, `ack_message` 및 callback header | 200 `data` submission; ACK3 상태·재조회·이벤트. 같은 payload 재호출 시 중복 이벤트. 토큰 누락/오류 400. `ack_level` 누락 검사는 400 또는 422를 허용하므로 단일 오류코드 보증이 아님 |
| `POST /internal/submissions/reconcile` | reconcile header; body 생략 가능, JSON `limit` 선택(기본 25, worker에서 1..100으로 clamp) | 200 `data.result`의 attempted/processed ID, 빈 결과·인증, HTTP reconcile 뒤 ESG 중복 전송 방지. legacy/AS2는 아래 service 검사와 구분 |
| `GET /internal/submissions/reconcile/status` | reconcile header | 200 `data.status`; total_runs 증가와 total_attempted 타입. 프로세스 재시작 이후 누적 보존 검사는 아님 |

근거: [callbacks](../api_submission/callbacks.rs)의 `test_internal_submission_endpoints_require_valid_callback_token`, `test_internal_ack_callback_rejects_malformed_payload`, 앞의 update/duplicate 검사; [reconcile HTTP](../api_submission/reconcile/web.rs)의 `test_internal_reconcile_endpoint_auth_and_empty_result`, `test_internal_reconcile_status_accumulates_runtime_counters`, `test_internal_reconcile_does_not_resend_esg_attempt`, `test_internal_reconcile_transport_change_does_not_resend_esg_attempt`. [worker 계약](../api_submission/reconcile/service.rs)의 `test_reconcile_blocks_legacy_attempts_without_reexport_or_resend`, `test_as2_pending_reconciles_without_resending_and_ignores_late_ack`는 service 직접 호출 검사이며 HTTP 근거로 계산하지 않는다.

공백: 토큰 미설정·비UTF8·빈 문자열 모든 조합, 모든 ACK 전이/역순/기관별 signature/MIC 검증은 이 HTTP 명세의 통과 증거가 없다. `test_reconcile_blocks_legacy_attempts_without_reexport_or_resend`는 직접 service 함수 검사다. `test_as2_pending_reconciles_without_resending_and_ignores_late_ack`도 reconcile과 late ACK는 직접 service 함수를 호출하고 GET 결과만 HTTP로 관찰한다. 이를 reconcile/callback POST의 해당 분기 검증으로 계산하지 않는다. callback의 dispatch-state 비200 성공 처리 분기를 제거했다. 실행 결과는 NOT_RUN이다.

### 조직별 발신·수신 ID

발신 ID는 Case Sender에 연결된 Sender presave의 해당 기관 기본 gateway에서 읽는다. ICH 수신 ID는 Case Product ID와 같은 조직의 활성 Product presave에 연결된 Receiver presave에서 읽는다. `CI / C.1.3 / Equal` 조건과 보고 유형이 일치하는 ICH route가 정확히 하나여야 하며, batch/message ID를 각각 보존한다. FDA/MFDS의 기관별 수신 옵션은 기존 보고 유형 매핑을 사용한다. 초안 생성에는 전송 ID를 요구하지 않는다.

근거: `tests/api_submission/headers.rs::ich_header_uses_product_receiver_presave_and_isolates_organizations` — presave API 저장, 동일 Product ID의 조직 격리, batch/message 구분, 불일치·중복 route 거절과 기존 헤더 보존.
