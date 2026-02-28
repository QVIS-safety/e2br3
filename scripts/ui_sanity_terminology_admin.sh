#!/usr/bin/env bash
set -euo pipefail

PWCLI="${PWCLI:-/Users/hyundonghoon/playwright-cli/node_modules/.bin/playwright-cli}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [ ! -x "$PWCLI" ]; then
  echo "Missing playwright-cli at $PWCLI" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required" >&2
  exit 1
fi

UI_URL="${E2BR3_TERMINOLOGY_UI_URL:-http://localhost:8080}"
MUTATING="${E2BR3_UI_SANITY_MUTATING:-1}"
DEMO_EMAIL="${E2BR3_DEMO_EMAIL:-demo.user@example.com}"
DEMO_PWD="${E2BR3_DEMO_PWD:-welcome}"
PWCLI_LOG="${PWCLI_LOG:-/tmp/ui_sanity_terminology_admin.pwcli.log}"
export PWCLI_LOG
export UI_URL MUTATING DEMO_EMAIL DEMO_PWD

mkdir -p .playwright-cli
TMP_DIR="$(mktemp -d "$ROOT_DIR/.playwright-cli/terminology-ui-sanity.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

TS="$(date +%Y%m%d%H%M%S)"
SHORT_TS="$(date +%H%M%S)"
MEDDRA_V1="M${SHORT_TS}"
WHODRUG_V1="W1${SHORT_TS}"
WHODRUG_V2="W2${SHORT_TS}"
export MEDDRA_V1 WHODRUG_V1 WHODRUG_V2

export TMP_DIR
python3 <<'PY'
import csv
import os
import pathlib
import zipfile

tmp = pathlib.Path(os.environ["TMP_DIR"])

def write_meddra_zip(path: pathlib.Path, label: str):
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("llt.asc", f"10000001$Headache {label}$10000002$\n")
        zf.writestr(
            "mdhier.asc",
            f"10000001$20000001$30000001$40000001$Headache PT {label}$Headache HLT {label}$Headache HLGT {label}$Nervous system disorders$\n",
        )

def write_whodrug_csv(path: pathlib.Path, label: str):
    with path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["code", "drug_name", "atc_code"])
        w.writerow(["W001", f"Example Drug {label}", "A01AA01"])
        w.writerow(["W002", f"Example Drug 2 {label}", "A01AA02"])

write_meddra_zip(tmp / "meddra_v1.zip", "V1")
write_whodrug_csv(tmp / "whodrug_v1.csv", "V1")
write_whodrug_csv(tmp / "whodrug_v2.csv", "V2")
PY

export E2BR3_TERMINOLOGY_UI_URL="$UI_URL"
export E2BR3_UI_SANITY_MUTATING="$MUTATING"
export E2BR3_MOCK_MEDDRA_ZIP="$TMP_DIR/meddra_v1.zip"
export E2BR3_MOCK_WHODRUG_V1="$TMP_DIR/whodrug_v1.csv"
export E2BR3_MOCK_WHODRUG_V2="$TMP_DIR/whodrug_v2.csv"
export E2BR3_MOCK_MEDDRA_VERSION="$MEDDRA_V1"
export E2BR3_MOCK_WHODRUG_V1_VERSION="$WHODRUG_V1"
export E2BR3_MOCK_WHODRUG_V2_VERSION="$WHODRUG_V2"

