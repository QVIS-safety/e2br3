#!/usr/bin/env python3
"""Exercise direct-page nested repeatables through their real Add controls."""

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
RESULT = ROOT / "rp-si-nr-ui-v5-result.json"
proof = {"completed": False, "caseId": CASE_ID, "grammar": "adaptive-case-edit-grammar-v5.json", "steps": []}


def rows(driver, section):
    result = scope["browser_api"](
        driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/{section}?authorities=ich"
    )
    if result["status"] != 200:
        raise AssertionError(result)
    return result["body"]["rows"]


def fill(driver, name, value):
    field = scope["WebDriverWait"](driver, 30).until(
        lambda current: current.find_element(scope["By"].CSS_SELECTOR, f'[name="{name}"]')
    )
    field.clear()
    field.send_keys(value)


def autocomplete(driver, label, index, query):
    controls = scope["WebDriverWait"](driver, 30).until(
        lambda current: current.find_elements(
            scope["By"].CSS_SELECTOR,
            f'button[role="combobox"][aria-label="{label}"]',
        )
    )
    controls[index].click()
    field = scope["WebDriverWait"](driver, 30).until(
        lambda current: next(
            (node for node in current.find_elements(scope["By"].CSS_SELECTOR, '[role="dialog"] input[role="combobox"]') if node.is_displayed()),
            False,
        )
    )
    field.send_keys(query)
    scope["WebDriverWait"](driver, 30).until(
        lambda current: current.execute_script(
            """
            const query = arguments[0].toLowerCase();
            const option = [...document.querySelectorAll('[role="option"]')]
              .find(node => node.offsetParent !== null && node.innerText.toLowerCase().includes(query));
            if (!option) return false;
            option.click();
            return true;
            """,
            query,
        )
    )


def relogin(driver, section):
    scope["button"](driver, "Log out").click()
    scope["WebDriverWait"](driver, 45).until(
        scope["EC"].presence_of_element_located((scope["By"].CSS_SELECTOR, "input[type=email]"))
    )
    scope["login"](driver, scope["constant"]("DEMO_CRO_ADMIN_EMAIL"), scope["constant"]("INITIAL_PASSWORD"))
    scope["open_section"](driver, section)


def record(name, **details):
    proof["steps"].append({"name": name, **details})


