#!/usr/bin/env python3
"""Compatibility entry point for the validator-owned business-rule fuzzer."""

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
FUZZ_ROOT = ROOT / "crates" / "libs" / "validator" / "fuzz"
sys.path.insert(0, str(FUZZ_ROOT))

from runner import *  # noqa: F403
from runner import main, parser


if __name__ == "__main__":
    sys.exit(main(parser().parse_args()))
