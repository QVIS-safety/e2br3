use crate::common::Result;
use lib_core::xml::validate::rule_test_matrix::{
	RuleLayer, CASE_RULE_TEST_MATRIX, XSD_RULE_TEST_MATRIX,
};

#[test]
fn l4_matrix_layer_shape_is_present() -> Result<()> {
	let has_xsd = XSD_RULE_TEST_MATRIX
		.iter()
		.any(|spec| spec.layer == RuleLayer::Xsd);
	let has_ich = CASE_RULE_TEST_MATRIX
		.iter()
		.any(|spec| spec.layer == RuleLayer::Ich);
	let has_fda = CASE_RULE_TEST_MATRIX
		.iter()
		.any(|spec| spec.layer == RuleLayer::Fda);
	let has_mfds = CASE_RULE_TEST_MATRIX
		.iter()
		.any(|spec| spec.layer == RuleLayer::Mfds);

	assert!(has_xsd, "xsd layer matrix is required");
	assert!(has_ich, "ich layer matrix is required");
	assert!(has_fda, "fda layer matrix is required");
	assert!(has_mfds, "mfds layer matrix is required");
	Ok(())
}
