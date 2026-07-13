# ICH Structured Allowed-Value Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every one of the 103 currently inactive, machine-checkable ICH structured allowed-value constraints executable or explicitly enforced at the input representation boundary through one reusable validation engine.

**Architecture:** The committed ICH dictionary remains the standards source of truth and carries executable constraint parameters. A new `allowed_value.rs` module interprets catalog metadata against typed values and immutable vocabulary snapshots, while `rule_table.rs` only traverses scalar/indexed/nested/grandchild model structures and emits concrete paths. Typed Rust fields use `representation_enforced` only when deserialization and persistence make an invalid value impossible; all string-backed constraints run in `CaseValidate`.

**Tech Stack:** Python 3 `unittest`, JSON Schema draft 2020-12, Rust 2021, Serde/serde_json, rust_decimal through `sqlx::types::Decimal`, `time::Date`, existing validator catalog and case rule tables.

## Global Constraints

- Scope is ICH only: do not activate or alter FDA/MFDS regional constraints.
- The target inventory is exactly 103 inactive machine-checkable constraints: numeric 40, vocabulary 26, format 23, boolean 7, true_marker 6, code_set 1.
- Preserve the original `allowed_values` prose byte-for-byte in generated dictionary entries.
- No rule-specific finite values, vocabulary membership, or format parameters may exist only in a section validator.
- Runtime case validation is offline and must not make network calls.
- UCUM grammar symbols come from official UCUM source/essence XML, ISO 639 from official Set 2 data, and EDQM from an authenticated approved export.
- General UCUM values use a grammar parser; constrained UCUM fields additionally require official ICH CL25/CL26 artifacts.
- MPID, PhPID, and SubstanceID are identifier profiles, not finite vocabulary membership checks.
- Empty optional values pass allowed-value validation; required rules own presence.
- A populated nullFlavor suppresses value validation only when the model has no simultaneous value.
- Invalid indexed values retain concrete paths such as `parents.1` and `medicalHistory.1`; no canonical path fallback is allowed.
- Existing required, max-length, future-date, nullFlavor, MedDRA, XML, and regional behavior must remain unchanged.
- EDQM production snapshot generation requires `EDQM_SOURCE_FILE` pointing to an approved official export; fixture tests may run without credentials, but the 103-rule completion claim cannot be made without the approved snapshot.

Reference design: `docs/superpowers/specs/2026-07-13-ich-structured-allowed-value-validation-design.md`.

---

### Task 1: Add Executable Constraint Metadata to the Dictionary

**Files:**
- Modify: `registry/dictionary.schema.json:24-57`
- Modify: `registry/tools/build_dictionary.py:59-105`
- Modify: `registry/tools/build_dictionary.py:150-170`
- Modify: `registry/tools/test_build_dictionary.py:80-160`
- Modify: `registry/tools/validate.py:284-329`
- Modify: `registry/tools/test_validate.py:943-1022`
- Regenerate: `registry/dictionary/ich-e2br3.json`

**Interfaces:**
- Consumes: official ICH CSV rows already read by `parse_ich_csv(text: str)`.
- Produces: `allowed_value_constraint` objects with `kind`, optional `values`, optional `numeric_shape`, optional `format_name`, optional `vocabulary_scope`, optional `identifier_profile`, and required `enforcement` for every non-descriptive ICH constraint.
- Produces: `allowed_value_constraint(value: str, code: str, data_type: str | None) -> dict[str, Any]`.

- [ ] **Step 1: Write failing schema and builder tests**

Add these focused assertions to `AllowedValueConstraintTests`:

```python
def test_adds_executable_parameters(self):
    self.assertEqual(
        {
            "kind": "numeric",
            "numeric_shape": "decimal",
            "enforcement": "case_validate",
        },
        build_dictionary.allowed_value_constraint("Numeric", "F.r.3.2", "ST"),
    )
    self.assertEqual(
        {
            "kind": "format",
            "format_name": "e2b_datetime",
            "enforcement": "case_validate",
        },
        build_dictionary.allowed_value_constraint(
            "CCYYMMDDHHMMSS.UUUU[+|-ZZzz]", "N.2.r.4", "TS"
        ),
    )
    self.assertEqual(
        {
            "kind": "vocabulary",
            "identifier_profile": "mpid",
            "enforcement": "case_validate",
        },
        build_dictionary.allowed_value_constraint("MPID", "D.8.r.2b", "II"),
    )

def test_partitions_machine_checkable_constraints_by_enforcement(self):
    entries = build_dictionary.parse_ich_csv(
        (build_dictionary.SOURCES_DIR / build_dictionary.ICH_SOURCE).read_text(
            encoding="utf-8"
        )
    )
    machine = [
        entry["allowed_value_constraint"]
        for entry in entries
        if entry.get("allowed_value_constraint", {}).get("kind") != "descriptive"
    ]
    self.assertEqual(133, len(machine))
    self.assertTrue(all(rule["enforcement"] in {
        "case_validate", "representation_enforced"
    } for rule in machine))
```

Add validator tests proving incompatible combinations fail:

```python
def validate_allowed_constraint(self, constraint: dict[str, object]):
    entry = (
        '{"code": "C.3.1", "name": "Sender Type", "section": "C",'
        ' "kind": "element", "conformance": "mandatory",'
        ' "allowed_values": "Numeric",'
        f' "allowed_value_constraint": {json.dumps(constraint)}}}'
    )
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        self.write_registry(root, self.sender_row(code="C.3.1"))
        self.write_dictionary(root, "ich-e2br3.json", self.ich_dictionary(entry))
        return validate.validate_registry(root, validate_backend_inventory=False)

def test_dictionary_numeric_constraint_requires_shape(self):
    result = self.validate_allowed_constraint(
        {"kind": "numeric", "enforcement": "case_validate"}
    )
    self.assertIn("numeric allowed_value_constraint requires numeric_shape", "\n".join(result.errors))

def test_dictionary_identifier_rejects_vocabulary_scope(self):
    result = self.validate_allowed_constraint(
        {
            "kind": "vocabulary",
            "identifier_profile": "mpid",
            "vocabulary_scope": "all",
            "enforcement": "case_validate",
        }
    )
    self.assertIn("identifier_profile cannot be combined with vocabulary_scope", "\n".join(result.errors))
```

