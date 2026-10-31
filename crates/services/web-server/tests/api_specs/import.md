# XML Import API 계약 명세

> **NOT_RUN:** 사용자 지시에 따라 테스트·빌드·API·DB 호출을 실행하지 않았다. 아래 판정은 source와 활성 test body 정적 검토 결과다.

이 문서는 현재 등록된 `/api/import/xml*` 네 method/path를 기술한다. `/api/cases/{...}/import*`와 `import-from-presave` 등록은 현재 [`routes/cases.rs`](../../src/web/rest/routes/cases.rs), 보호 경로 snapshot, 추출된 route 목록 어디에도 없다. 실행 결과나 외부 기관 acceptance 증명이 아니다.

<a id="xml-import"></a>

## XML import

`POST /api/import/xml`은 인증된 import mutation 권한과 enforced product scope를 요구한다. `multipart/form-data`의 `file` 또는 `xml` part와 그 part의 파일명, `productPresaveId` 또는 snake_case alias `product_presave_id` UUID가 필수다. `product_id`/`productId`는 거절한다. 선택한 Product presave는 삭제되지 않았고 scope 안에 있으며 비어 있지 않은 `productId`가 있어야 한다.

단일 XML 또는 ZIP을 받는다. 한 `file`/`xml` part는 최대 50 MiB이고 라우터의 전체 multipart request limit은 50 MiB + 64 KiB다. 따라서 후자는 boundary와 다른 part까지 포함한다. ZIP은 XML 파일 하나당 압축 해제 25 MiB, 디렉터리와 비 XML 파일을 포함한 archive 전체 entry 128개, 선택된 XML entry의 압축 해제 합계 100 MiB로 제한된다. 그 뒤 normalize된 개별 XML은 기본 validator의 10 MiB `max_bytes` 검사를 다시 받으며, 초과는 transport 거절이 아니라 validation report의 오류가 된다. unsafe path, 대소문자 기준 중복 XML 파일명, 잘못된 ZIP, XML 없는 ZIP은 400이다. 각 XML은 normalize와 XSD/import validation 후 C.1.1/C.1.2 및 잠재 중복으로 `new`, `followUp`, `skip`, `error`를 결정한다.

정상 HTTP 응답은 일부 entry가 error/skipped여도 200일 수 있다.

```json
{
  "data": {
    "importedCases": [{
      "caseNumber": "US-...",
      "status": "success|warning|skipped|error",
      "message": null,
      "caseId": "uuid-or-null",
      "caseVersion": 1,
      "decision": "new|followUp|skip|error",
      "sourceFileName": "case.xml",
      "matchedCaseId": null,
      "matchedCaseNumber": null,
      "matchedCaseVersion": null
    }],
    "caseId": "first-success-uuid-or-null",
    "caseVersion": 1,
    "xmlKey": null,
    "parsedJsonId": null
  }
}
```

선택한 Product presave ID와 업무 `productId`는 imported drug/case 연결에 쓰인다. 설정이 켜진 경우 연결 Product의 sender presave를 imported sender 정보에 적용한다. 상품 연결은 [import 계약](../api_import/import_contract_web.rs)의 `test_import_selected_product_links_sender_without_overwriting_xml_sender`, 설정 효과는 [import settings](../api_import/import_settings_web.rs)의 `test_import_settings_apply_default_sender_only_when_enabled`, `test_import_settings_apply_product_linked_sender_by_imported_product_id`에서 확인한다.

같은 settings 파일의 `test_import_settings_update_enabled_c1_dates`는 현재 지원하는 설정 조합 `(creation=true, mostRecent=true, firstReceived=false)`로 앞의 두 날짜가 import 시각의 날짜로 갱신되고, 최초 수신일은 원본 XML 값으로 유지되는지 검사한다. import 전후 UTC 날짜를 사용해 자정 경계를 허용한다. 모든 설정 조합을 검사하지는 않는다.

활성 테스트는 filename 누락 400, product/sender 연결과 sender 적용, C.1 날짜 설정 효과, imported Case DB 상태를 assertion한다. missing file, 잘못된 UUID, 삭제 Product, scope 밖 Product, 빈 Product ID, ZIP 한도/경로/중복, duplicate decision별 batch 혼합 결과는 HTTP 활성 assertion이 전부 갖춰져 있지 않다.

<a id="validation"></a>

## Validation

`POST /api/import/xml/validate`는 같은 multipart file reader, 50 MiB file-part limit, 50 MiB + 64 KiB request limit과 `import.xml.validate` mutation 권한을 사용한다. 이 endpoint에는 Product 선택과 파일명이 필요하지 않으며 `file` 또는 `xml` part만 필요하다. XML normalize 후 기본 10 MiB validator로 검사한다. 활성 assertion은 valid XML의 `200`, `ok:true`, root element와 빈 errors, malformed XML의 `400`과 XML 관련 오류 body, XSD-invalid XML의 `200`, `ok:false`와 비어 있지 않은 errors를 실제 endpoint에서 확인한다. 전후 `cases`와 `xml_import_history` row count도 같음을 확인한다(runtime NOT_RUN). 용량 boundary와 권한 거절은 남아 있다.

<a id="history"></a>

## History

`GET /api/import/xml/history`는 `import.history.list` read 권한과 enforced scope를 적용하고 최근 200건을 `uploadedAt DESC, createdAt DESC`로 반환한다. 관리자 계열은 case가 없는 자기 조직 import를 포함하고, 그 외 unscoped error는 업로더 본인 것만 포함한다.

