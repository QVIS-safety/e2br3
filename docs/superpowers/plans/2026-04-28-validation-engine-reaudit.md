# Validation Engine Re-Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make backend case validation reliable enough for null flavor handling, date business rules, and section/subsection UI indicators.

**Architecture:** Keep validation authority in `lib-core`; web-server endpoints continue to serialize validation reports without duplicating rule logic. Add stable subsection metadata and section/subsection summaries to `CaseValidationReport`, then add catalog-backed date rules in the relevant section collectors. UI work, regional field rendering, and terminology search UX are separate follow-up plans that should consume this backend contract.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL test database, `cargo test`, existing web-server validation test harness.

---

## Files

- Modify: `crates/libs/lib-core/src/validation/mod.rs`
- Modify: `crates/libs/lib-core/src/validation/catalog.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/mod.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/c.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/d.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/e.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/f.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/g.rs`
- Modify: `crates/services/web-server/tests/validation/validation_common.rs`
- Modify: `crates/services/web-server/tests/validation/c.rs`
- Modify: `crates/services/web-server/tests/validation/d.rs`
- Modify: `crates/services/web-server/tests/validation/e.rs`
- Modify: `crates/services/web-server/tests/validation/f.rs`
- Modify: `crates/services/web-server/tests/validation/g.rs`
- Modify: `docs/requirements/client_requirements_todo.md`

---

### Task 1: Add Subsection Metadata To Validation Issues

**Files:**
- Modify: `crates/libs/lib-core/src/validation/mod.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/mod.rs`
- Modify: `crates/services/web-server/tests/validation/validation_common.rs`
- Modify: `crates/services/web-server/tests/validation/c.rs`

- [ ] **Step 1: Write the failing API contract test**

Append this test to `crates/services/web-server/tests/validation/c.rs`:

```rust
#[serial]
#[tokio::test]
async fn c_validation_issues_include_stable_subsection_metadata() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE safety_report_identification SET transmission_date = NULL, transmission_date_null_flavor = NULL WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	let issue = report["data"]["issues"]
		.as_array()
		.expect("issues array")
		.iter()
		.find(|issue| issue["code"] == "ICH.C.1.2.REQUIRED")
		.expect("ICH.C.1.2.REQUIRED issue");

	assert_eq!(issue["section"].as_str(), Some("case-identification"));
	assert_eq!(issue["subsection"].as_str(), Some("C.1"));
	assert_eq!(
		issue["field_path"].as_str(),
		Some("safetyReportIdentification.transmissionDate")
	);
	Ok(())
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p web-server c_validation_issues_include_stable_subsection_metadata --test validation -- --nocapture
```

Expected: FAIL because `issue["subsection"]` is absent or null.

- [ ] **Step 3: Add subsection to `ValidationIssue`**

