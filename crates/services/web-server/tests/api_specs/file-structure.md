# API 테스트 파일별 구조 명세

**검토일: 2026-10-31 / 실행: NOT_RUN.** 현재 소스 등록·책임 목록이다. 테스트·빌드·API·DB 실행 결과가 아니다. [API 계약](README.md), [operation 색인](coverage.tsv), [반복행 검사 행렬](row-lifecycle.md)을 함께 확인한다.

9개 API 관련 target의 하위 Rust 파일 203개와 root 9개, 공유 helper·격리 guard 3개, lib-core database 8개까지 총 **223개 파일**이다. [TSV](file-structure.tsv)는 같은 목록이다.

`직접 선언`은 `#[test]`/`#[tokio::test]` 함수 수(ignored 포함), `scalar 등록`은 생성 파일의 field macro 호출 수다. Editor 수기 async verifier는 generated 등록에서 호출하므로 직접 선언 0이어도 실행 경로가 있다.

Editor는 scalar 373 + behavior 60 = 433개를 등록한다. 공통 patch 생성기 자체 검사 2개는 별개다. UI canonical 필드 360개·HTTP operation 436개·실행 성공 수와 구분한다.

## 실행 단위 요약

| Target | 하위 Rust 파일 | 직접 선언 | scalar 등록 | ignored 선언 |
|---|---:|---:|---:|---:|
| `api_cases` | 52 | 144 | 0 | 0 |
| `api_editor` | 42 | 62 | 373 | 0 |
| `api_presave` | 14 | 44 | 0 | 0 |
| `api_import` | 4 | 7 | 0 | 0 |
| `api_submission` | 12 | 29 | 0 | 1 |
| `api_terminology` | 6 | 23 | 0 | 0 |
| `api_administration` | 31 | 64 | 0 | 0 |
| `authz` | 24 | 84 | 0 | 1 |
| `xml` | 18 | 20 | 0 | 0 |

