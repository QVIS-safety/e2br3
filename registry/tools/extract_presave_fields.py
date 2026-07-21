import os
import re
from dataclasses import dataclass
from pathlib import Path

import validate


@dataclass(frozen=True)
class PresaveSectionConfig:
    frontend_section: str
    interface_name: str
    form_file: str
    backend_models: tuple[str, ...]
    transfer_files: tuple[str, ...]


@dataclass(frozen=True)
class TransferSpec:
    source_model: str
    source_field: str
    target_model: str
    target_field: str
    patterns: tuple[str, ...]


PRESAVE_MODEL_FILE = "crates/libs/lib-core/src/model/presave.rs"
PRESAVE_TYPE_FILE = "lib/types/presave.ts"
PRESAVE_SECTIONS = {
    "sender": PresaveSectionConfig(
        "sender",
        "SenderPresaveData",
        "components/presave/SenderForm.tsx",
        ("SenderPresave", "SenderPresaveGateway", "SenderPresaveResponsiblePerson"),
        ("app/(protected)/[authority]/case/[id]/detail/SD/hooks/useSenderPresaveImport.ts",),
    ),
    "receiver": PresaveSectionConfig(
        "receiver",
        "ReceiverPresaveData",
        "components/presave/ReceiverForm.tsx",
        ("ReceiverPresave", "ReceiverPresaveConsignee", "ReceiverPresaveRoute"),
        ("app/(protected)/[authority]/case/[id]/detail/SD/hooks/useReceiverPresaveImport.ts",),
    ),
    "product": PresaveSectionConfig(
        "product",
        "ProductPresaveData",
        "components/presave/ProductForm.tsx",
        ("ProductPresave", "ProductPresaveActiveSubstance"),
        (
            "app/(protected)/[authority]/case/[id]/detail/DG/components/SectionG.tsx",
            "app/(protected)/[authority]/case/[id]/detail/DG/hooks/useSectionGDrugs.ts",
        ),
    ),
    "reporter": PresaveSectionConfig(
        "reporter",
        "ReporterPresaveData",
        "components/presave/ReporterForm.tsx",
        ("ReporterPresave",),
        ("app/(protected)/[authority]/case/[id]/detail/RP/model/rpModel.ts",),
    ),
    "study": PresaveSectionConfig(
        "study",
        "StudyPresaveData",
        "components/presave/StudyForm.tsx",
        (
            "StudyPresave",
            "StudyPresaveRegistrationNumber",
            "StudyPresaveFdaCrossReportedIndNumber",
            "StudyPresaveProduct",
            "StudyPresaveReporter",
        ),
        ("app/(protected)/[authority]/case/[id]/detail/SI/hooks/useStudyImport.ts",),
    ),
    "narrative": PresaveSectionConfig(
        "narrative",
        "NarrativePresaveData",
        "components/presave/NarrativeForm.tsx",
        ("NarrativePresave",),
        ("app/(protected)/[authority]/case/[id]/detail/NR/NRPage.tsx",),
    ),
}

NESTED_FRONTEND_FIELDS = {
    "product": {
        "activeSubstances": (
            "substanceName",
            "substanceTermIdVersion",
            "substanceTermId",
            "mfdsVersion",
            "mfdsId",
            "substanceStrengthValue",
            "substanceStrengthUnit",
        ),
    },
    "study": {
        "studyRegistrationNumbers": ("registrationNumber", "countryCode"),
        "fdaCrossReportedIndNumbers": ("indNumber",),
    },
}

def transfer_call(target: str, source: str) -> str:
    return (
        rf"\b(?:importValue|setValue)\(\s*[\"'`]"
        rf"{re.escape(target)}[\"'`]\s*,\s*{re.escape(source)}"
    )


def value_assignment(target: str, source: str) -> str:
    return rf"\b{re.escape(target)}\s*[:=]\s*{re.escape(source)}"


