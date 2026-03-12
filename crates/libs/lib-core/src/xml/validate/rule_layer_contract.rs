use super::{
	canonical_rules_for_phase, find_canonical_rule_for_phase, RuleCategory,
	ValidationPhase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleLayerContract {
	pub case_validator: bool,
	pub xml_business: bool,
	pub xsd: bool,
}

pub fn rule_layer_contract(code: &str) -> Option<RuleLayerContract> {
	let is_case = canonical_rules_for_phase(ValidationPhase::CaseValidate)
		.into_iter()
		.any(|rule| {
			rule.category == RuleCategory::CaseBusiness && rule.code == code
		});
	let is_xsd = matches!(
		code,
		"ICH.XML.ROOT.ITSVERSION.REQUIRED"
			| "ICH.XML.ROOT.SCHEMALOCATION.REQUIRED"
			| "ICH.XML.PLACEHOLDER.VALUE.FORBIDDEN"
	);
	let is_import_blocking =
		find_canonical_rule_for_phase(code, ValidationPhase::Import)
			.map(|rule| rule.blocking)
			.unwrap_or(false);

	if is_case || is_xsd || is_import_blocking {
		return Some(RuleLayerContract {
			case_validator: is_case,
			xml_business: is_import_blocking && !is_xsd,
			xsd: is_xsd,
		});
	}
	None
}
