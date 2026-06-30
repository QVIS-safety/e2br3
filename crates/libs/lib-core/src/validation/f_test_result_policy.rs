use crate::model::test_result::TestResult;

// Shared Section F policy used by exporter + case validators.

pub fn has_test_name(test: &TestResult) -> bool {
	!test.test_name.trim().is_empty()
}

pub fn has_test_payload(test: &TestResult) -> bool {
	has_test_name(test)
		|| test.test_date.is_some()
		|| super::has_text(test.test_result_code.as_deref())
		|| super::has_text(test.test_result_value.as_deref())
		|| super::has_text(test.result_unstructured.as_deref())
}

#[cfg(test)]
mod tests {
	use super::*;
	use sqlx::types::Uuid;
	use time::OffsetDateTime;

	fn empty_test_result() -> TestResult {
		TestResult {
			id: Uuid::new_v4(),
			case_id: Uuid::new_v4(),
			sequence_number: 1,
			deleted: false,
			test_date: None,
			test_date_null_flavor: None,
			test_name: "".to_string(),
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
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	#[test]
	fn test_payload_false_when_empty() {
		let test = empty_test_result();
		assert!(!has_test_payload(&test));
	}

	#[test]
	fn test_payload_true_with_name() {
		let mut test = empty_test_result();
		test.test_name = "LFT".to_string();
		assert!(has_test_payload(&test));
	}
}
