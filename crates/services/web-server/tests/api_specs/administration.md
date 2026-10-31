# Administration API 명세와 소스 기반 커버리지

> **NOT_RUN:** 이 문서는 2026-10-31 현재 라우터·핸들러·DTO와 활성 테스트 소스만 읽어 작성했다. 테스트, 빌드, API 호출, DB 호출은 실행하지 않았다. 아래 `asserted`는 테스트 코드에 해당 HTTP 요청과 단언이 있다는 뜻이며 실행 성공이나 규제 적합성을 뜻하지 않는다.

이 문서는 사용자, 조직, 설정, 권한 프로필, 브랜딩, 감사 로그와 인증·health 라우트 34개를 다룬다. 최종 등록 대조표는 [`coverage.tsv`](coverage.tsv)이며 라우터 원본은 [`users.rs`](../../src/web/rest/routes/users.rs), [`misc.rs`](../../src/web/rest/routes/misc.rs), [`cases.rs`](../../src/web/rest/routes/cases.rs), [`routes_login.rs`](../../src/web/routes_login.rs), [`lib.rs`](../../src/lib.rs)다. 횡단 인증·RLS·CSRF 의미는 [authorization.md](authorization.md)에 분리했다.

상태는 다음처럼 제한해서 사용한다.

- `asserted`: 활성 테스트가 해당 HTTP 경로의 성공 응답, 응답 본문, 오류, 상태 변화 중 하나 이상을 직접 단언한다.
- `authorization_only`: 활성 테스트가 실제 HTTP 요청의 허용/거부 상태만 단언하고 성공 기능은 단언하지 않는다.
- `registration_only`: 활성 route inventory 단언으로 등록은 확인되지만 HTTP 동작 단언은 없다. 보호 경로 스냅샷은 HTTP 요청을 보내지 않으므로 권한 증거로 세지 않는다.
- `uncovered`: 등록 소스와 구현만 확인되고 활성 등록 inventory 단언이나 HTTP 단언도 없다.

아래 요청·응답 예시는 실행에서 관측한 값이 아니라 DTO와 handler가 기대하거나 직렬화하는 키 모양이다. 공통 JSON 변경 요청은 `{"data": {...}}` 형태다. 일반 REST 리소스는 `{"data": ...}`로 감싸지만 권한 프로필과 설정 응답은 핸들러가 값을 직접 반환한다. 오류는 응답 매퍼가 대체로 `{"error":{"message":"...","data":{"req_uuid":"...","detail":...}}}`로 만든다. 인증된 응답에는 구현상 `x-request-id`, 스냅샷을 붙인 요청에는 `x-rbac-policy-version`도 설정된다. 이 헤더들은 여기 연결한 경로 테스트 대부분에서 직접 단언하지 않는다.

## 사용자

사용자 요청/응답 필드는 [`dto.rs`](../../src/web/rest/user_rest/dto.rs), 동작은 [`handlers.rs`](../../src/web/rest/user_rest/handlers.rs)에 정의된다. `UserView` 응답은 camelCase이며 대표 형태는 `data.{id,organizationId,email,username,role,roleMeta,scope,active,mustChangePassword,lastLoginAt,createdAt,updatedAt}`다. 생성·수정 입력은 snake_case 필드다.

<a id="get-api-users"></a>
### `GET /api/users` — asserted

- 쿼리: `filters[...]`, `list_options.limit|offset|order_bys`; `include_blinded=true` 또는 `includeBlinded=true`가 없으면 `access_blind_allowed=false` 필터를 강제로 더하고 현재 조직 필터도 강제한다.
- 응답: `200 {"data":[UserView,...]}`. `user.list` 정책과 permit 기반 RLS가 선행한다.
- 직접 단언: 현재 조직만 보이는 목록과 교차 조직 필터 무력화, blinded 포함 옵션, 비관리자 `403`, 사용자 생성·비활성화 뒤 목록 동작. [`test_user_reads_are_scoped_to_current_org_for_system_and_sponsor_admins`](../authz/rbac_users/rls_scope_web.rs), [`test_non_admin_cannot_list_users`](../authz/rbac_users/non_admin_forbidden_web.rs), [`test_sponsor_admin_can_hide_and_restore_blind_user`](../authz/rbac_users/role_update_web.rs).
- 미단언: 모든 `ParamsList` 정렬/페이지 조합과 잘못된 쿼리의 상세 오류 본문.

<a id="post-api-users"></a>
### `POST /api/users` — asserted