TRANSFER_SPECS = {
    "sender": (
        TransferSpec("SenderPresave", "sender_type", "SenderInformation", "sender_type", (transfer_call("safetyReportIdentification.senderType", "d.senderType"),)),
        TransferSpec("SenderPresave", "organization_name", "SenderInformation", "organization_name", (transfer_call("safetyReportIdentification.senderOrganization", "d.senderOrganization"),)),
        TransferSpec("SenderPresave", "street_address", "SenderInformation", "street_address", (transfer_call("safetyReportIdentification.senderStreetAddress", "d.senderStreetAddress"),)),
        TransferSpec("SenderPresave", "city", "SenderInformation", "city", (transfer_call("safetyReportIdentification.senderCity", "d.senderCity"),)),
        TransferSpec("SenderPresave", "state", "SenderInformation", "state", (transfer_call("safetyReportIdentification.senderState", "d.senderState"),)),
        TransferSpec("SenderPresave", "postcode", "SenderInformation", "postcode", (transfer_call("safetyReportIdentification.senderPostcode", "d.senderPostcode"),)),
        TransferSpec("SenderPresave", "country_code", "SenderInformation", "country_code", (transfer_call("safetyReportIdentification.senderCountryCode", "d.senderCountryCode"),)),
        TransferSpec("SenderPresave", "telephone", "SenderInformation", "telephone", (transfer_call("safetyReportIdentification.senderTelephone", "d.senderTelephone"),)),
        TransferSpec("SenderPresave", "fax", "SenderInformation", "fax", (transfer_call("safetyReportIdentification.senderFax", "d.senderFax"),)),
        TransferSpec("SenderPresave", "email", "SenderInformation", "email", (transfer_call("safetyReportIdentification.senderEmail", "d.senderEmail"),)),
        TransferSpec("SenderPresaveResponsiblePerson", "department", "SenderInformation", "department", (transfer_call("safetyReportIdentification.senderDepartment", "defaultPerson.department"),)),
        TransferSpec("SenderPresaveResponsiblePerson", "person_title", "SenderInformation", "person_title", (transfer_call("safetyReportIdentification.senderPersonTitle", "defaultPerson.title"),)),
        TransferSpec("SenderPresaveResponsiblePerson", "person_given_name", "SenderInformation", "person_given_name", (transfer_call("safetyReportIdentification.senderPersonGivenName", "defaultPerson.givenName"),)),
        TransferSpec("SenderPresaveResponsiblePerson", "person_middle_name", "SenderInformation", "person_middle_name", (transfer_call("safetyReportIdentification.senderPersonMiddleName", "defaultPerson.middleName"),)),
        TransferSpec("SenderPresaveResponsiblePerson", "person_family_name", "SenderInformation", "person_family_name", (transfer_call("safetyReportIdentification.senderPersonFamilyName", "defaultPerson.familyName"),)),
        TransferSpec("SenderPresaveGateway", "sender_identifier", "MessageHeader", "message_sender_identifier", (transfer_call("messageHeader.messageSenderIdentifier", "routingGateway.senderId"),)),
    ),
    "receiver": (
        TransferSpec("ReceiverPresave", "organization_name", "ReceiverInformation", "organization_name", (transfer_call("safetyReportIdentification.receiverOrganization", "d.receiverOrganization"),)),
        TransferSpec("ReceiverPresave", "receiver_type", "ReceiverInformation", "receiver_type", (value_assignment("receiverType", "receiverTypeCode(d.receiverType)"), transfer_call("safetyReportIdentification.receiverType", "receiverType"))),
        TransferSpec("ReceiverPresaveRoute", "batch_receiver_identifier", "MessageHeader", "batch_receiver_identifier", (transfer_call("messageHeader.batchReceiverIdentifier", "route.batchReceiverIdentifier"),)),
        TransferSpec("ReceiverPresaveRoute", "message_receiver_identifier", "MessageHeader", "message_receiver_identifier", (transfer_call("messageHeader.messageReceiverIdentifier", "route.messageReceiverIdentifier"),)),
    ),
    "product": (
        TransferSpec("ProductPresave", "medicinal_product", "DrugInformation", "medicinal_product", (value_assignment("medicinalProduct", "d.medicinalProduct"),)),
        TransferSpec("ProductPresave", "mpid_version", "DrugInformation", "mpid_version", (value_assignment("mpidVersion", "d.mpidVersion"),)),
        TransferSpec("ProductPresave", "mpid", "DrugInformation", "mpid", (value_assignment("mpid", "d.mpid"),)),
        TransferSpec("ProductPresave", "mfds_mpid_version", "DrugInformation", "mfds_mpid_version", (value_assignment("mfdsMpidVersion", "d.mfdsMpidVersion"),)),
        TransferSpec("ProductPresave", "mfds_mpid", "DrugInformation", "mfds_mpid", (value_assignment("mfdsMpid", "d.mfdsMpid"),)),
        TransferSpec("ProductPresave", "phpid_version", "DrugInformation", "phpid_version", (value_assignment("phpidVersion", "d.phpidVersion"),)),
        TransferSpec("ProductPresave", "phpid", "DrugInformation", "phpid", (value_assignment("phpid", "d.phpid"),)),
        TransferSpec("ProductPresave", "obtain_drug_country", "DrugInformation", "obtain_drug_country", (value_assignment("obtainDrugCountry", "d.obtainDrugCountry"),)),
        TransferSpec("ProductPresave", "investigational_product_blinded", "DrugInformation", "investigational_product_blinded", (value_assignment("investigationalProductBlinded", "d.investigationalProductBlinded"),)),
        TransferSpec("ProductPresave", "drug_authorization_number", "DrugInformation", "drug_authorization_number", (value_assignment("drugAuthorizationNumber", "d.drugAuthorizationNumber"),)),
        TransferSpec("ProductPresave", "drug_authorization_country", "DrugInformation", "manufacturer_country", (value_assignment("drugAuthorizationCountry", "d.drugAuthorizationCountry"),)),
        TransferSpec("ProductPresave", "drug_authorization_holder", "DrugInformation", "manufacturer_name", (value_assignment("drugAuthorizationHolder", "d.drugAuthorizationHolder"),)),
        TransferSpec("ProductPresaveActiveSubstance", "substance_name", "DrugActiveSubstance", "substance_name", (value_assignment("substanceName", "s.substanceName"),)),
        TransferSpec("ProductPresaveActiveSubstance", "substance_termid_version", "DrugActiveSubstance", "substance_termid_version", (value_assignment("substanceTermIdVersion", "s.substanceTermIdVersion"),)),
        TransferSpec("ProductPresaveActiveSubstance", "substance_termid", "DrugActiveSubstance", "substance_termid", (value_assignment("substanceTermId", "s.substanceTermId"),)),
        TransferSpec("ProductPresaveActiveSubstance", "mfds_version", "DrugActiveSubstance", "mfds_version", (value_assignment("mfdsVersion", "s.mfdsVersion"),)),
        TransferSpec("ProductPresaveActiveSubstance", "mfds_id", "DrugActiveSubstance", "mfds_id", (value_assignment("mfdsId", "s.mfdsId"),)),
        TransferSpec("ProductPresaveActiveSubstance", "strength_value", "DrugActiveSubstance", "strength_value", (value_assignment("substanceStrengthValue", "s.substanceStrengthValue"),)),
        TransferSpec("ProductPresaveActiveSubstance", "strength_unit", "DrugActiveSubstance", "strength_unit", (value_assignment("substanceStrengthUnit", "s.substanceStrengthUnit"),)),
    ),
    "study": (
        TransferSpec("StudyPresave", "study_name", "StudyInformation", "study_name", (transfer_call("studyInformation.studyName", "d.studyName"),)),
        TransferSpec("StudyPresave", "sponsor_study_number", "StudyInformation", "sponsor_study_number", (transfer_call("studyInformation.sponsorStudyNumber", "d.sponsorStudyNumber"),)),
        TransferSpec("StudyPresave", "study_type_reaction", "StudyInformation", "study_type_reaction", (transfer_call("studyInformation.studyTypeReaction", "d.studyTypeReaction"),)),
        TransferSpec("StudyPresave", "fda_ind_number_occurred", "StudyInformation", "fda_ind_number_occurred", (transfer_call("studyInformation.fdaIndNumberOccurred", "d.fdaIndNumberOccurred"),)),
        TransferSpec("StudyPresave", "fda_pre_anda_number_occurred", "StudyInformation", "fda_pre_anda_number_occurred", (transfer_call("studyInformation.fdaPreAndaNumberOccurred", "d.fdaPreAndaNumberOccurred"),)),
        TransferSpec("StudyPresaveRegistrationNumber", "registration_number", "StudyRegistrationNumber", "registration_number", (r"d\.studyRegistrationNumbers\.filter", value_assignment("registrationNumber", "registration.registrationNumber"), transfer_call("studyInformation.studyRegistrationNumbers.${index}.registrationNumber", "registration.registrationNumber"))),
        TransferSpec("StudyPresaveRegistrationNumber", "country_code", "StudyRegistrationNumber", "country_code", (r"d\.studyRegistrationNumbers\.filter", value_assignment("countryCode", "registration.countryCode"), transfer_call("studyInformation.studyRegistrationNumbers.${index}.countryCode", "registration.countryCode"))),
        TransferSpec("StudyPresaveFdaCrossReportedIndNumber", "ind_number", "StudyFdaCrossReportedInd", "ind_number", (r"d\.fdaCrossReportedIndNumbers\.filter", value_assignment("indNumber", "item.indNumber"), transfer_call("studyInformation.fdaCrossReportedIndNumbers.${index}.indNumber", "item.indNumber"))),
    ),
    "narrative": (
        TransferSpec("NarrativePresave", "case_narrative", "NarrativeInformation", "case_narrative", (transfer_call("narrative.caseNarrative", "d.caseNarrative"),)),
        TransferSpec("NarrativePresave", "additional_information", "NarrativeInformation", "additional_information", (transfer_call("narrative.additionalInformation", "d.additionalInformation"),)),
    ),
}


