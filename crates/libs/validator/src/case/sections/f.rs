use super::rule_table::{
	eval_companions, eval_indexed, eval_indexed_constraints,
	eval_indexed_future_dates, eval_indexed_length, eval_indexed_meddra,
	CompanionRule, DateValues, IndexedConstraintRule, IndexedFutureDateRule,
	IndexedLengthRule, IndexedMeddraRule, IndexedRule, RuleValue,
};
use crate::allowed_value::ConstraintValue;
use crate::{
	has_test_payload, has_text, RegulatoryAuthority, RuleFacts, ValidationContext,
	ValidationIssue,
};
use lib_core::model::test_result::TestResult;
use std::borrow::Cow;

fn test_payload_facts(test: &TestResult) -> RuleFacts {
	RuleFacts {
		ich_test_payload_present: Some(has_test_payload(test)),
		..RuleFacts::default()
	}
}

const F_TEST_MEDDRA_RULES: &[IndexedMeddraRule<TestResult>] = &[IndexedMeddraRule {
	version_allowed_code: "ICH.F.r.2.2a.ALLOWED.VALUE",
	version_code: "ICH.F.r.2.2a.VOCABULARY",
	code_allowed_code: "ICH.F.r.2.2b.ALLOWED.VALUE",
	code_code: "ICH.F.r.2.2b.VOCABULARY",
	version_path: |idx| format!("testResults.{idx}.testMeddraVersion"),
	code_path: |idx| format!("testResults.{idx}.testMeddraCode"),
	values: |test| {
		(
			test.test_meddra_version.as_deref(),
			test.test_meddra_code.as_deref(),
		)
	},
}];

const F_INDEXED_RULES: &[IndexedRule<TestResult>] = &[IndexedRule {
	code: "ICH.F.r.2.REQUIRED",
	path: |idx| format!("testResults.{idx}.testName"),
	value: |test| RuleValue::borrowed(Some(test.test_name.as_str()), None),
	facts: test_payload_facts,
}];

const F_FUTURE_DATE_RULES: &[IndexedFutureDateRule<TestResult>] =
	&[IndexedFutureDateRule {
		code: "ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
		path: |idx| format!("testResults.{idx}.testDate"),
		dates: |test| DateValues::One(test.test_date),
	}];

const F_CONSTRAINT_RULES: &[IndexedConstraintRule<TestResult>] = &[
	IndexedConstraintRule {
		code: "ICH.F.r.3.3.ALLOWED.VALUE",
		path: |idx| format!("testResults.{idx}.testResultUnit"),
		value: |test| {
			ConstraintValue::Text(
				test.test_result_unit.as_deref().map(Cow::Borrowed),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.F.r.3.1.ALLOWED.VALUE",
		path: |idx| format!("testResults.{idx}.testResultCode"),
		value: |test| {
			ConstraintValue::Text(
				test.test_result_code.as_deref().map(Cow::Borrowed),
			)
		},
	},
	IndexedConstraintRule {
		code: "ICH.F.r.3.2.ALLOWED.VALUE",
		path: |idx| format!("testResults.{idx}.testResultValue"),
		value: |test| {
			ConstraintValue::Text(
				test.test_result_value.as_deref().map(Cow::Borrowed),
			)
		},
	},
];

const F_LENGTH_RULES: &[IndexedLengthRule<TestResult>] = &[
	IndexedLengthRule {
		code: "ICH.F.r.2.1.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.testName"),
		value: |test| Some(test.test_name.as_str()),
	},
	IndexedLengthRule {
		code: "ICH.F.r.2.2a.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.testMeddraVersion"),
		value: |test| test.test_meddra_version.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.F.r.2.2b.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.testMeddraCode"),
		value: |test| test.test_meddra_code.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.F.r.3.1.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.testResultCode"),
		value: |test| test.test_result_code.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.F.r.3.2.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.testResultValue"),
		value: |test| test.test_result_value.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.F.r.3.3.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.testResultUnit"),
		value: |test| test.test_result_unit.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.F.r.3.4.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.resultUnstructured"),
		value: |test| test.result_unstructured.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.F.r.4.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.normalLowValue"),
		value: |test| test.normal_low_value.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.F.r.5.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.normalHighValue"),
		value: |test| test.normal_high_value.as_deref(),
	},
	IndexedLengthRule {
		code: "ICH.F.r.6.LENGTH.MAX",
		path: |idx| format!("testResults.{idx}.comments"),
		value: |test| test.comments.as_deref(),
	},
];

