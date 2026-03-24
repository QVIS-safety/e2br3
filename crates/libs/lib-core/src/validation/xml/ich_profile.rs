use crate::validation::{
	export_normalization_spec_for_rule, AttrNullFlavorPairRuleSpec,
	AttrOrNullFlavorRequiredRuleSpec, AttrOrTextOrNullRequiredRuleSpec,
	AttrPrefixRuleSpec, CodeOrCodeSystemOrTextOrNullRequiredRuleSpec,
	CodeOrCodeSystemOrTextRequiredForbiddenRuleSpec, NormalizedCodeRuleSpec,
	RequiredAttrsRuleSpec, RequiredChildRuleSpec, RuleFacts,
	SupportedXsiTypesRuleSpec, TextNullFlavorPairRuleSpec,
	TypedChildrenAttrsOrNullFlavorRuleSpec,
	WhenAttrEqualsRequireAnyChildrenRuleSpec,
	WhenChildPresentRequireAnyChildrenRuleSpec, ICH_IDENTITY_ATTR_NULL_FLAVOR_RULES,
	ICH_IDENTITY_TEXT_NULL_FLAVOR_RULES, ICH_PROFILE_ATTR_NULL_FLAVOR_RULES,
	ICH_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_REQUIRED_WITH_FORBIDDEN_NULLFLAVOR_RULES,
	ICH_STRUCTURAL_NORMALIZED_CODE_RULES, ICH_STRUCTURAL_REQUIRED_ATTRS_RULES,
	ICH_STRUCTURAL_REQUIRED_CHILD_RULES, ICH_STRUCTURAL_WHEN_ATTR_EQUALS_RULES,
	ICH_STRUCTURAL_WHEN_CHILD_PRESENT_RULES,
};
use crate::xml::types::XmlValidationError;
use libxml::tree::Node;
use libxml::xpath::Context;

use super::{
	push_rule_error, validate_attr_null_flavor_pair_on_nodes,
	validate_attr_or_null_flavor_required_on_nodes,
	validate_attr_or_text_or_null_required_on_nodes, validate_attr_prefix_on_nodes,
	validate_code_or_codesystem_or_text_or_null_required_on_nodes,
	validate_code_or_codesystem_or_text_required_with_nullflavor_forbidden_on_nodes,
	validate_condition_rule_violation, validate_normalized_code_format_on_nodes,
	validate_required_attrs_on_nodes, validate_required_child_on_nodes,
	validate_supported_xsi_types_on_nodes, validate_text_null_flavor_pair_on_nodes,
	validate_typed_children_attrs_or_nullflavor_on_nodes,
	validate_when_attr_equals_require_any_children,
	validate_when_child_present_require_any_children, xpath_any_node,
	xpath_has_nodes,
};
use crate::validation::xml::sections::{
	c::{
		ICH_C_CASE_HISTORY_RULE_CODE, ICH_C_CASE_HISTORY_RULE_MESSAGE,
		ICH_C_IDENTITY_ATTR_NULL_FLAVOR_RULES, ICH_C_IDENTITY_ATTR_OR_NULL_RULES,
		ICH_C_IDENTITY_ATTR_PREFIX_RULES, ICH_C_IDENTITY_TEXT_NULL_FLAVOR_RULES,
		ICH_C_PROFILE_TEXT_NULL_FLAVOR_RULES,
	},
	d::{
		ICH_D_IDENTITY_ATTR_NULL_FLAVOR_RULES, ICH_D_MEDICAL_HISTORY_RULE_CODE,
		ICH_D_MEDICAL_HISTORY_RULE_MESSAGE, ICH_D_PROFILE_ATTR_NULL_FLAVOR_RULES,
		ICH_D_PROFILE_ATTR_OR_NULL_RULES, ICH_D_PROFILE_TEXT_NULL_FLAVOR_RULES,
	},
	e::{
		ICH_E_PROFILE_ATTR_NULL_FLAVOR_RULES, ICH_E_PROFILE_ATTR_OR_NULL_RULES,
		ICH_E_PROFILE_TEXT_NULL_FLAVOR_RULES, ICH_E_REACTION_TEMPORAL_RULE_CODE,
		ICH_E_REACTION_TEMPORAL_RULE_MESSAGE,
	},
	f::{
		ICH_F_STRUCTURAL_REQUIRED_ATTRS_RULES,
		ICH_F_STRUCTURAL_SUPPORTED_XSI_TYPES_RULES,
		ICH_F_STRUCTURAL_TYPED_CHILDREN_RULES,
	},
	g::{
		ICH_G_DRUG_TEMPORAL_RULE_CODE, ICH_G_DRUG_TEMPORAL_RULE_MESSAGE,
		ICH_G_IDENTITY_TEXT_NULL_FLAVOR_RULES, ICH_G_PROFILE_ATTR_NULL_FLAVOR_RULES,
		ICH_G_PROFILE_ATTR_OR_NULL_RULES, ICH_G_PROFILE_ATTR_OR_TEXT_OR_NULL_RULES,
		ICH_G_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_OR_NULL_RULES,
	},
};

