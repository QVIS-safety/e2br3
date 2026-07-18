# Presave Registry Full Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add strict registry, frontend, backend, and presave-to-case transfer coverage for Sender, Receiver, Product, Study, and Narrative while preserving Reporter coverage.

**Architecture:** Replace the Reporter-only inventory functions with section configurations that declare sources, models, field normalization, and transfer extractors. Add one registry JSON file per presave domain and make validation compare each domain independently so failures identify the responsible section.

**Tech Stack:** Python 3 standard library and unittest, JSON registry files, Rust model source extraction, TypeScript/React source extraction, Jest

## Global Constraints

- Delivery order is Sender, Receiver, Product, Study, Narrative.
- Every mapped row joins the case registry by `e2br3_code`.
- Workflow-only fields use `status: not_applicable` and `local_only: true`.
- Application-local fields with real case destinations remain transfer-validated.
- No compatibility aliases are introduced solely to satisfy inventory.
- Reporter strict inventory remains green after every task.

---

### Task 1: Generalize strict presave inventory by section

**Files:**
- Modify: `registry/tools/extract_presave_fields.py`
- Modify: `registry/tools/presave_registry.py`
- Modify: `registry/tools/validate.py`
- Modify: `registry/tools/test_extract_presave_fields.py`
- Modify: `registry/tools/test_presave_registry.py`
- Modify: `registry/tools/test_validate.py`

**Interfaces:**
- Produces: `PRESAVE_SECTIONS: dict[str, PresaveSectionConfig]`, `extract_presave_frontend(root, section)`, `extract_presave_backend(root, section)`, and `extract_presave_transfers(root, section)`.
- Preserves: `extract_reporter_frontend`, `extract_reporter_backend`, and `extract_reporter_transfers` wrappers for current callers.

- [ ] **Step 1: Add failing tests for two independent section namespaces**

```python
def test_registry_groups_rows_by_frontend_section(self):
    loaded = presave_registry.load_presave_registry(root, result)
    self.assertEqual({"sender", "reporter"}, set(loaded.sections))

def test_strict_inventory_labels_section_errors(self):
    result = validate.validate_registry(root, validate_presave_inventory=True)
    self.assertIn("sender: missing presave frontend mapping", "\n".join(result.errors))
```

- [ ] **Step 2: Run tests and verify RED**

Run: `python3 -m unittest registry.tools.test_presave_registry registry.tools.test_extract_presave_fields registry.tools.test_validate.RegistryValidatorTests.test_repository_reporter_presave_inventory_is_complete -v`

Expected: FAIL because the registry and extractor have no section configuration or grouping.

- [ ] **Step 3: Implement the configuration and section-aware comparison**

```python
@dataclass(frozen=True)
class PresaveSectionConfig:
    frontend_section: str
    interface_name: str
    frontend_files: tuple[str, ...]
    backend_models: dict[str, str]
    transfer_files: tuple[str, ...]
    frontend_to_backend: dict[str, str]
    target_frontend_to_backend: dict[str, str]
```

`PresaveRegistry` stores `section_by_code` and groups frontend/backend keys by the row's frontend section. `validate_registry` loops configured sections and prefixes all inventory and transfer errors with `<section>:`.

- [ ] **Step 4: Run the focused Python tests and verify GREEN**

Run: `python3 -m unittest registry.tools.test_presave_registry registry.tools.test_extract_presave_fields registry.tools.test_validate -v`

Expected: all tests pass and Reporter repository inventory remains complete.

- [ ] **Step 5: Commit**

Commit message: `refactor: generalize presave inventory by section`.

### Task 2: Add Sender strict coverage

**Files:**
- Create: `registry/presaves/sections/c-sender.json`
- Modify: `registry/presaves/index.json`
- Modify: `registry/tools/extract_presave_fields.py`
- Modify: `registry/tools/test_validate.py`
- Frontend transfer source: `app/(protected)/[authority]/case/[id]/detail/SD/hooks/useSenderPresaveImport.ts`

**Interfaces:**
- Backend models: `SenderPresave`, `SenderPresaveResponsiblePerson`, `SenderPresaveGateway`.
- Case targets: `SenderInformation` and `MessageHeader`.

