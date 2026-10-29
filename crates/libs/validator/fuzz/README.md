# Persisted business-validation fuzzer

`runner.py` is the existing seeded API fuzzer, now owned by the validator crate.
It verifies invalid/valid mutations through the real save/readback/validation
and audit paths. Shared API/audit helpers remain in repository `scripts/`.
The old script entry point forwards here for existing callers.

```sh
python3 -m unittest discover crates/libs/validator/fuzz/tests/unit
python3 crates/libs/validator/fuzz/runner.py --help
python3 crates/libs/validator/fuzz/runner.py --dry-run
```

Live execution requires an explicitly configured API target, credentials and the
relevant real reference data; see `--help` for the existing options. Missing live
dependencies must be reported, never substituted with unit fixtures.

Fast per-rule CI tests are Rust tests under `../src/case/sections/`. Their
execution gate and field/rule catalog are described in `../README.md`. This API
fuzzer remains a separate persistence check; it does not run in the credential-free
rule-test CI job. MedDRA version scenarios may be rebound to the global active
version rule by the API runner; its actual report is authoritative for that run.
