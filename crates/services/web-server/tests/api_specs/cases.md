# Case API 명세와 테스트 근거

이 문서는 `tests/authz/protected_route_inventory.snapshot`의 `/api/cases*`, `/api/case-query*` 중 Case CRUD/list/intake/validation/workflow/locking/settings/review/assignment, clinical/drug/patient/narrative/safety-report, field notation, identifier/source-document API를 현재 소스에서 역추적한 명세다. editor 전체, export/import, submissions/history/dispatch, message-header, `PUT /submission-receiver`, administration 소유의 case audit-trail은 제외했다.

검증 상태는 **runtime NOT_RUN**이다. 테스트, 빌드, API, DB는 실행하지 않았고 라우트 등록, handler/DTO, 활성 테스트 assertion만 읽었다. 총 231개 operation 중 `asserted` 108, `authorization_only` 1, `registration_only` 122, `uncovered` 0개다. 상세 판정과 operation별 gap은 [전체 커버리지](coverage.tsv)에서 `spec=cases.md#...` 행을 보면 된다.

## 공통 계약

모든 API는 인증 cookie와 authorization snapshot을 사용한다. collection read는 사용자 조직/송신자/제품/시험 scope로 제한되고, item/child read는 case scope 밖의 ID를 보통 404로 감춘다. mutation은 권한 외에도 삭제·잠금·QC 완료·validated·workflow read-only 상태에서 거절될 수 있다. 실제 assertion에는 400, 403, 404, 409가 있으며, handler/model validation에 따라 422도 반환된다.

handler/DTO 소스에서 읽은 일반 JSON body는 `{"data": {...}}`다. 공통 CRUD handler의 소스상 collection/item 응답은 각각 `200 {"data": [...]}`, `200 {"data": {...}}`; create는 `201`, update와 restore는 `200`이다. soft-delete handler는 주로 `204`이고 기본 list에서 삭제 행을 숨기지만 patient identifier delete처럼 `200 {"data": {...}}`인 예외가 있다. 이 문단은 실행 결과가 아니며, 실제 assertion 여부는 operation별 coverage 상태로만 판단한다. update/delete handler는 기존 공통 audit reason 전달 규칙을 사용하며 Case 삭제는 body의 `reason_for_change`와 `e_signature {meaning,password}`를 요구한다. 필드 이름은 별도 표기가 없으면 DTO의 snake_case이며 Case 공개 create만 camelCase를 받는다.

`registration_only`는 활성 [protected_inventory_matches_checked_in_snapshot](../authz/protected_route_inventory.rs)가 method/path 등록을 확인한다는 뜻이다. 같은 파일의 ignored dump test는 출력 도구일 뿐 행동·status 근거로 세지 않았다. fixture/setup 호출과 다른 endpoint를 준비하기 위한 helper도 대상 endpoint의 coverage로 세지 않았다.

<a id="case-core"></a>
## Case core와 query

