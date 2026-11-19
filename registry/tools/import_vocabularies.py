#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Iterable
from xml.etree import ElementTree


ROOT = Path(__file__).resolve().parents[1]
VOCABULARY_DIR = ROOT / "vocabularies"
SNAPSHOT_FIELDS = {
    "name",
    "version",
    "source",
    "source_sha256",
    "license",
    "entries",
}
VALID_SCOPES = {
    "prefix",
    "unit",
    "all",
    "time",
    "gestation",
    "dose",
    "frequency",
    "dose_form",
    "route",
}
ICH_FREQUENCY_CODES = {
    "a",
    "mo",
    "wk",
    "d",
    "h",
    "min",
    "{cyclical}",
    "{asnecessary}",
    "{total}",
}


def source_sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def normalized_entries(
    rows: Iterable[tuple[str, Iterable[str]]],
) -> list[dict[str, Any]]:
    merged: dict[str, set[str]] = {}
    for raw_code, scopes in rows:
        code = raw_code.strip()
        if code:
            merged.setdefault(code, set()).update(scopes)
    return [
        {"code": code, "scopes": sorted(scopes)}
        for code, scopes in sorted(merged.items())
    ]


def normalize_ucum(raw: bytes) -> dict[str, Any]:
    root = ElementTree.fromstring(raw)
    version = root.attrib.get("version")
    if not version:
        raise ValueError("UCUM source is missing root version")

    rows: list[tuple[str, list[str]]] = []
    for element in root:
        local_name = element.tag.rsplit("}", 1)[-1]
        if local_name == "prefix":
            rows.append((element.attrib.get("Code", ""), ["prefix"]))
        elif local_name in {"base-unit", "unit"}:
            rows.append((element.attrib.get("Code", ""), ["unit"]))

    rows.extend((code, ["frequency"]) for code in ICH_FREQUENCY_CODES)

    return {
        "name": "ICH constrained UCUM",
        "version": version,
        "source": f"https://raw.githubusercontent.com/ucum-org/ucum/v{version}/ucum-essence.xml",
        "source_sha256": source_sha256(raw),
        "license": "https://ucum.org/license",
        "entries": normalized_entries(rows),
    }


def normalize_iso639(raw: bytes) -> dict[str, Any]:
    rows: list[tuple[str, list[str]]] = []
    for line in raw.decode("utf-8-sig").splitlines():
        columns = line.split("|")
        if len(columns) != 5:
            raise ValueError(f"invalid ISO 639-2 row: {line!r}")
        bibliographic, terminologic = columns[:2]
        rows.append((bibliographic, ["all"]))
        if terminologic:
            rows.append((terminologic, ["all"]))

    return {
        "name": "ISO639-2",
        "version": "continuously-maintained",
        "source": "https://www.loc.gov/standards/iso639-2/ISO-639-2_utf-8.txt",
        "source_sha256": source_sha256(raw),
        "license": "https://www.loc.gov/standards/iso639-2/",
        "entries": normalized_entries(rows),
    }


def normalize_ich_units(raw: bytes) -> dict[str, Any]:
    root = ElementTree.fromstring(raw)
    name = root.findtext("Identification/ShortName")
    version = root.findtext("Identification/Version")
    if name not in {"ich-dose-strength-unit", "ich-interval-unit"} or not version:
        raise ValueError("expected versioned ICH CL25 or CL26 genericode source")
    rows = []
    seen = set()
    for row in root.findall("SimpleCodeList/Row"):
        values = {v.attrib["ColumnRef"]: v.findtext("SimpleValue") for v in row.findall("Value")}
        code = values.get("code")
        if not code or code in seen or values.get("cf") not in {"y", "n"}:
            raise ValueError("ICH unit requires unique code and current flag")
        seen.add(code)
        if name == "ich-dose-strength-unit":
            if values.get("dosef") not in {"y", "n"}:
                raise ValueError("ICH CL25 requires dose applicability flag")
            scopes = ["dose"] if values["dosef"] == "y" else []
        else:
            # Duration fields require elapsed time, not dosing-frequency annotations.
            scopes = ["time"] if code in {"s", "min", "h", "d", "wk", "mo", "a", "10.a"} else []
        if values["cf"] == "y" and scopes:
            rows.append((code, scopes))
    result = {
        "name": "ICH-UCUM",
        "version": version,
        "source": "https://admin.ich.org/sites/default/files/inline-files/IG_Complete_Package_v1_11_1.zip",
        "source_sha256": source_sha256(raw),
        "license": "ICH E2B(R3) code lists; https://www.ich.org/page/terms-conditions",
        "entries": normalized_entries(rows),
    }
    validate_snapshot(result)
    return result


