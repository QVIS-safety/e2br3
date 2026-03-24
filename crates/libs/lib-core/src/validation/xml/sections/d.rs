use crate::validation::{
	AttrNullFlavorPairRuleSpec, AttrOrNullFlavorRequiredRuleSpec,
	TextNullFlavorPairRuleSpec, ValueNodeRuleSpec,
};
use crate::xml::types::XmlValidationError;
use libxml::xpath::Context;

pub(crate) const ICH_D_IDENTITY_ATTR_NULL_FLAVOR_RULES: &[AttrNullFlavorPairRuleSpec] = &[
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:primaryRole//hl7:birthTime",
		value_attr: "value",
		required_code: "ICH.D.2.BIRTHTIME.NULLFLAVOR.REQUIRED",
		required_message: "birthTime missing value; nullFlavor is required",
		forbidden_code: Some("ICH.D.2.BIRTHTIME.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"birthTime has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_D_PROFILE_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:associatedPerson//hl7:name/*",
		required_code: "ICH.D.PARENT.NAME.NULLFLAVOR.REQUIRED",
		required_message: "associatedPerson name element is empty; nullFlavor is required",
		forbidden_code: Some("ICH.D.PARENT.NAME.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"associatedPerson name element has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_D_PROFILE_ATTR_NULL_FLAVOR_RULES: &[AttrNullFlavorPairRuleSpec] = &[
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:associatedPerson//hl7:birthTime",
		value_attr: "value",
		required_code: "ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.REQUIRED",
		required_message: "associatedPerson birthTime missing value; nullFlavor is required",
		forbidden_code: Some("ICH.D.PARENT.BIRTHTIME.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"associatedPerson birthTime has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_D_PROFILE_ATTR_OR_NULL_RULES: &[AttrOrNullFlavorRequiredRuleSpec] = &[
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath:
			"//hl7:primaryRole//hl7:effectiveTime//hl7:low | //hl7:primaryRole//hl7:effectiveTime//hl7:high",
		value_attr: "value",
		required_code: "ICH.D.EFFECTIVETIME.LOW_HIGH.NULLFLAVOR.REQUIRED",
		required_message:
			"patient effectiveTime low/high missing value; nullFlavor is required",
	},
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath: "//hl7:administrativeGenderCode",
		value_attr: "code",
		required_code: "ICH.D.5.SEX.CONDITIONAL",
		required_message: "administrativeGenderCode missing code; nullFlavor is required",
	},
];

pub(crate) const ICH_D_MEDICAL_HISTORY_RULE_CODE: &str = "ICH.D.7.2.CONDITIONAL";
pub(crate) const ICH_D_MEDICAL_HISTORY_RULE_MESSAGE: &str =
	"D.7.2 must be provided when D.7.1.r.1b is not provided";

pub(crate) const FDA_D_STATIC_VALUE_NODE_RULES: &[ValueNodeRuleSpec] = &[
	ValueNodeRuleSpec {
		xpath: "//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C17049' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value",
		value_attr: "code",
		rule_code: "FDA.D.11.REQUIRED",
		fallback_message: "FDA.D.11 patient race missing code; nullFlavor is required",
	},
	ValueNodeRuleSpec {
		xpath: "//hl7:primaryRole/hl7:subjectOf2/hl7:observation[hl7:code[@code='C16564' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value",
		value_attr: "code",
		rule_code: "FDA.D.12.REQUIRED",
		fallback_message:
			"FDA.D.12 patient ethnicity missing code; nullFlavor is required",
	},
];

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
	crate::validation::xml::ich_profile::collect_ich_identity_text_errors(
		xpath,
		&mut collected,
	);
	crate::validation::xml::ich_profile::collect_ich_profile_value_presence_errors(
		xpath,
		&mut collected,
	);
	crate::validation::xml::ich_profile::collect_ich_case_history_errors(
		xpath,
		&mut collected,
	);
	crate::validation::xml::fda_profile::collect_fda_profile_errors(
		xpath,
		&mut collected,
	);
	drain_section_errors(collected, 'D', errors);
}