driver = scope["new_driver"]()
try:
    scope["login"](driver, scope["constant"]("DEMO_CRO_ADMIN_EMAIL"), scope["constant"]("INITIAL_PASSWORD"))

    rp_label = "V5 UI REPORTER"
    rp_before = rows(driver, "RP")["primarySources"]
    scope["open_section"](driver, "RP")
    scope["button"](driver, "Add Reporter").click()
    rp_index = len(rp_before)
    fill(driver, f"primarySources.{rp_index}.reporterOrganization", rp_label)
    driver.find_element(
        scope["By"].CSS_SELECTOR,
        '[data-e2b-field-number="C.2.r.4"] input[value="1"]',
    ).click()
    scope["save"](driver, "Adaptive v5 RP UI create")
    relogin(driver, "RP")
    rp_created = rows(driver, "RP")["primarySources"]
    if not any(row.get("reporterOrganization") == rp_label for row in rp_created):
        raise AssertionError(rp_created)
    reporter_row = next(
        row for row in driver.find_elements(scope["By"].CSS_SELECTOR, "tbody tr") if rp_label in row.text
    )
    driver.execute_script(
        "arguments[0].scrollIntoView({block:'center'}); arguments[0].click()",
        reporter_row,
    )
    scope["button"](driver, "Remove").click()
    scope["save"](driver, "Adaptive v5 RP UI delete")
    driver.refresh(); scope["loaded"](driver)
    if any(row.get("reporterOrganization") == rp_label for row in rows(driver, "RP")["primarySources"]):
        raise AssertionError("RP reporter survived UI delete")
    record("rp-reporter-create-delete-reload")

    si_label = "V5-REGISTRATION"
    si_before = rows(driver, "SI")["studyRegistrationNumbers"]
    scope["open_section"](driver, "SI")
    scope["button"](driver, "Add Registration").click()
    si_index = sum(not row.get("deleted") for row in si_before)
    fill(driver, f"studyInformation.studyRegistrationNumbers.{si_index}.registrationNumber", si_label)
    autocomplete(driver, "Study Registration Country", si_index, "United States")
    scope["save"](driver, "Adaptive v5 SI UI create")
    relogin(driver, "SI")
    si_created = rows(driver, "SI")["studyRegistrationNumbers"]
    if not any(row.get("registration_number") == si_label for row in si_created):
        raise AssertionError(si_created)
    registration = driver.find_element(
        scope["By"].CSS_SELECTOR,
        f'input[value="{si_label}"]',
    ).find_element(scope["By"].XPATH, "ancestor::tr[1]")
    registration.find_element(scope["By"].CSS_SELECTOR, 'button[aria-label^="Remove row "]').click()
    scope["save"](driver, "Adaptive v5 SI UI delete")
    driver.refresh(); scope["loaded"](driver)
    if any(row.get("registration_number") == si_label and not row.get("deleted") for row in rows(driver, "SI")["studyRegistrationNumbers"]):
        raise AssertionError("SI registration survived UI delete")
    record("si-registration-create-delete-reload")

    diagnosis_query = "Headache"
    summary_label = "V5 UI CASE SUMMARY"
    meddra_count = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "psql", "-U", "postgres", "-d", DB_NAME,
         "-At", "-v", "ON_ERROR_STOP=1", "-c", "SELECT count(*) FROM meddra_terms WHERE active"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    if meddra_count == "0":
        proof["blocked"] = "active production MedDRA terminology is required for NR diagnosis selection"
        raise RuntimeError(proof["blocked"])
    nr_before = rows(driver, "NR")
    diagnosis_index = sum(not row.get("deleted") for row in nr_before["senderDiagnoses"])
    summary_index = sum(not row.get("deleted") for row in nr_before["caseSummaryInformation"])
    scope["open_section"](driver, "NR")
    scope["button"](driver, "Add Diagnosis").click()
    autocomplete(driver, "Sender's Diagnosis (MedDRA code)", diagnosis_index, diagnosis_query)
    scope["button"](driver, "Add Summary").click()
    fill(driver, f"caseSummaryInformation.{summary_index}.summaryText", summary_label)
    fill(driver, f"caseSummaryInformation.{summary_index}.languageCode", "eng")
    scope["save"](driver, "Adaptive v5 NR UI create")
    relogin(driver, "NR")
    nr_created = rows(driver, "NR")
    created_diagnoses = [
        row for row in nr_created["senderDiagnoses"]
        if not row.get("deleted") and row.get("id") not in {item.get("id") for item in nr_before["senderDiagnoses"]}
    ]
    if len(created_diagnoses) != 1:
        raise AssertionError(nr_created["senderDiagnoses"])
    if not any(row.get("summary_text") == summary_label for row in nr_created["caseSummaryInformation"]):
        raise AssertionError(nr_created["caseSummaryInformation"])
    diagnosis_row = next(
        row for row in driver.find_elements(scope["By"].CSS_SELECTOR, "tbody tr")
        if diagnosis_query.lower() in row.text.lower()
    )
    diagnosis_row.find_element(scope["By"].CSS_SELECTOR, 'button[aria-label^="Remove row "]').click()
    summary_row = driver.find_element(
        scope["By"].CSS_SELECTOR,
        f'textarea[name="caseSummaryInformation.{summary_index}.summaryText"]',
    ).find_element(scope["By"].XPATH, "ancestor::tr[1]")
    summary_row.find_element(scope["By"].CSS_SELECTOR, 'button[aria-label^="Remove row "]').click()
    scope["save"](driver, "Adaptive v5 NR UI delete")
    driver.refresh(); scope["loaded"](driver)
    nr_deleted = rows(driver, "NR")
    created_diagnosis_ids = {row["id"] for row in created_diagnoses}
    if any(row.get("id") in created_diagnosis_ids and not row.get("deleted") for row in nr_deleted["senderDiagnoses"]):
        raise AssertionError(nr_deleted["senderDiagnoses"])
    if any(row.get("summary_text") == summary_label and not row.get("deleted") for row in nr_deleted["caseSummaryInformation"]):
        raise AssertionError(nr_deleted["caseSummaryInformation"])
    record("nr-diagnosis-summary-create-delete-reload")

    integrity = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "psql", "-U", "postgres", "-d", DB_NAME,
         "-At", "-v", "ON_ERROR_STOP=1", "-c",
         "SELECT row_to_json(v) FROM verify_audit_log_hash_chain(NULL) v;"],
        check=True, capture_output=True, text=True,
    )
    proof["integrity"] = json.loads(integrity.stdout.strip())
    if proof["integrity"]["broken_rows"] != 0:
        raise AssertionError(proof["integrity"])
    proof["completed"] = True
finally:
    driver.quit()
    RESULT.write_text(json.dumps(proof, ensure_ascii=False, indent=2))

print(json.dumps(proof))