- [ ] **Step 1: Add a repository test requiring Sender inventory**

```python
def test_repository_sender_presave_inventory_is_complete(self):
    result = validate.validate_registry(validate_presave_inventory=True)
    self.assertFalse(any(error.startswith("sender:") for error in result.errors), result.errors)
```

- [ ] **Step 2: Run the test and verify RED with missing Sender rows**

Run the single unittest above; expect missing sender frontend/backend mappings.

- [ ] **Step 3: Register Sender mappings**

Map sender type, organization, responsible-person department/name fields, address, country, telephone, fax, and email to `C.3.*`. Map the selected regulator gateway sender identifier to its `MessageHeader` destination where production transfer exists. Mark defaults, sequence/order fields, routing conditions, ownership identifiers, and unsupported notation fields `not_applicable` plus `local_only`.

- [ ] **Step 4: Verify Sender and Reporter together**

Run: `python3 registry/tools/validate.py --strict-presave-inventory`

Expected: no `sender:` or `reporter:` errors.

- [ ] **Step 5: Commit**

Commit message: `feat: register sender presave fields`.

### Task 3: Add Receiver strict coverage

**Files:**
- Create: `registry/presaves/sections/c-receiver.json`
- Modify: `registry/presaves/index.json`
- Modify: `registry/tools/extract_presave_fields.py`
- Modify: `registry/tools/test_validate.py`
- Create frontend when transfer is absent: `app/(protected)/[authority]/case/[id]/detail/SD/hooks/useReceiverPresaveImport.ts`
- Modify frontend when transfer is absent: `app/(protected)/[authority]/case/[id]/detail/SD/SDPage.tsx`

**Interfaces:**
- Backend models: `ReceiverPresave`, `ReceiverPresaveConsignee`, `ReceiverPresaveRoute`.
- Case targets: `ReceiverInformation` and `MessageHeader` receiver identifiers.

- [ ] **Step 1: Add a failing Receiver repository inventory test**

Use the Sender test pattern with the `receiver:` prefix and run it to observe missing mappings.

- [ ] **Step 2: Register Receiver mappings**

Map organization/contact fields to `local.receiver.*` and route batch/message receiver identifiers to `N.1.4` and `N.2.r.3`. Mark transmission day-count policies, conditions, labels, ordering, descriptions, ownership links, and other routing metadata without case value destinations `not_applicable` plus `local_only`.

- [ ] **Step 3: Add a transfer test before any missing production receiver import implementation**

The test imports a Receiver presave route and asserts the two Message Header receiver identifiers and mapped Receiver Information values. Run it RED, implement only the missing assignments, then rerun GREEN.

- [ ] **Step 4: Run complete strict inventory and focused frontend receiver tests**

Expected: Reporter, Sender, and Receiver are green.

- [ ] **Step 5: Commit backend and frontend repositories independently**

Backend commit: `feat: register receiver presave fields`. Frontend commit, if needed: `fix: transfer receiver presave fields to case`.

### Task 4: Add Product strict coverage

**Files:**
- Create: `registry/presaves/sections/g-product.json`
- Modify: `registry/presaves/index.json`
- Modify: `registry/tools/extract_presave_fields.py`
- Modify: `registry/tools/test_validate.py`
- Frontend transfer sources: `app/(protected)/[authority]/case/[id]/detail/DG/components/SectionG.tsx` and `app/(protected)/[authority]/case/[id]/detail/DG/hooks/useSectionGDrugs.ts`.

**Interfaces:**
- Backend models: `ProductPresave`, `ProductPresaveSubstance`.
- Case targets: `DrugInformation`, `DrugActiveSubstance`.

- [ ] **Step 1: Add and run a failing Product inventory test**

Require a clean `product:` section and observe the missing field/transfer set.

- [ ] **Step 2: Register Product mappings**

Map medicinal product, MPID/PHPID variants, MFDS MPID fields, blinded indicator, obtain-drug country, authorization fields with case destinations, and all active-substance terminology/strength fields. Mark sender/receiver links, ownership identifiers, sequence numbers, unsupported notation, and descriptive master-data-only fields without case destinations as workflow-only.

