#!/usr/bin/env python3
"""Finish grammar v2 with LB in a fresh browser session and final all-section oracles."""

import json, os, re, subprocess
from pathlib import Path
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.firefox.service import Service
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.ui import Select, WebDriverWait

ROOT=Path(__file__).parent; UI_URL=os.environ["UI_URL"]
CASE_ID=json.loads((ROOT/"seed-result.json").read_text())["caseId"]
PREVIOUS=json.loads((ROOT/"remaining-sections-v2-final.json").read_text())
RESULT=ROOT/"remaining-sections-v2-complete.json"; ids=json.loads((ROOT/"remaining-sections-v2-result.json").read_text())["ids"]
SOURCE=(Path(__file__).resolve().parents[2]/"crates/services/web-server/src/bootstrap.rs").read_text()
proof={"completed":False,"grammarVersion":2,"caseId":CASE_ID,"continuedFrom":"remaining-sections-v2-final.json","steps":PREVIOUS["steps"][:]}

def persist(): RESULT.write_text(json.dumps(proof,ensure_ascii=False,indent=2))
def record(section,name,**details): proof["steps"].append({"section":section,"name":name,**details});persist()
def constant(name):
 m=re.search(rf'const {name}: &str = "([^"]+)";',SOURCE)
 if not m: raise RuntimeError(name)
 return m.group(1)
def loaded(d): WebDriverWait(d,60).until(lambda x:x.execute_script("return document.readyState")=="complete" and "Loading" not in x.find_element(By.TAG_NAME,"body").text)
def api(d,method,path,payload=None,reason=None,expected=(200,)):
 r=d.execute_async_script("""
 const [method,path,payload,reason]=arguments,done=arguments[arguments.length-1],headers={};
 if(payload!==null)headers['Content-Type']='application/json';
 if(reason!==null){headers['x-e2br3-reason-for-change']=reason;headers['x-e2br3-change-category']='Edited Data';}
 fetch(path,{method,credentials:'same-origin',headers,body:payload===null?undefined:JSON.stringify(payload)})
 .then(async x=>done({status:x.status,text:await x.text()})).catch(e=>done({error:String(e)}));
 """,method,path,payload,reason)
 if "error" in r: raise AssertionError(r)
 r["body"]=json.loads(r["text"]) if r["text"] else None;del r["text"]
 if r["status"] not in expected: raise AssertionError(r)
 return r
def login(d):
 d.get(UI_URL+"/");WebDriverWait(d,60).until(EC.presence_of_element_located((By.CSS_SELECTOR,"input[type=email]"))).send_keys(constant("DEMO_CRO_ADMIN_EMAIL"))
 d.find_element(By.CSS_SELECTOR,"input[type=password]").send_keys(constant("INITIAL_PASSWORD"));d.find_element(By.CSS_SELECTOR,"button[type=submit]").click()
 WebDriverWait(d,60).until(lambda x:not x.find_elements(By.CSS_SELECTOR,"input[type=email]"))
def click(d,label):
 n=WebDriverWait(d,45).until(lambda x:next((e for e in x.find_elements(By.TAG_NAME,"button") if e.is_displayed() and e.is_enabled() and e.text.strip()==label),False));d.execute_script("arguments[0].scrollIntoView({block:'center'})",n);n.click()
def save(d,reason):
 click(d,"Save");dialog=WebDriverWait(d,45).until(EC.visibility_of_element_located((By.XPATH,"//*[normalize-space()='Confirm']/ancestor::*[@role='dialog'][1]")))
 category=dialog.find_elements(By.CSS_SELECTOR,'input[value="Edited Data"]') or dialog.find_elements(By.CSS_SELECTOR,'input[value="New Data"]')
 if len(category)!=1: raise AssertionError("audit category")
 category[0].click();dialog.find_element(By.CSS_SELECTOR,'textarea[aria-label="Reason comments"]').send_keys(reason);dialog.find_element(By.XPATH,".//button[normalize-space()='Confirm']").click()
 WebDriverWait(d,60).until(EC.invisibility_of_element(dialog));WebDriverWait(d,60).until(lambda x:"saving…" not in x.find_element(By.TAG_NAME,"body").text.lower() and "saved" in x.find_element(By.TAG_NAME,"body").text.lower())
