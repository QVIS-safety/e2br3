# Cross-organization HTTP fuzzer

Creates **five new test organizations and three users per organization** (one sponsor admin and two custom-role operators), entirely through the real HTTP API. Each run gets a new namespace. Uses the opt-in synthetic MedDRA fixture **28.1 / 99999999**; these are isolated test results, not production or licensed terminology evidence.

## Run

Use the repository's isolated runner with its normal database credentials/configuration. Never use the shared development or production database:

```sh
UI_TEST_TERMINOLOGY_FIXTURE=1 \
SERVICE_BOOTSTRAP_ENABLED=true \
E2BR3_FRONTEND_ROOT=/path/to/e2br3-frontend \
scripts/ui-env.sh bombadil_api_fuzz_example 18961 18962
```

In another terminal, use the PID of that runner's **web-server** process:

```sh
node --test scripts/api-fuzzer/run.test.mjs
node scripts/api-fuzzer/run.mjs \
  --environment bombadil_api_fuzz_example --backend-pid SERVER_PID \
  --seed 20261123 --cases 100 --hops 5
```

Reproduce one indexed case without running the preceding cases:

```sh
node scripts/api-fuzzer/run.mjs \
  --environment bombadil_api_fuzz_example --backend-pid SERVER_PID \
  --seed 20261123 --case-index 13 --hops 5
```

Requires Linux `/proc`, Node 22+, `xmllint`, the existing built backend, PostgreSQL/UI-runner dependencies, bootstrap accounts, and the frontend checkout containing `scripts/bombadil/xml-cross-organization.mjs` and `xml-preservation.mjs`. No npm dependencies are added. The runner verifies the process name, loopback bind address, exact named test database, and explicit fixture flag before making requests. Credentials/cookies are never logged. The bootstrap account is read from the repository bootstrap constants and only used in that verified test environment.

`--cases` and `--hops` default to 100 and 5. Cases rotate through all five source organizations and ICH/FDA/MFDS; field order, users and the next organization are seeded. Each page receives one common ICH contract field and, when applicable, one field for the case authority. Valid values, prerequisites and fixtures come directly from the editor contracts, except local source documents use uniquely named text-file canaries with nonempty base64 content. The default 100 cases schedule every current contract field and the run fails if any XML field does not complete a verified import/export, or the explicitly separated local document/report-type fields do not complete persistence/non-disclosure checks. Each case retains one authority across its hops, so changing formats cannot excuse regional data loss. C.1.2 deliberately advances before each hop to exercise follow-up versions on return visits. Expected target versions come from previously observed cases in the run. Intentional mutations are restored before transferring.

## Checks and evidence

- Independently compare generated inputs against persisted editor-contract projections before exporting. Decimal projections use the contract's explicit fixed scale. `checkedFields` is updated only after the populated values survive the first cross-organization import/export.
- `CI.sourceDocuments` is app-local, not a standard attachment. Verify name, file bytes and MIME persistence, then inspect parsed XML for its unique filename, plaintext/base64 payload (including wrapped base64), and the forbidden `e2br3-source-documents-v1` processing instruction. Record success separately in `checkedLocalOnlyFields`; missing coverage still fails the campaign. These checks do not claim detection of every possible payload transformation.
- Do not require internal source-document rows/IDs to survive XML import; retain their raw differences under `localOnlyDocumentChanges`. A target organization may already have its own local documents on follow-up. Standard `documentsHeldBySender` descriptions, attachment bytes and metadata remain subject to exact preservation checks. No blanket exemption is applied to fields merely named `.local.`.
- Compare all 11 editor graphs plus message headers after each import, retaining array order, duplicates, nulls, empty collections, missing fields, dates, fractions, booleans and Unicode/XML escaping.
- Verify fresh IDs, correct parent references, target organization, target sender link and expected version using the existing cross-organization comparator. Raw differences remain in the artifacts.
- Product-link expectations come from the import request, not the source DB: standard XML intake links only the first drug to the selected target product; other drugs must remain unlinked. Reject exported local product-link processing instructions. FDA child `device_id` references must map to their corresponding target devices, not just any existing ID.
- When patient-initials NullFlavor is selected, omit only the conflicting synthetic `PT-FIXTURE` prerequisite; keep the parent/child rows and the selected NullFlavor. Deliberate field values are not suppressed.
- Reject cross-organization reads; compare reads by different users within the target organization.
- Check repeated export equality and that exporting/importing leaves the source unchanged.
- Reject blank strings, false E.i.3.2 markers, and the redundant `EQ` qualifier without changing the case; save then explicitly clear a narrative field with null and check omitted fields remain intact.
- Continue forwarding successfully imported cases after a value mismatch, while retaining the failure and returning a nonzero exit code. A structural/API failure stops that case; other cases continue.

Artifacts default to `tmp/api-fuzzer/<timestamp>-<seed>/`; `--output` may specify a **new** directory. `report.json` contains counts, phases, `contractFields`, `xmlRoundTripFields`, `checkedFields`, `uncheckedFields`, `localOnlyFields`, `checkedLocalOnlyFields`, `uncheckedLocalOnlyFields`, per-case `localDocumentChecks`, failures, source hashes and organization/user IDs. `requests.jsonl` contains requests/responses with credentials removed. Per-case plans name their target fields; before/after snapshots, exported XML and comparisons make each failure inspectable. A copy of the running script is retained. Reusing a seed reproduces generated choices; the namespace and server-generated UUIDs/timestamps change. Failed runs are never overwritten or marked successful. Test data stays in the named DB for inspection; stop its runner before deliberately resetting it with `scripts/ui-env.sh reset NAME`.

## Scope

The [local-field audit](LOCAL-FIELD-AUDIT.md) distinguishes truly app-only fields from standard XML companions. The reviewed inventory in `local-field-audit.json` drives the runner's local contract coverage and is checked against every local-marked case/presave registry entry. Do not use `local_only` or a `.local.` name as a blanket exemption. In particular, standard nullFlavor, ED attributes, result qualifiers, code systems and official `FDA.C.1.7.1` remain exact.

For each populated app-only scalar, temporarily clear only that field through the real API, verify it is null, compare the export byte-for-byte, and restore it in `finally`. Nested dosage/assessment fields use existing child IDs. Verify all observed source business values and headers after restoration; record the probe XML and `localFieldChecks`. Count it only in local-only persistence/non-disclosure coverage. The cross-organization comparison retains raw changes under `localOnlyFieldChanges`; missing fields and invalid types still fail. Reject `KR_DVC_*` private extensions and local product/dosage processing instructions in outbound XML. The probes create audit entries in the isolated test database.

This is a sequential, stateful integrity fuzzer, not a concurrency or throughput benchmark. The default campaign targets every field in the 11 current editor contracts, including nested arrays, nullFlavor and regional values. It uses each contract's reviewed valid literal rather than inventing unconstrained values; the existing Rust dictionary boundary matrix remains complementary. Product/sender catalog mapping combinations beyond the generated per-organization product, arbitrary malformed XML, concurrency, and all authorization role combinations remain outside scope. Empty or merely scheduled fields are never counted as checked.