fn apply_attr_prefix_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[AttrPrefixRuleSpec],
) {
	for rule in rules {
		validate_attr_prefix_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.value_attr,
			rule.allowed_prefixes,
			rule.rule_code,
			rule.value_label,
		);
	}
}

fn apply_attr_null_flavor_pair_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[AttrNullFlavorPairRuleSpec],
) {
	for rule in rules {
		validate_attr_null_flavor_pair_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.value_attr,
			rule.required_code,
			rule.required_message,
			rule.forbidden_code,
			rule.forbidden_message,
		);
	}
}

fn apply_text_null_flavor_pair_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[TextNullFlavorPairRuleSpec],
) {
	for rule in rules {
		validate_text_null_flavor_pair_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.required_code,
			rule.required_message,
			rule.forbidden_code,
			rule.forbidden_message,
		);
	}
}

fn apply_attr_or_null_flavor_required_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[AttrOrNullFlavorRequiredRuleSpec],
) {
	for rule in rules {
		validate_attr_or_null_flavor_required_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.value_attr,
			rule.required_code,
			rule.required_message,
		);
	}
}

fn apply_attr_or_text_or_null_required_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[AttrOrTextOrNullRequiredRuleSpec],
) {
	for rule in rules {
		validate_attr_or_text_or_null_required_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.value_attr,
			rule.required_code,
			rule.required_message,
		);
	}
}

fn apply_code_or_codesystem_or_text_or_null_required_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[CodeOrCodeSystemOrTextOrNullRequiredRuleSpec],
) {
	for rule in rules {
		validate_code_or_codesystem_or_text_or_null_required_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.required_code,
			rule.required_message,
		);
	}
}

fn apply_code_or_codesystem_or_text_required_with_nullflavor_forbidden_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[CodeOrCodeSystemOrTextRequiredForbiddenRuleSpec],
) {
	for rule in rules {
		validate_code_or_codesystem_or_text_required_with_nullflavor_forbidden_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.required_code,
			rule.required_message,
			rule.forbidden_code,
			rule.forbidden_message,
		);
	}
}

fn apply_required_child_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[RequiredChildRuleSpec],
) {
	for rule in rules {
		validate_required_child_on_nodes(
			xpath,
			errors,
			rule.parent_xpath,
			rule.required_child_name,
			rule.rule_code,
			rule.fallback_message,
		);
	}
}

fn apply_required_attrs_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[RequiredAttrsRuleSpec],
) {
	for rule in rules {
		validate_required_attrs_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.required_attrs,
			rule.rule_code,
			rule.fallback_message,
		);
	}
}