def open_section(d,section,row_id=None):
 suffix=f"/{row_id}" if row_id else "";expected=f"/ICH/case/{CASE_ID}/detail/{section}{suffix}";d.get(UI_URL+expected)
 WebDriverWait(d,60).until(lambda x:expected in x.current_url and x.find_elements(By.CSS_SELECTOR,f'[data-case-section-shell="{section.lower()}"]'));loaded(d)
def page(d,section): return api(d,"GET",f"/api/cases/{CASE_ID}/editor/pages/{section}?authorities=ich")["body"]
def detail(d,section,row_id,key): return api(d,"GET",f"/api/cases/{CASE_ID}/editor/pages/{section}/rows/{row_id}?authorities=ich")["body"]["data"][key]
def field_input(d,number):
 root=WebDriverWait(d,45).until(EC.visibility_of_element_located((By.CSS_SELECTOR,f'[data-e2b-field-number="{number}"]')));node=next((x for x in root.find_elements(By.CSS_SELECTOR,"input") if x.is_displayed()),None)
 if node is None: raise AssertionError(number)
 return node
def db_oracle():
 q=lambda values:",".join(f"'{v}'::uuid" for v in values);records=[ids[k] for k in ("patient","dhSurvivor","dhDeleted","aeSurvivor","aeDeleted","lbSurvivor","lbDeleted")]
 sql=f"""SELECT json_build_object(
 'dm',(SELECT json_build_object('id',id,'initials',patient_initials,'nullFlavor',patient_initials_null_flavor) FROM patient_information WHERE id='{ids['patient']}'),
 'dh',COALESCE((SELECT json_agg(json_build_object('id',id,'mpidVersion',mpid_version,'startDate',start_date,'startDateNullFlavor',start_date_null_flavor,'deleted',deleted) ORDER BY id) FROM past_drug_history WHERE id IN ({q([ids['dhSurvivor'],ids['dhDeleted']])})),'[]'::json),
 'ae',COALESCE((SELECT json_agg(json_build_object('id',id,'reaction',primary_source_reaction,'startDate',start_date,'startDateNullFlavor',start_date_null_flavor,'deleted',deleted) ORDER BY id) FROM reactions WHERE id IN ({q([ids['aeSurvivor'],ids['aeDeleted']])})),'[]'::json),
 'lb',COALESCE((SELECT json_agg(json_build_object('id',id,'testName',test_name,'testDate',test_date,'testDateNullFlavor',test_date_null_flavor,'deleted',deleted) ORDER BY id) FROM test_results WHERE id IN ({q([ids['lbSurvivor'],ids['lbDeleted']])})),'[]'::json),
 'audit',COALESCE((SELECT json_agg(json_build_object('recordId',record_id,'action',action,'changedFields',changed_fields,'reason',reason_for_change) ORDER BY id) FROM audit_logs WHERE record_id IN ({q(records)})),'[]'::json));"""
 r=subprocess.run(["docker","exec",os.environ["POSTGRES_CONTAINER"],"psql","-U","postgres","-d",os.environ["STATE_VERIFIER_DB"],"-At","-v","ON_ERROR_STOP=1","-c",sql],check=True,capture_output=True,text=True);return json.loads(r.stdout.strip())

