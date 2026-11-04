#!/usr/bin/env python3
"""Verify core Case edit persistence through the real headed UI and task DB."""

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
RESULT = ROOT / "case-edit-core-roundtrip.json"
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


def wait_loaded(driver):
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
          .then(async response => done({status: response.status, text: await response.text()}))
          .catch(error => done({error: String(error)}));
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
    driver.find_element(By.CSS_SELECTOR, "input[type=password]").send_keys(
        constant("INITIAL_PASSWORD")
    )
    driver.find_element(By.CSS_SELECTOR, "button[type=submit]").click()
    WebDriverWait(driver, 60).until(
        lambda d: not d.find_elements(By.CSS_SELECTOR, "input[type=email]")
    )


def click_button(driver, label):
    button = WebDriverWait(driver, 45).until(
        lambda d: next(
            (
                node
                for node in d.find_elements(By.TAG_NAME, "button")
                if node.is_displayed() and node.is_enabled() and node.text.strip() == label
            ),
            False,
        )
    )
    driver.execute_script("arguments[0].scrollIntoView({block:'center'})", button)
    button.click()


def save(driver, reason):
    click_button(driver, "Save")
    dialog = WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located(
            (By.XPATH, "//*[normalize-space()='Confirm']/ancestor::*[@role='dialog'][1]")
        )
    )
    category = dialog.find_elements(By.CSS_SELECTOR, 'input[value="Edited Data"]')
    if not category:
        category = dialog.find_elements(By.CSS_SELECTOR, 'input[value="New Data"]')
    if len(category) != 1:
        raise AssertionError("save dialog has no unique audit category")
    category[0].click()
    dialog.find_element(By.CSS_SELECTOR, 'textarea[aria-label="Reason comments"]').send_keys(reason)
    dialog.find_element(By.XPATH, ".//button[normalize-space()='Confirm']").click()
    WebDriverWait(driver, 60).until(EC.invisibility_of_element(dialog))
    WebDriverWait(driver, 60).until(
        lambda d: "saving…" not in d.find_element(By.TAG_NAME, "body").text.lower()
        and "saved" in d.find_element(By.TAG_NAME, "body").text.lower()
    )


def open_row(driver, row_id):
    driver.get(f"{UI_URL}/ICH/case/{CASE_ID}/detail/LR/{row_id}")
    wait_loaded(driver)
    return WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located(
            (By.CSS_SELECTOR, '[data-e2b-field-number="C.4.r.1"] textarea')
        )
    )


def row(driver, row_id):
    result = api(
        driver,
        "GET",
        f"/api/cases/{CASE_ID}/editor/pages/LR/rows/{row_id}?authorities=ich",
    )
    if result["status"] != 200:
        raise AssertionError(result)
    return result["body"]["data"]["literatureReference"]


def database_oracle(row_ids):
    quoted = ",".join(f"'{value}'::uuid" for value in row_ids)
    sql = f"""
    SELECT json_build_object(
      'rows', COALESCE((SELECT json_agg(json_build_object(
        'id', id, 'referenceText', reference_text,
        'referenceTextNullFlavor', reference_text_null_flavor, 'deleted', deleted
      ) ORDER BY id) FROM literature_references WHERE id IN ({quoted})), '[]'::json),
      'audit', COALESCE((SELECT json_agg(json_build_object(
        'recordId', record_id, 'action', action, 'changedFields', changed_fields,
        'reason', reason_for_change
      ) ORDER BY id) FROM audit_logs WHERE record_id IN ({quoted})), '[]'::json)
    );
    """
    completed = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "psql", "-U", "postgres", "-d", DB_NAME,
         "-At", "-v", "ON_ERROR_STOP=1", "-c", sql],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout.strip())


options = webdriver.FirefoxOptions()
options.binary_location = os.environ["FIREFOX_BINARY"]
driver = webdriver.Firefox(
    service=Service(os.environ["GECKODRIVER"]), options=options
)
driver.set_window_size(1440, 1000)
driver.set_script_timeout(45)