fn apply_when_child_present_require_any_children_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[WhenChildPresentRequireAnyChildrenRuleSpec],
) {
	for rule in rules {
		validate_when_child_present_require_any_children(
			xpath,
			errors,
			rule.node_xpath,
			rule.trigger_child_name,
			rule.required_child_names,
			rule.rule_code,
			rule.fallback_message,
		);
	}
}

fn apply_when_attr_equals_require_any_children_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[WhenAttrEqualsRequireAnyChildrenRuleSpec],
) {
	for rule in rules {
		validate_when_attr_equals_require_any_children(
			xpath,
			errors,
			rule.node_xpath,
			rule.attr_name,
			rule.expected_attr_value,
			rule.required_child_names,
			rule.rule_code,
			rule.fallback_message,
		);
	}
}

fn apply_typed_children_attrs_or_nullflavor_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[TypedChildrenAttrsOrNullFlavorRuleSpec],
) {
	for rule in rules {
		validate_typed_children_attrs_or_nullflavor_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.required_xsi_type,
			rule.child_names,
			rule.required_attrs,
			rule.component_required_rule_code,
			rule.component_required_message,
			rule.attr_rule_code,
			rule.attr_rule_message,
		);
	}
}

fn apply_supported_xsi_types_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[SupportedXsiTypesRuleSpec],
) {
	for rule in rules {
		validate_supported_xsi_types_on_nodes(
			xpath,
			errors,
			rule.node_xpath,
			rule.allowed_types,
			rule.rule_code,
			rule.fallback_message_prefix,
		);
	}
}

fn apply_normalized_code_rules(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	rules: &[NormalizedCodeRuleSpec],
) {
	for rule in rules {
		let Some(spec) = export_normalization_spec_for_rule(rule.rule_code) else {
			continue;
		};
		let prefix = rule.message_prefix;
		let formatter = move |code: &str| format!("{prefix}, got '{code}'");
		validate_normalized_code_format_on_nodes(
			xpath,
			errors,
			rule.rule_code,
			spec,
			formatter,
			rule.extra_required_attr,
		);
	}
}

pub(crate) fn collect_ich_identity_text_errors(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
) {
	apply_attr_prefix_rules(xpath, errors, ICH_C_IDENTITY_ATTR_PREFIX_RULES);
	apply_attr_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_IDENTITY_ATTR_NULL_FLAVOR_RULES,
	);
	apply_attr_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_C_IDENTITY_ATTR_NULL_FLAVOR_RULES,
	);
	apply_attr_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_D_IDENTITY_ATTR_NULL_FLAVOR_RULES,
	);
	apply_text_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_IDENTITY_TEXT_NULL_FLAVOR_RULES,
	);
	apply_text_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_C_IDENTITY_TEXT_NULL_FLAVOR_RULES,
	);
	apply_text_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_G_IDENTITY_TEXT_NULL_FLAVOR_RULES,
	);
	apply_attr_or_null_flavor_required_rules(
		xpath,
		errors,
		ICH_C_IDENTITY_ATTR_OR_NULL_RULES,
	);
}

