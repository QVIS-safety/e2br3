#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

ROLE_CONSOLE="web-folder/index.html"

if rg -n 'Display name|display name' "$ROLE_CONSOLE" >/tmp/e2br3_ui_static_contracts.matches; then
  echo "Role console must use Description terminology, not Display name:" >&2
  cat /tmp/e2br3_ui_static_contracts.matches >&2
  exit 1
fi

if ! rg -n 'id="role-description"' "$ROLE_CONSOLE" >/dev/null; then
  echo "Role console must expose a role description input." >&2
  exit 1
fi

if ! rg -n 'description: description' "$ROLE_CONSOLE" >/dev/null; then
  echo "Role creation payload must send the description field." >&2
  exit 1
fi

if rg -n 'display_name: displayName' "$ROLE_CONSOLE" >/dev/null; then
  echo "Role creation must not require a separate display_name value from the UI." >&2
  exit 1
fi

echo "Static UI contracts passed."
