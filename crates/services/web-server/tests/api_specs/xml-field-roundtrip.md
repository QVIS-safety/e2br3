# XML 전체 필드 왕복 검사

검사는 `crates/services/web-server/tests/api_editor/xml_fields.rs`에 있다.
실제 DB 저장까지 확인하므로 `xml` 크레이트 단위 검사와 별도로
`web-server`의 `api_editor` 통합 테스트에 등록한다.

`registry/dictionary/{ich-e2br3,fda-regional,mfds-regional}.json`에서 모든
element와 허용 nullFlavor를 직접 열거한다. 저장 경로, 유효값, 필요한
행은 기존 `registry/editor-contracts`를 재사용한다. 새 필드의 저장 경로가
없거나 모호하면 실패하며, 빈 값을 비교해 해당 필드가 검증됐다고 하지 않는다.
N.1.1은 편집 필드가 아닌 고정 전송 형식이므로 별도로 확인한다.

- ICH 공통 필드는 ICH·FDA·MFDS 형식 모두 검사한다.
- 지역 전용 필드는 해당 형식으로 검사한다.
- 각 필드의 대표값과 허용된 nullFlavor 각각을 별도 케이스로 저장한다.
- 원본 저장값 확인 → XML export → 바이트 수정 없이 다른 조직에 import → 대상 저장값 비교 순서다.
- 성공 응답뿐 아니라 새 케이스 생성, 양쪽 조직 소속, 원본 불변 여부를 확인한다.
- 반복 경로는 모든 행과 중첩 구조를 비교한다. 연결 보고서는 서로 다른 두 값의 순서와 개수도 확인한다.
- 반응 참조 ID는 대상 조직의 새 반응 행으로 연결됐는지 검사한다.
- N.1.5의 SQL timestamp → 원본 문자열 저장 차이는 정확한 시간·소수초·offset을 확인한다.
- 대상 조직의 발신자 및 날짜 덮어쓰기 설정은 명시적으로 끈다.
- 실패해도 다음 시나리오를 계속 실행하고, 마지막에 전체 실패를 보고하며 테스트를 실패시킨다.

2026-11-22 사전 기준 265개 항목이며, 형식별로 값 711개와 nullFlavor
473개를 합쳐 1,184개 시나리오다. 대표값 하나가 통과했다고 해당 필드의
가능한 모든 값과 모든 필드 조합이 검증된 것은 아니다.

미완성 케이스도 검사 대상이다. 예를 들어 중간 이름만 저장하거나 환자
정보 없이 연구 정보만 저장한 경우를 임의의 주변 값으로 보충하지 않는다.
제출용 XSD·업무 규칙 검사와 값 보존 검사는 별개다. 배열형 값이
nullFlavor로 대체될 때의 빈 배열은 저장 API의 명시적 표현으로 비교하며,
일반 필드의 null·빈 문자열·0·false를 서로 같게 처리하지 않는다.

## 실행

`SERVICE_DB_URL`에 임시 DB를 만들 수 있는 로컬 PostgreSQL 연결을 제공한다.
아래 runner가 별도의 `e2br3_test_*` DB를 만들고 종료 시 제거한다.
운영/개발 케이스에 이 테스트 값을 입력하지 않는다.

```sh
E2BR3_XML_FIELD_REPORT=/tmp/xml-field-report-new.json \
scripts/test-isolated-db.sh --locked -p web-server --test api_editor \
  case_editor_contract_web::xml_fields -- --test-threads=1 --nocapture
```

보고서 파일은 새 경로여야 한다. 환경변수를 생략하면 libtest 로그와
종료 코드로 결과를 확인할 수 있다. 이 테스트는 격리 DB의 결정적
fixture 검증이며 실제 서버 데이터의 성공 사례로 제시하면 안 된다.

## 2026-11-22 실행 결과

전체 1,184개 중 **1,060개 통과, 124개 실패**. 사전 항목으로는 31개,
필드와 형식을 묶으면 74개 조합에서 실패했다. 실패를 제외하거나 성공으로 바꾸지 않았다.

| 형식 | 통과 | 실패 |
|---|---:|---:|
| ICH | 335 | 38 |
| FDA | 378 | 45 |
| MFDS | 347 | 41 |

실패 분류: 값 누락·변환 89건, export 오류 25건, import 거절 10건.
시나리오 설정·저장 확인 단계의 실패는 최종 실행에 없었다.

