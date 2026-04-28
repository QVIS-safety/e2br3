use crate::validation::{
	has_test_payload, has_text, push_issue_by_code,
	push_issue_if_conditioned_value_invalid, RuleFacts, ValidationContext,
	ValidationIssue, ValidationProfile,
};

fn is_future_date(value: Option<sqlx::types::time::Date>) -> bool {
	let Some(value) = value else {
		return false;
	};
	let today = sqlx::types::time::OffsetDateTime::now_utc().date();
	value > today
}

pub(crate) fn collect(
	issues: &mut Vec<ValidationIssue>,
	profile: ValidationProfile,
	validation_ctx: &ValidationContext,
) {
	let _ = profile;
	collect_ich_issues(validation_ctx, issues);
}

pub(crate) fn collect_ich_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	validation_ctx
		.tests
		.iter()
		.enumerate()
		.for_each(|(idx, test)| {
			let has_payload = has_test_payload(test);
			let _ = push_issue_if_conditioned_value_invalid(
				issues,
				"ICH.F.r.2.REQUIRED",
				"ICH.F.r.2.REQUIRED",
				"ICH.F.r.2.REQUIRED",
				format!("testResults.{idx}.testName"),
				Some(test.test_name.as_str()),
				None,
				RuleFacts {
					ich_test_payload_present: Some(has_payload),
					..RuleFacts::default()
				},
				RuleFacts::default(),
			);
			let test_date_present = test.test_date.is_some()
				|| has_text(test.test_date_null_flavor.as_deref());
			let free_text_present = has_text(Some(test.test_name.as_str()));
			let meddra_version_present =
				has_text(test.test_meddra_version.as_deref());
			let meddra_code_present = has_text(test.test_meddra_code.as_deref());
			let test_result_value_present =
				has_text(test.test_result_value.as_deref());
			let test_result_unit_present =
				has_text(test.test_result_unit.as_deref());
			let test_result_code_present =
				has_text(test.test_result_code.as_deref());
			let result_unstructured_present =
				has_text(test.result_unstructured.as_deref());
			if free_text_present && !test_date_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.1.REQUIRED",
					format!("testResults.{idx}.testDate"),
				);
			}
			if is_future_date(test.test_date) {
				push_issue_by_code(
					issues,
					"ICH.F.r.1.FUTURE_DATE.FORBIDDEN",
					format!("testResults.{idx}.testDate"),
				);
			}
			if test_date_present && !meddra_code_present && !free_text_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.2.1.REQUIRED",
					format!("testResults.{idx}.testName"),
				);
			}
			if meddra_code_present && !meddra_version_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.2.2a.REQUIRED",
					format!("testResults.{idx}.testMeddraVersion"),
				);
			}
			if test_date_present && !free_text_present && !meddra_code_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.2.2b.REQUIRED",
					format!("testResults.{idx}.testMeddraCode"),
				);
			}
			if test_result_value_present && !test_result_unit_present {
				push_issue_by_code(
					issues,
					"ICH.F.r.3.3.REQUIRED",
					format!("testResults.{idx}.testResultUnit"),
				);
			}
			if free_text_present
				&& !test_result_code_present
				&& !test_result_value_present
				&& !result_unstructured_present
			{
				push_issue_by_code(
					issues,
					"ICH.F.r.3.1.REQUIRED",
					format!("testResults.{idx}.testResultCode"),
				);
				push_issue_by_code(
					issues,
					"ICH.F.r.3.2.REQUIRED",
					format!("testResults.{idx}.testResultValue"),
				);
				push_issue_by_code(
					issues,
					"ICH.F.r.3.4.REQUIRED",
					format!("testResults.{idx}.resultUnstructured"),
				);
			}
		});
}

pub(crate) fn field_path_for_rule(code: &str) -> Option<&'static str> {
	match code {
		"ICH.F.r.1.FUTURE_DATE.FORBIDDEN" => Some("testResults.0.testDate"),
		"ICH.F.r.2.REQUIRED" | "ICH.F.r.2.1.REQUIRED" => {
			Some("testResults.0.testName")
		}
		"ICH.F.r.2.2a.REQUIRED" => Some("testResults.0.testMeddraVersion"),
		"ICH.F.r.2.2b.REQUIRED" => Some("testResults.0.testMeddraCode"),
		"ICH.F.r.3.1.REQUIRED" => Some("testResults.0.testResultCode"),
		"ICH.F.r.3.2.REQUIRED" => Some("testResults.0.testResult"),
		"ICH.F.r.3.3.REQUIRED" => Some("testResults.0.testUnit"),
		"ICH.F.r.3.4.REQUIRED" => Some("testResults.0.testResultUnstructured"),
		_ => None,
	}
}
