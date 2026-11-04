#!/usr/bin/env python3
"""Inspect real Case import controls for stable browser selectors."""

import json
import os
import re
from pathlib import Path

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.firefox.service import Service
from selenium.webdriver.support.ui import WebDriverWait


ROOT = Path(__file__).parent
UI_URL = os.environ["UI_URL"]
seed = json.loads((ROOT / "seed-result.json").read_text())
source = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()


def constant(name):
    return re.search(rf'const {name}: &str = "([^"]+)";', source).group(1)


options = webdriver.FirefoxOptions()
options.binary_location = os.environ["FIREFOX_BINARY"]
driver = webdriver.Firefox(
    service=Service(os.environ["GECKODRIVER"]), options=options
)
result = {}
try:
    wait = WebDriverWait(driver, 50)
    driver.get(UI_URL + "/")
    wait.until(lambda d: d.find_element(By.CSS_SELECTOR, "input[type=email]")).send_keys(
        constant("DEMO_CRO_ADMIN_EMAIL")
    )
    driver.find_element(By.CSS_SELECTOR, "input[type=password]").send_keys(
        constant("INITIAL_PASSWORD")
    )
    driver.find_element(By.CSS_SELECTOR, "button[type=submit]").click()
    wait.until(lambda d: not d.find_elements(By.CSS_SELECTOR, "input[type=email]"))
    for section in ["SD", "RP", "DG", "SI", "NR"]:
        driver.get(f"{UI_URL}/ICH/case/{seed['caseId']}/detail/{section}")
        wait.until(lambda d: "Loading" not in d.find_element(By.TAG_NAME, "body").text)
        result[section] = {
            "url": driver.current_url,
            "buttons": [
                value
                for value in [
                    (element.text or element.get_attribute("aria-label") or "").strip()
                    for element in driver.find_elements(By.TAG_NAME, "button")
                ]
                if value
            ],
            "inputs": [
                {
                    key: element.get_attribute(key)
                    for key in ["id", "name", "placeholder", "type", "value"]
                }
                for element in driver.find_elements(By.CSS_SELECTOR, "input,textarea,select")
            ],
            "body": driver.find_element(By.TAG_NAME, "body").text[:3000],
        }
finally:
    driver.quit()
    (ROOT / "ui-controls.json").write_text(json.dumps(result, indent=2))

print(json.dumps({key: value["buttons"] for key, value in result.items()}, indent=2))
