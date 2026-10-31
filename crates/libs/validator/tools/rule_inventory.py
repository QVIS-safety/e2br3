"""Business-rule inventory discovery owned by the validator crate."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
RULE_RE = re.compile(r'"((?:ICH|FDA|MFDS)\.[A-Za-z0-9_.-]+)"')
INPUT_ONLY_SUFFIXES = (".LENGTH.MAX", ".ALLOWED.VALUE", ".NULLFLAVOR.ALLOWED")

GENERATED_BUSINESS_RULE_CODES = {
    *(f"FDA.{field}.{kind}" for field in ("C.1.6.1.r.2", "C.4.r.2")
      for kind in ("FILE_NAME.REQUIRED", "MEDIA_TYPE.MATCH")),
    *(f"ICH.E.i.3.2{suffix}.REQUIRED" for suffix in "abcdef"),
    *(
        f"ICH.{section}.r.{suffix}{part}.{kind}"
        for section in ("D.8", "D.10.8")
        for suffix in ("6", "7")
        for part in ("a", "b")
        for kind in ("REQUIRED", "VOCABULARY")
    ),
    *(
        f"ICH.D.9.{group}.r.{field}.{kind}"
        for group in ("2", "4")
        for field, kinds in (
            ("1a", ("REQUIRED", "VOCABULARY")),
            ("1b", ("REQUIRED", "VOCABULARY")),
            ("2", ("REQUIRED",)),
        )
        for kind in kinds
    ),
}


def discover_business_rule_codes(root: Path = ROOT) -> set[str]:
    codes: set[str] = set()
    section_root = root / "crates/libs/validator/src/case/sections"
    common_source = (section_root / "mod.rs").read_text().split("\n#[cfg(test)]", 1)[0]
    retained_source = common_source.split("fn retain_case_business_rules(", 1)[1].split("fn has_multiple_values", 1)[0]
    retained_codes = set(RULE_RE.findall(retained_source))
    collector_source = common_source.split("fn collect_meddra_version_issues(", 1)[1].split("pub(crate) fn normalize_validation_field_path", 1)[0]
    codes.update(RULE_RE.findall(collector_source))
    for name in "cdefgh":
        path = section_root / f"{name}.rs"
        production_source = path.read_text().split("\n#[cfg(test)]", 1)[0]
        codes.update(RULE_RE.findall(production_source))
    codes.update(GENERATED_BUSINESS_RULE_CODES)
    return {
        code
        for code in codes
        if not code.endswith((".", ".r"))
        and code not in {"ICH.E.i.3.2", "ICH.MEDDRA.VERSION.UNAVAILABLE"}
        and (
            code in retained_codes
            or not code.endswith(INPUT_ONLY_SUFFIXES)
        )
    }
