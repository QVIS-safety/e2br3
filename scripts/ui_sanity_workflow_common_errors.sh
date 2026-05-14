#!/usr/bin/env bash
set -euo pipefail

PWCLI="/Users/hyundonghoon/playwright-cli/node_modules/.bin/playwright-cli"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REFPY="$SCRIPT_DIR/pwcli_ref.py"

# Ensure `.playwright-cli/` artifacts are created under the frontend repo root.
cd "$SCRIPT_DIR/.."

if [ ! -x "$PWCLI" ]; then
  echo "Missing playwright-cli at $PWCLI" >&2
  exit 1
fi

FRONTEND_URL="${E2BR3_FRONTEND_URL:-http://localhost:3000}"
DEMO_EMAIL="${E2BR3_DEMO_EMAIL:-demo.user@example.com}"
DEMO_PWD="${E2BR3_DEMO_PWD:-welcome}"

EXAMPLE_XML="${E2BR3_EXAMPLE_XML:-/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/refs/instances/FAERS2022Scenario1.xml}"
PWCLI_LOG="${PWCLI_LOG:-/tmp/ui_sanity_workflow_common_errors.pwcli.log}"
# Keep session name short: macOS UNIX socket path length limits can break long names.

now_id() {
  # Stable enough for UI test identifiers; avoid purely-numeric strings.
  date +"SR-%Y%m%d-%H%M%S"
}

latest_snapshot() {
  # Avoid glob expansion on `.playwright-cli/page-*.yml` (can get huge / hit arg limits).
  local f=""
  if command -v rg >/dev/null 2>&1; then
    f="$(ls -t .playwright-cli 2>/dev/null | rg '^page-.*\\.yml$' | head -n 1 || true)"
  else
    f="$(ls -t .playwright-cli 2>/dev/null | grep -E '^page-.*\\.yml$' | head -n 1 || true)"
  fi
  if [ -n "$f" ]; then
    echo ".playwright-cli/$f"
  fi
}

pw() {
  # Keep stdout/stderr out of the terminal to avoid flakiness; capture for debugging.
  "$PWCLI" "$@" >>"$PWCLI_LOG" 2>&1
}

snapshot() {
  local out=""
  local snap=""
  local i
  for i in $(seq 1 5); do
    out="$("$PWCLI" snapshot 2>&1)"
    printf '%s\n' "$out" >>"$PWCLI_LOG"

    snap="$(python3 -c 'import re,sys; s=sys.stdin.read(); m=re.findall(r"\\.playwright-cli/page-[^)]*\\.yml", s); print(m[-1] if m else "")' <<<"$out")"
    if [ -z "$snap" ]; then
      snap="$(latest_snapshot || true)"
    fi

    if [ -n "$snap" ] && [ -f "$snap" ]; then
      echo "$snap"
      return 0
    fi
    sleep 1
  done
  echo "$snap"
  return 1
}

ref() {
  local match="$1"
  local snap="$2"
  python3 "$REFPY" --match "$match" "$snap"
}

ref_after() {
  local after="$1"
  local match="$2"
  local snap="$3"
  python3 "$REFPY" --after "$after" --match "$match" "$snap"
}

# Headed is intentional: this is a UI sanity + diagnostics script.
: >"$PWCLI_LOG"
pw session-stop-all || true
pw config --headed --in-memory
mkdir -p .playwright-cli

trap 'rc=$?; echo "UI sanity FAILED (rc=$rc) at line $LINENO" >&2; s="$(latest_snapshot || true)"; if [ -n "${s:-}" ]; then echo "Last snapshot: $s" >&2; tail -n 60 "$s" >&2 || true; fi; echo "PWCLI log: $PWCLI_LOG" >&2; tail -n 120 "$PWCLI_LOG" >&2 || true; exit $rc' ERR

########################################
# Login + Select Org
########################################
echo "UI sanity: login..."
pw open "$FRONTEND_URL"
SNAP="$(snapshot)"

EMAIL_REF="$(ref 'textbox "Email Address"' "$SNAP")"
PWD_REF="$(ref 'textbox "Password"' "$SNAP")"
SIGNIN_REF="$(ref 'button "Sign In"' "$SNAP")"

pw fill "$EMAIL_REF" "$DEMO_EMAIL"
pw fill "$PWD_REF" "$DEMO_PWD"
pw click "$SIGNIN_REF"

SNAP="$(snapshot)"
ORG_REF="$(ref 'button "Demo Organization' "$SNAP")"
pw click "$ORG_REF"

