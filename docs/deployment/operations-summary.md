# 배포·운영 상태 요약

## 현재까지 확인된 상태

- 백엔드와 프론트는 EC2에서 직접 Docker 빌드하는 방식으로 배포를 시도했다.
- 8GB 디스크와 2GB RAM 환경에서 Rust 백엔드 빌드가 실패했다.
  - 병렬 빌드: Rust 컴파일러가 `SIGKILL`로 종료됐다.
  - 단일 작업 빌드: Docker 중간 레이어가 디스크를 모두 사용해 `No space left on device`가 발생했다.
- 이후 빌드 중 EC2가 재부팅되면서 빌드 컨테이너가 `exit 255`로 종료됐다.
- 완성된 운영 이미지나 실행 중인 백·프론트 서비스는 확인되지 않았다.
- 따라서 EC2에서 직접 빌드하지 않고 GitHub Actions에서 이미지를 빌드해 GHCR에 게시한 뒤 EC2가 이미지만 pull하는 방식으로 전환한다.

## 목표 환경

```text
push dev  -> CI -> dev 이미지 -> dev EC2
push main -> CI -> 운영 이미지 -> main EC2

각 EC2 -> RDS PostgreSQL
       -> private S3
```

- main EC2는 운영 배포용, dev EC2는 테스트용으로 분리한다.
- RDS는 같은 인스턴스를 사용할 수 있지만 `app_main`, `app_dev`처럼 데이터베이스를 분리한다.
- S3는 private으로 유지하고 최소한 `main/`, `dev/` prefix를 분리한다. 가능하면 버킷도 분리한다.
- GitHub Actions에는 AWS access key를 저장하지 않고 OIDC 역할을 사용한다.

## 저장소에 이미 있는 배포 구성

- `deploy/ec2/docker-compose.prod.yml`: RDS를 사용하는 런타임 Compose
- `deploy/ec2/deploy.sh`: EC2에서 GHCR 이미지를 pull하고 기동하는 배포 스크립트
- `.github/workflows/cd.yml`: GitHub Actions에서 Docker 이미지를 빌드해 GHCR에 게시하고 AWS SSM으로 EC2에 배포하는 흐름
- 운영 환경 변수는 EC2의 `/opt/e2br3/.env.prod`에 두도록 되어 있다.

## CI에서 확인된 문제

- CI push 트리거는 `main`, `dev`지만 PR 트리거는 `main`만이라 dev 대상 PR에는 CI가 실행되지 않는다.
- PR 이벤트에서 frontend registry를 항상 `main` 브랜치로 checkout한다. dev 대상 PR은 `github.base_ref` 기준으로 checkout해야 한다.
- `scripts/test-isolated-db.sh --workspace`가 전체 workspace 테스트를 한 번에 실행한다. 오래된 테스트 하나가 전체 CI를 막을 수 있다.
- XML fixture와 schema를 과거 커밋에서 별도 checkout하는 임시 우회가 남아 있다.
- CI 최신 실패 로그는 로컬에 `gh` CLI가 설치되어 있지 않아 아직 확인하지 못했다. 실제 실패 원인은 GitHub Actions 로그로 재확인해야 한다.

## CD/OIDC/SSM에서 확인된 문제

OIDC 설정 자체는 workflow에 있다.

- workflow 권한: `id-token: write`
- `aws-actions/configure-aws-credentials`
- `AWS_ROLE_TO_ASSUME`, `AWS_REGION`, `AWS_SSM_TARGET` 시크릿 사용

하지만 다음 권한은 서로 별개다.

1. GitHub OIDC 역할
   - `ssm:SendCommand`
   - `ssm:GetCommandInvocation`
   - 필요한 경우 `ssm:ListCommandInvocations`
2. EC2 Instance Profile
   - SSM Agent가 인스턴스를 관리할 수 있도록 `AmazonSSMManagedInstanceCore`
3. EC2 운영 상태
   - SSM Agent 실행 중
   - 인스턴스가 SSM에 `Online`으로 등록됨
   - 네트워크에서 SSM 엔드포인트 접근 가능

main EC2에 SSM 권한이 없으면 OIDC가 정상이어도 CD의 `aws ssm send-command` 단계에서 실패한다. 이 경우 현재처럼 직접 접속해 배포해야 한다.