```json
{"data":{"items":[{
  "id":"uuid", "uploadedFileName":"batch.zip", "sourceFileName":"case.xml",
  "caseId":"uuid-or-null", "caseNumber":"US-...", "status":"success|warning|skipped|error",
  "errorMessage":null, "uploadedBy":"uuid", "uploaderEmail":"user@example.com",
  "uploadedAt":"2026-10-31T12:34:56Z"
}]}}
```

각 source XML마다 success/warning/skipped/error history를 별도 transaction으로 기록한다. [`import_history_web.rs::test_import_history_uploaded_at_is_rfc3339`](../api_import/import_history_web.rs)은 실제 import 뒤 200과 RFC3339 timestamp를 직접 확인한다. Case scope별 목록은 [`api_cases/scope/histories.rs`](../api_cases/scope/histories.rs#L179)에도 assertion이 있다.

`GET /api/import/xml/history/{id}/error.txt`는 `import.history.read`와 해당 history context를 요구한다. error가 있으면 200, `text/plain; charset=utf-8`, attachment filename `import-error-{id}-{sanitized-source}.txt`를 반환한다. record 없음과 error detail 없음은 현재 source에서 400이다. 이 endpoint의 [활성 오류 다운로드 검사](../api_import/import_history_web.rs) `test_import_history_error_details_download_as_text`는 DB에 준비한 오류 이력으로 실제 GET 응답의 200, content type, 이력 ID를 포함한 파일명과 오류 본문을 확인한다.

## 권한, 감사와 테스트 빈틈

import 실행/validation은 contextual mutation 정책, history는 contextual read 정책과 RLS context를 사용한다. import history가 per-entry 감사 이력 역할을 하며 업로더, 파일, Case, 상태, 오류, 시각을 보존한다. 일반 `audit_logs`에 대한 import endpoint별 active assertion은 없다.

실행 소속은 요청/상품 연결 `import_contract_web.rs`, 이력·오류 다운로드 `import_history_web.rs`, 설정 효과 `import_settings_web.rs`로 나뉘고 필요한 준비 함수만 `support.rs`에서 공유한다. 기존 컴파일 제외 검사 세 개 중 약한 timestamp 중복은 삭제했고 오류 다운로드·C.1 날짜 검사는 현재 계약에 맞춰 활성화했다. TSV의 네 경로 모두 하나 이상의 직접 HTTP assertion이 있지만 각 행의 미검증 축은 유지한다.

[XML import/export](../xml/import_export/mod.rs)와 [MFDS samples](../xml/mfds_import_samples_web.rs)도 관련 검사다. fixture 누락·setup/export 실패는 이제 성공으로 반환하지 않는다. MFDS sample 검사는 production parser가 산출한 section 존재·행 개수와 API 재조회를 대조하므로 영속 전달 근거이며, parser 자체의 모든 필드 정확성을 독립적으로 입증하는 oracle은 아니다.

ZIP batch의 부분 성공 원자성, 동시 import, 모든 duplicate branch의 Case version, history record 실패 시 import 결과, 업로드 상한 boundary, cross-org error download, 200 batch 안의 error를 UI가 처리하는 계약은 활성 HTTP assertion이 부족하다.

## MFDS 원본 샘플의 출처와 실행 조건

[식약처 의약품안전나라 배포 게시물](https://nedrug.mfds.go.kr/bbs/34/19) `XML 파일 Reference(MFDS, Republic of Korea)-(업데이트, 2021.5.27.)`의 [XML 첨부](https://nedrug.mfds.go.kr/cmn/edms/down/1NdaMZ5p64i)를 2026-10-31에 내려받아 [kr-literature-initial-v1.0.xml](../xml/fixtures/mfds/kr-literature-initial-v1.0.xml)로 보존했다. 원래 첨부명은 `1-1_ExampleCase_literature_KR_initial_v1_0_샘플.xml`, 크기는 68,407 bytes, SHA-256은 `4125399a0b8c37f7bc3a2cdc518feb864fa0104eaed00b2fe224e96a9c868159`이다. 파일 내용은 수정하지 않았으며 Git의 줄바꿈 변환과 원본 공백 정리를 제외한다.

MFDS 검사는 기본으로 이 추적된 디렉터리를 읽으므로 CI checkout에 포함된다. 별도 `E2BR3_MFDS_EXAMPLES_DIR`를 지정하면 그 디렉터리를 사용하며, 부재/빈 목록은 실패다. XSD와 FDA fixture 준비는 기존 `scripts/prepare-xml-assets.sh`와 CI 고정 자산 commit을 사용한다. 테스트 실행 중 외부 사이트에서 내려받지 않는다.

기존 로컬 `docs/exporter/mfds` 파일은 Git에서 제외됐고, 공식 원본과 달리 날짜 속성 두 곳 및 검사 `originalText` 요소 하나가 변경되어 있었다. 이를 공식 원본으로 채택하지 않았다. 2021년 배포 예제 하나의 확보는 최신 규칙 전체의 적합성이나 기관 접수를 증명하지 않는다. 현재 importer/validator와 원본 간 호환 여부는 실행하지 않았으며, 불일치를 만나면 샘플을 조용히 변조하거나 검사를 건너뛰지 않는다.