const F_COMPANION_RULES: &[CompanionRule<TestResult>] = &[
	CompanionRule {
		code: "ICH.F.r.1.REQUIRED",
		path: |idx| format!("testResults.{idx}.testDate"),
		trigger: |test| has_text(Some(test.test_name.as_str())),
		required: |test| {
			test.test_date.is_some()
				|| has_text(test.test_date_null_flavor.as_deref())
		},
	},
	CompanionRule {
		code: "ICH.F.r.2.1.REQUIRED",
		path: |idx| format!("testResults.{idx}.testName"),
		trigger: |test| {
			test.test_date.is_some() && !has_text(test.test_meddra_code.as_deref())
		},
		required: |test| has_text(Some(test.test_name.as_str())),
	},
	CompanionRule {
		code: "ICH.F.r.2.2a.REQUIRED",
		path: |idx| format!("testResults.{idx}.testMeddraVersion"),
		trigger: |test| has_text(test.test_meddra_code.as_deref()),
		required: |test| has_text(test.test_meddra_version.as_deref()),
	},
	CompanionRule {
		code: "ICH.F.r.2.2b.REQUIRED",
		path: |idx| format!("testResults.{idx}.testMeddraCode"),
		trigger: |test| {
			test.test_date.is_some() && !has_text(Some(test.test_name.as_str()))
		},
		required: |test| has_text(test.test_meddra_code.as_deref()),
	},
	CompanionRule {
		code: "ICH.F.r.3.3.REQUIRED",
		path: |idx| format!("testResults.{idx}.testResultUnit"),
		trigger: |test| has_text(test.test_result_value.as_deref()),
		required: |test| has_text(test.test_result_unit.as_deref()),
	},
	CompanionRule {
		code: "ICH.F.r.3.1.REQUIRED",
		path: |idx| format!("testResults.{idx}.testResultCode"),
		trigger: |test| {
			has_text(Some(test.test_name.as_str()))
				&& !has_text(test.test_result_value.as_deref())
				&& !has_text(test.result_unstructured.as_deref())
		},
		required: |test| has_text(test.test_result_code.as_deref()),
	},
	CompanionRule {
		code: "ICH.F.r.3.2.REQUIRED",
		path: |idx| format!("testResults.{idx}.testResultValue"),
		trigger: |test| {
			has_text(Some(test.test_name.as_str()))
				&& !has_text(test.test_result_code.as_deref())
				&& !has_text(test.result_unstructured.as_deref())
		},
		required: |test| has_text(test.test_result_value.as_deref()),
	},
	CompanionRule {
		code: "ICH.F.r.3.4.REQUIRED",
		path: |idx| format!("testResults.{idx}.resultUnstructured"),
		trigger: |test| {
			has_text(Some(test.test_name.as_str()))
				&& !has_text(test.test_result_code.as_deref())
				&& !has_text(test.test_result_value.as_deref())
		},
		required: |test| has_text(test.result_unstructured.as_deref()),
	},
];

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	authority: RegulatoryAuthority,
	validation_ctx: &ValidationContext,
) {
	let _ = authority;
	collect_ich_issues(validation_ctx, issues);
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	eval_indexed(issues, &validation_ctx.tests, F_INDEXED_RULES);
	eval_companions(issues, &validation_ctx.tests, F_COMPANION_RULES);
	eval_indexed_future_dates(issues, &validation_ctx.tests, F_FUTURE_DATE_RULES);
	eval_indexed_constraints(
		issues,
		&validation_ctx.tests,
		F_CONSTRAINT_RULES,
		&validation_ctx.vocabulary,
	);
	eval_indexed_length(issues, &validation_ctx.tests, F_LENGTH_RULES);
	eval_indexed_meddra(
		issues,
		&validation_ctx.vocabulary,
		&validation_ctx.tests,
		F_TEST_MEDDRA_RULES,
	);
}

#[cfg(test)]
pub(super) fn constraint_rule_codes() -> Vec<&'static str> {
	F_CONSTRAINT_RULES
		.iter()
		.map(|rule| rule.code)
		.chain(super::rule_table::indexed_meddra_constraint_codes(
			F_TEST_MEDDRA_RULES,
		))
		.collect()
}

