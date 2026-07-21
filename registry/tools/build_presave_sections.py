import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import extract_presave_fields as inventory
import validate


ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parent

DOMAIN_SECTIONS = {
    "sender": "C",
    "receiver": "C",
    "product": "G",
    "study": "C",
    "narrative": "H",
}

FRONTEND_TO_BACKEND = {
    "sender": {
        "senderType": ("SenderPresave", "sender_type"),
        "senderOrganization": ("SenderPresave", "organization_name"),
        "senderOrganizationNotation": ("SenderPresave", "organization_name_notation"),
        "senderDefault": ("SenderPresave", "is_default"),
        "senderStreetAddress": ("SenderPresave", "street_address"),
        "senderCity": ("SenderPresave", "city"),
        "senderState": ("SenderPresave", "state"),
        "senderPostcode": ("SenderPresave", "postcode"),
        "senderCountryCode": ("SenderPresave", "country_code"),
        "senderTelephone": ("SenderPresave", "telephone"),
        "senderFax": ("SenderPresave", "fax"),
        "senderEmail": ("SenderPresave", "email"),
    },
    "receiver": {
        "receiverOrganization": ("ReceiverPresave", "organization_name"),
        "receiverType": ("ReceiverPresave", "receiver_type"),
        "dayCountRule": ("ReceiverPresave", "day_count_rule"),
        "nonSaeSpontaneousDayCount": ("ReceiverPresave", "nsae_non_solicited_day_count"),
        "nonSaeSpontaneousNotApplicable": ("ReceiverPresave", "nsae_non_solicited_not_applicable"),
        "saeSpontaneousDayCount": ("ReceiverPresave", "sae_non_solicited_day_count"),
        "saeSpontaneousNotApplicable": ("ReceiverPresave", "sae_non_solicited_not_applicable"),
        "nonSaeSolicitedDayCount": ("ReceiverPresave", "nsae_solicited_day_count"),
        "nonSaeSolicitedNotApplicable": ("ReceiverPresave", "nsae_solicited_not_applicable"),
        "saeSolicitedDayCount": ("ReceiverPresave", "sae_solicited_day_count"),
        "saeSolicitedNotApplicable": ("ReceiverPresave", "sae_solicited_not_applicable"),
        "receiverDescription": ("ReceiverPresave", "description"),
        "receiverId": ("ReceiverPresave", "receiver_identifier"),
    },
    "product": {
        "productId": ("ProductPresave", "product_id"),
        "medicinalProduct": ("ProductPresave", "medicinal_product"),
        "medicinalProductNotation": ("ProductPresave", "medicinal_product_notation"),
        "drugBrandName": ("ProductPresave", "brand_name"),
        "obtainDrugCountry": ("ProductPresave", "obtain_drug_country"),
        "drugAuthorizationCountry": ("ProductPresave", "drug_authorization_country"),
        "drugAuthorizationHolder": ("ProductPresave", "drug_authorization_holder"),
        "drugAuthorizationNumber": ("ProductPresave", "drug_authorization_number"),
        "preApprovalIpName": ("ProductPresave", "preapproval_ip_name"),
        "originalManufacturer": ("ProductPresave", "original_manufacturer"),
        "senderPresaveId": ("ProductPresave", "sender_presave_id"),
        "receiverPresaveId": ("ProductPresave", "receiver_presave_id"),
        "productDescription": ("ProductPresave", "product_description"),
        "mpidVersion": ("ProductPresave", "mpid_version"),
        "mpid": ("ProductPresave", "mpid"),
        "mfdsMpidVersion": ("ProductPresave", "mfds_mpid_version"),
        "mfdsMpid": ("ProductPresave", "mfds_mpid"),
        "phpidVersion": ("ProductPresave", "phpid_version"),
        "phpid": ("ProductPresave", "phpid"),
        "investigationalProductBlinded": ("ProductPresave", "investigational_product_blinded"),
        "holderApplicantNameNotation": ("ProductPresave", "holder_applicant_name_notation"),
    },
    "study": {
        "productPresaveId": ("StudyPresave", "product_presave_id"),
        "excludeCaseKeyFromSync": ("StudyPresave", "exclude_case_key_from_sync"),
        "studyName": ("StudyPresave", "study_name"),
        "studyNameNotation": ("StudyPresave", "study_name_notation"),
        "sponsorStudyNumberKind": ("StudyPresave", "sponsor_study_number_kind"),
        "sponsorStudyNumber": ("StudyPresave", "sponsor_study_number"),
        "studyTypeReaction": ("StudyPresave", "study_type_reaction"),
        "fdaIndNumberOccurred": ("StudyPresave", "fda_ind_number_occurred"),
        "fdaPreAndaNumberOccurred": ("StudyPresave", "fda_pre_anda_number_occurred"),
        "edcSync": ("StudyPresave", "edc_sync"),
    },
    "narrative": {
        "caseNarrative": ("NarrativePresave", "case_narrative"),
        "caseNarrativeNotation": ("NarrativePresave", "case_narrative_notation"),
        "additionalInformation": ("NarrativePresave", "additional_information"),
    },
}

