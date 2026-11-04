#!/usr/bin/env python3
"""Execute adaptive grammar v2 for DM, DH, AE, and LB in the real UI."""

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
RESULT = ROOT / "remaining-sections-v2-result.json"
SOURCE = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()
proof = {"completed": False, "grammarVersion": 2, "caseId": CASE_ID, "steps": []}


def persist():
    RESULT.write_text(json.dumps(proof, ensure_ascii=False, indent=2))


def record(section, name, **details):
    proof["steps"].append({"section": section, "name": name, **details})
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


def api(driver, method, path, payload=None, reason=None, expected=(200,)):
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
    if result["status"] not in expected:
        raise AssertionError(result)
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


def open_section(driver, section, row_id=None):
    suffix = f"/{row_id}" if row_id else ""
    expected = f"/ICH/case/{CASE_ID}/detail/{section}{suffix}"
    driver.get(f"{UI_URL}{expected}")
    WebDriverWait(driver, 60).until(
        lambda d: expected in d.current_url
        and d.find_elements(By.CSS_SELECTOR, f'[data-case-section-shell="{section.lower()}"]')
    )
    loaded(driver)


def page(driver, section):
    return api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/{section}?authorities=ich")["body"]


def detail(driver, section, row_id, key):
    return api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/{section}/rows/{row_id}?authorities=ich")["body"]["data"][key]


def create_repeatable(driver, section, key, value, sequence):
    response = api(
        driver, "POST", f"/api/cases/{CASE_ID}/editor/pages/{section}/rows",
        {"authorities":["ich"],"rows":{key:{"sequenceNumber":sequence,**value}}},
        f"Adaptive v2 {section} verification row", expected=(201,),
    )
    return response["body"]["rowId"]


def visible_input(driver, field_number):
    root = WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located((By.CSS_SELECTOR, f'[data-e2b-field-number="{field_number}"]'))
    )
    return next((node for node in root.find_elements(By.CSS_SELECTOR, "input") if node.is_displayed()), None)


