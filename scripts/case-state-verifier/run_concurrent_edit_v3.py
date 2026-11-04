#!/usr/bin/env python3
"""Exercise two-user stale saves through the real headed Case editor."""

import http.cookiejar
import json
import os
import re
import secrets
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.firefox.service import Service
from selenium.common.exceptions import TimeoutException
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.ui import WebDriverWait


ROOT = Path(__file__).parent
UI_URL = os.environ["UI_URL"]
API_URL = os.environ["API_URL"]
DB_NAME = os.environ["STATE_VERIFIER_DB"]
CASE_ID = json.loads((ROOT / "seed-result.json").read_text())["caseId"]
ORG_ID = "00000000-0000-0000-0000-000000000001"
SOURCE = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()
RESULT = ROOT / "concurrent-edit-v3-result.json"
proof = {"completed": False, "caseId": CASE_ID, "grammar": "adaptive-case-edit-grammar-v3.json", "steps": []}


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


def admin_client():
    opener = urllib.request.build_opener(
        urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar())
    )
    status, _ = http_json(
        opener,
        "POST",
        "/auth/v1/login",
        {"email": constant("INITIAL_ADMIN_EMAIL"), "pwd": constant("INITIAL_PASSWORD")},
    )
    if status != 200:
        raise AssertionError(f"admin login returned {status}")
    return opener


def http_json(opener, method, path, payload=None):
    data = None if payload is None else json.dumps(payload).encode()
    request = urllib.request.Request(
        API_URL + path,
        data=data,
        method=method,
        headers={"Content-Type": "application/json"} if data is not None else {},
    )
    try:
        with opener.open(request, timeout=30) as response:
            raw = response.read()
            return response.status, decode_body(raw)
    except urllib.error.HTTPError as error:
        raw = error.read()
        return error.code, decode_body(raw)


def decode_body(raw):
    if not raw:
        return None
    text = raw.decode(errors="replace")
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return text


def response_data(body):
    if not isinstance(body, dict):
        raise AssertionError(body)
    value = body.get("data")
    return value if isinstance(value, dict) else body


def create_actor(admin):
    suffix = f"{int(time.time())}-{secrets.token_hex(3)}"
    profile_path = f"/api/admin/permission-profiles?organization_id={ORG_ID}"
    status, body = http_json(admin, "POST", profile_path, {"data": {
        "name": f"Concurrent editor {suffix}",
        "description": "Owned adaptive grammar v3 verification role",
        "privileges": [{
            "menu_key": "case", "can_read": True, "can_edit": True,
            "can_review": False, "can_lock": False,
        }],
        "active": True,
    }})
    if status != 201:
        raise AssertionError({"profileStatus": status, "body": body})
    profile = response_data(body)
    profile_id = str(profile["id"])
    password = f"InitialV3!{secrets.token_hex(8)}Aa1"
    replacement_password = f"ConcurrentV3!{secrets.token_hex(8)}Aa1"
    email = f"concurrent-{suffix}@example.invalid"
    status, body = http_json(admin, "POST", "/api/users", {"data": {
        "email": email,
        "username": f"concurrent_{suffix.replace('-', '_')}",
        "pwd_clear": password,
        "role": profile_id,
        "organization_id": ORG_ID,
    }})
    if status != 201:
        http_json(admin, "DELETE", f"{profile_path.replace('?', '/' + profile_id + '?')}")
        raise AssertionError({"userStatus": status, "body": body})
    user_id = str(response_data(body)["id"])
    return {
        "id": user_id, "email": email, "password": password,
        "replacementPassword": replacement_password,
        "profileId": profile_id, "profilePrivileges": profile.get("privileges"),
    }


def cleanup_actor(admin, actor):
    user_status, _ = http_json(admin, "PUT", f"/api/users/{actor['id']}", {"data": {"active": False}})
    profile_path = f"/api/admin/permission-profiles/{actor['profileId']}?organization_id={ORG_ID}"
    profile_status, _ = http_json(admin, "DELETE", profile_path)
    if profile_status != 204:
        profile_status, _ = http_json(admin, "PUT", profile_path, {"data": {"active": False}})
    return {"userDeactivate": user_status, "profileCleanup": profile_status}


def new_driver():
    options = webdriver.FirefoxOptions()
    options.binary_location = os.environ["FIREFOX_BINARY"]
    driver = webdriver.Firefox(
        service=Service(os.environ["GECKODRIVER"], service_args=["--host", "127.0.0.1"]),
        options=options,
    )
    driver.set_window_size(1400, 950)
    driver.set_script_timeout(45)
    if driver.capabilities.get("moz:headless") is not False:
        driver.quit()
        raise AssertionError("Firefox is not headed")
    return driver


