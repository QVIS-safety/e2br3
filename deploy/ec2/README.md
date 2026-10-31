# EC2 Deployment Bundle

## Files

- `docker-compose.prod.yml`: Production compose file (app only, uses RDS).
- `deploy.sh`: Pull and rollout script used by CD. In the temporary demo environment it can reset RDS, load seed data, preserve terminology, and restart the app.
- `init-rds.sh`: One-time SQL bootstrap runner for RDS.
- `run-terminology-manifest.sh`: Loads every terminology release listed in the EC2 terminology manifest.
- `terminology-manifest.prod.example`: Production manifest format. Real licensed dictionary files stay outside git.
- `terminology-load.sh`: Docker Compose wrapper for one-off terminology dry-run/load operations.
- `../../db/`: Organized SQL source tree (`admin/`, `bootstrap/`, `migrations/`, `seed/`).

## Dev staging automatic deployment

Backend dev pushes run the existing CI gates before publishing the `:dev` image.
Frontend dev pushes run the editor contract tests and production image build before
publication. Both images carry the exact source revision and `io.qvis.staging.ci=passed`.
The EC2 `e2br3-dev-update.timer` checks these images every minute with the host's
existing GHCR credentials; GitHub Actions publication success alone is not deployment success.

`update-dev.py` updates the existing `safetydb-staging` Compose project. It preserves
the database credential, schema/upload mounts, network, and port mappings. It pins both
images by registry digest, rejects missing CI metadata, and checks the complete applied
migration history before touching a running service. Unknown or rewritten migrations
require a deliberate database reconciliation; they are never baselined automatically.
Pending migrations use the existing migration runner after a successful private DB dump.
No database reset, seed, image prune, or legacy container creation is performed.

Install only after both CI-published images and a compatible database are available:

```sh
sudo COMPOSE_FILE=/opt/safetydb-staging/releases/fda9f62-20261027/compose.yml \
  STATE_DIR=/opt/safetydb-staging/private/dev-deploy \
  sh /opt/safetydb-staging/deploy/ec2/install-dev-updater.sh
```

The installer runs a read-only preflight before enabling the timer. Host dependencies
are Docker Compose, Python 3, curl, PostgreSQL client tools compatible with RDS, and
working GHCR authentication in `/root/.docker`. Runtime bootstrap must remain disabled.
Inspect `systemctl status e2br3-dev-update.service`, its journal, and private
`last-success.json`/`last-error.log` in `STATE_DIR` for the actual deployment result.

Rollout verifies both backend ports, frontend HTTP, the frontend API proxy, and the
staging HTTPS health endpoint. Failed rollout restores the previous images and reports
failure. Applied SQL remains applied; image restoration is not a database rollback.
Future SQL migrations must therefore preserve rollback compatibility. Backups remain
on the instance and do not provide recovery after loss of that instance.