## web-server::api_cases

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [api_cases.rs](../api_cases.rs) | api_cases Cargo integration target과 하위 모듈 연결 | target 등록 | 31 | 0 | 0 |
| [api_cases/case_support.rs](../api_cases/case_support.rs) | Case HTTP 준비·workflow·validation 호출 공통 함수 | helper | 830 | 0 | 0 |
| [api_cases/cioms_export.rs](../api_cases/cioms_export.rs) | CIOMS PDF 다운로드 응답·파일 구조·case 번호·권한 및 조직 경계 | HTTP 계약 | 182 | 2 | 0 |
| [api_cases/compliance.rs](../api_cases/compliance.rs) | Case nullification/amendment 코드와 감사 사유·전자서명 요구 | HTTP 계약 | 116 | 3 | 0 |
| [api_cases/contract/audit.rs](../api_cases/contract/audit.rs) | Case CRUD 감사와 긴 한글 narrative 저장의 감사·dirty flag·no-op 계약 | HTTP 계약 | 257 | 2 | 0 |
| [api_cases/contract/delete.rs](../api_cases/contract/delete.rs) | Case 삭제 사유·soft-delete 가시성·삭제 후 수정 거절 | HTTP 계약 | 325 | 3 | 0 |
| [api_cases/contract/list_view.rs](../api_cases/contract/list_view.rs) | Case 목록 투영·누락된 safety report·limit/offset | HTTP 계약 | 386 | 3 | 0 |
| [api_cases/contract/mod.rs](../api_cases/contract/mod.rs) | api_cases/contract 하위 책임 모듈 등록 | 모듈 등록 | 15 | 0 | 0 |
| [api_cases/contract/query.rs](../api_cases/contract/query.rs) | Case 조건 검색의 가벼운 목록 응답 | HTTP 계약 | 114 | 1 | 0 |
| [api_cases/contract/review_receivers.rs](../api_cases/contract/review_receivers.rs) | RE 수신자 JSON 저장·clear·행 정규화·기한 기본값과 입력 거절 | HTTP 계약 | 922 | 10 | 0 |
| [api_cases/contract/save.rs](../api_cases/contract/save.rs) | Case 생성·수정 공개 필드·C.1.2 기본 시각·생략/null/blank 저장 의미 | HTTP 계약 | 569 | 8 | 0 |
| [api_cases/contract/support.rs](../api_cases/contract/support.rs) | Case contract HTTP 전송·JSON/raw 응답 helper | helper | 141 | 0 | 0 |
| [api_cases/contract/validation_cache.rs](../api_cases/contract/validation_cache.rs) | 목록 경고 수·기관별 cache·수정 동시성과 validator revision이 바뀐 cache 무효화 | HTTP 계약 | 625 | 4 | 0 |
| [api_cases/field_notation.rs](../api_cases/field_notation.rs) | 필드 주석 저장·clear·행 ID·감사 및 recordId 존재/case/org 소속 | HTTP 계약 | 549 | 5 | 0 |
| [api_cases/intake/duplicates.rs](../api_cases/intake/duplicates.rs) | Intake 중복 기준·활성 필드·override·누락/NullFlavor 입력 | HTTP 계약 | 490 | 8 | 0 |
| [api_cases/intake/input.rs](../api_cases/intake/input.rs) | Intake 필수 product·실패 시 rollback·follow-up 복제와 버전 | HTTP 계약 | 262 | 3 | 0 |
| [api_cases/intake/mod.rs](../api_cases/intake/mod.rs) | api_cases/intake 하위 책임 모듈 등록 | 모듈 등록 | 5 | 0 | 0 |
| [api_cases/intake/persistence.rs](../api_cases/intake/persistence.rs) | Intake로 생성한 C.1 날짜·batch 번호와 downstream XML export | HTTP 계약 | 146 | 3 | 0 |
| [api_cases/intake/support.rs](../api_cases/intake/support.rs) | Intake 입력·follow-up 페이지 및 HTTP fixture helper | helper | 314 | 0 | 0 |
| [api_cases/lifecycle.rs](../api_cases/lifecycle.rs) | Case 상태 입력·보호 상태·전이 금지·검토 상태와 저장 분리 | HTTP 계약 | 142 | 5 | 0 |
| [api_cases/locking.rs](../api_cases/locking.rs) | Lock/QC 축·상태별 수정 차단·stale 화면 충돌 | HTTP 계약 | 325 | 7 | 0 |
| [api_cases/scope/assignments.rs](../api_cases/scope/assignments.rs) | 사용자 sender/product scope 할당·clear·blinded 필드와 권한 scope 구분 | HTTP 계약 | 198 | 4 | 0 |
| [api_cases/scope/cases.rs](../api_cases/scope/cases.rs) | Case 조회·목록·수정의 sender/product/study scope 교집합 | HTTP 계약 | 754 | 9 | 0 |
| [api_cases/scope/editor.rs](../api_cases/scope/editor.rs) | Editor 약물 생성의 product 할당 scope | HTTP 계약 | 206 | 1 | 0 |
| [api_cases/scope/histories.rs](../api_cases/scope/histories.rs) | Import/export/submission 이력 조회의 동일 Case scope 정책 | HTTP 계약 | 232 | 1 | 0 |
| [api_cases/scope/mod.rs](../api_cases/scope/mod.rs) | api_cases/scope 하위 책임 모듈 등록 | 모듈 등록 | 7 | 0 | 0 |
| [api_cases/scope/sender_options.rs](../api_cases/scope/sender_options.rs) | Routing sender 선택지의 presave/역할 scope | HTTP 계약 | 278 | 4 | 0 |
| [api_cases/scope/support.rs](../api_cases/scope/support.rs) | Scope 역할·사용자·Case·Presave·이력 준비 helper | helper | 559 | 0 | 0 |
| [api_cases/settings.rs](../api_cases/settings.rs) | Workflow config 응답과 관리자 case-number 설정의 Case 생성 결과 | HTTP 계약 | 124 | 2 | 0 |
| [api_cases/subresources/drug.rs](../api_cases/subresources/drug.rs) | Drug와 하위 substance/dosage/device/relatedness CRUD·삭제/복원·원자성; 소속 경계는 drug_scope | HTTP 계약 | 708 | 8 | 0 |
| [api_cases/subresources/drug_scope.rs](../api_cases/subresources/drug_scope.rs) | legacy Drug 자식의 Case/부모 소속·삭제된 부모 접근 거절·부모 복원 후 자식 ID/count | HTTP 계약 | 489 | 4 | 0 |
| [api_cases/subresources/lifecycle_audit.rs](../api_cases/subresources/lifecycle_audit.rs) | 감사 행 조회와 actor/org/reason/old/new/changed_fields 검증 | 공통 검증 helper | 185 | 0 | 0 |
| [api_cases/subresources/lifecycle_drug.rs](../api_cases/subresources/lifecycle_drug.rs) | Drug와 자식 9종의 재저장·삭제/복원·소속·권한·잠금·감사 | HTTP 계약 | 310 | 1 | 0 |
| [api_cases/subresources/lifecycle_general.rs](../api_cases/subresources/lifecycle_general.rs) | Narrative·linked report·reaction·test result 6종과 field notation lifecycle | HTTP 계약 | 542 | 2 | 0 |
| [api_cases/subresources/lifecycle_patient.rs](../api_cases/subresources/lifecycle_patient.rs) | Patient와 자식 10종의 재저장·삭제/복원·소속·권한·잠금·감사 | HTTP 계약 | 379 | 1 | 0 |
| [api_cases/subresources/lifecycle_policy.rs](../api_cases/subresources/lifecycle_policy.rs) | viewer·다른 조직/부모·Case 잠금 거절과 상태·감사 불변 | 공통 검증 helper | 210 | 0 | 0 |
| [api_cases/subresources/lifecycle_report.rs](../api_cases/subresources/lifecycle_report.rs) | Report·sender·study·source document·header·receiver 12종 lifecycle | HTTP 계약 | 441 | 1 | 0 |
| [api_cases/subresources/lifecycle_singleton.rs](../api_cases/subresources/lifecycle_singleton.rs) | Singleton 갱신·동일 저장·hard delete/재생성 계약 | 공통 검증 helper | 282 | 0 | 0 |
| [api_cases/subresources/lifecycle_support.rs](../api_cases/subresources/lifecycle_support.rs) | 기존 행 계약·새 GET/list·soft/hard 삭제/복원 재호출 공통 assertion | 공통 검증 helper | 601 | 0 | 0 |
| [api_cases/subresources/mod.rs](../api_cases/subresources/mod.rs) | api_cases/subresources 하위 책임 모듈 등록 | 모듈 등록 | 21 | 0 | 0 |
| [api_cases/subresources/narrative.rs](../api_cases/subresources/narrative.rs) | Narrative·summary·sender diagnosis CRUD/복원과 patient token preview | HTTP 계약 | 228 | 5 | 0 |
| [api_cases/subresources/patient.rs](../api_cases/subresources/patient.rs) | Patient/parent 병력·identifier·단위·NullFlavor·감사 사유·소속과 복원 | HTTP 계약 | 623 | 7 | 0 |
| [api_cases/subresources/primary_source.rs](../api_cases/subresources/primary_source.rs) | Reporter 지역별 RP 필드 저장 | HTTP 계약 | 134 | 1 | 0 |
| [api_cases/subresources/reaction.rs](../api_cases/subresources/reaction.rs) | Reaction AE 공통 필드와 MFDS 의료기기 필드 생성·수정 | HTTP 계약 | 87 | 1 | 0 |
| [api_cases/subresources/safety_report.rs](../api_cases/subresources/safety_report.rs) | Safety report C.1·하위 identifier/document/link CRUD·복원·날짜/NullFlavor | HTTP 계약 | 331 | 4 | 0 |
| [api_cases/subresources/sender.rs](../api_cases/subresources/sender.rs) | Sender MFDS 건강전문가 유형 KR.1 저장 | HTTP 계약 | 63 | 1 | 0 |
| [api_cases/subresources/singleton.rs](../api_cases/subresources/singleton.rs) | 단일행 리소스 POST 멱등성·없을 때 응답·행 ID 유지 | HTTP 계약 | 196 | 1 | 0 |
| [api_cases/subresources/study.rs](../api_cases/subresources/study.rs) | Study type reaction 입력 저장 | HTTP 계약 | 42 | 1 | 0 |
| [api_cases/subresources/support.rs](../api_cases/subresources/support.rs) | 하위 리소스 HTTP/raw 응답·Case/부모 행 준비 helper | helper | 371 | 0 | 0 |
| [api_cases/subresources/test_result.rs](../api_cases/subresources/test_result.rs) | 검사결과 normal range 길이·MedDRA 검사명 대체 입력 | HTTP 계약 | 96 | 2 | 0 |
| [api_cases/validation.rs](../api_cases/validation.rs) | Case validation 기관 선택·오류 projection·validator token과 상태 갱신 | HTTP 계약 | 386 | 10 | 0 |
| [api_cases/validation_common.rs](../api_cases/validation_common.rs) | Validation fixture·JSON HTTP·필수 Case 하위 행 준비 helper | helper | 366 | 0 | 0 |
| [api_cases/workflow.rs](../api_cases/workflow.rs) | Workflow 전이·담당자 지정·역할/담당자/lock 제한과 관리자 override 감사 | HTTP 계약 | 515 | 6 | 0 |

