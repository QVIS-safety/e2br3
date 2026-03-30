# EC2 Deployment Bundle

## Files

- `docker-compose.prod.yml`: Production compose file (app only, uses RDS).
- `.env.prod.example`: Template for required runtime environment variables.
- `deploy.sh`: Pull and rollout script used by CD.
- `init-rds.sh`: One-time SQL bootstrap runner for RDS.
- `../../db/`: Organized SQL source tree (`admin/`, `bootstrap/`, `migrations/`, `seed/`).

## One-time setup on EC2

1. Install Docker and Docker Compose plugin.
2. Create app directory:
   - `sudo mkdir -p /opt/e2br3/schemas`
3. Copy these files to `/opt/e2br3`:
   - `docker-compose.prod.yml`
   - `.env.prod.example` as `.env.prod`
   - `deploy.sh`
   - `schemas/`
4. Make script executable:
   - `chmod +x /opt/e2br3/deploy.sh`
5. Fill `/opt/e2br3/.env.prod` with real secrets and RDS URL.
6. Ensure `/opt/e2br3/.env.prod` has `E2BR3_SCHEMAS_DIR=/opt/e2br3/schemas`
   so the container bind-mount includes `/app/schemas/...`.
   - `deploy.sh` now syncs the bundled `schemas/` tree into that runtime directory on each deploy,
     so newly added XSDs are included automatically.
7. Keep `SERVICE_PWD_KEY` stable across deployments and bootstrap runs.
   Seeded user password hashes are derived from `SERVICE_PWD_KEY`, so changing the key later will make
   existing passwords fail with `403 LOGIN_FAIL`.
8. The app now re-syncs the built-in demo admin user (`demo.user@example.com` / `welcome`)
   through application code on startup, instead of relying on a hard-coded SQL password hash.

## One-time RDS bootstrap

Run from this repository (local machine or EC2 clone):

```sh
DATABASE_URL='postgres://<user>:<pwd>@<rds-endpoint>:5432/app_db?sslmode=require' \
./deploy/ec2/init-rds.sh
```

Optional: reset DB/user first (destructive, runs `db/admin/00-recreate-db.sql`):

```sh
RESET_DB=1 \
ROOT_DATABASE_URL='postgres://<admin-user>:<admin-pwd>@<rds-endpoint>:5432/postgres?sslmode=require' \
DATABASE_URL='postgres://<app-user>:<app-pwd>@<rds-endpoint>:5432/app_db?sslmode=require' \
./deploy/ec2/init-rds.sh
```

If you keep DB URLs in `/opt/e2br3/e2br3/deploy/ec2/.env.prod`, you can run:

```sh
cd /opt/e2br3/e2br3
set -a
. /opt/e2br3/e2br3/deploy/ec2/.env.prod
set +a
RESET_DB=1 DATABASE_URL="$SERVICE_DB_URL" ./deploy/ec2/init-rds.sh
```

`init-rds.sh` will use `SERVICE_DB_ROOT_URL` from the env file as `ROOT_DATABASE_URL`.
It currently applies `db/bootstrap/*.sql` and then `db/seed/*.sql` when `INCLUDE_SEED=1`.
`db/migrations/` is reserved for future incremental changes.

To skip dev seed data (`db/seed/*.sql`):

```sh
INCLUDE_SEED=0 DATABASE_URL='postgres://<user>:<pwd>@<rds-endpoint>:5432/app_db?sslmode=require' \
./deploy/ec2/init-rds.sh
```

## Recovering login after changing `SERVICE_PWD_KEY`

If the app returns `403 LOGIN_FAIL` for the built-in demo user right after an EC2 deploy, verify that the
running container and the database bootstrap used the same `SERVICE_PWD_KEY`, then redeploy the updated app.
On startup it will re-hash the built-in demo user password with the current runtime key.

## Manual deploy

```sh
cd /opt/e2br3
IMAGE_REF=ghcr.io/<owner>/e2br3-web-server:<sha> ./deploy.sh
```

## GitHub Actions secrets (for CD deploy job)

- `DEPLOY_HOST`
- `DEPLOY_USER`
- `DEPLOY_SSH_KEY`
- `DEPLOY_COMMAND`:
  - `cd /opt/e2br3 && APP_DIR=/opt/e2br3 ./deploy.sh`
- `GHCR_USERNAME` (optional if host already authenticated)
- `GHCR_TOKEN` (optional if host already authenticated)

The workflow passes `IMAGE_REF` automatically to the remote command.
