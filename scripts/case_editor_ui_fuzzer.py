#!/usr/bin/env python3
"""Run the API fuzzer's seeded candidate matrix through the case-editor UI."""

from __future__ import annotations

import argparse
import functools
import hashlib
import http.server
import json
import os
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any

import case_editor_input_fuzzer as candidates
from rbac_rls_blackbox import guard_target


ROOT = Path(__file__).resolve().parents[1]
PWCODE = Path(__file__).with_suffix(".pwcode.js")


def prepared_contract(args: argparse.Namespace) -> list[dict[str, Any]]:
    contract = json.loads(Path(args.contract).resolve().read_text(encoding="utf-8"))
    candidates.apply_max_lengths(contract, candidates.load_max_lengths(ROOT))
    candidates.apply_generated_rules(
        contract,
        candidates.load_generated_rules(ROOT, candidates.IDENTIFIER_RE),
        candidates.load_generated_rules(ROOT, candidates.BOOLEAN_RE),
    )
    candidates.expand_null_flavor_contracts(
        contract,
        candidates.load_null_flavor_pairs(Path(args.null_flavor_pairs).resolve()),
        candidates.load_dictionary_null_flavors(ROOT),
    )
    return contract


def field_shard(field: dict[str, Any], count: int) -> int:
    identity = "|".join(str(field.get(key, "")) for key in ("authority", "code", "frontendPath", "payloadPath"))
    return int.from_bytes(hashlib.sha256(identity.encode()).digest()[:8]) % count


def build_plan(args: argparse.Namespace) -> dict[str, Any]:
    pages = [value.strip().upper() for value in args.pages.split(",") if value.strip()]
    requested = set(args.field or ())
    fields: list[dict[str, Any]] = []
    contract = prepared_contract(args)
    for page in pages:
        for field in candidates.contract_rows(contract, page):
            if requested and field["code"] not in requested:
                continue
            if field_shard(field, args.shard_count) != args.shard_index:
                continue
            mutations = []
            for ordinal in range(candidates.candidate_count(field, args.values_per_field)):
                for sample in range(candidates.candidate_sample_count(field, ordinal, args.samples_per_category)):
                    rng = candidates.candidate_rng(args.seed, field, ordinal, sample)
                    value = candidates.field_value(field, rng, ordinal, sample)
                    expectation = candidates.candidate_expectation(field, ordinal)
                    if candidates.is_nullflavor_field(field):
                        if candidates.nullflavor_invalid_candidate(field, value) or ordinal == 13:
                            expectation = ("reject", field.get("constraint", {}).get("ruleCode"))
                        elif ordinal in {2, 3}:
                            expectation = ("accept", None)
                    mutations.append({
                        "ordinal": ordinal,
                        "sample": sample,
                        "kind": candidates.candidate_kind(field, ordinal),
                        "value": value,
                        "expectation": expectation[0] if expectation else None,
                        "ruleCode": expectation[1] if expectation else None,
                        "fingerprint": candidates.candidate_fingerprint(field, ordinal, sample, value),
                        "withValue": candidates.is_nullflavor_field(field) and ordinal == 13,
                    })
            fields.append({
                "page": page,
                "authority": str(field.get("authority", "ICH")).lower(),
                "code": field["code"],
                "frontendPath": field.get("frontendPath"),
                "owner": field["patch"]["owner"],
                "baseline": field.get("roundTripValue"),
                "nullFlavor": candidates.is_nullflavor_field(field),
                "nullFlavorPartnerCode": field.get("nullFlavorPartnerCode"),
                "nullFlavorPartnerValue": field.get("_nullFlavorPartnerValue"),
                "mutations": mutations,
            })
    return {
        "schemaVersion": 1,
        "seed": args.seed,
        "pages": pages,
        "shard": {"index": args.shard_index, "count": args.shard_count},
        "fields": fields,
        "fieldCount": len(fields),
        "mutationCount": sum(len(field["mutations"]) for field in fields),
    }


def setup_case(args: argparse.Namespace) -> str:
    setup_dir = Path(args.artifact_dir) / "setup"
    command = [
        sys.executable, str(ROOT / "scripts/case_editor_input_fuzzer.py"),
        "--base-url", args.base_url, "--email", args.email, "--password", args.password,
        "--seed", str(args.seed), "--pages", args.pages, "--values-per-field", "0",
        "--samples-per-category", "1", "--artifact-dir", str(setup_dir), "--no-run-gates",
        "--complete-baseline",
    ]
    subprocess.run(command, cwd=ROOT, check=True)
    artifact = setup_dir / f"case-editor-{args.seed}.jsonl"
    run = json.loads(artifact.read_text(encoding="utf-8").splitlines()[-1])
    if not run.get("case_id"):
        raise RuntimeError(f"setup artifact has no case_id: {artifact}")
    return str(run["case_id"])