| 메서드 | 경로 |
|---|---|
| `GET` | `/api/case-query/catalog` |
| `POST` | `/api/case-query/search` |
| `GET`, `POST` | `/api/cases` |
| `GET`, `POST`, `PUT`, `DELETE` | `/api/cases/{case_id}/receiver` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{id}` |
| `GET` | `/api/cases/link-options` |
| `GET` | `/api/cases/list-view` |
| `POST` | `/api/cases/query` |

- `POST /cases`: `data.safetyReportIdentification.safetyReportId`, `dgPrdKey`, `status`, `reviewReceiversJson`, `workflowRoutesJson`, `mfdsReportType`, `fdaReportType`, `reportYear`. status는 생성을 위한 draft만 허용한다. 정상 응답은 `201 Data<CaseReadResult>`.
- `PUT /cases/{id}`: `data`에 `dg_prd_key`, `status`, `review_receivers_json`, `workflow_routes_json`, `mfds_report_type`, `fda_report_type`, `report_year`; 바깥에 `reason_for_change`, `e_signature`. patch-value 필드는 omitted/문자열/null을 구분한다. `review_receivers_json`은 array 또는 `reviewReceivers` array 객체이며 receiver 이름, 비음수 reportDue, YYYY-MM-DD 날짜를 검증한다.
- `GET /cases`: raw query의 `filters[organization_id|safety_report_id|status]`와 `list_options[limit|offset|order_bys]`. `GET /list-view`도 list options와 comma-separated `authorities`를 받고 `{items,summary}`를 반환한다. `GET /link-options`는 `{items}`.
- `GET /cases/{id}`: Case 공개 필드와 `qc_state`, `is_locked`, workflow actionability/write-block reason을 반환한다.
- `DELETE /cases/{id}`: 본문 `reason_for_change`, `e_signature {meaning,password}`를 검증하고 `200 {"data":CaseReadResult}`로 deleted 상태를 반환한다. 삭제 후에도 Case 조회·목록·lifecycle에 남는다. 일반 child DELETE의 204와 구분한다.
- `POST /cases/query`와 alias `POST /case-query/search`: body 자체가 camelCase `conditions[] {page,item,operator,values}`, `resultPages`, `reportTypeLast`, `noAckAcceptHistory`, `authorities`, `limit`, `offset`; 최대 5,000행. 응답은 `caseIds`, list-view `items`, 선택한 `elements`, `elementValues`, `total`. `GET /case-query/catalog`은 query 가능한 page/item catalog를 반환한다.
- 주요 assertion은 Case create/list/get/update/delete, 조직·송신자 scope, list-view projection과 query 결과다. catalog, alias search, link-options에는 행동 assertion이 없다.

근거:

- [tests/api_cases/contract/delete.rs::test_delete_case_requires_reason_for_change](../api_cases/contract/delete.rs)
- [tests/api_cases/contract/delete.rs::test_delete_case_soft_deletes_and_keeps_case_visible](../api_cases/contract/delete.rs)
- [tests/api_cases/contract/list_view.rs::test_case_list_view_honors_limit_and_offset](../api_cases/contract/list_view.rs)
- [tests/api_cases/contract/list_view.rs::test_case_list_view_projects_reference_grid_fields](../api_cases/contract/list_view.rs)
- [tests/api_cases/contract/query.rs::test_case_query_returns_lightweight_list_view_items](../api_cases/contract/query.rs)
- [tests/api_cases/lifecycle.rs::test_case_save_does_not_auto_transition_status_when_updating_fields](../api_cases/lifecycle.rs)
- [tests/api_cases/lifecycle.rs::test_create_case_rejects_every_protected_lifecycle_status](../api_cases/lifecycle.rs)
- [tests/api_cases/locking.rs::test_case_read_returns_separate_qc_and_lock_axes](../api_cases/locking.rs)
- [tests/api_cases/scope/cases.rs::test_case_list_is_filtered_by_sender_scope](../api_cases/scope/cases.rs)
- [tests/api_cases/scope/cases.rs::test_case_update_requires_matching_sender_scope](../api_cases/scope/cases.rs)
- [tests/api_cases/subresources/singleton.rs::test_singleton_post_endpoints_are_idempotent](../api_cases/subresources/singleton.rs)
- [tests/authz/rbac_cases.rs::test_admin_can_create_case](../authz/rbac_cases.rs)
- [tests/authz/rls_web.rs::test_rls_list_cases_filters_org](../authz/rls_web.rs)

<a id="intake"></a>
## Intake와 follow-up

| 메서드 | 경로 |
|---|---|
| `POST` | `/api/cases/{source_case_id}/follow-up` |
| `POST` | `/api/cases/from-intake` |
| `POST` | `/api/cases/intake-check` |

- intake 공통 필드: `safety_report_id`, `date_of_most_recent_information`, `report_type`, reporter organization(+null flavor), sponsor study number(+null flavor), patient initials(+null flavor), investigation number(+null flavor), `age_d2_2a`, `sex_d5`(+null flavor), `dg_prd_key`, reaction MedDRA version/code, AE start date(+null flavor).
- `POST /intake-check`은 위 필드를 `data`에 받고 `200 {duplicate,basis_complete,warnings,matches}`. report type별 활성 중복 기준과 필드 쌍을 검증한다.
- `POST /from-intake`는 추가로 필수 `authority`, `date_of_most_recent_information`, `report_type`; 선택 `transmission_date`, `date_first_received_from_source`, `status`, `allow_duplicate_override`, MFDS/FDA report type, report year를 받는다. Case shell 하위 행을 transaction으로 만들고 `201 {case_id,safety_report_id,version}`.
- `POST /{source_case_id}/follow-up`은 editor follow-up page patch request를 받고 새 Case ID와 safety-report version을 반환한다. source scope, 버전 증가, 복제 범위가 assertion 되어 있다.

근거:

- [tests/api_cases/intake/duplicates.rs::test_case_intake_duplicate_check_and_create](../api_cases/intake/duplicates.rs)
- [tests/api_cases/intake/duplicates.rs::test_case_intake_duplicate_check_requires_all_active_fields](../api_cases/intake/duplicates.rs)
- [tests/api_cases/intake/input.rs::test_case_from_intake_rolls_back_shell_when_patient_create_fails](../api_cases/intake/input.rs)
- [tests/api_cases/intake/input.rs::test_follow_up_creation_preserves_scope_and_allocates_versions](../api_cases/intake/input.rs)

<a id="workflow-validation"></a>
## Validation, lifecycle, workflow, locking

| 메서드 | 경로 |
|---|---|
| `GET` | `/api/cases/{case_id}/validation` |
| `GET` | `/api/cases/{case_id}/versions` |
| `GET` | `/api/cases/{id}/lifecycle` |
| `POST` | `/api/cases/{id}/lock/toggle` |
| `POST` | `/api/cases/{id}/restore` |
| `POST` | `/api/cases/{id}/review/toggle` |
| `POST` | `/api/cases/{id}/validator/mark-validated` |
| `POST` | `/api/cases/{id}/workflow/assign` |
| `GET` | `/api/cases/{id}/workflow/events` |
| `POST` | `/api/cases/{id}/workflow/transition` |
| `GET` | `/api/cases/workflow/config` |

- `GET /validation?authority=ich|fda|mfds`; 생략 시 receiver 설정에서 추론하고 없으면 ICH다. `CaseValidationReport`를 `200 Data`로 반환하며 중복/알 수 없는 authority는 400.
- 목록 `warn`은 선택 기관들의 `(code,path)` 중복을 제거한 캐시 집계다. Case 변경, validator 구현 변경, 활성 사전 변경·롤백 후의 구버전 캐시 및 판정 버전이 없는 기존 캐시는 `Not checked`로 표시한다. 재검증이 새 버전의 집계를 저장한다. 검증 도중 사전 기준이 바뀌면 결과를 캐시하지 않고 재시도를 요구하는 400 응답을 반환한다.
- `POST /validator/mark-validated`는 `x-validator-token`과 validation issue 0개를 요구하고 Case를 validated로 갱신한다. 성공/실패와 system audit reason이 assertion 되어 있다.
- `POST /lock/toggle`, `POST /review/toggle`: `{"data":{"expected_status":"..."}}`; stale 화면 상태는 409. Case read shape를 `200`으로 반환한다.
- `POST /restore`: `reason_for_change`가 필수이고 deleted Case를 draft로 복원한다. `GET /lifecycle`은 동일 safety-report 계열의 `{safety_report_id,current_case_id,items[]}`.
- `POST /workflow/transition`: `data {to_status,target_role?,target_user_id?,comment?,due_at?,override_reason?}`. `POST /workflow/assign`: `data {target_role,target_user_id?,comment?,due_at?,override_reason?}`. due_at은 RFC3339. 역할/assignee, lock, 관리자 override를 검증하고 `CaseReadResult`를 반환한다.
- `GET /workflow/events`는 camelCase event array, `GET /workflow/config`는 `workflowEnabled,statuses,roles`. `GET /versions`는 현재 활성 assertion이 200/404 조직 가시성뿐이어서 authorization_only다.

근거:

- [tests/api_cases/contract/delete.rs::test_delete_case_soft_deletes_and_keeps_case_visible](../api_cases/contract/delete.rs)
- [tests/api_cases/lifecycle.rs::test_review_toggle_allows_validated_to_draft_transition](../api_cases/lifecycle.rs)
- [tests/api_cases/locking.rs::test_case_can_be_marked_locked](../api_cases/locking.rs)
- [tests/api_cases/locking.rs::test_case_read_returns_separate_qc_and_lock_axes](../api_cases/locking.rs)
- [tests/api_cases/locking.rs::test_lifecycle_toggle_rejects_stale_screen_status](../api_cases/locking.rs)
- [tests/api_cases/settings.rs::test_workflow_config_runtime_endpoint_returns_default_statuses](../api_cases/settings.rs)
- [tests/api_cases/validation.rs::test_validation_defaults_to_ich_profile](../api_cases/validation.rs)
- [tests/api_cases/validation.rs::test_validation_rejects_unknown_profile](../api_cases/validation.rs)
- [tests/api_cases/validation.rs::test_validator_endpoint_marks_validated_when_clean](../api_cases/validation.rs)
- [tests/api_cases/validation.rs::test_validator_endpoint_requires_token](../api_cases/validation.rs)
- [tests/api_cases/workflow.rs::test_locked_case_blocks_workflow_transition_even_for_admin_override](../api_cases/workflow.rs)
- [tests/api_cases/workflow.rs::test_workflow_assignment_updates_owner_without_changing_status](../api_cases/workflow.rs)
- [tests/api_cases/workflow.rs::test_workflow_transition_updates_case_and_persists_event](../api_cases/workflow.rs)
- [tests/api_submission/validation.rs::test_submission_rejects_case_validation_before_routing_or_dispatch](../api_submission/validation.rs)
- [tests/authz/rls_web.rs::test_rls_case_versions_filters_org](../authz/rls_web.rs)

<a id="field-notation"></a>
## Field notation

| 메서드 | 경로 |
|---|---|
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/field-notation` |