## web-server::api_editor

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [api_editor.rs](../api_editor.rs) | api_editor Cargo integration target과 하위 모듈 연결 | target 등록 | 4 | 0 | 0 |
| [api_editor/audit.rs](../api_editor/audit.rs) | Editor 전후 값·감사 snapshot 비교와 거절 후 불변 공통 assertion | helper | 249 | 0 | 0 |
| [api_editor/audit/assessment.rs](../api_editor/audit/assessment.rs) | 인과성 assessment 저장 값과 단일 감사 entry | HTTP 계약 | 130 | 0 | 0 |
| [api_editor/audit/projection.rs](../api_editor/audit/projection.rs) | FDA device scalar 저장의 감사 API projection | HTTP 계약 | 105 | 0 | 0 |
| [api_editor/fields.rs](../api_editor/fields.rs) | Registry 기반 scalar 저장/clear/ID/감사 검사 공통 실행기와 patch 생성 자체 검사 | HTTP 계약 | 625 | 2 | 0 |
| [api_editor/fields/coupled_clear.rs](../api_editor/fields/coupled_clear.rs) | 연결 필드 clear·생략 구분·sibling 유지·행 재조회 | HTTP 계약 | 614 | 0 | 0 |
| [api_editor/fields/message_header.rs](../api_editor/fields/message_header.rs) | N registry 항목의 전용 message-header POST/PUT/GET 저장·거절·감사 불변 검사 | HTTP 계약 | 181 | 0 | 0 |
| [api_editor/fields/null_flavor.rs](../api_editor/fields/null_flavor.rs) | 값↔NullFlavor 전이·동시 값 거절·부모/등록번호 관계 | HTTP 계약 | 566 | 0 | 0 |
| [api_editor/fields/required_values.rs](../api_editor/fields/required_values.rs) | 필수 값 blank/잘못된 report year 거절과 저장 값·자식 보존 | HTTP 계약 | 209 | 0 | 0 |
| [api_editor/mod.rs](../api_editor/mod.rs) | api_editor 하위 책임 모듈 등록 | 모듈 등록 | 42 | 0 | 0 |
| [api_editor/pages/ae.generated.rs](../api_editor/pages/ae.generated.rs) | AE registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 77 | 3 | 49 |
| [api_editor/pages/ci.generated.rs](../api_editor/pages/ci.generated.rs) | CI registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 69 | 6 | 26 |
| [api_editor/pages/dg.generated.rs](../api_editor/pages/dg.generated.rs) | DG registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 177 | 14 | 92 |
| [api_editor/pages/dh.generated.rs](../api_editor/pages/dh.generated.rs) | DH registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 49 | 4 | 16 |
| [api_editor/pages/dm.generated.rs](../api_editor/pages/dm.generated.rs) | DM registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 140 | 8 | 87 |
| [api_editor/pages/lb.generated.rs](../api_editor/pages/lb.generated.rs) | LB registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 57 | 6 | 14 |
| [api_editor/pages/lr.generated.rs](../api_editor/pages/lr.generated.rs) | LR registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 31 | 3 | 3 |
| [api_editor/pages/n.generated.rs](../api_editor/pages/n.generated.rs) | N registry의 8개 header 필드 검사 실행 등록; 전용 REST API 사용 | 생성된 실행 등록 | 21 | 0 | 8 |
| [api_editor/pages/nr.generated.rs](../api_editor/pages/nr.generated.rs) | NR registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 46 | 5 | 8 |
| [api_editor/pages/rp.generated.rs](../api_editor/pages/rp.generated.rs) | RP registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 62 | 4 | 29 |
| [api_editor/pages/sd.generated.rs](../api_editor/pages/sd.generated.rs) | SD registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 45 | 1 | 27 |
| [api_editor/pages/si.generated.rs](../api_editor/pages/si.generated.rs) | SI registry의 scalar 및 behavior 검사 실행 등록; 검사 구현은 fields/rows/relations/audit 공유 | 생성된 실행 등록 | 57 | 6 | 14 |
| [api_editor/relations.rs](../api_editor/relations.rs) | 관계 모듈 등록·바깥 행 5종 재저장/삭제/복원·감사와 하위 PATCH→GET·삭제 감사·자원별 권한/잠금 공통 assertion | 모듈 등록 | 507 | 0 | 0 |
| [api_editor/relations/ae_restore.rs](../api_editor/relations/ae_restore.rs) | AE 삭제 행 read/restore와 case·권한 경계 | HTTP 계약 | 239 | 0 | 0 |
| [api_editor/relations/ci_rows.rs](../api_editor/relations/ci_rows.rs) | CI soft 자식 3종 삭제/복원/재저장·ID/count·소속·감사·rollback | HTTP 계약 | 198 | 0 | 0 |
| [api_editor/relations/device_children.rs](../api_editor/relations/device_children.rs) | Device 목록 교체 시 제거된 부모의 하위행 삭제 | HTTP 계약 | 200 | 0 | 0 |
| [api_editor/relations/dg_restore.rs](../api_editor/relations/dg_restore.rs) | Drug 삭제/복원 시 자식과 case·권한 경계 | HTTP 계약 | 273 | 0 | 0 |
| [api_editor/relations/dg_rows.rs](../api_editor/relations/dg_rows.rs) | DG 하위 행 삭제·일반 재저장·명시 복원·반복 요청·ID/count·새 GET·malformed ID 불변 | HTTP 계약 | 166 | 0 | 0 |
| [api_editor/relations/dm_rows.rs](../api_editor/relations/dm_rows.rs) | DM 하위 행 삭제·일반 재저장·명시 복원·반복 요청·ID/count·새 GET·malformed ID 불변 | HTTP 계약 | 244 | 0 | 0 |
| [api_editor/relations/documents.rs](../api_editor/relations/documents.rs) | Source document 삭제 시 목표 행 숨김 | HTTP 계약 | 102 | 0 | 0 |
| [api_editor/relations/lb_restore.rs](../api_editor/relations/lb_restore.rs) | LB 삭제 행 read/restore와 case·권한 경계 | HTTP 계약 | 225 | 0 | 0 |
| [api_editor/relations/nr_rows.rs](../api_editor/relations/nr_rows.rs) | NR 하위 행 삭제·일반 재저장·명시 복원·반복 요청·ID/count·새 GET·malformed ID 불변 | HTTP 계약 | 139 | 0 | 0 |
| [api_editor/relations/parents.rs](../api_editor/relations/parents.rs) | 부모 자동 생성/재사용·자식 수정의 부모 ID 유지 | HTTP 계약 | 236 | 0 | 0 |
| [api_editor/relations/relatedness.rs](../api_editor/relations/relatedness.rs) | Relatedness 삭제의 sibling 보존과 assessment 부모 삭제 | HTTP 계약 | 386 | 0 | 0 |
| [api_editor/relations/restore.rs](../api_editor/relations/restore.rs) | DH/LR 삭제 행의 원자적 복원·수정 공통 검사 | HTTP 계약 | 243 | 0 | 0 |
| [api_editor/relations/rp_rows.rs](../api_editor/relations/rp_rows.rs) | RP 하위 행 삭제·일반 재저장·명시 복원·반복 요청·ID/count·새 GET·malformed ID 불변 | HTTP 계약 | 113 | 0 | 0 |
| [api_editor/relations/scope.rs](../api_editor/relations/scope.rs) | Study 자식의 케이스 분리·외부 ID 거절 | HTTP 계약 | 672 | 0 | 0 |
| [api_editor/relations/si_rows.rs](../api_editor/relations/si_rows.rs) | SI 하위 행 삭제·일반 재저장·명시 복원·반복 요청·ID/count·새 GET·malformed ID 불변 | HTTP 계약 | 158 | 0 | 0 |
| [api_editor/rows.rs](../api_editor/rows.rs) | api_editor 하위 책임 모듈 등록 | 모듈 등록 | 3 | 0 | 0 |
| [api_editor/rows/creation.rs](../api_editor/rows/creation.rs) | 빈 collection/child 객체 생성 의미·draft 행·빈 product 거절 | HTTP 계약 | 157 | 0 | 0 |
| [api_editor/rows/identity.rs](../api_editor/rows/identity.rs) | Drug device·code 하위행 재조회/교체의 ID 보존 | HTTP 계약 | 404 | 0 | 0 |
| [api_editor/rows/projection.rs](../api_editor/rows/projection.rs) | 반복행 정렬·목록 필드·일괄 상세 일치·인증 | HTTP 계약 | 293 | 0 | 0 |
| [api_editor/support.rs](../api_editor/support.rs) | Editor HTTP·행 ID·registry behavior 조회·fixture helper | helper | 324 | 0 | 0 |

