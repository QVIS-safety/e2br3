#!/usr/bin/env sh
set -eu

: "${COMPOSE_FILE:?Set COMPOSE_FILE to the existing safetydb-staging compose.yml}"
: "${STATE_DIR:?Set STATE_DIR to the private deployment state directory}"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
test -f "${COMPOSE_FILE}"
test -f "${SCRIPT_DIR}/update-dev.py"
python3 "${SCRIPT_DIR}/update-dev.py" --compose-file "${COMPOSE_FILE}" --state-dir "${STATE_DIR}" --check-only

cat >/etc/systemd/system/e2br3-dev-update.service <<EOF
[Unit]
Description=Deploy CI-published dev images to safetydb-staging
After=docker.service network-online.target

[Service]
Type=oneshot
Environment=DOCKER_CONFIG=/root/.docker
TimeoutStartSec=30min
ExecStart=/usr/bin/python3 "${SCRIPT_DIR}/update-dev.py" --compose-file "${COMPOSE_FILE}" --state-dir "${STATE_DIR}"
EOF

cat >/etc/systemd/system/e2br3-dev-update.timer <<'EOF'
[Unit]
Description=Check CI-published dev images every minute

[Timer]
OnBootSec=1min
OnUnitInactiveSec=1min

[Install]
WantedBy=timers.target
EOF

systemctl daemon-reload
systemctl enable --now e2br3-dev-update.timer
systemctl start e2br3-dev-update.service