- `GET`, `DELETE`: query `e2bCode` 필수, `recordId` 선택. `PUT`: body `{e2bCode,recordId?,notation}`, unknown field 거절. e2bCode는 1–64 byte의 ASCII 영숫자/점/밑줄/하이픈, notation은 최대 10,000 UTF-8 byte다.
- recordId를 생략한 scalar 또는 같은 Case의 유효한 행이면, GET은 notation 저장 행이 없어도 `200 {id:null,e2bCode,recordId,notation:""}`다. PUT upsert와 DELETE는 `200 FieldNotationResponse`이며, 유효한 행의 없는 notation 삭제도 멱등적 200이다.
- 세 operation 모두 asserted다. [field_notation.rs](../api_cases/field_notation.rs)는 scalar 저장→조회→수정 시 ID 유지→공백 PUT(trim 후 빈 문자열, 행 유지)→DELETE→없는 행 조회를 검사한다. 감사 action 순서 CREATE/UPDATE/UPDATE/DELETE도 확인한다.
- 실제 Drug 행 UUID로 같은 e2bCode의 scalar/record notation을 따로 저장·조회하고 ID/값 격리를 검사한다. viewer GET은 200, PUT/DELETE는 403이며 거절 후 저장 값과 해당 notation 감사 action이 불변인지 확인한다.
- `recordId`를 주면 GET/PUT/DELETE 공통으로 실제 행의 FK chain이 요청 Case로 이어지는지 확인한다. 존재하지 않는 UUID, 같은 조직의 다른 Case 행, 다른 조직의 행은 `404 ENTITY_UUID_NOT_FOUND`다. 요청 Case의 권한 검사 뒤, notation 조회/변경과 같은 DB transaction에서 검사한다. 거절하면 notation을 변경하기 전에 rollback한다.
- Soft-deleted 행은 복원 가능한 기존 identity이므로 허용한다. `recordId=None`인 scalar 계약은 유지한다. `e2bCode`와 record table 사이의 필드 의미 일치까지 검사하는 계약은 아니다.
- 신규 [존재·nested/soft-delete 검사](../api_cases/field_notation.rs)와 [case/org 소속 검사](../api_cases/field_notation.rs)는 실제 Drug/Dosage 행을 사용한다. 임의 UUID와 교차 소속의 GET/PUT/DELETE 404, 정상 notation 재조회 값·ID, 해당 조직의 notation 감사 건수 불변을 단언한다. 37종 각각의 runtime 성공을 검사한 것은 아니다.