- 요청: `{"data":{"organization_id":"uuid","email":"...","username":"...","pwd_clear":"...","role":"role-id","comments":null,"other_information":null,"access_start_at":"RFC3339","access_end_at":"RFC3339","active_sender_identifier":"uuid","access_sender_ids":["uuid"],"access_product_ids":["uuid"],"access_study_ids":["uuid"],"access_blind_allowed":false}}`. 플랫폼 관리자는 `organization_id`가 필수이고 sponsor 관리자는 현재 조직으로 고정된다.
- 처리: 이메일·사용자명, UUID scope와 sender/product/study 계층, 역할 소속과 sponsor 관리자 수를 검증한다. 역할·사용자명이 없으면 일반 사용자 역할과 이메일 local-part를 사용하고, 초기 암호가 없으면 백엔드 초기 암호를 사용한다. 생성 뒤 `must_change_password=true`가 된다.
- 응답: `201 {"data":UserView}`. 중복 이메일 `409`, 잘못된 역할·길이·scope `400`, 권한 부족 `403`이 직접 단언된다.
- 직접 단언: 생성·기본값·중복·길이는 [users/create_web.rs](../api_administration/users/create_web.rs), 기간·첫 로그인은 [users/access_web.rs](../api_administration/users/access_web.rs), 역할 제한은 [role_create_web.rs](../authz/rbac_users/role_create_web.rs), 감사 `CREATE` 조회는 [`test_admin_can_see_user_create_in_audit_logs`](../authz/rbac_audit.rs).
- 미단언: 모든 선택 필드 조합과 모든 scope 위반 메시지.

<a id="get-api-users-id"></a>
### `GET /api/users/{id}` — asserted

- 경로의 `id`는 UUID다. `user.read` 정책 뒤 현재 조직의 사용자만 `200 {"data":UserView}`로 반환하며 다른 조직은 not-found로 은닉한다.
- 직접 단언: 동일 조직 조회/역할 메타데이터, 교차 조직 not-found, 비관리자 `403`, 잘못된 UUID `400`, 없는 UUID `404`. [`test_user_api_role_metadata_uses_authoritative_assignment`](../api_administration/role_admin/metadata_web.rs), [`test_user_reads_are_scoped_to_current_org_for_system_and_sponsor_admins`](../authz/rbac_users/rls_scope_web.rs), [`error_mapping_web.rs`](../api_administration/error_mapping_web.rs).

<a id="put-api-users-id"></a>
### `PUT /api/users/{id}` — asserted

- 요청: `email`, `username`, `role`, `active`, `comments`, `other_information`, access 기간·scope·blind·active sender, `last_login_at`의 patch다. `access_start_at`/`access_end_at`의 명시적 `null`은 값을 지운다. `organization_id`와 `pwd_clear`는 update DTO에 없으므로 이 경로에서 바꿀 수 없다.
- 처리: 역할 변경 여부에 따라 `user.update`, `user.update.role_assignment`, `user.update.built_in_role_assignment` 정책을 선택한다. 사용자·역할·scope 검증 후 patch하고 최신 `UserView`를 반환한다.
- 응답: 성공 `200`; 빈 사용자명, 과도한 길이, 조직 유형과 맞지 않는 관리자 역할, scope 불일치 `400`; 권한 부족 `403`.
- 직접 단언: 일반 값·날짜 clear·길이는 [users/update_web.rs](../api_administration/users/update_web.rs), 역할 할당·blind 정책은 [role_update_web.rs](../authz/rbac_users/role_update_web.rs), scope 변경은 [`assignments.rs`](../api_cases/scope/assignments.rs), 감사 field 필터 연결은 [`test_admin_can_filter_user_audit_logs_by_field`](../authz/rbac_audit.rs).

<a id="get-api-users-me"></a>
### `GET /api/users/me` — asserted

- 인증 컨텍스트의 사용자만 `200 {"data":UserView}`로 반환한다.
- 직접 단언: 토큰 사용자 ID와 요청 간 컨텍스트 비누출은 [`middleware_ctx.rs`](../api_administration/middleware_ctx.rs). 암호 변경 뒤 이전 세션 `403`은 [`test_new_user_temp_password_and_first_login_reset_flow`](../api_administration/users/access_web.rs).

<a id="get-api-users-me-profile"></a>
### `GET /api/users/me/profile` — asserted

- 응답: `200 {"data":{"user":UserView,"activeOrganization":{id,name,type},"availableOrganizations":[...],"routing":RoutingProfile,"privileges":[...],"eligibleActions":[...],"policyVersion":number}}`.
- 이 경로는 최초 암호 변경 전에도 허용된다. 멤버십 조직 목록, custom role 표시, 메뉴 권한과 eligible action의 대응이 직접 단언된다. [`one_account_lists_and_switches_membership_organizations`](../api_administration/user_organization_membership_web.rs), [`test_current_user_profile_exposes_custom_role_name`](../api_administration/role_admin/metadata_web.rs), [`test_system_admin_profile_menu_privileges_match_eligible_actions`](../authz/rbac_users/permission_profiles_web.rs).
- 미단언: 응답 `policyVersion`과 `x-rbac-policy-version` 헤더의 동일성.

