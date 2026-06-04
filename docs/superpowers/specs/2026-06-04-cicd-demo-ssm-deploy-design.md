# CI/CD Demo SSM Deploy Cleanup Design

## Purpose

Clean up the current CI/CD structure by making deployment a first-class CD responsibility. The immediate target is a temporary demo EC2 environment, not durable production. Until database migrations are ready, each automatic `main` deployment will destructively refresh PostgreSQL, reload demo seed data, and reload required terminology releases.

## Current Problems

- CI owns validation and also has a Docker image build job, while CD separately publishes an image.
- The CD deploy job exists but is disabled.
- Deployment is not clearly tied to successful CI on `main`.
- The Docker image is named like a web-server image, but it contains both `/app/web-server` and `/app/terminology-loader`.
- Terminology loading has two operational paths: Docker Compose and a host-side wrapper that can fall back to `cargo run` on EC2.
- Because migrations are not complete, a fresh demo database needs bootstrap, seed data, and terminology reload after destructive reset.

## Goals

- Keep CI responsible for validation only.
- Make CD responsible for image publishing, automatic demo deployment, and manual terminology operations.
- Use GitHub OIDC to assume an AWS role.
- Use AWS SSM to run commands on EC2. Do not use SSH.
- Keep one runtime image per commit containing both the web server and terminology loader.
- Make destructive demo database reset explicit by using the existing `RESET_DB=1` and `INCLUDE_SEED=1` conventions.
- Automatically reload required terminology releases after demo DB reset.
- Keep licensed terminology source files outside git.

## Non-Goals

- Do not build separate images for `web-server` and `terminology-loader`.
- Do not solve the full test-suite tiering problem in this cleanup.
- Do not introduce real production migration handling in this pass.
- Do not load terminology automatically on non-reset deploys.
- Do not require Rust or `cargo run` on EC2 for production operations.

## Architecture

### CI Workflow

`.github/workflows/ci.yml` remains the validation workflow. It runs on pull requests and pushes to `main`.

It should contain:

- formatting checks
- clippy
- workspace tests
- existing release validation gates

It should not contain:

- Docker image build jobs
- image publishing
- deploy commands

The existing slow/duplicated test structure can be cleaned up in a later test-tiering plan.

### CD Workflow

`.github/workflows/cd.yml` owns delivery.

Automatic path:

1. Trigger from a successful `CI` workflow run on `main`.
2. Build the runtime image.
3. Push the image to GHCR with the commit SHA.
4. Verify the pushed SHA image can be pulled.
5. Assume the configured AWS role through GitHub OIDC.
6. Use AWS SSM to run the EC2 deploy command.
7. EC2 pulls and deploys the exact SHA image.

Manual path:

`workflow_dispatch` supports:

- manual deploy
- terminology dry run
- terminology load

Manual terminology inputs:

- `operation`: `deploy`, `terminology-dry-run`, or `terminology-load`
- `dictionary`: `meddra` or `whodrug`
- `input_path`: EC2 path to the licensed source file
- `version`
- `language`, defaulting to `en`
- `image_ref`, optional for manual operations

### Runtime Image

The repository keeps one Docker image per commit. It contains:

- `/app/web-server`
- `/app/terminology-loader`

The GHCR package may stay named `e2br3-web-server` for compatibility, but workflow and documentation language should call it the runtime image where practical.

`deploy/ec2/docker-compose.prod.yml` keeps two services:

- `app`, which runs the web server
- `terminology-loader`, which runs the loader as a one-off tool service with the same image

## Demo Deploy Data Flow

Automatic `main` deployment is a destructive demo refresh while migrations are incomplete.

EC2 deploy flow:

1. Pull the runtime image.
2. Stop the app container.
3. Run DB bootstrap with existing reset variables:

   ```sh
   RESET_DB=1 INCLUDE_SEED=1 ./deploy/ec2/init-rds.sh
   ```

4. Load required terminology releases from an EC2-side manifest.
5. Start the app container.
6. Run a health check.

The destructive reset depends on existing repository conventions:

- `RESET_DB=1` recreates the database through `deploy/ec2/init-rds.sh`.
- `INCLUDE_SEED=1` applies demo seed SQL.
- `SERVICE_DB_ROOT_URL` supplies the admin connection.
- `SERVICE_DB_URL` supplies the app connection.