########################################
# Create Case From Scratch (Dup Check)
########################################
CASE_SR_ID="$(now_id)"
echo "UI sanity: create case from scratch (C.1.1=$CASE_SR_ID)..."
pw open "$FRONTEND_URL/cases/new"

# Wait for the duplication check page to load.
for _ in $(seq 1 30); do
  SNAP="$(snapshot)"
  if grep -q 'heading "Duplication Check"' "$SNAP"; then
    break
  fi
  sleep 1
done

SR_REF="$(ref 'textbox "Enter C.1.1"' "$SNAP")"
DATE_MOST_RECENT_REF="$(ref_after 'Most Recent Information' 'textbox' "$SNAP")"
REPORTTYPE1_REF="$(ref 'radio "1: Spontaneous report"' "$SNAP")"
RUN_REF="$(ref 'button "Run Check & Continue"' "$SNAP")"

pw fill "$SR_REF" "$CASE_SR_ID"
pw fill "$DATE_MOST_RECENT_REF" "2026-01-01"
pw click "$REPORTTYPE1_REF"
pw click "$RUN_REF"

# Wait for redirect to the edit page ("Save" should appear).
for _ in $(seq 1 60); do
  SNAP="$(snapshot)"
  if grep -q 'button "Save"' "$SNAP" && grep -q 'button "SD (C.3)"' "$SNAP"; then
    break
  fi
  sleep 1
done

########################################
# Export XML (Submission page)
########################################
echo "UI sanity: export XML..."
pw open "$FRONTEND_URL/submission"
for _ in $(seq 1 60); do
  SNAP="$(snapshot)"
  if grep -q 'heading "Ready for Submission"' "$SNAP"; then
    break
  fi
  sleep 1
done

# Select the row for the new case via the "Case Number" cell text.
# Then hit "Export XML" (single-case export).
for _ in $(seq 1 60); do
  SNAP="$(snapshot)"
  if grep -q "$CASE_SR_ID" "$SNAP"; then
    break
  fi
  sleep 1
done

ROW_CHECKBOX_REF="$(ref_after "row \\\"$CASE_SR_ID" 'checkbox' "$SNAP")"
EXPORT_REF="$(ref 'button "Export XML"' "$SNAP")"
pw click "$ROW_CHECKBOX_REF"
pw click "$EXPORT_REF"

########################################
# Import XML (Import page)
########################################
echo "UI sanity: import XML..."
pw open "$FRONTEND_URL/import"
for _ in $(seq 1 60); do
  SNAP="$(snapshot)"
  if grep -q 'heading "Import"' "$SNAP" || grep -q 'text: Import XML' "$SNAP"; then
    break
  fi
  sleep 1
done

# Use the file chooser input (usually shows up as a button "Select Files").
SNAP="$(snapshot)"
SELECT_FILES_REF="$(ref 'button \"Select Files\"' "$SNAP")"
pw click "$SELECT_FILES_REF"
pw upload "$EXAMPLE_XML"

########################################
# Syntax gating check: clear required C.1.1 => Save disabled (no backend request)
########################################
echo "UI sanity: verify Save disables when required C.1.1 is cleared..."
pw open "$FRONTEND_URL/cases"
for _ in $(seq 1 60); do
  SNAP="$(snapshot)"
  if grep -q 'button "Edit case"' "$SNAP"; then
    break
  fi
  sleep 1
done

EDIT_REF="$(ref 'button \"Edit case\"' "$SNAP")"
pw click "$EDIT_REF"

# Wait for form loaded.
for _ in $(seq 1 60); do
  SNAP="$(snapshot)"
  if grep -q 'button "Save"' "$SNAP" && grep -q "Sender's (case) Safety Report Unique Identifier" "$SNAP"; then
    break
  fi
  sleep 1
done

# Clear C.1.1 and verify Save becomes disabled (syntax gating).
# In the snapshot, the C.1.1 label is rendered as plain text, not as `text: C.1.1`.
SR_EDIT_REF="$(ref_after 'Sender\\x27s \\(case\\) Safety Report Unique Identifier' 'textbox' "$SNAP")"
SAVE_REF="$(ref 'button \"Save\"' "$SNAP")"
pw fill "$SR_EDIT_REF" ""
SNAP="$(snapshot)"
if ! grep -q 'button "Save" \\[disabled\\]' "$SNAP"; then
  echo "UI sanity: expected Save to be disabled after clearing C.1.1, but it was not." >&2
  exit 1
fi

echo "UI sanity workflow complete. Created case C.1.1=$CASE_SR_ID, exported XML, imported XML, and verified Save disables when C.1.1 is cleared."