def run_browser(args: argparse.Namespace, plan_path: Path, case_id: str) -> dict[str, Any]:
    pwcli = Path(args.pwcli).expanduser().resolve()
    if not pwcli.is_file() or not os.access(pwcli, os.X_OK):
        raise SystemExit(f"--pwcli is not executable: {pwcli}")
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(plan_path.parent))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    plan_url = f"http://127.0.0.1:{server.server_port}/{plan_path.name}"
    source = PWCODE.read_text(encoding="utf-8")
    config = json.dumps({
        "frontendUrl": args.frontend_url.rstrip("/"), "planUrl": plan_url,
        "caseId": case_id, "email": args.email, "password": args.password,
        "maxActions": args.max_actions,
    }, ensure_ascii=True)
    code = source.replace("__CONFIG__", config)
    try:
        subprocess.run([str(pwcli), "session-stop-all"], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run([str(pwcli), "config", "--headed", "--in-memory"], check=True)
        subprocess.run([str(pwcli), "open", args.frontend_url], check=True)
        result = subprocess.run([str(pwcli), "run-code", code], check=True, capture_output=True, text=True)
    finally:
        server.shutdown()
    payload = None
    for line in reversed(result.stdout.splitlines()):
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict) and value.get("marker") == "E2BR3_UI_FUZZ_RESULT":
            payload = value.get("result")
            break
    if not isinstance(payload, dict):
        raise RuntimeError(f"browser result marker missing\n{result.stdout[-4000:]}\n{result.stderr[-2000:]}")
    return payload


def parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", default=os.getenv("E2BR3_BASE_URL", "http://127.0.0.1:8080"))
    parser.add_argument("--frontend-url", default=os.getenv("E2BR3_FRONTEND_URL", "http://127.0.0.1:3000"))
    parser.add_argument("--email", default=os.getenv("E2BR3_ADMIN_EMAIL", "demo.cro.admin@example.com"))
    parser.add_argument("--password", default=os.getenv("E2BR3_ADMIN_PASSWORD", "welcome"))
    parser.add_argument("--seed", type=int, default=int(time.time()))
    parser.add_argument("--pages", default="CI,RP,SD,LR,SI,DM,DH,NR,AE,LB,DG")
    parser.add_argument("--field", action="append")
    parser.add_argument("--values-per-field", type=int, default=candidates.IDENTIFIER_CANDIDATES)
    parser.add_argument("--samples-per-category", type=int, default=3)
    parser.add_argument("--shard-count", type=int, default=1)
    parser.add_argument("--shard-index", type=int, default=0)
    parser.add_argument("--max-actions", type=int, default=30000)
    parser.add_argument("--case-id")
    parser.add_argument("--setup-case", action="store_true")
    parser.add_argument("--pwcli", default=os.getenv("PWCLI"))
    parser.add_argument("--artifact-dir", default="tmp/rbac-rls-fuzz/case-editor-ui")
    parser.add_argument("--contract", default=str(candidates.DEFAULT_CONTRACT))
    parser.add_argument("--null-flavor-pairs", default=str(candidates.DEFAULT_NULL_FLAVOR_PAIRS))
    parser.add_argument("--dry-run", action="store_true")
    return parser


def main(args: argparse.Namespace) -> int:
    if args.samples_per_category < 1 or args.shard_count < 1 or not 0 <= args.shard_index < args.shard_count:
        raise SystemExit("invalid sample or shard arguments")
    plan = build_plan(args)
    out_dir = Path(args.artifact_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    plan_path = out_dir / f"ui-plan-{args.seed}-{args.shard_index}.json"
    plan_path.write_text(json.dumps(plan, ensure_ascii=True, sort_keys=True), encoding="utf-8")
    if args.dry_run:
        print(f"fields={plan['fieldCount']} mutations={plan['mutationCount']} artifact={plan_path}")
        return 0
    if not plan["mutationCount"]:
        print(f"fields=0 mutations=0 artifact={plan_path}")
        return 1
    guard_target(args.base_url, False)
    if not args.pwcli:
        raise SystemExit("set PWCLI or pass --pwcli; no browser fallback is used")
    if args.setup_case == bool(args.case_id):
        raise SystemExit("choose exactly one of --setup-case or --case-id")
    case_id = setup_case(args) if args.setup_case else args.case_id
    result = run_browser(args, plan_path, str(case_id))
    result_path = out_dir / f"ui-result-{args.seed}-{args.shard_index}.json"
    result_path.write_text(json.dumps(result, ensure_ascii=True, indent=2, sort_keys=True), encoding="utf-8")
    counts = result.get("counts", {})
    failures = sum(counts.get(name, 0) for name in {
        "FAIL", "FIELD_MISSING", "NOT_RUN", "UNRENDERABLE", "NO_EXPECTATION",
    })
    failures += len(result.get("results", [])) != plan["mutationCount"]
    print(f"fields={plan['fieldCount']} mutations={plan['mutationCount']} counts={json.dumps(result.get('counts', {}), sort_keys=True)} artifact={result_path}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(parser().parse_args()))