def normalize_edqm(raw: bytes) -> dict[str, Any]:
    payload = json.loads(raw)
    release = payload.get("release")
    terms = payload.get("terms")
    if not isinstance(release, str) or not release:
        raise ValueError("EDQM export is missing release")
    if not isinstance(terms, list):
        raise ValueError("EDQM export is missing terms")

    rows = []
    for term in terms:
        if term.get("status") != "CURRENT":
            continue
        domain = term.get("domain")
        if domain not in {"dose_form", "route"}:
            continue
        rows.append((str(term.get("code", "")), [domain]))

    return {
        "name": "EDQM",
        "version": release,
        "source": "https://standardterms.edqm.eu/",
        "source_sha256": source_sha256(raw),
        "license": "EDQM Standard Terms authenticated export; redistribution subject to EDQM terms",
        "entries": normalized_entries(rows),
    }


def snapshot_bytes(snapshot: dict[str, Any]) -> bytes:
    return (
        json.dumps(snapshot, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")


def validate_snapshot(snapshot: dict[str, Any]) -> None:
    if set(snapshot) != SNAPSHOT_FIELDS:
        raise ValueError("snapshot fields do not match vocabulary schema")
    for field in ("name", "version", "source", "license"):
        if not isinstance(snapshot[field], str) or not snapshot[field]:
            raise ValueError(f"snapshot {field} must be a non-empty string")
    digest = snapshot["source_sha256"]
    if not isinstance(digest, str) or len(digest) != 64 or any(
        char not in "0123456789abcdef" for char in digest
    ):
        raise ValueError("snapshot source_sha256 must be lowercase SHA-256")

    entries = snapshot["entries"]
    if not isinstance(entries, list) or not entries:
        raise ValueError("snapshot entries must be non-empty")
    codes = [entry.get("code") for entry in entries]
    if codes != sorted(set(codes)):
        raise ValueError("snapshot entries must use sorted unique codes")
    for entry in entries:
        if set(entry) != {"code", "scopes"}:
            raise ValueError("snapshot entry fields must be code and scopes")
        scopes = entry["scopes"]
        if scopes != sorted(set(scopes)) or not scopes or not set(scopes) <= VALID_SCOPES:
            raise ValueError("snapshot scopes must be sorted, unique, and known")


def write_snapshot(path: Path, snapshot: dict[str, Any]) -> None:
    validate_snapshot(snapshot)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(snapshot_bytes(snapshot))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ucum-source", type=Path)
    parser.add_argument("--iso639-source", type=Path)
    parser.add_argument("--edqm-source", type=Path)
    parser.add_argument("--ich-dose-source", type=Path)
    parser.add_argument("--ich-interval-source", type=Path)
    args = parser.parse_args()
    if not any((args.ucum_source, args.iso639_source, args.edqm_source, args.ich_dose_source, args.ich_interval_source)):
        parser.error("at least one source argument is required")

    jobs = (
        (args.ucum_source, "ucum.json", normalize_ucum),
        (args.iso639_source, "iso639-2.json", normalize_iso639),
        (args.edqm_source, "edqm.json", normalize_edqm),
        (args.ich_dose_source, "ich-dose.json", normalize_ich_units),
        (args.ich_interval_source, "ich-time.json", normalize_ich_units),
    )
    for source, output_name, normalize in jobs:
        if source is not None:
            write_snapshot(VOCABULARY_DIR / output_name, normalize(source.read_bytes()))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