#[cfg(test)]
pub(super) fn table_rule_codes() -> Vec<&'static str> {
	let mut codes = Vec::new();
	codes.extend(super::rule_table::table_rule_codes(F_INDEXED_RULES));
	codes.extend(super::rule_table::table_rule_codes(F_FUTURE_DATE_RULES));
	codes.extend(super::rule_table::table_rule_codes(F_CONSTRAINT_RULES));
	codes.extend(super::rule_table::table_rule_codes(F_LENGTH_RULES));
	codes.extend(super::rule_table::table_rule_codes(F_COMPANION_RULES));
	codes.extend(super::rule_table::indexed_meddra_rule_codes(
		F_TEST_MEDDRA_RULES,
	));
	codes
}

#[cfg(test)]
mod golden_f_required_tests {
	use super::*;
	use lib_core::model::case::Case;
	use lib_core::model::test_result::TestResult;
	use sqlx::types::time::{Date, OffsetDateTime};
	use sqlx::types::Uuid;
	use time::Month;

	fn dummy_case() -> Case {
		Case {
			id: Uuid::nil(),
			organization_id: Uuid::nil(),
			dg_prd_key: None,
			status: String::new(),
			review_receivers_json: None,
			workflow_routes_json: None,
			workflow_status: String::new(),
			workflow_assigned_role: None,
			workflow_assigned_user_id: None,
			workflow_due_at: None,
			workflow_description: None,
			workflow_updated_at: OffsetDateTime::UNIX_EPOCH,
			mfds_report_type: None,
			fda_report_type: None,
			report_year: None,
			created_by: Uuid::nil(),
			updated_by: None,
			submitted_by: None,
			submitted_at: None,
			raw_xml: None,
			dirty_c: false,
			dirty_d: false,
			dirty_e: false,
			dirty_f: false,
			dirty_g: false,
			dirty_h: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
		}
	}

	fn empty_ctx() -> ValidationContext {
		ValidationContext {
			vocabulary: Default::default(),
			case: dummy_case(),
			safety_report: None,
			message_header: None,
			sender: None,
			patient: None,
			narrative: None,
			sender_diagnoses: Vec::new(),
			case_summaries: Vec::new(),
			medical_history: Vec::new(),
			past_drugs: Vec::new(),
			death_info: None,
			reported_causes_of_death: Vec::new(),
			autopsy_causes_of_death: Vec::new(),
			parents: Vec::new(),
			parent_medical_history: Vec::new(),
			parent_past_drugs: Vec::new(),
			primary_sources: Vec::new(),
			documents_held_by_sender: Vec::new(),
			literature_references: Vec::new(),
			other_case_identifiers: Vec::new(),
			linked_report_numbers: Vec::new(),
			studies: Vec::new(),
			study_registrations: Vec::new(),
			reactions: Vec::new(),
			tests: Vec::new(),
			drugs: Vec::new(),
			active_substances: Vec::new(),
			indications: Vec::new(),
			dosages: Vec::new(),
			drug_reaction_assessments: Vec::new(),
			relatedness_assessments: Vec::new(),
			patient_identifiers: Vec::new(),
		}
	}

	fn test_result() -> TestResult {
		TestResult {
			id: Uuid::nil(),
			case_id: Uuid::nil(),
			sequence_number: 1,
			test_date: None,
			test_date_null_flavor: None,
			test_name: String::new(),
			test_meddra_version: None,
			test_meddra_code: None,
			test_result_code: None,
			test_result_value: None,
			test_result_unit: None,
			result_unstructured: None,
			normal_low_value: None,
			normal_high_value: None,
			comments: None,
			more_info_available: None,
			deleted: false,
			created_at: OffsetDateTime::UNIX_EPOCH,
			updated_at: OffsetDateTime::UNIX_EPOCH,
			created_by: Uuid::nil(),
			updated_by: None,
		}
	}

	fn codes_for(test: TestResult) -> Vec<String> {
		let mut ctx = empty_ctx();
		ctx.tests.push(test);
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		issues.into_iter().map(|issue| issue.code).collect()
	}

	fn length_issue(code: &str, path: &str) -> (String, String) {
		(code.to_string(), path.to_string())
	}

	fn length_issues_for(test: TestResult) -> Vec<(String, String)> {
		let mut ctx = empty_ctx();
		ctx.tests.push(test);
		let mut issues = Vec::new();
		collect_ich_issues(&ctx, &mut issues);
		let mut out = issues
			.into_iter()
			.filter(|issue| issue.code.contains(".LENGTH.MAX"))
			.map(|issue| (issue.code, issue.path))
			.collect::<Vec<_>>();
		out.sort();
		out
	}

