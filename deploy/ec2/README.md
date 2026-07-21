# EC2 Deployment Bundle

## Files

- `docker-compose.prod.yml`: Production compose file (app only, uses RDS).
- `deploy.sh`: Pull and rollout script used by CD. In the temporary demo environment it can reset RDS, load seed data, preserve terminology, and restart the app.
- `init-rds.sh`: One-time SQL bootstrap runner for RDS.
- `run-terminology-manifest.sh`: Loads every terminology release listed in the EC2 terminology manifest.
- `terminology-manifest.prod.example`: Example manifest format. Real licensed dictionary files stay outside git.
- `terminology-load.sh`: Docker Compose wrapper for one-off terminology dry-run/load operations.
- `../../db/`: Organized SQL source tree (`admin/`, `bootstrap/`, `migrations/`, `seed/`).

## One-time setup on EC2

1. Install Docker and Docker Compose plugin.
2. Create app directory:
   - `sudo mkdir -p /opt/e2br3/schemas`
3. Clone or update this repository under `/opt/e2br3`.
4. Make deployment scripts executable:
   - `chmod +x /opt/e2br3/deploy/ec2/deploy.sh`
   - `chmod +x /opt/e2br3/deploy/ec2/init-rds.sh`
   - `chmod +x /opt/e2br3/deploy/ec2/run-terminology-manifest.sh`
   - `chmod +x /opt/e2br3/deploy/ec2/terminology-load.sh`
5. Create the production terminology manifest:
   - `sudo mkdir -p /opt/e2br3/terminology/incoming`
   - `sudo chown -R "$USER":"$USER" /opt/e2br3/terminology`
   - `cp /opt/e2br3/deploy/ec2/terminology-manifest.prod.example /opt/e2br3/terminology/terminology-manifest.prod`
   - Edit `/opt/e2br3/terminology/terminology-manifest.prod` to reference the licensed release files uploaded to `/opt/e2br3/terminology/incoming`.
6. Create `/opt/e2br3/.env.prod` manually with the runtime image, RDS URLs, secrets, and deployment
   settings. Use real operational values for secrets and URLs:

   ```sh
   IMAGE_REF=ghcr.io/<owner>/e2br3-web-server:<sha>
   APP_PORT=3000
   SERVICE_DB_URL=postgres://<app-user>:<app-password>@<rds-endpoint>:5432/<app-db>?sslmode=require
   SERVICE_MIGRATION_DB_URL=postgres://<migration-user>:<migration-password>@<rds-endpoint>:5432/<app-db>?sslmode=require
   SERVICE_DB_ROOT_URL=postgres://<admin-user>:<admin-password>@<rds-endpoint>:5432/postgres?sslmode=require
   SERVICE_PWD_KEY=<stable-password-hash-key>
   SERVICE_TOKEN_KEY=<stable-token-signing-key>
   SERVICE_TOKEN_DURATION_SEC=86400
   E2BR3_VALIDATOR_TOKEN=<validator-token>
   E2BR3_SCHEMAS_DIR=/opt/e2br3/schemas
   E2BR3_XSD_PATH=/opt/e2br3/schemas/ich-icsr-v3.0.xsd
   E2BR3_SKIP_XML_VALIDATE=false
   E2BR3_EXPORT_VALIDATE=true
   E2BR3_DEFAULT_MESSAGE_SENDER=<sender-id>
   E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA=<fda-receiver-id>
   E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH=<ich-receiver-id>
   E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS=<mfds-receiver-id>
   RUST_LOG=info
   E2BR3_ENV=prod
   E2BR3_STRICT_SUBMISSION_CONFIG=true
   E2BR3_ALLOW_MOCK_SUBMISSION=false
   ```
7. Ensure `/opt/e2br3/.env.prod` has `E2BR3_SCHEMAS_DIR=/opt/e2br3/schemas`
   so the container bind-mount includes `/app/schemas/...`.
   - `deploy.sh` now syncs the bundled `schemas/` tree into that runtime directory on each deploy,
     so newly added XSDs are included automatically.
8. Keep `SERVICE_PWD_KEY` stable across deployments and bootstrap runs.
   Seeded user password hashes are derived from `SERVICE_PWD_KEY`, so changing the key later will make
   existing passwords fail with `403 LOGIN_FAIL`.
   Keep `SERVICE_MIGRATION_DB_URL` separate from `SERVICE_DB_URL`: the migration user owns the
   versioned authorization migration objects, while request connections retain the lower-privilege
   application role. Deployment preflight rejects URLs that resolve to the same database username.
9. The app now re-syncs the initial platform admin (`hdh4063@gmail.com` / `welcome`)
   and demo tenant sponsor admins through application code on startup, instead of relying on
   hard-coded SQL password hashes.
10. Ensure the EC2 instance is managed by AWS Systems Manager Session Manager and can pull the
    GHCR runtime image, either through existing Docker authentication or an instance/runtime process
    that provides registry credentials.

## One-time RDS bootstrap

Run from this repository (local machine or EC2 clone):

```sh
DATABASE_URL='postgres://<user>:<pwd>@<rds-endpoint>:5432/app_db?sslmode=require' \
./deploy/ec2/init-rds.sh
```

