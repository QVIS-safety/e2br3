use lib_core::model::test_result::TestResult;

// Shared Section F policy used by exporter + case validators.

pub fn has_test_name(test: &TestResult) -> bool {
	!test.test_name.trim().is_empty()
		|| super::has_text(test.test_meddra_code.as_deref())
}

pub fn has_test_payload(test: &TestResult) -> bool {
	has_test_name(test)
		|| test.test_date.is_some()
		|| super::has_text(test.test_meddra_code.as_deref())
		|| super::has_text(test.test_result_code.as_deref())
		|| super::has_text(test.test_result_value.as_deref())
		|| super::has_text(test.result_unstructured.as_deref())
}

#[cfg(test)]
#[path = "tests/f_test_result_policy.rs"]
mod tests;
