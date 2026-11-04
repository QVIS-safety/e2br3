#!/usr/bin/env python3
"""Run CI and DG core edit resilience checks through the real headed UI."""

import json
import os
import re
import subprocess
import time
from pathlib import Path

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.firefox.service import Service
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.ui import Select, WebDriverWait


ROOT = Path(__file__).parent
UI_URL = os.environ["UI_URL"]
CASE_ID = json.loads((ROOT / "seed-result.json").read_text())["caseId"]
DB_NAME = os.environ["STATE_VERIFIER_DB"]
SOURCE = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()
RESULT = ROOT / "ci-dg-core-roundtrip.json"
proof = {"completed": False, "caseId": CASE_ID, "steps": []}


def persist():
    RESULT.write_text(json.dumps(proof, ensure_ascii=False, indent=2))


def record(name, **details):
    proof["steps"].append({"name": name, **details})
    persist()


def constant(name):
    match = re.search(rf'const {name}: &str = "([^"]+)";', SOURCE)
    if not match:
        raise RuntimeError(f"missing bootstrap value {name}")
    return match.group(1)


def loaded(driver):
    WebDriverWait(driver, 60).until(
        lambda d: d.execute_script("return document.readyState") == "complete"
        and "Loading" not in d.find_element(By.TAG_NAME, "body").text
    )


def api(driver, method, path, payload=None, reason=None):
    result = driver.execute_async_script(
        """
        const [method, path, payload, reason] = arguments;
        const done = arguments[arguments.length - 1];
        const headers = {};
        if (payload !== null) headers['Content-Type'] = 'application/json';
        if (reason !== null) {
          headers['x-e2br3-reason-for-change'] = reason;
          headers['x-e2br3-change-category'] = 'Edited Data';
        }
        fetch(path, {method, credentials:'same-origin', headers,
          body: payload === null ? undefined : JSON.stringify(payload)})
          .then(async response => done({status:response.status, text:await response.text()}))
          .catch(error => done({error:String(error)}));
        """,
        method,
        path,
        payload,
        reason,
    )
    if "error" in result:
        raise AssertionError(result)
    result["body"] = json.loads(result["text"]) if result["text"] else None
    del result["text"]
    return result


def login(driver):
    driver.get(UI_URL + "/")
    WebDriverWait(driver, 60).until(
        EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]"))
    ).send_keys(constant("DEMO_CRO_ADMIN_EMAIL"))
    driver.find_element(By.CSS_SELECTOR, "input[type=password]").send_keys(constant("INITIAL_PASSWORD"))
    driver.find_element(By.CSS_SELECTOR, "button[type=submit]").click()
    WebDriverWait(driver, 60).until(lambda d: not d.find_elements(By.CSS_SELECTOR, "input[type=email]"))


def button(driver, label):
    node = WebDriverWait(driver, 45).until(
        lambda d: next((item for item in d.find_elements(By.TAG_NAME, "button")
                        if item.is_displayed() and item.is_enabled() and item.text.strip() == label), False)
    )
    driver.execute_script("arguments[0].scrollIntoView({block:'center'})", node)
    return node


def save(driver, reason):
    button(driver, "Save").click()
    dialog = WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located((By.XPATH, "//*[normalize-space()='Confirm']/ancestor::*[@role='dialog'][1]"))
    )
    categories = dialog.find_elements(By.CSS_SELECTOR, 'input[value="Edited Data"]')
    if not categories:
        categories = dialog.find_elements(By.CSS_SELECTOR, 'input[value="New Data"]')
    if len(categories) != 1:
        raise AssertionError("save dialog audit category is not unique")
    categories[0].click()
    dialog.find_element(By.CSS_SELECTOR, 'textarea[aria-label="Reason comments"]').send_keys(reason)
    dialog.find_element(By.XPATH, ".//button[normalize-space()='Confirm']").click()
    WebDriverWait(driver, 60).until(EC.invisibility_of_element(dialog))
    WebDriverWait(driver, 60).until(
        lambda d: "saving…" not in d.find_element(By.TAG_NAME, "body").text.lower()
        and "saved" in d.find_element(By.TAG_NAME, "body").text.lower()
    )


def page(driver, section, authorities):
    response = api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/{section}?authorities={authorities}")
    if response["status"] != 200:
        raise AssertionError(response)
    return response["body"]


def drug(driver, row_id):
    response = api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/DG/rows/{row_id}?authorities=ich,fda")
    if response["status"] != 200:
        raise AssertionError(response)
    return response["body"]["data"]["drug"]


def open_section(driver, authority, section, row_id=None):
    suffix = f"/{row_id}" if row_id else ""
    driver.get(f"{UI_URL}/{authority}/case/{CASE_ID}/detail/{section}{suffix}")
    loaded(driver)