## web-server::api_presave

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [api_presave.rs](../api_presave.rs) | api_presave Cargo integration target과 하위 모듈 연결 | target 등록 | 4 | 0 | 0 |
| [api_presave/audit_web.rs](../api_presave/audit_web.rs) | 부모/자식 15종 HTTP 수정의 actor/org/reason/category/old/new 감사 | HTTP 계약 | 367 | 1 | 0 |
| [api_presave/authorization_web.rs](../api_presave/authorization_web.rs) | 부모/자식 15종 read-only/조직 경계와 거절 후 상태·감사 불변 | HTTP 계약 | 679 | 1 | 0 |
| [api_presave/crud_matrix_web.rs](../api_presave/crud_matrix_web.rs) | Sender/receiver/product/reporter 공통 aggregate CRUD와 생략/null/blank 저장 | HTTP 계약 | 648 | 3 | 0 |
| [api_presave/delete_constraints_web.rs](../api_presave/delete_constraints_web.rs) | 참조 중인 Presave 부모 삭제 제약과 product soft-delete scope | HTTP 계약 | 162 | 2 | 0 |
| [api_presave/helpers.rs](../api_presave/helpers.rs) | Presave HTTP와 부모·자식·Info editor fixture helper | helper | 649 | 0 | 0 |
| [api_presave/legacy_routes_web.rs](../api_presave/legacy_routes_web.rs) | 제거한 legacy presave templates URL의 404 | HTTP 계약 | 27 | 1 | 0 |
| [api_presave/lifecycle_matrix_web.rs](../api_presave/lifecycle_matrix_web.rs) | 반복 삭제/복원·부모 보관 시 자식 ID 보존·부모 소속과 rollback | HTTP 계약 | 868 | 4 | 0 |
| [api_presave/mod.rs](../api_presave/mod.rs) | api_presave 하위 책임 모듈 등록 | 모듈 등록 | 13 | 0 | 0 |
| [api_presave/narrative_web.rs](../api_presave/narrative_web.rs) | Narrative Presave CRUD·모든 parent/child 필드 저장 | HTTP 계약 | 158 | 1 | 0 |
| [api_presave/product_web.rs](../api_presave/product_web.rs) | Product 정체성·기관 공통 값·scope와 details graph 원자 저장/삭제/오류 | HTTP 계약 | 886 | 10 | 0 |
| [api_presave/receiver_web.rs](../api_presave/receiver_web.rs) | Receiver 필수 값·중복·timeline과 routes/details graph 계약 | HTTP 계약 | 537 | 4 | 0 |
| [api_presave/reporter_web.rs](../api_presave/reporter_web.rs) | Reporter MFDS qualification detail 입력·재조회 | HTTP 계약 | 173 | 2 | 0 |
| [api_presave/sender_web.rs](../api_presave/sender_web.rs) | Sender 정체성·person/gateway details graph·명시적 삭제·rollback·감사 사유 | HTTP 계약 | 942 | 10 | 0 |
| [api_presave/study_web.rs](../api_presave/study_web.rs) | Study registration/product/reporter graph 저장·삭제·잘못된 child 거절 | HTTP 계약 | 751 | 5 | 0 |

## web-server::api_import

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [api_import.rs](../api_import.rs) | api_import Cargo integration target과 하위 모듈 연결 | target 등록 | 10 | 0 | 0 |
| [api_import/import_contract_web.rs](../api_import/import_contract_web.rs) | XML upload filename·선택 product/sender 연결과 XML 값 보존 | HTTP 계약 | 232 | 2 | 0 |
| [api_import/import_history_web.rs](../api_import/import_history_web.rs) | Import 이력 시각 형식과 오류 상세 text 다운로드 | HTTP 계약 | 154 | 2 | 0 |
| [api_import/import_settings_web.rs](../api_import/import_settings_web.rs) | Import C.1 날짜·default sender·product-linked sender 설정 적용 | HTTP 계약 | 386 | 3 | 0 |
| [api_import/support.rs](../api_import/support.rs) | 실제 XML fixture 탐색·import multipart와 JSON/raw HTTP 준비 | helper | 208 | 0 | 0 |

## web-server::api_submission

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [api_submission.rs](../api_submission.rs) | api_submission Cargo integration target과 하위 모듈 연결 | target 등록 | 4 | 0 | 0 |
| [api_submission/callbacks.rs](../api_submission/callbacks.rs) | 내부 ACK callback 인증·상태 갱신·중복 idempotency·malformed 거부와 Rust-to-submitter payload 흐름을 검증한다. | HTTP 계약 | 412 | 5 | 0 |
| [api_submission/dispatch.rs](../api_submission/dispatch.rs) | FDA ESG/MFDS 제출 routing, transport 설정 오류, 헤더·payload, 비성공/연결 실패 상태 보존을 검증한다. | HTTP 계약 | 346 | 6 | 0 |
| [api_submission/external.rs](../api_submission/external.rs) | 실제 Java submitter와 MFDS 제출·event 연계를 확인하는 명시적 ignored 외부 통합 테스트를 담는다. | HTTP 계약 | 83 | 1 | 0 |
| [api_submission/headers.rs](../api_submission/headers.rs) | authority별 receiver option과 message-header 식별자 갱신, 동시 FDA/MFDS 요청의 request-scoped 값을 검증한다. | HTTP 계약 | 369 | 3 | 0 |
| [api_submission/history.rs](../api_submission/history.rs) | 제출 history의 최신 ACK 시각·event, 제출 XML과 ACK text 다운로드 계약을 검증한다. | HTTP 계약 | 205 | 2 | 0 |
| [api_submission/idempotency.rs](../api_submission/idempotency.rs) | 제출 idempotency key의 순차 재사용과 병렬 요청 단일 submission 생성을 검증한다. | HTTP 계약 | 127 | 2 | 0 |
| [api_submission/mod.rs](../api_submission/mod.rs) | callback·dispatch·external·header·history·idempotency·reconcile·validation 제출 테스트와 support를 등록한다. | 모듈 등록 | 10 | 0 | 0 |
| [api_submission/reconcile/mod.rs](../api_submission/reconcile/mod.rs) | reconcile의 HTTP endpoint 테스트와 직접 worker 테스트 모듈을 등록한다. | 모듈 등록 | 2 | 0 | 0 |
| [api_submission/reconcile/service.rs](../api_submission/reconcile/service.rs) | 직접 reconcile worker를 호출해 legacy attempt 차단, AS2 pending poll, 무재전송과 늦은 ACK 순서 보존을 검증한다. | worker 계약 | 221 | 2 | 0 |
| [api_submission/reconcile/web.rs](../api_submission/reconcile/web.rs) | 내부 reconcile endpoint의 token 인증·빈 결과·runtime counter와 ESG attempt 무재전송을 HTTP로 검증한다. | HTTP 계약 | 342 | 4 | 0 |
| [api_submission/support.rs](../api_submission/support.rs) | 제출 테스트의 case/report seed, HTTP client, idempotency·compliance payload와 로컬 ESG/AS2 transport harness를 제공한다. | fixture/helper | 707 | 0 | 0 |
| [api_submission/validation.rs](../api_submission/validation.rs) | case workflow/규칙/MedDRA 상태가 제출을 막는 조건과 manual XML export의 독립성을 검증한다. | HTTP 계약 | 257 | 4 | 0 |

