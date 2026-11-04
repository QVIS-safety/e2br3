# E2B(R3) Safety Case Management

ICSR 작성·검증·XML 입출력, Presave, 검토·전자서명, 감사 이력 및 기관 전송을 다루는 프로그램이다. 현재 제품 범위는 ICH 공통 형식과 FDA·MFDS 지역 규칙이며, ICH 화면은 XML 내보내기용이다.

## 완료 범위와 남은 작업 — 2026-11-04

**판정: Case 작성부터 제출 직전까지의 핵심 기능은 구현했고 실제 PostgreSQL·API·브라우저로 주요 경로를 검증했다. 기관의 최종 접수와 운영 준비까지 완료한 상태는 아니다.** 아래의 ‘검증 완료’는 명시한 환경·시나리오에 한정한다. 로컬 검증, 원격 CI 성공, staging 배포, 규제기관 수락은 각각 별도 증거가 필요하다.

확인 기준은 backend `90059796` 이후 API 명세 보정 `7d288a34`, frontend `f093188d`, AS2 submitter `345f4b2`다. 이후 변경의 완료 여부는 해당 커밋과 실행 결과로 갱신한다. 개발 작업 상세 원장은 Oracle 작업 공간의 `/home/opc/e2br3/CURRENT-STATUS.md`에 유지하며, 이 문서는 인계용 요약이다.

### 구현·검증한 범위