- [ ] **Step 2: Run the focused tests and confirm RED**

Run:

```bash
python3 -m unittest \
  registry.tools.test_build_dictionary.AllowedValueConstraintTests.test_adds_executable_parameters \
  registry.tools.test_build_dictionary.AllowedValueConstraintTests.test_partitions_machine_checkable_constraints_by_enforcement \
  registry.tools.test_validate.DictionaryValidatorTests.test_dictionary_numeric_constraint_requires_shape \
  registry.tools.test_validate.DictionaryValidatorTests.test_dictionary_identifier_rejects_vocabulary_scope -v
```

Expected: FAIL because the builder accepts only one argument and the schema/validator know only `kind` and `values`.

- [ ] **Step 3: Extend the schema and deterministic builder**

Add these enums to the schema properties and conditional requirements:

```json
"numeric_shape": { "enum": ["decimal", "integer", "dotted_version"] },
"format_name": { "enum": ["e2b_datetime", "base64", "ich_identifier"] },
"vocabulary_scope": { "enum": ["all", "time", "gestation", "dose", "frequency", "dose_form", "route"] },
"identifier_profile": { "enum": ["mpid", "phpid", "substance_id"] },
"enforcement": { "enum": ["case_validate", "representation_enforced"] }
```

Implement classification through named helpers, keeping ambiguous code decisions centralized:

```python
ICH_IDENTIFIER_PROFILES = {
    "D.8.r.2b": "mpid", "D.10.8.r.2b": "mpid", "G.k.2.1.1b": "mpid",
    "D.8.r.3b": "phpid", "D.10.8.r.3b": "phpid", "G.k.2.1.2b": "phpid",
    "G.k.2.3.r.2b": "substance_id",
}

ICH_VOCABULARY_SCOPES = {
    "D.2.2b": "time", "D.2.2.1b": "gestation", "D.10.2.2b": "time",
    "E.i.6b": "time", "F.r.3.3": "all", "G.k.2.3.r.3b": "all",
    "G.k.4.r.1b": "dose", "G.k.4.r.3": "frequency",
    "G.k.4.r.6b": "time", "G.k.5b": "dose", "G.k.6b": "gestation",
    "G.k.9.i.3.1b": "time", "G.k.9.i.3.2b": "time",
    "E.i.1.1b": "all", "H.5.r.1b": "all",
    "G.k.4.r.9.2a": "dose_form", "G.k.4.r.9.2b": "dose_form",
    "G.k.4.r.10.2b": "route", "G.k.4.r.11.2b": "route",
}

def allowed_value_constraint(value: str, code: str, data_type: str | None) -> dict[str, Any]:
    result = classify_allowed_value_kind(value)
    kind = result["kind"]
    if kind == "descriptive":
        return result
    result["enforcement"] = enforcement_for(code, kind, data_type)
    if kind == "numeric":
        result["numeric_shape"] = numeric_shape_for(value, data_type)
    elif kind == "format":
        result["format_name"] = format_name_for(value)
    elif code in ICH_IDENTIFIER_PROFILES:
        result["identifier_profile"] = ICH_IDENTIFIER_PROFILES[code]
    elif code in ICH_VOCABULARY_SCOPES:
        result["vocabulary_scope"] = ICH_VOCABULARY_SCOPES[code]
    return result
```

`enforcement_for` must return `representation_enforced` only for Boolean, Decimal, integer, and Date model representations proven in Task 6 boundary tests. Everything else returns `case_validate`.

Update `parse_ich_csv` to pass the code and source data type:

```python
data_type = optional_value(cell(row, "DATA TYPE"))
if data_type is not None:
    entry["data_type"] = data_type
entry["allowed_value_constraint"] = allowed_value_constraint(
    allowed_values, code, data_type
)
```

Update `validate_dictionary_entry` with exact allowed keys and mutual-exclusion checks. Require `enforcement` for every kind except `descriptive`; require each kind-specific field and reject it on other kinds.

- [ ] **Step 4: Regenerate and verify dictionary stability**

Run:

```bash
python3 registry/tools/build_dictionary.py
python3 -m unittest registry.tools.test_build_dictionary -q
python3 -m unittest \
  registry.tools.test_validate.DictionaryValidatorTests.test_dictionary_numeric_constraint_requires_shape \
  registry.tools.test_validate.DictionaryValidatorTests.test_dictionary_identifier_rejects_vocabulary_scope -v
```

Expected: all selected tests PASS; the committed dictionary differs only by structured constraint metadata.

- [ ] **Step 5: Commit the dictionary contract**

```bash
git add registry/dictionary.schema.json registry/tools/build_dictionary.py \
  registry/tools/test_build_dictionary.py registry/tools/validate.py \
  registry/tools/test_validate.py registry/dictionary/ich-e2br3.json
git commit -m "feat: structure ICH allowed value metadata"
```

---

### Task 2: Import Deterministic Official Vocabulary Snapshots

**Files:**
- Create: `registry/vocabulary.schema.json`
- Create: `registry/tools/import_vocabularies.py`
- Create: `registry/tools/test_import_vocabularies.py`
- Create: `registry/tools/fixtures/ucum-essence-minimal.xml`
- Create: `registry/tools/fixtures/iso-639-2-minimal.txt`
- Create: `registry/tools/fixtures/edqm-minimal.json`
- Create: `registry/vocabularies/ucum.json`
- Create after official CL25/CL26 artifacts are supplied: `registry/vocabularies/ich-ucum-scopes.json`
- Create: `registry/vocabularies/iso639-2.json`