In `crates/libs/lib-core/src/validation/mod.rs`, change `ValidationIssue` to include `subsection`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
	pub code: String,
	pub message: String,
	pub path: String,
	pub field_path: Option<String>,
	pub section: String,
	pub subsection: String,
	pub blocking: bool,
}
```

Then update both `push_issue_by_code` branches so each issue sets:

```rust
subsection: case::sections::resolve_validation_subsection(code, Some(&path)),
```

- [ ] **Step 4: Implement subsection resolution**

In `crates/libs/lib-core/src/validation/case/sections/mod.rs`, add this function near `resolve_validation_field_path`:

```rust
pub(crate) fn resolve_validation_subsection(
	code: &str,
	path: Option<&str>,
) -> String {
	if code.starts_with("ICH.C.1.") || code.starts_with("FDA.C.1.") {
		return "C.1".to_string();
	}
	if code.starts_with("ICH.C.2.") || code.starts_with("FDA.C.2.") {
		return "C.2".to_string();
	}
	if code.starts_with("ICH.C.3.") || code.starts_with("MFDS.C.3.") {
		return "C.3".to_string();
	}
	if code.starts_with("ICH.C.5.") || code.starts_with("FDA.C.5.") {
		return "C.5".to_string();
	}
	if code.starts_with("ICH.D.10.") {
		return "D.10".to_string();
	}
	if code.starts_with("ICH.D.") || code.starts_with("FDA.D.") {
		return "D".to_string();
	}
	if code.starts_with("ICH.E.") || code.starts_with("FDA.E.") {
		return "E.i".to_string();
	}
	if code.starts_with("ICH.F.") {
		return "F.r".to_string();
	}
	if code.starts_with("ICH.G.k.4.") {
		return "G.k.4.r".to_string();
	}
	if code.starts_with("ICH.G.") || code.starts_with("FDA.G.") || code.starts_with("MFDS.G.") {
		return "G.k".to_string();
	}
	if code.starts_with("ICH.H.") {
		return "H".to_string();
	}
	if code.starts_with("ICH.N.") || code.starts_with("FDA.N.") {
		return "N".to_string();
	}

	path.and_then(|value| value.split('.').next())
		.unwrap_or("unknown")
		.to_string()
}
```

Also add a unit test in the same file:

```rust
#[test]
fn resolves_validation_subsection_from_rule_code() {
	assert_eq!(
		resolve_validation_subsection("ICH.C.1.2.REQUIRED", None),
		"C.1"
	);
	assert_eq!(
		resolve_validation_subsection("FDA.C.5.5a.REQUIRED", None),
		"C.5"
	);
	assert_eq!(
		resolve_validation_subsection("ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED", None),
		"G.k.4.r"
	);
}
```

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test -p lib-core validation::case::sections::tests::resolves_validation_subsection_from_rule_code
cargo test -p web-server c_validation_issues_include_stable_subsection_metadata --test validation -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/libs/lib-core/src/validation/mod.rs crates/libs/lib-core/src/validation/case/sections/mod.rs crates/services/web-server/tests/validation/c.rs
git commit -m "feat: expose validation subsection metadata"
```

---

### Task 2: Add Section And Subsection Summaries For Red-Dot Consumers

**Files:**
- Modify: `crates/libs/lib-core/src/validation/mod.rs`
- Modify: `crates/services/web-server/tests/validation/c.rs`

- [ ] **Step 1: Write the failing summary test**

Append this test to `crates/services/web-server/tests/validation/c.rs`:

```rust
#[serial]
#[tokio::test]
async fn c_validation_report_summarizes_section_and_subsection_counts() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE safety_report_identification SET transmission_date = NULL, transmission_date_null_flavor = NULL WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;

	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	let section = report["data"]["section_summaries"]
		.as_array()
		.expect("section_summaries array")
		.iter()
		.find(|entry| entry["section"] == "case-identification")
		.expect("case-identification summary");
	assert_eq!(section["blocking_count"].as_u64(), Some(1));

	let subsection = report["data"]["subsection_summaries"]
		.as_array()
		.expect("subsection_summaries array")
		.iter()
		.find(|entry| entry["subsection"] == "C.1")
		.expect("C.1 summary");
	assert_eq!(subsection["blocking_count"].as_u64(), Some(1));
	Ok(())
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test -p web-server c_validation_report_summarizes_section_and_subsection_counts --test validation -- --nocapture
```

Expected: FAIL because the report has no `section_summaries` or `subsection_summaries`.

- [ ] **Step 3: Add report summary structs**

In `crates/libs/lib-core/src/validation/mod.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSectionSummary {
	pub section: String,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSubsectionSummary {
	pub section: String,
	pub subsection: String,
	pub blocking_count: usize,
	pub non_blocking_count: usize,
}
```

Then extend `CaseValidationReport`:

```rust
pub section_summaries: Vec<ValidationSectionSummary>,
pub subsection_summaries: Vec<ValidationSubsectionSummary>,
```

- [ ] **Step 4: Build summaries in `build_report`**

Replace the body of `build_report` after `non_blocking_count` calculation with this logic:

```rust
use std::collections::BTreeMap;

let mut by_section: BTreeMap<String, (usize, usize)> = BTreeMap::new();
let mut by_subsection: BTreeMap<(String, String), (usize, usize)> = BTreeMap::new();
for issue in &issues {
	let section_counts = by_section.entry(issue.section.clone()).or_default();
	let subsection_counts = by_subsection
		.entry((issue.section.clone(), issue.subsection.clone()))
		.or_default();
	if issue.blocking {
		section_counts.0 += 1;
		subsection_counts.0 += 1;
	} else {
		section_counts.1 += 1;
		subsection_counts.1 += 1;
	}
}
let section_summaries = by_section
	.into_iter()
	.map(|(section, (blocking_count, non_blocking_count))| {
		ValidationSectionSummary {
			section,
			blocking_count,
			non_blocking_count,
		}
	})
	.collect();
let subsection_summaries = by_subsection
	.into_iter()
	.map(|((section, subsection), (blocking_count, non_blocking_count))| {
		ValidationSubsectionSummary {
			section,
			subsection,
			blocking_count,
			non_blocking_count,
		}
	})
	.collect();
```

