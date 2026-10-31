# 인증·인가·RLS·CSRF 횡단 명세

> **NOT_RUN:** 이 문서는 2026-10-31 현재 소스와 테스트 본문을 읽어 작성했다. 테스트, 빌드, API 호출, DB 호출은 실행하지 않았다. 아래 테스트 연결은 코드상 단언 범위를 설명하며 실행 성공 또는 규제 증명을 뜻하지 않는다.

이 문서는 Administration API를 포함한 `/api` 전체에 적용되는 요청 인증, request authorization snapshot, 정책 허가, RLS 컨텍스트, CSRF와 오류 매핑을 설명한다. 각 관리 경로의 payload와 route별 커버리지는 [administration.md](administration.md)다.

<a id="request-pipeline"></a>
## 요청 파이프라인

[`app`](../../src/lib.rs)은 바깥쪽부터 request stamp, cookie manager, 전역 context resolver, response mapper를 적용한다. `/api`에는 추가로 CSRF origin middleware와 `mw_ctx_require_and_set_dbx`를 적용한다. `/auth/v1/login`과 `/auth/v1/logoff`는 public router, `/auth/v1/refresh`만 인증 middleware 아래 있고 `/health`는 공개다.

1. [`mw_req_stamp_resolver`](../../../../libs/lib-web/src/middleware/mw_req_stamp.rs)가 request UUID와 수신 시각을 만든다.
2. [`mw_ctx_resolver`](../../../../libs/lib-web/src/middleware/mw_auth.rs)가 `auth-token` cookie를 읽는다. token 식별자가 `email|organization_uuid`이면 그 조직 계정을, legacy email-only이면 이메일 계정을 찾는다. 서명과 만료를 검증하고 DB에서 repeatable-read authorization snapshot을 로드한다.
3. 성공하면 resolver는 snapshot에서 `Ctx`를 만들고 snapshot, 조직 policy revision을 request extension에 붙이며 auth cookie를 갱신한다. token 없음 외의 resolver 오류에서는 cookie를 제거한다.
4. [`mw_ctx_require_and_set_dbx`](../../../../libs/lib-web/src/middleware/mw_db_ctx.rs)는 보호 route에서 인증 오류를 거부한다. `must_change_password=true`이면 nested path 기준 `POST /users/me/password`, `GET /users/me/profile`만 통과시킨다.
5. handler는 `AuthorizationSnapshotW`와 정책 registry의 contextual action으로 read/mutation permit을 만든다. 저장 접근에는 그 permit에서 만든 RLS context를 넘긴다.
6. [`mw_response_map`](../../../../libs/lib-web/src/middleware/mw_res_map.rs)은 오류 envelope, `x-request-id`, snapshot이 있는 경우 `x-rbac-policy-version`을 붙인다. `PERMISSION_DENIED` detail에는 policy version도 넣는다.

활성 HTTP 근거: token 사용자 해석과 요청 간 context 분리는 [`middleware_ctx.rs`](../api_administration/middleware_ctx.rs), 인증 누락 `403`은 [`test_middleware_requires_auth`](../authz/auth_mw_audit.rs), 첫 암호 변경 제한은 [`test_new_user_temp_password_and_first_login_reset_flow`](../authz/rbac_users/create_validation_web.rs)에서 단언한다. 반면 [`authorization_snapshot.rs`](../authz/authorization_snapshot.rs)는 extractor에 직접 extension을 넣는 단위 테스트여서 전체 middleware 순서나 경로 허용의 HTTP 증거가 아니다.

<a id="authentication-cookie-lifecycle"></a>
## 인증 cookie 수명

- login은 `{"email","pwd"}`를 받아 암호 검증 후 조직 ID와 token salt에 묶인 cookie를 발급한다. 응답의 `must_change_password`는 다음 보호 요청 제한을 예고한다.
- resolver는 유효 요청마다 token을 재발급한다. refresh도 현재 사용자를 다시 읽어 cookie를 설정하고 응답에 15분 뒤 `expiresAt`을 계산한다.
- logoff는 body의 `logoff=true`일 때 cookie 제거를 요청한다. 서버측 revocation 저장을 호출하지 않는다.
- 암호 변경은 token salt가 바뀌는 모델 동작과 기존 cookie 제거를 거쳐 이전 session을 거부하게 한다. 이 흐름은 활성 HTTP 테스트가 이전 session/암호 `403`, 새 암호 login `200`을 직접 단언한다.

[`test_auth_login_refresh_logoff`](../authz/auth_mw_audit.rs)는 login, refresh, logoff 각각 `200`을 확인하지만 refresh/logoff에는 login 응답 cookie 대신 별도로 생성한 token을 사용한다. 따라서 이 함수는 login-cookie→refresh→logoff와 logoff 뒤 재사용 거부를 end-to-end로 증명하지 않는다. refresh의 `expiresAt`, 갱신 cookie 속성, token rotation 관계도 미단언이다.

