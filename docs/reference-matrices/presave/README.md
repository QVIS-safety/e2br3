# Presave Reference Matrices

This directory is the canonical source of truth for E2BR3 presave reference alignment.

Historical investigation notes can remain under `docs/superpowers/audits`, but implementation decisions should point here.

## Files

- `sender-c3.md`: Sender Presave -> C.3 / message sender routing.
- `receiver-submission.md`: Receiver Presave -> submission receiver routing.
- `product-dg.md`: Product Presave -> DG.
- `reporter-c2r.md`: Reporter Presave -> C.2.r.
- `study-c5.md`: Study Presave -> C.5.
- `narrative-h.md`: Narrative Presave -> H / NR.
- `index.json`: registry of canonical matrix files and current alignment status.
- `schema.md`: matrix authoring and validation rules.

## Rule

Every presave field must appear in exactly one row with one of these categories:

- `referenceImportedToCase`
- `referencePreserveOnly`
- `localSystemOnly`
- `removed`

Run the validator after editing:

```sh
python3 scripts/validate_presave_reference_matrices.py
```