REPORTER_FRONTEND_FILES = (
    "../frontend/E2BR3-frontend/components/presave/ReporterForm.tsx",
    "../frontend/E2BR3-frontend/lib/types/presave.ts",
)
REPORTER_BACKEND_MODELS = {
    "ReporterPresave": "crates/libs/lib-core/src/model/presave.rs",
}
TECHNICAL_FIELDS = {
    "id",
    "organization_id",
    "deleted",
    "created_at",
    "updated_at",
    "created_by",
    "updated_by",
}
REPORTER_FRONTEND_TO_BACKEND = {
    "reporterTitle": "reporter_title",
    "reporterGivenName": "reporter_given_name",
    "reporterMiddleName": "reporter_middle_name",
    "reporterFamilyName": "reporter_family_name",
    "reporterOrganization": "organization",
    "reporterDepartment": "department",
    "reporterStreet": "street",
    "reporterCity": "city",
    "reporterState": "state",
    "reporterPostcode": "postcode",
    "reporterTelephone": "telephone",
    "reporterCountry": "country_code",
    "qualification": "qualification",
    "qualificationKr1": "qualification_kr1",
    "primarySourceForRegulatoryPurposes": "primary_source_regulatory",
    "reporterTitleNullFlavor": "reporter_title_null_flavor",
    "reporterGivenNameNullFlavor": "reporter_given_name_null_flavor",
    "reporterMiddleNameNullFlavor": "reporter_middle_name_null_flavor",
    "reporterFamilyNameNullFlavor": "reporter_family_name_null_flavor",
    "reporterOrganizationNullFlavor": "organization_null_flavor",
    "reporterDepartmentNullFlavor": "department_null_flavor",
    "reporterStreetNullFlavor": "street_null_flavor",
    "reporterCityNullFlavor": "city_null_flavor",
    "reporterStateNullFlavor": "state_null_flavor",
    "reporterPostcodeNullFlavor": "postcode_null_flavor",
    "reporterTelephoneNullFlavor": "telephone_null_flavor",
    "reporterCountryNullFlavor": "country_code_null_flavor",
    "qualificationNullFlavor": "qualification_null_flavor",
}
PRIMARY_SOURCE_FRONTEND_TO_BACKEND = dict(REPORTER_FRONTEND_TO_BACKEND)
REPORTER_TRANSFER_FILE = (
    "../frontend/E2BR3-frontend/app/(protected)/[authority]/case/[id]/detail/"
    "RP/model/rpModel.ts"
)