- [ ] **Step 3: Add RED transfer tests for every mapped Product field**

Extend the existing explicit product presave import contract test so its expected field matrix is derived from the Product registry mappings. Fix only missing or wrong assignments, including regional effective-field selection.

- [ ] **Step 4: Run strict inventory and Product import suites**

Expected: Reporter, Sender, Receiver, and Product are green.

- [ ] **Step 5: Commit**

Backend: `feat: register product presave fields`; frontend if needed: `fix: complete product presave case transfer`.

### Task 5: Add Study strict coverage

**Files:**
- Create: `registry/presaves/sections/c-study.json`
- Modify: `registry/presaves/index.json`
- Modify: `registry/tools/extract_presave_fields.py`
- Modify: `registry/tools/test_validate.py`
- Frontend transfer source: `app/(protected)/[authority]/case/[id]/detail/SI/hooks/useStudyImport.ts`

**Interfaces:**
- Backend models: `StudyPresave`, `StudyPresaveRegistrationNumber`, `StudyPresaveFdaCrossReportedInd`, `StudyPresaveProduct`, `StudyPresaveReporter`.
- Case targets: `StudyInformation`, `StudyRegistrationNumber`, `StudyFdaCrossReportedInd`, plus explicit product/reporter destinations supported by case import.

- [ ] **Step 1: Add and run a failing Study inventory test**

Require a clean `study:` section and capture missing repeatable assignments.

- [ ] **Step 2: Register Study mappings**

Map study name, sponsor number, study type, FDA occurrence fields, registrations, and cross-reported INDs. Mark relationship IDs, sync controls, sequence numbers, unsupported notation/kind fields, and display-only denormalized names without case value destinations workflow-only.

- [ ] **Step 3: Add RED frontend tests for registrations and FDA IND repeats**

Assert that every active registered number and FDA IND is copied with order and deletion semantics preserved. Implement missing assignments in `useStudyImport.ts` and rerun GREEN.

- [ ] **Step 4: Run strict inventory and Study import tests**

Expected: the first five configured presave sections plus Reporter are green except Narrative, which is not yet configured.

- [ ] **Step 5: Commit**

Backend: `feat: register study presave fields`; frontend if needed: `fix: complete study presave case transfer`.

### Task 6: Add Narrative strict coverage and enable complete CI gate

**Files:**
- Create: `registry/presaves/sections/h-narrative.json`
- Modify: `registry/presaves/index.json`
- Modify: `registry/tools/extract_presave_fields.py`
- Modify: `registry/tools/test_validate.py`
- Modify: `registry/README.md`
- Modify: `registry/SPEC.md`
- Modify frontend when transfer extraction finds a gap: `app/(protected)/[authority]/case/[id]/detail/NR/NRPage.tsx`

**Interfaces:**
- Backend model: `NarrativePresave`.
- Case target: `NarrativeInformation`.

- [ ] **Step 1: Add and run a failing Narrative inventory test**

Require clean `narrative:` errors and verify missing case narrative/additional information transfers are detected.

- [ ] **Step 2: Register and verify Narrative mappings**

Map `case_narrative` to `H.1` and `additional_information` to `H.additionalInformation`. Mark unsupported notation workflow-only. Add assignments only if production import is missing them.

- [ ] **Step 3: Update documentation and repository-wide assertion**

Replace Reporter-only scope text with all six configured sections. The repository test asserts the configured set is exactly `sender`, `receiver`, `product`, `reporter`, `study`, and `narrative`.

- [ ] **Step 4: Run final verification**

Run:

```sh
python3 -m unittest discover -s registry/tools -p 'test_*.py' -v
python3 registry/tools/validate.py --strict-presave-registry
python3 registry/tools/validate.py --strict-presave-inventory
python3 scripts/validate_presave_reference_matrices.py
```

Run affected backend presave API tests and frontend presave mapper/import Jest suites. Expected: all scoped commands pass with zero missing joins or mappings.

- [ ] **Step 5: Commit**

Backend: `feat: complete strict presave registry coverage`; frontend if needed: `fix: complete narrative presave case transfer`.