Return `section_summaries` and `subsection_summaries` in the `CaseValidationReport` initializer.

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test -p web-server c_validation_report_summarizes_section_and_subsection_counts --test validation -- --nocapture
cargo test -p web-server c_validation_issues_include_stable_subsection_metadata --test validation -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/libs/lib-core/src/validation/mod.rs crates/services/web-server/tests/validation/c.rs
git commit -m "feat: summarize validation issues by section"
```

---

### Task 3: Enforce Future-Date Blocking For C Section Report Dates

**Files:**
- Modify: `crates/libs/lib-core/src/validation/catalog.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/c.rs`
- Modify: `crates/services/web-server/tests/validation/c.rs`

- [ ] **Step 1: Write failing tests for C.1.2, C.1.4, and C.1.5**

Append these tests to `crates/services/web-server/tests/validation/c.rs`:

```rust
#[serial]
#[tokio::test]
async fn c_ich_c_1_2_future_date_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE safety_report_identification SET transmission_date = DATE '2999-01-01', transmission_date_null_flavor = NULL WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.2.FUTURE_DATE.FORBIDDEN");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_4_future_date_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE safety_report_identification SET date_first_received_from_source = DATE '2999-01-01', date_first_received_from_source_null_flavor = NULL WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.4.FUTURE_DATE.FORBIDDEN");
	Ok(())
}

#[serial]
#[tokio::test]
async fn c_ich_c_1_5_future_date_returns_banner_issue() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	db_exec_case_sql(
		&ctx,
		&format!(
			"UPDATE safety_report_identification SET date_of_most_recent_information = DATE '2999-01-01', date_of_most_recent_information_null_flavor = NULL WHERE case_id = '{}'",
			ctx.case_id
		),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_banner_issue(&report, "ICH.C.1.5.FUTURE_DATE.FORBIDDEN");
	Ok(())
}
```

Add these rule codes to `tested_rule_codes()` in the same file:

```rust
"ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
"ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
"ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p web-server c_ich_c_1_2_future_date_returns_banner_issue --test validation -- --nocapture
cargo test -p web-server c_ich_c_1_4_future_date_returns_banner_issue --test validation -- --nocapture
cargo test -p web-server c_ich_c_1_5_future_date_returns_banner_issue --test validation -- --nocapture
```

Expected: FAIL because the future-date rule codes are not emitted.

- [ ] **Step 3: Add catalog metadata**

In `crates/libs/lib-core/src/validation/catalog.rs`, add these entries near the existing C.1 date rules:

```rust
ValidationRuleMetadata {
	code: "ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
	profile: ValidationProfile::Ich,
	section: "case-identification",
	blocking: true,
	message: "[C.1.2] must not be later than today.",
},
ValidationRuleMetadata {
	code: "ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
	profile: ValidationProfile::Ich,
	section: "case-identification",
	blocking: true,
	message: "[C.1.4] must not be later than today.",
},
ValidationRuleMetadata {
	code: "ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
	profile: ValidationProfile::Ich,
	section: "case-identification",
	blocking: true,
	message: "[C.1.5] must not be later than today.",
},
```

- [ ] **Step 4: Add field paths**

In `crates/libs/lib-core/src/validation/case/sections/c.rs`, extend `field_path_for_rule`:

```rust
"ICH.C.1.2.FUTURE_DATE.FORBIDDEN" => {
	Some("safetyReportIdentification.transmissionDate")
}
"ICH.C.1.4.FUTURE_DATE.FORBIDDEN" => {
	Some("safetyReportIdentification.dateFirstReceivedFromSource")
}
"ICH.C.1.5.FUTURE_DATE.FORBIDDEN" => {
	Some("safetyReportIdentification.dateOfMostRecentInformation")
}
```

- [ ] **Step 5: Emit future-date issues**

In `crates/libs/lib-core/src/validation/case/sections/c.rs`, add:

```rust
fn is_future_date(value: Option<sqlx::types::time::Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	let today = sqlx::types::time::OffsetDateTime::now_utc().date();
	value > today
}
```

Inside `collect_ich_issues`, after the existing required checks for `C.1.2`, `C.1.4`, and `C.1.5`, add:

```rust
if is_future_date(report.transmission_date) {
	push_issue_by_code(
		issues,
		"ICH.C.1.2.FUTURE_DATE.FORBIDDEN",
		"safetyReportIdentification.transmissionDate",
	);
}
if is_future_date(report.date_first_received_from_source) {
	push_issue_by_code(
		issues,
		"ICH.C.1.4.FUTURE_DATE.FORBIDDEN",
		"safetyReportIdentification.dateFirstReceivedFromSource",
	);
}
if is_future_date(report.date_of_most_recent_information) {
	push_issue_by_code(
		issues,
		"ICH.C.1.5.FUTURE_DATE.FORBIDDEN",
		"safetyReportIdentification.dateOfMostRecentInformation",
	);
}
```

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test -p web-server c_ich_c_1_2_future_date_returns_banner_issue --test validation -- --nocapture
cargo test -p web-server c_ich_c_1_4_future_date_returns_banner_issue --test validation -- --nocapture
cargo test -p web-server c_ich_c_1_5_future_date_returns_banner_issue --test validation -- --nocapture
cargo test -p web-server c_rule_coverage_matches_backend_banner_contract --test validation
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/libs/lib-core/src/validation/catalog.rs crates/libs/lib-core/src/validation/case/sections/c.rs crates/services/web-server/tests/validation/c.rs
git commit -m "feat: block future C section report dates"
```