**Interfaces:**
- Produces: `normalize_ucum(source: bytes) -> dict[str, Any]`.
- Produces: `normalize_iso639(source: bytes) -> dict[str, Any]`.
- Produces: `normalize_edqm(source: bytes) -> dict[str, Any]`.
- Produces normalized JSON with `name`, `version`, `source`, `source_sha256`, `license`, and sorted unique `entries`; each entry has `code` and sorted `scopes`.

- [ ] **Step 1: Write failing deterministic importer tests**

```python
class VocabularyImporterTests(unittest.TestCase):
    def test_ucum_normalization_is_deterministic_and_scoped(self):
        raw = (FIXTURES / "ucum-essence-minimal.xml").read_bytes()
        first = import_vocabularies.normalize_ucum(raw)
        second = import_vocabularies.normalize_ucum(raw)
        self.assertEqual(first, second)
        self.assertEqual(hashlib.sha256(raw).hexdigest(), first["source_sha256"])
        self.assertIn({"code": "m", "scopes": ["prefix"]}, first["entries"])
        self.assertIn({"code": "g", "scopes": ["unit"]}, first["entries"])
        self.assertNotIn("mg", [entry["code"] for entry in first["entries"]])

    def test_iso639_uses_set_two_three_letter_codes(self):
        result = import_vocabularies.normalize_iso639(
            (FIXTURES / "iso-639-2-minimal.txt").read_bytes()
        )
        self.assertEqual(["eng", "kor"], [entry["code"] for entry in result["entries"]])

    def test_edqm_maps_dose_form_and_route_scopes(self):
        result = import_vocabularies.normalize_edqm(
            (FIXTURES / "edqm-minimal.json").read_bytes()
        )
        self.assertEqual(
            ["dose_form", "route"],
            sorted({scope for entry in result["entries"] for scope in entry["scopes"]}),
        )
```

- [ ] **Step 2: Run importer tests and confirm RED**

Run: `python3 -m unittest registry.tools.test_import_vocabularies -v`

Expected: ERROR because `import_vocabularies.py` does not exist.

- [ ] **Step 3: Implement schema, fixtures, and importers**

Use `xml.etree.ElementTree` for UCUM, pipe-delimited parsing for ISO 639-2, and `json` for an EDQM export. Serialize with deterministic formatting:

```python
def write_snapshot(path: Path, snapshot: dict[str, Any]) -> None:
    path.write_text(
        json.dumps(snapshot, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

def source_sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()

def normalized_entries(rows: Iterable[tuple[str, Iterable[str]]]) -> list[dict[str, Any]]:
    merged: dict[str, set[str]] = {}
    for code, scopes in rows:
        merged.setdefault(code.strip(), set()).update(scopes)
    return [
        {"code": code, "scopes": sorted(scopes)}
        for code, scopes in sorted(merged.items())
        if code
    ]
```

The CLI must accept explicit official files and never fetch the network:

```bash
python3 registry/tools/import_vocabularies.py \
  --ucum-source "$UCUM_SOURCE_FILE" \
  --iso639-source "$ISO639_SOURCE_FILE" \
  --edqm-source "$EDQM_SOURCE_FILE"
```

UCUM source metadata must reference `https://ucum.org/ucum.xml`; ISO metadata must identify ISO 639-2; EDQM metadata must preserve the export release/retrieval date supplied by the approved payload. Do not commit the minimal EDQM fixture as the production snapshot.

- [ ] **Step 4: Generate official snapshots and verify hashes**

Before this step, set UCUM and ISO source variables to readable official artifacts. Generate `ich-ucum-scopes.json` only from official CL25/CL26 files; the implementation guide identifies the OIDs but does not contain the complete constrained term lists. Each scope row must carry the ICH element code, UCUM code, and source artifact version. Then run:

```bash
python3 -m unittest registry.tools.test_import_vocabularies -v
python3 -m json.tool registry/vocabularies/ucum.json >/dev/null
python3 -m json.tool registry/vocabularies/iso639-2.json >/dev/null
```

Expected: all tests PASS and repeated UCUM/ISO imports produce no Git diff. The EDQM fixture proves parser behavior only; Task 8 requires `EDQM_SOURCE_FILE` and creates the production EDQM snapshot.

- [ ] **Step 5: Commit importer and approved snapshots**

```bash
git add registry/vocabulary.schema.json registry/tools/import_vocabularies.py \
  registry/tools/test_import_vocabularies.py registry/tools/fixtures \
  registry/vocabularies/ucum.json \
  registry/vocabularies/iso639-2.json
git commit -m "feat: add official ICH vocabulary snapshots"
```

---

### Task 3: Carry Dictionary Metadata Through the Catalog

**Files:**
- Modify: `crates/libs/validator/src/catalog.rs:20-78`
- Modify: `crates/libs/validator/src/catalog.rs:3889-3925`
- Modify: `crates/libs/validator/src/catalog.rs:4286-4290`
- Modify: `crates/libs/validator/src/catalog.rs:5257-5300`
- Regenerate: `crates/libs/validator/src/catalog_dictionary_constraints.rs`

**Interfaces:**
- Produces Rust enums `NumericShape`, `FormatName`, `VocabularyScope`, `IdentifierProfile`, and `ConstraintEnforcement` with `snake_case` Serde representations.
- Extends `AllowedValueConstraint` with optional typed metadata.
- Produces `allowed_value_enforcement_for_rule(code: &str) -> Option<ConstraintEnforcement>`.
- Produces `representation_enforced_rule_codes() -> BTreeSet<&'static str>` for boundary-test parity.
- Produces generated `ICH_STRUCTURED_ALLOWED_VALUE_TARGET_CODES: &[&str]` containing the exact 103 pre-activation codes.

- [ ] **Step 1: Add failing catalog parity and coverage tests**

