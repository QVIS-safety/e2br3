# Terminology Loader (MedDRA / WHODrug)

This tool loads licensed MedDRA and WHODrug dictionary files into SafetyDB.

## Location

- Binary crate: `crates/tools/terminology-loader`
- Command: `cargo run -p terminology-loader -- <subcommand> ...`

## MedDRA load

Input supports:
- Extracted MedDRA folder containing `llt.asc` and `mdhier.asc`
- MedDRA zip containing those files

Dry run:

```bash
cargo run -p terminology-loader -- \
  meddra \
  --input /path/to/MedDRA_27_1_English.zip \
  --version 27.1 \
  --language en \
  --dry-run
```

Load + activate:

```bash
cargo run -p terminology-loader -- \
  meddra \
  --input /path/to/MedDRA_27_1_English.zip \
  --version 27.1 \
  --language en
```

## WHODrug load

Input supports:
- A delimited file (`.csv`, `.tsv`, `.txt`) with a header row
- A folder or zip containing such a file

Required columns (header aliases accepted):
- code: `code`, `drug_code`, `record_id`, `drugid`, `drecno`, `mpid`
- drug name: `drug_name`, `name`, `drugname`, `medicinal_product_name`, `medicinal product name`, `product_name`

Optional column:
- ATC: `atc`, `atc_code`, `atc1`

Dry run:

```bash
cargo run -p terminology-loader -- \
  whodrug \
  --input /path/to/WHODrug_global.csv \
  --version 2025.09 \
  --language en \
  --dry-run
```

Load + activate:

```bash
cargo run -p terminology-loader -- \
  whodrug \
  --input /path/to/WHODrug_global.csv \
  --version 2025.09 \
  --language en
```

## Behavior

- Loads happen in a DB transaction.
- Existing active rows for the same dictionary + language are deactivated.
- New version rows are upserted and marked active.
- Release metadata is recorded in `terminology_releases`.

## Prerequisites

- DB reachable through existing `DB_URL` config (same as web server).
- Schema includes terminology tables and `terminology_releases`.
