# EC2 Deployment Bundle

## Files

- `docker-compose.prod.yml`: Production compose file (app only, uses RDS).
- `.env.prod.example`: Template for required runtime environment variables.
- `deploy.sh`: Pull and rollout script used by CD.
- `init-rds.sh`: One-time SQL bootstrap runner for RDS.
- `terminology-load.sh`: EC2 host-side MedDRA/WHODrug loader wrapper.
- `../../db/`: Organized SQL source tree (`admin/`, `bootstrap/`, `migrations/`, `seed/`).

## One-time setup on EC2

1. Install Docker and Docker Compose plugin.
2. Create app directory:
   - `sudo mkdir -p /opt/e2br3/schemas`
3. Copy these files to `/opt/e2br3`:
   - `docker-compose.prod.yml`
   - `.env.prod.example` as `.env.prod`
   - `deploy.sh`
   - `run-terminology-manifest.sh`
   - `terminology-load.sh`
   - `terminology-manifest.prod.example`
   - `schemas/`
4. Make script executable:
   - `chmod +x /opt/e2br3/deploy.sh`
   - `chmod +x /opt/e2br3/run-terminology-manifest.sh`
   - `chmod +x /opt/e2br3/terminology-load.sh`
5. Create the production terminology manifest:
   - `sudo mkdir -p /opt/e2br3/terminology`
   - `cp /opt/e2br3/terminology-manifest.prod.example /opt/e2br3/terminology/terminology-manifest.prod`
   - Edit `/opt/e2br3/terminology/terminology-manifest.prod` to reference the licensed release files uploaded to `/opt/e2br3/terminology/incoming`.
6. Fill `/opt/e2br3/.env.prod` with real secrets and RDS URL.
7. Ensure `/opt/e2br3/.env.prod` has `E2BR3_SCHEMAS_DIR=/opt/e2br3/schemas`
   so the container bind-mount includes `/app/schemas/...`.
   - `deploy.sh` now syncs the bundled `schemas/` tree into that runtime directory on each deploy,
     so newly added XSDs are included automatically.
8. Keep `SERVICE_PWD_KEY` stable across deployments and bootstrap runs.
   Seeded user password hashes are derived from `SERVICE_PWD_KEY`, so changing the key later will make
   existing passwords fail with `403 LOGIN_FAIL`.
9. The app now re-syncs the initial platform admin (`hdh4063@gmail.com` / `welcome`)
   and demo tenant sponsor admins through application code on startup, instead of relying on
   hard-coded SQL password hashes.

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

If you keep DB URLs in `/opt/e2br3/.env.prod`, you can run:

```sh
cd /opt/e2br3
set -a
. /opt/e2br3/.env.prod
set +a
RESET_DB=1 DATABASE_URL="$SERVICE_DB_URL" ./deploy/ec2/init-rds.sh
```

`init-rds.sh` will use `SERVICE_DB_ROOT_URL` from the env file as `ROOT_DATABASE_URL`.
When `ROOT_DATABASE_URL` is available, it derives an admin connection to the target app database
and uses that for applying `db/bootstrap/*.sql` and `db/seed/*.sql`. This avoids role/GRANT failures
when `SERVICE_DB_URL` points at the lower-privilege `app_user`.
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
On startup it now ensures the built-in demo organization exists and re-hashes the built-in demo user password
with the current runtime key.

## Manual deploy

```sh
cd /opt/e2br3
IMAGE_REF=ghcr.io/<owner>/e2br3-web-server:<sha> ./deploy.sh
```

## Loading MedDRA and WHODrug on EC2

Dictionary files are licensed operational inputs. Do not commit them to git, bake them into the Docker
image, or leave extra copies in deployment bundles.

Create a private incoming directory on the EC2 host:

```sh
sudo mkdir -p /opt/e2br3/terminology/incoming
sudo chown -R "$USER":"$USER" /opt/e2br3/terminology
chmod 700 /opt/e2br3/terminology /opt/e2br3/terminology/incoming
```

Upload the licensed source files from your workstation with `scp`:

```sh
scp './<meddra-release>.zip' 'ec2-user@<ec2-host>:/opt/e2br3/terminology/incoming/'
scp './<whodrug-release>.zip' 'ec2-user@<ec2-host>:/opt/e2br3/terminology/incoming/'
```

If the EC2 host has no public IP, upload the files to private S3 first, then pull them down from a
Session Manager shell:

```sh
mkdir -p /opt/e2br3/terminology/incoming
chmod 700 /opt/e2br3/terminology /opt/e2br3/terminology/incoming

aws s3 cp 's3://<private-bucket>/terminology/<meddra-release>.zip' \
  '/opt/e2br3/terminology/incoming/<meddra-release>.zip' \
  --region ap-northeast-2

aws s3 cp 's3://<private-bucket>/terminology/<whodrug-release>.zip' \
  '/opt/e2br3/terminology/incoming/<whodrug-release>.zip' \
  --region ap-northeast-2
```

Run a dry run first through the one-off Docker Compose service. This uses the same deployed image as
the app, but runs `/app/terminology-loader` instead of starting the web server:

```sh
cd /opt/e2br3

docker compose --env-file .env.prod -f docker-compose.prod.yml run --rm terminology-loader \
  meddra \
  --input '/terminology/incoming/<meddra-release>.zip' \
  --version '<meddra-version>' \
  --language en \
  --dry-run

docker compose --env-file .env.prod -f docker-compose.prod.yml run --rm terminology-loader \
  whodrug \
  --input '/terminology/incoming/<whodrug-release>.zip' \
  --version '<whodrug-version>' \
  --language en \
  --dry-run
```

If the dry run succeeds, load the releases:

```sh
cd /opt/e2br3

docker compose --env-file .env.prod -f docker-compose.prod.yml run --rm terminology-loader \
  meddra \
  --input '/terminology/incoming/<meddra-release>.zip' \
  --version '<meddra-version>' \
  --language en

docker compose --env-file .env.prod -f docker-compose.prod.yml run --rm terminology-loader \
  whodrug \
  --input '/terminology/incoming/<whodrug-release>.zip' \
  --version '<whodrug-version>' \
  --language en
```

`E2BR3_TERMINOLOGY_DIR` can override the host directory mounted at `/terminology`. The default is
`/opt/e2br3/terminology`.

Remove uploaded source files after the load has been verified according to your retention policy.

## GitHub Actions secrets (for CD deploy job)

- `DEPLOY_HOST`
- `DEPLOY_USER`
- `DEPLOY_SSH_KEY`
- `DEPLOY_COMMAND`:
  - `cd /opt/e2br3 && APP_DIR=/opt/e2br3 ./deploy.sh`
- `GHCR_USERNAME` (optional if host already authenticated)
- `GHCR_TOKEN` (optional if host already authenticated)

The workflow passes `IMAGE_REF` automatically to the remote command.