<a id="put-api-users-me-organization"></a>
### `PUT /api/users/me/organization` — asserted

- 요청: `{"data":{"organization_id":"uuid"}}`; `organizationId` 별칭도 허용한다.
- 같은 이메일과 대상 조직의 활성 계정을 찾아 새 조직·계정에 묶인 auth cookie를 발급하고 `200 {"data":{"activeOrganization":...,"availableOrganizations":[...]}}`를 반환한다. nil UUID는 `400`, 멤버십 없음은 `403`이다.
- 직접 단언: 새 `Set-Cookie`를 다음 profile 요청에 사용해 대상 조직과 동일 계정 ID를 확인하고, 멤버십 없는 전환 `403`도 확인한다. [`user_organization_membership_web.rs`](../api_administration/user_organization_membership_web.rs).

<a id="post-api-users-me-password"></a>
### `POST /api/users/me/password` — asserted

- 요청: `{"data":{"current_password":"...","new_password":"..."}}`. 최초 변경에서는 `current_password`를 생략할 수 있고, 일반 변경에서는 필수다.
- 새 암호 검증 뒤 암호와 `must_change_password`를 갱신하고 기존 auth cookie를 제거하며 `204`를 반환한다. 약한 암호/현재 암호 누락·불일치는 `400`이다.
- 직접 단언: 최초 변경의 약한 암호 `400`, 성공 `204`, 이전 세션과 이전 암호 `403`, 새 암호 로그인 `200`. [`test_new_user_temp_password_and_first_login_reset_flow`](../api_administration/users/access_web.rs).
- 미단언: 일반 세션에서 올바른/틀린 `current_password` 분기와 cookie 제거 속성.

<a id="get-api-users-me-routing"></a>
### `GET /api/users/me/routing` — asserted

- 선택 쿼리 `organizationId=uuid`; 다른 조직이면 동일 이메일 멤버십이 필요하다.
- 응답: `200 {"data":{"builtInRoleId":"...","operational":bool,"senderSelectionRequired":bool,"activeSenderIdentifier":string|null,"availableSenders":[{"senderIdentifier":"...","senderOrganization":string|null,"caseCount":number}],"effectiveScope":{...}}}`.
- 직접 단언: INFO sender master 포함, 역할 scope에 따른 후보 축소와 건수는 [`sender_options.rs`](../api_cases/scope/sender_options.rs). 다른 조직 routing 조회는 소스 구현만 확인되고 직접 단언되지 않는다.

<a id="put-api-users-me-routing"></a>
### `PUT /api/users/me/routing` — asserted

- 요청: `{"data":{"active_sender_identifier":"uuid-or-null"}}`; `sender_id` 별칭을 허용한다. 선택 값이 유효 scope에 없으면 `403`이다.
- 성공 시 사용자 routing 선택을 갱신하고 위 `RoutingProfile`을 `200`으로 반환한다.
- 직접 단언: 허용된 sender 선택 `200`, 무작위 sender `403`, 선택이 case 목록 자체를 축소하지 않음. [`test_active_sender_selection_does_not_filter_case_list`](../api_cases/scope/cases.rs).

<a id="get-api-users-workflow-options"></a>
### `GET /api/users/workflow-options` — authorization_only

- 쿼리: camelCase `limit`; 생략 시 소스상 200이다. 성공 구현은 `case.workflow.config.read` 권한 아래 활성 사용자 옵션 `data:[{id,email,displayName}]`을 반환한다.
- 직접 단언은 권한 없는 일반 사용자와 case read만 가진 사용자에 대한 `403`뿐이다. [`test_user_without_case_read_cannot_list_workflow_user_options`](../authz/rbac_users/non_admin_forbidden_web.rs), [`test_case_reader_without_workflow_grant_cannot_list_workflow_user_options`](../authz/rbac_users/non_admin_forbidden_web.rs).
- 공백/음수/대형 `limit`, 성공 응답 내용과 조직 범위는 미단언이다.

## 조직

