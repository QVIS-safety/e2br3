use crate::common::Result;
use lib_core::xml::validate::{
	rule_layer_contract::rule_layer_contract, ValidationProfile, VALIDATION_RULES,
};

#[test]
fn l4_runtime_layer_shape_is_present() -> Result<()> {
	let has_xsd = [
		"ICH.XML.ROOT.ITSVERSION.REQUIRED",
		"ICH.XML.ROOT.SCHEMALOCATION.REQUIRED",
		"ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN",
	]
	.into_iter()
	.all(|code| rule_layer_contract(code).map(|c| c.xsd).unwrap_or(false));
	let has_ich = VALIDATION_RULES
		.iter()
		.any(|rule| rule.profile == ValidationProfile::Ich);
	let has_fda = VALIDATION_RULES
		.iter()
		.any(|rule| rule.profile == ValidationProfile::Fda);
	let has_mfds = VALIDATION_RULES
		.iter()
		.any(|rule| rule.profile == ValidationProfile::Mfds);

	assert!(has_xsd, "xsd layer matrix is required");
	assert!(has_ich, "ich layer matrix is required");
	assert!(has_fda, "fda layer matrix is required");
	assert!(has_mfds, "mfds layer matrix is required");
	Ok(())
}
