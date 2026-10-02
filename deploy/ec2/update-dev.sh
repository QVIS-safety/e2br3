#!/usr/bin/env sh
set -eu

APP_DIR=/opt/e2br3
BACKEND_IMAGE=ghcr.io/qvis-safety/e2br3-web-server:dev
FRONTEND_IMAGE=ghcr.io/qvis-safety/e2br3-frontend-dev:dev

cd "${APP_DIR}"
sudo -u ec2-user git -C "${APP_DIR}" fetch origin dev
sudo -u ec2-user git -C "${APP_DIR}" checkout --detach origin/dev
set -a
. "${APP_DIR}/.env.dev"
set +a

docker image prune -af
docker pull "${BACKEND_IMAGE}"
docker pull "${FRONTEND_IMAGE}"

running_frontend=$(docker inspect e2br3-frontend --format '{{.Image}}' 2>/dev/null || true)
latest_frontend=$(docker image inspect "${FRONTEND_IMAGE}" --format '{{.Id}}')
if [ "${running_frontend}" != "${latest_frontend}" ]; then
	docker rm -f e2br3-frontend 2>/dev/null || true
	docker run -d --name e2br3-frontend --restart unless-stopped --network host \
		-e PORT=4033 -e API_PROXY_TARGET=http://127.0.0.1:8216 "${FRONTEND_IMAGE}"
	healthy=0
	for i in 1 2 3 4 5 6 7 8 9 10; do
		if curl -fsS http://127.0.0.1:4033/ >/dev/null; then
			healthy=1
			break
		fi
		sleep 3
	done
	if [ "${healthy}" != "1" ]; then
		docker rm -f e2br3-frontend || true
		test -z "${running_frontend}" || docker run -d --name e2br3-frontend \
			--restart unless-stopped --network host -e PORT=4033 \
			-e API_PROXY_TARGET=http://127.0.0.1:8216 "${running_frontend}"
		exit 1
	fi
fi

running_backend=$(docker inspect e2br3-web-server --format '{{.Image}}' 2>/dev/null || true)
latest_backend=$(docker image inspect "${BACKEND_IMAGE}" --format '{{.Id}}')
if [ "${running_backend}" != "${latest_backend}" ]; then
	DATABASE_URL="${SERVICE_MIGRATION_DB_URL}" PROJECT_DIR="${APP_DIR}" \
		"${APP_DIR}/deploy/ec2/migrate-rds.sh"
	APP_DIR="${APP_DIR}" ENV_FILE="${APP_DIR}/.env.dev" \
		COMPOSE_FILE="${APP_DIR}/deploy/ec2/docker-compose.prod.yml" \
		IMAGE_REF="${BACKEND_IMAGE}" \
		RESET_DB=0 INCLUDE_SEED=0 RELOAD_TERMINOLOGY=0 \
		HEALTHCHECK_URL=http://127.0.0.1:8216/health \
		"${APP_DIR}/deploy/ec2/deploy.sh"
fi
