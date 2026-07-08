use super::rule_table::{
	eval_companions, eval_indexed, CompanionRule, IndexedRule, RuleValue,
};
use crate::{
	has_test_payload, has_text, push_issue_by_code, RegulatoryAuthority, RuleFacts,
	ValidationContext, ValidationIssue,
};
use lib_core::model::test_result::TestResult;

fn is_future_date(value: Option<sqlx::types::time::Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	let today = sqlx::types::time::OffsetDateTime::now_utc().date();
	value > today
}

fn test_payload_facts(test: &TestResult) -> RuleFacts {
	RuleFacts {
		ich_test_payload_present: Some(has_test_payload(test)),
		..RuleFacts::default()
	}
}

const F_INDEXED_RULES: &[IndexedRule<TestResult>] = &[IndexedRule {
	code: "ICH.F.r.2.REQUIRED",
	path: |idx| format!("testResults.{idx}.testName"),
	value: |test| RuleValue::borrowed(Some(test.test_name.as_str()), None),
	facts: test_payload_facts,
}];

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
	validation_ctx
		.tests
		.iter()
		.enumerate()
		.for_each(|(idx, test)| {
			if is_future_date(test.test_date) {
				push_issue_by_code(
					issues,
					"ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
					format!("testResults.{idx}.testDate"),
				);
			}
		});
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
			other_case_identifiers: Vec::new(),
			studies: Vec::new(),
			reactions: Vec::new(),
			tests: Vec::new(),
			drugs: Vec::new(),
			active_substances: Vec::new(),
			indications: Vec::new(),
			dosages: Vec::new(),
			drug_reaction_assessments: Vec::new(),
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

	#[test]
	fn empty_test_result_is_silent() {
		assert!(codes_for(test_result()).is_empty());
	}

	#[test]
	fn test_payload_without_name_flags_test_name() {
		let mut test = test_result();
		test.test_result_code = Some("123".to_string());

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
}