def login(driver, email, password, replacement_password=None):
    driver.get(UI_URL + "/")
    WebDriverWait(driver, 60).until(
        EC.presence_of_element_located((By.CSS_SELECTOR, "input[type=email]"))
    ).send_keys(email)
    driver.find_element(By.CSS_SELECTOR, "input[type=password]").send_keys(password)
    driver.find_element(By.CSS_SELECTOR, "button[type=submit]").click()
    WebDriverWait(driver, 60).until(
        lambda d: not d.find_elements(By.CSS_SELECTOR, "input[type=email]")
    )
    if driver.current_url.rstrip("/").endswith("/set-password"):
        if replacement_password is None:
            raise AssertionError("unexpected mandatory password change")
        fields = WebDriverWait(driver, 30).until(
            lambda d: d.find_elements(By.CSS_SELECTOR, "input[type=password]")
        )
        if len(fields) != 2:
            raise AssertionError("set-password form does not have two password fields")
        fields[0].send_keys(replacement_password)
        fields[1].send_keys(replacement_password)
        button(driver, "Save Password").click()
        WebDriverWait(driver, 60).until(
            lambda d: not d.current_url.rstrip("/").endswith("/set-password")
        )
        login(driver, email, replacement_password)


def loaded(driver):
    WebDriverWait(driver, 60).until(
        lambda d: d.execute_script("return document.readyState") == "complete"
        and "Loading" not in d.find_element(By.TAG_NAME, "body").text
    )


def open_section(driver, section, row_id=None):
    suffix = f"/{row_id}" if row_id else ""
    driver.get(f"{UI_URL}/ICH/case/{CASE_ID}/detail/{section}{suffix}")
    loaded(driver)


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
    category = dialog.find_elements(By.CSS_SELECTOR, 'input[value="Edited Data"]')
    if len(category) != 1:
        raise AssertionError("save category is not unique")
    category[0].click()
    dialog.find_element(By.CSS_SELECTOR, 'textarea[aria-label="Reason comments"]').send_keys(reason)
    driver.execute_script(
        """
        window.__saveDialogTrace = [];
        const snapshot = (event) => window.__saveDialogTrace.push({
          event,
          at: performance.now(),
          dialogs: [...document.querySelectorAll('[role="dialog"]')].map((node) => ({
            state: node.dataset.state,
            text: node.innerText.slice(0, 160),
          })),
        });
        snapshot('installed');
        new MutationObserver(() => snapshot('mutation')).observe(document.body, {
          subtree: true,
          childList: true,
          attributes: true,
          attributeFilter: ['data-state'],
        });
        document.addEventListener('click', (event) => snapshot(`click:${event.target.innerText || ''}`), true);
        """
    )
    dialog.find_element(By.XPATH, ".//button[normalize-space()='Confirm']").click()
    try:
        WebDriverWait(driver, 10).until(EC.invisibility_of_element(dialog))
    except TimeoutException:
        proof.setdefault("findings", []).append({
            "kind": "save-dialog-stuck-after-committed-write",
            "reason": reason,
            "dialogState": driver.execute_script(
                "return window.__saveDialogTrace?.slice(-40) || [];"
            ),
        })
        persist()
        driver.save_screenshot(str(ROOT / "concurrent-v3-stuck-save-dialog.png"))
        driver.find_element(By.TAG_NAME, "body").send_keys(Keys.ESCAPE)
        WebDriverWait(driver, 10).until(EC.invisibility_of_element(dialog))
    WebDriverWait(driver, 60).until(
        lambda d: "saving…" not in d.find_element(By.TAG_NAME, "body").text.lower()
        and "saved" in d.find_element(By.TAG_NAME, "body").text.lower()
    )


def browser_api(driver, method, path, payload=None, reason=None):
    result = driver.execute_async_script(
        """
        const [method,path,payload,reason]=arguments;
        const done=arguments[arguments.length-1];
        const headers={};
        if(payload!==null) headers['Content-Type']='application/json';
        if(reason!==null){headers['x-e2br3-reason-for-change']=reason;headers['x-e2br3-change-category']='Edited Data';}
        fetch(path,{method,credentials:'same-origin',headers,body:payload===null?undefined:JSON.stringify(payload)})
          .then(async r=>done({status:r.status,text:await r.text()})).catch(e=>done({error:String(e)}));
        """,
        method, path, payload, reason,
    )
    if "error" in result:
        raise AssertionError(result)
    result["body"] = json.loads(result["text"]) if result["text"] else None
    del result["text"]
    return result


def ci_rows(driver):
    result = browser_api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/CI?authorities=ich")
    if result["status"] != 200:
        raise AssertionError(result)
    return result["body"]["rows"]


