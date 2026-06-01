# Presave Reference Matrix Schema

Each canonical matrix must contain a markdown table with a `category` column.

Preferred columns:

| column | meaning |
|---|---|
| `field` | Reference, local frontend, backend, table, route, or import target field being classified. |
| `reference evidence` | UI, route, bundle, payload, or workflow evidence used for the decision. |
| `local frontend` | Frontend type, form, mapper, hook, or section import surface. |
| `local backend/BMC` | Backend model, DTO, BMC, schema, REST route, or child table surface. |
| `case target` | Target case section path or routing target populated by import. |
| `category` | Exactly one allowed category. |
| `action` | Required local behavior. |

Allowed categories:

- `referenceImportedToCase`: reference imports the presave value into the case section or submission routing target.
- `referencePreserveOnly`: reference stores or shows it on presave, but does not import it into the target case section.
- `localSystemOnly`: local identity, metadata, row IDs, audit fields, or persistence mechanics needed by the app.
- `removed`: local drift, fake field, legacy alias, unsupported table, or route that must not remain in presave surfaces.

Each matrix must end with a coverage section that states there are zero uncategorized fields and zero ambiguous fields.

Do not use `unknown`, `maybe`, `TBD`, `fallback`, or blank categories in canonical matrices.