def create_drug(driver, label, sequence):
    drug_row = {
        "drugCharacterization": "1", "medicinalProduct": label,
        "medicinalProductNotation": None, "investigationalProductBlinded": None,
        "mpidVersion": None, "mpid": None, "phpidVersion": None, "phpid": None,
        "obtainDrugCountry": None, "fdaAdditionalInfoCoded": None,
        "fdaAdditionalInfoCodedNullFlavor": None,
        "drugAuthorizationCountry": None, "drugAuthorizationHolder": None,
        "holderApplicantNameNotation": None, "drugAuthorizationNumber": None,
        "drugBatchNumber": None, "cumulativeDoseValue": None, "cumulativeDoseUnit": None,
        "gestationPeriodExposureValue": None, "gestationPeriodExposureUnit": None,
        "drugIndication": None, "drugIndicationMeddraCode": None,
        "drugIndicationMeddraVersion": None, "drugActionTaken": None,
        "drugAssessmentNotApplicable": False, "drugAdditionalInformationCodes": [],
        "drugAdditionalInformation": None, "fdaSpecializedProductCategory": None,
        "fdaOtherCharacterization": None,
        "fdaDeviceInfo": {"malfunction": None, "followUpTypes": [], "deviceProblemCodes": [],
            "deviceBrandName": None, "commonDeviceName": None, "deviceProductCode": None,
            "manufacturerName": None, "manufacturerAddress": None, "manufacturerCity": None,
            "manufacturerState": None, "manufacturerCountry": None, "deviceUsage": None,
            "deviceLotNumber": None, "operatorOfDevice": None, "remedialActions": []},
        "activeSubstances": [], "indications": [], "dosageInformation": [],
        "drugReactionAssessments": [], "deviceCharacteristics": [],
        "sequenceNumber": sequence,
    }
    response = api(
        driver, "PATCH", f"/api/cases/{CASE_ID}/editor/pages/DG/rows",
        {"authorities": ["ich", "fda"], "operations": [
            {"operation": "create", "clientRowIndex": 0, "rows": {"drug": drug_row}}
        ]},
        "Adaptive CI DG verification row",
    )
    if response["status"] != 200:
        raise AssertionError(response)
    return response["body"]["results"][0]["rowId"]


def database_oracle(sri_id, drug_ids, reasons):
    quoted_ids = ",".join(f"'{value}'::uuid" for value in drug_ids)
    quoted_records = ",".join(f"'{value}'::uuid" for value in [CASE_ID, sri_id, *drug_ids])
    quoted_reasons = ",".join("'" + value.replace("'", "''") + "'" for value in reasons)
    sql = f"""
    SELECT json_build_object(
      'ci', (SELECT json_build_object('reportYear', c.report_year,
        'fulfilValue', s.fulfil_expedited_criteria,
        'fulfilNullFlavor', s.fulfil_expedited_criteria_null_flavor)
        FROM cases c JOIN safety_report_identification s ON s.case_id=c.id WHERE c.id='{CASE_ID}'),
      'drugs', COALESCE((SELECT json_agg(json_build_object('id',id,
        'medicinalProduct',medicinal_product,'fdaNullFlavor',fda_additional_info_coded_null_flavor,
        'deleted',deleted) ORDER BY id) FROM drug_information WHERE id IN ({quoted_ids})), '[]'::json),
      'audit', COALESCE((SELECT json_agg(json_build_object('recordId',record_id,'action',action,
        'changedFields',changed_fields,'reason',reason_for_change) ORDER BY id)
        FROM audit_logs WHERE record_id IN ({quoted_records}) AND reason_for_change IN ({quoted_reasons})), '[]'::json)
    );
    """
    result = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "psql", "-U", "postgres", "-d", DB_NAME,
         "-At", "-v", "ON_ERROR_STOP=1", "-c", sql],
        check=True, capture_output=True, text=True,
    )
    return json.loads(result.stdout.strip())


options = webdriver.FirefoxOptions()
options.binary_location = os.environ["FIREFOX_BINARY"]
driver = webdriver.Firefox(service=Service(os.environ["GECKODRIVER"]), options=options)
driver.set_window_size(1440, 1000)
driver.set_script_timeout(45)