Optional: reset DB/user first. By default this preserves terminology rows by dumping
`meddra_terms`, `whodrug_products`, and `terminology_releases`, recreating the database,
then restoring those rows before seed data is applied:

```sh
RESET_DB=1 \
RESET_PRESERVE_TERMINOLOGY=1 \
ROOT_DATABASE_URL='postgres://<admin-user>:<admin-pwd>@<rds-endpoint>:5432/postgres?sslmode=require' \
DATABASE_URL='postgres://<app-user>:<app-pwd>@<rds-endpoint>:5432/app_db?sslmode=require' \
./deploy/ec2/init-rds.sh
```

For a truly destructive terminology reset, use `RESET_PRESERVE_TERMINOLOGY=0`.

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
It applies `db/bootstrap/*.sql`, then `db/migrations/*.sql`, and finally `db/seed/*.sql`
when `INCLUDE_SEED=1`. Both clean initialization and upgrades therefore use the same ordered
authorization migrations.

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
IMAGE_REF=ghcr.io/<owner>/e2br3-web-server:<sha> ./deploy/ec2/deploy.sh
```

Manual demo reset without reloading terminology:

```sh
cd /opt/e2br3
IMAGE_REF=ghcr.io/<owner>/e2br3-web-server:<sha> \
RESET_DB=1 \
RESET_PRESERVE_TERMINOLOGY=1 \
INCLUDE_SEED=1 \
RELOAD_TERMINOLOGY=0 \
HEALTHCHECK_URL=http://127.0.0.1:8080/health \
./deploy/ec2/deploy.sh
```

Manual demo reset that also reloads terminology from the manifest:

```sh
cd /opt/e2br3
IMAGE_REF=ghcr.io/<owner>/e2br3-web-server:<sha> \
RESET_DB=1 \
RESET_PRESERVE_TERMINOLOGY=0 \
INCLUDE_SEED=1 \
RELOAD_TERMINOLOGY=1 \
HEALTHCHECK_URL=http://127.0.0.1:8080/health \
./deploy/ec2/deploy.sh
```

## Automatic demo deploy

After CI succeeds on `main`, CD builds the runtime image and deploys through AWS Systems Manager
using:

```sh
IMAGE_REF=ghcr.io/<owner>/e2br3-web-server:<sha> RESET_DB=1 RESET_PRESERVE_TERMINOLOGY=1 INCLUDE_SEED=1 RELOAD_TERMINOLOGY=0 ./deploy/ec2/deploy.sh
```

`RESET_DB=1` recreates the database. With the default `RESET_PRESERVE_TERMINOLOGY=1`,
the deploy preserves the already-loaded terminology tables and avoids parsing the licensed zip files
on every deploy. `INCLUDE_SEED=1` reloads demo seed data. `RELOAD_TERMINOLOGY=0` means
automatic demo deploys do not run the terminology manifest.

Use `RELOAD_TERMINOLOGY=1` only when you intentionally want to reload the releases listed in
`/opt/e2br3/terminology/terminology-manifest.prod`.

## GitHub Actions configuration

Required repository secrets for the CD deploy job:

- `AWS_ROLE_TO_ASSUME`
- `AWS_REGION`
- `AWS_SSM_TARGET`

CD does not use SSH. Runtime DB URLs and app secrets stay on the EC2 host in `/opt/e2br3/.env.prod`.
The workflow passes `IMAGE_REF` automatically to the SSM command.

## Terminology manifest

The production manifest lives at `/opt/e2br3/terminology/terminology-manifest.prod`. Start from
`/opt/e2br3/terminology-manifest.prod.example` and keep real licensed dictionary archive names outside
git.

Each non-comment line is whitespace-delimited:

```text
# dictionary host_input_path version [language]
meddra /opt/e2br3/terminology/incoming/<meddra-release>.zip <meddra-version> en
whodrug /opt/e2br3/terminology/incoming/<whodrug-release>.zip <whodrug-version> en
```

The fields are `dictionary host_input_path version [language]`. `dictionary` is `meddra` or `whodrug`,
`version` is the release identifier passed to the loader, and `language` defaults to `en` when omitted.
The host input path must be inside `E2BR3_TERMINOLOGY_DIR`. The default `E2BR3_TERMINOLOGY_DIR` is
`/opt/e2br3/terminology`, mounted into the container as `/terminology`.

## Loading Terminology on EC2

Dictionary files are licensed operational inputs. Do not commit them to git, bake them into the Docker
image, or leave extra copies in deployment bundles.

Create a private incoming directory on the EC2 host:

```sh
sudo mkdir -p /opt/e2br3/terminology/incoming
sudo chown -R "$USER":"$USER" /opt/e2br3/terminology
chmod 700 /opt/e2br3/terminology /opt/e2br3/terminology/incoming
```

Upload licensed source files to private object storage first, then pull them down from a
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

Load ISO 3166-1 alpha-2 countries into the same database used by the terminology endpoints:

```sh
cd /opt/e2br3
./deploy/ec2/load-iso-countries.sh
```

The script reads `SERVICE_DB_URL` from `.env.prod`, downloads the DataHub/Core country-list CSV, and
upserts `iso_countries`. Existing country rows missing from the source are marked inactive.