<a id="authorization-snapshot"></a>
## request authorization snapshot

snapshot은 principal, active organization, identity/grants, 조직 policy revision, 인증 만료 시각을 한 요청의 권한 판단 기준으로 고정한다. [`SnapshotRepository::load_repeatable_read`](../../../../libs/lib-core/src/model/authorization/snapshot_repo.rs)는 resolver 안에서 호출되고, handler extractor [`AuthorizationSnapshotW`](../../../../libs/lib-web/src/middleware/mw_authorization_snapshot.rs)는 extension이 없으면 실패한다.

- [`protected_snapshot_extractor_reads_the_attached_snapshot`](../authz/authorization_snapshot.rs)는 직접 붙인 snapshot의 principal/organization을 읽는 helper-level 검증이다.
- [`protected_snapshot_extractor_fails_when_middleware_did_not_attach_one`](../authz/authorization_snapshot.rs)는 extension 누락 오류만 확인한다.
- [`protected_inventory_matches_checked_in_snapshot`](../authz/protected_route_inventory.rs)는 라우터 등록 문자열을 snapshot 파일과 비교한다. HTTP 요청, cookie, 정책 허용/거부를 수행하지 않으므로 route authorization 증거가 아니다.
- `dump_protected_inventory_snapshot`은 `#[ignore]`인 수동 생성기다.

스냅샷 로드 중 정책 revision 변경, 인증 만료 경계, 한 요청 도중 grant 변경의 격리, 응답 헤더 revision과 profile body `policyVersion`의 일치는 활성 HTTP 테스트에서 직접 단언되지 않는다.

<a id="policy-and-rls"></a>
## 정책 허가와 permit 기반 RLS

관리 handler는 collection/proposed/existing resource context를 정책 action과 함께 평가한다. 대표 action은 `user.list|read|create|update`, 역할 할당 전용 user action, `organization.*`, `role.*`, settings/notice, audit collection 및 case audit read다. read/mutation permit을 얻은 뒤 [`rls_ctx_for_authorized_read` / `rls_ctx_for_authorized_mutation`](../../../../libs/lib-rest-core/src/lib.rs)으로 DB context를 만든다.

직접 HTTP 단언이 있는 범위는 다음과 같다.

- 사용자: 일반 역할의 list/get/create/update `403`, 같은 조직 관리자의 성공, 플랫폼·sponsor 관리자도 사용자 읽기가 현재 조직에 고정됨. [`rbac_users`](../authz/rbac_users), [`test_users_endpoints_require_admin_guard`](../authz/rls_web.rs).
- 조직: manager/user/viewer/sponsor admin의 collection/item/mutation `403`, 플랫폼 관리자의 create/get 성공. 성공 list/update/delete는 미단언이다. [`rbac_organizations.rs`](../authz/rbac_organizations.rs).
- 역할: 플랫폼 관리자에게 `organization_id` scope 필수, sponsor 관리자의 조직 격리, 교차 조직 item `404`, custom role 수 제한. [`permission_profiles_web.rs`](../authz/rbac_users/permission_profiles_web.rs).
- 감사: sponsor admin의 자기 조직 목록, system admin의 cross-org collection, case audit는 system admin도 부모 case scope를 우회하지 못함. by-record collection은 권한이 있으면 cross-org와 hard-deleted record도 조회한다. [`rbac_audit.rs`](../authz/rbac_audit.rs).
- 일반 case RLS의 조직 필터와 item 은닉은 [`test_rls_list_cases_filters_org`](../authz/rls_web.rs)가 단언하지만, 이것만으로 모든 보호 route의 tenant 격리를 일반화할 수 없다.

[`authorization_legacy_gate.rs`](../authz/authorization_legacy_gate.rs), [`platform_admin_policy.rs`](../authz/platform_admin_policy.rs), [`authorization_startup.rs`](../authz/authorization_startup.rs)는 소스 문자열 또는 startup 구조를 검사한다. 회귀 gate로는 의미가 있지만 HTTP 허용·DB RLS 결과를 직접 단언하지 않는다.

<a id="csrf-origin"></a>
## CSRF origin 경계

[`mw_csrf_origin`](../../src/lib.rs)은 `/api` router에만 적용된다. `GET`, `HEAD`, `OPTIONS`, `TRACE`가 아닌 요청이면서 `auth-token` cookie가 있을 때 origin을 검사한다.

- `Origin`이 없으면 허용한다.
- `Origin: null`, UTF-8이 아닌 값, URI authority가 없는 값은 거부한다.
- `E2BR3_PUBLIC_ORIGIN`이 비어 있지 않으면 trailing slash를 제거해 그것과 비교한다.
- 설정 origin이 없으면 `Host` 또는 comma-separated `x-forwarded-host` 중 하나와 origin authority를 대소문자 무시 비교한다.
- 거부 응답은 response mapper 전 단계의 `403` body `cross-origin state-changing request rejected`다.