조직 DTO와 저장 모델은 [`organization.rs`](../../../../libs/lib-core/src/model/organization.rs), 핸들러는 [`organization_rest.rs`](../../src/web/rest/organization_rest.rs)다. 응답 `Organization`은 `data.{id,name,type,address,city,state,postcode,country_code,contact_email,contact_phone,active,created_at,updated_at,created_by,updated_by}`처럼 모델의 기본 snake_case를 유지하며 `org_type`만 `type`으로 바뀐다. 조직 관리는 정책상 플랫폼 관리자 전용이다.

<a id="get-api-organizations"></a>
### `GET /api/organizations` — authorization_only

- 구현: `filters[...]`와 `list_options.*`를 받아 `200 {"data":[Organization,...]}`를 반환한다.
- 활성 HTTP 단언은 manager/user/viewer와 sponsor admin의 `403`뿐이다. 이름과 달리 [`test_system_admin_can_create_and_list_organizations`](../authz/rbac_organizations.rs)는 목록 요청을 보내지 않고 생성 뒤 item GET만 수행한다.
- 성공 목록, 필터·페이지, 플랫폼 관리자 교차 조직 가시성은 미단언이다.

<a id="post-api-organizations"></a>
### `POST /api/organizations` — asserted

- 요청: `{"data":{"name":"...","type":"CRO|Pharmaceutical company","address":null,"contact_email":"..."}}`; `org_type`도 별칭이다. 유형은 `cro` 또는 `pharmaceutical_company`로 정규화된다.
- 응답: `201 {"data":Organization}`. 알려지지 않은 유형 `400`, 비플랫폼 관리자 `403`이 직접 단언된다. 생성 시 조직 기본 설정 생성은 모델 소스에 있으나 HTTP 테스트는 그 부수 효과를 직접 조회하지 않는다.
- 증거: [`test_system_admin_can_create_and_list_organizations`](../authz/rbac_organizations.rs), [`test_system_admin_cannot_create_organization_with_unknown_type`](../authz/rbac_organizations.rs).

<a id="get-api-organizations-id"></a>
### `GET /api/organizations/{id}` — asserted

- UUID item을 `200 {"data":Organization}`로 반환한다. 플랫폼 관리자가 방금 만든 조직 ID와 응답 ID가 같음을 직접 단언하며 비관리자 `403`도 단언한다. [`rbac_organizations.rs`](../authz/rbac_organizations.rs).
- 없는/잘못된 UUID 오류 본문은 미단언이다.

<a id="put-api-organizations-id"></a>
### `PUT /api/organizations/{id}` — authorization_only

- 구현 요청은 `{"data":{"name":...,"type":...,"address":...,"city":...,"state":...,"postcode":...,"country_code":...,"contact_email":...,"contact_phone":...,"active":...}}`, 성공은 `200 {"data":Organization}`다.
- 직접 단언은 manager/user/viewer 및 sponsor admin의 `403`뿐이다. [`test_non_admin_cannot_update_organization`](../authz/rbac_organizations.rs), [`test_sponsor_admin_cannot_access_organization_admin_endpoints`](../authz/rbac_organizations.rs).

<a id="delete-api-organizations-id"></a>
### `DELETE /api/organizations/{id}` — authorization_only

- 구현 성공은 모델 delete 후 `204`다. 직접 단언은 manager/user/viewer 및 sponsor admin의 `403`뿐이다. [`test_non_admin_cannot_delete_organization`](../authz/rbac_organizations.rs).
- 성공 삭제의 hard/soft 의미, 종속 데이터 오류, 감사 행은 미단언이다.

## 권한 프로필

요청과 응답은 [`permission_profile_rest.rs`](../../src/web/rest/permission_profile_rest.rs)에 있다. 응답은 wrapper 없는 `{id,name,description,privileges:[{menu_key,can_read,can_edit,can_review,can_lock}],active,built_in,editable}` 또는 그 배열이다. 플랫폼 관리자는 모든 작업에 `?organization_id=uuid`가 필요하고 대상은 활성 CRO/company여야 한다. sponsor 관리자는 자신의 조직으로 고정된다.

<a id="get-api-admin-permission-profiles"></a>
### `GET /api/admin/permission-profiles` — asserted

- `role.list` 권한 뒤 보이는 built-in 역할과 조직 custom 역할 배열을 `200`으로 반환한다.
- 직접 단언: built-in metadata/메뉴 권한, 조직별 custom role 격리, 플랫폼 관리자의 scope 누락 `400`, 20개 제한 연결. [`metadata_web.rs`](../api_administration/role_admin/metadata_web.rs), [`permission_profiles_web.rs`](../authz/rbac_users/permission_profiles_web.rs).

<a id="post-api-admin-permission-profiles"></a>
### `POST /api/admin/permission-profiles` — asserted