## web-server::api_terminology

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [api_terminology.rs](../api_terminology.rs) | api_terminology Cargo integration target과 하위 모듈 연결 | target 등록 | 4 | 0 | 0 |
| [api_terminology/imports.rs](../api_terminology/imports.rs) | MedDRA·WHODrug zip dry-run/stage parser, 잘못된 archive, C3 batch persistence와 실제 case 용어 사용 흐름을 검증한다. | HTTP 계약 | 540 | 8 | 0 |
| [api_terminology/mod.rs](../api_terminology/mod.rs) | terminology import·permission·release·search 테스트와 support를 등록한다. | 모듈 등록 | 6 | 0 | 0 |
| [api_terminology/permissions.rs](../api_terminology/permissions.rs) | viewer·operational user의 용어 조회 권한과 import/release mutation 거부를 검증한다. | HTTP 계약 | 113 | 3 | 0 |
| [api_terminology/releases.rs](../api_terminology/releases.rs) | 사전 release 승인·활성화·rollback·언어 전환·재import와 활성 사전 변경 후 Case 경고 cache 무효화 | HTTP 계약 | 756 | 7 | 0 |
| [api_terminology/search.rs](../api_terminology/search.rs) | 국가·code list·MedDRA·WHODrug·활성 MFDS product/substance 검색 응답을 검증한다. | HTTP 계약 | 160 | 5 | 0 |
| [api_terminology/support.rs](../api_terminology/support.rs) | terminology 관리자·활성 release seed, MedDRA/WHODrug/C3 archive·multipart 생성, persistence 조회와 HTTP 요청 helper를 제공한다. | fixture/helper | 377 | 0 | 0 |

## web-server::api_administration

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [api_administration.rs](../api_administration.rs) | api_administration Cargo integration target과 하위 모듈 연결 | target 등록 | 21 | 0 | 0 |
| [api_administration/admin_settings/appendices_web.rs](../api_administration/admin_settings/appendices_web.rs) | 관리자 설정의 appendices 저장·조회와 빈 값 거부 계약을 HTTP로 검증한다. | HTTP 계약 | 53 | 1 | 0 |
| [api_administration/admin_settings/audit_web.rs](../api_administration/admin_settings/audit_web.rs) | 관리자 설정 변경 시 변경 필드가 감사 로그에 기록되는지 HTTP 응답과 저장 감사 행으로 검증한다. | HTTP 계약 | 171 | 1 | 0 |
| [api_administration/admin_settings/mod.rs](../api_administration/admin_settings/mod.rs) | 관리자 설정의 appendices·audit·notice·settings·workflow 테스트 모듈을 등록한다. | 모듈 등록 | 5 | 0 | 0 |
| [api_administration/admin_settings/notices_web.rs](../api_administration/admin_settings/notices_web.rs) | 대시보드 공지의 조직 격리·감사 기록과 notice-read 권한에 따른 runtime 노출을 HTTP로 검증한다. | HTTP 계약 | 227 | 2 | 0 |
| [api_administration/admin_settings/settings_web.rs](../api_administration/admin_settings/settings_web.rs) | MedDRA release, idle-session, 정렬·case-number·import-date 설정의 검증·조직 격리·round-trip을 HTTP로 검증한다. | HTTP 계약 | 309 | 3 | 0 |
| [api_administration/admin_settings/workflow_web.rs](../api_administration/admin_settings/workflow_web.rs) | workflow due-day와 허용 역할 설정에서 음수·미등록·system-admin 거부 및 빈 역할 허용을 HTTP로 검증한다. | HTTP 계약 | 156 | 4 | 0 |
| [api_administration/app_branding_web.rs](../api_administration/app_branding_web.rs) | 앱 branding API가 기본 이름과 구성된 이름을 반환하는 계약을 HTTP로 검증한다. | HTTP 계약 | 67 | 2 | 0 |
| [api_administration/audit_web.rs](../api_administration/audit_web.rs) | 감사 목록·정상 hash-chain 무결성 보고와 viewer 거절; 손상된 chain 보고는 미검사 | HTTP 계약 | 85 | 2 | 0 |
| [api_administration/error_mapping_web.rs](../api_administration/error_mapping_web.rs) | 사용자 API의 잘못된 UUID는 400, 존재하지 않는 UUID는 404로 매핑되는지 검증한다. | HTTP 계약 | 44 | 2 | 0 |
| [api_administration/middleware_ctx.rs](../api_administration/middleware_ctx.rs) | 인증 middleware가 사용자 context를 해석하고 연속 요청 사이에 context를 누출하지 않는지 검증한다. | HTTP 계약 | 80 | 2 | 0 |
| [api_administration/role_admin/crud_web.rs](../api_administration/role_admin/crud_web.rs) | custom permission profile 생성·수정·soft delete와 메뉴 권한·설명 persistence 및 비활성 역할 사용자 목록 동작을 검증한다. | HTTP 계약 | 293 | 5 | 0 |
| [api_administration/role_admin/effective_access.rs](../api_administration/role_admin/effective_access.rs) | 역할 effective-access의 administration·case·dashboard·information·persistence·terminology·transfer 모듈을 등록한다. | 모듈 등록 | 9 | 0 | 0 |
| [api_administration/role_admin/effective_access/administration_web.rs](../api_administration/role_admin/effective_access/administration_web.rs) | custom 역할의 admin/user/PDF/audit 권한 행렬이 실제 관리 API 접근으로 반영되는지 검증한다. | HTTP 계약 | 571 | 4 | 0 |
| [api_administration/role_admin/effective_access/case_web.rs](../api_administration/role_admin/effective_access/case_web.rs) | case 권한 행렬과 home-workflow-read가 case 목록·queue 접근 및 profile capability에 반영되는지 검증한다. | HTTP 계약 | 181 | 2 | 0 |
| [api_administration/role_admin/effective_access/dashboard_web.rs](../api_administration/role_admin/effective_access/dashboard_web.rs) | home notice 권한의 runtime capability 반영과 예약된 report/mail 행의 비권한 효과를 검증한다. | HTTP 계약 | 134 | 2 | 0 |
| [api_administration/role_admin/effective_access/information_web.rs](../api_administration/role_admin/effective_access/information_web.rs) | information 권한 행렬이 sender presave의 조회·생성·수정·삭제 권한으로 반영되는지 검증한다. | HTTP 계약 | 218 | 1 | 0 |
| [api_administration/role_admin/effective_access/persistence_web.rs](../api_administration/role_admin/effective_access/persistence_web.rs) | permission profile의 privilege matrix 메뉴 키가 저장되고 current-user profile에 재현되는지 검증한다. | HTTP 계약 | 225 | 1 | 0 |
| [api_administration/role_admin/effective_access/support.rs](../api_administration/role_admin/effective_access/support.rs) | effective-access 하위 테스트가 역할 생성·API 호출·seed helper와 공용 타입을 한 경로에서 쓰도록 재노출한다. | helper | 20 | 0 | 0 |
| [api_administration/role_admin/effective_access/terminology_web.rs](../api_administration/role_admin/effective_access/terminology_web.rs) | terminology 권한 행렬이 국가 조회, MedDRA import와 release 승인 접근으로 반영되는지 검증한다. | HTTP 계약 | 141 | 1 | 0 |
| [api_administration/role_admin/effective_access/transfer_web.rs](../api_administration/role_admin/effective_access/transfer_web.rs) | export/submission/import 권한 행렬이 XML export, submission receiver, import와 history 접근으로 분리 적용되는지 검증한다. | HTTP 계약 | 329 | 2 | 0 |
| [api_administration/role_admin/helpers.rs](../api_administration/role_admin/helpers.rs) | custom 역할·사용자·case 하위 자원 생성, 권한 갱신, 상태 assertion과 DB fixture를 제공한다. | fixture/helper | 606 | 0 | 0 |
| [api_administration/role_admin/metadata_web.rs](../api_administration/role_admin/metadata_web.rs) | 역할 metadata, authoritative assignment, current-user 표시 이름, 활성 역할 수 제한과 기본 visible name을 검증한다. | HTTP 계약 | 233 | 5 | 0 |
| [api_administration/role_admin/mod.rs](../api_administration/role_admin/mod.rs) | 역할 관리 CRUD·metadata·validation·review lock·effective-access 모듈과 helper를 등록한다. | 모듈 등록 | 6 | 0 | 0 |
| [api_administration/role_admin/review_lock_web.rs](../api_administration/role_admin/review_lock_web.rs) | case review와 lock 권한·상태 전환을 분리하고 reserved email 행 및 메뉴 정규화를 검증한다. | HTTP 계약 | 476 | 4 | 0 |
| [api_administration/role_admin/validation_web.rs](../api_administration/role_admin/validation_web.rs) | 역할 API가 migration alias, 중복 이름, 과도한 이름·설명을 생성과 수정에서 거부하는지 검증한다. | HTTP 계약 | 262 | 5 | 0 |
| [api_administration/support.rs](../api_administration/support.rs) | administration 테스트용 Axum JSON 요청 실행과 JSON/raw 응답 해석을 제공한다. | helper | 32 | 0 | 0 |
| [api_administration/user_organization_membership_web.rs](../api_administration/user_organization_membership_web.rs) | 한 계정의 조직 membership 목록·전환과 membership 없는 조직 전환 거부를 검증한다. | HTTP 계약 | 115 | 2 | 0 |
| [api_administration/users/access_web.rs](../api_administration/users/access_web.rs) | 임시 비밀번호와 최초 로그인 재설정, 접근 종료 비활성화, access 기간 persistence를 검증한다. | HTTP 계약 | 283 | 3 | 0 |
| [api_administration/users/create_web.rs](../api_administration/users/create_web.rs) | 관리자 사용자 생성, 기본 역할·선택 필드 기본값, 중복 email과 길이 제한 오류를 검증한다. | HTTP 계약 | 258 | 5 | 0 |
| [api_administration/users/mod.rs](../api_administration/users/mod.rs) | 사용자 생성·수정·접근 수명주기 테스트 모듈을 등록한다. | 모듈 등록 | 3 | 0 | 0 |
| [api_administration/users/update_web.rs](../api_administration/users/update_web.rs) | 관리자 사용자 수정, 빈 이름 거부·시작일 제거와 username/email 길이 제한을 검증한다. | HTTP 계약 | 119 | 3 | 0 |

