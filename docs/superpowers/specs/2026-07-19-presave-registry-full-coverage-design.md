# Presave Registry Full Coverage Design

## Goal

Extend strict presave registry coverage from Reporter to Sender, Receiver, Product, Study, and Narrative. A covered section is complete only when its registry rows join the case registry, its frontend and Rust inventories match, and every mapped presave field reaches the correct case destination.

## Delivery Strategy

Work proceeds in dependency order:

1. Sender
2. Receiver
3. Product
4. Study
5. Narrative

Each phase is independently testable and must pass strict inventory validation before the next phase begins. Sender and Receiver precede Product because product presaves reference them. Product precedes Study because study presaves reference products. Narrative has no upstream presave dependency and is last so the shared extraction and validation framework is already stable.

## Registry Structure

`registry/presaves/index.json` will enumerate one section file per presave domain in addition to Reporter. Every row retains the existing registry schema and uses `e2br3_code` as the join key to its case destination.

Rows are unique within the presave namespace. An official or application-local case destination must have a corresponding case registry row. Workflow-only fields without a case value destination are marked `local_only` and `not_applicable`; they do not create a false transfer requirement. A field that is local to E2B but does have a case destination remains transfer-validated.

The registry remains the expected contract. Extraction reads production sources and reports missing, unknown, or wrongly targeted mappings rather than silently generating authoritative rows.

## Inventory Framework

The Reporter-specific extractor will become a configuration-driven multi-section extractor. Each section configuration declares:

- Frontend form and TypeScript type sources.
- Rust presave model sources.
- Presave-to-case transfer source files.
- Frontend-to-backend field normalization.
- Case destination model normalization.
- Technical or workflow-only fields excluded from value transfer.

The validation entry point will support both a named section and the complete configured set. Existing Reporter behavior remains unchanged. Error messages include the presave section so the same source or target field name in different domains is unambiguous.

## Section Contracts

### Sender

Sender registry rows cover every persisted `SenderPresave` business field exposed by the sender presave form. Mapped fields join the Sender Information case rows and transfer to the case sender model. Product linkage to a sender presave remains workflow metadata and is validated in the Product phase.

### Receiver

Receiver registry rows cover every persisted `ReceiverPresave` business field exposed by the receiver form. Mapped fields join Receiver Information case rows and transfer to the receiver case model. Product linkage to a receiver presave remains workflow metadata and is validated in the Product phase.

### Product

Product registry rows cover product identity and applicable regional product fields that can populate a case drug. Sender/receiver presave identifiers and organization ownership are workflow fields. The transfer inventory verifies only fields with an explicit Drug Information destination and detects source-family fallback into an incorrect effective field.

### Study

Study registry rows cover study identification, product associations, reporter associations, registrations, and regional study fields. Relationship identifiers are classified separately from case value fields. Transfer validation includes repeatable products, reporters, registrations, and FDA cross-reported IND values where the current case importer supports them.

### Narrative

Narrative registry rows cover case narrative text, reporter comments, sender diagnosis, sender comments, and other persisted narrative presave values. Each field joins its Narrative Information or sender-diagnosis case destination and transfers without collapsing distinct fields.

## Frontend and Backend Changes

The project already contains forms, TypeScript types, API mappers, Rust models, and transfer paths for all five domains. The implementation first inventories those sources. Production changes are limited to genuine gaps revealed by strict validation: missing mappings, obsolete fields, incorrect targets, or missing transfer assignments. No UI redesign or unrelated model refactor is included.

When a registry/backend/frontend mismatch exposes an ambiguous field meaning, the case destination model and API contract determine the canonical interpretation. Compatibility aliases are not added merely to make inventory pass.

## Validation and Testing

Each phase follows test-driven development:

1. Add a failing registry or extractor test for the uncovered field or transfer.
2. Add the section registry rows and minimal extractor configuration.
3. Run the section's strict inventory and focused frontend/backend tests.
4. Preserve all earlier sections as a regression suite.

Final completion requires:

- Structural validation for all six presave sections, including Reporter.
- Zero missing case registry joins.
- Zero missing or unknown frontend mappings.
- Zero missing or unknown Rust model mappings.
- Zero missing or wrong presave-to-case assignments.
- Existing Reporter strict inventory remains green.
- Backend registry unit tests, focused presave API tests, and affected frontend mapper/form tests pass.
- CI runs the complete strict presave inventory rather than Reporter-only validation.

## Repository and Branching

The registry and Rust validation work lives on backend branch `codex/presave-registry-full-coverage`, based on backend `dev`. Frontend changes, if strict validation reveals any, use a matching frontend branch based on frontend `dev`. Each repository is committed independently and verified together before integration.
