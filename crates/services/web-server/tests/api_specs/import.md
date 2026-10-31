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

선택한 Product presave ID와 업무 `productId`는 imported drug/case 연결에 쓰인다. 설정이 켜진 경우 연결 Product의 sender presave를 imported sender 정보에 적용한다. import date 설정은 C.1 날짜를 선택적으로 갱신한다. Product/sender 상태 동작은 [`import_contract_web.rs::test_import_selected_product_links_sender_without_overwriting_xml_sender`](../api_import/import_contract_web.rs#L293), [`test_import_settings_apply_default_sender_only_when_enabled`](../api_import/import_contract_web.rs#L616), [`test_import_settings_apply_product_linked_sender_by_imported_product_id`](../api_import/import_contract_web.rs#L774)에서 직접 확인한다. C.1 날짜 검사는 [`test_import_settings_update_enabled_c1_dates`](../api_import/import_contract_web.rs#L551)에 있으나 `#[cfg(any())]`로 비활성이다.

활성 테스트는 filename 누락 400, product/sender 연결과 sender 적용, imported Case DB 상태를 assertion한다. source에 있는 C.1 날짜 설정, missing file, 잘못된 UUID, 삭제 Product, scope 밖 Product, 빈 Product ID, ZIP 한도/경로/중복, duplicate decision별 batch 혼합 결과는 HTTP 활성 assertion이 전부 갖춰져 있지 않다.

<a id="validation"></a>

## Validation

`POST /api/import/xml/validate`는 같은 multipart file reader, 50 MiB file-part limit, 50 MiB + 64 KiB request limit과 `import.xml.validate` mutation 권한을 사용한다. 이 endpoint에는 Product 선택과 파일명이 필요하지 않으며 `file` 또는 `xml` part만 필요하다. XML normalize 후 기본 10 MiB validator로 검사하고 `XmlValidationReport`를 `{"data":{"ok":...,"errors":[...]}}` 형태로 200 반환한다. 이 route는 source와 보호 snapshot에는 있지만 직접 호출하는 활성 API assertion이 없다. import handler 내부 validation이나 Case validation 테스트를 이 별도 endpoint의 증거로 간주하지 않는다.

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

각 source XML마다 success/warning/skipped/error history를 별도 transaction으로 기록한다. [`import_history_web.rs::test_import_history_uploaded_at_is_rfc3339`](../api_import/import_history_web.rs#L78)은 실제 import 뒤 200과 RFC3339 timestamp를 직접 확인한다. Case scope별 목록은 [`api_cases/scope/histories.rs`](../api_cases/scope/histories.rs#L179)에도 assertion이 있다.

`GET /api/import/xml/history/{id}/error.txt`는 `import.history.read`와 해당 history context를 요구한다. error가 있으면 200, `text/plain; charset=utf-8`, attachment filename `import-error-{id}-{sanitized-source}.txt`를 반환한다. record 없음과 error detail 없음은 현재 source에서 400이다. 이 endpoint의 content/header 검사는 [`import_contract_web.rs::test_import_history_error_details_download_as_text`](../api_import/import_contract_web.rs#L483)에 있으나 함수가 `#[cfg(any())]`로 비활성화되어 coverage에는 포함하지 않았다.

## 권한, 감사와 테스트 빈틈

import 실행/validation은 contextual mutation 정책, history는 contextual read 정책과 RLS context를 사용한다. import history가 per-entry 감사 이력 역할을 하며 업로더, 파일, Case, 상태, 오류, 시각을 보존한다. 일반 `audit_logs`에 대한 import endpoint별 active assertion은 없다.

`api_import`의 test attribute 중 세 개는 `#[cfg(any())]`로 비활성이다. `xml_import_export`와 MFDS sample test도 관련 근거지만, 일부는 `E2BR3_EXAMPLES_DIR` 또는 fixture directory가 없으면 assertion 전에 성공 반환으로 skip되므로 무조건 실행 증거나 해당 route coverage로 세지 않았다. TSV는 네 경로 중 import 실행과 history 목록만 `asserted`, validation과 error download는 `registration_only`로 분류한다.

ZIP batch의 부분 성공 원자성, 동시 import, 모든 duplicate branch의 Case version, history record 실패 시 import 결과, 업로드 상한 boundary, cross-org error download, 200 batch 안의 error를 UI가 처리하는 계약은 활성 HTTP assertion이 부족하다.