| 사전 항목 | 형식 | 실패 시나리오 | 분류 |
|---|---|---:|---|
| FDA.C.5.5a | fda | 1 | 값 누락·변환 |
| FDA.C.5.5b | fda | 1 | 값 누락·변환 |
| FDA.C.5.6.r | fda | 2 | 값 누락·변환 |
| FDA.D.1 | fda | 1 | import 거절 |
| FDA.G.k.10.1 | fda | 1 | 값 누락·변환 |
| FDA.G.k.12.r.5 | fda | 2 | 값 누락·변환 |
| C.2.r.1.3 | fda, ich, mfds | 12 | export 오류 |
| C.2.r.2.1 | fda, ich, mfds | 12 | 값 누락·변환 |
| C.3.3.4 | fda, ich, mfds | 3 | export 오류 |
| C.5.1.r.1 | fda, ich, mfds | 9 | 값 누락·변환 |
| C.5.1.r.2 | fda, ich, mfds | 9 | 값 누락·변환 |
| C.5.2 | fda, ich, mfds | 9 | 값 누락·변환 |
| C.5.3 | fda, ich, mfds | 9 | 값 누락·변환 |
| C.5.4 | fda, ich, mfds | 3 | 값 누락·변환 |
| D.10.8.r.2a | fda, ich, mfds | 3 | 값 누락·변환 |
| D.10.8.r.3a | fda, ich, mfds | 3 | 값 누락·변환 |
| D.8.r.2a | ich, mfds | 2 | 값 누락·변환 |
| D.8.r.3a | fda, ich, mfds | 3 | 값 누락·변환 |
| E.i.6b | fda, ich, mfds | 3 | 값 누락·변환 |
| F.r.3.3 | fda, ich, mfds | 3 | 값 누락·변환 |
| G.k.10.r | fda, ich, mfds | 3 | export 오류 |
| G.k.4.r.11.1 | fda, ich, mfds | 9 | import 거절 |
| G.k.4.r.4 | fda, ich, mfds | 3 | export 오류 |
| G.k.4.r.5 | fda, ich, mfds | 3 | export 오류 |
| G.k.9.i.2.r.3 | mfds | 1 | 값 누락·변환 |
| G.k.9.i.3.1a | fda, ich, mfds | 3 | 값 누락·변환 |
| G.k.9.i.3.1b | fda, ich, mfds | 3 | 값 누락·변환 |
| G.k.9.i.3.2a | fda, ich, mfds | 3 | 값 누락·변환 |
| G.k.9.i.3.2b | fda, ich, mfds | 3 | 값 누락·변환 |
| C.2.r.4.KR.1 | mfds | 1 | export 오류 |
| C.5.4.KR.1 | mfds | 1 | 값 누락·변환 |

대표 조건은 다음과 같다.

- 부서 없이 신고자 조직명만 저장하면 XML의 부서 위치로 출력되고, 대상에서는 부서 값 또는 부서 nullFlavor로 이동한다.
- 이름의 중간 부분만 저장하면 export가 500으로 실패한다.
- 환자 정보가 없는 케이스의 연구 정보가 전달되지 않는다.
- 반응 지속시간 단위 `d`가 재입력 후 `804`로 바뀐다.
- 검사 결과값 없이 저장한 단위, 일부 제품 식별자 버전과 투여 시간 간격 값이 누락된다.
- 모체 투여 경로의 UNK·ASKU·NASK, FDA 환자 식별의 NA는 exporter 출력의 재입력이 거절된다. 원인은 아래 단계 추적에 구분했다.

현재 테스트는 이 실패 때문에 CI에서 실패한다. 이번 변경은 검사 추가와 결과 기록이며,
발견된 serializer/parser 문제의 수정 완료를 뜻하지 않는다.

실행 로그와 전체 시나리오 결과는 이 서버에 보관했다.

- `/home/opc/e2br3/runtime-dev/xml-all-fields-results-20261122/scenarios.json`
- `/home/opc/e2br3/runtime-dev/xml-all-fields-results-20261122/summary.json`
- `/tmp/xml-all-fields-run-profiles-20261122.log`

실제 실행 중인 개발 서버도 기존 B→A 전달 케이스를 다시 읽었다. 이 실제 데이터에서는
사전 265개 중 41개에 값 또는 nullFlavor가 있어 비교됐고, 212개는 비어 있었으며
11개는 부모 정보가 없었다. 나머지 1개는 이미 별도로 허용한 추가 정보 컬렉션의
`[]`/null 차이다. 이 읽기 검증을 새 업로드나 전체 필드의 실제 데이터 검증으로 세지 않는다.
근거: `/home/opc/e2br3/runtime-dev/xml-all-fields-live-20261122/dictionary-coverage.json`.

## 추가 확인

비교 함수 단위 테스트 `projection_preserves_all_rows_and_rejects_missing_leaves`는 통과했다.
`git diff --check`도 통과했다.

