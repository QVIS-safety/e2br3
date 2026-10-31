# Presave API 계약 명세

> **NOT_RUN:** 사용자 지시에 따라 테스트·빌드·API·DB 호출을 실행하지 않았다. 아래 판정은 source와 활성 test body 정적 검토 결과다.

이 문서는 현재 등록된 `/api/presaves/**` 73개 method/path의 요청, 응답, 상태 변경, 권한, 오류와 활성 테스트 연결을 기술한다. 실행 결과나 규제 적합성 증명이 아니다. 등록 원본은 [`routes/presaves.rs`](../../src/web/rest/routes/presaves.rs), handler/DTO 원본은 [`section_presave_rest`](../../src/web/rest/section_presave_rest.rs)다.

<a id="common-contract"></a>

## 공통 계약

모든 경로는 인증 쿠키와 INFO presave 권한을 요구한다. collection/read/update wrapper가 조직과 sender/product/study scope를 적용한다. 권한이 없으면 403, scope 밖 또는 존재하지 않는 UUID는 정책·모델 경로에 따라 403/404가 된다. JSON 역직렬화 실패는 `422`, `INPUT.JSON.INVALID`로 변환한다. 알 수 없는 키의 거절은 `deny_unknown_fields`가 있는 parent/graph DTO에 적용되며, 이 선언이 없는 직접 child DTO까지 일반화하지 않는다. 필드 계약 위반은 보통 422 constraint 응답, 필수값·부모 소속·삭제 조합 오류는 400, 활성 identity 중복과 참조 중인 부모 삭제는 409다.

parent 단일 행과 graph 생성 본문은 `{"data":{"rows":{...}}}`다. PATCH의 부모 DTO는 `{"data":{...}}`인 sender/receiver/product/study와 `{"data":{"rows":{"reporter|narrative":{...}}}}`인 macro 기반 reporter/narrative로 나뉜다. 직접 child CRUD의 JSON 본문은 `{"data":{...}}`다. graph PUT은 `{"data":{"rows":{...}}}`이고, 포함한 row 집합만 처리한다. 새 자식은 `id` 없이 `sequenceNumber`와 업무 필드를 보내며 기존 자식은 `id`를 보낸다. `deleted:true`는 명시적 삭제이고 빈 배열은 기존 자식 전체 삭제가 아니다.

성공 응답은 `{"data":...}`다. 생성은 201, 조회·수정은 200, DELETE는 본문 없는 204다. parent list는 배열이며 sender list만 details graph를 반환한다. receiver/product/study list는 parent row, reporter/narrative는 `rows` wrapper를 반환한다. details graph는 camelCase row를 반환한다.

부모6종의 HTTP DELETE는 `PresaveLifecycleService::archive`를 호출해 `UPDATE deleted=true`를 수행한다. 내부 BMC의 `delete`와 lifecycle `hard_delete`는 SQL DELETE 경로이므로 HTTP 계약의 근거로 혼용하지 않는다. 자식은 soft 6종(sender gateway/responsible person·study product/registration/reporter/FDA IND), hard 3종(product active substance·receiver consignee/route)으로 구분한다.

<a id="sender"></a>

## Sender

