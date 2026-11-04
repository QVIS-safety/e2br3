#!/usr/bin/env python3
"""Continue grammar v2 after verified DM/DH phases, covering AE and LB."""

import json
import os
import re
import subprocess
from pathlib import Path

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.firefox.service import Service
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.ui import Select, WebDriverWait


ROOT = Path(__file__).parent
UI_URL = os.environ["UI_URL"]
CASE_ID = json.loads((ROOT / "seed-result.json").read_text())["caseId"]
PREVIOUS = json.loads((ROOT / "remaining-sections-v2-result.json").read_text())
RESULT = ROOT / "remaining-sections-v2-final.json"
SOURCE = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()
proof = {"completed": False, "grammarVersion": 2, "caseId": CASE_ID,
         "continuedFrom": "remaining-sections-v2-result.json", "steps": PREVIOUS["steps"][:]}
ids = PREVIOUS["ids"]


def persist():
    RESULT.write_text(json.dumps(proof, ensure_ascii=False, indent=2))


def record(section, name, **details):
    proof["steps"].append({"section":section,"name":name,**details})
    persist()


def constant(name):
    match = re.search(rf'const {name}: &str = "([^"]+)";', SOURCE)
    if not match:
        raise RuntimeError(f"missing bootstrap value {name}")
    return match.group(1)


def loaded(driver):
    WebDriverWait(driver,60).until(lambda d:d.execute_script("return document.readyState") == "complete" and "Loading" not in d.find_element(By.TAG_NAME,"body").text)


def api(driver,method,path,payload=None,reason=None,expected=(200,)):
    result = driver.execute_async_script(
        """
        const [method,path,payload,reason]=arguments; const done=arguments[arguments.length-1];
        const headers={}; if(payload!==null) headers['Content-Type']='application/json';
        if(reason!==null){headers['x-e2br3-reason-for-change']=reason;headers['x-e2br3-change-category']='Edited Data';}
        fetch(path,{method,credentials:'same-origin',headers,body:payload===null?undefined:JSON.stringify(payload)})
          .then(async r=>done({status:r.status,text:await r.text()})).catch(e=>done({error:String(e)}));
        """,method,path,payload,reason)
    if "error" in result: raise AssertionError(result)
    result["body"]=json.loads(result["text"]) if result["text"] else None
    del result["text"]
    if result["status"] not in expected: raise AssertionError(result)
    return result


def login(driver):
    driver.get(UI_URL+"/")
    WebDriverWait(driver,60).until(EC.presence_of_element_located((By.CSS_SELECTOR,"input[type=email]"))).send_keys(constant("DEMO_CRO_ADMIN_EMAIL"))
    driver.find_element(By.CSS_SELECTOR,"input[type=password]").send_keys(constant("INITIAL_PASSWORD"))
    driver.find_element(By.CSS_SELECTOR,"button[type=submit]").click()
    WebDriverWait(driver,60).until(lambda d:not d.find_elements(By.CSS_SELECTOR,"input[type=email]"))


def click(driver,label):
    node=WebDriverWait(driver,45).until(lambda d:next((x for x in d.find_elements(By.TAG_NAME,"button") if x.is_displayed() and x.is_enabled() and x.text.strip()==label),False))
    driver.execute_script("arguments[0].scrollIntoView({block:'center'})",node);node.click()


def save(driver,reason):
    click(driver,"Save")
    dialog=WebDriverWait(driver,45).until(EC.visibility_of_element_located((By.XPATH,"//*[normalize-space()='Confirm']/ancestor::*[@role='dialog'][1]")))
    category=dialog.find_elements(By.CSS_SELECTOR,'input[value="Edited Data"]') or dialog.find_elements(By.CSS_SELECTOR,'input[value="New Data"]')
    if len(category)!=1: raise AssertionError("save audit category is not unique")
    category[0].click();dialog.find_element(By.CSS_SELECTOR,'textarea[aria-label="Reason comments"]').send_keys(reason)
    dialog.find_element(By.XPATH,".//button[normalize-space()='Confirm']").click()
    WebDriverWait(driver,60).until(EC.invisibility_of_element(dialog))
    WebDriverWait(driver,60).until(lambda d:"saving…" not in d.find_element(By.TAG_NAME,"body").text.lower() and "saved" in d.find_element(By.TAG_NAME,"body").text.lower())