행 주체의 범위는 registry Editor audit subject 36종과 실제 frontend device-characteristic subject 1종이다. 같은 모델의 [membership query](../../../../libs/lib-core/src/model/e2b_field_notation.rs)가 아래 FK 경로를 사용한다.

| 분야 | 허용 record table | Case 연결 |
|---|---|---|
| CI | cases, safety_report_identification, documents_held_by_sender, source_documents, other_case_identifiers, linked_report_numbers | cases.id 또는 직접 case_id |
| RP/SD/LR | primary_sources, sender_information, receiver_information, literature_references | 직접 case_id |
| SI | study_information, study_registration_numbers, study_fda_cross_reported_inds | 직접 case_id 또는 study_information 경유 |
| DM/DH | patient_information, patient_identifiers, medical_history_episodes, past_drug_history, patient_death_information, reported_causes_of_death, autopsy_causes_of_death, parent_information, parent_medical_history, parent_past_drug_history | patient 또는 death/parent→patient 경유 |
| AE/LB | reactions, test_results | 직접 case_id |
| DG | drug_information, drug_active_substances, dosage_information, drug_indications, drug_device_characteristics, fda_device_information, fda_device_codes, drug_reaction_assessments, relatedness_assessments | drug 또는 device/assessment→drug 경유; assessment는 reaction도 같은 Case 필요 |
| NR | narrative_information, sender_diagnoses, case_summary_information | 직접 case_id 또는 narrative 경유 |

기존 데이터에 이미 들어간 잘못된 recordId를 삭제·수정하는 migration은 없다. 이 행의 개별 GET/PUT/DELETE는 404가 되며, CIOMS에서 사용하는 `list_by_case`는 기존 행을 그대로 읽는다. 기존 데이터 정리와 export에서의 처리 여부는 별도 확인 대상이다.
- 미단언: 잘못된 e2bCode·notation 길이·unknown field 경계, 모든 행 종류/권한 조합, 감사 old/new payload, hard-delete 시 notation 정리. 입력 길이 제한은 구현의 UTF-8 byte 길이 기준이다.

<a id="drug"></a>
## Drug와 하위 리소스

| 메서드 | 경로 |
|---|---|
| `GET`, `POST` | `/api/cases/{case_id}/drugs` |
| `GET`, `POST` | `/api/cases/{case_id}/drugs/{drug_id}/active-substances` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{drug_id}/active-substances/{id}` |
| `POST` | `/api/cases/{case_id}/drugs/{drug_id}/active-substances/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/drugs/{drug_id}/device-characteristics` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{drug_id}/device-characteristics/{id}` |
| `POST` | `/api/cases/{case_id}/drugs/{drug_id}/device-characteristics/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/drugs/{drug_id}/devices` |
| `GET`, `POST` | `/api/cases/{case_id}/drugs/{drug_id}/devices/{device_id}/codes` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{drug_id}/devices/{device_id}/codes/{id}` |
| `POST` | `/api/cases/{case_id}/drugs/{drug_id}/devices/{device_id}/codes/{id}/restore` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{drug_id}/devices/{id}` |
| `POST` | `/api/cases/{case_id}/drugs/{drug_id}/devices/{id}/restore` |
| `PUT` | `/api/cases/{case_id}/drugs/{drug_id}/devices/replace` |
| `GET`, `POST` | `/api/cases/{case_id}/drugs/{drug_id}/dosages` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{drug_id}/dosages/{id}` |
| `POST` | `/api/cases/{case_id}/drugs/{drug_id}/dosages/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/drugs/{drug_id}/indications` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{drug_id}/indications/{id}` |
| `POST` | `/api/cases/{case_id}/drugs/{drug_id}/indications/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments` |
| `GET`, `POST` | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}` |
| `POST` | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{id}/restore` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{id}` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/drugs/{id}` |
| `POST` | `/api/cases/{case_id}/drugs/{id}/restore` |

