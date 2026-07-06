use crate::{
	AttrNullFlavorPairRuleSpec, AttrOrNullFlavorRequiredRuleSpec,
	AttrPrefixRuleSpec, TextNullFlavorPairRuleSpec, ValueNodeRuleSpec,
};
use lib_core::xml::types::XmlValidationError;
use libxml::xpath::Context;

pub(crate) const ICH_C_IDENTITY_ATTR_PREFIX_RULES: &[AttrPrefixRuleSpec] =
	&[AttrPrefixRuleSpec {
		node_xpath: "//hl7:telecom",
		value_attr: "value",
		allowed_prefixes: &["tel:", "fax:", "mailto:"],
		rule_code: "ICH.XML.TELECOM.FORMAT.REQUIRED",
		value_label: "telecom value",
	}];

pub(crate) const ICH_C_IDENTITY_ATTR_NULL_FLAVOR_RULES: &[AttrNullFlavorPairRuleSpec] = &[
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:primaryRole/hl7:id",
		value_attr: "extension",
		required_code: "ICH.C.2.r.1.ID.NULLFLAVOR.REQUIRED",
		required_message: "primaryRole/id missing extension; nullFlavor is required",
		forbidden_code: Some("ICH.C.2.r.1.ID.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"primaryRole/id has extension and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_C_IDENTITY_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:primaryRole//hl7:name/*",
		required_code: "ICH.C.2.r.2.NAME.NULLFLAVOR.REQUIRED",
		required_message: "primaryRole name element is empty; nullFlavor is required",
		forbidden_code: Some("ICH.C.2.r.2.NAME.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"primaryRole name element has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:representedOrganization/hl7:name",
		required_code: "ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.REQUIRED",
		required_message: "representedOrganization/name is empty; nullFlavor is required",
		forbidden_code: Some("ICH.C.2.r.3.ORG_NAME.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"representedOrganization/name has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_C_IDENTITY_ATTR_OR_NULL_RULES: &[AttrOrNullFlavorRequiredRuleSpec] = &[AttrOrNullFlavorRequiredRuleSpec {
	node_xpath: "//hl7:primaryRole/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.6']",
	value_attr: "extension",
	required_code: "ICH.C.2.r.1.ID.ROOT_3_6.NULLFLAVOR.REQUIRED",
	required_message:
		"primaryRole/id with root 2.16.840.1.113883.3.989.2.1.3.6 requires extension or nullFlavor",
}];

pub(crate) const ICH_C_PROFILE_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:researchStudy/hl7:title",
		required_code: "ICH.C.5.TITLE.NULLFLAVOR.REQUIRED",
		required_message: "researchStudy/title is empty; nullFlavor is required",
		forbidden_code: Some("ICH.C.5.TITLE.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"researchStudy/title has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub(crate) const ICH_C_CASE_HISTORY_RULE_CODE: &str = "ICH.C.1.9.1.CONDITIONAL";
pub(crate) const ICH_C_CASE_HISTORY_RULE_MESSAGE: &str =
	"C.1.9.1 is true but C.1.9.1.r.1/.r.2 are missing";

pub(crate) const FDA_C_STATIC_VALUE_NODE_RULES: &[ValueNodeRuleSpec] = &[
	ValueNodeRuleSpec {
		xpath: "//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.5.1.2.2.1.3']]/hl7:value",
		value_attr: "value",
		rule_code: "FDA.C.1.12.REQUIRED",
		fallback_message:
			"FDA.C.1.12 combination product indicator missing value; nullFlavor is required",
	},
	ValueNodeRuleSpec {
		xpath: "//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value",
		value_attr: "code",
		rule_code: "FDA.C.1.7.1.REQUIRED.MISSING_CODE",
		fallback_message:
			"FDA.C.1.7.1 local criteria report type missing code; nullFlavor is required",
	},
];

pub(crate) const FDA_C_LOCAL_CRITERIA_VALUE_XPATH: &str =
	"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value";
pub(crate) const FDA_C_REPORT_TYPE_VALUE_XPATH: &str =
	"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value";
pub(crate) const FDA_C_FACT_COMBINATION_PRODUCT_XPATH: &str =
	"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.5.1.2.2.1.3']]/hl7:value/@value";
pub(crate) const FDA_C_FACT_FULFIL_EXPEDITED_XPATH: &str =
	"//hl7:component/hl7:observationEvent[hl7:code[@code='23' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";
pub(crate) const FDA_C_FACT_PREANDA_XPATH: &str =
	"//hl7:researchStudy/hl7:authorization/hl7:studyRegistration/hl7:id[@root='2.16.840.1.113883.3.989.5.1.2.2.1.2.2']/@extension";
pub(crate) const FDA_C_FACT_STUDY_TYPE_XPATH: &str =
	"//hl7:researchStudy/hl7:code/@code";
pub(crate) const FDA_C_FACT_TYPE_OF_REPORT_XPATH: &str =
	"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/@code";
pub(crate) const FDA_C_FACT_PRIMARY_SOURCE_NODE_XPATH: &str =
	"//hl7:outboundRelationship[@typeCode='SPRT']/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
pub(crate) const FDA_C_FACT_PRIMARY_SOURCE_EMAIL_XPATH: &str =
	"//hl7:outboundRelationship[@typeCode='SPRT']/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:telecom/@value";
pub(crate) const FDA_C_LOCAL_CRITERIA_CONDITIONAL_RULE_CODE: &str =
	"FDA.C.1.7.1.REQUIRED";
pub(crate) const FDA_C_LOCAL_CRITERIA_CONDITIONAL_RULE_MESSAGE: &str =
	"FDA.C.1.7.1 local criteria report type is invalid for current expedited/combination product facts";
pub(crate) const FDA_C_REPORTER_EMAIL_RULE_CODE: &str = "FDA.C.2.r.2.EMAIL.REQUIRED";
pub(crate) const FDA_C_REPORTER_EMAIL_RULE_MESSAGE: &str =
	"FDA requires reporter email when primary source is present";
pub(crate) const FDA_C_ICH_C13_CONDITIONAL_RULE_CODE: &str = "ICH.C.1.3.CONDITIONAL";
pub(crate) const FDA_C_ICH_C13_CONDITIONAL_RULE_MESSAGE: &str =
	"C.1.3 must be 2 when premarket receiver and FDA.C.5.5b present with study type 1/2/3";
pub(crate) const FDA_C_PREANDA_REQUIRED_RULE_CODE: &str = "FDA.C.5.5b.REQUIRED";
pub(crate) const FDA_C_PREANDA_REQUIRED_RULE_MESSAGE: &str =
	"FDA.C.5.5b required when C.1.3=2 and N.2.r.3=CDER_IND_EXEMPT_BA_BE";
pub(crate) const FDA_C_PREANDA_FORBIDDEN_RULE_CODE: &str = "FDA.C.5.5b.FORBIDDEN";
pub(crate) const FDA_C_PREANDA_FORBIDDEN_RULE_MESSAGE: &str =
	"FDA.C.5.5b must not be provided for postmarket (N.1.4=ZZFDA, N.2.r.3=CDER/CBER)";

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
	crate::xml::ich_profile::collect_ich_case_history_errors(xpath, &mut collected);
	crate::xml::fda_profile::collect_fda_profile_errors(xpath, &mut collected);
	drain_section_errors(collected, 'C', errors);
}
