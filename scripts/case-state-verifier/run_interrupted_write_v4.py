#!/usr/bin/env python3
"""Lose a real PATCH response, then resolve the outcome before any retry."""

import json
import os
import subprocess
import time
from pathlib import Path

from selenium.common.exceptions import WebDriverException


ROOT = Path(__file__).parent
V3 = ROOT / "run_concurrent_edit_v3.py"
scope = {"__file__": str(V3)}
exec(compile(V3.read_text().split("\nadmin = admin_client()", 1)[0], str(V3), "exec"), scope)

CASE_ID = scope["CASE_ID"]
DB_NAME = scope["DB_NAME"]
RESULT = ROOT / "interrupted-write-v4-result.json"
proof = {"completed": False, "caseId": CASE_ID, "grammar": "adaptive-case-edit-grammar-v4.json"}


def sql_json(sql):
    result = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "psql", "-U", "postgres", "-d", DB_NAME,
         "-At", "-v", "ON_ERROR_STOP=1", "-c", sql],
        check=True, capture_output=True, text=True,
    )
    return json.loads(result.stdout.strip())


driver = scope["new_driver"]()
try:
    scope["login"](
        driver,
        scope["constant"]("DEMO_CRO_ADMIN_EMAIL"),
        scope["constant"]("INITIAL_PASSWORD"),
    )
    scope["open_section"](driver, "CI")
    baseline = scope["ci_rows"](driver)["case"]["reportYear"]
    if baseline is not None:
        raise AssertionError(f"CI reportYear baseline is not empty: {baseline}")

    audit_floor = int(scope["database_value"]("SELECT COALESCE(max(id),0) FROM audit_logs;"))
    intended = "2029"
    reason = "Adaptive v4 lost response report year"
    path = f"/api/cases/{CASE_ID}/editor/pages/CI"
    payload = {
        "authorities": ["ich"],
        "rows": {"case": {"reportYear": intended}},
    }
    old_document = driver.find_element(scope["By"].TAG_NAME, "html")
    try:
        driver.execute_script(
            """
            const [path, payload, reason] = arguments;
            sessionStorage.setItem('interruptedWriteResponseObserved', '0');
            const request = new XMLHttpRequest();
            request.open('PATCH', path);
            request.setRequestHeader('Content-Type', 'application/json');
            request.setRequestHeader('x-e2br3-reason-for-change', reason);
            request.setRequestHeader('x-e2br3-change-category', 'Edited Data');
            request.onload = () => sessionStorage.setItem('interruptedWriteResponseObserved', '1');
            request.upload.onload = () => {
              request.abort();
              location.reload();
            };
            request.send(JSON.stringify(payload));
            """,
            path, payload, reason,
        )
    except WebDriverException:
        pass

    scope["WebDriverWait"](driver, 30).until(scope["EC"].staleness_of(old_document))
    scope["loaded"](driver)
    deadline = time.time() + 15
    reread = None
    while time.time() < deadline:
        reread = scope["ci_rows"](driver)["case"]["reportYear"]
        if reread == intended:
            break
        time.sleep(0.25)
    if reread != intended:
        raise AssertionError(f"interrupted write did not reach a committed terminal state: {reread}")
    response_observed = driver.execute_script(
        "return sessionStorage.getItem('interruptedWriteResponseObserved')"
    )
    if response_observed != "0":
        raise AssertionError("write response settled before the document interruption")

    audit = sql_json(f"""
      SELECT json_agg(json_build_object(
        'id',id,'userId',user_id,'reason',reason_for_change,'changedFields',changed_fields
      ) ORDER BY id)
      FROM audit_logs
      WHERE id > {audit_floor} AND reason_for_change = '{reason}';
    """)
    if not isinstance(audit, list) or len(audit) != 1:
        raise AssertionError({"lostResponseAudit": audit})
    if audit[0]["changedFields"].get("report_year", {}).get("new") != intended:
        raise AssertionError(audit[0])

    restore = scope["browser_api"](
        driver, "PATCH", path,
        {"authorities": ["ich"], "rows": {"case": {"reportYear": None}}},
        "Adaptive v4 lost response restore",
    )
    if restore["status"] != 200 or scope["ci_rows"](driver)["case"]["reportYear"] is not None:
        raise AssertionError({"restore": restore})
    integrity = sql_json("SELECT row_to_json(v) FROM verify_audit_log_hash_chain(NULL) v;")
    if integrity["broken_rows"] != 0:
        raise AssertionError(integrity)

    proof.update({
        "completed": True,
        "responseObserved": False,
        "canonicalReadback": intended,
        "retryIssued": False,
        "audit": audit,
        "restored": True,
        "integrity": integrity,
    })
finally:
    driver.quit()
    RESULT.write_text(json.dumps(proof, ensure_ascii=False, indent=2))

print(json.dumps({
    "completed": proof["completed"],
    "responseObserved": proof.get("responseObserved"),
    "retryIssued": proof.get("retryIssued"),
    "restored": proof.get("restored"),
}))