- 요청: `{"data":{"name":"...","description":"...","privileges":[{"menu_key":"...","can_read":true,"can_edit":false,"can_review":false,"can_lock":false}],"active":true}}`.
- 빈 name은 `Custom Role`, active 생략은 `true`, privileges 생략은 빈 배열이다. 성공 `201`.
- 직접 단언: 빈 권한 역할, description 보존, 메뉴 권한 저장, 중복 이름/구 migration alias/128자 name·512자 description 초과 `400`, 조직 21번째 역할 `409`. [`crud_web.rs`](../api_administration/role_admin/crud_web.rs), [`validation_web.rs`](../api_administration/role_admin/validation_web.rs), [`test_permission_profiles_reject_twenty_first_custom_role_per_org`](../authz/rbac_users/permission_profiles_web.rs).

<a id="get-api-admin-permission-profiles-id"></a>
### `GET /api/admin/permission-profiles/{id}` — asserted

- `{id}`는 built-in role 문자열 또는 custom UUID다. `role.read` 뒤 `200` row를 반환한다.
- 직접 단언: built-in 시스템 관리자 profile과 custom profile 조회, 플랫폼 관리자의 잘못된 조직 scope에서 `404`. [`test_pdf_admin_read_and_edit_grant_registered_admin_actions`](../api_administration/role_admin/effective_access/administration_web.rs), [`test_system_admin_cross_org_profile_read_is_not_found`](../authz/rbac_users/permission_profiles_web.rs).

<a id="put-api-admin-permission-profiles-id"></a>
### `PUT /api/admin/permission-profiles/{id}` — asserted

- 요청은 create 필드의 patch이며 `description:null`은 설명을 지운다. `active:true`면 `role.restore`, 그 밖에는 `role.update` 정책을 사용한다. built-in 수정은 `403`, UUID 형식 오류는 `400`, 성공 `200` row다.
- 직접 단언: 이름/설명/권한 변경, 중복·길이 오류, 비활성 profile 복원과 21번째 활성 역할 `409`. [`crud_web.rs`](../api_administration/role_admin/crud_web.rs), [`validation_web.rs`](../api_administration/role_admin/validation_web.rs), [`test_restoring_twenty_first_active_custom_role_returns_conflict`](../api_administration/role_admin/metadata_web.rs).

<a id="delete-api-admin-permission-profiles-id"></a>
### `DELETE /api/admin/permission-profiles/{id}` — asserted

- custom profile을 실제 삭제하지 않고 `active=false`로 갱신한 뒤 `204`를 반환한다. built-in은 `400`이다.
- 직접 단언: soft delete 뒤 목록에 비활성 role이 남고, 할당 사용자의 목록 표현이 유지되며, 감사 로그에 `active true→false` UPDATE가 보인다. [`test_delete_permission_profile_soft_deletes_and_keeps_role_visible`](../api_administration/role_admin/crud_web.rs), [`test_admin_can_see_role_create_update_delete_in_audit_logs`](../authz/rbac_audit.rs).

## 설정·공지·브랜딩

설정 DTO와 검증은 [`admin_settings_rest.rs`](../../src/web/rest/admin_settings_rest.rs), 브랜딩은 [`app_branding_rest.rs`](../../src/web/rest/app_branding_rest.rs)다.

<a id="get-api-admin-settings"></a>
### `GET /api/admin/settings` — asserted

- 응답: wrapper 없는 `200` 객체. 주요 키는 `timezone`, `meddra_language`, `meddra_version`, `idf_version`, `company_logo`, `orientation`, `data_ordering`, export/import boolean, `import_date_update`, `appendices`, case number 필드, `workflow_enabled`, `workflow`, session 시간, `notices`다.
- 설정 read 권한과 notice read 권한은 별도이며 notice read가 없으면 `notices:[]`로 가린다.
- 직접 단언: 저장된 MedDRA 설정과 viewer `403`, admin-read custom 권한의 `200`, notice read 권한이 없을 때 마지막 GET의 `notices:[]` redaction. [`settings_web.rs`](../api_administration/admin_settings/settings_web.rs), [`test_dashboard_notices_hidden_without_notice_read`](../api_administration/admin_settings/notices_web.rs), [`test_pdf_admin_read_and_edit_grant_registered_admin_actions`](../api_administration/role_admin/effective_access/administration_web.rs).

<a id="put-api-admin-settings"></a>
### `PUT /api/admin/settings` — asserted