opts=webdriver.FirefoxOptions();opts.binary_location=os.environ["FIREFOX_BINARY"];driver=webdriver.Firefox(service=Service(os.environ["GECKODRIVER"]),options=opts);driver.set_window_size(1440,1000);driver.set_script_timeout(45)
try:
 if driver.capabilities.get("moz:headless") is not False: raise AssertionError("not headed")
 login(driver)
 ae=detail(driver,"AE",ids["aeSurvivor"],"reaction");ae_deleted=detail(driver,"AE",ids["aeDeleted"],"reaction")
 if ae["start_date"]!="20200202" or ae["start_date_null_flavor"] is not None or not ae_deleted["deleted"]: raise AssertionError("AE recovery")
 record("ALL","ae-recovery-state-verified")

 lb_direct=f"LB DIRECT {str(ids['lbSurvivor'])[-6:]}";open_section(driver,"LB",ids["lbSurvivor"])
 inp=field_input(driver,"F.r.2.1");inp.clear();inp.send_keys(lb_direct);save(driver,"LB direct test name save")
 saved=detail(driver,"LB",ids["lbSurvivor"],"testResult")
 if saved["test_name"]!=lb_direct: raise AssertionError(saved)
 driver.refresh();loaded(driver)
 if field_input(driver,"F.r.2.1").get_attribute("value")!=lb_direct: raise AssertionError("LB reload")
 record("LB","direct-save-reload",value=lb_direct)

 nf=Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Test Date"]'));nf.select_by_value("UNK");save(driver,"LB test date NullFlavor UNK")
 saved=detail(driver,"LB",ids["lbSurvivor"],"testResult")
 if saved["test_date"] is not None or saved["test_date_null_flavor"]!="UNK": raise AssertionError(saved)
 driver.refresh();loaded(driver)
 if Select(driver.find_element(By.CSS_SELECTOR,'select[aria-label="Null flavor for Test Date"]')).first_selected_option.get_attribute("value")!="UNK": raise AssertionError("LB NF reload")
 record("LB","nullflavor-save-reload",nullFlavor="UNK")

 field_input(driver,"F.r.1").send_keys("20200303");save(driver,"LB replace NullFlavor with date")
 saved=detail(driver,"LB",ids["lbSurvivor"],"testResult")
 if saved["test_date"]!="20200303" or saved["test_date_null_flavor"] is not None: raise AssertionError(saved)
 record("LB","nullflavor-to-value",value="20200303")

 open_section(driver,"LB",ids["lbDeleted"]);click(driver,"Remove");save(driver,"LB delete repeatable row")
 if not detail(driver,"LB",ids["lbDeleted"],"testResult")["deleted"] or ids["lbDeleted"] in [x["id"] for x in page(driver,"LB")["rows"]["rows"]]: raise AssertionError("LB live")
 if detail(driver,"LB",ids["lbSurvivor"],"testResult")["test_name"]!=lb_direct: raise AssertionError("LB sibling")
 record("LB","ui-delete-sibling-preserved")

 click(driver,"Log out");WebDriverWait(driver,45).until(EC.presence_of_element_located((By.CSS_SELECTOR,"input[type=email]")));login(driver)
 dm=page(driver,"DM")["rows"]["patientInformation"];dh=detail(driver,"DH",ids["dhSurvivor"],"pastDrugHistory");ae=detail(driver,"AE",ids["aeSurvivor"],"reaction")
 checks=(("DM",None,"D.1",dm["patient_initials"]),("DH",ids["dhSurvivor"],"D.8.r.2a",dh["mpid_version"]),("AE",ids["aeSurvivor"],"E.i.1.1a",ae["primary_source_reaction"]),("LB",ids["lbSurvivor"],"F.r.2.1",lb_direct))
 for section,row_id,number,value in checks:
  open_section(driver,section,row_id)
  if field_input(driver,number).get_attribute("value")!=value: raise AssertionError(section)
 driver.save_screenshot(str(ROOT/"remaining-sections-v2-complete.png"));record("ALL","relogin-readback")

 db=db_oracle();proof["database"]=db
 expected={ids[k] for k in ("patient","dhSurvivor","dhDeleted","aeSurvivor","aeDeleted","lbSurvivor","lbDeleted")}
 if expected-{x["recordId"] for x in db["audit"]}: raise AssertionError("audit")
 for section,survivor,deleted in (("dh",ids["dhSurvivor"],ids["dhDeleted"]),("ae",ids["aeSurvivor"],ids["aeDeleted"]),("lb",ids["lbSurvivor"],ids["lbDeleted"])):
  rows={x["id"]:x for x in db[section]}
  if rows[survivor]["deleted"] or not rows[deleted]["deleted"]: raise AssertionError(section)
 record("ALL","database-and-audit-oracle",auditCount=len(db["audit"]));proof["completed"]=True;proof["terminal"]="PASS"
except Exception as e:
 proof["error"]=f"{type(e).__name__}: {e}";proof["currentUrl"]=driver.current_url;proof["bodyOnError"]=driver.find_element(By.TAG_NAME,"body").text[-6000:];driver.save_screenshot(str(ROOT/"lb-v2-error.png"));raise
finally:
 persist();driver.quit()