```rust
#[test]
fn ich_machine_constraints_have_enforcement() {
    let machine = allowed_value_constraints()
        .iter()
        .filter(|(_, constraint)| constraint.kind != AllowedValueConstraintKind::Descriptive)
        .collect::<Vec<_>>();
    assert_eq!(machine.len(), 133);
    assert!(machine.iter().all(|(_, constraint)| constraint.enforcement.is_some()));
}

#[test]
fn inactive_ich_target_is_exactly_103_before_activation() {
    let inactive = ALLOWED_VALUE_RULES
        .iter()
        .filter(|rule| {
            let constraint = allowed_value_constraint_for_rule(rule.code).unwrap();
            constraint.kind != AllowedValueConstraintKind::Descriptive
                && phases_for_allowed_value_rule(rule.code) == PHASES_METADATA_ONLY
        })
        .collect::<Vec<_>>();
    assert_eq!(inactive.len(), 103);
}
```

Extend `dictionary_allowed_value_constraints_match_catalog_exactly` to compare all new fields, not only kind and values.
Generate `ICH_STRUCTURED_ALLOWED_VALUE_TARGET_CODES` once from the current metadata-only, non-descriptive baseline and assert its kind distribution is exactly numeric 40, vocabulary 26, format 23, boolean 7, true-marker 6, code-set 1.

- [ ] **Step 2: Run catalog tests and confirm RED**

Run:

```bash
cargo test -p validator --lib catalog::tests::ich_machine_constraints_have_enforcement -- --exact
cargo test -p validator --lib catalog::tests::inactive_ich_target_is_exactly_103_before_activation -- --exact
```

Expected: compile failure because `enforcement` and the typed enums do not exist.

- [ ] **Step 3: Add typed catalog metadata**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericShape { Decimal, Integer, DottedVersion }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatName { E2bDatetime, Base64, IchIdentifier }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VocabularyScope { All, Time, Gestation, Dose, Frequency, DoseForm, Route }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentifierProfile { Mpid, Phpid, SubstanceId }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintEnforcement { CaseValidate, RepresentationEnforced }

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AllowedValueConstraint {
    pub kind: AllowedValueConstraintKind,
    #[serde(default)] pub values: Vec<String>,
    pub numeric_shape: Option<NumericShape>,
    pub format_name: Option<FormatName>,
    pub vocabulary_scope: Option<VocabularyScope>,
    pub identifier_profile: Option<IdentifierProfile>,
    pub enforcement: Option<ConstraintEnforcement>,
}

pub fn representation_enforced_rule_codes() -> BTreeSet<&'static str> {
    ALLOWED_VALUE_RULES.iter()
        .filter_map(|rule| {
            (allowed_value_enforcement_for_rule(rule.code)
                == Some(ConstraintEnforcement::RepresentationEnforced))
                .then_some(rule.code)
        })
        .collect()
}
```

Keep the existing phase allowlist in this task. Dictionary `enforcement` states how a rule must ultimately be enforced, while catalog phase states whether its production case table exists now. Tasks 6-8 add a code to `phases_for_allowed_value_rule` in the same commit that adds its production table; representation-enforced and descriptive codes remain metadata-only.

- [ ] **Step 4: Run catalog parity tests**

Run: `cargo test -p validator --lib catalog::tests --quiet`

Expected: PASS, including exact dictionary/catalog metadata parity and the 103 baseline inventory.

- [ ] **Step 5: Commit catalog metadata**

```bash
git add crates/libs/validator/src/catalog.rs \
  crates/libs/validator/src/catalog_dictionary_constraints.rs