| Method/path | 정상 계약 | 활성 assertion |
|---|---|---|
| `GET /api/presaves/senders` | scope 내 sender graph 목록 | 목록과 responsible person 포함 |
| `POST /api/presaves/senders` | `rows.sender`, 선택 `gateways[]`, `responsiblePersons[]`; 201 graph | canonical camelCase, 중복 409, 자식 원자 생성 |
| `GET /api/presaves/senders/{id}` | parent row | archive/restore 뒤 동일 ID와 deleted 상태 |
| `PATCH /api/presaves/senders/{id}` | parent 일부 변경 또는 `deleted:true` archive | 누락 보존, blank/null clear, `deleted:false` 동일 ID 반복 restore |
| `DELETE /api/presaves/senders/{id}` | 참조 없을 때 archive, 204 | 반복 DELETE 204, deleted readback, 참조 중 409와 상태 불변 |
| `GET`, `PUT /api/presaves/senders/{id}/details` | `rows.sender`, `gateways[]`, `responsiblePersons[]`의 원자 graph 조회/변경 | load/save, rollback, 명시 삭제/복원, 잘못된 자식 ID |
| `GET`, `POST /api/presaves/senders/{sender_id}/gateways` | child 목록 / snake_case 단건 생성 | 직접 목록의 soft-delete/restore 동일 ID·count와 POST assertion |
| `GET`, `PATCH`, `DELETE /api/presaves/senders/{sender_id}/gateways/{id}` | child 조회/변경/soft delete | 세 method 직접 assertion |
| `GET`, `POST /api/presaves/senders/{sender_id}/responsible-persons` | child 목록 / snake_case 단건 생성 | 직접 목록의 soft-delete/restore 동일 ID·count와 POST assertion |
| `GET`, `PATCH`, `DELETE /api/presaves/senders/{sender_id}/responsible-persons/{id}` | child 조회/변경/soft delete | 반복 DELETE, 명시 restore, 값 round-trip, wrong-parent 404 |

대표 생성 키는 `senderType`, `organizationName`, `organizationNameNotation`, `streetAddress`, `city`, `state`, `postcode`, `countryCode`, `telephone`, `fax`, `email`, `isDefault`다. gateway 직접 CRUD의 요청·응답 키는 snake_case이며 생성은 `sequence_number`, `gateway_authority`, `sender_identifier`, `routing_identifier`, `cde_sender_identifier`, `cdr_sender_identifier`, `is_default_for_authority`를 받는다. responsible-person 직접 생성도 snake_case `sequence_number`, `department`, `person_title`, `person_given_name`, `person_middle_name`, `person_family_name`, `is_default`를 받으며 PATCH는 이 필드들과 `deleted`를 받는다. direct child 응답도 `sender_presave_id`, `created_at` 등을 포함한 snake_case다. 반면 sender create/details graph의 `gateways[]`, `responsiblePersons[]`는 같은 필드를 `sequenceNumber`, `gatewayAuthority`, `personTitle`, `personGivenName`처럼 camelCase로 받거나 반환한다. parent에는 사람/부서 필드가 노출되지 않는다.

<a id="receiver"></a>

## Receiver

| Method/path | 정상 계약 | 활성 assertion |
|---|---|---|
| `GET /api/presaves/receivers` | scope 내 receiver parent row 목록 | 목록 포함 |
| `POST /api/presaves/receivers` | `rows.receiver`, 선택 `consignees[]`, `routes[]`; 201 graph | 필수값·허용 receiver type·timeline·중복 오류 |
| `GET /api/presaves/receivers/{id}` | parent row | archive/restore 뒤 동일 ID와 deleted 상태 |
| `PATCH /api/presaves/receivers/{id}` | parent 일부 변경/archive | null clear, `deleted:false` 동일 ID 반복 restore |
| `DELETE /api/presaves/receivers/{id}` | 참조 없을 때 archive | 반복 DELETE 204와 deleted readback |
| `GET`, `PUT /api/presaves/receivers/{id}/details` | `rows.receiver`, `consignees[]`, `routes[]` 원자 graph | camelCase, load/save/no-op, hard delete old-ID replay 404·전체 rollback·새 ID/count |
| `GET`, `POST /api/presaves/receivers/{receiver_id}/consignees` | child 목록/생성 | replacement 새 ID와 정확한 count |
| `GET`, `PATCH`, `DELETE /api/presaves/receivers/{receiver_id}/consignees/{id}` | child 조회/변경/hard delete | PATCH 값/감사, DELETE 204, GET·재DELETE 404, wrong-parent 404 |

receiver 대표 키는 `receiverType`(`Regulatory Authority` 또는 `Original Manufacturer`), `organizationName`, `description`, 기관별 timeline day count/not-applicable 쌍이다. consignee는 `sequence_number`, `name`, `email`, `phone`을 받는다. details source에는 route rows도 있지만 별도 route CRUD는 등록되지 않았다. route는 details PUT으로 hard delete되며 old ID replay는 404와 전체 graph rollback, 재생성은 새 ID와 정확한 count를 직접 검사한다.

<a id="product"></a>

## Product

