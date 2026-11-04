#!/usr/bin/env python3
"""Create isolated Presave import records through the real API."""

import http.cookiejar
import json
import os
import re
import urllib.error
import urllib.request
import uuid
from pathlib import Path


ROOT = Path(__file__).parent
BASE = os.environ.get("API_URL")
if not BASE:
    raise SystemExit("API_URL is required")
BOOTSTRAP = (Path(__file__).resolve().parents[2] / "crates/services/web-server/src/bootstrap.rs").read_text()
REASON = "New Data: Presave import resilience verification 20261104"


def constant(name):
    match = re.search(rf'const {name}: &str = "([^"]+)";', BOOTSTRAP)
    if not match:
        raise RuntimeError(f"missing bootstrap value {name}")
    return match.group(1)


class Client:
    def __init__(self):
        jar = http.cookiejar.CookieJar()
        self.opener = urllib.request.build_opener(
            urllib.request.HTTPCookieProcessor(jar)
        )
        self.events = []

    def request(self, method, path, data=None):
        request = urllib.request.Request(
            BASE + path,
            data=json.dumps(data).encode() if data is not None else None,
            headers={
                "Content-Type": "application/json",
                "x-e2br3-reason-for-change": REASON,
                "x-e2br3-change-category": "New Data",
            },
            method=method,
        )
        try:
            response = self.opener.open(request, timeout=30)
        except urllib.error.HTTPError as error:
            response = error
        raw = response.read()
        body = json.loads(raw) if raw else None
        event = {
            "method": method,
            "path": path,
            "status": response.code,
            "requestId": response.headers.get("x-request-id"),
        }
        self.events.append(event)
        if response.code >= 400:
            raise RuntimeError(f"{method} {path}: {response.code} {body}")
        return body

    def login(self):
        self.request(
            "POST",
            "/auth/v1/login",
            {
                "email": constant("DEMO_CRO_ADMIN_EMAIL"),
                "pwd": constant("INITIAL_PASSWORD"),
            },
        )


def create(client, path, rows, key):
    body = client.request("POST", path, {"data": {"rows": rows}})
    return body["data"]["rows"][key]["id"]


client = Client()
client.login()
suffix = uuid.uuid4().hex[:8].upper()

sender_label = f"IMPORT-SENDER-{suffix}"
sender_id = create(
    client,
    "/api/presaves/senders",
    {
        "sender": {
            "senderType": "1",
            "organizationName": sender_label,
            "city": "Import City",
            "countryCode": "US",
        },
        "gateways": [],
        "responsiblePersons": [
            {
                "sequenceNumber": 1,
                "personGivenName": "Import",
                "personFamilyName": "Sender",
                "isDefault": True,
            }
        ],
    },
    "sender",
)

receiver_label = f"IMPORT-RECEIVER-{suffix}"
receiver_id = create(
    client,
    "/api/presaves/receivers",
    {
        "receiver": {
            "receiverType": "Regulatory Authority",
            "organizationName": receiver_label,
            "description": "Receiver import snapshot",
        },
        "consignees": [],
        "routes": [],
    },
    "receiver",
)

reporter_label = f"IMPORT-REPORTER-{suffix}"
reporter_id = create(
    client,
    "/api/presaves/reporters",
    {
        "reporter": {
            "reporterGivenName": "Import",
            "reporterFamilyName": "Reporter",
            "organization": reporter_label,
            "countryCode": "US",
            "qualification": "1",
            "primarySourceRegulatory": "1",
        }
    },
    "reporter",
)

product_label = f"IMPORT-PRODUCT-{suffix}"
source_product_id = f"PID-{suffix}"
product_id = create(
    client,
    "/api/presaves/products",
    {
        "product": {
            "senderPresaveId": sender_id,
            "receiverPresaveId": receiver_id,
            "productId": source_product_id,
            "medicinalProduct": product_label,
            "drugBrandName": f"BRAND-{suffix}",
        },
        "activeSubstances": [
            {
                "sequenceNumber": 1,
                "substanceName": f"ACTIVE-{suffix}",
            }
        ],
    },
    "product",
)

study_label = f"IMPORT-STUDY-{suffix}"
study_number = f"STUDY-{suffix}"
study_id = create(
    client,
    "/api/presaves/studies",
    {
        "study": {
            "productPresaveId": product_id,
            "studyName": study_label,
            "sponsorStudyNumber": study_number,
            "studyTypeReaction": "1",
        },
        "products": [],
        "reporters": [],
        "registrationNumbers": [
            {
                "sequenceNumber": 1,
                "registrationNumber": f"REG-{suffix}",
                "countryCode": "US",
            }
        ],
        "fdaCrossReportedInds": [],
    },
    "study",
)

narrative_text = f"Presave import narrative {suffix}."
narrative_id = create(
    client,
    "/api/presaves/narratives",
    {
        "narrative": {
            "caseNarrative": narrative_text,
            "caseNarrativeNotation": "en",
            "additionalInformation": f"Additional {suffix}",
        }
    },
    "narrative",
)

case_body = client.request(
    "POST",
    "/api/cases",
    {
        "data": {
            "safetyReportIdentification": {
                "safetyReportId": f"IMPORT-CASE-{suffix}"
            },
            "status": "draft",
        }
    },
)

result = {
    "caseId": case_body["data"]["id"],
    "suffix": suffix,
    "sender": {"id": sender_id, "label": sender_label},
    "receiver": {"id": receiver_id, "label": receiver_label},
    "reporter": {"id": reporter_id, "label": reporter_label},
    "product": {
        "id": product_id,
        "label": product_label,
        "sourceProductId": source_product_id,
        "substance": f"ACTIVE-{suffix}",
    },
    "study": {"id": study_id, "label": study_label, "number": study_number},
    "narrative": {"id": narrative_id, "text": narrative_text},
    "events": client.events,
}
(ROOT / "seed-result.json").write_text(json.dumps(result, indent=2))
print(json.dumps(result, indent=2))
