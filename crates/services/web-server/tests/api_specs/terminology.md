# Terminology API 계약 명세

> **NOT_RUN:** 사용자 지시에 따라 테스트·빌드·API·DB 호출을 실행하지 않았다. 아래 판정은 source와 활성 test body 정적 검토 결과다.

이 문서는 현재 등록된 `/api/terminology/**` 15개 method/path의 검색, import, release lifecycle과 권한을 기술한다. 실행 결과나 특정 release의 운영 활성·규제 적합성 증명이 아니다. HTTP 원본은 [`terminology_rest.rs`](../../src/web/rest/terminology_rest.rs), 저장/lifecycle 원본은 [`terminology_import.rs`](../../../../libs/lib-core/src/model/terminology_import.rs)다.

<a id="search-read"></a>

## Search/read

모든 GET은 인증과 terminology read 정책을 요구하고 `{"data":[...]}`를 200 반환한다.

| Method/path | query와 응답 row 핵심 키 | 활성 assertion |
|---|---|---|
| `GET /api/terminology/meddra` | 필수 `q`, 선택 `limit`(기본 20), `version`, `language`, `level`; `id,code,term,level,version,language,active,created_at` | active release만, code/name, version/language 전환, viewer/operational read |
| `GET /api/terminology/whodrug` | 필수 `q`, 선택 `limit`; `id,code,drug_name,atc_code,version,language,active,created_at` | active row 검색 |
| `GET /api/terminology/mfds-products` | 필수 `q`, 선택 `limit`; `id,item_seq,product_name_kr/en,manufacturer_name_kr/en,permit_date,cancellation_date,cancellation_status,version,active,created_at` | code/name와 inactive 제외 |
| `GET /api/terminology/mfds-substances` | `q` 두 글자 이상, 선택 `limit`; `substance_code,substance_name_kr/en,version` | 보호 snapshot만; 2자 오류/정상 결과 직접 assertion 없음 |
| `GET /api/terminology/mfds-products/{item_seq}/substances` | path의 품목기준코드; `item_seq,substance_code,substance_name_kr/en,quantity,unit,component_content,version` | active explicit link와 주요 값 |
| `GET /api/terminology/countries` | query 없음; `code,name,active` | viewer 200와 KR 포함 |
| `GET /api/terminology/code-lists` | 필수 `list_name`; `id,list_name,code,display_name,description,sort_order,active` | `sender_type` active code 순서 |
| `GET /api/terminology/fda-code-search` | 필수 `list_name,q`, 선택 `limit`; 2자 미만은 빈 배열; `id,list_name,ncit_concept_code,level1_term,level2_term,level3_term,fda_code,imdrf_code` | 보호 snapshot만 |
| `GET /api/terminology/ucum-units` | query 없음; `id,code,display_name,description,unit_type,active` | 보호 snapshot만 |
| `GET /api/terminology/releases` | 선택 `dictionary`, `language`; release row 배열 | filter/list와 operational read |

검색 row는 DB model의 snake_case serialization을 사용한다. `whodrug`, `mfds-products`, `mfds-substances`도 공용 query DTO 때문에 `version`, `language`, `level`을 역직렬화하지만 현재 handler는 이 세 값을 사용하지 않는다. limit의 handler 기본은 20이며 MedDRA model은 1..100으로 clamp한다. 다른 검색의 음수/과대 limit 경계는 이 문서가 보장하지 않으며 활성 assertion도 없다.