The selected images are persisted as `STATE_DIR/images.json`. Manual Compose operations
must include `-f STATE_DIR/images.json` after the original Compose file, otherwise they
would select the original release's image tags. Stop automatic updates with
`systemctl stop e2br3-dev-update.timer` before deliberate manual changes.

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
   - The MedDRA entry must use the licensed English 28.1 release:
     `meddra /opt/e2br3/terminology/incoming/meddra_28_1 28.1 en`
     (the directory must contain the release's `llt.asc` and `mdhier.asc`, possibly in nested folders).
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
   E2BR3_EXPORT_VALIDATE=true
   E2BR3_DEFAULT_MESSAGE_SENDER=<sender-id>
   E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA=<fda-receiver-id>
   E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH=<ich-receiver-id>
   E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS=<mfds-receiver-id>
   RUST_LOG=info
   E2BR3_ENV=prod
   E2BR3_STRICT_SUBMISSION_CONFIG=true
   E2BR3_PUBLIC_ORIGIN=https://safety.example.com
   SUBMISSION_RECONCILE_TOKEN=<separate-internal-secret>
   ```
7. Ensure `/opt/e2br3/.env.prod` has `E2BR3_SCHEMAS_DIR=/opt/e2br3/schemas`
   so the container bind-mount includes `/app/schemas/...`.
   - `deploy.sh` now syncs the bundled `schemas/` tree into that runtime directory on each deploy,
     so newly added XSDs are included automatically.
8. Keep `SERVICE_PWD_KEY` stable across deployments and bootstrap runs.
   Seeded user password hashes are derived from `SERVICE_PWD_KEY`, so changing the key later will make
   existing passwords fail with `403 LOGIN_FAIL`.
   The first HMAC token rollout invalidates pre-rollout `auth-token` cookies;
   users must sign in again once after deployment.
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
`meddra_terms`, `whodrug_products`, `controlled_terminology_terms`, `mfds_products`,
`mfds_product_substances`, and `terminology_releases`, recreating the database,
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
IMAGE_REF=ghcr.io/<owner>/e2br3-web-server:<sha> RESET_DB=1 RESET_PRESERVE_TERMINOLOGY=1 INCLUDE_SEED=1 RELOAD_TERMINOLOGY=1 ./deploy/ec2/deploy.sh
```

`RESET_DB=1` recreates the database. With the default `RESET_PRESERVE_TERMINOLOGY=1`,
the deploy preserves existing terminology rows before the configured MedDRA 28.1 release is reloaded.
`INCLUDE_SEED=1` reloads demo seed data. Production preflight fails if the external MedDRA 28.1
source is missing, so a deployment cannot silently start with an empty catalog.

The Docker image intentionally packages the loader, not the licensed MedDRA source. The repository's
`data/` directory is ignored and absent from the CI checkout/build context; embedding the catalog
would require explicit MedDRA distribution authorization and a separately provisioned build artifact.
Until that authorization/artifact exists, upload the licensed 28.1 release outside git under
`/opt/e2br3/terminology/incoming/meddra_28_1` and let the mounted terminology-loader import it.

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
mfds-products /opt/e2br3/terminology/incoming/<mfds-products-release>.json <mfds-products-version> ko
iso3166 /opt/e2br3/terminology/incoming/<iso3166-release>.json <iso3166-version> en
ich-constrained-ucum /opt/e2br3/terminology/incoming/<ucum-release>.json <ucum-version> en
edqm /opt/e2br3/terminology/incoming/<edqm-release>.json <edqm-version> en
```

The fields are `dictionary host_input_path version [language]`. Supported dictionaries are shown above;
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

## AS2 submitter

The web server does not talk AS2 itself. It POSTs to an AS2 submitter over HTTP
(`crates/services/web-server/src/submission/gateway.rs`), and that submitter owns
the protocol in both directions — FDA delivers ACK3 and asynchronous MDNs back to
*our* gateway over AS2, so the submitter has to be reachable from outside.

Implementation: `github.com/donihyun/as2-submitter`.

`docker-compose.as2.yml` is an **opt-in overlay**. It changes nothing unless you
pass it explicitly, and it does not modify `docker-compose.prod.yml`.

### Why the API binds the wildcard

`AS2_SUBMITTER_BIND=0.0.0.0` looks alarming but is required: inside a container,
loopback is that container's own loopback, so `app` could not reach the submitter
over the Docker network. The API port is deliberately **not published to the
host** — only 4080, the AS2 receiver, is. What protects the API is the container
boundary plus `AS2_INBOUND_TOKEN`.

### One-time setup

1. Clone the submitter, since it publishes no image and is built on the instance:

   ```sh
   sudo git clone https://github.com/donihyun/as2-submitter /opt/as2-submitter
   ```

2. Place key material in `/opt/e2br3/as2-certs`:

   - `submitter.p12` — our signing key, one certificate per FDA routing ID
   - `partner.crt` — FDA's public certificate (ZZFDA), from the Gateway
     Configuration Information form

   For a pre-onboarding test run, generate throwaway material instead:

   ```sh
   cd /opt/as2-submitter && ./scripts/gen_test_certs.sh /opt/e2br3/as2-certs
   ```

3. Add to `/opt/e2br3/.env.prod`:

   ```sh
   AS2_SUBMITTER_SRC=/opt/as2-submitter
   AS2_CERTS_DIR=/opt/e2br3/as2-certs
   AS2_FROM_ID=<our-industry-routing-id>       # assigned by FDA
   AS2_CALLBACK_TOKEN=<shared-secret>
   AS2_SIGNING_PKCS12_PASSWORD=<keystore-password>
   AS2_FDA_ENV=test                            # test until FDA approves production
   AS2_RECEIVER_PORT=4080
   ```

   `AS2_SUBMITTER_URL`, `AS2_ACK_CALLBACK_URL`, the app-side
   `AS2_CALLBACK_TOKEN`, and `SUBMISSION_RECONCILE_TOKEN` are consumed by the
   app overlay. Keep the reconcile secret separate from the gateway callback
   secret.

   Note `E2BR3_STRICT_SUBMISSION_CONFIG=true` makes the web server refuse to
   start without a configured transport, so these must land before the rollout.

4. Open inbound TCP 4080 in the security group. Restrict the source: your own IP
   for a test run, and FDA's outbound range `150.148.0.0/16` for production.

### Rollout

```sh
cd /opt/e2br3
docker compose --env-file .env.prod \
  -f deploy/ec2/docker-compose.prod.yml \
  -f deploy/ec2/docker-compose.as2.yml \
  up -d --build
```

`deploy.sh` does not know about the overlay. Either run the command above after
it, or set `COMPOSE_FILE` to both files.

### Verify

```sh
# The submitter is up and both listeners bound.
docker logs e2br3-as2-submitter | head

# The API answers from inside the app container but not from the host.
docker exec e2br3-web-server curl -s http://as2-submitter:9090/health
curl -s --max-time 3 http://127.0.0.1:9090/health || echo "correctly not published"

# The AS2 receiver is reachable from outside.
curl -s -o /dev/null -w '%{http_code}\n' -X POST http://<ec2-host>:4080/as2/receive --data x
```

A 500 from that last call is expected — it means the port is open and OpenAS2-style
parsing rejected the garbage body.

### Interop against the mock gateway

Run the OpenAS2 FDA mock on a workstation and point the submitter at it, which
exercises the path FDA will actually use. See
`/opt/as2-submitter/docs/interop-ec2.md`.
