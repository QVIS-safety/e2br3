# Web Server Examples

These examples are organized by user flow and all start with login.

## Layout

- `case_creation/`
- `import_export/`
- `common/` shared helper module

## Example flows

1. `case_create_fda_minimal`
   - Login -> create FDA case -> validate -> export
2. `case_create_ich_minimal`
   - Login -> create ICH case -> validate -> export
3. `case_create_fda_validation_batch`
   - Login -> create multiple FDA scratch cases with varied reaction/drug combinations -> validate -> export all
4. `case_create_fda_submission_pack`
   - Login -> create multiple FDA scratch cases (postmarket + premarket variants) -> validate -> export all
5. `import_export_fda_roundtrip`
   - Login -> import FDA XML -> validate -> export
6. `import_export_ich_roundtrip`
   - Login -> import ICH XML -> validate -> export

## Run

From repository root:

```bash
cargo run -p web-server --example case_create_fda_minimal
cargo run -p web-server --example case_create_ich_minimal
cargo run -p web-server --example case_create_fda_validation_batch
cargo run -p web-server --example case_create_fda_submission_pack
cargo run -p web-server --example import_export_fda_roundtrip
cargo run -p web-server --example import_export_ich_roundtrip
```

## Environment variables

- `E2BR3_BASE_URL` default: `http://localhost:8080`
- `E2BR3_EXAMPLE_EMAIL` default: `demo.cro.admin@example.com`
- `E2BR3_EXAMPLE_PWD` default: `welcome`
- `E2BR3_EXAMPLE_ORG_ID` default: `00000000-0000-0000-0000-000000000001`
- `E2BR3_EXAMPLE_OUTPUT_DIR` default: `~/Documents`
- `E2BR3_IMPORT_XML_FDA` path override for FDA import example
- `E2BR3_IMPORT_XML_ICH` path override for ICH import example

## Download behavior note

- Backend endpoint `/api/cases/:id/export/xml` returns XML bytes.
- In frontend, browser handles actual file download.
- In CLI examples, we explicitly write the response to local files.
