#!/usr/bin/env python3
"""Drive all six Presave imports through the real headed Case UI."""

import json
import os
import re
from pathlib import Path

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.firefox.service import Service
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.ui import WebDriverWait


ROOT = Path(__file__).parent
UI_URL = os.environ["UI_URL"]
seed = json.loads((ROOT / "seed-result.json").read_text())
source = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()
events = []
completed = False
failure = None


def write_trace():
    (ROOT / "browser-trace-adaptive.json").write_text(
        json.dumps(
            {"completed": completed, "failure": failure, "events": events},
            indent=2,
        )
    )


def record(event):
    events.append(event)
    write_trace()


def constant(name):
    match = re.search(rf'const {name}: &str = "([^"]+)";', source)
    if not match:
        raise RuntimeError(f"missing bootstrap value {name}")
    return match.group(1)


def wait_page(driver, section, row_id=None):
    suffix = f"/{row_id}" if row_id else ""
    driver.get(f"{UI_URL}/ICH/case/{seed['caseId']}/detail/{section}{suffix}")
    WebDriverWait(driver, 60).until(
        lambda d: "Loading" not in d.find_element(By.TAG_NAME, "body").text
    )
    driver.execute_script(
        """
        sessionStorage.setItem('presaveImportRequests', '[]');
        if (window.__presaveImportFetchInstalled) return;
        window.__presaveImportFetchInstalled = true;
        const original = window.fetch;
        window.fetch = async (...args) => {
          const request = args[0] instanceof Request ? args[0] : null;
          const options = args[1] || {};
          const method = String(options.method || request?.method || 'GET').toUpperCase();
          const url = new URL(request?.url || String(args[0]), location.href);
          const body = options.body ?? request?.body ?? null;
          const response = await original(...args);
          if (url.origin === location.origin && ['POST','PUT','PATCH','DELETE'].includes(method)) {
            const rows = JSON.parse(sessionStorage.getItem('presaveImportRequests') || '[]');
            rows.push({method, path:url.pathname + url.search, body:String(body), status:response.status,
              responseBody: await response.clone().text()});
            sessionStorage.setItem('presaveImportRequests', JSON.stringify(rows));
          }
          return response;
        };
        """
    )


def click_button(driver, label):
    button = WebDriverWait(driver, 45).until(
        EC.element_to_be_clickable((By.XPATH, f"//button[normalize-space()={json.dumps(label)}]"))
    )
    driver.execute_script("arguments[0].scrollIntoView({block:'center'})", button)
    button.click()


def import_record(driver, button_label, dialog_title, query, expected):
    click_button(driver, button_label)
    dialog = WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located(
            (By.XPATH, f"//*[normalize-space()={json.dumps(dialog_title)}]/ancestor::*[@role='dialog'][1]")
        )
    )
    search = dialog.find_element(By.CSS_SELECTOR, "input")
    search.send_keys(query)
    row = WebDriverWait(dialog, 45).until(
        lambda node: next(
            (button for button in node.find_elements(By.CSS_SELECTOR, "button.w-full") if expected in button.text),
            False,
        )
    )
    row.click()
    WebDriverWait(driver, 45).until(EC.invisibility_of_element(dialog))
    record({"action": "import", "title": dialog_title, "query": query})


def value(driver, name):
    return WebDriverWait(driver, 45).until(
        EC.presence_of_element_located((By.CSS_SELECTOR, f'[name="{name}"]'))
    ).get_attribute("value")


def assert_value(driver, name, expected):
    actual = value(driver, name)
    if actual != expected:
        raise AssertionError(f"{name}: expected {expected!r}, got {actual!r}")


def mutate_sender_after_selection(driver):
    changed = f"SOURCE-CHANGED-BEFORE-SAVE-{seed['suffix']}"
    result = driver.execute_async_script(
        """
        const [path, payload] = arguments;
        const done = arguments[arguments.length - 1];
        fetch(path, {
          method: 'PUT',
          credentials: 'same-origin',
          headers: {
            'Content-Type': 'application/json',
            'x-e2br3-reason-for-change': 'Adaptive source mutation before Case save',
            'x-e2br3-change-category': 'Edited Data',
          },
          body: JSON.stringify(payload),
        }).then(async response => done({status: response.status, body: await response.text()}))
          .catch(error => done({error: String(error)}));
        """,
        f"/api/presaves/senders/{seed['sender']['id']}/details",
        {"data": {"rows": {"sender": {"organizationName": changed}}}},
    )
    if result.get("status") != 200:
        raise AssertionError(f"source mutation failed: {result}")
    assert_value(driver, "senderInformation.organizationName", seed["sender"]["label"])
    record({"action": "source-mutated-after-selection", "kind": "sender"})


def login(driver):
    driver.get(UI_URL + "/")
    WebDriverWait(driver, 45).until(
        EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]"))
    ).send_keys(constant("DEMO_CRO_ADMIN_EMAIL"))
    driver.find_element(By.CSS_SELECTOR, "input[type=password]").send_keys(
        constant("INITIAL_PASSWORD")
    )
    driver.find_element(By.CSS_SELECTOR, "button[type=submit]").click()
    WebDriverWait(driver, 45).until(
        lambda d: not d.find_elements(By.CSS_SELECTOR, "input[type=email]")
    )