git commit -m "feat: expose ICH constraint metadata in catalog"
```

---

### Task 4: Build the Shared Semantic Constraint Engine

**Files:**
- Create: `crates/libs/validator/src/allowed_value.rs`
- Modify: `crates/libs/validator/Cargo.toml:11-26`
- Modify: `crates/libs/validator/src/lib.rs:1-15`
- Modify: `crates/libs/validator/src/context.rs:38-77`

**Interfaces:**
- Produces `ConstraintValue<'a> = Text(Option<Cow<'a, str>>) | Boolean(Option<bool>) | Decimal(Option<Decimal>) | Date(Option<Date>)`.
- Produces `is_allowed_value_valid(rule_code: &str, value: ConstraintValue<'_>, vocabulary: &VocabularyContext) -> bool`.
- Extends `VocabularyContext` with `contains_snapshot_code(vocabulary: &str, scope: VocabularyScope, code: &str) -> bool`.

- [ ] **Step 1: Write failing engine tests**

```rust
#[test]
fn empty_optional_values_are_valid() {
    assert!(is_allowed_value_valid(
        "ICH.F.r.3.2.ALLOWED.VALUE",
        ConstraintValue::Text(None),
        &VocabularyContext::default(),
    ));
}

#[test]
fn validates_catalog_code_set_and_true_marker() {
    let vocabulary = VocabularyContext::default();
    assert!(is_allowed_value_valid("ICH.E.i.7.ALLOWED.VALUE", ConstraintValue::Text(Some("1".into())), &vocabulary));
    assert!(!is_allowed_value_valid("ICH.E.i.7.ALLOWED.VALUE", ConstraintValue::Text(Some("99".into())), &vocabulary));
    assert!(!is_allowed_value_valid("ICH.D.7.3.ALLOWED.VALUE", ConstraintValue::Boolean(Some(false)), &vocabulary));
}

#[test]
fn validates_numeric_shapes_without_accepting_partial_parses() {
    let vocabulary = VocabularyContext::default();
    assert!(is_allowed_value_valid("ICH.F.r.3.2.ALLOWED.VALUE", ConstraintValue::Text(Some("12.5".into())), &vocabulary));
    assert!(!is_allowed_value_valid("ICH.F.r.3.2.ALLOWED.VALUE", ConstraintValue::Text(Some("12mg".into())), &vocabulary));
}
```

- [ ] **Step 2: Run engine tests and confirm RED**

Run: `cargo test -p validator --lib allowed_value::tests --quiet`

Expected: compile failure because the module and interfaces do not exist.

- [ ] **Step 3: Implement semantic dispatch**

```rust
pub(crate) enum ConstraintValue<'a> {
    Text(Option<Cow<'a, str>>),
    Boolean(Option<bool>),
    Decimal(Option<Decimal>),
    Date(Option<Date>),
}

impl ConstraintValue<'_> {
    fn is_empty(&self) -> bool {
        match self {
            Self::Text(value) => value.as_deref().map(str::trim).is_none_or(str::is_empty),
            Self::Boolean(value) => value.is_none(),
            Self::Decimal(value) => value.is_none(),
            Self::Date(value) => value.is_none(),
        }
    }
}

pub(crate) fn is_allowed_value_valid(
    rule_code: &str,
    value: ConstraintValue<'_>,
    vocabulary: &VocabularyContext,
) -> bool {
    let constraint = allowed_value_constraint_for_rule(rule_code)
        .unwrap_or_else(|| panic!("missing catalog allowed-value constraint: {rule_code}"));
    if value.is_empty() { return true; }
    match constraint.kind {
        AllowedValueConstraintKind::CodeSet => validate_code_set(constraint, value),
        AllowedValueConstraintKind::Boolean => validate_boolean(value),
        AllowedValueConstraintKind::TrueMarker => validate_true_marker(value),
        AllowedValueConstraintKind::Numeric => validate_numeric(constraint, value),
        AllowedValueConstraintKind::Format => validate_format(constraint, value),
        AllowedValueConstraintKind::Vocabulary => validate_vocabulary(constraint, value, vocabulary),
        AllowedValueConstraintKind::Descriptive => true,
    }
}
```

Use full-string predicates: Decimal parsing for `decimal`, ASCII digits for `integer`, `digits.digits` for `dotted_version`, existing `flex_date::e2b_datetime_date` for E2B time, `base64 = "0.22"` with `general_purpose::STANDARD.decode(value)` for Base64, and a UCUM grammar parser compatible with Rust 1.88. Implement exact identifier-profile predicates from the ICH representation text. A type mismatch between metadata and `ConstraintValue` must panic in tests/configuration instead of accepting the value.

- [ ] **Step 4: Embed vocabulary snapshots in `VocabularyContext`**

Parse the committed UCUM and ISO 639 JSON once with `OnceLock` and merge it with the existing MedDRA state. Task 8 adds the EDQM snapshot through the same loader after the approved export exists:

```rust
#[derive(Debug, Clone, Default)]
pub struct VocabularyContext {
    meddra_available: bool,
    meddra_versions: HashSet<String>,
    meddra_terms: HashSet<MeddraTermKey>,
    snapshot_codes: Arc<HashMap<(String, VocabularyScope), HashSet<String>>>,
}
```

`load_vocabulary_context` must initialize snapshot codes even when no MedDRA release is installed. Unknown vocabulary/scope pairs must panic during snapshot construction, never return a permissive result.

- [ ] **Step 5: Run focused and existing validator tests**

Run:

```bash
cargo test -p validator --lib allowed_value::tests --quiet
cargo test -p validator --lib context::tests --quiet
cargo test -p validator --lib --quiet
```

Expected: all tests PASS; the last command retains the existing 119+ passing library tests.

- [ ] **Step 6: Commit the semantic engine**

```bash
git add crates/libs/validator/src/allowed_value.rs \
  crates/libs/validator/src/context.rs crates/libs/validator/src/lib.rs \
  crates/libs/validator/Cargo.toml Cargo.lock
git commit -m "feat: add shared allowed value engine"
```

---

### Task 5: Generalize Rule-Table Traversal and Preserve Concrete Paths

**Files:**
- Modify: `crates/libs/validator/src/case/sections/rule_table.rs:80-170`
- Modify: `crates/libs/validator/src/case/sections/rule_table.rs:235-617`
- Modify: `crates/libs/validator/src/case/sections/rule_table.rs:1088-1210`
- Modify: `crates/libs/validator/src/case/sections/mod.rs`

**Interfaces:**
- Produces `ConstraintRule<T>`, `IndexedConstraintRule<T>`, `NestedConstraintRule<P, T>`, and `GrandchildConstraintRule<P, C, T>`.
- Produces `eval_constraints`, `eval_indexed_constraints`, `eval_nested_constraints`, and `eval_grandchild_constraints`.
- Produces `implemented_allowed_value_rule_codes() -> BTreeSet<&'static str>` by combining each section's table codes.
- All evaluators call only `is_allowed_value_valid`; they never reimplement semantics.

- [ ] **Step 1: Write failing traversal and path tests**

```rust
#[test]
fn indexed_constraint_retains_actual_index() {
    let items = [Item { value: Some("1") }, Item { value: Some("99") }];
    let rules = [IndexedConstraintRule {
        code: "ICH.E.i.7.ALLOWED.VALUE",
        path: |index| format!("reactions.{index}.outcome"),
        value: |item| ConstraintValue::Text(item.value.map(Cow::Borrowed)),
    }];
    let mut issues = Vec::new();
    eval_indexed_constraints(&mut issues, &items, &rules, &VocabularyContext::default());
    assert_eq!(issues[0].field_path, "reactions.1.outcome");
}

#[test]
fn grandchild_constraint_retains_parent_child_and_item_indexes() {
    let mut issues = Vec::new();
    eval_grandchild_constraints(&mut issues, &parents, &children, &items, &rules, &vocabulary);
    assert_eq!(issues[0].field_path, "drugs.1.dosages.2.intervals.3.unit");
}
```

- [ ] **Step 2: Run traversal tests and confirm RED**

Run: `cargo test -p validator --lib case::sections::rule_table::tests --quiet`

Expected: compile failure because the generic constraint rule shapes do not exist.

- [ ] **Step 3: Implement the four reusable evaluator shapes**

```rust
pub(crate) struct IndexedConstraintRule<T> {
    pub code: &'static str,
    pub path: fn(usize) -> String,
    pub value: for<'a> fn(&'a T) -> ConstraintValue<'a>,
}

pub(crate) struct ConstraintRule<T> {
    pub code: &'static str,
    pub path: &'static str,
    pub value: for<'a> fn(&'a T) -> ConstraintValue<'a>,
}

pub(crate) fn eval_indexed_constraints<T>(
    issues: &mut Vec<ValidationIssue>,
    items: &[T],
    rules: &[IndexedConstraintRule<T>],
    vocabulary: &VocabularyContext,
) {
    for (index, item) in items.iter().enumerate() {
        for rule in rules {
            if !is_allowed_value_valid(rule.code, (rule.value)(item), vocabulary) {
                push_issue_by_code(issues, rule.code, (rule.path)(index));
            }
        }
    }
}
```

Nested and grandchild evaluators must join records by explicit owner key functions and build paths from the actual owner and child indexes. Delete `invalid_code`, `invalid_true_marker`, `invalid_numeric_text`, `invalid_datetime_text`, and `invalid_vocabulary` only after all current callers use the new engine.

For existing true-marker storage where `false` accompanies a nullFlavor placeholder, add one shared extractor helper in `allowed_value.rs`:

```rust
pub(crate) fn true_marker_value<'a>(
    value: Option<bool>,
    null_flavor: Option<&'a str>,
) -> ConstraintValue<'a> {
    if null_flavor.is_some_and(|nf| !nf.trim().is_empty()) {
        ConstraintValue::Boolean(None)
    } else {
        ConstraintValue::Boolean(value)
    }
}
```

All true-marker table extractors call this helper; no section duplicates nullFlavor suppression logic.

- [ ] **Step 4: Migrate existing allowed-value helpers without behavior changes**

Replace `AllowedCodeRule`, `VocabularyRule`, `TrueMarkerRule`, indexed variants, and nested variants with aliases or direct `ConstraintRule` tables. Existing section tests must keep the same issue codes and paths.

- [ ] **Step 5: Run rule-table and full validator tests**

Run:

```bash
cargo test -p validator --lib case::sections::rule_table::tests --quiet
cargo test -p validator --lib --quiet
cargo test -p validator --test xml --quiet
```

Expected: all tests PASS, including concrete index regression tests and 15 XML tests.

- [ ] **Step 6: Commit traversal reuse**

```bash
git add crates/libs/validator/src/case/sections/rule_table.rs \
  crates/libs/validator/src/case/sections/mod.rs \
  crates/libs/validator/src/case/sections/{c,d,e,f,g,h,n}.rs
git commit -m "refactor: unify allowed value rule traversal"
```

---

### Task 6: Prove Representation-Enforced Numeric and Boolean Rules

**Files:**
- Create: `crates/libs/validator/tests/allowed_value_representation.rs`
- Modify: `crates/libs/validator/src/case/sections/{d,e,f,g,h}.rs`

**Interfaces:**
- Consumes dictionary `ConstraintEnforcement::RepresentationEnforced` metadata.
- Produces one table-driven deserialization-boundary case per representation-enforced code and proves the case-code set exactly matches catalog metadata.

- [ ] **Step 1: Write failing boundary tests**

```rust
struct BoundaryCase {
    code: &'static str,
    rejects: fn() -> bool,
}

#[test]
fn every_representation_enforced_code_rejects_invalid_input() {
    let cases = boundary_cases();
    assert!(cases.iter().all(|case| (case.rejects)()));
    let tested = cases.iter().map(|case| case.code).collect::<BTreeSet<_>>();
    let catalog = representation_enforced_rule_codes();
    assert_eq!(tested, catalog);
}
```

`boundary_cases()` must include every representation-enforced code. Each `rejects` function deserializes that code's actual create/update DTO with an invalid JSON representation, such as `"12mg"` for Decimal, `"1"` for Boolean, `1.5` for integer, or an impossible calendar date for Date.

Add catalog coverage that collects all `representation_enforced` target codes, asserts the set is non-empty, and asserts no such code appears in a production `ConstraintRule` table.

- [ ] **Step 2: Run boundary tests and establish the baseline**

Run: `cargo test -p validator --test allowed_value_representation --quiet`

Expected: tests PASS for already typed fields. If a DTO accepts the invalid payload, classify that code as `case_validate` and add a production case rule instead of weakening the test.

- [ ] **Step 3: Add case rules only for string-backed numeric/boolean fields**

Use the shared tables:

```rust
const F_CONSTRAINT_RULES: &[IndexedConstraintRule<TestResult>] = &[
    IndexedConstraintRule {
        code: "ICH.F.r.3.2.ALLOWED.VALUE",
        path: |index| format!("tests.{index}.testResultValue"),
        value: |test| ConstraintValue::Text(
            test.test_result_value.as_deref().map(Cow::Borrowed)
        ),
    },
];
```

Do not turn typed Decimal values into strings merely to produce a case issue. Their catalog phase remains metadata-only with `representation_enforced`.

- [ ] **Step 4: Verify numeric/boolean target coverage**

Add a catalog test that filters the original 47 inactive numeric/boolean target codes and asserts each is now either present in a production table with `case_validate` or covered by the representation boundary set.

Run:

```bash
cargo test -p validator --test allowed_value_representation --quiet
cargo test -p validator --lib catalog::tests::numeric_and_boolean_targets_are_enforced -- --exact
cargo test -p validator --lib case::sections --quiet
```

Expected: PASS with exactly 47 target codes partitioned and no overlap.

- [ ] **Step 5: Commit numeric and boolean enforcement**

```bash
git add crates/libs/validator/tests/allowed_value_representation.rs \
  crates/libs/validator/src/case/sections/{d,e,f,g,h}.rs \
  registry/dictionary/ich-e2br3.json
git commit -m "feat: enforce ICH numeric and boolean constraints"
```

---

### Task 7: Activate Code-Set, True-Marker, and Format Rules

**Files:**
- Create from approved source: `registry/vocabularies/edqm.json`
- Modify: `crates/libs/validator/src/allowed_value.rs`
- Modify: `crates/libs/validator/src/case/sections/{n,c,d,e,f}.rs`
- Modify: `crates/libs/validator/src/catalog.rs`

**Interfaces:**
- Activates exactly 7 target code-set/true-marker constraints and 23 target format constraints.
- Reuses `ConstraintRule` traversal and catalog semantics; section files contain only code, path, and extractor.

- [ ] **Step 1: Add failing semantic and section regressions**

For each format profile, test one valid and one invalid full string:

```rust
#[test]
fn e2b_datetime_rejects_trailing_text() {
    assert!(!valid_format(FormatName::E2bDatetime, "20260713123000+0900junk"));
}

#[test]
fn base64_rejects_invalid_alphabet_and_padding() {
    assert!(valid_format(FormatName::Base64, "SGVsbG8="));
    assert!(!valid_format(FormatName::Base64, "SGVsbG8*"));
}
```

Add section tests that construct invalid values in N/C/D/E/F and assert exact code plus concrete path. Include a false true-marker with no nullFlavor, and a false true-marker with an accepted nullFlavor to preserve the current exception.

- [ ] **Step 2: Run section tests and confirm RED**

Run:

```bash
cargo test -p validator --lib allowed_value::tests::e2b_datetime_rejects_trailing_text -- --exact
cargo test -p validator --lib allowed_value::tests::base64_rejects_invalid_alphabet_and_padding -- --exact
cargo test -p validator --lib case::sections --quiet
```

Expected: new target-rule tests FAIL because their tables are absent.

- [ ] **Step 3: Add scalar/indexed/nested tables by section**

Each table entry must have this form:

```rust
ConstraintRule {
    code: "ICH.N.2.r.4.ALLOWED.VALUE",
    path: "messageHeader.messageDateTime",
    value: |header| ConstraintValue::Text(
        header.message_date_time.as_deref().map(Cow::Borrowed)
    ),
}
```

Use dictionary `format_name`, `values`, and true-marker kind for semantics. Do not add code arrays, parsers, or date patterns in section files.

- [ ] **Step 4: Verify exact target counts and regressions**

Run:

```bash
cargo test -p validator --lib catalog::tests::code_marker_and_format_targets_are_case_validated -- --exact
cargo test -p validator --lib case::sections --quiet
cargo test -p validator --test xml --quiet
```

Expected: exactly 30 newly targeted codes are enforced; all tests PASS.

- [ ] **Step 5: Commit marker and format activation**

```bash
git add crates/libs/validator/src/allowed_value.rs crates/libs/validator/src/catalog.rs \
  crates/libs/validator/src/case/sections/{n,c,d,e,f}.rs
git commit -m "feat: enforce ICH marker and format constraints"
```

---

### Task 8: Activate UCUM, ISO 639, EDQM, and Identifier Rules

**Files:**
- Modify: `crates/libs/validator/src/allowed_value.rs`
- Modify: `crates/libs/validator/src/context.rs`
- Modify: `crates/libs/validator/src/case/sections/{d,e,f,g,h}.rs`
- Modify: `crates/libs/validator/src/catalog.rs`
- Test: `crates/libs/validator/src/allowed_value.rs`

**Interfaces:**
- Activates exactly 26 target vocabulary/identifier constraints.
- Uses 13 UCUM rules, 2 ISO 639 rules, 4 EDQM rules, 3 MPID rules, 3 PhPID rules, and 1 SubstanceID rule.

- [ ] **Step 1: Write failing vocabulary and identifier tests**

```rust
#[test]
fn ucum_scope_rejects_valid_code_in_wrong_scope() {
    let vocabulary = VocabularyContext::for_snapshot_entries(&[
        ("UCUM", VocabularyScope::All, "mg"),
        ("UCUM", VocabularyScope::Dose, "mg"),
        ("UCUM", VocabularyScope::All, "Cel"),
    ]);
    assert!(vocabulary.contains_snapshot_code("UCUM", VocabularyScope::Dose, "mg"));
    assert!(!vocabulary.contains_snapshot_code("UCUM", VocabularyScope::Dose, "Cel"));
}

#[test]
fn iso639_requires_set_two_code() {
    let vocabulary = VocabularyContext::for_snapshot_entries(&[
        ("ISO639", VocabularyScope::All, "eng"),
    ]);
    assert!(vocabulary.contains_snapshot_code("ISO639", VocabularyScope::All, "eng"));
    assert!(!vocabulary.contains_snapshot_code("ISO639", VocabularyScope::All, "en"));
}

#[test]
fn identifiers_require_complete_ich_identifier_representation() {
    assert!(valid_identifier(IdentifierProfile::Mpid, "2.16.840.1.113883.3.989^12345"));
    assert!(!valid_identifier(IdentifierProfile::Mpid, "12345"));
}
```

Add section regressions for a second reaction language, a second parent product identifier, and a nested second drug dosage unit. Assert paths include index `1`.

- [ ] **Step 2: Run vocabulary tests and confirm RED**

Run:

```bash
cargo test -p validator --lib allowed_value::tests::ucum_scope_rejects_valid_code_in_wrong_scope -- --exact
cargo test -p validator --lib allowed_value::tests::iso639_requires_set_two_code -- --exact
cargo test -p validator --lib allowed_value::tests::identifiers_require_complete_ich_identifier_representation -- --exact
```

Expected: FAIL until snapshot lookup and identifier predicates are connected.

- [ ] **Step 3: Add the exact 26 production table entries**

First generate the production EDQM snapshot from the approved export and verify it against `registry/vocabulary.schema.json`:

```bash
test -r "$EDQM_SOURCE_FILE"
python3 registry/tools/import_vocabularies.py --edqm-source "$EDQM_SOURCE_FILE"
python3 -m json.tool registry/vocabularies/edqm.json >/dev/null
```

Use these inventories:

```text
UCUM: D.2.2b, D.2.2.1b, D.10.2.2b, E.i.6b, F.r.3.3,
      G.k.2.3.r.3b, G.k.4.r.1b, G.k.4.r.3, G.k.4.r.6b,
      G.k.5b, G.k.6b, G.k.9.i.3.1b, G.k.9.i.3.2b
MPID: D.8.r.2b, D.10.8.r.2b, G.k.2.1.1b
PhPID: D.8.r.3b, D.10.8.r.3b, G.k.2.1.2b
ISO639: E.i.1.1b, H.5.r.1b
SubstanceID: G.k.2.3.r.2b
EDQM: G.k.4.r.9.2a, G.k.4.r.9.2b, G.k.4.r.10.2b, G.k.4.r.11.2b
```

Table extractors return borrowed text. UCUM and EDQM membership comes from `VocabularyContext`; identifier predicates come from catalog `identifier_profile`.

- [ ] **Step 4: Verify exact vocabulary coverage and concrete paths**

Run:

```bash
cargo test -p validator --lib allowed_value::tests --quiet
cargo test -p validator --lib catalog::tests::vocabulary_and_identifier_targets_are_case_validated -- --exact
cargo test -p validator --lib case::sections::{d,e,f,g,h} --quiet
```

Expected: exactly 26 target codes are `CaseValidate`, EDQM uses the approved committed snapshot, and indexed paths retain their real indexes.

- [ ] **Step 5: Commit vocabulary activation**

```bash
git add crates/libs/validator/src/allowed_value.rs crates/libs/validator/src/context.rs \
  crates/libs/validator/src/catalog.rs crates/libs/validator/src/case/sections/{d,e,f,g,h}.rs \
  registry/vocabularies
git commit -m "feat: enforce ICH vocabulary and identifier constraints"
```

---

### Task 9: Close the 103-Rule Inventory and Guard Against Drift

**Files:**
- Modify: `crates/libs/validator/src/catalog.rs:5257-5310`
- Modify: `registry/tools/test_build_dictionary.py:105-160`
- Modify: `registry/tools/test_validate.py`
- Modify: `registry/catalog-implementation-inventory.md`

**Interfaces:**
- Produces a permanent zero-gap test for the original 103 inactive machine-checkable ICH constraints.
- Produces exact parity checks from official CSV to dictionary to catalog to runtime enforcement.

- [ ] **Step 1: Replace the baseline inventory test with the completion invariant**

```rust
#[test]
fn all_103_ich_structured_allowed_value_targets_are_enforced() {
    let target = ICH_STRUCTURED_ALLOWED_VALUE_TARGET_CODES
        .iter()
        .map(|code| (*code).to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(target.len(), 103);

    let mut case_validate = BTreeSet::new();
    let mut representation = BTreeSet::new();
    for code in &target {
        match allowed_value_enforcement_for_rule(code) {
            Some(ConstraintEnforcement::CaseValidate) => {
                assert!(production_constraint_rule_codes().contains(code));
                case_validate.insert(code.clone());
            }
            Some(ConstraintEnforcement::RepresentationEnforced) => {
                representation.insert(code.clone());
            }
            None => panic!("unclassified ICH allowed-value target: {code}"),
        }
    }
    assert!(case_validate.is_disjoint(&representation));
    assert_eq!(case_validate.len() + representation.len(), 103);
}
```

Use `ICH_STRUCTURED_ALLOWED_VALUE_TARGET_CODES` from `catalog_dictionary_constraints.rs` as `target`; never recompute the target from current phases. Use `case::sections::implemented_allowed_value_rule_codes()` for the production registry. The integration test from Task 6 independently proves every member of `representation` has an input-boundary case.

- [ ] **Step 2: Run the completion test and confirm any remaining RED entries**

Run: `cargo test -p validator --lib catalog::tests::all_103_ich_structured_allowed_value_targets_are_enforced -- --exact`

Expected before final fixes: FAIL listing every case-validated code missing from a production table. Add the reported code to its owning section table with its concrete model extractor and path.

- [ ] **Step 3: Update inventory documentation from measured results**

Record the completion invariant in `registry/catalog-implementation-inventory.md`:

```markdown
| Constraint group | Dictionary target | Enforced total | Gap |
|---|---:|---:|---:|
| ICH structured allowed-value target | 103 | 103 | 0 |
```

- [ ] **Step 4: Run complete verification**

Run:

```bash
python3 -m unittest registry.tools.test_build_dictionary -q
python3 -m unittest registry.tools.test_import_vocabularies -q
python3 -m unittest registry.tools.test_validate -q
cargo test -p validator --test allowed_value_representation --quiet
cargo test -p validator --lib --quiet
cargo test -p validator --test xml --quiet
cargo fmt --all -- --check
cargo clippy -p validator -p lib-core --all-targets -- -D warnings
```

Expected:

- Dictionary and vocabulary tests PASS.
- The full `test_validate` suite may still report only the pre-existing frontend mapping inventory failure; compare the exact failure to the baseline before this work and do not suppress new failures.
- Validator library and XML tests PASS with zero warnings.
- Formatting and Clippy PASS.
- The 103 target codes partition exactly into case validation and representation enforcement with zero gap and zero overlap.

- [ ] **Step 5: Commit the coverage guard and inventory**

```bash
git add crates/libs/validator/src/catalog.rs registry/tools/test_build_dictionary.py \
  registry/tools/test_validate.py registry/catalog-implementation-inventory.md
git commit -m "test: close ICH structured value coverage"
```