def open_section(driver,section,row_id=None):
    suffix=f"/{row_id}" if row_id else ""; expected=f"/ICH/case/{CASE_ID}/detail/{section}{suffix}"
    driver.get(UI_URL+expected)
    WebDriverWait(driver,60).until(lambda d:expected in d.current_url and d.find_elements(By.CSS_SELECTOR,f'[data-case-section-shell="{section.lower()}"]'))
    loaded(driver)


def page(driver,section):
    return api(driver,"GET",f"/api/cases/{CASE_ID}/editor/pages/{section}?authorities=ich")["body"]


def detail(driver,section,row_id,key):
    return api(driver,"GET",f"/api/cases/{CASE_ID}/editor/pages/{section}/rows/{row_id}?authorities=ich")["body"]["data"][key]


def field_input(driver,field_number):
    root=WebDriverWait(driver,45).until(EC.visibility_of_element_located((By.CSS_SELECTOR,f'[data-e2b-field-number="{field_number}"]')))
    node=next((x for x in root.find_elements(By.CSS_SELECTOR,"input") if x.is_displayed()),None)
    if node is None: raise AssertionError(f"visible input missing for {field_number}")
    return node


def db_oracle():
    q=lambda values:",".join(f"'{value}'::uuid" for value in values)
    records=[ids["patient"],ids["dhSurvivor"],ids["dhDeleted"],ids["aeSurvivor"],ids["aeDeleted"],ids["lbSurvivor"],ids["lbDeleted"]]
    sql=f"""
    SELECT json_build_object(
      'dm',(SELECT json_build_object('id',id,'initials',patient_initials,'nullFlavor',patient_initials_null_flavor) FROM patient_information WHERE id='{ids['patient']}'),
      'dh',COALESCE((SELECT json_agg(json_build_object('id',id,'mpidVersion',mpid_version,'startDate',start_date,'startDateNullFlavor',start_date_null_flavor,'deleted',deleted) ORDER BY id) FROM past_drug_history WHERE id IN ({q([ids['dhSurvivor'],ids['dhDeleted']])})),'[]'::json),
      'ae',COALESCE((SELECT json_agg(json_build_object('id',id,'reaction',primary_source_reaction,'startDate',start_date,'startDateNullFlavor',start_date_null_flavor,'deleted',deleted) ORDER BY id) FROM reactions WHERE id IN ({q([ids['aeSurvivor'],ids['aeDeleted']])})),'[]'::json),
      'lb',COALESCE((SELECT json_agg(json_build_object('id',id,'testName',test_name,'testDate',test_date,'testDateNullFlavor',test_date_null_flavor,'deleted',deleted) ORDER BY id) FROM test_results WHERE id IN ({q([ids['lbSurvivor'],ids['lbDeleted']])})),'[]'::json),
      'audit',COALESCE((SELECT json_agg(json_build_object('recordId',record_id,'action',action,'changedFields',changed_fields,'reason',reason_for_change) ORDER BY id) FROM audit_logs WHERE record_id IN ({q(records)})),'[]'::json)
    );
    """
    result=subprocess.run(["docker","exec",os.environ["POSTGRES_CONTAINER"],"psql","-U","postgres","-d",os.environ["STATE_VERIFIER_DB"],"-At","-v","ON_ERROR_STOP=1","-c",sql],check=True,capture_output=True,text=True)
    return json.loads(result.stdout.strip())


options=webdriver.FirefoxOptions();options.binary_location=os.environ["FIREFOX_BINARY"]
driver=webdriver.Firefox(service=Service(os.environ["GECKODRIVER"]),options=options)
driver.set_window_size(1440,1000);driver.set_script_timeout(45)