def database_oracle(ids):
    q = lambda values: ",".join(f"'{value}'::uuid" for value in values)
    records = [ids["patient"], ids["dhSurvivor"], ids["dhDeleted"], ids["aeSurvivor"], ids["aeDeleted"], ids["lbSurvivor"], ids["lbDeleted"]]
    sql = f"""
    SELECT json_build_object(
      'dm', (SELECT json_build_object('id',id,'initials',patient_initials,
        'nullFlavor',patient_initials_null_flavor) FROM patient_information WHERE id='{ids['patient']}'),
      'dh', COALESCE((SELECT json_agg(json_build_object('id',id,'mpidVersion',mpid_version,
        'startDate',start_date,'startDateNullFlavor',start_date_null_flavor,'deleted',deleted) ORDER BY id)
        FROM past_drug_history WHERE id IN ({q([ids['dhSurvivor'],ids['dhDeleted']])})), '[]'::json),
      'ae', COALESCE((SELECT json_agg(json_build_object('id',id,'reaction',primary_source_reaction,
        'startDate',start_date,'startDateNullFlavor',start_date_null_flavor,'deleted',deleted) ORDER BY id)
        FROM reactions WHERE id IN ({q([ids['aeSurvivor'],ids['aeDeleted']])})), '[]'::json),
      'lb', COALESCE((SELECT json_agg(json_build_object('id',id,'testName',test_name,
        'testDate',test_date,'testDateNullFlavor',test_date_null_flavor,'deleted',deleted) ORDER BY id)
        FROM test_results WHERE id IN ({q([ids['lbSurvivor'],ids['lbDeleted']])})), '[]'::json),
      'audit', COALESCE((SELECT json_agg(json_build_object('recordId',record_id,'action',action,
        'changedFields',changed_fields,'reason',reason_for_change) ORDER BY id)
        FROM audit_logs WHERE record_id IN ({q(records)})), '[]'::json)
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
    recovered_dm = None
    for section in ("DM", "DH", "AE", "LB"):
        expected = ({"autopsyCauses":[],"deathInfo":None,"medicalHistoryEpisodes":[],"parentInfo":None,"parentMedicalHistory":[],"parentPastDrugs":[],"patientIdentifiers":[],"patientInformation":None,"reportedCauses":[]} if section == "DM" else {"rows":[]})
        actual = page(driver, section)["rows"]
        if section == "DM" and actual != expected:
            candidate = actual.get("patientInformation")
            other = {**actual, "patientInformation": None}
            other.pop("parents", None)
            if not isinstance(candidate, dict) or not str(candidate.get("patient_initials", "")).startswith("DM-") or other != expected:
                raise AssertionError(f"{section} baseline contains non-owned data")
            recovered_dm = candidate
        elif actual != expected:
            raise AssertionError(f"{section} baseline is not empty")

    suffix = str(int(time.time()))
    if recovered_dm:
        patient_id = recovered_dm["id"]
        direct_dm = recovered_dm["patient_initials"]
        proof["ids"] = {"patient":patient_id}
        open_section(driver, "DM")
        if visible_input(driver, "D.1").get_attribute("value") != direct_dm:
            raise AssertionError("DM recovered value differs in UI")
        record("DM", "direct-save-reload-recovered", value=direct_dm)
    else:
        dm_value = f"DM-{suffix[-8:]}"
        dm = api(driver, "PATCH", f"/api/cases/{CASE_ID}/editor/pages/DM",
            {"authorities":["ich"],"rows":{"patientInformation":{"patientInitials":dm_value,"patientInitialsNullFlavor":None}}},
            "Adaptive v2 DM verification patient")["body"]["rows"]["patientInformation"]
        patient_id = dm["id"]
        proof["ids"] = {"patient":patient_id}
        open_section(driver, "DM")
        initials = visible_input(driver, "D.1")
        if initials is None or initials.get_attribute("value") != dm_value:
            raise AssertionError("DM baseline UI mismatch")
        direct_dm = dm_value + "X"
        initials.clear(); initials.send_keys(direct_dm)
        save(driver, "DM direct initials save")
        dm_saved = page(driver, "DM")["rows"]["patientInformation"]
        if dm_saved["patient_initials"] != direct_dm:
            raise AssertionError(dm_saved)
        driver.refresh(); loaded(driver)
        if visible_input(driver, "D.1").get_attribute("value") != direct_dm:
            raise AssertionError("DM direct value did not survive reload")
        record("DM", "direct-save-reload", value=direct_dm)

    dm_nf = Select(driver.find_element(By.CSS_SELECTOR, 'select[aria-label="Null flavor for Patient Name or Initials"]'))
    dm_nf.select_by_value("MSK")
    save(driver, "DM D.1 NullFlavor MSK")
    dm_nf_saved = page(driver, "DM")["rows"]["patientInformation"]
    if dm_nf_saved["patient_initials"] is not None or dm_nf_saved["patient_initials_null_flavor"] != "MSK":
        raise AssertionError(dm_nf_saved)
    driver.refresh(); loaded(driver)
    if Select(driver.find_element(By.CSS_SELECTOR, 'select[aria-label="Null flavor for Patient Name or Initials"]')).first_selected_option.get_attribute("value") != "MSK":
        raise AssertionError("DM NullFlavor did not survive reload")
    record("DM", "nullflavor-save-reload", nullFlavor="MSK")

    initials = visible_input(driver, "D.1")
    initials.send_keys(direct_dm)
    save(driver, "DM restore value and clear NullFlavor")
    dm_restored = page(driver, "DM")["rows"]["patientInformation"]
    if dm_restored["patient_initials"] != direct_dm or dm_restored["patient_initials_null_flavor"] is not None:
        raise AssertionError(dm_restored)
    record("DM", "nullflavor-to-value", value=direct_dm)

    base = 970000 + (int(suffix) % 10000) * 10
    ids = proof["ids"]
    ids["dhSurvivor"] = create_repeatable(driver,"DH","pastDrugHistory",{"drugName":"Advanced Aspirin"},base)
    ids["dhDeleted"] = create_repeatable(driver,"DH","pastDrugHistory",{"drugName":"DG DELETE HISTORY"},base+1)
    ids["aeSurvivor"] = create_repeatable(driver,"AE","reaction",{"primarySourceReaction":"AE SURVIVOR"},base+2)
    ids["aeDeleted"] = create_repeatable(driver,"AE","reaction",{"primarySourceReaction":"AE DELETE"},base+3)
    ids["lbSurvivor"] = create_repeatable(driver,"LB","testResult",{"testName":"LB SURVIVOR"},base+4)
    ids["lbDeleted"] = create_repeatable(driver,"LB","testResult",{"testName":"LB DELETE"},base+5)
    record("ALL", "created-owned-repeatable-targets", ids=ids)

    open_section(driver,"DH",ids["dhSurvivor"])
    dh_direct = f"V{suffix[-6:]}"
    dh_input = visible_input(driver,"D.8.r.2a")
    dh_input.send_keys(dh_direct)
    save(driver,"DH direct MPID version save")
    dh_saved = detail(driver,"DH",ids["dhSurvivor"],"pastDrugHistory")
    if dh_saved["mpid_version"] != dh_direct:
        raise AssertionError(dh_saved)
    driver.refresh(); loaded(driver)
    if visible_input(driver,"D.8.r.2a").get_attribute("value") != dh_direct:
        raise AssertionError("DH direct value did not survive reload")
    record("DH","direct-save-reload",value=dh_direct)

    dh_nf = Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Start Date"]'))
    dh_nf.select_by_value("ASKU")
    save(driver,"DH start date NullFlavor ASKU")
    dh_nf_saved = detail(driver,"DH",ids["dhSurvivor"],"pastDrugHistory")
    if dh_nf_saved["start_date"] is not None or dh_nf_saved["start_date_null_flavor"] != "ASKU":
        raise AssertionError(dh_nf_saved)
    driver.refresh(); loaded(driver)
    if Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Start Date"]')).first_selected_option.get_attribute("value") != "ASKU":
        raise AssertionError("DH NullFlavor did not survive reload")
    record("DH","nullflavor-save-reload",nullFlavor="ASKU")

    dh_date = visible_input(driver,"D.8.r.4")
    dh_date.send_keys("20200101")
    save(driver,"DH replace NullFlavor with date")
    dh_date_saved = detail(driver,"DH",ids["dhSurvivor"],"pastDrugHistory")
    if dh_date_saved["start_date"] != "20200101" or dh_date_saved["start_date_null_flavor"] is not None:
        raise AssertionError(dh_date_saved)
    record("DH","nullflavor-to-value",value="20200101")

    open_section(driver,"DH",ids["dhDeleted"])
    remove_dh = WebDriverWait(driver,45).until(EC.element_to_be_clickable((By.CSS_SELECTOR,'button[aria-label^="Remove past drug history row "]')))
    remove_dh.click(); save(driver,"DH delete repeatable row")
    if not detail(driver,"DH",ids["dhDeleted"],"pastDrugHistory")["deleted"] or ids["dhDeleted"] in [x["id"] for x in page(driver,"DH")["rows"]["rows"]]:
        raise AssertionError("DH deleted row remains live")
    if detail(driver,"DH",ids["dhSurvivor"],"pastDrugHistory")["mpid_version"] != dh_direct:
        raise AssertionError("DH sibling changed")
    record("DH","ui-delete-sibling-preserved")

    open_section(driver,"AE",ids["aeSurvivor"])
    ae_direct = f"AE DIRECT {suffix[-6:]}"
    ae_input = visible_input(driver,"E.i.1.1a")
    ae_input.clear(); ae_input.send_keys(ae_direct)
    save(driver,"AE direct reaction save")
    ae_saved = detail(driver,"AE",ids["aeSurvivor"],"reaction")
    if ae_saved["primary_source_reaction"] != ae_direct:
        raise AssertionError(ae_saved)
    driver.refresh(); loaded(driver)
    if visible_input(driver,"E.i.1.1a").get_attribute("value") != ae_direct:
        raise AssertionError("AE direct value did not survive reload")
    record("AE","direct-save-reload",value=ae_direct)

    ae_nf = Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Date of Start of Reaction / Event"]'))
    ae_nf.select_by_value("ASKU")
    save(driver,"AE start date NullFlavor ASKU")
    ae_nf_saved = detail(driver,"AE",ids["aeSurvivor"],"reaction")
    if ae_nf_saved["start_date"] is not None or ae_nf_saved["start_date_null_flavor"] != "ASKU":
        raise AssertionError(ae_nf_saved)
    driver.refresh(); loaded(driver)
    if Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Date of Start of Reaction / Event"]')).first_selected_option.get_attribute("value") != "ASKU":
        raise AssertionError("AE NullFlavor did not survive reload")
    record("AE","nullflavor-save-reload",nullFlavor="ASKU")

    ae_date = visible_input(driver,"E.i.4")
    ae_date.send_keys("20200202")
    save(driver,"AE replace NullFlavor with date")
    ae_date_saved = detail(driver,"AE",ids["aeSurvivor"],"reaction")
    if ae_date_saved["start_date"] != "20200202" or ae_date_saved["start_date_null_flavor"] is not None:
        raise AssertionError(ae_date_saved)
    record("AE","nullflavor-to-value",value="20200202")

    open_section(driver,"AE",ids["aeDeleted"]); click(driver,"Remove"); save(driver,"AE delete repeatable row")
    if not detail(driver,"AE",ids["aeDeleted"],"reaction")["deleted"] or ids["aeDeleted"] in [x["id"] for x in page(driver,"AE")["rows"]["rows"]]:
        raise AssertionError("AE deleted row remains live")
    if detail(driver,"AE",ids["aeSurvivor"],"reaction")["primary_source_reaction"] != ae_direct:
        raise AssertionError("AE sibling changed")
    record("AE","ui-delete-sibling-preserved")

    open_section(driver,"LB",ids["lbSurvivor"])
    lb_direct = f"LB DIRECT {suffix[-6:]}"
    lb_input = visible_input(driver,"F.r.2.1")
    lb_input.clear(); lb_input.send_keys(lb_direct)
    save(driver,"LB direct test name save")
    lb_saved = detail(driver,"LB",ids["lbSurvivor"],"testResult")
    if lb_saved["test_name"] != lb_direct:
        raise AssertionError(lb_saved)
    driver.refresh(); loaded(driver)
    if visible_input(driver,"F.r.2.1").get_attribute("value") != lb_direct:
        raise AssertionError("LB direct value did not survive reload")
    record("LB","direct-save-reload",value=lb_direct)

    lb_nf = Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Test Date"]'))
    lb_nf.select_by_value("UNK")
    save(driver,"LB test date NullFlavor UNK")
    lb_nf_saved = detail(driver,"LB",ids["lbSurvivor"],"testResult")
    if lb_nf_saved["test_date"] is not None or lb_nf_saved["test_date_null_flavor"] != "UNK":
        raise AssertionError(lb_nf_saved)
    driver.refresh(); loaded(driver)
    if Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Test Date"]')).first_selected_option.get_attribute("value") != "UNK":
        raise AssertionError("LB NullFlavor did not survive reload")
    record("LB","nullflavor-save-reload",nullFlavor="UNK")

    lb_date = visible_input(driver,"F.r.1")
    lb_date.send_keys("20200303")
    save(driver,"LB replace NullFlavor with date")
    lb_date_saved = detail(driver,"LB",ids["lbSurvivor"],"testResult")
    if lb_date_saved["test_date"] != "20200303" or lb_date_saved["test_date_null_flavor"] is not None:
        raise AssertionError(lb_date_saved)
    record("LB","nullflavor-to-value",value="20200303")

    open_section(driver,"LB",ids["lbDeleted"]); click(driver,"Remove"); save(driver,"LB delete repeatable row")
    if not detail(driver,"LB",ids["lbDeleted"],"testResult")["deleted"] or ids["lbDeleted"] in [x["id"] for x in page(driver,"LB")["rows"]["rows"]]:
        raise AssertionError("LB deleted row remains live")
    if detail(driver,"LB",ids["lbSurvivor"],"testResult")["test_name"] != lb_direct:
        raise AssertionError("LB sibling changed")
    record("LB","ui-delete-sibling-preserved")

    click(driver,"Log out")
    WebDriverWait(driver,45).until(EC.presence_of_element_located((By.CSS_SELECTOR,"input[type=email]")))
    login(driver)
    expected = {
        "DM":("D.1",direct_dm), "DH":("D.8.r.2a",dh_direct),
        "AE":("E.i.1.1a",ae_direct), "LB":("F.r.2.1",lb_direct),
    }
    row_ids = {"DH":ids["dhSurvivor"],"AE":ids["aeSurvivor"],"LB":ids["lbSurvivor"]}
    for section,(field_number,value) in expected.items():
        open_section(driver,section,row_ids.get(section))
        if visible_input(driver,field_number).get_attribute("value") != value:
            raise AssertionError(f"{section} value did not survive relogin")
    driver.save_screenshot(str(ROOT / "remaining-sections-v2-final.png"))
    record("ALL","relogin-readback")

    db = database_oracle(ids)
    if db["dm"] != {"id":patient_id,"initials":direct_dm,"nullFlavor":None}:
        raise AssertionError(db)
    for section,key,survivor,deleted in (("dh","mpidVersion",ids["dhSurvivor"],ids["dhDeleted"]),("ae","reaction",ids["aeSurvivor"],ids["aeDeleted"]),("lb","testName",ids["lbSurvivor"],ids["lbDeleted"])):
        by_id = {item["id"]:item for item in db[section]}
        if by_id[survivor]["deleted"] or not by_id[deleted]["deleted"]:
            raise AssertionError(db)
    expected_records = {patient_id,ids["dhSurvivor"],ids["dhDeleted"],ids["aeSurvivor"],ids["aeDeleted"],ids["lbSurvivor"],ids["lbDeleted"]}
    if expected_records - {item["recordId"] for item in db["audit"]}:
        raise AssertionError("missing DM/DH/AE/LB audit records")
    record("ALL","database-and-audit-oracle",database=db)
    proof["completed"] = True
    proof["terminal"] = "PASS"
except Exception as error:
    proof["error"] = f"{type(error).__name__}: {error}"
    proof["bodyOnError"] = driver.find_element(By.TAG_NAME,"body").text[-6000:]
    driver.save_screenshot(str(ROOT / "remaining-sections-v2-error.png"))
    raise
finally:
    persist()
    driver.quit()