- 요청: `{"data":{AdminSettingsUpdateBody의 부분집합}}`; 기존 JSON과 merge하고 검증한 후 upsert한다. 성공은 최신 전체 설정 객체 `200`.
- 직접 단언: appendices 정규화와 빈 배열 `400`, 존재하지 않는 MedDRA release `400` 및 원값 유지, 조직별 idle/warning 시간과 최소·상대값 검증, timezone 변경 감사와 동일 값 재저장 시 중복 감사 없음. [`appendices_web.rs`](../api_administration/admin_settings/appendices_web.rs), [`settings_web.rs`](../api_administration/admin_settings/settings_web.rs), [`audit_web.rs`](../api_administration/admin_settings/audit_web.rs).
- alignment 필드 전체 round-trip, 지원/비지원 `import_date_update`, case-number 설정이 생성 case의 C.1.1/C.1.8.1 값과 순번에 미치는 영향도 직접 단언된다. workflow는 음수 `due_days`, 알 수 없는 역할과 `system_admin`을 `400`으로 거부하고 빈 `allowed_roles`를 제한 없음으로 보존하는 분기가 단언된다. [`test_admin_settings_round_trips_alignment_fields`](../api_administration/admin_settings/settings_web.rs), [`test_case_number_settings_generate_c11_and_c181`](../api_cases/settings.rs), [`test_workflow_settings_reject_negative_due_days`](../api_administration/admin_settings/workflow_web.rs), [`test_workflow_settings_reject_unknown_role`](../api_administration/admin_settings/workflow_web.rs), [`test_workflow_settings_reject_system_admin_role`](../api_administration/admin_settings/workflow_web.rs), [`test_workflow_settings_allow_empty_roles_as_unrestricted`](../api_administration/admin_settings/workflow_web.rs).
- 소스 전용으로 남는 대표 분기는 잘못된 IANA timezone/orientation, case-number 필수 필드와 중복 format field, workflow status 누락·중복·editable/Saved 필수 검증이다.

<a id="get-api-settings-runtime"></a>
### `GET /api/settings/runtime` — asserted

- 인증 사용자가 필요한 축약 설정을 wrapper 없이 `200`으로 받는다: `{timezone,meddra_language,meddra_version,orientation,data_ordering,notation,apply_sender_info_to_imported_cases,import_date_update,appendices,idle_session_minutes,session_warning_minutes,notices,notices_revision}`.
- notice update 권한은 전체 공지를, notice read만 있으면 현재 timezone 기준 유효 공지만, 둘 다 없으면 빈 배열을 받는다.
- 직접 단언: 조직별 session 값, 공지 조직 격리, notice read 없는 사용자 redaction. [`test_idle_session_settings_are_org_scoped_and_validated`](../api_administration/admin_settings/settings_web.rs), [`notices_web.rs`](../api_administration/admin_settings/notices_web.rs).

<a id="put-api-admin-notices"></a>
### `PUT /api/admin/notices` — asserted

- 요청: `{"data":{"revision":"현재 revision","notices":[{"id":"...","title":"...","body":"...","effective_date":"YYYY-MM-DD","expire_date":"YYYY-MM-DD"}]}}`. 서버가 writer를 현재 사용자 이메일로 덮어쓴다.
- revision 누락·불일치는 `400`; 날짜 형식, 중복 ID, 제목/ID 누락, effective 이후 expire도 `400`. 성공은 `{notices:[...],revision:"..."}` `200`.
- 직접 단언: 작성/수정 응답, writer, revision 갱신, 조직 격리, CREATE/UPDATE 감사의 old/new title. [`test_dashboard_notices_are_org_scoped_and_audited`](../api_administration/admin_settings/notices_web.rs).
- 미단언: 경쟁 요청으로 실제 stale revision `400`과 모든 날짜 오류.

<a id="get-api-app-branding"></a>
### `GET /api/app/branding` — asserted

- 인증된 `/api` 경로다. 응답은 `200 {"data":{"appName":"...","appShortName":"..."}}`.
- 소스 기본값은 `QVIS Safety`; `E2BR3_APP_NAME`, `E2BR3_APP_SHORT_NAME`의 trim된 비어 있지 않은 값을 사용한다.
- 기본값과 설정값은 [`app_branding_web.rs`](../api_administration/app_branding_web.rs)에서 직접 단언된다. 빈 문자열 fallback과 인증 누락은 이 경로 전용 테스트에서 단언되지 않는다.

## 감사 조회

`AuditLog`와 chain report는 [`audit.rs`](../../../../libs/lib-core/src/model/audit.rs), HTTP projection은 [`audit_rest.rs`](../../src/web/rest/audit_rest.rs)다. 저장 감사 행은 snake_case 키를 유지한다. 감사 사유·범주는 인증 middleware가 각각 `x-e2br3-reason-for-change`, `x-e2br3-change-category`의 percent-decoded 값을 컨텍스트에 넣어 전달한다.