OFFICIAL = {
    "sender": {
        ("SenderPresave", "sender_type"): "C.3.1",
        ("SenderPresave", "organization_name"): "C.3.2",
        ("SenderPresaveResponsiblePerson", "department"): "C.3.3.1",
        ("SenderPresaveResponsiblePerson", "person_title"): "C.3.3.2",
        ("SenderPresaveResponsiblePerson", "person_given_name"): "C.3.3.3",
        ("SenderPresaveResponsiblePerson", "person_middle_name"): "C.3.3.4",
        ("SenderPresaveResponsiblePerson", "person_family_name"): "C.3.3.5",
        ("SenderPresave", "street_address"): "C.3.4.1",
        ("SenderPresave", "city"): "C.3.4.2",
        ("SenderPresave", "state"): "C.3.4.3",
        ("SenderPresave", "postcode"): "C.3.4.4",
        ("SenderPresave", "country_code"): "C.3.4.5",
        ("SenderPresave", "telephone"): "C.3.4.6",
        ("SenderPresave", "fax"): "C.3.4.7",
        ("SenderPresave", "email"): "C.3.4.8",
        ("SenderPresaveGateway", "sender_identifier"): "N.2.r.2",
    },
    "receiver": {
        ("ReceiverPresave", "organization_name"): "local.receiver.1",
        ("ReceiverPresave", "receiver_type"): "local.receiver.2",
        ("ReceiverPresaveRoute", "batch_receiver_identifier"): "N.1.4",
        ("ReceiverPresaveRoute", "message_receiver_identifier"): "N.2.r.3",
    },
    "product": {
        ("ProductPresave", "medicinal_product"): "G.k.2.2",
        ("ProductPresave", "mpid_version"): "G.k.2.1.1a",
        ("ProductPresave", "mpid"): "G.k.2.1.1b",
        ("ProductPresave", "mfds_mpid_version"): "G.k.2.1.KR.1a",
        ("ProductPresave", "mfds_mpid"): "G.k.2.1.KR.1b",
        ("ProductPresave", "phpid_version"): "G.k.2.1.2a",
        ("ProductPresave", "phpid"): "G.k.2.1.2b",
        ("ProductPresave", "obtain_drug_country"): "G.k.2.4",
        ("ProductPresave", "investigational_product_blinded"): "G.k.2.5",
        ("ProductPresave", "drug_authorization_number"): "G.k.3.1",
        ("ProductPresave", "drug_authorization_country"): "G.k.3.2",
        ("ProductPresave", "drug_authorization_holder"): "G.k.3.3",
        ("ProductPresaveActiveSubstance", "substance_name"): "G.k.2.3.r.1",
        ("ProductPresaveActiveSubstance", "substance_termid_version"): "G.k.2.3.r.2a",
        ("ProductPresaveActiveSubstance", "substance_termid"): "G.k.2.3.r.2b",
        ("ProductPresaveActiveSubstance", "mfds_version"): "G.k.2.3.r.1.KR.1a",
        ("ProductPresaveActiveSubstance", "mfds_id"): "G.k.2.3.r.1.KR.1b",
        ("ProductPresaveActiveSubstance", "strength_value"): "G.k.2.3.r.3a",
        ("ProductPresaveActiveSubstance", "strength_unit"): "G.k.2.3.r.3b",
    },
    "study": {
        ("StudyPresave", "study_name"): "C.5.2",
        ("StudyPresave", "sponsor_study_number"): "C.5.3",
        ("StudyPresave", "study_type_reaction"): "C.5.4",
        ("StudyPresave", "fda_ind_number_occurred"): "FDA.C.5.5a",
        ("StudyPresave", "fda_pre_anda_number_occurred"): "FDA.C.5.5b",
        ("StudyPresaveRegistrationNumber", "registration_number"): "C.5.1.r.1",
        ("StudyPresaveRegistrationNumber", "country_code"): "C.5.1.r.2",
        ("StudyPresaveFdaCrossReportedIndNumber", "ind_number"): "FDA.C.5.6.r",
    },
    "narrative": {
        ("NarrativePresave", "case_narrative"): "H.1",
        ("NarrativePresave", "additional_information"): "H.additionalInformation",
    },
}