다음 Clippy 실행은 기존 `api_editor` 테스트 파일들의 `expect`, `unwrap`,
금지된 fallback 메서드 등 91건으로 실패했다. 새 `xml_fields.rs`에는
Clippy 오류가 보고되지 않았다. 기존 파일을 고치거나 lint를 완화하지 않았다.

```sh
cargo clippy --locked --all-features -p web-server --test api_editor -- \
  -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic \
  -D clippy::todo -D clippy::unimplemented -D clippy::disallowed_methods
```

로그: `/tmp/xml-all-fields-clippy-20261122.log`.

## 2026-11-22 단계 추적: 어디서 처음 달라지는가

진단 정보를 추가해 전체 1,184개를 다시 실행했다. 1,060개 통과·124개 실패로,
이전 실행과 실패 시나리오 집합이 정확히 일치했다. 이 실행도 별도 실제 PostgreSQL
테스트 DB와 실제 애플리케이션 HTTP handler를 사용했다. 실제 고객 데이터의 실행 결과는 아니다.

각 실패에 원본 API 조회값, export HTTP 상태와 출력 원문, import 응답,
대상 API 조회값을 보관한다. 출력된 XML에 대해서는 공개된 D/E/F/G parser 결과도
직접 기록한다. 해당 정보를 사용해 분류했고, 단순 문자열 존재 여부만으로
필드가 보존됐다고 판단하지 않았다. 실패한 nullFlavor 시나리오는 원본에
해당 nullFlavor가 실제 저장됐는지도 다시 확인했다.

| 처음 차이 또는 실패가 생기는 지점 | 시나리오 수 | 확인된 내용 |
|---|---:|---|
| export: 값/노드 미출력 | 59 | 연구 정보 44, 과거약물 MPID/PHPID 버전 11, 결과값 없는 검사 단위 3, MFDS 인과성 자유문자 결과 1 |
| XML 필드 매핑 불일치: 다른 필드로 이동 | 15 | 신고자 조직명/부서 12, 일반기기명/브랜드명 2, FDA 전문 제품 분류/일반 특성 1 |
| import: XML에 있는 값 미저장 | 12 | 투여 시작/마지막 투여와 반응 사이의 시간 간격 값·단위 |
| import: 값의 코드 표현 변환 | 3 | 반응 지속시간 단위 `d` → `804` |
| export: 출력 오류 | 25 | 중간 이름 15, MFDS 자격 1, 날짜 저장 표현 충돌 6, 추가정보 배열 표현 불일치 3 |
| import: 재입력 거절 | 10 | 모체 투여 경로 nullFlavor 9, FDA 환자 식별 NA 1 |

앞서 묶었던 **값 누락·변환 89건 = 59 + 15 + 12 + 3**이다.
나머지 35건은 값 비교까지 도달하지 못한 export/import 오류다.

### 미출력과 필드 이동은 다르다

- **연구 정보**: 원본에는 있지만 XML에 `researchStudy`가 없다.
  `export/sections/c.rs::apply_c_5_study`가 환자 `primaryRole` 없는 XML에
  연구 조각을 삽입하지 못해도 성공으로 끝난다. 공통 연구 필드·FDA 연구 필드·MFDS 연구 유형에서 재현됐다.
- **과거약물 버전**: `export/shared/postprocess.rs`의 MPID/PHPID 출력이 식별자 값 존재를
  조건으로 하므로 버전만 저장한 케이스에서는 식별자 노드 전체가 없다.
- **검사 단위**: `export/sections/f.rs::write_f_r_test_result`가 숫자 결과를 쓰는 분기에서
  단위도 출력한다. 원본 `mg/dL`이 출력 XML에는 없다.
- **MFDS 인과성 자유문자**: `export/sections/g.rs::write_g_k_9_causality`의 MFDS 분기는
  지역 코드 필드만 처리한다. 일반 결과 `Possible`은 XML에 없다. 이 항목은 형식별 표현 정책도 고려해야 한다.
- **신고자 조직명**: 원본 `reporterOrganization="QVIS Safety"`가 XML의 직접
  `representedOrganization/name`으로 출력되고 대상 `reporterDepartment`로 저장된다.
  사전의 조직명 경로는 그 아래 중첩 `assignedEntity/representedOrganization/name`이다.
  exporter의 `apply_c_2_primary_source_values`는 부서 유무에 따라 조직명 위치를 바꾸지만,
  importer의 `read_c_2_r_2_1_2`는 직접 위치를 항상 부서로 읽는다. nullFlavor도 같은 경로로 이동한다.