---

### Task 4: Extend Future-Date Checks To D, E, F, And G Date Fields

**Files:**
- Modify: `crates/libs/lib-core/src/validation/catalog.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/d.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/e.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/f.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/g.rs`
- Modify: `crates/services/web-server/tests/validation/d.rs`
- Modify: `crates/services/web-server/tests/validation/e.rs`
- Modify: `crates/services/web-server/tests/validation/f.rs`
- Modify: `crates/services/web-server/tests/validation/g.rs`

- [ ] **Step 1: Add catalog entries**

Add these `ValidationRuleMetadata` entries in `crates/libs/lib-core/src/validation/catalog.rs` near the matching section rules:

```rust
ValidationRuleMetadata {
	code: "ICH.D.2.1.FUTURE_DATE.FORBIDDEN",
	profile: ValidationProfile::Ich,
	section: "patient",
	blocking: true,
	message: "[D.2.1] Date of birth must not be later than today.",
},
ValidationRuleMetadata {
	code: "ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN",
	profile: ValidationProfile::Ich,
	section: "patient",
	blocking: true,
	message: "[D.7.1.r] Medical history dates must not be later than today.",
},
ValidationRuleMetadata {
	code: "ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN",
	profile: ValidationProfile::Ich,
	section: "reactions",
	blocking: true,
	message: "[E.i.4/E.i.5] Reaction dates must not be later than today.",
},
ValidationRuleMetadata {
	code: "ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
	profile: ValidationProfile::Ich,
	section: "tests",
	blocking: true,
	message: "[F.r.1] Test date must not be later than today.",
},
ValidationRuleMetadata {
	code: "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
	profile: ValidationProfile::Ich,
	section: "drugs",
	blocking: true,
	message: "[G.k.4.r.4/G.k.4.r.5] Drug administration dates must not be later than today.",
},
```

- [ ] **Step 2: Write one failing test per section**

For each section test file, add one test that uses existing helper creation functions from `validation_common.rs` and direct SQL updates when the helper does not expose the needed field:

```rust
assert_banner_issue(&report, "ICH.D.2.1.FUTURE_DATE.FORBIDDEN");
assert_banner_issue(&report, "ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN");
assert_banner_issue(&report, "ICH.F.r.1.FUTURE_DATE.FORBIDDEN");
assert_banner_issue(&report, "ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN");
```

Add the new rule code to each file's `tested_rule_codes()` array. Keep each test limited to a single date family so a failure identifies one broken section.

- [ ] **Step 3: Run the new tests to verify failure**