공통 CRUD handler 소스상 status는 create 201, list/get/update 200, soft delete 204, restore 200이다. path의 `case_id`, `drug_id`, `device_id`, `assessment_id`가 body의 부모 ID보다 우선하며 다른 부모 아래의 child ID는 404다. assertion이 없는 operation의 실제 runtime 결과로 간주하지 않는다.

허용 필드:

- drug: create `case_id, source_product_presave_id, sequence_number, drug_characterization, medicinal_product, drug_authorization_number, manufacturer_name, manufacturer_country, batch_lot_number, cumulative_dose_first_reaction_value/unit, gestation_period_exposure_value/unit, action_taken, investigational_product_blinded, mpid/version/source_code_system/source_code_system_version, mfds_mpid/version, phpid/version, obtain_drug_country, fda_additional_info_coded(+null_flavor), drug_additional_info_codes_json, drug_additional_information, fda_specialized_product_category, fda_other_characterization`; update는 부모/sequence를 제외한 같은 필드.
- active-substance: create `drug_id, sequence_number, substance_name, substance_termid/version/code_system, mfds_version/id, strength_value/unit`; update는 부모/sequence 제외.
- dosage: `drug_id, sequence_number, dose_value/unit, number_of_units, frequency_unit, first/last_administration_date/raw/null_flavor, duration_value/unit, continuing, batch_lot_number(+null_flavor), dosage_text, dose_form(+null_flavor/termid/version), route_of_administration(+null_flavor/termid/version/code_system), parent_route(+null_flavor/termid/version/code_system)`.
- indication: `drug_id, sequence_number, indication_text(+null_flavor), indication_meddra_version/code`.
- device-characteristic: `drug_id, sequence_number, code, code_system, code_display_name, value_type, value_value, value_code, value_code_system, value_display_name`.
- FDA device: `drug_id, sequence_number, malfunction, device_brand_name(+null_flavor), common_device_name(+null_flavor), device_product_code, manufacturer_name/address/city/state/country, device_usage, device_lot_number, operator_of_device`. device code는 `device_id, element, sequence_number, value_code`. `PUT /devices/replace`는 `data.devices[]`와 각 device의 code/follow-up 배열을 원자 교체한다.
- drug-reaction assessment: create `drug_id, reaction_id, administration_start_interval_value/unit, last_dose_interval_value/unit, recurrence_action, reaction_recurred, dechallenge_result, expectedness`; update는 drug_id 제외. relatedness는 create `drug_reaction_assessment_id, sequence_number, source_of_assessment, method_of_assessment, method_of_assessment_kr1, result_of_assessment, result_of_assessment_kr1(+null_flavor), result_of_assessment_kr2`; update는 부모/sequence 제외.

활성 assertion은 drug와 active-substance의 일부 CRUD, device create/replace/list, device code create, device-characteristic create/list, dosage/indication create, relatedness create에 집중되어 있다. 나머지 item verb는 registration_only다. device와 code POST는 생성 응답 뒤 editor DG 조회로 저장된 값을 확인한다.

근거:

- [tests/api_editor/pages/dg.generated.rs::device_subresources_survive_dg_row_reload](../api_editor/pages/dg.generated.rs)

- [tests/api_cases/subresources/drug.rs::test_kr_device_characteristics_round_trip_via_api](../api_cases/subresources/drug.rs)
- [tests/api_cases/subresources/drug.rs::test_relatedness_assessment_endpoints_ok](../api_cases/subresources/drug.rs)
- [tests/api_cases/subresources/drug.rs::dg_kr_product_fields_create_read_and_update_independently](../api_cases/subresources/drug.rs)
- [tests/api_cases/subresources/drug.rs::dg_kr_substance_fields_create_read_and_update_independently](../api_cases/subresources/drug.rs)
- [tests/api_cases/subresources/drug.rs::drug_child_id_is_hidden_under_a_different_drug](../api_cases/subresources/drug.rs)
- [tests/api_cases/subresources/drug.rs::fda_device_replace_normalizes_blank_null_flavors](../api_cases/subresources/drug.rs)
- [tests/api_cases/subresources/drug.rs::fda_device_replace_rolls_back_on_constraint_violation](../api_cases/subresources/drug.rs)
- [tests/api_cases/subresources/drug.rs::test_drug_active_substance_soft_delete_restore](../api_cases/subresources/drug.rs)
- [tests/api_cases/subresources/drug.rs::test_drug_subresources_endpoints_ok](../api_cases/subresources/drug.rs)
- [tests/api_editor/pages/dg.generated.rs::deleted_drug_can_be_read_and_restored_with_children_and_guards](../api_editor/pages/dg.generated.rs)

