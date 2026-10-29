"""Business-rule inventory discovery owned by the validator crate."""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
RULE_RE = re.compile(r'"((?:ICH|FDA|MFDS)\.[A-Za-z0-9_.-]+)"')
INPUT_ONLY_SUFFIXES = (".LENGTH.MAX", ".ALLOWED.VALUE", ".NULLFLAVOR.ALLOWED")
RETAINED_ALLOWED_VALUE_RULES = {
    "ICH.C.1.6.1.r.2.ALLOWED.VALUE",
    "ICH.C.4.r.2.ALLOWED.VALUE",
    "ICH.D.6.NULLFLAVOR.ALLOWED",
    "ICH.D.7.1.r.1a.ALLOWED.VALUE",
    "ICH.D.10.7.1.r.1a.ALLOWED.VALUE",
    "ICH.E.i.2.1a.ALLOWED.VALUE",
    "ICH.F.r.2.2a.ALLOWED.VALUE",
    "ICH.G.k.7.r.2a.ALLOWED.VALUE",
    "ICH.H.3.r.1a.ALLOWED.VALUE",
}
GENERATED_BUSINESS_RULE_CODES = {
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
    for name in "cdefgh":
        path = section_root / f"{name}.rs"
        production_source = path.read_text().split("\n#[cfg(test)]", 1)[0]
        codes.update(RULE_RE.findall(production_source))
    codes.update(GENERATED_BUSINESS_RULE_CODES)
    return {
        code
        for code in codes
        if not code.endswith((".", ".r"))
        and code != "ICH.E.i.3.2"
        and (
            code in RETAINED_ALLOWED_VALUE_RULES
            or not code.endswith(INPUT_ONLY_SUFFIXES)
        )
    }