| 영역 | 완료한 내용 | 확인 범위 / 근거 |
| --- | --- | --- |
| Case 편집·저장 | 주요 11개 편집 섹션의 생성·수정·삭제·복원, 빈값/NullFlavor 처리, 삭제된 행의 오래된 저장 요청 거절 및 입력 보존 | 실제 API·DB 상태 검사 214요청/474판정 통과. 별도 삭제·동시성 회귀 및 Firefox 저장·새로고침 확인. 모든 동시성 조합을 보장하지는 않음 |
| Presave 6종 | Sender·Receiver·Reporter·Product·Study·Narrative 선택·적용·저장, 출처 보존, 원본 후속 변경과 Case 값 분리, 참조 중 원본 삭제 거절 | 11/04 실제 Firefox에서 6종 적용→저장→API/DB 재조회→새로고침→재로그인 확인. 반복 행 저장을 단일 batch transaction으로 처리 |
| Validation | ICH/FDA/MFDS 규칙, 입력 계약, 필드 오류·배너·기관별 집계, 규칙/사전 변경 시 검증 캐시 무효화 | [규칙·실행 검사](crates/libs/validator/README.md). 구현된 규칙의 테스트 연결은 공식 규정 전체 충족이나 기관 수락을 뜻하지 않음 |
| XML 입출력 | 기관별 XML 생성·XSD 검사 경로, import와 Case/감사/import history의 원자적 저장, 단일/묶음 export | 실제 XML 왕복·격리 DB 회귀와 로컬 전송 XML 대조 이력 보유. 최신 배포본의 공식 기관 검증은 별도 필요 |
| 권한·Workflow | 조직/역할/Case 범위 권한, 세션 회수, 검토·상태 전환, 중요 작업의 비밀번호 재확인과 서명 연결 | 권한·교차 조직·거절 시 불변성 회귀 보유. 전자서명 저장 자체의 추가 보호는 아래 잔여 작업 |
| Audit trail | 변경 전후 값·행위자·조직·사유·서명 연결, v2 hash chain, 범위별 검증, 일반 앱 계정의 감사 직접 변경 차단 | 실제 데이터 검증 및 Case/Presave 상태·감사 대조 완료. 독립 저장소 등록은 미완료. [감사 체크포인트와 기존 이력 처리](deploy/ec2/AUDIT-CHECKPOINT.md) |
| AS2 전송 기반 | 서명·암호화·MIC·MDN, 송신 전 영속 저장, 동일 요청 중복 송신 차단, 불명확한 전송 상태 보존, callback 재시도 | 실제 Oracle/OpenAS2 통신으로 검증. 기관 서버를 대신한 로컬 시험이며, 최종 기관 ACK 해석은 미구현. [Submitter 계약](https://github.com/donihyun/as2-submitter/blob/345f4b2fbcde8148e1c93a5a3fccb432a6a32d5d/README.md) |
| CI·배포·DB 변경 | `dev` CI 통과 이미지 게시, staging 자동 갱신, migration 이력/체크섬 검사, 변경 전 백업, 실패 시 배포 차단 | 10/31 EC2 자동 실행 확인. 이후 최신 실행 SHA는 재확인 필요. SQL 적용 뒤 앱 이미지를 되돌려도 SQL은 자동 rollback되지 않음 |

실제 검증 증거는 작업 공간의 `save-readback-fix-20261103/`, `save-readback-audit-resilience-20261103/`, `presubmit-state-check-20261103/`, `presave-import-resilience-20261104/`에 보존한다. 이 자료에는 실행 환경별 기록이 포함되므로 Git에 원본 업무 데이터·인증정보를 넣지 않는다.

### 먼저 끝낼 작업

다음 순서는 현재 코드와 증거에 따른 인계 우선순위다. ‘추가 검증’은 결함 확정이나 미구현과 구분한다.

| 우선순위 | 남은 작업 / 현재 상태 | 완료 조건 |
| --- | --- | --- |
| 납품 전 1 | **최신 배포본 확정.** backend `90059796` CI는 batch API 5개 검사 목록 누락으로 실패했다. `7d288a34`에서 3개 명세/목록을 보정했고 로컬 명세 검사 441 operations/376 types를 통과했다. 후속 CI·EC2 실행 SHA 확인 필요 | 성공한 BE/FE CI의 SHA와 실제 컨테이너 SHA 일치, migration 성공, 도메인에서 로그인→Case 생성→Presave 적용→일괄 저장→재조회→XML export→감사 조회 통과. 11/04 AWS 세션 만료로 EC2 직접 재확인은 차단됨 |
| 납품 전 2 | **대상 환경의 실제 사전·기관 설정 확인.** 새 staging DB에 필요한 사전이 모두 설치됐다는 증거가 없음. Oracle의 사전 검증을 staging 결과로 대체할 수 없음 | 사용 범위에 맞는 MedDRA 언어/버전·WHODrug·MFDS 품목/성분·단위/코드 목록 및 필요한 EDQM 릴리스를 실제 적재·활성화하고, 검색→선택→저장→XML의 코드/버전을 대조. 라이선스·원본이 없는 항목은 미지원/차단 범위를 명시 |
| 기관 연동 전 1 | **FDA/MFDS 연결 개통.** 등록된 송신 ID·기관별 연결 설정·인증서가 없어 EC2 submitter는 마지막 직접 확인일인 10/31에 비활성이었음 | 보고자/기관 식별자와 AS2 전송 ID를 구분해 등록하고, 인증서·서명키·callback 인증·수신 HTTPS·기관 routing을 설정. FDA 공식 test 경로 및 MFDS가 제공한 실제 연계 규격으로 왕복 시험. 로컬 MFDS AS2 성공만으로 실제 MFDS 연동 완료를 주장하지 않음 |
| 기관 연동 전 2 | **기관 ACK parser와 최종 결과 연결 — 미구현.** 현재 파일 ACK는 `ACK_FILE_UNVERIFIED`로 보존하며 자동 접수 판정을 하지 않음 | 공식 정상/거절 ACK로 메시지·개별 ICSR 식별자와 결과를 정확히 연결하고 DB/UI/원문 이력을 일치시킴. 중복·순서 역전·잘못된 참조·callback 장애·재시작·지연 응답을 검증. MDN 또는 HTTP 성공을 최종 기관 수락으로 표시하지 않음 |
| 기관 연동 전 3 | **기관별 보고 시나리오 최종 검증 — 증거 부족.** XSD/자체 validator 통과 이후 단계가 남음 | FDA/MFDS별 적용 가능한 임상/시판후·국내/국외, 최초/추적/정정/무효화, 보고 식별자 유지, 날짜/NullFlavor, 반복 약물·반응, 첨부, 사전 버전 시나리오를 공식 규칙과 대조. 기관 테스트의 실제 접수/거절 결과 및 재제출 이력 확보 |
| 운영 전 1 | **전자서명 저장 보호 — 확인된 보완점.** 비밀번호 재확인은 있으나 signature row 변경 차단과 잘못된 서명 context 거절이 부족 | `e_signatures`의 변경·삭제 제한 및 Case/조직 범위 정책 보강, malformed context를 NULL로 삼키지 않고 거절, signer/Case/action 불일치·재사용·잘못된 비밀번호·후속 실패 rollback 회귀. 재인증 실패 제한/보안 이벤트와 최종 화면 확인 |
| 운영 전 2 | **독립 감사 보관 — 구현 후 운영 연결 미완료.** S3 Object Lock checkpoint 도구는 있으나 실제 등록/스케줄/경보가 없음 | 별도 관리 주체·최소 권한 writer·승인된 보존 기간으로 독립 저장소를 준비하고, 실제 감사 추가 사이의 연속 checkpoint·버전·보존·복원/삭제 탐지 검증 후 정기 실행/실패 경보 연결. 과거 감사 정리의 승인·원본 증거를 새 기준선과 함께 보존 |
| 운영 전 3 | **백업·복구·장애 운영 — 대상 배포 환경 증거 부족.** EC2 새 DB는 마지막 확인 시 로컬 디스크와 로컬 백업에 의존 | DB·업로드·원문 XML/ACK·submitter 상태·필요 키를 보호된 외부 저장소에 백업. 승인된 복구 목표로 새 호스트 복원, count/hash·감사 검증, 기존 송신기 차단 및 불명확한 전송의 중복 송신 방지 확인. 디스크/DB/인증서 만료/ACK 지연 경보와 담당자 확정 |
| 운영 전 4 | **정식 인수·운영 검증 — 미완료.** 선별 회귀와 API 문서가 전체 제품 인수 증거를 대신하지 않음 | 릴리스 SHA·규격/사전 버전 고정, 요구사항→코드→테스트→실행 증거 연결, 업무 담당자의 UAT 승인, 권한·전자서명·감사·보존/복구·장애 대응 절차 인계. 동시 편집·응답 유실·서버/DB 재시작·실제 데이터량 성능의 허용 기준과 결과 확보 |

전자서명 보완 근거는 [schema/서명 context](db/bootstrap/01-safetydb-schema.sql), [서명 모델](crates/libs/lib-core/src/model/e_signature.rs), [감사·서명 trigger](db/bootstrap/10-triggers.sql)다. 감사 hash 검증이 통과해도 독립적인 과거 진본성이나 전자서명 통제 완료까지 증명하지 않는다. Oracle의 기존 이력 정리 내역과 한계는 [감사 runbook](deploy/ec2/AUDIT-CHECKPOINT.md#historical-link-failures-and-authorized-removal-2026-11-02)을 따른다.

### 후속 점검과 유지보수

- 제출 예약이 commit된 뒤 통신이 끊기는 경로의 `dispatch_unknown` 복구, 일반 동시 편집의 lost update, 실패한 요청의 실제 commit 여부는 추가 장애 검증 대상이다. 확인 없이 실패로 간주해 같은 보고를 새 ID로 보내지 않는다.
- 최신 export 성공 경로는 history 저장 오류를 전파한다. 과거 조사에 있던 ‘history 실패를 경고로만 남기고 export 성공’ 지적을 그대로 재사용하지 않는다. 실패 이력 저장 실패·운영 경보 등 남은 경로는 현재 코드로 재현해 판단한다.
- 프런트 API 위키는 고정 backend snapshot을 사용한다. 새 batch API 등 계약 변경 시 위키 원본·API 명세·실행 증거를 함께 갱신한다. [API 명세](crates/services/web-server/tests/api_specs/README.md)의 `NOT_RUN`/coverage 표시는 실제 결과 없이 PASS로 바꾸지 않는다.
- 보존된 9~10월 dirty worktree/stash는 후속 이관·재작성·폐기된 검사기가 섞여 있다. 필요한 차이만 검토하고 전체를 다시 merge하지 않는다.
- [9월 준비도 감사](docs/production-readiness-goysop-audit.md)는 과거 기록이다. 이후 bootstrap 차단, 권한/세션, migration, 감사 수정이 반영됐으므로 그 문서의 열린 항목을 현재 결함 목록으로 복제하지 않는다.

### 규격 확인 기준

규격 문서·사전·기관별 규칙의 버전과 적용 범위를 릴리스마다 고정하고 변경 영향을 검토한다. 자체 규칙 목록의 개수만으로 E2B(R3) 적합성 전체를 판단하지 않는다. [ICH의 IG·schema·code list·Q&A](https://admin.ich.org/node/348)와 [FDA의 E2B(R3) 및 지역 구현 문서](https://www.fda.gov/regulatory-information/search-fda-guidance-documents/e2br3-electronic-transmission-individual-case-safety-reports-implementation-guide-data-elements-and)를 기준으로 삼는다.

FDA는 [E2B(R3) 시험 검증](https://www.fda.gov/drugs/fda-adverse-event-monitoring-system-aems/fda-adverse-event-monitoring-system-aems-e2br3-data-standards)과 [기관/보고 유형별 ACK](https://www.fda.gov/industry/getting-started-esg-nextgen/submission-acknowledgements)를 구분한다. ACK 단계는 기관·보고 유형에 따라 달라지므로 모든 전송에 같은 단계 수를 가정하지 않는다. MFDS 연결은 [의약품안전나라 전자보고 자료·인증서 공지](https://nedrug.mfds.go.kr/minwon/adverse)와 실제 제공받은 연계 명세로 확인한다. 이 문서는 법적 적합성 인증서나 기관 접수 증명서가 아니다.

## Local Development Setup

This guide is for developers who need read-only access to the private repositories and want to run the app locally.

## Repository Access

The repositories are owned by the `QVIS-safety` GitHub organization.

Ask an organization owner to grant `Read` access to both repositories:

- `QVIS-safety/e2br3`
- `QVIS-safety/E2BR3-frontend`

`Read` access is enough to clone and run locally. It does not allow pushing changes to the repositories.

## Prerequisites

Install:

- Git
- Docker Desktop
- Node.js 20 or later, Node.js 22 recommended
- npm

## Clone Repositories

Using HTTPS:

```sh
mkdir -p ~/projects/e2br3
cd ~/projects/e2br3

git clone --branch dev https://github.com/QVIS-safety/e2br3.git
git clone --branch dev https://github.com/QVIS-safety/E2BR3-frontend.git
```

Using SSH:

```sh
mkdir -p ~/projects/e2br3
cd ~/projects/e2br3

git clone --branch dev git@github.com:QVIS-safety/e2br3.git
git clone --branch dev git@github.com:QVIS-safety/E2BR3-frontend.git
```

## Run Backend and Database

The backend and PostgreSQL database run with Docker Compose.

```sh
cd ~/projects/e2br3/e2br3
docker compose up -d --build
```

The backend runs at:

```text
http://127.0.0.1:8080
```

Check that the containers are running:

```sh
docker compose ps
```

Check the backend response:

```sh
curl http://127.0.0.1:8080/api/app/branding
```

For normal local Docker development, no extra backend env file setup is required. The local development values are already defined in `docker-compose.yml`.

## Frontend Environment

Create the frontend local env file:

```sh
cd ~/projects/e2br3/E2BR3-frontend

cat > .env.local <<'EOF'
API_PROXY_TARGET=http://127.0.0.1:8080
NEXT_PUBLIC_API_BASE_URL=
NEXT_PUBLIC_APP_NAME=QVIS Safety
NEXT_PUBLIC_APP_SHORT_NAME=QVIS Safety
E2BR3_VALIDATOR_TOKEN=validator-secret
EOF
```

Do not use production secrets, RDS URLs, or `.env.prod` values for local development.

## Run Frontend

The frontend is a Next.js app and should be run directly with npm.

```sh
cd ~/projects/e2br3/E2BR3-frontend
npm ci
npm run dev
```

The frontend runs at:

```text
http://localhost:3000
```

The frontend proxies API requests to the local backend through:

```text
API_PROXY_TARGET=http://127.0.0.1:8080
```

## Login

Development accounts:

```text
hdh4063@gmail.com / welcome
demo.cro.admin@example.com / welcome
demo.company.admin@example.com / welcome
```

These are development/demo credentials only.

## Useful Commands

Backend logs:

```sh
cd ~/projects/e2br3/e2br3
docker compose logs -f app
```

Database logs:

```sh
cd ~/projects/e2br3/e2br3
docker compose logs -f postgres
```

Stop backend and database:

```sh
cd ~/projects/e2br3/e2br3
docker compose down
```

Reset the local database completely:

```sh
cd ~/projects/e2br3/e2br3
docker compose down -v
docker compose up -d --build
```

Update local repositories:

```sh
cd ~/projects/e2br3/e2br3
git pull --ff-only origin dev

cd ~/projects/e2br3/E2BR3-frontend
git pull --ff-only origin dev
```

## EC2 Note

현재 staging은 backend/frontend `dev` CI가 게시한 이미지를 `e2br3-dev-update.timer`가 확인해 갱신한다. [staging 배포 안내](deploy/ec2/README.md)와 [업데이트 구현](deploy/ec2/update-dev.py)을 참고한다. CI 성공 여부와 실제 실행 버전은 별도로 확인한다. 아래 `main`/수동 배포 안내는 해당 배포 경로를 운영할 때의 절차이며 staging `dev` 갱신 절차를 대신하지 않는다.

EC2 deployment uses the backend repository and the production deployment files under `deploy/ec2`.

After the repository transfer to `QVIS-safety`, the EC2 clone should point to:

```sh
cd /opt/e2br3
git remote set-url origin https://github.com/QVIS-safety/e2br3.git
git pull --ff-only origin main
```

Manual deployment requires an image reference:

```sh
cd /opt/e2br3
IMAGE_REF=ghcr.io/QVIS-safety/e2br3-web-server:TAG_OR_SHA ./deploy.sh
```

For CI/CD deployment, confirm that the `QVIS-safety/e2br3` repository has the required GitHub Actions secrets:

```text
AWS_ROLE_TO_ASSUME
AWS_REGION
AWS_SSM_TARGET
```

Also confirm that EC2 can pull the GHCR image:

```text
ghcr.io/QVIS-safety/e2br3-web-server:TAG_OR_SHA
```