	#[test]
	fn allowed_value_rule_flags_invalid_test_result_code() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();
		test.test_date =
			Some(Date::from_calendar_date(2020, Month::January, 1).unwrap());
		test.test_result_code = Some("9".to_string());

		assert!(codes_for(test).contains(&"ICH.F.r.3.1.ALLOWED.VALUE".to_string()));
	}

	#[test]
	fn numeric_rule_flags_non_numeric_test_result_value() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();
		test.test_date =
			Some(Date::from_calendar_date(2020, Month::January, 1).unwrap());
		test.test_result_value = Some("not-numeric".to_string());

		assert!(codes_for(test).contains(&"ICH.F.r.3.2.ALLOWED.VALUE".to_string()));
	}

	#[test]
	fn empty_test_result_is_silent() {
		assert!(codes_for(test_result()).is_empty());
	}

	#[test]
	fn test_payload_without_name_flags_test_name() {
		let mut test = test_result();
		test.test_result_code = Some("1".to_string());

		assert_eq!(codes_for(test), vec!["ICH.F.r.2.REQUIRED".to_string()]);
	}

	#[test]
	fn test_name_without_date_flags_date_and_result_group() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();

		assert_eq!(
			codes_for(test),
			vec![
				"ICH.F.r.1.REQUIRED".to_string(),
				"ICH.F.r.3.1.REQUIRED".to_string(),
				"ICH.F.r.3.2.REQUIRED".to_string(),
				"ICH.F.r.3.4.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn test_date_without_name_or_meddra_code_flags_name_variants() {
		let mut test = test_result();
		test.test_date =
			Some(Date::from_calendar_date(2020, Month::January, 1).unwrap());

		assert_eq!(
			codes_for(test),
			vec![
				"ICH.F.r.2.REQUIRED".to_string(),
				"ICH.F.r.2.1.REQUIRED".to_string(),
				"ICH.F.r.2.2b.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn meddra_code_without_version_flags_version() {
		let mut test = test_result();
		test.test_meddra_code = Some("10000001".to_string());

		assert_eq!(codes_for(test), vec!["ICH.F.r.2.2a.REQUIRED".to_string()]);
	}

	#[test]
	fn result_value_without_unit_flags_unit() {
		let mut test = test_result();
		test.test_name = "ALT".to_string();
		test.test_result_value = Some("15".to_string());

		assert_eq!(
			codes_for(test),
			vec![
				"ICH.F.r.1.REQUIRED".to_string(),
				"ICH.F.r.3.3.REQUIRED".to_string(),
			]
		);
	}

	#[test]
	fn max_length_rules_cover_f_test_result_text_fields() {
		let mut test = test_result();
		test.test_name = "T".repeat(251);
		test.test_meddra_version = Some("V".repeat(5));
		test.test_meddra_code = Some("M".repeat(9));
		test.test_result_code = Some("RC".to_string());
		test.test_result_value = Some("V".repeat(51));
		test.test_result_unit = Some("U".repeat(51));
		test.result_unstructured = Some("R".repeat(2001));
		test.normal_low_value = Some("L".repeat(51));
		test.normal_high_value = Some("H".repeat(51));
		test.comments = Some("C".repeat(2001));

		assert_eq!(
			length_issues_for(test),
			vec![
				length_issue("ICH.F.r.2.1.LENGTH.MAX", "testResults.0.testName"),
				length_issue(
					"ICH.F.r.2.2a.LENGTH.MAX",
					"testResults.0.testMeddraVersion"
				),
				length_issue(
					"ICH.F.r.2.2b.LENGTH.MAX",
					"testResults.0.testMeddraCode"
				),
				length_issue(
					"ICH.F.r.3.1.LENGTH.MAX",
					"testResults.0.testResultCode"
				),
				length_issue(
					"ICH.F.r.3.2.LENGTH.MAX",
					"testResults.0.testResultValue"
				),
				length_issue(
					"ICH.F.r.3.3.LENGTH.MAX",
					"testResults.0.testResultUnit"
				),
				length_issue(
					"ICH.F.r.3.4.LENGTH.MAX",
					"testResults.0.resultUnstructured"
				),
				length_issue("ICH.F.r.4.LENGTH.MAX", "testResults.0.normalLowValue"),
				length_issue(
					"ICH.F.r.5.LENGTH.MAX",
					"testResults.0.normalHighValue"
				),
				length_issue("ICH.F.r.6.LENGTH.MAX", "testResults.0.comments"),
			]
		);
	}
}