Run:

```bash
cargo test -p web-server future_date_returns_banner_issue --test validation -- --nocapture
```

Expected: FAIL for the new D/E/F/G tests because the section collectors do not yet emit those codes.

- [ ] **Step 4: Add a shared local helper in each touched section file**

Add this function to `d.rs`, `e.rs`, `f.rs`, and `g.rs`:

```rust
fn is_future_date(value: Option<sqlx::types::time::Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	let today = sqlx::types::time::OffsetDateTime::now_utc().date();
	value > today
}
```

- [ ] **Step 5: Emit the date issues in section collectors**

In each collector, emit the rule only when a concrete date exists and is after today:

```rust
if is_future_date(patient.birth_date) {
	push_issue_by_code(issues, "ICH.D.2.1.FUTURE_DATE.FORBIDDEN", "patient.birthDate");
}
```

```rust
if is_future_date(reaction.start_date) || is_future_date(reaction.end_date) {
	push_issue_by_code(
		issues,
		"ICH.E.i.4-5.FUTURE_DATE.FORBIDDEN",
		format!("reactions.{idx}.dateRange"),
	);
}
```

```rust
if is_future_date(test_result.test_date) {
	push_issue_by_code(
		issues,
		"ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
		format!("testResults.{idx}.testDate"),
	);
}
```

```rust
if is_future_date(dosage.first_administration_date)
	|| is_future_date(dosage.last_administration_date)
{
	push_issue_by_code(
		issues,
		"ICH.G.k.4.r.4-5.FUTURE_DATE.FORBIDDEN",
		format!("drugs.{drug_idx}.dosages.{dosage_idx}.dateRange"),
	);
}
```

Place the D-section check inside the `if let Some(patient) = validation_ctx.patient.as_ref()` block. Place the D.7 check inside the existing `validation_ctx.medical_history.iter().enumerate().for_each(|(idx, episode)| { ... })` loop.

Place the E-section check inside the existing `validation_ctx.reactions.iter().enumerate().for_each(|(idx, reaction)| { ... })` loop.

Place the F-section check inside the existing `validation_ctx.tests.iter().enumerate().for_each(|(idx, test)| { ... })` loop.

