#!/usr/bin/env python3
"""Run the DG core edit, NullFlavor, delete, and reread checks."""

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
RESULT = ROOT / "dg-core-roundtrip.json"
SOURCE = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()
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
        const [method,path,payload,reason] = arguments;
        const done = arguments[arguments.length - 1];
        const headers = {};
        if (payload !== null) headers['Content-Type'] = 'application/json';
        if (reason !== null) {
          headers['x-e2br3-reason-for-change'] = reason;
          headers['x-e2br3-change-category'] = 'Edited Data';
        }
        fetch(path,{method,credentials:'same-origin',headers,
          body:payload === null ? undefined : JSON.stringify(payload)})
          .then(async response => done({status:response.status,text:await response.text()}))
          .catch(error => done({error:String(error)}));
        """,
        method, path, payload, reason,
    )
    if "error" in result:
        raise AssertionError(result)
    result["body"] = json.loads(result["text"]) if result["text"] else None
    del result["text"]
    return result


def login(driver):
    driver.get(UI_URL + "/")
    WebDriverWait(driver, 60).until(EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]"))).send_keys(constant("DEMO_CRO_ADMIN_EMAIL"))
    driver.find_element(By.CSS_SELECTOR, "input[type=password]").send_keys(constant("INITIAL_PASSWORD"))
    driver.find_element(By.CSS_SELECTOR, "button[type=submit]").click()
    WebDriverWait(driver, 60).until(lambda d: not d.find_elements(By.CSS_SELECTOR, "input[type=email]"))


def click(driver, label):
    node = WebDriverWait(driver, 45).until(
        lambda d: next((item for item in d.find_elements(By.TAG_NAME, "button")
                        if item.is_displayed() and item.is_enabled() and item.text.strip() == label), False)
    )
    driver.execute_script("arguments[0].scrollIntoView({block:'center'})", node)
    node.click()


def save(driver, reason):
    click(driver, "Save")
    dialog = WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located((By.XPATH, "//*[normalize-space()='Confirm']/ancestor::*[@role='dialog'][1]"))
    )
    category = dialog.find_elements(By.CSS_SELECTOR, 'input[value="Edited Data"]') or dialog.find_elements(By.CSS_SELECTOR, 'input[value="New Data"]')
    if len(category) != 1:
        raise AssertionError("save dialog audit category is not unique")
    category[0].click()
    dialog.find_element(By.CSS_SELECTOR, 'textarea[aria-label="Reason comments"]').send_keys(reason)
    dialog.find_element(By.XPATH, ".//button[normalize-space()='Confirm']").click()
    WebDriverWait(driver, 60).until(EC.invisibility_of_element(dialog))
    WebDriverWait(driver, 60).until(
        lambda d: "saving…" not in d.find_element(By.TAG_NAME, "body").text.lower()
        and "saved" in d.find_element(By.TAG_NAME, "body").text.lower()
    )


def open_drug(driver, row_id):
    driver.get(f"{UI_URL}/US/case/{CASE_ID}/detail/DG/{row_id}")
    loaded(driver)


def drug(driver, row_id):
    response = api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/DG/rows/{row_id}?authorities=ich,fda")
    if response["status"] != 200:
        raise AssertionError(response)
    return response["body"]["data"]["drug"]


def live_ids(driver):
    response = api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/DG?authorities=ich,fda")
    if response["status"] != 200:
        raise AssertionError(response)
    return [item["id"] for item in response["body"]["rows"]["rows"]]


def create_drug(driver, label, sequence):
    row = {
        "drugCharacterization":"1", "medicinalProduct":label,
        "medicinalProductNotation":None, "investigationalProductBlinded":None,
        "mpidVersion":None, "mpid":None, "phpidVersion":None, "phpid":None,
        "obtainDrugCountry":None, "fdaAdditionalInfoCoded":None,
        "fdaAdditionalInfoCodedNullFlavor":None, "drugAuthorizationCountry":None,
        "drugAuthorizationHolder":None, "holderApplicantNameNotation":None,
        "drugAuthorizationNumber":None, "drugBatchNumber":None,
        "cumulativeDoseValue":None, "cumulativeDoseUnit":None,
        "gestationPeriodExposureValue":None, "gestationPeriodExposureUnit":None,
        "drugIndication":None, "drugIndicationMeddraCode":None,
        "drugIndicationMeddraVersion":None, "drugActionTaken":None,
        "drugAssessmentNotApplicable":False, "drugAdditionalInformationCodes":[],
        "drugAdditionalInformation":None, "fdaSpecializedProductCategory":None,
        "fdaOtherCharacterization":None,
        "fdaDeviceInfo":{"malfunction":None,"followUpTypes":[],"deviceProblemCodes":[],
          "deviceBrandName":None,"commonDeviceName":None,"deviceProductCode":None,
          "manufacturerName":None,"manufacturerAddress":None,"manufacturerCity":None,
          "manufacturerState":None,"manufacturerCountry":None,"deviceUsage":None,
          "deviceLotNumber":None,"operatorOfDevice":None,"remedialActions":[]},
        "activeSubstances":[], "indications":[], "dosageInformation":[],
        "drugReactionAssessments":[], "deviceCharacteristics":[], "sequenceNumber":sequence,
    }
    response = api(driver, "PATCH", f"/api/cases/{CASE_ID}/editor/pages/DG/rows",
        {"authorities":["ich","fda"],"operations":[{"operation":"create","clientRowIndex":0,"rows":{"drug":row}}]},
        "Adaptive DG verification row")
    if response["status"] != 200:
        raise AssertionError(response)
    return response["body"]["results"][0]["rowId"]


def db_oracle(row_ids):
    quoted = ",".join(f"'{value}'::uuid" for value in row_ids)
    sql = f"""
    SELECT json_build_object(
      'drugs', COALESCE((SELECT json_agg(json_build_object('id',id,
        'medicinalProduct',medicinal_product,'fdaValue',fda_additional_info_coded,
        'fdaNullFlavor',fda_additional_info_coded_null_flavor,'deleted',deleted) ORDER BY id)
        FROM drug_information WHERE id IN ({quoted})), '[]'::json),
      'audit', COALESCE((SELECT json_agg(json_build_object('recordId',record_id,
        'action',action,'changedFields',changed_fields,'reason',reason_for_change) ORDER BY id)
        FROM audit_logs WHERE record_id IN ({quoted})), '[]'::json)
    );
    """
    result = subprocess.run(["docker","exec",os.environ["POSTGRES_CONTAINER"],"psql","-U","postgres",
        "-d",os.environ["STATE_VERIFIER_DB"],"-At","-v","ON_ERROR_STOP=1","-c",sql],
        check=True,capture_output=True,text=True)
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
    suffix = str(int(time.time()))
    base_sequence = 950000 + (int(suffix) % 10000) * 2
    survivor_id = create_drug(driver, f"DG-SURVIVOR-{suffix}", base_sequence)
    delete_id = create_drug(driver, f"DG-DELETE-{suffix}", base_sequence + 1)
    proof["rowIds"] = {"survivor":survivor_id,"deleted":delete_id}
    record("create-two-real-dg-rows", survivor=survivor_id, deleted=delete_id)

    value = f"DG-DIRECT-EDIT-{suffix}"
    open_drug(driver, survivor_id)
    medicinal = WebDriverWait(driver, 45).until(EC.visibility_of_element_located((By.CSS_SELECTOR, '[data-e2b-field-number="G.k.2.2"] input')))
    medicinal.clear(); medicinal.send_keys(value)
    save(driver, "DG direct medicinal product save")
    saved = drug(driver, survivor_id)
    if saved["medicinal_product"] != value:
        raise AssertionError(saved)
    driver.refresh(); loaded(driver)
    if driver.find_element(By.CSS_SELECTOR, '[data-e2b-field-number="G.k.2.2"] input').get_attribute("value") != value:
        raise AssertionError("DG value did not survive reload")
    record("dg-direct-save-reload", medicinalProduct=value)

    nf = Select(driver.find_element(By.CSS_SELECTOR, 'select[aria-label="Null flavor for Additional Information on Drug"]'))
    nf.select_by_value("NA")
    save(driver, "DG FDA.G.k.10a NullFlavor NA")
    nf_saved = drug(driver, survivor_id)
    if nf_saved["fda_additional_info_coded"] is not None or nf_saved["fda_additional_info_coded_null_flavor"] != "NA":
        raise AssertionError(nf_saved)
    driver.refresh(); loaded(driver)
    nf = Select(driver.find_element(By.CSS_SELECTOR, 'select[aria-label="Null flavor for Additional Information on Drug"]'))
    if nf.first_selected_option.get_attribute("value") != "NA":
        raise AssertionError("DG NullFlavor did not survive reload")
    record("dg-nullflavor-save-reload", value=None, nullFlavor="NA")

    nf.select_by_value("")
    save(driver, "DG clear FDA.G.k.10a NullFlavor")
    cleared = drug(driver, survivor_id)
    if cleared["fda_additional_info_coded_null_flavor"] is not None:
        raise AssertionError(cleared)
    record("dg-nullflavor-cleared")

    open_drug(driver, delete_id)
    click(driver, "Remove")
    save(driver, "DG delete repeatable product row")
    deleted = drug(driver, delete_id)
    survivor = drug(driver, survivor_id)
    ids = live_ids(driver)
    if not deleted["deleted"] or delete_id in ids or survivor["medicinal_product"] != value:
        raise AssertionError({"deleted":deleted,"liveIds":ids,"survivor":survivor})
    record("dg-repeatable-row-ui-delete", siblingPreserved=True, deletedAbsent=True)

    click(driver, "Log out")
    WebDriverWait(driver, 45).until(EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]")))
    login(driver)
    open_drug(driver, survivor_id)
    if driver.find_element(By.CSS_SELECTOR, '[data-e2b-field-number="G.k.2.2"] input').get_attribute("value") != value:
        raise AssertionError("DG survivor did not survive relogin")
    ids = live_ids(driver)
    if survivor_id not in ids or delete_id in ids:
        raise AssertionError(ids)
    driver.save_screenshot(str(ROOT / "dg-core-final.png"))
    record("dg-relogin-readback", survivorValue=value, deletedAbsent=True)

    db = db_oracle([survivor_id, delete_id])
    by_id = {item["id"]:item for item in db["drugs"]}
    if by_id[survivor_id] != {"id":survivor_id,"medicinalProduct":value,"fdaValue":None,"fdaNullFlavor":None,"deleted":False}:
        raise AssertionError(db)
    if not by_id[delete_id]["deleted"]:
        raise AssertionError(db)
    if {survivor_id,delete_id} - {item["recordId"] for item in db["audit"]}:
        raise AssertionError("DG audit records incomplete")
    record("dg-database-and-audit-oracle", database=db)
    proof["completed"] = True
    proof["terminal"] = "PASS"
except Exception as error:
    proof["error"] = f"{type(error).__name__}: {error}"
    proof["bodyOnError"] = driver.find_element(By.TAG_NAME, "body").text[-5000:]
    driver.save_screenshot(str(ROOT / "dg-core-error.png"))
    raise
finally:
    persist()
    driver.quit()