def resolve_frontend_path(root: Path, relative: str) -> Path:
    explicit_root = os.environ.get("E2BR3_FRONTEND_ROOT")
    if explicit_root:
        return Path(explicit_root) / relative
    repo_root = root if (root / "registry").exists() else root.parent
    candidates = (
        repo_root.parent / "frontend" / "E2BR3-frontend" / relative,
        repo_root / "frontend" / "E2BR3-frontend" / relative,
    )
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    return candidates[-1]


def extract_rust_presave_source(source: str, model: str) -> set[str]:
    return {
        f"{model}.{field}"
        for field in validate.extract_rust_struct_fields(source, model)
        if field not in TECHNICAL_FIELDS
    }


def extract_presave_frontend_source(source: str, section: str) -> set[str]:
    names = set(
        re.findall(
            r'(?:register\(|name\s*=\s*)[^\n]*?["\'`]([A-Za-z][A-Za-z0-9.]*)',
            source,
        )
    )
    names.update(
        re.findall(r"^\s{2}([A-Za-z][A-Za-z0-9]+)\??:\s", source, re.MULTILINE)
    )
    return {f"{section}.{name}" for name in names if name not in {"id", "deleted"}}


def _reporter_type_source(source: str) -> str:
    match = re.search(
        r"export interface ReporterPresaveData\s*\{(?P<body>.*?)^\}",
        source,
        re.MULTILINE | re.DOTALL,
    )
    if not match:
        raise validate.InventoryError("ReporterPresaveData interface not found")
    return match.group("body")