try:
    if driver.capabilities.get("moz:headless") is not False:
        raise AssertionError("Firefox is not headed")
    login(driver)
    suffix = str(int(time.time()))
    created = []
    for offset, label in enumerate((f"CORE-SURVIVOR-{suffix}", f"CORE-DELETE-{suffix}")):
        response = api(
            driver,
            "POST",
            f"/api/cases/{CASE_ID}/editor/pages/LR/rows",
            {"authorities": ["ich"], "rows": {"literatureReference": {
                "sequenceNumber": 900000 + (int(suffix) % 10000) * 2 + offset,
                "referenceText": label, "referenceTextNullFlavor": None,
            }}},
            "Adaptive core Case edit verification row",
        )
        if response["status"] != 201:
            raise AssertionError(response)
        created.append(response["body"]["rowId"])
    survivor_id, delete_id = created
    proof["rowIds"] = {"survivor": survivor_id, "deleted": delete_id}
    record("create-two-real-rows", statuses=[201, 201])

    text_value = f"DIRECT-CASE-EDIT-{suffix}"
    textarea = open_row(driver, survivor_id)
    textarea.clear()
    textarea.send_keys(text_value)
    save(driver, "Direct Case edit scalar save")
    saved = row(driver, survivor_id)
    if saved["reference_text"] != text_value or saved["reference_text_null_flavor"] is not None:
        raise AssertionError(saved)
    driver.refresh()
    wait_loaded(driver)
    textarea = WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located((By.CSS_SELECTOR, '[data-e2b-field-number="C.4.r.1"] textarea'))
    )
    if textarea.get_attribute("value") != text_value:
        raise AssertionError("scalar value did not survive reload")
    record("direct-edit-save-reload", value=text_value, api=saved)

    nf = Select(driver.find_element(By.CSS_SELECTOR, 'select[aria-label="Null flavor for Literature Reference"]'))
    nf.select_by_value("ASKU")
    WebDriverWait(driver, 15).until(lambda _d: textarea.get_attribute("value") == "")
    save(driver, "Set Literature Reference NullFlavor ASKU")
    nf_saved = row(driver, survivor_id)
    if nf_saved["reference_text"] is not None or nf_saved["reference_text_null_flavor"] != "ASKU":
        raise AssertionError(nf_saved)
    driver.refresh()
    wait_loaded(driver)
    textarea = driver.find_element(By.CSS_SELECTOR, '[data-e2b-field-number="C.4.r.1"] textarea')
    nf = Select(driver.find_element(By.CSS_SELECTOR, 'select[aria-label="Null flavor for Literature Reference"]'))
    if textarea.get_attribute("value") != "" or textarea.is_enabled() or nf.first_selected_option.get_attribute("value") != "ASKU":
        raise AssertionError("NullFlavor did not survive reload in the UI")
    record("nullflavor-set-save-reload", api=nf_saved, ui={"value": "", "enabled": False, "nf": "ASKU"})

    nf.select_by_value("")
    WebDriverWait(driver, 15).until(lambda _d: textarea.is_enabled())
    textarea.send_keys(text_value)
    save(driver, "Clear NullFlavor and restore direct value")
    restored = row(driver, survivor_id)
    if restored["reference_text"] != text_value or restored["reference_text_null_flavor"] is not None:
        raise AssertionError(restored)
    record("nullflavor-clear-and-value-restore", api=restored)

    delete_text = row(driver, delete_id)["reference_text"]
    open_row(driver, delete_id)
    remove = WebDriverWait(driver, 45).until(
        lambda d: next((node for node in d.find_elements(By.CSS_SELECTOR, 'button[aria-label^="Remove reference "]') if node.is_displayed()), False)
    )
    remove.click()
    save(driver, "Delete repeatable Literature Reference row")
    detail_after_delete = api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/LR/rows/{delete_id}?authorities=ich")
    page_after_delete = api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/LR?authorities=ich")
    live_ids = [item["id"] for item in page_after_delete["body"]["rows"]["rows"]]
    survivor_after_delete = row(driver, survivor_id)
    if detail_after_delete["status"] not in (200, 404) or delete_id in live_ids:
        raise AssertionError({"detail": detail_after_delete, "liveIds": live_ids})
    if survivor_after_delete["reference_text"] != text_value:
        raise AssertionError("deleting sibling changed survivor")
    record("repeatable-row-ui-delete", deletedValue=delete_text, detailStatus=detail_after_delete["status"], siblingPreserved=True)

    click_button(driver, "Log out")
    WebDriverWait(driver, 45).until(EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]")))
    login(driver)
    textarea = open_row(driver, survivor_id)
    if textarea.get_attribute("value") != text_value:
        raise AssertionError("survivor value did not survive relogin")
    final_page = api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/LR?authorities=ich")
    final_ids = [item["id"] for item in final_page["body"]["rows"]["rows"]]
    if survivor_id not in final_ids or delete_id in final_ids:
        raise AssertionError(final_ids)
    driver.save_screenshot(str(ROOT / "case-edit-core-final.png"))
    record("relogin-readback", survivorValue=text_value, deletedAbsent=True)

    db = database_oracle(created)
    by_id = {item["id"]: item for item in db["rows"]}
    if by_id[survivor_id]["referenceText"] != text_value or by_id[survivor_id]["referenceTextNullFlavor"] is not None or by_id[survivor_id]["deleted"]:
        raise AssertionError(db)
    if not by_id[delete_id]["deleted"]:
        raise AssertionError(db)
    audited_ids = {item["recordId"] for item in db["audit"]}
    if not set(created).issubset(audited_ids):
        raise AssertionError("missing audit rows for created Case rows")
    record("database-and-audit-oracle", database=db)
    proof["completed"] = True
    proof["terminal"] = "PASS"
except Exception as error:
    proof["error"] = f"{type(error).__name__}: {error}"
    proof["bodyOnError"] = driver.find_element(By.TAG_NAME, "body").text[-4000:]
    driver.save_screenshot(str(ROOT / "case-edit-core-error.png"))
    raise
finally:
    persist()
    driver.quit()