def case_rows() -> dict[str, dict]:
    rows = {}
    index = json.loads((ROOT / "index.json").read_text(encoding="utf-8"))
    for relative in index["sections"]:
        for row in json.loads((ROOT / relative).read_text(encoding="utf-8")):
            rows[row["e2br3_code"]] = row
    return rows


def slug(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9]+", ".", value).strip(".")


def mapping(status: str, **values: str) -> dict:
    return {"status": status, **values}


def build_section(section: str) -> list[dict]:
    cases = case_rows()
    frontend_keys = {
        key.split(".", 1)[1]
        for key in inventory.extract_presave_frontend(REPO, section)
    }
    backend_keys = {
        tuple(key.split(".", 1))
        for key in inventory.extract_section_backend(REPO, section)
    }
    frontend_map = FRONTEND_TO_BACKEND[section]
    backend_to_frontend = {value: key for key, value in frontend_map.items()}
    official = OFFICIAL[section]
    consumed_frontend = set()
    consumed_backend = set()
    rows = []

    def add_row(code: str, backend_key=None, frontend_field=None, local=False):
        case = cases.get(code, {})
        backend_status = "mapped" if backend_key else "not_applicable"
        frontend_status = "mapped" if frontend_field else "not_applicable"
        status = (
            "complete"
            if backend_key and frontend_field and not local
            else "intentionally_unmapped"
            if not local
            else "not_applicable"
        )
        authority = case.get("authority", "ICH")
        row_section = case.get("section", DOMAIN_SECTIONS[section])
        label = case.get("label") or (frontend_field or ".".join(backend_key or (code,)))
        row = {
            "id": f"presave.{section}.{slug(code)}",
            "e2br3_code": code,
            "label": label,
            "section": row_section,
            "authority": authority,
            "status": status,
            "backend": mapping(
                backend_status,
                **(
                    {
                        "model": backend_key[0],
                        "field": backend_key[1],
                        "file": inventory.PRESAVE_MODEL_FILE,
                        "evidence": f"{backend_key[0]}.{backend_key[1]} persists the presave value.",
                    }
                    if backend_key
                    else {}
                ),
            ),
            "frontend": mapping(
                frontend_status,
                **(
                    {
                        "section": section,
                        "field": frontend_field,
                        "file": inventory.PRESAVE_SECTIONS[section].form_file,
                        "evidence": f"{inventory.PRESAVE_SECTIONS[section].interface_name}.{frontend_field} exposes the presave value.",
                    }
                    if frontend_field
                    else {}
                ),
            ),
            "action": "",
            "notes": "",
        }
        if local:
            row["local_only"] = True
        rows.append(row)

    for backend_key, code in official.items():
        frontend_field = backend_to_frontend.get(backend_key)
        if frontend_field not in frontend_keys:
            frontend_field = None
        add_row(code, backend_key, frontend_field)
        consumed_backend.add(backend_key)
        if frontend_field:
            consumed_frontend.add(frontend_field)

    for frontend_field, backend_key in sorted(frontend_map.items()):
        if frontend_field not in frontend_keys or frontend_field in consumed_frontend:
            continue
        if backend_key not in backend_keys or backend_key in consumed_backend:
            continue
        add_row(
            f"local.presave.{section}.{slug(frontend_field)}",
            backend_key,
            frontend_field,
            local=True,
        )
        consumed_frontend.add(frontend_field)
        consumed_backend.add(backend_key)

    for frontend_field in sorted(frontend_keys - consumed_frontend):
        add_row(
            f"local.presave.{section}.frontend.{slug(frontend_field)}",
            frontend_field=frontend_field,
            local=True,
        )

    for backend_key in sorted(backend_keys - consumed_backend):
        add_row(
            f"local.presave.{section}.backend.{slug('.'.join(backend_key))}",
            backend_key=backend_key,
            local=True,
        )
    return rows


def main() -> int:
    sections_dir = ROOT / "presaves" / "sections"
    sections_dir.mkdir(parents=True, exist_ok=True)
    for section in DOMAIN_SECTIONS:
        name = f"{DOMAIN_SECTIONS[section].lower()}-{section}.json"
        (sections_dir / name).write_text(
            json.dumps(build_section(section), indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