def _interface_source(source: str, interface_name: str) -> str:
    match = re.search(
        rf"export interface {re.escape(interface_name)}\s*\{{(?P<body>.*?)^\}}",
        source,
        re.MULTILINE | re.DOTALL,
    )
    if not match:
        raise validate.InventoryError(f"{interface_name} interface not found")
    return match.group("body")


def extract_presave_frontend(root: Path, section: str) -> set[str]:
    config = PRESAVE_SECTIONS[section]
    form_path = resolve_frontend_path(root, config.form_file)
    type_path = resolve_frontend_path(root, PRESAVE_TYPE_FILE)
    for path in (form_path, type_path):
        if not path.is_file():
            raise validate.InventoryError(f"presave frontend source not found: {path}")
    type_source = _interface_source(
        type_path.read_text(encoding="utf-8"), config.interface_name
    )
    type_fields = extract_presave_frontend_source(type_source, section)
    form_source = form_path.read_text(encoding="utf-8")
    form_fields = extract_presave_frontend_source(form_source, section)
    fields = type_fields & (form_fields | type_fields)
    for container, child_fields in NESTED_FRONTEND_FIELDS.get(section, {}).items():
        for child_field in child_fields:
            type_has_field = re.search(
                rf"\b{re.escape(child_field)}\??:\s", type_source
            )
            form_path_fragment = f"{container}.${{index}}.{child_field}"
            if type_has_field and form_path_fragment in form_source:
                fields.add(f"{section}.{container}[].{child_field}")
    return fields


def extract_reporter_frontend(root: Path) -> set[str]:
    return extract_presave_frontend(root, "reporter")


def extract_presave_backend(root: Path, models: dict[str, str]) -> set[str]:
    repo_root = root if (root / "registry").exists() else root.parent
    fields: set[str] = set()
    for model, relative in models.items():
        path = repo_root / relative
        if not path.is_file():
            raise validate.InventoryError(f"presave backend source not found: {path}")
        fields.update(
            extract_rust_presave_source(path.read_text(encoding="utf-8"), model)
        )
    return fields


def extract_section_backend(root: Path, section: str) -> set[str]:
    config = PRESAVE_SECTIONS[section]
    return extract_presave_backend(
        root, {model: PRESAVE_MODEL_FILE for model in config.backend_models}
    )


def extract_reporter_transfer_source(source: str) -> set[tuple[str, str]]:
    pairs: set[tuple[str, str]] = set()
    for match in re.finditer(
        r"(?P<target>[A-Za-z][A-Za-z0-9]*)\s*:\s*[^,\n]*?"
        r"data\.(?P<source>[A-Za-z][A-Za-z0-9]*)",
        source,
    ):
        source_field = REPORTER_FRONTEND_TO_BACKEND.get(match.group("source"))
        target_field = PRIMARY_SOURCE_FRONTEND_TO_BACKEND.get(match.group("target"))
        if source_field and target_field:
            pairs.add(
                (
                    f"ReporterPresave.{source_field}",
                    f"PrimarySource.{target_field}",
                )
            )
    if "normalizePrimarySourceValue" in source and "primarySourceForRegulatoryPurposes" in source:
        pairs.add(
            (
                "ReporterPresave.primary_source_regulatory",
                "PrimarySource.primary_source_regulatory",
            )
        )
    return pairs


def extract_reporter_transfers(root: Path) -> set[tuple[str, str]]:
    path = resolve_frontend_path(
        root, PRESAVE_SECTIONS["reporter"].transfer_files[0]
    )
    if not path.is_file():
        raise validate.InventoryError(f"reporter transfer source not found: {path}")
    return extract_reporter_transfer_source(path.read_text(encoding="utf-8"))


def transfer_spec_matches(source: str, spec: TransferSpec) -> bool:
    uncommented = re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.DOTALL)
    return all(re.search(pattern, uncommented, flags=re.DOTALL) for pattern in spec.patterns)


def extract_presave_transfers(root: Path, section: str) -> set[tuple[str, str]]:
    if section == "reporter":
        return extract_reporter_transfers(root)
    source = "\n".join(
        path.read_text(encoding="utf-8")
        for relative in PRESAVE_SECTIONS[section].transfer_files
        for path in (resolve_frontend_path(root, relative),)
        if path.is_file()
    )
    return {
        (
            f"{spec.source_model}.{spec.source_field}",
            f"{spec.target_model}.{spec.target_field}",
        )
        for spec in TRANSFER_SPECS.get(section, ())
        if transfer_spec_matches(source, spec)
    }