<a id="clinical"></a>
## Reaction과 test result

| 메서드 | 경로 |
|---|---|
| `GET`, `POST` | `/api/cases/{case_id}/reactions` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/reactions/{id}` |
| `POST` | `/api/cases/{case_id}/reactions/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/test-results` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/test-results/{id}` |
| `POST` | `/api/cases/{case_id}/test-results/{id}/restore` |

- reaction create 필드: `case_id, sequence_number, primary_source_reaction/translation, reaction_language, reaction_meddra_code/version, term_highlighted, serious`, 각 serious criterion(+null flavor), `required_intervention(+null flavor), expectedness, severity`, MFDS device AE 분류/결과/원인/action 필드, `start_date/end_date(+null flavor), duration_value/unit, outcome, medical_confirmation, country_code, deleted`. update는 case_id/sequence 제외.
- test result create: `case_id, sequence_number, test_date(+null flavor), test_name, test_meddra_version/code, test_result_code/value/qualifier/unit, result_unstructured, normal_low_value, normal_high_value, comments, more_info_available`; update는 case_id/sequence 제외.
- 공통 CRUD handler 소스상 status는 create 201, list/get/update 200, soft delete 204, restore 200이며 assertion 없는 verb에는 runtime 검증을 주장하지 않는다.
- reaction은 create/update와 목록 GET, test-result는 create/update에 활성 행동 assertion이 있다. 두 리소스의 item GET/delete/restore와 test-result 목록은 다른 테스트의 setup helper 호출을 coverage로 세지 않아 registration_only다.

근거:

- [tests/api_cases/subresources/reaction.rs::test_reaction_supports_ae_common_and_mfds_device_fields](../api_cases/subresources/reaction.rs)
- [tests/api_cases/subresources/test_result.rs::test_test_results_support_50_character_normal_ranges](../api_cases/subresources/test_result.rs)
- [tests/api_editor/pages/lb.generated.rs::list_projection_includes_displayed_result_columns](../api_editor/pages/lb.generated.rs)
- [tests/api_submission/validation.rs::test_unavailable_meddra_allows_save_and_export_but_blocks_submission](../api_submission/validation.rs)

<a id="patient"></a>
## Patient와 하위 리소스

| 메서드 | 경로 |
|---|---|
| `GET`, `POST`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/death-info` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}` |
| `POST` | `/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}` |
| `POST` | `/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{id}/restore` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/death-info/{id}` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/identifiers` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/identifiers/{id}` |
| `POST` | `/api/cases/{case_id}/patient/identifiers/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/medical-history` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/medical-history/{id}` |
| `POST` | `/api/cases/{case_id}/patient/medical-history/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}` |
| `POST` | `/api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}` |
| `POST` | `/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/parents` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/parents/{id}` |
| `POST` | `/api/cases/{case_id}/patient/parents/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/patient/past-drugs` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/patient/past-drugs/{id}` |
| `POST` | `/api/cases/{case_id}/patient/past-drugs/{id}/restore` |

- patient: `case_id`(create만), initials/birth date/age/gestation/age group/weight/height/sex/race/ethnicity/LMP/medical-history 각각 값과 해당 null flavor, `concomitant_therapy`. POST는 singleton이라 기존 행이 있으면 200으로 기존 값을 유지하고, 신규는 201. GET/PUT 200, DELETE 204.
- identifier: create `patient_id, sequence_number, identifier_type_code, identifier_value(+null_flavor)`; update는 type/value/null flavor. delete는 다른 child와 달리 200 entity 응답.
- medical history: create `patient_id, sequence_number, meddra_code, start_date_null_flavor, continuing_null_flavor, end_date_null_flavor`; update `meddra_version/code, start_date(+null_flavor), continuing(+null_flavor), end_date(+null_flavor), comments, family_history`.
- past drug: `patient_id, sequence_number, drug_name(+null_flavor), mfds_medicinal_product_version/id, mpid/version, phpid/version, start/end_date(+null flavor), indication/reaction MedDRA version/code`. source code-system 필드는 server 관리라 입력에서 무시된다.
- death info: `patient_id, date_of_death(+null_flavor), autopsy_performed(+null_flavor)`. reported/autopsy cause: `death_info_id, sequence_number, meddra_version/code, comments`.
- parent: `patient_id, parent_identification(+null_flavor), parent_birth_date(+null_flavor), parent_age/unit, LMP(+null_flavor), weight_kg, height_cm, sex(+null_flavor), medical_history_text`. parent medical history는 patient medical history와 같은 update 필드; parent past drug는 `parent_id, sequence_number, drug_name, mpid/version, mfds_medicinal_product_version/id, phpid/version, dates/null flavors, indication/reaction MedDRA version/code`.
- parent medical-history/past-drugs와 death-info reported/autopsy causes는 create/list/get/update/delete/restore, wrong case/parent 404, viewer 변경 요청 403, soft-delete 가시성까지 한 활성 table-driven test에서 assertion 되어 있다. 다른 patient item family는 operation별 TSV 상태를 따른다.

근거:

