# Validator rule tests

The agreed 298 baseline business rules each have a named Rust test under
`src/case/sections/{c,d,e,f,g,h}/rule_tests.rs`. Each test runs the production
validator against invalid and repaired inputs, then checks the issue code,
all declared issue paths and removal of the error after repair. The shared
assertion in `src/tests/rule_assertions.rs` reads `rules.json` directly; it does
not rerun the rule or capture results through another test layer.

`field-rules/*.json` retains field-to-rule metadata. The catalog checker checks
those mappings against `rules.json`; they do not generate or execute separate
field tests. Numeric row indices normalize to `[]` when comparing issue paths.
These are deterministic, isolated logic tests, not production data or regulatory
acceptance evidence.

Test files are organized by ownership:

```text
src/tests/                         # Policy/direct-issue tests and rule assertions
src/case/sections/
  tests.rs                         # Cross-section validation tests
  helpers/tests.rs                 # Shared helper tests
  {c,d,e,f,g,h}/
    *tests.rs                      # Existing section regression tests
    rule_tests.rs                  # Named rule tests
    rule_fixtures/                 # C/D/G inputs and production validator calls
tests/allowed_value_representation.rs  # Public API integration test
rules.json                         # Rule-to-test and expected-issue bindings
field-rules/                       # Field-to-rule JSON metadata
tools/                             # Catalog and execution checks
```

C, D and G include their test setup and validation calls from
`rule_fixtures/{ich,fda,mfds}.rs`. D and G share fixture constructors through
`rule_fixtures/common.rs`. Section and policy regression tests use private
`#[cfg(test)]` modules in separate files. These files preserve the existing
Rust test names and do not expose production APIs.

- `rules.json` binds baseline rule codes to Rust modules/test names and current
  issue codes/paths. Twelve historical MedDRA codes explicitly map to the current
  shared `ICH.MEDDRA.VERSION.UNAVAILABLE` rule.
- `field-rules/*.json` **fields** owns field-path-to-rule metadata.
  **editorMappings** preserves UI metadata for canonical inventory consumers;
  its historical statuses are not test results.
- `tools/rule_inventory.py` discovers the agreed production-source baseline.

```sh
python3 -m unittest discover crates/libs/validator/tools/tests
# All named business-rule tests:
cargo test -p validator '::rule_tests::rule_'
# One business rule:
cargo test -p validator case::sections::c::rule_tests::rule_ich_c_5_4_required -- --exact
# Full CI gate:
set -o pipefail
cargo test -p validator --locked -- --color never | tee /tmp/validator-tests.log
python3 crates/libs/validator/tools/check_rule_tests.py --test-log /tmp/validator-tests.log
```

When changing a rule or field mapping, update its production implementation,
rule test, `rules.json` and the corresponding `field-rules` entry. CI checks
source/catalog/field identity consistency and requires every fully qualified
rule test to execute and pass exactly once. Missing, ignored, failed, duplicate,
extra or wrongly located rule tests fail the gate. Each rule test itself checks
the declared issue paths. The baseline is not an official-document completeness
claim.

Business-rule assertions belong to this crate. The web-server integration suite
keeps authority routing, DB projection-to-issue paths, cache invalidation and
counts, authentication and submission gates. It does not repeat the individual
rule fixtures. Editor persistence tests remain separate from validation tests;
the RP collection/row and SI sequence-gap checks live in `case_validation_web.rs`.
Frontend integration tests cover scoped banners/dots, per-authority aggregation
and stale validation responses, and run in the registry CI job.

CI selects tests on `dev`/`main` pushes and PRs targeting either branch:

| Changed files | Validator tests | Other backend/registry tests |
| --- | --- | --- |
| Only `crates/libs/validator/**` | Run | Skip |
| Unrelated backend files | Skip | Run |
| Both areas or shared validator inputs | Run | Run |

`scripts/ci_test_scope.py` lists shared inputs: transitive Rust dependencies,
Cargo/toolchain configuration, the CI workflow/selector and registry metadata.
Pushes compare the complete before/after range; PRs compare from the merge base.
Deletions and both sides of renames are
included. A new branch checks its complete tree; missing Git history fails the
selection job instead of skipping tests. No changed files selects no test suite.

Formatting and workspace Clippy checks still run, including compilation of
downstream validator consumers. The validator tests require no DB credentials.
The other workspace tests still exclude validator. Dev image publication requires
the selected suites to pass and the unselected suites to be explicitly skipped;
a failed, cancelled or unexpectedly skipped required suite blocks publication.
The existing main-branch CD still requires a successful CI workflow.

The shared Test Scope job also preserves `scripts/ci_test_plan.py`: when backend
tests are selected, test-only PRs may run complete affected integration targets;
backend pushes run the full remaining workspace. Validator selection remains
independent of that backend target plan.