| Method/path | 정상 계약 | 활성 assertion |
|---|---|---|
| `GET /api/presaves/products` | scope와 선택 `senderIds=uuid,...` 필터 목록 | scope 필터와 목록 |
| `POST /api/presaves/products` | `rows.product`, 선택 `activeSubstances[]`; 201 graph | 원자 생성, sender/identity 필수, identity 중복 |
| `GET /api/presaves/products/{id}` | parent row | archive/restore 뒤 동일 ID와 deleted 상태 |
| `PATCH /api/presaves/products/{id}` | parent 일부 변경/archive | 저장·clear, legacy key 거절, `deleted:false` 동일 ID 반복 restore |
| `DELETE /api/presaves/products/{id}` | 참조 없을 때 archive | 반복 DELETE 204, deleted readback, 참조 중 409 뒤 상태 불변 |
| `GET`, `PUT /api/presaves/products/{id}/details` | `rows.product`, `activeSubstances[]` 원자 graph | save/no-op, hard delete old-ID replay 404·전체 rollback·새 ID/count |
| `GET`, `POST /api/presaves/products/{product_id}/active-substances` | child 목록/생성 | replacement 새 ID와 정확한 active count |
| `GET`, `PATCH`, `DELETE /api/presaves/products/{product_id}/active-substances/{id}` | child 조회/변경/hard delete | PATCH 값/감사, DELETE 204, GET·재DELETE 404, wrong-parent 404 |

product create/PATCH와 graph의 canonical 요청 키는 `senderPresaveId`, `receiverPresaveId`, `productId`, `medicinalProduct`, `medicinalProductNotation`, `preApprovalIpName`, `drugBrandName`, `originalManufacturer`, `productDescription`, `mpid`, `mpidVersion`, `mfdsMpid`, `mfdsMpidVersion`, `phpid`, `phpidVersion`, `investigationalProductBlinded`, `obtainDrugCountry`, `drugAuthorizationNumber`, `drugAuthorizationCountry`, `drugAuthorizationHolder`, `holderApplicantNameNotation`다. identity에는 `productId` 또는 `preApprovalIpName` 중 하나가 필요하다. `brandName`은 canonical key가 아니며 unknown field로 거절된다. details graph 응답은 모두 camelCase다. 반면 direct product list/get/PATCH 응답은 model serialization을 그대로 써서 대다수 키가 snake_case이고 `preApprovalIpName`, `drugBrandName`, `mpidVersion`, `phpidVersion`만 명시적 serde 이름을 쓴다. substance graph 키는 `id`, `deleted`, `sequenceNumber`, `substanceName`, `substanceTermIdVersion`, `substanceTermId`, `mfdsVersion`, `mfdsId`, `substanceStrengthValue`, `substanceStrengthUnit`다. MFDS device rows와 과거 source 필드는 details 응답에 포함되지 않는다.

<a id="reporter-narrative"></a>

## Reporter와 Narrative

| Method/path | 정상 계약 | 활성 assertion |
|---|---|---|
| `GET`, `POST /api/presaves/reporters` | `rows.reporter` 목록/생성 | 두 method 직접 assertion |
| `GET`, `PATCH`, `DELETE /api/presaves/reporters/{id}` | wrapped row 조회/변경/archive | 반복 DELETE 204와 동일 ID 반복 restore/readback |
| `GET`, `POST /api/presaves/narratives` | `rows.narrative` 목록/생성 | 두 method 직접 assertion |
| `GET`, `PATCH`, `DELETE /api/presaves/narratives/{id}` | wrapped row 조회/변경/archive | 반복 DELETE 204와 동일 ID 반복 restore/readback |

reporter 대표 키는 `reporterTitle`, `reporterGivenName`, `reporterFamilyName`, 각 null-flavor, `organization`, `countryCode`, `qualification`, `qualificationKr1`, `primarySourceRegulatory`다. narrative는 `caseNarrative`, `caseNarrativeNotation`, `additionalInformation`, `templateTitle`을 사용한다. legacy snake_case unwrapped body는 422다. DELETE 뒤 단건과 목록은 `deleted:true` row를 계속 반환한다.

<a id="study"></a>