try:
    if driver.capabilities.get("moz:headless") is not False:
        raise AssertionError("Firefox is not headed")
    login(driver)
    baseline_ci = page(driver, "CI", "ich")["rows"]
    sri_id = baseline_ci["safetyReportIdentification"]["id"]
    if baseline_ci["case"]["reportYear"] is not None:
        raise AssertionError("CI reportYear baseline is not empty")
    if baseline_ci["safetyReportIdentification"]["fulfilExpeditedCriteria"] is not None or baseline_ci["safetyReportIdentification"]["fulfilExpeditedCriteriaNullFlavor"] is not None:
        raise AssertionError("CI C.1.7 baseline is not empty")
    proof["ciRecordId"] = sri_id

    open_section(driver, "ICH", "CI")
    report_year = WebDriverWait(driver, 45).until(EC.visibility_of_element_located((By.CSS_SELECTOR, '[name="case.reportYear"]')))
    report_year.send_keys("2026")
    save(driver, "CI direct report year save")
    ci_saved = page(driver, "CI", "ich")["rows"]
    if ci_saved["case"]["reportYear"] != "2026":
        raise AssertionError(ci_saved)
    driver.refresh(); loaded(driver)
    if driver.find_element(By.CSS_SELECTOR, '[name="case.reportYear"]').get_attribute("value") != "2026":
        raise AssertionError("CI reportYear did not survive reload")
    record("ci-direct-save-reload", reportYear="2026")

    nf_button = driver.find_element(By.CSS_SELECTOR, 'button[aria-label="safetyReportIdentification.fulfilExpeditedCriteriaNullFlavor nullFlavor NI"]')
    nf_button.click()
    save(driver, "CI C.1.7 NullFlavor NI")
    ci_nf = page(driver, "CI", "ich")["rows"]
    sri = ci_nf["safetyReportIdentification"]
    if sri["fulfilExpeditedCriteria"] is not None or sri["fulfilExpeditedCriteriaNullFlavor"] != "NI":
        raise AssertionError(ci_nf)
    driver.refresh(); loaded(driver)
    nf_button = driver.find_element(By.CSS_SELECTOR, 'button[aria-label="safetyReportIdentification.fulfilExpeditedCriteriaNullFlavor nullFlavor NI"]')
    if nf_button.get_attribute("aria-pressed") != "true":
        raise AssertionError("CI NullFlavor did not survive reload")
    record("ci-nullflavor-save-reload", value=None, nullFlavor="NI")

    button(driver, "Log out").click()
    WebDriverWait(driver, 45).until(EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]")))
    login(driver)
    open_section(driver, "ICH", "CI")
    if driver.find_element(By.CSS_SELECTOR, '[name="case.reportYear"]').get_attribute("value") != "2026":
        raise AssertionError("CI reportYear did not survive relogin")
    nf_button = driver.find_element(By.CSS_SELECTOR, 'button[aria-label="safetyReportIdentification.fulfilExpeditedCriteriaNullFlavor nullFlavor NI"]')
    if nf_button.get_attribute("aria-pressed") != "true":
        raise AssertionError("CI NullFlavor did not survive relogin")
    record("ci-relogin-readback", reportYear="2026", nullFlavor="NI")

    report_year = driver.find_element(By.CSS_SELECTOR, '[name="case.reportYear"]')
    report_year.clear()
    nf_button.click()
    save(driver, "CI restore original empty values")
    ci_restored = page(driver, "CI", "ich")["rows"]
    if ci_restored["case"]["reportYear"] is not None:
        raise AssertionError(ci_restored)
    sri = ci_restored["safetyReportIdentification"]
    if sri["fulfilExpeditedCriteria"] is not None or sri["fulfilExpeditedCriteriaNullFlavor"] is not None:
        raise AssertionError(ci_restored)
    record("ci-original-restored")

    suffix = str(int(time.time()))
    base_sequence = 930000 + (int(suffix) % 10000) * 2
    survivor_id = create_drug(driver, f"DG-SURVIVOR-{suffix}", base_sequence)
    delete_id = create_drug(driver, f"DG-DELETE-{suffix}", base_sequence + 1)
    proof["dgRowIds"] = {"survivor": survivor_id, "deleted": delete_id}
    record("dg-create-two-real-rows", survivor=survivor_id, deleted=delete_id)

    dg_value = f"DG-DIRECT-EDIT-{suffix}"
    open_section(driver, "FDA", "DG", survivor_id)
    medicinal = WebDriverWait(driver, 45).until(EC.visibility_of_element_located((By.CSS_SELECTOR, '[data-e2b-field-number="G.k.2.2"] input')))
    medicinal.clear(); medicinal.send_keys(dg_value)
    save(driver, "DG direct medicinal product save")
    dg_saved = drug(driver, survivor_id)
    if dg_saved["medicinal_product"] != dg_value:
        raise AssertionError(dg_saved)
    driver.refresh(); loaded(driver)
    medicinal = driver.find_element(By.CSS_SELECTOR, '[data-e2b-field-number="G.k.2.2"] input')
    if medicinal.get_attribute("value") != dg_value:
        raise AssertionError("DG medicinal product did not survive reload")
    record("dg-direct-save-reload", medicinalProduct=dg_value)

    dg_nf = Select(driver.find_element(By.CSS_SELECTOR, 'select[aria-label="Null flavor for Additional Information on Drug"]'))
    dg_nf.select_by_value("NA")
    save(driver, "DG FDA.G.k.10a NullFlavor NA")
    dg_nf_saved = drug(driver, survivor_id)
    if dg_nf_saved["fda_additional_info_coded"] is not None or dg_nf_saved["fda_additional_info_coded_null_flavor"] != "NA":
        raise AssertionError(dg_nf_saved)
    driver.refresh(); loaded(driver)
    dg_nf = Select(driver.find_element(By.CSS_SELECTOR, 'select[aria-label="Null flavor for Additional Information on Drug"]'))
    if dg_nf.first_selected_option.get_attribute("value") != "NA":
        raise AssertionError("DG NullFlavor did not survive reload")
    record("dg-nullflavor-save-reload", value=None, nullFlavor="NA")

    dg_nf.select_by_value("")
    save(driver, "DG clear FDA.G.k.10a NullFlavor")
    dg_nf_cleared = drug(driver, survivor_id)
    if dg_nf_cleared["fda_additional_info_coded_null_flavor"] is not None:
        raise AssertionError(dg_nf_cleared)
    record("dg-nullflavor-cleared")

    open_section(driver, "FDA", "DG", delete_id)
    button(driver, "Remove").click()
    save(driver, "DG delete repeatable product row")
    deleted = drug(driver, delete_id)
    dg_page = page(driver, "DG", "ich,fda")
    live_ids = [item["id"] for item in dg_page["rows"]["rows"]]
    survivor = drug(driver, survivor_id)
    if not deleted["deleted"] or delete_id in live_ids or survivor["medicinal_product"] != dg_value:
        raise AssertionError({"deleted": deleted, "liveIds": live_ids, "survivor": survivor})
    record("dg-repeatable-row-ui-delete", siblingPreserved=True, deletedAbsent=True)

    button(driver, "Log out").click()
    WebDriverWait(driver, 45).until(EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]")))
    login(driver)
    open_section(driver, "FDA", "DG", survivor_id)
    medicinal = driver.find_element(By.CSS_SELECTOR, '[data-e2b-field-number="G.k.2.2"] input')
    if medicinal.get_attribute("value") != dg_value:
        raise AssertionError("DG survivor did not survive relogin")
    final_ids = [item["id"] for item in page(driver, "DG", "ich,fda")["rows"]["rows"]]
    if survivor_id not in final_ids or delete_id in final_ids:
        raise AssertionError(final_ids)
    driver.save_screenshot(str(ROOT / "ci-dg-core-final.png"))
    record("dg-relogin-readback", survivorValue=dg_value, deletedAbsent=True)

    reasons = [
        "Adaptive CI DG verification row",
        "Edited Data: CI direct report year save", "Edited Data: CI C.1.7 NullFlavor NI",
        "Edited Data: CI restore original empty values",
        "Edited Data: DG direct medicinal product save",
        "Edited Data: DG FDA.G.k.10a NullFlavor NA",
        "Edited Data: DG clear FDA.G.k.10a NullFlavor",
        "Edited Data: DG delete repeatable product row",
    ]
    db = database_oracle(sri_id, [survivor_id, delete_id], reasons)
    if db["ci"] != {"reportYear": None, "fulfilValue": None, "fulfilNullFlavor": None}:
        raise AssertionError(db)
    by_id = {item["id"]: item for item in db["drugs"]}
    if by_id[survivor_id]["medicinalProduct"] != dg_value or by_id[survivor_id]["fdaNullFlavor"] is not None or by_id[survivor_id]["deleted"]:
        raise AssertionError(db)
    if not by_id[delete_id]["deleted"]:
        raise AssertionError(db)
    expected_records = {CASE_ID, sri_id, survivor_id, delete_id}
    if not expected_records.issubset({item["recordId"] for item in db["audit"]}):
        raise AssertionError("CI/DG target audit records are incomplete")
    record("ci-dg-database-and-audit-oracle", database=db)
    proof["completed"] = True
    proof["terminal"] = "PASS"
except Exception as error:
    proof["error"] = f"{type(error).__name__}: {error}"
    proof["bodyOnError"] = driver.find_element(By.TAG_NAME, "body").text[-5000:]
    driver.save_screenshot(str(ROOT / "ci-dg-core-error.png"))
    raise
finally:
    persist()
    driver.quit()
