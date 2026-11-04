#!/usr/bin/env python3
"""Create and delete DG nested rows only through the rendered controls."""

import json
import os
import subprocess
from pathlib import Path


ROOT = Path(__file__).parent
V3 = ROOT / "run_concurrent_edit_v3.py"
scope = {"__file__": str(V3)}
exec(compile(V3.read_text().split("\nadmin = admin_client()", 1)[0], str(V3), "exec"), scope)

CASE_ID = scope["CASE_ID"]
DB_NAME = scope["DB_NAME"]
ROW_ID = "c869d471-2f99-40be-85c5-8990a09e367d"
RESULT = ROOT / "dg-nested-ui-v4-result.json"
proof = {"completed": False, "caseId": CASE_ID, "rowId": ROW_ID, "grammar": "adaptive-case-edit-grammar-v4.json"}


def read_drug(driver):
    result = scope["browser_api"](
        driver, "GET",
        f"/api/cases/{CASE_ID}/editor/pages/DG/rows/{ROW_ID}?authorities=fda",
    )
    if result["status"] != 200:
        raise AssertionError(result)
    return result["body"]["data"]["drug"]


def click(driver, label):
    scope["button"](driver, label).click()


def fill(driver, name, value):
    field = scope["WebDriverWait"](driver, 30).until(
        lambda current: current.find_element(scope["By"].CSS_SELECTOR, f'[name="{name}"]')
    )
    field.clear()
    field.send_keys(value)


driver = scope["new_driver"]()
try:
    scope["login"](driver, scope["constant"]("DEMO_CRO_ADMIN_EMAIL"), scope["constant"]("INITIAL_PASSWORD"))
    baseline = read_drug(driver)
    collections = ["activeSubstances", "dosageInformation", "indications", "drugReactionAssessments"]
    if any(baseline[name] for name in collections):
        raise AssertionError({name: baseline[name] for name in collections})

    scope["open_section"](driver, "DG", ROW_ID)
    click(driver, "Add Substance")
    fill(driver, "drugs.0.activeSubstances.0.substanceName", "V4 UI SUBSTANCE")
    click(driver, "Add Dosage")
    fill(driver, "drugs.0.dosageInformation.0.dosageText", "V4 UI DOSAGE")
    click(driver, "Add Indication")
    fill(driver, "drugs.0.indications.0.indicationText", "V4 UI INDICATION")
    click(driver, "Add Assessment")
    fill(driver, "drugs.0.drugReactionAssessments.0.sourceOfAssessment", "V4 UI ASSESSOR")
    scope["save"](driver, "Adaptive v4 DG nested UI create")

    driver.refresh()
    scope["loaded"](driver)
    created = read_drug(driver)
    if created["activeSubstances"][0]["substance_name"] != "V4 UI SUBSTANCE":
        raise AssertionError(created["activeSubstances"])
    if created["dosageInformation"][0]["dosage_text"] != "V4 UI DOSAGE":
        raise AssertionError(created["dosageInformation"])
    if created["indications"][0]["indication_text"] != "V4 UI INDICATION":
        raise AssertionError(created["indications"])
    if len(created["drugReactionAssessments"]) != 1:
        raise AssertionError(created["drugReactionAssessments"])

    for label in [
        "Remove active substance row 1",
        "Remove dosage row 1",
        "Remove indication row 1",
        "Remove assessment row 1",
    ]:
        node = driver.find_element(scope["By"].CSS_SELECTOR, f'button[aria-label="{label}"]')
        driver.execute_script("arguments[0].scrollIntoView({block:'center'})", node)
        node.click()
    scope["save"](driver, "Adaptive v4 DG nested UI delete")

    driver.refresh()
    scope["loaded"](driver)
    deleted = read_drug(driver)
    if any(any(not row.get("deleted", False) for row in deleted[name]) for name in collections):
        raise AssertionError({name: deleted[name] for name in collections})

    integrity = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "psql", "-U", "postgres", "-d", DB_NAME,
         "-At", "-v", "ON_ERROR_STOP=1", "-c",
         "SELECT row_to_json(v) FROM verify_audit_log_hash_chain(NULL) v;"],
        check=True, capture_output=True, text=True,
    )
    integrity = json.loads(integrity.stdout.strip())
    if integrity["broken_rows"] != 0:
        raise AssertionError(integrity)
    proof.update({
        "completed": True,
        "created": {name: len(created[name]) for name in collections},
        "deleted": {name: len(deleted[name]) for name in collections},
        "reloadReadback": True,
        "integrity": integrity,
    })
finally:
    driver.quit()
    RESULT.write_text(json.dumps(proof, ensure_ascii=False, indent=2))

print(json.dumps(proof))