## web-server::authz

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [authz.rs](../authz.rs) | authz Cargo integration target과 하위 모듈 연결 | target 등록 | 36 | 0 | 0 |
| [authz/authentication_web.rs](../authz/authentication_web.rs) | login·refresh·logoff 상태, 잘못된 자격증명/token, 생성 사용자 로그인, legacy hash upgrade와 인증 middleware 거부를 검증한다. | HTTP 계약 | 450 | 10 | 0 |
| [authz/authorization_snapshot.rs](../authz/authorization_snapshot.rs) | DB에서 repeatable-read로 적재한 authorization snapshot이 request-parts middleware extractor로 전달되고, 미부착 요청은 거부되는지 검증한다. | middleware component | 42 | 2 | 0 |
| [authz/protected_route_inventory.rs](../authz/protected_route_inventory.rs) | source parser가 만든 보호 route inventory의 대표 context·중복·family·checked-in snapshot 일치를 검사한다; HTTP 요청 검사는 아니다. | 소스 gate | 83 | 5 | 0 |
| [authz/protected_route_parser.rs](../authz/protected_route_parser.rs) | Rust route 등록 source를 method/path/context inventory로 변환하는 parser와 괄호·인자 분리 로직을 단위 검증한다. | 소스 gate | 236 | 2 | 0 |
| [authz/rbac_audit.rs](../authz/rbac_audit.rs) | audit 목록·record 조회·case trail의 역할/조직 범위, pagination, hard-delete와 field filter 권한을 HTTP로 검증한다. | HTTP 계약 | 656 | 13 | 0 |
| [authz/rbac_cases.rs](../authz/rbac_cases.rs) | 관리자 case 생성과 viewer 생성 거부·목록 허용을 검증한다. | HTTP 계약 | 86 | 2 | 0 |
| [authz/rbac_drug.rs](../authz/rbac_drug.rs) | 관리자 drug 생성 허용과 viewer 생성 거부를 검증한다. | HTTP 계약 | 56 | 1 | 0 |
| [authz/rbac_narrative.rs](../authz/rbac_narrative.rs) | 관리자 narrative 생성 허용과 viewer 생성 거부를 검증한다. | HTTP 계약 | 55 | 1 | 0 |
| [authz/rbac_organizations.rs](../authz/rbac_organizations.rs) | organization 관리 endpoint의 system-admin 전용 CRUD와 잘못된 org type 거부를 검증한다. | HTTP 계약 | 348 | 8 | 0 |
| [authz/rbac_patient.rs](../authz/rbac_patient.rs) | 관리자 patient 생성 허용과 viewer 생성 거부를 검증한다. | HTTP 계약 | 56 | 1 | 0 |
| [authz/rbac_safety_report.rs](../authz/rbac_safety_report.rs) | 관리자 safety report 생성 허용과 viewer 생성 거부를 검증한다. | HTTP 계약 | 67 | 1 | 0 |
| [authz/rbac_subresources.rs](../authz/rbac_subresources.rs) | drug active-substance와 patient medical-history 하위 자원의 관리자 허용·viewer 거부를 검증한다. | HTTP 계약 | 173 | 2 | 0 |
| [authz/rbac_users/mod.rs](../authz/rbac_users/mod.rs) | 사용자 역할 생성/수정 정책·비관리자 거절·profile·조직 scope 모듈 등록 | 모듈 등록 | 5 | 0 | 0 |
| [authz/rbac_users/non_admin_forbidden_web.rs](../authz/rbac_users/non_admin_forbidden_web.rs) | 비관리자의 사용자 CRUD·목록·workflow option 접근 거부와 permission denied 응답을 검증한다. | HTTP 계약 | 288 | 9 | 0 |
| [authz/rbac_users/permission_profiles_web.rs](../authz/rbac_users/permission_profiles_web.rs) | system-admin eligible action, target-org 요구, cross-org 숨김, sponsor org scope와 custom role 20개 제한을 검증한다. | HTTP 계약 | 350 | 5 | 0 |
| [authz/rbac_users/rls_scope_web.rs](../authz/rbac_users/rls_scope_web.rs) | system/sponsor admin 사용자 목록·단건 조회가 현재 조직으로 제한되는지 검증한다. | HTTP 계약 | 151 | 1 | 0 |
| [authz/rbac_users/role_create_web.rs](../authz/rbac_users/role_create_web.rs) | 사용자 생성 시 plain user 허용, sponsor-admin 위임·중복·profile 없는 역할 거부를 검증한다. | HTTP 계약 | 158 | 4 | 0 |
| [authz/rbac_users/role_update_web.rs](../authz/rbac_users/role_update_web.rs) | 사용자 hide/restore, plain-role 수정, normalized assignment 원자성, sponsor-admin 역할 위임 제한을 검증한다. | HTTP 계약 | 274 | 6 | 0 |
| [authz/rls_web.rs](../authz/rls_web.rs) | users 관리자 guard와 case 목록·단건·version의 조직 RLS 범위를 실제 HTTP 요청으로 검증한다. | HTTP 계약 | 141 | 3 | 0 |
| [authz/source_gates/authorization_legacy_gate.rs](../authz/source_gates/authorization_legacy_gate.rs) | REST authorization source가 permit-bound RLS context와 role.restore action을 유지하는지 문자열 gate로 검사한다. | 소스 gate | 77 | 3 | 0 |
| [authz/source_gates/authorization_startup.rs](../authz/source_gates/authorization_startup.rs) | startup authorization storage 순서, bootstrap membership 선행과 system-org membership 처리를 source gate로 검사한다. | 소스 gate | 52 | 3 | 0 |
| [authz/source_gates/mod.rs](../authz/source_gates/mod.rs) | authorization legacy·startup·platform policy source gate 모듈을 등록한다. | 모듈 등록 | 3 | 0 | 0 |
| [authz/source_gates/platform_admin_policy.rs](../authz/source_gates/platform_admin_policy.rs) | user/audit에는 system-admin permission bypass가 없고 organization만 system-admin 전용인지 source로 검사한다. | 소스 gate | 38 | 2 | 0 |
| [authz/support.rs](../authz/support.rs) | RBAC 테스트가 인증된 HTTP로 case fixture를 생성하는 helper를 제공한다. | helper | 45 | 0 | 0 |

