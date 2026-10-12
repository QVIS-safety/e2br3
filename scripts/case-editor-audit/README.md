# Case Edit audit runner

This directory packages the reviewed 348-field persisted-field inventory and
valid alternate-value map used by `scripts/case_editor_input_fuzzer.py`.
CI runs the isolated runner tests and validates that the current frontend
contract still produces a 348-field plan with no blocked or unresolved fields.
It does not send API requests.

Run the same plan locally with a frontend checkout that matches the API version:

```sh
python3 scripts/case_editor_input_fuzzer.py \
  --dry-run \
  --audit-all-fields \
  --contract ../frontend-origin-dev/lib/case-editor/generated/editorContracts.json \
  --null-flavor-pairs ../frontend-origin-dev/lib/case-save/pages/null-flavor-pairs.ts \
  --persisted-field-inventory scripts/case-editor-audit/case-edit-persisted-field-inventory.json \
  --audit-alternate-map scripts/case-editor-audit/valid-alternate-map.json \
  --seed 20261012 \
  --values-per-field 3 \
  --samples-per-category 1 \
  --no-run-gates \
  --no-verify-global-audit-chain
```

The final JSON object must report `persisted_field_denominator`,
`selected_persisted_fields`, and `executable_persisted_fields` as 348, with
empty `blocked_persisted_fields` and `audit_alternate_unresolved` arrays.

The live audit creates and mutates a Case. Use only a dedicated real API and
dedicated test data. Confirm that the frontend contract is from the version
deployed with that API; a repository dry plan does not prove this. Supply real
credentials and identifiers through the environment, then run:

```sh
export E2BR3_BASE_URL='https://dedicated-api.example'
export E2BR3_ADMIN_EMAIL='dedicated-audit-user@example.com'
export E2BR3_ADMIN_PASSWORD='<actual actor password>'
export E2BR3_PRODUCT_KEY='<accessible product ID>'
export E2BR3_MEDDRA_VERSION='<active version>'
export E2BR3_MEDDRA_CODE='<active code>'

python3 scripts/case_editor_input_fuzzer.py \
  --allow-remote \
  --audit-all-fields \
  --contract /path/to/matching-frontend/lib/case-editor/generated/editorContracts.json \
  --null-flavor-pairs /path/to/matching-frontend/lib/case-save/pages/null-flavor-pairs.ts \
  --persisted-field-inventory scripts/case-editor-audit/case-edit-persisted-field-inventory.json \
  --audit-alternate-map scripts/case-editor-audit/valid-alternate-map.json \
  --artifact-dir /path/to/private/audit-artifacts \
  --seed 20261012 \
  --values-per-field 3 \
  --samples-per-category 1 \
  --no-run-gates \
  --no-verify-global-audit-chain
```

Omit `--allow-remote` for a loopback API. The command scopes this campaign to
the Case Edit field checks, so coordinate backend validator tests, frontend
contract checks, and the global audit-chain check separately when their shared
runtime dependencies are available. Missing credentials, input files, or API
data are errors rather than successful skips. Artifacts can contain Case values
and should be stored as private audit evidence.

The alternate values for EDQM fields `G.k.4.r.9.2a` and `G.k.4.r.9.2b` are
structural strings for persistence and audit checks only. They are not EDQM
dictionary codes and do not demonstrate vocabulary membership, regulatory
semantic validity, or availability of production EDQM terminology.

Absolute paths and the loopback URL in the JSON `sources` and
`resolutionEvidence` objects are historical provenance only. The runner uses
the command-line paths and live API settings shown above; it does not read
credentials or runtime locations from those metadata objects.
