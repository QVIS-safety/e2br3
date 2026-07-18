# G.k.4.r.8 Dosage Text Validation Consistency Design

## Goal

Keep the removed drug-level supplemental dosage text absent while making the official repeated `G.k.4.r.8` field consistently enforce its ICH maximum length and surface backend errors on the rendered dosage row.

## Regulatory Contract

- `G.k.4.r.8` belongs only to repeated `DosageInformation` rows.
- The field is optional.
- A present value may contain at most 2,000 characters.
- The removed `DrugInformation.dosage_text` / `drugDosageText` field must not return.

## Chosen Approach

Use a narrow validation bridge for this field instead of enabling every generated portable catalog rule in the frontend create gate at once.

1. Extend the existing nested drug syntax collector to validate `drugs[].dosageInformation[].dosageText` against the generated `ICH.G.k.4.r.8.LENGTH.MAX` rule, preserving the generated catalog as the source of the 2,000-character boundary.
2. Extend backend path aliases so `drugs.N.dosages.M.*` resolves to `drugs.N.dosageInformation.M.*`. This fixes the dosage-text banner and the same structural mismatch for other dosage-row backend issues.
3. Do not add validation to the deleted drug-level field, change XML behavior, or change persistence schemas.

## Alternatives Considered

### Enable all portable catalog bindings in `collectSyntaxIssues`

This is architecturally attractive but would activate hundreds of rules in the create gate at once, potentially changing unrelated sections. It is outside this focused fix.

### Hard-code `dosageText.length > 2000`

This is the smallest code change but duplicates the regulatory limit outside the generated catalog. The chosen design reuses the existing catalog rule evaluator instead.

## Data and Error Flow

1. The case editor collects a repeated dosage row.
2. Frontend syntax validation evaluates its `dosageText` value with `ICH.G.k.4.r.8.LENGTH.MAX`.
3. A 2,000-character value passes; a 2,001-character value produces the concrete path `drugs.N.dosageInformation.M.dosageText`.
4. If backend semantic validation reports `drugs.N.dosages.M.dosageText`, the alias layer converts it to the same rendered frontend path.

## Tests

- Frontend syntax regression: 2,000 characters pass and 2,001 characters fail at the concrete repeated-row path.
- Backend field-banner regression: a `drugs.0.dosages.0.dosageText` issue resolves to `drugs.0.dosageInformation.0.dosageText`.
- Existing generated-catalog exhaustive tests continue to cover the exact rule boundary.
- Existing backend nested G validation test continues to cover `DosageInformation.dosage_text`.

## Out of Scope

- Activating all generated portable rules in the frontend create gate.
- Adding REST mutation-time validation to every repeated subresource.
- Database constraints or migrations.
- XML import/export or CIOMS behavior changes.