- [tests/api_cases/subresources/patient.rs::nested_patient_children_preserve_crud_scope_and_restore_contracts](../api_cases/subresources/patient.rs)
- [tests/api_cases/subresources/patient.rs::parent_information_preserves_dob_null_flavor_and_decade_unit](../api_cases/subresources/patient.rs)
- [tests/api_cases/subresources/patient.rs::patient_identifier_preserves_scope_and_delete_response](../api_cases/subresources/patient.rs)
- [tests/api_cases/subresources/patient.rs::patient_physical_fields_preserve_values_and_ucum_units](../api_cases/subresources/patient.rs)
- [tests/api_cases/subresources/patient.rs::test_audit_reason_header_records_normal_patient_update_reason](../api_cases/subresources/patient.rs)
- [tests/api_cases/subresources/patient.rs::test_patient_subresources_endpoints_ok](../api_cases/subresources/patient.rs)
- [tests/api_cases/subresources/singleton.rs::test_singleton_post_endpoints_are_idempotent](../api_cases/subresources/singleton.rs)
- [tests/api_submission/validation.rs::test_submission_rejects_case_validation_before_routing_or_dispatch](../api_submission/validation.rs)

<a id="narrative"></a>
## Narrative

| 메서드 | 경로 |
|---|---|
| `GET`, `POST`, `PUT`, `DELETE` | `/api/cases/{case_id}/narrative` |
| `POST` | `/api/cases/{case_id}/narrative/preview` |
| `GET`, `POST` | `/api/cases/{case_id}/narrative/sender-diagnoses` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/narrative/sender-diagnoses/{id}` |
| `POST` | `/api/cases/{case_id}/narrative/sender-diagnoses/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/narrative/summaries` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/narrative/summaries/{id}` |
| `POST` | `/api/cases/{case_id}/narrative/summaries/{id}/restore` |

- narrative singleton: create `case_id, source_narrative_presave_id, case_narrative, reporter_comments, sender_comments, additional_information`; update는 case_id 제외. 신규 POST 201, 기존 POST 200 무변경, GET/PUT 200, DELETE 204.
- preview: body `{template}`; `200 {"data":{"rendered":"...","tokens":[...]}}`. 알려진 patient token을 치환하고 unknown token은 유지한다.
- sender diagnosis: create `narrative_id, sequence_number, diagnosis_meddra_version/code`; update는 version/code. summary: create `narrative_id, sequence_number, language_code, summary_text`; update는 language/text.
- child handler는 소스상 공통 CRUD/restore status를 쓴다. sender diagnosis create/list/delete/restore와 summary create/list가 assertion 되었고 item GET/PUT 및 summary delete/restore는 registration_only다.

근거:

- [tests/api_cases/contract/audit.rs::long_hangul_narrative_update_preserves_jsonb_and_audit_contracts](../api_cases/contract/audit.rs)
- [tests/api_cases/subresources/narrative.rs::test_narrative_child_lists_return_empty_when_parent_narrative_missing](../api_cases/subresources/narrative.rs)
- [tests/api_cases/subresources/narrative.rs::test_narrative_subresources_endpoints_ok](../api_cases/subresources/narrative.rs)
- [tests/api_cases/subresources/narrative.rs::test_sender_diagnosis_soft_delete_restore](../api_cases/subresources/narrative.rs)
- [tests/api_cases/subresources/narrative.rs::test_narrative_preview_resolves_patient_tokens](../api_cases/subresources/narrative.rs)
- [tests/api_cases/subresources/singleton.rs::test_singleton_post_endpoints_are_idempotent](../api_cases/subresources/singleton.rs)

<a id="safety-report"></a>
## Safety report