## web-server::xml

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [xml.rs](../xml.rs) | xml Cargo integration target과 하위 모듈 연결 | target 등록 | 16 | 0 | 0 |
| [xml/import_export/downloads.rs](../xml/import_export/downloads.rs) | XML import 후 단건·bulk export, zip 파일명·내용, export history와 다운로드 XML을 HTTP와 XML 검증으로 확인한다. | HTTP/XML 통합 | 348 | 1 | 0 |
| [xml/import_export/drug_updates.rs](../xml/import_export/drug_updates.rs) | XML import 뒤 D/G drug 하위 자원을 HTTP로 수정하고 재export XML에 갱신값이 반영되는지 검증한다. | HTTP/XML 통합 | 307 | 1 | 0 |
| [xml/import_export/mod.rs](../xml/import_export/mod.rs) | XML import/export의 download·drug update·nullFlavor·rejection 모듈을 등록한다. | 모듈 등록 | 5 | 0 | 0 |
| [xml/import_export/nullflavor.rs](../xml/import_export/nullflavor.rs) | XML F/E nullFlavor를 import한 뒤 test-result와 reaction API readback에 보존되는지 검증한다. | HTTP/XML 통합 | 103 | 2 | 0 |
| [xml/import_export/rejections.rs](../xml/import_export/rejections.rs) | 필수 header·기관별 N 수신처 위반의 단건/ZIP 거절과 정상 draft 다운로드·오류 이력 계약을 검증한다. | HTTP/XML 통합 | 381 | 2 | 0 |
| [xml/import_export/support.rs](../xml/import_export/support.rs) | XML fixture 변형·고유 report ID 생성과 multipart import/case-id 추출 helper를 제공한다. | helper | 98 | 0 | 0 |
| [xml/mfds_import_samples_web.rs](../xml/mfds_import_samples_web.rs) | MFDS example XML들을 HTTP import하고 case 각 section·validation·재export 및 XML schema 결과를 검증한다. | HTTP/XML 통합 | 398 | 1 | 0 |
| [xml/roundtrip/drug.rs](../xml/roundtrip/drug.rs) | D/G drug·active substance·dosage·indication 잔여 필드를 HTTP 편집 후 XML export round-trip으로 검증한다. | HTTP/XML 통합 | 219 | 1 | 0 |
| [xml/roundtrip/mod.rs](../xml/roundtrip/mod.rs) | XML roundtrip의 drug·narrative·patient·reaction·report·schema 모듈과 support를 등록한다. | 모듈 등록 | 8 | 0 | 0 |
| [xml/roundtrip/narrative.rs](../xml/roundtrip/narrative.rs) | N/R narrative와 summary 필드를 HTTP 편집 후 XML export round-trip으로 검증한다. | HTTP/XML 통합 | 121 | 1 | 0 |
| [xml/roundtrip/patient.rs](../xml/roundtrip/patient.rs) | D/M patient·parent·past-drug 필드와 parent nullFlavor를 HTTP/API 및 serializer round-trip으로 검증한다. | HTTP/XML 통합 | 242 | 2 | 0 |
| [xml/roundtrip/reaction.rs](../xml/roundtrip/reaction.rs) | E adverse-event와 sender diagnosis 전체 필드를 HTTP persistence와 XML export round-trip으로 검증한다. | HTTP/XML 통합 | 635 | 2 | 0 |
| [xml/roundtrip/report.rs](../xml/roundtrip/report.rs) | C/I report identification·message header 및 R/P source·sender·study 필드를 HTTP 편집 후 XML round-trip으로 검증한다. | HTTP/XML 통합 | 312 | 2 | 0 |
| [xml/roundtrip/schema.rs](../xml/roundtrip/schema.rs) | persisted section의 RLS export, authority별 full-build schema/rule 검증과 빈 autopsy parent 출력 구조를 검증한다. | HTTP/XML 통합 | 538 | 3 | 0 |
| [xml/roundtrip/support.rs](../xml/roundtrip/support.rs) | roundtrip fixture import, FDA device 보정, authority header와 HTTP XML export helper를 제공한다. | helper | 239 | 0 | 0 |
| [xml/roundtrip_profiles_web.rs](../xml/roundtrip_profiles_web.rs) | ICH/FDA/MFDS fixture를 HTTP import·validate·export하고 원본/재export XML을 schema 재검증한다. | HTTP/XML 통합 | 276 | 1 | 0 |
| [xml/support.rs](../xml/support.rs) | workspace/XSD 경로 해석, Axum JSON 요청과 reaction language·batch date 준비 helper를 제공한다. | helper | 259 | 0 | 0 |
| [xml/targeted_device_export.rs](../xml/targeted_device_export.rs) | 같은 조직의 1,001개 장치와 교차 조직 행에서 대상 case 장치·code만 production serializer가 내보내는지 검증한다. | XML 변환 | 203 | 1 | 0 |