pub(crate) fn collect_ich_profile_value_presence_errors(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
) {
	apply_text_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_C_PROFILE_TEXT_NULL_FLAVOR_RULES,
	);
	apply_text_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_D_PROFILE_TEXT_NULL_FLAVOR_RULES,
	);
	apply_text_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_E_PROFILE_TEXT_NULL_FLAVOR_RULES,
	);
	apply_attr_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_PROFILE_ATTR_NULL_FLAVOR_RULES,
	);
	apply_attr_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_D_PROFILE_ATTR_NULL_FLAVOR_RULES,
	);
	apply_attr_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_E_PROFILE_ATTR_NULL_FLAVOR_RULES,
	);
	apply_attr_null_flavor_pair_rules(
		xpath,
		errors,
		ICH_G_PROFILE_ATTR_NULL_FLAVOR_RULES,
	);
	apply_attr_or_null_flavor_required_rules(
		xpath,
		errors,
		ICH_D_PROFILE_ATTR_OR_NULL_RULES,
	);
	apply_attr_or_null_flavor_required_rules(
		xpath,
		errors,
		ICH_E_PROFILE_ATTR_OR_NULL_RULES,
	);
	apply_attr_or_null_flavor_required_rules(
		xpath,
		errors,
		ICH_G_PROFILE_ATTR_OR_NULL_RULES,
	);
	apply_attr_or_text_or_null_required_rules(
		xpath,
		errors,
		ICH_G_PROFILE_ATTR_OR_TEXT_OR_NULL_RULES,
	);
	apply_code_or_codesystem_or_text_or_null_required_rules(
		xpath,
		errors,
		ICH_G_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_OR_NULL_RULES,
	);
	apply_code_or_codesystem_or_text_required_with_nullflavor_forbidden_rules(
		xpath,
		errors,
		ICH_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_REQUIRED_WITH_FORBIDDEN_NULLFLAVOR_RULES,
	);
}

pub(crate) fn collect_ich_structural_value_errors(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
) {
	apply_when_child_present_require_any_children_rules(
		xpath,
		errors,
		ICH_STRUCTURAL_WHEN_CHILD_PRESENT_RULES,
	);

	// Rule: start/end/duration combos for reaction event (E.i.4/E.i.5/E.i.6)
	validate_temporal_markers_required_on_nodes(
		xpath,
		errors,
		"//hl7:observation[hl7:id and hl7:code[@code='29'] and hl7:code[@codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]",
		ICH_E_REACTION_TEMPORAL_RULE_CODE,
		ICH_E_REACTION_TEMPORAL_RULE_MESSAGE,
		reaction_temporal_markers,
	);

	// Rule: drug start/end/duration combos for dosage (G.k.4.r.4/5/8)
	validate_temporal_markers_required_on_nodes(
		xpath,
		errors,
		"//hl7:substanceAdministration/hl7:effectiveTime[@xsi:type='SXPR_TS' or @xsi:type='IVL_TS']",
		ICH_G_DRUG_TEMPORAL_RULE_CODE,
		ICH_G_DRUG_TEMPORAL_RULE_MESSAGE,
		temporal_markers_from_children,
	);

	apply_required_child_rules(xpath, errors, ICH_STRUCTURAL_REQUIRED_CHILD_RULES);
	apply_required_attrs_rules(xpath, errors, ICH_STRUCTURAL_REQUIRED_ATTRS_RULES);
	apply_required_attrs_rules(xpath, errors, ICH_F_STRUCTURAL_REQUIRED_ATTRS_RULES);

	// Rule: IVL_TS with operator='A' must include low/high or width
	apply_when_attr_equals_require_any_children_rules(
		xpath,
		errors,
		ICH_STRUCTURAL_WHEN_ATTR_EQUALS_RULES,
	);

	// Rule: test result values must be structurally valid
	apply_typed_children_attrs_or_nullflavor_rules(
		xpath,
		errors,
		ICH_F_STRUCTURAL_TYPED_CHILDREN_RULES,
	);
	apply_supported_xsi_types_rules(
		xpath,
		errors,
		ICH_F_STRUCTURAL_SUPPORTED_XSI_TYPES_RULES,
	);

	apply_normalized_code_rules(xpath, errors, ICH_STRUCTURAL_NORMALIZED_CODE_RULES);
}

