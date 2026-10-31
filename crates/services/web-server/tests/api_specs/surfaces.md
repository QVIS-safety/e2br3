# API 범위와 프런트 서버 경계

**검토 기준: 2026-10-31 / 실행 NOT_RUN.** 백엔드 기준은 `18f9b0e6e1f54cde3b88fb6e7c4e0ab8f0949ed5` 이후 이 변경의 소스다. 프런트는 `QVIS-safety/e2br3-frontend`의 `b9c8f45718704b561b034db8f21d990c56f88133`다. 아래 개수는 명시적으로 등록한 method/path 수이며 테스트 통과 수가 아니다.

| 서버·경계 | 등록 | 정본·명세 |
|---|---:|---|
| E2BR3 web-server `/api` | 429 | [라우트](../../src/web/rest/routes), [보호 경로 snapshot](../authz/protected_route_inventory.snapshot), [operation 색인](coverage.tsv) |
| 같은 서버 `/auth/v1` | 3 | [인증 라우트](../../src/web/routes_login.rs), [인증 명세](administration.md#post-auth-v1-login) |
| 같은 서버 `/internal` | 3 | [제출 내부 라우트](../../src/web/rest/routes/submissions.rs), [제출 명세](submission.md) |
| 같은 서버 `/health` | 1 | [app router](../../src/lib.rs), [health 명세](administration.md#get-health) |
| Next.js 자체 Route Handler | 1 | 아래 validator proxy POST |
| **두 저장소의 명시적 operation 합계** | **437** | 백엔드 436개와 프런트 자체 1개를 구분. 프록시를 통해 접근한 백엔드 경로는 중복 계산하지 않음 |

백엔드 workspace의 서버 crate는 `web-server` 하나다. validator/XML crate는 이 서버 안에서 호출되는 라이브러리이며 독립 HTTP 서버로 계산하지 않는다. `app`의 `route.*` 파일을 검색한 프런트 Route Handler는 아래 하나이며, `pages/api`와 `src` 트리는 해당 checkout에 없다. 프레임워크가 자동 제공하는 HEAD/OPTIONS, 정적 자산·페이지 렌더링·fallback은 명시 등록 API 분모에서 제외한다. 제외 사유가 존재하는 것이 해당 기능의 실행 검증을 의미하지는 않는다.

## 프런트 validator proxy

`POST /internal/validator/mark-validated`는 Next.js 서버의 [route.ts](https://github.com/QVIS-safety/e2br3-frontend/blob/b9c8f45718704b561b034db8f21d990c56f88133/app/internal/validator/mark-validated/route.ts)에 등록된다. 백엔드의 `/internal/submissions/...`와 별개 경로다.

| 항목 | 실제 소스 계약 |
|---|---|
| 요청 | query `caseId`를 먼저 읽고, 없으면 JSON body의 `caseId`, 다음 `case_id`를 읽음. backend 호출 본문은 없음 |
| ID 검증 | Next handler는 truthy 여부만 검사하고 UUID 검증·URL 인코딩을 직접 수행하지 않음. 실제 UUID extractor는 backend 경로에서 적용. body의 숫자/객체를 정상 UUID 입력으로 보장하지 않음 |
| 인증 | 수신 cookie를 그대로 backend에 전달하고 Next 서버의 `E2BR3_VALIDATOR_TOKEN`을 `x-validator-token`으로 추가. 서버 토큰이 사용자 인증·Case 권한을 대체하지 않음 |
| 목적지 | 현재 구현은 `API_PROXY_TARGET`, 다음 `NEXT_PUBLIC_API_BASE_URL`, 다음 `http://localhost:8080` 순서. 이는 기존 구현의 fallback이며 이번 작업이 추가한 동작이 아님 |
| Next 자체 오류 | 토큰 설정 부재: 500 JSON `{error:{message}}`. query/body에서 ID를 얻지 못함: 400 JSON `{error:{message:"caseId is required"}}`. 잘못된 JSON은 ID 부재로 처리 |
| backend 전달 | `POST /api/cases/{caseId}/validator/mark-validated`; backend status와 response text를 전달. `content-type`은 backend 값이 없으면 현재 코드상 `application/json`, `cache-control:no-store` |
| 전송 실패 | fetch 예외는 이 handler가 별도의 성공 응답으로 바꾸지 않음. response text 읽기 실패를 빈 문자열로 바꾸는 기존 분기는 존재하며 전달 본문 무결성을 보장하지 않음 |
| 상태·감사 | Next 자체는 DB를 변경하지 않음. backend의 `case.validate` 권한·Case scope·lifecycle 검사, 실제 validation cache 갱신 후 issue 0개일 때 `status=validated` 및 시스템 validation 사유의 감사 기록 |
| backend 오류 | backend token 미설정/불일치 400, UUID 형식 오류 extractor 거절, 사용자 인증·권한 부족 403, Case 부재/가시성 없음 404, validation issue 남음 400. backend 상세 계약은 [Case 명세](cases.md)와 [handler](../../src/web/rest/case_rest.rs) |
| 직접 테스트 근거 | [validator-proxy.integration.test.ts](https://github.com/QVIS-safety/e2br3-frontend/blob/b9c8f45718704b561b034db8f21d990c56f88133/__tests__/integration/validator-proxy.integration.test.ts)의 live 검사 4개: ID 부재 400, body 두 표기의 실제 cookie/token 전달·validation 400/body/header, query 우선 404, 미인증 403. **asserted / NOT_RUN** |

### 실제 integration 실행 조건

기본 unit Jest에서 제외하고 `jest.integration.config.js`의 Node 환경에 등록했다. 브라우저용 `jest.setup.ts`가 URL을 대체하지 않도록 integration에서는 setup을 비웠다. 기존 workflow 2개와 신규 proxy 4개가 같은 integration 설정에 있으며 아래 경로 선택으로 proxy만 실행할 수 있다. 이번 작업에서는 실행하지 않았다.

```sh
npm run test:integration -- --runTestsByPath __tests__/integration/validator-proxy.integration.test.ts
```

| 필수 입력/환경 | 의미 |
|---|---|
| `VALIDATOR_PROXY_TEST_FRONTEND_URL` | 실제 실행 중인 Next의 HTTP(S) origin |
| `VALIDATOR_PROXY_TEST_BACKEND_URL` | 실제 backend origin; Next의 `API_PROXY_TARGET`과 같아야 함 |
| `VALIDATOR_PROXY_TEST_COOKIE` | 해당 Case를 읽고 validate할 권한이 있는 실제 유효 세션 cookie |
| `E2BR3_VALIDATOR_TOKEN` | test runner·backend·이미 시작한 Next 프로세스에 같은 값 설정 |
| `VALIDATOR_PROXY_TEST_DRAFT_CASE_ID` | validation issue가 남아 있는 전용 실제 draft Case UUID |

필수 입력 누락, 다른 서버/토큰, 부적절한 Case는 실패한다. skip/mock/fallback으로 대체하지 않는다. 직접 backend preflight와 proxy 요청은 실제 validation cache를 갱신하므로 공유 업무 Case를 사용하지 않는다. 이 검사는 draft 상태 보존을 확인한다. 정상 검증 완료 200, 서버 토큰 부재 500, upstream 5xx·전송 실패·본문 읽기 실패는 남은 공백이다. 현재 자동 CI job은 없으며 기존 integration 명령에 실행 진입점만 연결했다. 서버·DB·인증을 갖춘 환경의 실행 로그가 생기기 전에는 PASS 증거가 아니다.

## 프록시·리다이렉트·외부 전송 서비스

[next.config.js](https://github.com/QVIS-safety/e2br3-frontend/blob/b9c8f45718704b561b034db8f21d990c56f88133/next.config.js)의 `/api/:path*`, `/auth/:path*` rewrite는 동일한 backend operation을 전달한다. 이 rewrite의 목적지 선택은 위 Route Handler와 다르다: `API_PROXY_TARGET`, 또는 Vercel/Netlify 환경이면 설정된 staging origin, 그 외 `http://127.0.0.1:8080`이다. 각 환경에서 동일 backend를 가리키는지는 실행하지 않았으므로 확인되지 않았다.

[middleware.ts](https://github.com/QVIS-safety/e2br3-frontend/blob/b9c8f45718704b561b034db8f21d990c56f88133/middleware.ts)는 기존 `/admin`, `/cases`, `/docs`, `/forms`, `/import`, `/submission` 페이지 URL을 `/ICH` 아래로 307 리다이렉트한다. `/api`·`/auth`·위 `/internal` API는 matcher에 없다. Host 부재/잘못된 Host는 오류이며 임의 기본 Host를 만들지 않는다.

AS2 submitter는 [배포 overlay](../../../../../deploy/ec2/docker-compose.as2.yml)가 참조하는 **별도 저장소·프로세스**다. 이 두 저장소의 API 개수에는 AS2 submitter의 관리/수신 경로를 합치지 않는다. E2BR3가 호출하는 submit/status와 수신하는 ACK/reconcile 경계는 [submission.md](submission.md)에 명세화하며, transport 내부 API 전체의 명세·기관 수용·암호 서명·실제 네트워크 전달 성공을 이 자료로 인증하지 않는다. E2BR3의 내부 callback 3개는 이미 백엔드 436개 안에 포함된다.

## 완전성의 판정 범위

등록 누락 여부는 백엔드 router 436개, snapshot 429개, `coverage.tsv`, [operation/DTO manifest](api-contract-manifest.json)의 집합을 대조한다. 프런트 자체 1개는 위에 별도 기록한다. 요청·응답 타입/serde 필드는 manifest, 동적 JSON·저장·오류·권한·감사는 API군별 문서, 실제 assertion과 미검사 범위는 coverage 및 lifecycle 행렬에 연결한다.

이 자료는 **등록된 API가 명세에서 빠졌는지**와 **명세의 어떤 부분에 검사 코드가 있는지**를 확인하도록 구성한다. 모든 가능한 입력·동시성·외부 장애가 검증됐다는 뜻은 아니다. 실행 증거는 전부 NOT_RUN이며 같은 commit의 CI 결과가 추가되어야 실제 통과를 입증할 수 있다.
