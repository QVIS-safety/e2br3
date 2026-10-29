# Validator field and rule tests

The validation test binding is:

`field-rules/*.json fields -> ruleCodes -> rules.json Rust test binding -> production validator`

Each field has a named Rust test under `src/field_tests/`. It reads its field's
JSON entry at execution time, runs every referenced rule fixture, and checks
that the invalid result reports the expected code at that exact field path and
that repair removes it. Fields are validator issue paths, including collection
paths, not a count of frontend controls. Numeric row indices normalize to `[]`.

The existing 298 baseline rule tests remain under
`src/case/sections/{c,d,e,f,g,h}/rule_tests.rs`. The generated function tables
reuse those deterministic fixtures in field tests. Thread-local test capture
collects the actual invalid/repaired issues; it does not manufacture results.
`catalog_matches_rule_fixtures` also checks the declared issue paths against
actual validator output. These are isolated logic tests, not production data
or regulatory acceptance evidence.

- `field-rules/*.json` **fields** owns the field-path-to-rule bindings consumed
  by Rust field tests. **editorMappings** preserves the previous UI metadata
  for canonical inventory consumers; its historical statuses are not test results.
- `rules.json` binds baseline rule codes to Rust modules/test names and current
  issue codes/paths. It contains no fuzzer scenarios or mutation targets. Twelve
  historical MedDRA codes explicitly map to the current shared
  `ICH.MEDDRA.VERSION.UNAVAILABLE` rule.
- `tools/rule_inventory.py` discovers the agreed production-source baseline.
  The fuzzer may import that inventory; field tests do not depend on the fuzzer.
- `fuzz/runner.py` separately owns real API save/readback/validation/audit checks.
  The old script command forwards to it. Its scenarios do not define field-test
  bindings or stand in for field-test execution.

```sh
python3 crates/libs/validator/tools/generate_field_tests.py --write
python3 -m unittest discover crates/libs/validator/tools/tests
python3 -m unittest discover crates/libs/validator/fuzz/tests/unit
cargo test -p validator field_tests::
# One field and all its connected rules:
cargo test -p validator field_tests::si::field_si_studyinformation_studytypereaction -- --exact
# Full CI gate:
set -o pipefail
cargo test -p validator --locked -- --color never | tee /tmp/validator-tests.log
python3 crates/libs/validator/tools/check_rule_tests.py --test-log /tmp/validator-tests.log
```

When changing a rule or field mapping, update its production implementation,
rule fixture, `rules.json` and the corresponding `field-rules` entry, then run
the generator. CI checks source/catalog/field identity consistency, generated
file freshness, actual issue paths and execution of every rule and field test.
Missing, ignored, failed, duplicate or wrongly connected tests fail the gate.
The baseline is not an official-document completeness claim.

Business-rule assertions belong to this crate. The web-server integration suite
keeps authority routing, DB projection-to-issue paths, cache invalidation and
counts, authentication and submission gates. It does not repeat the individual
rule fixtures. Editor persistence tests remain separate from validation tests;
the RP collection/row and SI sequence-gap checks live in `case_validation_web.rs`.
Frontend integration tests cover scoped banners/dots, per-authority aggregation
and stale validation responses, and run in the registry CI job.

The validator CI job runs on `dev`/`main` pushes and PRs targeting either branch,
without DB credentials. Dev image publication requires both this job and the
workspace tests to pass. The workspace DB job excludes validator to avoid
running the same suite twice.