## Study

| Method/path | 정상 계약 | 활성 assertion |
|---|---|---|
| `GET /api/presaves/studies` | scope와 선택 `productIds=uuid,...` 필터 목록 | restore 뒤 동일 ID가 정확히 1개 |
| `POST /api/presaves/studies` | `rows.study`, `products[]`, `reporters[]`, `registrationNumbers[]`, `fdaCrossReportedInds[]`; 201 graph | 모든 canonical row 원자 생성 |
| `GET /api/presaves/studies/{id}` | parent row | archive/restore 뒤 동일 ID와 deleted 상태 |
| `PATCH /api/presaves/studies/{id}` | parent 일부 변경/archive | optional text clear, `deleted:false` 동일 ID 반복 restore |
| `DELETE /api/presaves/studies/{id}` | archive, 204 | 반복 DELETE 204와 deleted readback |
| `GET`, `PUT /api/presaves/studies/{id}/details` | 5개 row 집합 원자 조회/변경 | 네 child 종류의 명시 삭제·생략 시 삭제 유지·동일 ID 반복 restore/re-delete, invalid/wrong-parent 전체 graph rollback |
| `GET`, `POST /api/presaves/studies/{study_id}/products` 및 `/{id}`의 `GET`, `PATCH`, `DELETE` | study-product 단건 CRUD | 반복 soft delete, GET/list 동일 ID·count, 명시 restore, PATCH 감사, wrong-parent 404 |
| `GET`, `POST /api/presaves/studies/{study_id}/registration-numbers` 및 `/{id}`의 `GET`, `PATCH`, `DELETE` | registration 단건 CRUD | 반복 soft delete, GET/list 동일 ID·count, 명시 restore, PATCH 감사, wrong-parent 404 |
| `GET`, `POST /api/presaves/studies/{study_id}/reporters` 및 `/{id}`의 `GET`, `PATCH`, `DELETE` | study-reporter 단건 CRUD | 반복 soft delete, GET/list 동일 ID·count, 명시 restore, PATCH 감사, wrong-parent 404 |

study 대표 키는 `productPresaveId`, `studyName`, `studyNameNotation`, `sponsorStudyNumber`, `studyTypeReaction`이다. 자식은 공통으로 `id`, `deleted`, `sequenceNumber`를 사용하고 각각 `productPresaveId/productName`, `registrationNumber/countryCode`와 null-flavor, `reporterPresaveId/reporterOrganization/reporterGivenName/reporterQualification`을 더한다.

<a id="lifecycle-audit"></a>

## 상태와 감사

details handler는 transaction 안에서 parent/child preflight 후 일괄 적용한다. 잘못된 다른-parent ID, 새 row의 `deleted:true`, id 없는 delete, parent archive와 child 변경 동시 요청을 거절한다. Product active-substance, receiver consignee/route의 hard delete 뒤 old ID graph replay는 404이고 요청 전후 전체 details 객체가 같은지 확인한다. 새 행은 다른 ID로 생성되고 active count를 확인한다. Study product, registration number, reporter, FDA cross-reported IND는 `deleted:true` 뒤 일반 필드 업데이트에서 `deleted`를 생략해도 삭제 상태를 유지하고, 명시적 `deleted:false`를 두 번 보내도 같은 ID와 count로 복원되며, 다시 `deleted:true`로 삭제되는지 매 단계 GET으로 재조회한다. Study invalid/wrong-parent 여섯 요청도 전체 details 불변을 확인한다.

여섯 parent family는 DELETE를 두 번 보내 모두 204인지, GET의 동일 ID/deleted=true, `deleted:false` PATCH를 두 번 보내 매번 응답과 GET이 동일 ID/deleted=false인지, 목록에 그 ID가 정확히 하나인지 검사한다. parent DELETE와 PATCH `deleted:true`는 lifecycle archive를 사용한다. 참조 중 parent archive 409는 sender/product에서 확인하며 후속 GET이 동일 ID/deleted=false인지도 검사한다. 이 추가 검사는 runtime NOT_RUN이다.