## CD workflow의 추가 위험

- `push main`과 `workflow_run(CI 완료)`가 모두 CD를 실행할 수 있어 같은 커밋이 중복 배포될 수 있다.
- deploy job에 `always()`가 있어 이미지 publish가 실패해도 deploy가 진행되며, 이후 이미지가 없어 별도 실패할 수 있다.
- 운영 deploy 명령에 `RESET_DB=1`이 기본 포함돼 있다. 운영 자동 배포에서는 제거하고 migration만 실행해야 한다.
- dev와 main은 서로 다른 SSM target, `.env`, RDS DB, S3 prefix/bucket을 사용해야 한다.

## 권장 정리 순서

1. main EC2에 SSM Agent·Instance Profile·Online 상태를 확인하고 복구한다.
2. GitHub OIDC deploy role의 SSM 권한과 trust policy를 확인한다.
3. CI PR 트리거를 `main`, `dev`로 확장하고 frontend checkout 기준을 수정한다.
4. 오래된 전체 테스트를 필수 계약 테스트와 별도 legacy 테스트로 분리한다.
5. CD의 main 중복 트리거와 `always()`를 제거한다.
6. 운영 CD에서 `RESET_DB=1`을 제거한다.
7. dev/main용 SSM target과 RDS/S3 환경 변수를 분리한다.
8. 사용자 첨부파일은 현재 RDS base64 저장 구조이므로, S3까지 사용하려면 파일 메타데이터는 RDS에 저장하고 실제 파일은 S3에 저장하도록 백엔드 API와 스키마를 별도로 변경한다.


## Strict production lint 전환 시 필요한 설정

운영 Rust 대상은 CI에서 `unwrap`, `expect`, `panic!`, `todo!`,
`unimplemented!`, `unwrap_or*`를 deny(빌드 오류)한다. 테스트의 assertion은 별도 검사한다.
이 검사는 알려진 구문을 차단하며, 모든 형태의 의미적 fallback을 증명하지는 않는다.

기존 값에 의존하지 말고 실제 배포 설정을 명시해야 한다.

- 공통: `SERVICE_DB_MAX_CONNECTIONS`(양수), `SERVICE_BIND_ADDR`,
  `SERVICE_WEB_FOLDER`, `E2BR3_ENV`, `E2BR3_COOKIE_SECURE`(HTTPS 운영은 true),
  `E2BR3_PUBLIC_ORIGIN`(정확한 브라우저 origin), `E2BR3_APP_NAME`, `E2BR3_APP_SHORT_NAME`.
- AS2 사용 시: `AS2_SUBMITTER_URL`, `AS2_SUBMITTER_TOKEN`,
  `AS2_ACK_CALLBACK_URL`, `AS2_CALLBACK_TOKEN`, `AS2_SUBMITTER_TIMEOUT_SECS`(양수).
- ESG 사용 시: `FDA_ESG_ENABLED`, `FDA_ESG_BASE_URL`,
  `FDA_ESG_SUBMIT_PATH`, `FDA_ESG_TIMEOUT_SECS`(양수) 및 실제 서비스 인증 설정.
- 기능 flag도 `FDA_ESG_ENABLED`, `E2BR3_STRICT_SUBMISSION_CONFIG`,
  `SUBMISSION_RECONCILE_ENABLED`를 true/false로 명시한다.
- reconcile 활성화 시: `SUBMISSION_RECONCILE_INTERVAL_SECS`,
  `SUBMISSION_RECONCILE_LIMIT`(둘 다 양수), `SUBMISSION_RECONCILE_TOKEN`.

API 호출부도 명시적 값을 보내야 한다. 사용자 생성은 초기 비밀번호와 scope 배열,
권한 생성은 name/active/privileges, 신규 editor row는 sequenceNumber,
validation은 authority, reconcile은 limit이 필요하다.
권한 scope의 문자열 표현은 JSON 배열만 허용하며 CSV 재해석을 하지 않는다.
누락된 입력·잘못된 저장 데이터·지원하지 않는 PDF 글자는 성공값으로 대체하지 않는다.
이 변경만으로 실제 운영 배포 또는 전송 검증을 완료한 것은 아니다.