| 메서드 | 경로 |
|---|---|
| `GET`, `POST`, `PUT`, `DELETE` | `/api/cases/{case_id}/safety-report` |
| `GET`, `POST` | `/api/cases/{case_id}/safety-report/documents` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/safety-report/documents/{id}` |
| `POST` | `/api/cases/{case_id}/safety-report/documents/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/safety-report/literature` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/safety-report/literature/{id}` |
| `POST` | `/api/cases/{case_id}/safety-report/literature/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/safety-report/primary-sources` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/safety-report/primary-sources/{id}` |
| `POST` | `/api/cases/{case_id}/safety-report/primary-sources/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/safety-report/senders` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/safety-report/senders/{id}` |
| `GET`, `POST` | `/api/cases/{case_id}/safety-report/studies` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/safety-report/studies/{id}` |
| `GET`, `POST` | `/api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}` |
| `POST` | `/api/cases/{case_id}/safety-report/studies/{study_id}/fda-cross-reported-inds/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}` |
| `POST` | `/api/cases/{case_id}/safety-report/studies/{study_id}/registrations/{id}/restore` |

- safety-report singleton: create `case_id, safety_report_id, version, transmission_date, report_type, date_first_received_from_source, date_of_most_recent_information, fulfil_expedited_criteria(+null_flavor), local_criteria_report_type, combination_product_report_indicator(+null_flavor), first_sender_type, additional_documents_available, other_case_identifiers_exist(+null_flavor), worldwide_unique_id, nullification_code/reason, receiver_organization`; update는 case_id 제외. POST는 신규/기존 모두 201 entity, GET/PUT 200, DELETE 204.
- sender: `case_id, source_sender_presave_id, sender_type, health_professional_type_kr1, organization_name, department, street_address, city, state, postcode, country_code, person title/name parts(+null flavors), telephone, fax, email`.
- primary source: `case_id, source_reporter_presave_id, sequence_number`, reporter name/address/contact 각 값과 null flavor, `country_code, email(+null flavor), qualification(+null flavor), qualification_kr1, primary_source_regulatory`.
- literature: `case_id, reference_text(+null_flavor), sequence_number, document_base64, file_name, media_type, representation, compression`. documents: `case_id, title, document_base64, file_name, media_type, representation, compression, sequence_number`.
- study: `case_id, source_study_presave_id, study_name(+null_flavor), sponsor_study_number(+null_flavor), study_type_reaction, study_type_reaction_kr1, fda_ind_number_occurred, fda_pre_anda_number_occurred`.
- registration: `study_information_id, registration_number(+null_flavor), country_code(+null_flavor), sequence_number`. FDA cross-reported IND: `study_information_id, ind_number(+null_flavor), sequence_number`.
- 하위 collection/item handler는 소스상 create 201/list-get-update 200/delete 204/restore 200이다. 활성 assertion은 singleton create/get/update, sender create/get/update, primary-source create/list/update, literature/study/registration/IND create에 집중되어 있으며 나머지 status는 runtime 검증을 주장하지 않는다.

근거:

- [tests/api_cases/subresources/safety_report.rs::safety_report_preserves_c1_boolean_null_flavors](../api_cases/subresources/safety_report.rs)
- [tests/api_cases/subresources/primary_source.rs::test_primary_source_supports_regional_rp_fields](../api_cases/subresources/primary_source.rs)
- [tests/api_cases/subresources/safety_report.rs::test_safety_report_subresources_endpoints_ok](../api_cases/subresources/safety_report.rs)
- [tests/api_cases/subresources/sender.rs::test_sender_information_supports_mfds_health_professional_type_kr1](../api_cases/subresources/sender.rs)
- [tests/api_cases/subresources/study.rs::test_study_create_accepts_study_type_reaction](../api_cases/subresources/study.rs)
- [tests/api_cases/subresources/singleton.rs::test_singleton_post_endpoints_are_idempotent](../api_cases/subresources/singleton.rs)

<a id="identifiers-source-documents"></a>
## Identifier, linked report, source document

| 메서드 | 경로 |
|---|---|
| `GET`, `POST` | `/api/cases/{case_id}/linked-reports` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/linked-reports/{id}` |
| `POST` | `/api/cases/{case_id}/linked-reports/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/other-identifiers` |
| `GET`, `PUT`, `DELETE` | `/api/cases/{case_id}/other-identifiers/{id}` |
| `POST` | `/api/cases/{case_id}/other-identifiers/{id}/restore` |
| `GET`, `POST` | `/api/cases/{case_id}/source-documents` |
| `PUT`, `DELETE` | `/api/cases/{case_id}/source-documents/{id}` |

- other identifier: create `case_id, sequence_number, source_of_identifier, case_identifier`; update는 source/identifier. 공통 CRUD/restore이며 create/list/delete/restore가 assertion 되어 있다.
- linked report: create `case_id, sequence_number, linked_report_number`; update는 linked number. 모든 operation은 registration_only다.
- source document: create `case_id, source_document_name, source_document_base64, source_document_media_type, sequence_number`; update는 case_id 제외. 등록된 operation은 list/create/update/delete이며 모두 registration_only다.
- receiver singleton: `case_id, receiver_type, organization_name, department, street_address, city, state_province, postcode, country_code, telephone, fax, email`. 이 리소스의 route는 core 표에 배치되며 POST/GET만 assertion, PUT/DELETE는 registration_only다.

근거:

- [tests/api_cases/subresources/safety_report.rs::test_other_case_identifier_soft_delete_restore](../api_cases/subresources/safety_report.rs)

## Coverage gap 우선순위

1. registration_only 122개 중 drug device/code/assessment, patient의 non-nested item, safety-report documents/literature/study item, linked-report/source-document의 mutation이 가장 넓은 공백이다. 성공 status와 body뿐 아니라 wrong parent/case 404, viewer 403, soft-delete/restore, audit reason을 묶어 검증해야 한다.
2. `GET /case-query/catalog`, `POST /case-query/search`, `GET /cases/link-options`는 등록 이외 행동 assertion이 전혀 없다.
3. `GET /cases/{case_id}/versions`는 조직 가시성만 검증되어 반환 필드, 정렬, 빈 결과가 미검증이다.
4. setup helper가 201/200을 내부 확인하는 경우도 대상 API의 반환 계약이나 상태 변화를 테스트 목적에서 assertion하지 않으면 coverage로 올리지 않았다.