Place the G-section check inside the existing `validation_ctx.dosages.iter().enumerate().for_each(|(idx, dosage)| { ... })` loop.

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test -p web-server future_date_returns_banner_issue --test validation -- --nocapture
cargo test -p web-server d_rule_coverage_matches_backend_banner_contract --test validation
cargo test -p web-server e_rule_coverage_matches_backend_banner_contract --test validation
cargo test -p web-server f_rule_coverage_matches_backend_banner_contract --test validation
cargo test -p web-server g_rule_coverage_matches_backend_banner_contract --test validation
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/libs/lib-core/src/validation/catalog.rs crates/libs/lib-core/src/validation/case/sections/d.rs crates/libs/lib-core/src/validation/case/sections/e.rs crates/libs/lib-core/src/validation/case/sections/f.rs crates/libs/lib-core/src/validation/case/sections/g.rs crates/services/web-server/tests/validation/d.rs crates/services/web-server/tests/validation/e.rs crates/services/web-server/tests/validation/f.rs crates/services/web-server/tests/validation/g.rs
git commit -m "feat: block future clinical section dates"
```

---

### Task 5: Verify Required-Date Null Flavor Does Not Produce False Missing-Required Errors

**Files:**
- Modify: `crates/services/web-server/tests/validation/c.rs`
- Modify: `crates/services/web-server/tests/validation/d.rs`
- Modify: `crates/services/web-server/tests/validation/e.rs`
- Modify: `crates/services/web-server/tests/validation/f.rs`
- Modify: `crates/services/web-server/tests/validation/g.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/c.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/d.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/e.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/f.rs`
- Modify: `crates/libs/lib-core/src/validation/case/sections/g.rs`

- [ ] **Step 1: Add regression tests for null-flavor-only date fields**

Add tests that set date to `NULL`, set the field's null flavor to a valid value, and assert the corresponding `*.REQUIRED` code is absent. Use this existing C-section test as the pattern:

```rust
#[serial]
#[tokio::test]
async fn c_ich_c_1_2_allows_transmission_date_null_flavor() -> Result<()> {
	let ctx = setup_case().await?;
	create_safety_report(&ctx.app, &ctx.cookie, ctx.case_id, false).await?;
	update_safety_report(
		&ctx.app,
		&ctx.cookie,
		ctx.case_id,
		json!({"data": {
			"transmission_date_null_flavor": "UNK"
		}}),
	)
	.await?;
	let report = validate_case(&ctx.app, &ctx.cookie, ctx.case_id, "ich").await?;
	assert_lacks_code(&report, "ICH.C.1.2.REQUIRED");
	Ok(())
}
```

Add equivalent tests for:

```rust
ICH.C.1.4.REQUIRED
ICH.C.1.5.REQUIRED
ICH.D.2.BIRTHTIME.NULLFLAVOR.REQUIRED
ICH.E.i.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED
ICH.F.r.1.REQUIRED
ICH.G.k.4.r.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED
```

- [ ] **Step 2: Run tests to classify current behavior**

Run:

```bash
cargo test -p web-server allows_ --test validation -- --nocapture
```

Expected: Existing passing tests stay green. New tests expose any section that still treats valid null flavor as a missing required date.

- [ ] **Step 3: Fix sections that fail the null-flavor regression tests**

For each failing section, change the collector to call `push_issue_if_rule_invalid` with both `value_code` and the matching `null_flavor` instead of checking only whether the date is present. For example:

```rust
let value = item.test_date.map(|date| date.to_string());
let _ = push_issue_if_rule_invalid(
	issues,
	"ICH.F.r.1.REQUIRED",
	format!("testResults.{idx}.testDate"),
	value.as_deref(),
	item.test_date_null_flavor.as_deref(),
	RuleFacts::default(),
);
```

- [ ] **Step 4: Run focused tests**

Run:

```bash
cargo test -p web-server allows_ --test validation -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/services/web-server/tests/validation crates/libs/lib-core/src/validation/case/sections
git commit -m "test: cover required date null flavor validation"
```

---

### Task 6: Final Verification And Requirement Status Update

**Files:**
- Modify: `docs/requirements/client_requirements_todo.md`

- [ ] **Step 1: Run full validation test suite**

Run:

```bash
cargo fmt --all
cargo test -p lib-core validation -- --nocapture
cargo test -p web-server --test validation -- --nocapture --test-threads=1
cargo test -p web-server case_validation_web --test api -- --nocapture --test-threads=1
```

Expected: PASS.

- [ ] **Step 2: Update the requirement checklist**

In `docs/requirements/client_requirements_todo.md`, change this item:

```markdown
- [ ] Make validation warnings reliable at both section and subsection level so red dots and required indicators match real errors.
```

to:

```markdown
- [-] Make validation warnings reliable at both section and subsection level so red dots and required indicators match real errors. Backend validation now exposes stable `section`, `subsection`, `field_path`, and section/subsection issue counts; frontend red-dot rendering still needs UAT against the client screens.
```

Change this item:

```markdown
- [ ] Ensure date pickers are consistently English, support partial/UK-style requirements where applicable, and block future dates where required.
```

to:

```markdown
- [-] Ensure date pickers are consistently English, support partial/UK-style requirements where applicable, and block future dates where required. Backend case validation blocks future dates for covered C/D/E/F/G fields; date-picker locale and partial-date UI behavior remain frontend work.
```

Keep the null-flavor checklist item open unless Task 5 confirms all sampled required-date null-flavor regressions pass across the touched sections.

- [ ] **Step 3: Commit**

```bash
git add docs/requirements/client_requirements_todo.md
git commit -m "docs: update validation re-audit status"
```

---

## Self-Review

- Spec coverage: This plan covers the backend validation-engine slice: null flavor regression coverage, future-date blocking, field paths, section metadata, and subsection metadata. Regional field rendering, MedDRA/WHO-Drug/UCUM UX, and frontend date picker locale are deliberately excluded because they touch separate UI and terminology flows.
- Placeholder scan: The plan contains concrete file paths, rule codes, command lines, and code snippets for each production change.
- Type consistency: The plan uses the existing `ValidationIssue`, `CaseValidationReport`, `push_issue_by_code`, `push_issue_if_rule_invalid`, `ValidationRuleMetadata`, and section collector patterns already present in the repo.
