use crate::{
	AttrNullFlavorPairRuleSpec, AttrOrNullFlavorRequiredRuleSpec,
	AttrOrTextOrNullRequiredRuleSpec, CodeOrCodeSystemOrTextOrNullRequiredRuleSpec,
	TextNullFlavorPairRuleSpec,
};
use lib_core::xml::types::XmlValidationError;
use libxml::xpath::Context;

pub(crate) const ICH_G_IDENTITY_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:ingredientSubstance/hl7:name",
		required_code: "ICH.G.k.2.3.NAME.NULLFLAVOR.REQUIRED",
		required_message: "ingredientSubstance/name is empty; nullFlavor is required",
		forbidden_code: Some("ICH.G.k.2.3.NAME.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"ingredientSubstance/name has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_G_PROFILE_ATTR_NULL_FLAVOR_RULES: &[AttrNullFlavorPairRuleSpec] = &[
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:adverseEventAssessment/hl7:id",
		value_attr: "extension",
		required_code: "ICH.G.k.9.i.2.ID.NULLFLAVOR.REQUIRED",
		required_message: "adverseEventAssessment/id missing extension; nullFlavor is required",
		forbidden_code: Some("ICH.G.k.9.i.2.ID.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"adverseEventAssessment/id has extension and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_G_PROFILE_ATTR_OR_NULL_RULES: &[AttrOrNullFlavorRequiredRuleSpec] = &[
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath:
			"//hl7:substanceAdministration/hl7:effectiveTime//hl7:low | //hl7:substanceAdministration/hl7:effectiveTime//hl7:high",
		value_attr: "value",
		required_code: "ICH.G.k.4.r.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		required_message:
			"drug effectiveTime low/high missing value; nullFlavor is required",
	},
];

pub(crate) const ICH_G_PROFILE_ATTR_OR_TEXT_OR_NULL_RULES:
	&[AttrOrTextOrNullRequiredRuleSpec] = &[AttrOrTextOrNullRequiredRuleSpec {
	node_xpath: "//hl7:routeCode",
	value_attr: "code",
	required_code: "ICH.G.k.4.r.11.NULLFLAVOR.REQUIRED",
	required_message:
		"routeCode missing code; originalText or nullFlavor is required",
}];

pub(crate) const ICH_G_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_OR_NULL_RULES:
	&[CodeOrCodeSystemOrTextOrNullRequiredRuleSpec] =
	&[CodeOrCodeSystemOrTextOrNullRequiredRuleSpec {
		node_xpath: "//hl7:formCode",
		required_code: "ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED",
		required_message:
			"formCode missing code/codeSystem/originalText; nullFlavor is required",
	}];

pub(crate) const ICH_G_DRUG_TEMPORAL_RULE_CODE: &str = "ICH.G.k.4.r.4-8.CONDITIONAL";
pub(crate) const ICH_G_DRUG_TEMPORAL_RULE_MESSAGE: &str =
	"Drug requires start, end, or duration";

pub(crate) const FDA_G_GK10A_VALUE_XPATH: &str =
	"//hl7:organizer[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration/hl7:outboundRelationship2[@typeCode='REFR']/hl7:observation[hl7:code[@code='9']]/hl7:value";
pub(crate) const FDA_G_GK10A_RULE_CODE: &str = "FDA.G.k.10a.REQUIRED";
pub(crate) const FDA_G_GK10A_REQUIRED_MESSAGE: &str =
	"FDA.G.k.10a missing: required when FDA.C.5.5b is present";
pub(crate) const FDA_G_GK10A_VALUE_MESSAGE: &str =
	"FDA.G.k.10a must be code 1/2 or nullFlavor NA when FDA.C.5.5b is present";

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
	crate::xml::ich_profile::collect_ich_identity_text_errors(xpath, &mut collected);
	crate::xml::ich_profile::collect_ich_profile_value_presence_errors(
		xpath,
		&mut collected,
	);
	crate::xml::ich_profile::collect_ich_structural_value_errors(
		xpath,
		&mut collected,
	);
	crate::xml::fda_profile::collect_fda_profile_errors(xpath, &mut collected);
	drain_section_errors(collected, 'G', errors);
}