try:
    if driver.capabilities.get("moz:headless") is not False: raise AssertionError("Firefox is not headed")
    login(driver)
    dm=page(driver,"DM")["rows"]["patientInformation"]
    dh_survivor=detail(driver,"DH",ids["dhSurvivor"],"pastDrugHistory")
    dh_deleted=detail(driver,"DH",ids["dhDeleted"],"pastDrugHistory")
    if dm["patient_initials_null_flavor"] is not None or not str(dm["patient_initials"]).startswith("DM-"): raise AssertionError("DM recovery state invalid")
    if dh_survivor["start_date"]!="20200101" or dh_survivor["start_date_null_flavor"] is not None or not dh_deleted["deleted"]: raise AssertionError("DH recovery state invalid")
    record("ALL","dm-dh-recovery-state-verified")

    suffix=str(ids["aeSurvivor"])[-6:]
    ae_direct=f"AE DIRECT {suffix}"
    open_section(driver,"AE",ids["aeSurvivor"])
    ae_input=field_input(driver,"E.i.1.1a");ae_input.clear();ae_input.send_keys(ae_direct)
    save(driver,"AE direct reaction save")
    ae_saved=detail(driver,"AE",ids["aeSurvivor"],"reaction")
    if ae_saved["primary_source_reaction"]!=ae_direct: raise AssertionError(ae_saved)
    driver.refresh();loaded(driver)
    if field_input(driver,"E.i.1.1a").get_attribute("value")!=ae_direct: raise AssertionError("AE reload mismatch")
    record("AE","direct-save-reload",value=ae_direct)

    ae_nf=Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Date of Start of Reaction / Event"]'))
    ae_nf.select_by_value("ASKU");save(driver,"AE start date NullFlavor ASKU")
    ae_nf_saved=detail(driver,"AE",ids["aeSurvivor"],"reaction")
    if ae_nf_saved["start_date"] is not None or ae_nf_saved["start_date_null_flavor"]!="ASKU": raise AssertionError(ae_nf_saved)
    driver.refresh();loaded(driver)
    if Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Date of Start of Reaction / Event"]')).first_selected_option.get_attribute("value")!="ASKU": raise AssertionError("AE NF reload mismatch")
    record("AE","nullflavor-save-reload",nullFlavor="ASKU")

    field_input(driver,"E.i.4").send_keys("20200202");save(driver,"AE replace NullFlavor with date")
    ae_date=detail(driver,"AE",ids["aeSurvivor"],"reaction")
    if ae_date["start_date"]!="20200202" or ae_date["start_date_null_flavor"] is not None: raise AssertionError(ae_date)
    record("AE","nullflavor-to-value",value="20200202")

    open_section(driver,"AE",ids["aeDeleted"]);click(driver,"Remove");save(driver,"AE delete repeatable row")
    if not detail(driver,"AE",ids["aeDeleted"],"reaction")["deleted"] or ids["aeDeleted"] in [x["id"] for x in page(driver,"AE")["rows"]["rows"]]: raise AssertionError("AE row remains live")
    if detail(driver,"AE",ids["aeSurvivor"],"reaction")["primary_source_reaction"]!=ae_direct: raise AssertionError("AE sibling changed")
    record("AE","ui-delete-sibling-preserved")

    lb_direct=f"LB DIRECT {str(ids['lbSurvivor'])[-6:]}"
    open_section(driver,"LB",ids["lbSurvivor"])
    lb_input=field_input(driver,"F.r.2.1");lb_input.clear();lb_input.send_keys(lb_direct)
    save(driver,"LB direct test name save")
    lb_saved=detail(driver,"LB",ids["lbSurvivor"],"testResult")
    if lb_saved["test_name"]!=lb_direct: raise AssertionError(lb_saved)
    driver.refresh();loaded(driver)
    if field_input(driver,"F.r.2.1").get_attribute("value")!=lb_direct: raise AssertionError("LB reload mismatch")
    record("LB","direct-save-reload",value=lb_direct)

    lb_nf=Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Test Date"]'))
    lb_nf.select_by_value("UNK");save(driver,"LB test date NullFlavor UNK")
    lb_nf_saved=detail(driver,"LB",ids["lbSurvivor"],"testResult")
    if lb_nf_saved["test_date"] is not None or lb_nf_saved["test_date_null_flavor"]!="UNK": raise AssertionError(lb_nf_saved)
    driver.refresh();loaded(driver)
    if Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Test Date"]')).first_selected_option.get_attribute("value")!="UNK": raise AssertionError("LB NF reload mismatch")
    record("LB","nullflavor-save-reload",nullFlavor="UNK")

    field_input(driver,"F.r.1").send_keys("20200303");save(driver,"LB replace NullFlavor with date")
    lb_date=detail(driver,"LB",ids["lbSurvivor"],"testResult")
    if lb_date["test_date"]!="20200303" or lb_date["test_date_null_flavor"] is not None: raise AssertionError(lb_date)
    record("LB","nullflavor-to-value",value="20200303")

    open_section(driver,"LB",ids["lbDeleted"]);click(driver,"Remove");save(driver,"LB delete repeatable row")
    if not detail(driver,"LB",ids["lbDeleted"],"testResult")["deleted"] or ids["lbDeleted"] in [x["id"] for x in page(driver,"LB")["rows"]["rows"]]: raise AssertionError("LB row remains live")
    if detail(driver,"LB",ids["lbSurvivor"],"testResult")["test_name"]!=lb_direct: raise AssertionError("LB sibling changed")
    record("LB","ui-delete-sibling-preserved")

    click(driver,"Log out");WebDriverWait(driver,45).until(EC.presence_of_element_located((By.CSS_SELECTOR,"input[type=email]")));login(driver)
    checks=(("DM",None,"D.1",dm["patient_initials"]),("DH",ids["dhSurvivor"],"D.8.r.2a",dh_survivor["mpid_version"]),("AE",ids["aeSurvivor"],"E.i.1.1a",ae_direct),("LB",ids["lbSurvivor"],"F.r.2.1",lb_direct))
    for section,row_id,field_number,value in checks:
        open_section(driver,section,row_id)
        if field_input(driver,field_number).get_attribute("value")!=value: raise AssertionError(f"{section} relogin mismatch")
    driver.save_screenshot(str(ROOT/"remaining-sections-v2-final.png"));record("ALL","relogin-readback")

    db=db_oracle();proof["database"]=db
    if db["dm"]!={"id":ids["patient"],"initials":dm["patient_initials"],"nullFlavor":None}: raise AssertionError(db)
    for section,survivor,deleted in (("dh",ids["dhSurvivor"],ids["dhDeleted"]),("ae",ids["aeSurvivor"],ids["aeDeleted"]),("lb",ids["lbSurvivor"],ids["lbDeleted"])):
        by_id={x["id"]:x for x in db[section]}
        if by_id[survivor]["deleted"] or not by_id[deleted]["deleted"]: raise AssertionError(db)
    expected_records={ids["patient"],ids["dhSurvivor"],ids["dhDeleted"],ids["aeSurvivor"],ids["aeDeleted"],ids["lbSurvivor"],ids["lbDeleted"]}
    if expected_records-{x["recordId"] for x in db["audit"]}: raise AssertionError("audit targets incomplete")
    record("ALL","database-and-audit-oracle",auditCount=len(db["audit"]))
    proof["completed"]=True;proof["terminal"]="PASS"
except Exception as error:
    proof["error"]=f"{type(error).__name__}: {error}";proof["bodyOnError"]=driver.find_element(By.TAG_NAME,"body").text[-6000:]
    driver.save_screenshot(str(ROOT/"remaining-sections-v2-continuation-error.png"));raise
finally:
    persist();driver.quit()