def lr_row(driver, row_id):
    result = browser_api(driver, "GET", f"/api/cases/{CASE_ID}/editor/pages/LR/rows/{row_id}?authorities=ich")
    if result["status"] != 200:
        raise AssertionError(result)
    return result["body"]["data"]["literatureReference"]


def create_lr(driver, label, sequence):
    result = browser_api(driver, "POST", f"/api/cases/{CASE_ID}/editor/pages/LR/rows", {
        "authorities": ["ich"],
        "rows": {"literatureReference": {
            "sequenceNumber": sequence,
            "referenceText": label,
            "referenceTextNullFlavor": None,
        }},
    }, "Adaptive v3 concurrent verification row")
    if result["status"] != 201:
        raise AssertionError(result)
    return result["body"]["rowId"]


def field_input(driver, number, selector="input"):
    root = WebDriverWait(driver, 45).until(
        EC.visibility_of_element_located((By.CSS_SELECTOR, f'[data-e2b-field-number="{number}"]'))
    )
    return next((item for item in root.find_elements(By.CSS_SELECTOR, selector) if item.is_displayed()), None)


def erase_c17(driver):
    root = driver.find_element(By.CSS_SELECTOR, '[data-e2b-field-number="C.1.7"]')
    actions = root.find_element(By.CSS_SELECTOR, 'button[aria-label="Open field actions for C.1.7"]')
    actions.click()
    erase = WebDriverWait(driver, 20).until(
        lambda d: next((item for item in d.find_elements(By.CSS_SELECTOR, '[role="menuitem"]')
                        if item.is_displayed() and item.text.strip() == "Erase"), False)
    )
    erase.click()
    driver.find_element(By.TAG_NAME, "body").send_keys(Keys.ESCAPE)
    WebDriverWait(driver, 10).until(
        EC.invisibility_of_element_located((By.CSS_SELECTOR, '[role="menuitem"]'))
    )


def database_value(sql):
    result = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "psql", "-U", "postgres", "-d", DB_NAME,
         "-At", "-v", "ON_ERROR_STOP=1", "-c", sql],
        check=True, capture_output=True, text=True,
    )
    return result.stdout.strip()


def database_oracle(ids, reasons, audit_floor):
    quoted_ids = ",".join(f"'{value}'::uuid" for value in ids)
    quoted_reasons = ",".join("'" + value.replace("'", "''") + "'" for value in reasons)
    sql = f"""
    SELECT json_build_object(
      'rows',(SELECT json_agg(json_build_object('id',id,'text',reference_text,'deleted',deleted) ORDER BY id)
        FROM literature_references WHERE id IN ({quoted_ids})),
      'audit',(SELECT json_agg(json_build_object('id',id,'recordId',record_id,'userId',user_id,
        'changedFields',changed_fields,'reason',reason_for_change) ORDER BY id)
        FROM audit_logs WHERE id > {audit_floor} AND reason_for_change IN ({quoted_reasons})),
      'integrity',(SELECT row_to_json(v) FROM verify_audit_log_hash_chain(NULL) v));
    """
    result = subprocess.run(
        ["docker", "exec", os.environ["POSTGRES_CONTAINER"], "psql", "-U", "postgres", "-d", DB_NAME,
         "-At", "-v", "ON_ERROR_STOP=1", "-c", sql],
        check=True, capture_output=True, text=True,
    )
    return json.loads(result.stdout.strip())