<a id="get-api-audit-logs"></a>
### `GET /api/audit-logs` — asserted

- 쿼리: `filters[table_name]`, `filters[action]`, `list_options.*`. 성공 `200 {"data":[{"id", "organization_id","table_name","record_id","action","user_id","reason_for_change","change_category","e_signature_id","user_display","changed_fields","old_values","new_values","ip_address","user_agent","prev_hash","entry_hash","created_at"},...]}`.
- 직접 단언: admin `200`, viewer/user/권한 없는 custom manager `403`, sponsor admin은 자신의 조직만, system admin은 교차 조직 행도 조회. 사용자·role 변경 감사 내용도 실제 API로 조회한다. [`rbac_audit.rs`](../authz/rbac_audit.rs).
- 미단언: 필터/정렬/페이지의 전체 조합과 chain hash 필드 노출 제한.

<a id="get-api-audit-logs-by-record"></a>
### `GET /api/audit-logs/by-record/{table_name}/{record_id}` — asserted

- `{table_name}`은 문자열, `{record_id}`는 UUID. 선택 `?field=email,username`은 `changed_fields`, `old_values`, `new_values` 중 하나라도 해당 필드를 건드린 행만 남긴다.
- 성공 `200 {"data":[AuditLog,...]}`. 직접 단언: system admin 조회, 권한 없는 사용자 `403`, 사용자 CREATE 내용, field 필터, hard-deleted case DELETE 이력, 교차 조직 collection 조회에서 case-version descendant 유지. [`rbac_audit.rs`](../authz/rbac_audit.rs).
- `table_name` allowlist는 핸들러에 없으며 임의 이름·잘못된 UUID와 다중 field 공백 처리는 직접 단언되지 않는다.

<a id="get-api-cases-case-id-audit-trail"></a>
### `GET /api/cases/{case_id}/audit-trail` — asserted

- case 부모 scope가 필요한 별도 조회다. 쿼리는 `list_options.limit`, `offset`, `order_bys`; 정렬은 `!created_at`만 허용하며 음수 limit/offset은 `400`이다.
- 응답은 `200 {"data":[{"no", "auditLogId","dateTime","userDisplay","page","item","rowNo1","rowNo2","rowNo3","value","notation","nullFlavor","reason","eSignatureId"},...]}`. case와 descendant 감사 변경을 필드 단위 UI 행으로 펼치고 최신 시간/로그 ID 순으로 정렬한 뒤 pagination한다.
- 직접 단언: 한 audit log의 여러 projected field를 offset으로 나누는 동작, cross-org case scope의 `403|404`, system admin도 부모 case scope를 우회하지 못함. [`test_case_audit_trail_paginates_projected_rows`](../authz/rbac_audit.rs), [`test_cross_org_case_audit_requires_case_scope_but_collection_keeps_descendants`](../authz/rbac_audit.rs).
- editor 연결: FDA device scalar 저장 뒤 `item`, `value`, 반복행 번호, reason을 이 API로 조회하는 활성 generated test가 있다. [`dg.generated.rs`](../api_editor/pages/dg.generated.rs), [`projection.rs`](../api_editor/audit/projection.rs). 이는 UI 자체 렌더링 테스트가 아니라 editor 저장 결과의 조회 표현 단언이다.

<a id="get-api-audit-logs-verify-integrity"></a>
### `GET /api/audit-logs/verify-integrity` — asserted

- 구현은 audit collection 권한 뒤 hash chain을 검사하고 `200 {"data":{"total_rows":number,"verified_ok_rows":number,"broken_rows":number,"first_broken_id":number|null,"first_broken_reason":string|null,"checked_at":"RFC3339"}}`를 반환한다.
- [`test_audit_integrity_reports_healthy_chain_and_requires_permission`](../api_administration/audit_web.rs)는 admin GET 200, 비어 있지 않은 정상 chain의 total=verified·broken=0·최초 손상 필드 null·RFC3339 checked_at, viewer GET 403와 보고서 비노출을 검사한다.
- [`test_audit_chain_preserves_id_order_with_waiting_writer`](../../../../libs/lib-core/tests/database/audit_chain.rs)는 별도 lib-core/database 검사다. 실제 advisory lock 대기 중인 writer를 만들어 ID 순서와 hash chain을 검증하며 HTTP 근거로 계산하지 않는다.
- 손상 chain의 first_broken_id/reason 반환과 다른 권한 조합은 미단언이다. SQL 함수가 전체 chain을 검사하므로 이 보고서는 조직별 row 필터 결과가 아니다.

## 인증과 health