This behavior is only for the temporary demo environment. When migrations are implemented, the reset/bootstrap/reload path should be replaced with migration execution and terminology should only be loaded through explicit operations.

## Terminology Loading

Terminology is operational data. Licensed dictionary source files remain outside git under paths such as:

```text
/opt/e2br3/terminology/incoming/meddra_28_1.zip
/opt/e2br3/terminology/incoming/whodrug_global_b3_mar_1_2026.zip
```

CD should read a manifest at `/opt/e2br3/terminology/terminology-manifest.prod`. The manifest lists the required releases for demo refresh:

```text
meddra /opt/e2br3/terminology/incoming/meddra_28_1.zip 28.1 en
whodrug /opt/e2br3/terminology/incoming/whodrug_global_b3_mar_1_2026.zip 2026.03 en
```

For each line, the deploy script runs Docker Compose:

```sh
docker compose --env-file .env.prod -f docker-compose.prod.yml run --rm terminology-loader \
  meddra \
  --input /terminology/incoming/meddra_28_1.zip \
  --version 28.1 \
  --language en
```

The same workflow also supports manual dry-run/load operations through SSM.

MedDRA and WHODrug releases are loaded one release at a time. For a dictionary and language, the newest loaded release becomes active and previous active rows are retired by the loader.

## EC2 Script Responsibilities

`deploy/ec2/deploy.sh` should be the single EC2 deploy entry point.

Responsibilities:

- validate required environment
- validate `IMAGE_REF`
- optionally sync schema files
- pull `IMAGE_REF`
- stop the app before destructive reset
- run `init-rds.sh` with `RESET_DB=1 INCLUDE_SEED=1` for demo refresh
- load terminology manifest when reset occurs
- update `.env.prod` with `IMAGE_REF`
- start the app service
- run health check
- prune unused images

`deploy/ec2/terminology-load.sh` should either be removed or rewritten to call Docker Compose only. It should not require a repository checkout, host Rust installation, or `cargo run` on EC2.

## Failure Behavior

- If image pull fails, deployment fails before touching the app or database.
- If DB reset/bootstrap fails, deployment fails and the app is not started against a half-reset database.
- If terminology loading fails, deployment fails because the demo environment is incomplete.
- If the health check fails, deployment fails.
- CD logs must show the image SHA and each terminology release attempted.

## Configuration

GitHub repository or environment secrets:

- `AWS_ROLE_TO_ASSUME`
- `AWS_REGION`
- `AWS_SSM_TARGET`

EC2 `.env.prod`:

- `IMAGE_REF`
- `SERVICE_DB_URL`
- `SERVICE_DB_ROOT_URL`
- `SERVICE_PWD_KEY`
- `SERVICE_TOKEN_KEY`
- application runtime config
- optional GHCR credentials if the host is not already logged in

## File Changes

- `.github/workflows/ci.yml`
  - remove Docker build job
  - keep validation only

- `.github/workflows/cd.yml`
  - trigger automatic deploy from successful CI on `main`
  - build and push runtime image
  - assume AWS role through OIDC
  - deploy through SSM
  - support manual deploy and terminology operations

- `deploy/ec2/deploy.sh`
  - add demo reset/bootstrap/terminology/health-check flow

- `deploy/ec2/terminology-manifest.prod.example`
  - document the required manifest format
  - keep real licensed file paths environment-owned on EC2

- `deploy/ec2/docker-compose.prod.yml`
  - keep one image with `app` and `terminology-loader` services
  - ensure both use the deployed `IMAGE_REF`

- `deploy/ec2/terminology-load.sh`
  - remove or rewrite to Docker Compose only

- `deploy/ec2/README.md`
  - document OIDC/SSM setup
  - document automatic destructive demo refresh
  - document terminology manifest and manual operations

## Acceptance Criteria

- PR CI no longer builds Docker images or contains deploy logic.
- A successful CI run on `main` automatically triggers CD.
- CD publishes one runtime image tagged with the commit SHA.
- CD assumes AWS credentials through OIDC and runs deployment through SSM.
- EC2 deploy reset uses `RESET_DB=1 INCLUDE_SEED=1`.
- Demo deploy reloads required terminology releases from `/opt/e2br3/terminology/terminology-manifest.prod`.
- Manual terminology dry-run/load works from `workflow_dispatch`.
- No production operation requires SSH, host Rust, or `cargo run` on EC2.
- Documentation clearly states that this is a temporary destructive demo deploy until migrations are implemented.