JS_FILE="$TMP_DIR/terminology_ui_sanity.js"
cat >"$JS_FILE" <<'JS'
async (page) => {
  const uiUrl = "__UI_URL__";
  const mutating = "__MUTATING__" !== "0";
  const meddraZip = "__MEDDRA_ZIP__";
  const whodrugCsvV1 = "__WHODRUG_CSV_V1__";
  const whodrugCsvV2 = "__WHODRUG_CSV_V2__";
  const meddraVersion = "__MEDDRA_VERSION__";
  const whodrugV1 = "__WHODRUG_V1__";
  const whodrugV2 = "__WHODRUG_V2__";
  const demoEmail = "__DEMO_EMAIL__";
  const demoPwd = "__DEMO_PWD__";
  const out = { ok: false, uiUrl, mutating, steps: [], error: null };

  const capture = async (name, action) => {
    try {
      const step = await action();
      out.steps.push({ name, ...step });
      if (step.status < 200 || step.status >= 300) {
        throw new Error(`${name} failed with status=${step.status}`);
      }
    } catch (e) {
      out.steps.push({ name, status: 0, body: String(e) });
      throw e;
    }
  };

  const clickWait = async (buttonSel, responseMatch) => {
    const [resp] = await Promise.all([
      page.waitForResponse(
        (r) => r.request().method() === "POST" && r.url().includes(responseMatch),
        { timeout: 60000 }
      ),
      page.click(buttonSel),
    ]);
    const body = await resp.text().catch(() => "");
    return { status: resp.status(), body: body.slice(0, 1200) };
  };

  try {
    await page.goto(uiUrl, { waitUntil: "domcontentloaded" });
    const login = await page.evaluate(
      async ({ email, pwd }) => {
        const resp = await fetch("/auth/v1/login", {
          method: "POST",
          credentials: "include",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ email, pwd }),
        });
        const text = await resp.text().catch(() => "");
        return { status: resp.status, body: text.slice(0, 500) };
      },
      { email: demoEmail, pwd: demoPwd }
    );
    out.steps.push({ name: "login", ...login });
    if (login.status < 200 || login.status >= 300) {
      throw new Error(`login failed with status=${login.status}`);
    }

    await page.goto(uiUrl, { waitUntil: "domcontentloaded" });
    await page.waitForSelector("#meddra-file", { timeout: 30000 });

    await page.locator("#meddra-file").setInputFiles(meddraZip);
    await page.fill("#meddra-version", meddraVersion);
    await page.fill("#meddra-lang", "en");
    await capture("meddra_dry_run", () => clickWait("#meddra-dry", "/api/terminology/import/meddra"));

    await page.locator("#whodrug-file").setInputFiles(whodrugCsvV1);
    await page.fill("#whodrug-version", whodrugV1);
    await page.fill("#whodrug-lang", "en");
    await capture("whodrug_dry_run", () => clickWait("#whodrug-dry", "/api/terminology/import/whodrug"));

    if (mutating) {
      await capture("meddra_stage", () => clickWait("#meddra-load", "/api/terminology/import/meddra"));
      await capture(
        "meddra_approve",
        () => clickWait("#meddra-approve", `/api/terminology/releases/meddra/${encodeURIComponent(meddraVersion)}/approve`)
      );
      await capture(
        "meddra_activate",
        () => clickWait("#meddra-activate", `/api/terminology/releases/meddra/${encodeURIComponent(meddraVersion)}/activate`)
      );

      await capture("whodrug_stage_v1", () => clickWait("#whodrug-load", "/api/terminology/import/whodrug"));
      await capture(
        "whodrug_approve_v1",
        () => clickWait("#whodrug-approve", `/api/terminology/releases/whodrug/${encodeURIComponent(whodrugV1)}/approve`)
      );
      await capture(
        "whodrug_activate_v1",
        () => clickWait("#whodrug-activate", `/api/terminology/releases/whodrug/${encodeURIComponent(whodrugV1)}/activate`)
      );

      await page.locator("#whodrug-file").setInputFiles(whodrugCsvV2);
      await page.fill("#whodrug-version", whodrugV2);
      await capture("whodrug_stage_v2", () => clickWait("#whodrug-load", "/api/terminology/import/whodrug"));
      await capture(
        "whodrug_approve_v2",
        () => clickWait("#whodrug-approve", `/api/terminology/releases/whodrug/${encodeURIComponent(whodrugV2)}/approve`)
      );
      await capture(
        "whodrug_activate_v2",
        () => clickWait("#whodrug-activate", `/api/terminology/releases/whodrug/${encodeURIComponent(whodrugV2)}/activate`)
      );

      await page.fill("#whodrug-version", whodrugV1);
      await capture(
        "whodrug_rollback_to_v1",
        () => clickWait("#whodrug-rollback", `/api/terminology/releases/whodrug/${encodeURIComponent(whodrugV1)}/rollback`)
      );
    }

    out.ok = true;
    return out;
  } catch (e) {
    out.error = String(e);
    return out;
  }
}
JS

python3 <<'PY'
import os
import pathlib

js_path = pathlib.Path(os.environ["TMP_DIR"]) / "terminology_ui_sanity.js"
content = js_path.read_text()
repls = {
    "__UI_URL__": os.environ["UI_URL"],
    "__MUTATING__": os.environ["MUTATING"],
    "__MEDDRA_ZIP__": f'{os.environ["TMP_DIR"]}/meddra_v1.zip',
    "__WHODRUG_CSV_V1__": f'{os.environ["TMP_DIR"]}/whodrug_v1.csv',
    "__WHODRUG_CSV_V2__": f'{os.environ["TMP_DIR"]}/whodrug_v2.csv',
    "__MEDDRA_VERSION__": os.environ["MEDDRA_V1"],
    "__WHODRUG_V1__": os.environ["WHODRUG_V1"],
    "__WHODRUG_V2__": os.environ["WHODRUG_V2"],
    "__DEMO_EMAIL__": os.environ["DEMO_EMAIL"],
    "__DEMO_PWD__": os.environ["DEMO_PWD"],
}
for key, value in repls.items():
    content = content.replace(key, value)
js_path.write_text(content)
PY

: >"$PWCLI_LOG"
"$PWCLI" session-stop-all >>"$PWCLI_LOG" 2>&1 || true
"$PWCLI" config --headed --in-memory >>"$PWCLI_LOG" 2>&1

RAW_OUT_FILE="$TMP_DIR/run_code.out"
"$PWCLI" run-code "$(cat "$JS_FILE")" >"$RAW_OUT_FILE" 2>>"$PWCLI_LOG"

python3 <<'PY'
import json
import os
import pathlib
import sys

tmp = pathlib.Path(os.environ["TMP_DIR"])
raw = (tmp / "run_code.out").read_text()

payload = None
for line in reversed(raw.splitlines()):
    s = line.strip()
    if not s.startswith("{") or not s.endswith("}"):
        continue
    try:
        payload = json.loads(s)
        break
    except Exception:
        continue

if payload is None:
    decoder = json.JSONDecoder()
    for idx, ch in enumerate(raw):
        if ch != "{":
            continue
        try:
            obj, _ = decoder.raw_decode(raw[idx:])
            payload = obj
        except Exception:
            continue

if payload is None:
    print("Could not find JSON payload in playwright output", file=sys.stderr)
    print(raw, file=sys.stderr)
    sys.exit(1)

print(json.dumps(payload, indent=2))

if not payload.get("ok"):
    print("\nUI sanity failed. See PWCLI log:", os.environ.get("PWCLI_LOG", ""), file=sys.stderr)
    sys.exit(1)
PY

echo "UI sanity passed for terminology admin at $UI_URL"
echo "PWCLI log: $PWCLI_LOG"