직접 근거는 [`search.rs::test_sponsor_admin_can_search_active_meddra_and_whodrug_terms`](../api_terminology/search.rs#L128), [`test_sponsor_admin_searches_only_active_mfds_products_by_code_or_name`](../api_terminology/search.rs#L94), [`test_mfds_product_substances_returns_active_explicit_link`](../api_terminology/search.rs#L42), [`test_sponsor_admin_receives_active_sender_type_code_list`](../api_terminology/search.rs#L64), [`permissions.rs::test_operational_user_can_list_and_search_active_meddra`](../api_terminology/permissions.rs#L55)다.

<a id="imports"></a>

## Imports

`POST /api/terminology/import/meddra?version={version}&language={language}&dry_run={bool}`와 `POST /api/terminology/import/whodrug?...`는 terminology mutation 권한과 `multipart/form-data`의 `file` part를 요구한다. `version`은 필수, `language` 기본은 `en`, `dry_run` 기본은 `false`다.

다음 구조에서 `123`은 설명용 row count 정수다.

```json
{"data":{
  "dictionary":"meddra|whodrug", "version":"<release-version>", "language":"en",
  "loaded_rows":123, "dry_run":false, "status":"validated|dry_run_validated"
}}
```

MedDRA는 ZIP의 `llt.asc`와 `mdhier.asc`를 parse한다. multipart reader 자체의 file-part cap은 250 MiB지만 MedDRA route에는 `DefaultBodyLimit` override가 없어 현재 pin된 Axum 기본 2 MiB가 multipart request 전체에 먼저 적용된다. 따라서 250 MiB는 reader 방어 한도이며 현재 route의 실효 request 상한이 아니다.

WHODrug route는 multipart request 전체를 512 MiB + 1 MiB로 제한하고, `file` part를 mode 0600 임시 파일에 최대 512 MiB까지 spool하며 SHA-256을 계산한다. official C3 ZIP은 streaming stage 경로를 사용한다. 그 외 CSV/ZIP은 spool 뒤 다시 250 MiB file cap을 적용하고 메모리 parse한다. 일반 ZIP parser는 archive 전체 256 entries, 선택 entry당 250 MiB, 선택 entry 압축 해제 합계 512 MiB를 제한한다. official C3 streaming parser의 내부 상한은 entry당 1 GiB, 압축 해제 합계 2 GiB지만 앞단의 512 MiB spool과 513 MiB request limit도 함께 적용된다. dry run은 parse/count만 하고 stage하지 않는다. 성공/오류 뒤 임시 파일 cleanup을 시도한다.

활성 테스트는 MedDRA/WHODrug dry run, 잘못된 ZIP/필수 열, alternate header/zip delimited, C3 batch와 late invalid rollback, 실제 stage의 count/checksum/상태 보존, 같은 version 재import idempotency, viewer 403을 확인한다. 시작점은 [`imports.rs::test_admin_can_dry_run_meddra_import`](../api_terminology/imports.rs#L5), [`test_admin_c3_dry_run_streams_batches_and_rejects_late_invalid_data`](../api_terminology/imports.rs#L185), [`test_admin_c3_stage_persists_thirteen_character_short_once`](../api_terminology/imports.rs#L269), [`permissions.rs::test_viewer_can_read_terminology_but_cannot_import`](../api_terminology/permissions.rs#L5)다.

missing file, request/body 정확한 boundary, spool write/flush/cleanup 실패, language/version 빈 문자열, MedDRA non-dry-run 응답 전체, 대용량 official C3의 실제 production release는 활성 assertion으로 완전히 덮이지 않는다.

<a id="release-lifecycle"></a>

## Release lifecycle

release row는 `id,dictionary,version,language,status,source_path,source_checksum,loaded_rows,approved_by,approved_at,activated_by,activated_at,rollback_from_version,note,created_at,updated_at`를 반환한다.

| Method/path | 요청 | 상태 변경과 오류 |
|---|---|---|
| `POST /api/terminology/releases/{dictionary}/{version}/approve` | 선택 `language`(기본 `en`), `note` query | approvable release를 approved로 변경; 잘못된 dictionary/대상은 400 |
| `POST /api/terminology/releases/{dictionary}/{version}/activate` | 선택 `language`(기본 `en`) | 해당 dictionary/version/language 활성화, 기존 active 전환 |
| `POST /api/terminology/releases/{dictionary}/{version}/rollback` | 선택 `language`(기본 `en`) | target을 다시 active로 하고 `rollback_from_version` 기록 |

허용 dictionary는 `meddra`, `whodrug`, `iso3166`, `ich_constrained_ucum`, `edqm`, `mfds_product`다. mutation은 user ID를 approval/activation metadata에 기록한다. MedDRA/WHODrug 언어별 activation과 검색 가시성, MFDS product activation/rollback, 잘못된 dictionary/없는 target 400, viewer의 세 action 403, rollback metadata는 [`releases.rs::test_admin_can_meddra_approve_activate_and_rollback_release`](../api_terminology/releases.rs#L151), [`test_language_specific_activation_and_search_switching`](../api_terminology/releases.rs#L446), [`test_mfds_product_activation_and_rollback_change_visible_search_release`](../api_terminology/releases.rs#L5), [`permissions.rs::test_viewer_cannot_approve_activate_or_rollback_release`](../api_terminology/permissions.rs#L91)에서 확인한다.

일반 `audit_logs` row의 old/new/user/org를 HTTP 테스트가 직접 확인하지는 않는다. release metadata와 lifecycle 상태가 현재 직접 확인되는 감사성 기록이며, concurrent activate/rollback, action 재시도, note round-trip, 승인자/활성자 ID와 시각 전체는 활성 assertion이 부족하다.

## Coverage 판정

Route별로는 15개 중 12개가 직접 behavior/permission assertion을 가지며 `fda-code-search`, `mfds-substances`, `ucum-units` 세 경로는 `registration_only`다. 보호 경로 snapshot은 HTTP authorization assertion이 아니라 등록 inventory 근거다. 공통 근거는 [`protected_route_inventory.rs::protected_inventory_matches_checked_in_snapshot`](../authz/protected_route_inventory.rs#L46)이다.

검색 응답 전체 schema, SQL 오류 mapping, cross-org release visibility, exact pagination/order, GET query deserialization 오류, production dictionary content와 release row count는 이 source-only 검토에서 검증하지 않았다.