def save(driver, section):
    button = WebDriverWait(driver, 45).until(
        EC.element_to_be_clickable((By.CSS_SELECTOR, 'button[title="Save"]'))
    )
    driver.execute_script("arguments[0].scrollIntoView({block:'center'})", button)
    button.click()
    dialog = WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located(
            (By.XPATH, "//*[normalize-space()='Confirm']/ancestor::*[@role='dialog'][1]")
        )
    )
    dialog.find_element(
        By.CSS_SELECTOR, 'input[name="save-audit-reason"][value="Edited Data"]'
    ).click()
    dialog = WebDriverWait(driver, 10).until(
        EC.visibility_of_element_located(
            (By.XPATH, "//*[normalize-space()='Confirm']/ancestor::*[@role='dialog'][1]")
        )
    )
    dialog.find_element(By.CSS_SELECTOR, 'textarea[aria-label="Reason comments"]').send_keys(
        f"Presave import state verification: {section}"
    )
    dialog.find_element(By.XPATH, ".//button[normalize-space()='Confirm']").click()
    WebDriverWait(driver, 60).until(EC.invisibility_of_element(dialog))

    def complete(current):
        text = current.find_element(By.TAG_NAME, "body").text.lower()
        if "save failed" in text or "unable to save" in text:
            raise AssertionError(text[-2000:])
        return "saved" in text and "saving…" not in text

    WebDriverWait(driver, 60).until(complete)
    requests = driver.execute_script(
        "return JSON.parse(sessionStorage.getItem('presaveImportRequests') || '[]')"
    )
    successful = [
        row for row in requests
        if row["status"] in (200, 201) and row["method"] in ("POST", "PATCH", "PUT")
    ]
    if not successful:
        raise AssertionError(f"no successful save request for {section}: {requests}")
    record({"action": "save", "section": section, "requests": requests})
    return requests


def relogin(driver):
    click_button(driver, "Log out")
    WebDriverWait(driver, 45).until(
        EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]"))
    )
    record({"action": "logout"})
    login(driver)
    record({"action": "relogin"})


options = webdriver.FirefoxOptions()
options.binary_location = os.environ["FIREFOX_BINARY"]
driver = webdriver.Firefox(
    service=Service(os.environ["GECKODRIVER"]), options=options
)
try:
    login(driver)

    wait_page(driver, "SD")
    import_record(driver, "Import Sender", "Import Sender Record", seed["sender"]["label"], seed["sender"]["label"])
    assert_value(driver, "senderInformation.organizationName", seed["sender"]["label"])
    if os.environ.get("ADAPTIVE_SOURCE_MUTATION") == "1":
        mutate_sender_after_selection(driver)
    import_record(driver, "Import Receiver", "Import Receiver Record", seed["receiver"]["label"], seed["receiver"]["label"])
    assert_value(driver, "senderInformation.organizationName", seed["sender"]["label"])
    save(driver, "SD")

    wait_page(driver, "RP")
    import_record(driver, "Import Reporter", "Import Reporter Record", seed["reporter"]["label"], seed["reporter"]["label"])
    assert_value(driver, "primarySources.0.reporterOrganization", seed["reporter"]["label"])
    save(driver, "RP")

    wait_page(driver, "DG")
    import_record(driver, "Import", "Import Product Record", seed["product"]["label"], seed["product"]["label"])
    assert_value(driver, "drugs.0.medicinalProduct", seed["product"]["label"])
    dg_requests = save(driver, "DG")
    dg_response = next(
        json.loads(row["responseBody"])
        for row in dg_requests
        if row["path"].endswith("/DG/rows")
    )
    dg_row_id = dg_response["results"][0]["rowId"]

    wait_page(driver, "SI")
    import_record(driver, "Import", "Import Study Record", seed["study"]["number"], seed["study"]["label"])
    assert_value(driver, "studyInformation.studyName", seed["study"]["label"])
    assert_value(driver, "studyInformation.sponsorStudyNumber", seed["study"]["number"])
    save(driver, "SI")

    wait_page(driver, "NR")
    import_record(driver, "Import Record", "Import Narrative Record", seed["narrative"]["text"], seed["narrative"]["text"])
    assert_value(driver, "narrative.caseNarrative", seed["narrative"]["text"])
    save(driver, "NR")

    for section, fields in {
        "SD": {"senderInformation.organizationName": seed["sender"]["label"]},
        "RP": {"primarySources.0.reporterOrganization": seed["reporter"]["label"]},
        "DG": {"drugs.0.medicinalProduct": seed["product"]["label"]},
        "SI": {
            "studyInformation.studyName": seed["study"]["label"],
            "studyInformation.sponsorStudyNumber": seed["study"]["number"],
        },
        "NR": {"narrative.caseNarrative": seed["narrative"]["text"]},
    }.items():
        wait_page(driver, section, dg_row_id if section == "DG" else None)
        for field, expected in fields.items():
            assert_value(driver, field, expected)
        record({"action": "reload-readback", "section": section})

    relogin(driver)
    for section, fields in {
        "SD": {"senderInformation.organizationName": seed["sender"]["label"]},
        "RP": {"primarySources.0.reporterOrganization": seed["reporter"]["label"]},
        "DG": {"drugs.0.medicinalProduct": seed["product"]["label"]},
        "SI": {"studyInformation.studyName": seed["study"]["label"]},
        "NR": {"narrative.caseNarrative": seed["narrative"]["text"]},
    }.items():
        wait_page(driver, section, dg_row_id if section == "DG" else None)
        for field, expected in fields.items():
            assert_value(driver, field, expected)
        record({"action": "relogin-readback", "section": section})
    completed = True
    record({"action": "terminal", "result": "PASS"})
except Exception as error:
    failure = f"{type(error).__name__}: {error}"
    write_trace()
    raise
finally:
    write_trace()
    if driver:
        try:
            driver.save_screenshot(str(ROOT / "browser-final.png"))
        finally:
            driver.quit()

print(json.dumps({"completed": completed, "failure": failure, "events": events}, indent=2))
