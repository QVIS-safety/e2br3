#!/usr/bin/env sh
set -eu
# Pre-release baseline: validate the complete bootstrap, not append-only history.
python3 deploy/ec2/migrate-rds.py --validate-only