## web-server::shared

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [common/mod.rs](../common/mod.rs) | 공통 DB 격리 guard·환경·조직/사용자/role/Case fixture; 각 target이 필요한 함수만 사용 | 공유 준비 | 831 | 0 | 0 |
| [common/permission_profiles.rs](../common/permission_profiles.rs) | 실제 permission-profiles HTTP 요청으로 빈 권한 profile 생성 | 공유 helper | 32 | 0 | 0 |

## web-server::isolation_guard

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [isolation_guard.rs](../isolation_guard.rs) | 공유 개발 DB를 통합 테스트 대상으로 사용하는 경로 거절 | 격리 guard | 18 | 1 | 0 |

## lib-core::database

| 파일 | 책임 | 종류 | 줄 | 직접 선언 | scalar 등록 |
|---|---|---|---:|---:|---:|
| [tests/database.rs](../../../../libs/lib-core/tests/database.rs) | database 모듈 등록과 실제 DB 이름이 전용 e2br3_test_ 이름인지 확인 | 모듈 등록/격리 guard | 32 | 0 | 0 |
| [tests/database/audit_chain.rs](../../../../libs/lib-core/tests/database/audit_chain.rs) | 감사 hash-chain 동시성 순서와 JSONB native 비교 11개 의미 | DB | 100 | 2 | 0 |
| [tests/database/device_rls.rs](../../../../libs/lib-core/tests/database/device_rls.rs) | Device/code/characteristic BMC의 tenant 본인 행 허용·타 조직 은닉 | DB | 238 | 1 | 0 |
| [tests/database/editor_storage.rs](../../../../libs/lib-core/tests/database/editor_storage.rs) | Editor 값/NullFlavor DB 컬럼 쌍과 mutual-exclusion constraint 존재 | DB schema | 140 | 1 | 0 |
| [tests/database/reaction_term_highlight_migration.rs](../../../../libs/lib-core/tests/database/reaction_term_highlight_migration.rs) | Reaction highlight의 신규 varchar schema와 legacy boolean migration | DB migration | 138 | 2 | 0 |
| [tests/database/submission_schema.rs](../../../../libs/lib-core/tests/database/submission_schema.rs) | 제출 dispatch state 감사 schema guard | DB schema | 75 | 1 | 0 |
| [tests/database/transactions.rs](../../../../libs/lib-core/tests/database/transactions.rs) | Dbx nested commit/rollback·query 실패·취소 후 transaction 상태 | DB | 174 | 4 | 0 |
| [tests/database/user_expiration.rs](../../../../libs/lib-core/tests/database/user_expiration.rs) | 관리자 UserBmc update→만료 사용자 비활성화→같은 조직 재조회 | DB | 108 | 1 | 0 |

## Rust 외의 테스트 자산

| 파일 | 역할 |
|---|---|
| [api-contract-manifest.json](api-contract-manifest.json) | 백엔드436 operation·중첩 DTO·소스 해시·테스트/명세 연결 |
| [check_api_contract_manifest.py](check_api_contract_manifest.py) | stdlib 정적 생성/불일치 검사; Cargo 실행 단위 아님 |
| [authz/protected_route_inventory.snapshot](../authz/protected_route_inventory.snapshot) | 보호 API 429개 등록 inventory 기준; 동작 검사가 아님 |
| [xml/fixtures/mfds/kr-literature-initial-v1.0.xml](../xml/fixtures/mfds/kr-literature-initial-v1.0.xml) | MFDS 공식 sample 원본; 출처와 사용 제한은 Import 명세 참조 |
| [xml/fixtures/mfds/.gitattributes](../xml/fixtures/mfds/.gitattributes) | XML 원본 byte/개행 보존 |

## 경계를 유지하는 이유

- HTTP handler의 권한/응답/저장 결과는 web-server, 직접 BMC·SQL·DB constraint는 lib-core database가 소유한다. HTTP 검사 안의 DB readback은 그 요청의 결과를 확인하므로 같은 함수에 남긴다.
- Presave graph 원자 저장, Case scope의 여러 endpoint 비교, Intake→XML 결과는 한 계약을 확인하는 응집된 검사다. 파일 줄 수만으로 독립 테스트로 쪼개지 않는다.
- Editor의 `pages/*.generated.rs`는 registry가 관리하는 실행 등록이다. `fields/rows/relations/audit`는 수기 assertion 구현이며 역할이 달라 둘 다 필요하다.
- `api_submission/reconcile/service.rs`는 web-server worker 구현의 직접 검사다. HTTP API 증거로 계산하지 않는다. 기존 transport harness 출력도 기관 수용 증거로 계산하지 않는다.
- `authz/source_gates`와 보호 route parser/inventory는 소스 등록 검사다. `authorization_snapshot.rs`는 DB-backed middleware extractor 검사다. 실제 HTTP 허용/거절과 구분한다.
- `xml`은 실제 HTTP import/export와 serializer/XSD 계약을 하위 파일로 구분한다. Targeted device export와 autopsy schema 검사는 HTTP API 증거로 계산하지 않는다.

## CI 선택과 미확인 범위

`scripts/ci_test_plan.py`는 test 파일의 owning target과 crate 의존성을 사용한다. 하위 test 파일 변경은 해당 target, 공통 test helper 변경은 이를 공유하는 crate target, production lib-core 변경은 필요한 의존 crate까지 선택한다. 파일을 나누었다는 이유만으로 같은 Cargo target 내부 함수를 개별 job으로 실행하지 않는다. API 명세 문서만 바꾸면 Rust 실행을 선택하지 않는다.

Editor는 registry의 verifier 연결에 따라 변경 leaf의 실제 사용 페이지·책임 범위를 선택한다. 여러 범위를 한 `api_editor` 실행과 한 DB 준비로 묶으며 공통 patch 생성기 자체 검사 2개도 실행한다. 공통 모듈·연결 없는 leaf 변경은 Editor 전체로 넓힌다. Clippy도 test-only 변경은 선택 integration target만, production 변경은 의존 crate의 전체 target을 검사한다.

테스트만 수정하면 frontend/registry 전체 job은 생략하지만, backend 필수 계약은 현재 editor contract와 registry의 명시적 test 연결에서 읽어 실제 실행 로그와 대조한다. Production·계약 입력 변경 시에는 frontend 추출/테스트와 전체 registry 검증을 유지하고 두 계약 목록의 전체·target별 일치도 확인한다. XML 자산은 xml crate, web-server 전체 또는 api_import/api_submission/xml target에만 준비하며 CI와 로컬 격리 실행기가 같은 선택 규칙을 사용한다.

Module 연결·파일 누락·명세 근거·이동 전후 선언 및 assertion을 정적으로 대조했다. 컴파일, SQL 실행, 전체 runtime PASS는 미확인이다. 모든 동작의 공백은 각 API 명세의 미단언 항목과 coverage.tsv의 gaps에 남긴다.
