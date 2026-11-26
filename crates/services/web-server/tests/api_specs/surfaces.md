# API 범위와 프런트 서버 경계

## 백엔드 API

등록된 operation과 검사 범위는 [coverage.tsv](coverage.tsv), [보호 경로 snapshot](../authz/protected_route_inventory.snapshot), [operation/DTO manifest](api-contract-manifest.json)에서 관리한다. 등록 개수는 테스트 통과 수가 아니다.

- `/api`: 케이스·검증·제출·설정 등 제품 API.
- `/auth/v1`: 로그인·세션 관련 API.
- `/internal/submissions`: 제출 ACK callback과 재조회 API. 인증·권한 조건은 [제출 명세](submission.md)를 따른다.
- `/health`: 서버 상태 확인.

validator/XML crate는 web-server 내부에서 호출되는 라이브러리이며 독립 HTTP 서버가 아니다. 프레임워크가 제공하는 HEAD/OPTIONS와 페이지 렌더링은 명시 등록 API 분모에서 제외한다.

## 프런트 서버 경계

Next.js에 별도 API Route Handler는 없다. `/api`와 `/auth` 요청은 `next.config.js`의 rewrite를 통해 `API_PROXY_TARGET`의 백엔드로 전달한다. 검증은 화면 라우트의 기관을 명시해 일반 validation API로 요청하고, 제출은 제출 API 내부에서 검증한다. 테스트에도 별도 검증 상태 변경이나 서버 전용 검증 토큰이 필요하지 않다.

일반 검증은 검증 결과를 반환하고 캐시를 갱신한다. 검토 상태 변경은 실제 화면과 같은 `/api/cases/{id}/review/toggle`을 사용한다. 검증과 검토 상태 변경은 별개다.

## 외부 전송 서비스

AS2 submitter는 [배포 overlay](../../../../../deploy/ec2/docker-compose.as2.yml)가 참조하는 별도 저장소·프로세스다. E2BR3가 호출하는 submit/status와 수신하는 ACK/reconcile 경계는 [submission.md](submission.md)에 명세화한다. AS2 서비스의 관리·수신 경로는 이 저장소의 API 개수에 합치지 않는다.

## 완전성의 판정 범위

라우트·snapshot·coverage·manifest의 집합을 대조해 명세 누락을 확인한다. 요청·응답 타입은 manifest, 동적 JSON·저장·오류·권한·감사는 API군별 문서, 실제 assertion과 미검사 범위는 coverage 및 lifecycle 행렬에 연결한다. 등록과 정적 검사만으로 실제 서버 실행 성공을 주장하지 않는다.