인증 handler는 [`handlers_login.rs`](../../../../libs/lib-web/src/handlers/handlers_login.rs), 라우터는 [`routes_login.rs`](../../src/web/routes_login.rs)다.

<a id="post-auth-v1-login"></a>
### `POST /auth/v1/login` — asserted

- 요청: `{"email":"...","pwd":"..."}`. 이메일로 사용자를 찾고 조직·역할 컨텍스트를 만들며 암호 hash를 검증한다. 구 scheme이면 로그인 사용자 귀속으로 hash를 업그레이드한다.
- 성공: `200 {"result":{"success":true,"must_change_password":bool}}`와 auth cookie. 잘못된 암호/없는 이메일은 `403`과 `LOGIN_INVALID_CREDENTIALS`; nil/system 조직 계정은 `403`이다.
- 직접 단언: 성공 상태, 잘못된 자격 증명 오류 envelope, 이메일 대소문자, 지정 초기 암호, legacy hash 업그레이드, 최초 암호 변경 flag/후속 흐름. [`authentication_web.rs`](../authz/authentication_web.rs), [`test_new_user_temp_password_and_first_login_reset_flow`](../api_administration/users/access_web.rs).
- 성공 테스트 일부는 cookie 존재/속성을 단언하지 않는다. 최초 암호 흐름만 실제 login 응답 cookie를 후속 요청에 사용한다.

<a id="post-auth-v1-logoff"></a>
### `POST /auth/v1/logoff` — asserted

- 공개 라우트이며 요청은 `{"logoff":true|false}`. true이면 auth cookie 제거를 요청하고 `200 {"result":{"logged_off":true}}`, false이면 제거하지 않고 false를 반환한다.
- [`test_auth_login_refresh_logoff`](../authz/authentication_web.rs)는 별도로 만든 token을 cookie로 보내 true 요청의 `200`만 확인한다. login 응답 cookie의 연속 사용, 응답 body, `Set-Cookie` 제거 속성, 이후 token 무효화는 단언하지 않는다.

<a id="post-auth-v1-refresh"></a>
### `POST /auth/v1/refresh` — asserted

- 인증 middleware를 거치는 빈-body POST다. 현재 사용자를 다시 읽고 새 auth cookie를 설정하며 `200 {"data":{"expiresAt":"RFC3339"}}`를 반환한다.
- 별도로 생성한 유효 token의 `200`, 변조 token과 누락 token의 `403`이 직접 단언된다. [`authentication_web.rs`](../authz/authentication_web.rs).
- login cookie를 그대로 refresh에 전달하는 end-to-end 연결, 새 cookie/만료 값, 이전 token의 효력은 미단언이다.

<a id="get-health"></a>
### `GET /health` — uncovered

- 공개 handler는 body 없이 `204 No Content`를 반환한다. 전체 응답 mapper가 `x-request-id`를 추가하는 구현은 별도 unit test로 확인되지만 `/health` router를 실제 호출하지 않는다. [`lib.rs`](../../src/lib.rs), [`mw_res_map.rs`](../../../../libs/lib-web/src/middleware/mw_res_map.rs).
- health route의 `204`, body, content-type, 인증 독립성은 활성 HTTP 테스트에서 직접 단언되지 않는다.

## 비활성·간접 증거와 남은 공백

- [`dump_protected_inventory_snapshot`](../authz/protected_route_inventory.rs)은 `#[ignore]`이며 수동 snapshot 생성기다. 활성 inventory 테스트도 라우터 소스와 snapshot 문자열을 비교할 뿐 HTTP 인증을 단언하지 않는다.
- [`authorization_snapshot.rs`](../authz/authorization_snapshot.rs)는 extractor 단위 테스트다. snapshot attach 성공/누락을 확인하지만 위 34개 경로 중 어느 하나의 HTTP 허용을 대신하지 않는다.
- [`lib.rs`](../../src/lib.rs)의 CSRF 테스트는 origin 판정 helper만 호출한다. 실제 state-changing `/api` 요청의 `403`과 login/logoff/refresh의 CSRF 경계는 단언하지 않는다.
- 조직 목록·수정·삭제와 workflow options는 성공 기능 단언이 없다. health는 경로 자체의 활성 HTTP 단언이 없다. Audit integrity는 정상 chain report와 viewer `403`까지 활성 HTTP 단언이 있고, 손상 chain 보고 분기가 남아 있다.
- 사용자/role/settings 변경은 일부 감사 행을 확인하지만, 모든 mutation의 감사 user/reason/category/e-signature와 실패 요청의 무변경성은 포괄적으로 단언되지 않는다.

34개 등록 경로와 상태·증거의 최종 병합 위치는 [`coverage.tsv`](coverage.tsv)다.
