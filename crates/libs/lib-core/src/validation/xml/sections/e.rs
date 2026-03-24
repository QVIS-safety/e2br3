use crate::validation::{
	AttrNullFlavorPairRuleSpec, AttrOrNullFlavorRequiredRuleSpec,
	TextNullFlavorPairRuleSpec, ValueNodeRuleSpec,
};
use crate::xml::types::XmlValidationError;
use libxml::xpath::Context;

pub(crate) const ICH_E_PROFILE_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:observation[hl7:code[@code='30']]/hl7:value[@xsi:type='ED']",
		required_code: "ICH.E.i.1.2.NULLFLAVOR.REQUIRED",
		required_message: "reaction translation missing value; nullFlavor is required",
		forbidden_code: Some("ICH.E.i.1.2.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"reaction translation has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_E_PROFILE_ATTR_NULL_FLAVOR_RULES: &[AttrNullFlavorPairRuleSpec] = &[
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:outboundRelationship[@typeCode='SPRT']/hl7:relatedInvestigation/hl7:code",
		value_attr: "code",
		required_code: "ICH.E.i.0.RELATIONSHIP.CODE.NULLFLAVOR.REQUIRED",
		required_message: "relatedInvestigation/code missing code; nullFlavor is required",
		forbidden_code: Some("ICH.E.i.0.RELATIONSHIP.CODE.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"relatedInvestigation/code has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:observation[hl7:code[@code='27']]/hl7:value",
		value_attr: "code",
		required_code: "ICH.E.i.7.NULLFLAVOR.REQUIRED",
		required_message: "reaction outcome value missing code; nullFlavor is required",
		forbidden_code: Some("ICH.E.i.7.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"reaction outcome value has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:observation[hl7:code[@code='29']]/hl7:value",
		value_attr: "code",
		required_code: "ICH.E.i.2.NULLFLAVOR.REQUIRED",
		required_message: "reaction term missing code; nullFlavor is required",
		forbidden_code: Some("ICH.E.i.2.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"reaction term has code and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_E_PROFILE_ATTR_OR_NULL_RULES: &[AttrOrNullFlavorRequiredRuleSpec] = &[
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath:
			"//hl7:observation[hl7:code[@code='29']]/hl7:effectiveTime/hl7:low | //hl7:observation[hl7:code[@code='29']]/hl7:effectiveTime/hl7:high",
		value_attr: "value",
		required_code: "ICH.E.i.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		required_message:
			"reaction effectiveTime low/high missing value; nullFlavor is required",
	},
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath: "//hl7:locatedPlace/hl7:code",
		value_attr: "code",
		required_code: "ICH.E.i.9.COUNTRY.NULLFLAVOR.REQUIRED",
		required_message: "reaction country missing code; nullFlavor is required",
	},
];

pub(crate) const ICH_E_REACTION_TEMPORAL_RULE_CODE: &str = "ICH.E.i.4-6.CONDITIONAL";
pub(crate) const ICH_E_REACTION_TEMPORAL_RULE_MESSAGE: &str =
	"Reaction requires start, end, or duration";

pub(crate) const FDA_E_STATIC_VALUE_NODE_RULES: &[ValueNodeRuleSpec] = &[
	ValueNodeRuleSpec {
		xpath: "//hl7:observation[hl7:code[@code='29' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]//hl7:outboundRelationship2/hl7:observation[(hl7:code[@code='7' and @codeSystem='2.16.840.1.113883.3.989.5.1.2.2.1.3']) or (hl7:code[@code='726' and @codeSystem='2.16.840.1.113883.3.989.5.1.2.2.1.32'])]/hl7:value",
		value_attr: "value",
		rule_code: "FDA.E.i.3.2h.REQUIRED",
		fallback_message:
			"FDA.E.i.3.2h required intervention missing value; nullFlavor is required",
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
	crate::validation::xml::ich_profile::collect_ich_profile_value_presence_errors(
		xpath,
		&mut collected,
	);
	crate::validation::xml::ich_profile::collect_ich_structural_value_errors(
		xpath,
		&mut collected,
	);
	crate::validation::xml::fda_profile::collect_fda_profile_errors(
		xpath,
		&mut collected,
	);
	drain_section_errors(collected, 'E', errors);
}