admin = admin_client()
actor = None
first = second = None
cleanup = None
try:
    actor = create_actor(admin)
    record(
        "created-real-second-actor", userId=actor["id"], profileId=actor["profileId"],
        profilePrivileges=actor["profilePrivileges"],
    )
    first = new_driver()
    second = new_driver()
    login(first, constant("DEMO_CRO_ADMIN_EMAIL"), constant("INITIAL_PASSWORD"))
    login(second, actor["email"], actor["password"], actor["replacementPassword"])
    audit_floor = int(database_value("SELECT COALESCE(max(id),0) FROM audit_logs;"))

    baseline = ci_rows(first)
    sri = baseline["safetyReportIdentification"]
    if baseline["case"]["reportYear"] is not None or sri["fulfilExpeditedCriteria"] is not None:
        raise AssertionError("CI concurrency baseline is not empty")
    open_section(first, "CI")
    open_section(second, "CI")
    if not second.find_elements(By.CSS_SELECTOR, '[data-e2b-field-number="C.1.7"]'):
        raise AssertionError({
            "secondActorUrl": second.current_url,
            "secondActorBody": second.find_element(By.TAG_NAME, "body").text[:2000],
        })
    report_year = first.find_element(By.CSS_SELECTOR, '[name="case.reportYear"]')
    report_year.send_keys("2027")
    second.find_element(By.CSS_SELECTOR, '[data-e2b-field-number="C.1.7"] input[value="true"]').click()
    save(first, "Concurrent v3 actor A report year")
    save(second, "Concurrent v3 actor B expedited flag")
    concurrent = ci_rows(first)
    if concurrent["case"]["reportYear"] != "2027":
        raise AssertionError("stale save lost actor A report year")
    if concurrent["safetyReportIdentification"]["fulfilExpeditedCriteria"] is not True:
        raise AssertionError("actor B expedited value was not saved")
    first.refresh(); loaded(first)
    second.refresh(); loaded(second)
    if first.find_element(By.CSS_SELECTOR, '[name="case.reportYear"]').get_attribute("value") != "2027":
        raise AssertionError("actor A reload mismatch")
    if not second.find_element(By.CSS_SELECTOR, '[data-e2b-field-number="C.1.7"] input[value="true"]').is_selected():
        raise AssertionError("actor B reload mismatch")
    record("concurrent-disjoint-scalar-save", reportYear="2027", expedited=True, staleSavePreservedPeer=True)

    suffix = int(time.time())
    survivor_id = create_lr(first, f"V3-SIBLING-{suffix}", 970000 + suffix % 10000 * 2)
    target_id = create_lr(first, f"V3-TARGET-{suffix}", 970001 + suffix % 10000 * 2)
    open_section(first, "LR", target_id)
    open_section(second, "LR", target_id)
    text = field_input(first, "C.4.r.1", "textarea")
    if text is None:
        raise AssertionError("actor A LR field missing")
    text.clear(); text.send_keys(f"V3-EDITED-{suffix}")
    save(first, "Concurrent v3 actor A edit row")
    remove = WebDriverWait(second, 45).until(
        lambda d: next((item for item in d.find_elements(By.CSS_SELECTOR, 'button[aria-label^="Remove reference "]')
                        if item.is_displayed()), False)
    )
    remove.click()
    save(second, "Concurrent v3 actor B stale delete")
    target = lr_row(first, target_id)
    sibling = lr_row(first, survivor_id)
    if not target["deleted"]:
        raise AssertionError("stale delete did not reach a terminal deleted state")
    if sibling["reference_text"] != f"V3-SIBLING-{suffix}" or sibling["deleted"]:
        raise AssertionError("concurrent delete changed sibling")
    record("concurrent-edit-then-stale-delete", targetDeleted=True, siblingPreserved=True)

    restore_response = browser_api(
        first,
        "PATCH",
        f"/api/cases/{CASE_ID}/editor/pages/CI",
        {
            "authorities": ["ich"],
            "rows": {
                "case": {"reportYear": None},
                "safetyReportIdentification": {
                    "fulfilExpeditedCriteria": None,
                    "fulfilExpeditedCriteriaNullFlavor": None,
                },
            },
        },
        "Concurrent v3 restore CI baseline",
    )
    if restore_response["status"] != 200:
        raise AssertionError(restore_response)
    restored = ci_rows(first)
    if restored["case"]["reportYear"] is not None or restored["safetyReportIdentification"]["fulfilExpeditedCriteria"] is not None:
        raise AssertionError("CI baseline restoration failed")
    record("restored-ci-baseline")

    reasons = [
        "Edited Data: Concurrent v3 actor A report year",
        "Edited Data: Concurrent v3 actor B expedited flag",
        "Edited Data: Concurrent v3 actor A edit row",
        "Edited Data: Concurrent v3 actor B stale delete",
        "Concurrent v3 restore CI baseline",
    ]
    db = database_oracle([survivor_id, target_id], reasons, audit_floor)
    audits = db["audit"]
    if len(audits) != 6:
        raise AssertionError({"auditCount": len(audits), "expected": 6, "audit": audits})
    if db["integrity"]["broken_rows"] != 0:
        raise AssertionError(db["integrity"])
    actor_ids = {item["userId"] for item in audits[:4]}
    if actor["id"] not in actor_ids or constant("DEMO_CRO_ADMIN_EMAIL") == actor["email"]:
        raise AssertionError("audit did not distinguish both actors")
    proof["database"] = db
    proof["completed"] = True
    persist()
finally:
    if first is not None:
        first.quit()
    if second is not None:
        second.quit()
    if actor is not None:
        cleanup = cleanup_actor(admin, actor)
        proof["actorCleanup"] = cleanup
        persist()

print(json.dumps({
    "completed": proof["completed"],
    "steps": [item["name"] for item in proof["steps"]],
    "actorCleanup": cleanup,
}, ensure_ascii=False))