여섯 soft parent와 soft direct child 다섯 종류(sender gateway/responsible person, study product/registration/reporter)는 DELETE를 두 번 보내 모두 204인지와 첫 요청만 `deleted` 업무 변경 UPDATE 감사를 하나 추가하는지 확인한다. 두 번째 no-op DELETE는 감사 trigger가 `updated_at/updated_by`만 다른 UPDATE를 제거하므로 새 감사 행을 만들지 않아야 한다. child는 이어 GET과 collection GET에서 삭제 행의 ID·업무값을 확인하고, `deleted:false` PATCH 두 번과 fresh GET/list가 같은 ID와 count를 보존하는지 검사한다. hard child인 product active substance와 receiver consignee/route는 물리 삭제, old-ID 재사용 404, 새 ID 생성 계약을 유지한다.

parent archive는 child를 cascade하지 않는다. [`lifecycle_matrix_web.rs::parent_archive_restore_preserves_hard_soft_and_graph_only_children`](../api_presave/lifecycle_matrix_web.rs)은 sender의 두 child, receiver의 두 child, product substance, study의 네 child 모두에 대해 parent archive/restore 전후 child ID와 row-key별 count를 확인한다. Reporter는 활성 study-reporter가 있으면 409이며 parent가 그대로 유지된다. child를 soft delete한 뒤 reporter를 archive/restore하고 같은 child ID를 복원하는 순서도 검사한다. Narrative는 child가 없어 parent-child 보존 축은 해당 없음이다.

직접 child 일곱 종류의 다른-parent URL은 GET/PATCH/DELETE 모두 404이고 owner 경로의 값은 바뀌지 않아야 한다. 별도 URL이 없는 receiver route와 study FDA IND는 다른-parent ID를 details PUT에 넣으면 400이고, 같은 요청에 포함한 parent 변경까지 rollback되어 양쪽 graph가 요청 전과 같아야 한다. 이 거절 matrix는 대상 child와 sentinel parent의 감사 행 수도 바꾸지 않는지 확인한다.

`x-e2br3-reason-for-change`와 `x-e2br3-change-category`가 audit context로 전달된다. [`audit_web.rs::presave_parent_and_child_http_updates_record_complete_audit_rows`](../api_presave/audit_web.rs)은 parent 여섯 종류와 child 아홉 종류를 실제 HTTP로 수정한 뒤 각 table/record의 `organization_id`, `user_id`, `action`, reason/category, `changed_fields.old/new`, `old_values/new_values`를 auditor role로 확인한다. 거절된 read-only/cross-org mutation은 owner graph와 이 15개 record의 audit count를 바꾸지 않아야 한다. 이 신규 검사는 모두 runtime NOT_RUN이다.

<a id="authorization"></a>

## 권한과 조직 경계

INFO read만 가진 같은 조직 manager는 15개 resource를 parent/direct/details 경로로 읽을 수 있지만 update, 13개 direct DELETE, soft-row의 명시적 `deleted:false`, graph delete/restore mutation은 403이다. case read만 가진 viewer는 INFO read가 없으므로 같은 조회와 mutation 모두 403이다. 다른 조직 sponsor admin은 자체 `info.read`/`info.edit` 권한이 있어도 typed isolation context의 platform bypass가 false여서 FORCE RLS가 owner presave fact를 숨기고, fact loader의 not-found가 404로 매핑된다. `SameOrganization`은 RLS를 통과해 fact가 로드된 경우의 추가 조건이며 이 sponsor-admin 경로의 직접 오류 원인은 아니다. [`authorization_web.rs::presave_read_only_and_cross_org_roles_cannot_mutate_any_resource`](../api_presave/authorization_web.rs)은 graph-only route/FDA ID까지 읽기 응답 안에서 확인하고, 각 mutation 뒤 parent 네 graph와 reporter/narrative fresh GET, 15개 record 감사 count가 같은지 검사한다. Receiver route는 hard-delete row라 기존 ID의 restore가 계약상 해당 없으며, `deleted:false`는 restore로 분류하지 않는다.

Presave authorization context에는 case ID, case lifecycle 또는 lock fact가 없고 `info.read`/`info.update`의 조직·principal scope만 평가한다. 따라서 Case lock 축은 Presave 자원에 해당 없음이다. Presave에 별도 세부 역할 정책을 새로 가정하지 않는다.