- **FDA 일반기기명**: 브랜드명이 없으면 exporter가 `name` 하나만 출력한다.
  importer는 `name[1]`을 브랜드명, `name[2]`를 일반기기명으로 읽어
  `Common device` 또는 NI가 브랜드명 쪽으로 이동한다.
- **FDA 전문 제품 분류**: XML에는 `FDAGK101` characteristic의 `value/@code="C102834"`로 있다.
  대상에도 `deviceCharacteristics`로 저장된다. 그러나 원래 `fda_specialized_product_category`는 null이다.
  `import_fda_specialized_product_category`가 특정 `displayName`까지 요구하는데 exporter는 이를 출력하지 않는다.
  따라서 이 항목은 XML 미출력이 아니라 서로 맞지 않는 필드 복원 조건이다.

### importer에서 달라지는 항목

- **G.k.9.i.3.1a/b, G.k.9.i.3.2a/b**: XML의 SAS/SAE `pauseQuantity`에 값·단위와 반응 참조가 있다.
  `import_sections/g_drug/helpers.rs::parse_drug_observations`는 이를 map에 읽지만,
  재투여 반응 `code=31` observation을 순회할 때만 결과 행에 붙인다. 그 노드가 없는 케이스에서는
  간격이 저장되지 않고 relatedness 생성 경로에서 빈 간격으로 만들어진다.
- **E.i.6b**: XML에는 `d`가 유지되고 `import_sections/e_reaction.rs::read_e_i_6b`가
  parser 단계에서 `804`로 변환한다. 값의 코드 표현 변경이며 누락과 구별한다.
- **G.k.4.r.11.1 nullFlavor**: XML의 `value`에는 허용 nullFlavor가 있고 그 아래 빈 `originalText`도 있다.
  importer의 `first_text`가 자식의 빈 내용을 `Some("")`로 읽어 부모 nullFlavor와 함께 input-contract에 넘긴다.
  실제 parser 오류는 `INPUT.VALUE_NULLFLAVOR.CONFLICT`다. 허용 목록 때문에 거절된 것이 아니다.
- **FDA.D.1 NA**: XML의 환자 `name`에 `nullFlavor="NA"`가 있다.
  importer는 batch/message receiver가 `TARGET`인 이 조직 간 전달에서 ICH를 선택한다.
  실제 parser 오류는 `ICH.D.1.NULLFLAVOR.ALLOWED`다. 업로드의 `format` 항목은 현재 reader가 소비하지 않는다.

### 출력 오류 25건은 누락에 포함하지 않는다

이 시나리오들은 export HTTP 500으로 끝나 import가 호출되지 않았다.

- 중간 이름만 있는 경우 또는 MFDS 자격만 있는 경우: 값 기록에 필요한 부모/위치 노드를 만들지 않는 경로가 있다.
- **날짜 6건**: 실행 중인 격리 DB를 직접 조회했다. 날짜 열에는 실제 날짜가, raw 열에는 빈 문자열이 함께 있었다.
  `lib-core/src/serde/flex_date.rs::deserialize_optional_partial_ts_raw`는 완전한 날짜 입력 시 이전 raw 값을 지우려고
  빈 문자열을 만든다. 반면 `xml/src/export/field.rs::stored_date`는 `(Some(date), Some(""))`도 충돌로 거절한다.
  원본 dosage ID를 실패 증거와 대조했으며 전체 6건을 확인했다.
- **G.k.10.r 3건**: 저장값은 문자열 배열 `["1"]`인데 exporter의 `write_g_k_10_r`는
  각 원소에 `value_code` 속성이 있는 객체 배열을 요구한다.

### 근거와 한계

서버의 `/home/opc/e2br3/runtime-dev/xml-field-stages-20261122/`에 다음을 보관했다.

- `classification.json`: 124건 전체의 단계, 관측값, 관련 함수와 개별 근거 경로.
- `scenarios.json`: 새 전체 실행 원본 결과.
- 각 시나리오 디렉터리의 `evidence.json`, 출력 성공 시 `export.xml`.
- `stored-dates.json`: 날짜 오류 6건의 실제 격리 DB 열 값.
- `classify.py`: 출력 XML과 원본/대상/파싱 증거를 assertion으로 확인한 일회성 분류 스크립트.

실행 로그는 `/tmp/xml-field-stages-run-20261122.log`다.
이번에는 단계별 진단과 분류만 추가했다. serializer/parser와 저장 동작은 수정하지 않았고,
새 회귀 검사는 여전히 124건 때문에 실패한다. XSD 적합성이나 모든 값 조합의 보장을 뜻하지 않는다.
