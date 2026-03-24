use crate::validation::{
	RequiredAttrsRuleSpec, SupportedXsiTypesRuleSpec,
	TypedChildrenAttrsOrNullFlavorRuleSpec,
};
use crate::xml::types::XmlValidationError;
use libxml::xpath::Context;

pub(crate) const ICH_F_STRUCTURAL_REQUIRED_ATTRS_RULES: &[RequiredAttrsRuleSpec] = &[
	RequiredAttrsRuleSpec {
		node_xpath: "//hl7:organizer[hl7:code[@code='3']]/hl7:component/hl7:observation/hl7:value[@xsi:type='PQ']",
		required_attrs: &["value", "unit"],
		rule_code: "ICH.XML.TESTRESULT.PQ.VALUE_UNIT.REQUIRED",
		fallback_message: "PQ must include value and unit",
	},
];

pub(crate) const ICH_F_STRUCTURAL_TYPED_CHILDREN_RULES: &[TypedChildrenAttrsOrNullFlavorRuleSpec] =
	&[TypedChildrenAttrsOrNullFlavorRuleSpec {
		node_xpath:
			"//hl7:organizer[hl7:code[@code='3']]/hl7:component/hl7:observation/hl7:value",
		required_xsi_type: "IVL_PQ",
		child_names: &["low", "high", "center"],
		required_attrs: &["value", "unit"],
		component_required_rule_code: "ICH.XML.TESTRESULT.IVL_PQ.COMPONENT.REQUIRED",
		component_required_message: "IVL_PQ must include low/high/center",
		attr_rule_code: "ICH.XML.TESTRESULT.IVL_PQ.VALUE_UNIT.REQUIRED",
		attr_rule_message: "IVL_PQ low/high/center must include value and unit",
	}];

pub(crate) const ICH_F_STRUCTURAL_SUPPORTED_XSI_TYPES_RULES: &[SupportedXsiTypesRuleSpec] =
	&[SupportedXsiTypesRuleSpec {
		node_xpath:
			"//hl7:organizer[hl7:code[@code='3']]/hl7:component/hl7:observation/hl7:value",
		allowed_types: &["IVL_PQ", "PQ", "ED", "ST", "BL", "CE"],
		rule_code: "ICH.XML.TESTRESULT.XSI_TYPE.UNSUPPORTED",
		fallback_message_prefix: "Unsupported test result xsi:type",
	}];

fn drain_section_errors(
	collected: Vec<XmlValidationError>,
	section_letter: char,
	errors: &mut Vec<XmlValidationError>,
) {
	errors.extend(collected.into_iter().filter(|error| {
		super::error_owned_by_section_letter(error, section_letter)
	}));
}

pub(crate) fn collect(xpath: &mut Context, errors: &mut Vec<XmlValidationError>) {
	let mut collected = Vec::new();
	crate::validation::xml::ich_profile::collect_ich_profile_value_presence_errors(
		xpath,
		&mut collected,
	);
	crate::validation::xml::ich_profile::collect_ich_structural_value_errors(
		xpath,
		&mut collected,
	);
	drain_section_errors(collected, 'F', errors);
}