pub(crate) fn collect_ich_case_history_errors(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
) {
	// Rule: C.1.9.1 true requires linked prior case identifiers
	let has_true = xpath_any_node(
		xpath,
		"//hl7:investigationCharacteristic[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value",
		|n| {
			n.get_attribute("value")
				.map(|v| v == "true" || v == "1")
				.unwrap_or(false)
		},
	);
	if has_true {
		let has_ids = xpath_any_node(
			xpath,
			"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.3']",
			|n| {
				n.get_attribute("assigningAuthorityName")
					.map(|v| !v.trim().is_empty())
					.unwrap_or(false)
					&& n.get_attribute("extension")
						.map(|v| !v.trim().is_empty())
						.unwrap_or(false)
			},
		);
		validate_condition_rule_violation(
			errors,
			ICH_C_CASE_HISTORY_RULE_CODE,
			RuleFacts {
				ich_case_history_true_missing_prior_ids: Some(!has_ids),
				..RuleFacts::default()
			},
			ICH_C_CASE_HISTORY_RULE_MESSAGE,
		);
	}

	// Rule: D.7.2 required when coded medical history entry missing
	let has_coded = xpath_has_nodes(
		xpath,
		"//hl7:organizer[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:observation[hl7:code[@code!='18']]",
	);
	if has_coded {
		return;
	}
	let has_text = xpath_any_node(
		xpath,
		"//hl7:organizer[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:observation[hl7:code[@code='18']]/hl7:value",
		|n| {
			let content = n.get_content();
			!content.trim().is_empty() && !looks_placeholder(content.trim())
		},
	);
	validate_condition_rule_violation(
		errors,
		ICH_D_MEDICAL_HISTORY_RULE_CODE,
		RuleFacts {
			ich_medical_history_missing_d72_text: Some(!has_text),
			..RuleFacts::default()
		},
		ICH_D_MEDICAL_HISTORY_RULE_MESSAGE,
	);
}

fn validate_temporal_markers_required_on_nodes(
	xpath: &mut Context,
	errors: &mut Vec<XmlValidationError>,
	node_xpath: &str,
	rule_code: &str,
	message: &str,
	detector: fn(&Node) -> (bool, bool, bool),
) {
	if let Ok(nodes) = xpath.findnodes(node_xpath, None) {
		for node in nodes {
			let (has_start, has_end, has_duration) = detector(&node);
			if !has_start && !has_end && !has_duration {
				push_rule_error(errors, rule_code, message);
			}
		}
	}
}

fn reaction_temporal_markers(node: &Node) -> (bool, bool, bool) {
	let mut has_start = false;
	let mut has_end = false;
	let mut has_duration = false;

	for child in node.get_child_elements() {
		match child.get_name().as_str() {
			"effectiveTime" => {
				if child.get_attribute("value").is_some() {
					has_start = true;
				} else {
					let (s, e, d) = temporal_markers_from_children(&child);
					has_start |= s;
					has_end |= e;
					has_duration |= d;
				}
			}
			"value" => {
				if child.get_attribute("value").is_some() {
					has_duration = true;
				}
			}
			_ => {}
		}
	}

	(has_start, has_end, has_duration)
}

fn temporal_markers_from_children(node: &Node) -> (bool, bool, bool) {
	let mut has_start = false;
	let mut has_end = false;
	let mut has_duration = false;

	for child in node.get_child_elements() {
		match child.get_name().as_str() {
			"low" => has_start = true,
			"high" => has_end = true,
			"width" => has_duration = true,
			"comp" => {
				let (s, e, d) = temporal_markers_from_children(&child);
				has_start |= s;
				has_end |= e;
				has_duration |= d;
			}
			_ => {}
		}
	}

	(has_start, has_end, has_duration)
}

fn looks_placeholder(value: &str) -> bool {
	let v = value.trim();
	if v.is_empty() {
		return false;
	}
	if v.chars().any(|c| c.is_whitespace()) {
		return false;
	}
	if v.len() > 24 {
		return false;
	}
	let mut chars = v.chars();
	let Some(first) = chars.next() else {
		return false;
	};
	if !first.is_ascii_uppercase() {
		return false;
	}
	if !v.contains('.') {
		return false;
	}
	if !v.chars().any(|c| c.is_ascii_digit()) {
		return false;
	}
	v.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}
