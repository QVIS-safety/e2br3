# E2BR3 Local Development Setup

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

git clone https://github.com/QVIS-safety/e2br3.git
git clone https://github.com/QVIS-safety/E2BR3-frontend.git
```

Using SSH:

```sh
mkdir -p ~/projects/e2br3
cd ~/projects/e2br3

git clone git@github.com:QVIS-safety/e2br3.git
git clone git@github.com:QVIS-safety/E2BR3-frontend.git
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
git pull --ff-only origin main

cd ~/projects/e2br3/E2BR3-frontend
git pull --ff-only origin main
```

## EC2 Note

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