[`csrf_rejects_null_origin`](../../src/lib.rs)과 [`csrf_accepts_the_origin_forwarded_by_the_frontend_proxy`](../../src/lib.rs)는 `csrf_origin_allowed` helper만 직접 호출한다. 실제 auth cookie가 있는 POST/PUT/DELETE `/api` 요청, `E2BR3_PUBLIC_ORIGIN`, 다중 forwarded host, body/status 매핑은 HTTP 테스트가 없다. `/auth/v1/login`, `/auth/v1/logoff`, `/auth/v1/refresh`는 이 middleware 밖에 있으므로 같은 origin 검사를 받지 않는 구현이다. 이 경계에 대한 공격/브라우저 수준 테스트도 없다.

<a id="audit-attribution"></a>
## 감사 귀속과 조회 권한

resolver는 `x-e2br3-reason-for-change`, `x-e2br3-change-category`를 percent-decode하고 공백만 있는 값은 버린 뒤 `Ctx`에 넣는다. mutation 모델이 이 컨텍스트와 e-signature ID를 감사 trigger/model에 전달한다.

- 사용자 CREATE와 role CREATE/UPDATE/soft-delete를 HTTP mutation 뒤 audit API로 조회하는 테스트가 있다. [`test_admin_can_see_user_create_in_audit_logs`](../authz/rbac_audit.rs), [`test_admin_can_see_role_create_update_delete_in_audit_logs`](../authz/rbac_audit.rs).
- 설정은 최초 default/create가 system user, 이후 UPDATE가 요청 사용자이며 동일 값 재저장은 새 timezone UPDATE를 만들지 않음을 DB query로 확인한다. [`test_admin_settings_audit_trail_records_changed_field`](../api_administration/admin_settings/audit_web.rs).
- 공지는 writer 이메일과 CREATE/UPDATE old/new title을 확인한다. [`test_dashboard_notices_are_org_scoped_and_audited`](../api_administration/admin_settings/notices_web.rs).
- editor 변경 reason이 case audit projection의 `reason`과 field row에 나타나는 것은 [`fda_device_scalar_editor_save_projects_audit_trail`](../api_editor/audit/projection.rs)에서 확인한다.

설정/공지 테스트의 DB 직접 query는 audit 조회 API 권한을 증명하지 않는다. 반대로 audit API 테스트는 모든 mutation이 감사된다는 전수 증명이 아니다. category header, percent-decoding 오류, IP/user-agent, e-signature, 실패 mutation의 무감사, 감사 chain HTTP integrity 경로는 남은 공백이다.

<a id="error-contract"></a>
## 오류 계약과 정보 은닉

response mapper는 인증 오류를 보통 `403 NO_AUTH`, 최초 암호 제한을 `403 PASSWORD_CHANGE_REQUIRED`, 정책 오류를 `403 ACCESS_DENIED|PERMISSION_DENIED`, validation을 `400 INVALID_REQUEST`, 중복을 `409 INVALID_REQUEST`, UUID entity 누락을 `404`로 매핑한다. 오류 body에는 request UUID가 들어가며 debug detail은 환경 설정에 따라 제한된다. 로그인 사용자 없음과 암호 불일치는 둘 다 `LOGIN_INVALID_CREDENTIALS`로 합쳐 credential enumeration을 줄인다.

직접 단언은 login 오류 body, user 잘못된 UUID `400`/없는 UUID `404`, 여러 권한 `403`, validation `400`, 중복 `409`에 분산되어 있다. 모든 `ClientError` variant의 envelope, production debug 설정, `x-request-id`와 body `req_uuid` 동일성, `x-rbac-policy-version`은 route-level로 전수 단언되지 않는다. [`api_route_not_found_is_json`](../../src/lib.rs)은 fallback handler만 직접 호출하고 실제 router의 unknown `/api` 요청은 보내지 않는다.

<a id="authorization-evidence-limits"></a>
## 증거 한계

- route inventory와 소스 parser 테스트는 등록 누락을 찾을 수 있지만 인증·인가를 실행하지 않는다.
- policy kernel/model/helper 테스트는 정책 규칙을 격리해 확인하지만 middleware, error mapping, 실제 handler context를 함께 통과하지 않는다.
- HTTP `403`만 확인한 경로는 성공 payload/state/audit 계약을 확인하지 않는다. Administration 표에서 이 경우를 `authorization_only`로 표시했다.
- 조직 성공 list/update/delete, workflow options 성공, audit integrity HTTP, health HTTP에는 활성 직접 단언이 없다.
- CSRF는 helper unit test만 있으며 `/auth/v1`의 origin 경계에는 직접 테스트가 없다.
- 이 문서 작성 과정에서는 어떤 검증 명령도 실행하지 않았으므로 현재 환경에서의 통과 상태를 주장하지 않는다.