## 테스트 연결과 빈틈

Coverage 판정에서는 fixture 생성을 위한 요청만으로 endpoint를 asserted 처리하지 않았다. 직접 근거의 시작점은 다음과 같다.

- 공통 CRUD/blank/lifecycle: [`crud_matrix_web.rs::test_section_presave_sender_receiver_product_reporter_rest_contract`](../api_presave/crud_matrix_web.rs), [`crud_matrix_web.rs::presave_optional_text_blank_null_and_omitted_contract`](../api_presave/crud_matrix_web.rs), [`crud_matrix_web.rs::presave_parent_delete_is_repeatable_and_patch_restores_the_same_row`](../api_presave/crud_matrix_web.rs)
- graph/원자성: [`sender_web.rs::test_sender_presave_details_graph_load_and_save`](../api_presave/sender_web.rs), [`receiver_web.rs::test_receiver_presave_details_graph_load_save_noop_delete_and_invalid`](../api_presave/receiver_web.rs), [`receiver_web.rs::section_presave_receiver_details_contract_includes_routes`](../api_presave/receiver_web.rs), [`product_web.rs::test_product_presave_details_noop_delete_and_invalid_child_operations`](../api_presave/product_web.rs), [`study_web.rs::test_study_presave_details_requires_explicit_child_delete`](../api_presave/study_web.rs), [`study_web.rs::test_study_presave_details_rejects_invalid_child_operations`](../api_presave/study_web.rs)
- 삭제 제약: [`delete_constraints_web.rs::test_presave_rest_rejects_deleting_referenced_parent`](../api_presave/delete_constraints_web.rs#L10)
- direct/graph lifecycle과 parent scope: [`lifecycle_matrix_web.rs::parent_repeat_delete_emits_one_business_update_audit`](../api_presave/lifecycle_matrix_web.rs), [`lifecycle_matrix_web.rs::direct_soft_children_repeat_delete_restore_same_id_and_noop_audit`](../api_presave/lifecycle_matrix_web.rs), [`lifecycle_matrix_web.rs::parent_archive_restore_preserves_hard_soft_and_graph_only_children`](../api_presave/lifecycle_matrix_web.rs), [`lifecycle_matrix_web.rs::direct_and_graph_children_reject_wrong_parent_without_state_change`](../api_presave/lifecycle_matrix_web.rs)
- 권한·조직·거절 무감사: [`authorization_web.rs::presave_read_only_and_cross_org_roles_cannot_mutate_any_resource`](../api_presave/authorization_web.rs)
- 15 table 감사: [`audit_web.rs::presave_parent_and_child_http_updates_record_complete_audit_rows`](../api_presave/audit_web.rs)

현재 [coverage.tsv](coverage.tsv)는 73개 중 70개에 동작 assertion, 3개에 등록 검사만 연결한다. Study direct child products/registration-numbers/reporters의 POST는 준비용 생성 호출만 있어 `registration_only`를 유지한다. 새 assertion을 모든 생성 계약의 증거로 확대하지 않는다. 남은 공백은 parent/child 동시 경쟁, pagination, 제한 scope가 비어 있지 않은 sender/product/study 조합, 감사 hash-chain 자체 무결성, 각 constraint 오류의 모든 nullable 조합이다. HTTP 상태·응답·DB 상태·감사를 확인하는 신규 검사는 실행하지 않았으므로 통과했다고 주장하지 않는다. 전체 요청·응답 Rust 필드와 serde 속성은 [operation/DTO manifest](api-contract-manifest.json)에서 해당 method/path와 타입을 연결한다.

## Legacy 미등록 404

`GET /api/presave-templates`, `GET /api/presave-templates/{id}`, `GET /api/presave-templates/{id}/audit`, `GET /api/presaves/products/{id}/substances`는 등록 경로가 아니다. [`legacy_routes_web.rs::test_legacy_presave_templates_route_is_removed`](../api_presave/legacy_routes_web.rs#L8)이 404를 확인한다. 이 네 경로는 등록 coverage TSV 행에 넣지 않는다.
