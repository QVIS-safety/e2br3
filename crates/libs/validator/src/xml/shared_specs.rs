pub struct AttrPrefixRuleSpec {
	pub node_xpath: &'static str,
	pub value_attr: &'static str,
	pub allowed_prefixes: &'static [&'static str],
	pub rule_code: &'static str,
	pub value_label: &'static str,
}

pub struct AttrNullFlavorPairRuleSpec {
	pub node_xpath: &'static str,
	pub value_attr: &'static str,
	pub required_code: &'static str,
	pub required_message: &'static str,
	pub forbidden_code: Option<&'static str>,
	pub forbidden_message: Option<&'static str>,
}

pub struct TextNullFlavorPairRuleSpec {
	pub node_xpath: &'static str,
	pub required_code: &'static str,
	pub required_message: &'static str,
	pub forbidden_code: Option<&'static str>,
	pub forbidden_message: Option<&'static str>,
}

pub struct AttrOrNullFlavorRequiredRuleSpec {
	pub node_xpath: &'static str,
	pub value_attr: &'static str,
	pub required_code: &'static str,
	pub required_message: &'static str,
}

pub struct AttrOrTextOrNullRequiredRuleSpec {
	pub node_xpath: &'static str,
	pub value_attr: &'static str,
	pub required_code: &'static str,
	pub required_message: &'static str,
}

pub struct CodeOrCodeSystemOrTextOrNullRequiredRuleSpec {
	pub node_xpath: &'static str,
	pub required_code: &'static str,
	pub required_message: &'static str,
}

pub struct CodeOrCodeSystemOrTextRequiredForbiddenRuleSpec {
	pub node_xpath: &'static str,
	pub required_code: &'static str,
	pub required_message: &'static str,
	pub forbidden_code: &'static str,
	pub forbidden_message: &'static str,
}

pub struct RequiredChildRuleSpec {
	pub parent_xpath: &'static str,
	pub required_child_name: &'static str,
	pub rule_code: &'static str,
	pub fallback_message: &'static str,
}

pub struct RequiredAttrsRuleSpec {
	pub node_xpath: &'static str,
	pub required_attrs: &'static [&'static str],
	pub rule_code: &'static str,
	pub fallback_message: &'static str,
}

pub struct WhenChildPresentRequireAnyChildrenRuleSpec {
	pub node_xpath: &'static str,
	pub trigger_child_name: &'static str,
	pub required_child_names: &'static [&'static str],
	pub rule_code: &'static str,
	pub fallback_message: &'static str,
}

pub struct WhenAttrEqualsRequireAnyChildrenRuleSpec {
	pub node_xpath: &'static str,
	pub attr_name: &'static str,
	pub expected_attr_value: &'static str,
	pub required_child_names: &'static [&'static str],
	pub rule_code: &'static str,
	pub fallback_message: &'static str,
}

pub struct TypedChildrenAttrsOrNullFlavorRuleSpec {
	pub node_xpath: &'static str,
	pub required_xsi_type: &'static str,
	pub child_names: &'static [&'static str],
	pub required_attrs: &'static [&'static str],
	pub component_required_rule_code: &'static str,
	pub component_required_message: &'static str,
	pub attr_rule_code: &'static str,
	pub attr_rule_message: &'static str,
}

pub struct SupportedXsiTypesRuleSpec {
	pub node_xpath: &'static str,
	pub allowed_types: &'static [&'static str],
	pub rule_code: &'static str,
	pub fallback_message_prefix: &'static str,
}

pub struct NormalizedCodeRuleSpec {
	pub rule_code: &'static str,
	pub message_prefix: &'static str,
	pub extra_required_attr: Option<(&'static str, &'static str, &'static str)>,
}

pub struct ValueNodeRuleSpec {
	pub xpath: &'static str,
	pub value_attr: &'static str,
	pub rule_code: &'static str,
	pub fallback_message: &'static str,
}

pub const ICH_IDENTITY_ATTR_PREFIX_RULES: &[AttrPrefixRuleSpec] =
	&[AttrPrefixRuleSpec {
		node_xpath: "//hl7:telecom",
		value_attr: "value",
		allowed_prefixes: &["tel:", "fax:", "mailto:"],
		rule_code: "ICH.XML.TELECOM.FORMAT.REQUIRED",
		value_label: "telecom value",
	}];

pub const ICH_IDENTITY_ATTR_NULL_FLAVOR_RULES: &[AttrNullFlavorPairRuleSpec] = &[
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:telecom",
		value_attr: "value",
		required_code: "ICH.XML.TELECOM.NULLFLAVOR.REQUIRED",
		required_message: "telecom missing value; nullFlavor is required",
		forbidden_code: Some("ICH.XML.TELECOM.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"telecom has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub const ICH_IDENTITY_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:text | //hl7:originalText",
		required_code: "ICH.XML.TEXT.NULLFLAVOR.REQUIRED",
		required_message: "text/originalText is empty; nullFlavor is required",
		forbidden_code: Some("ICH.XML.TEXT.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"text/originalText has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub const ICH_IDENTITY_ATTR_OR_NULL_RULES: &[AttrOrNullFlavorRequiredRuleSpec] = &[];

pub const ICH_PROFILE_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[];

pub const ICH_PROFILE_ATTR_NULL_FLAVOR_RULES: &[AttrNullFlavorPairRuleSpec] = &[
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:low | //hl7:high",
		value_attr: "value",
		required_code: "ICH.XML.LOW_HIGH.NULLFLAVOR.REQUIRED",
		required_message: "low/high missing value; nullFlavor is required",
		forbidden_code: Some("ICH.XML.LOW_HIGH.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"low/high has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:value[@xsi:type='BL']",
		value_attr: "value",
		required_code: "ICH.XML.BL.NULLFLAVOR.REQUIRED",
		required_message: "BL value missing value; nullFlavor is required",
		forbidden_code: Some("ICH.XML.BL.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"BL value has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
	AttrNullFlavorPairRuleSpec {
		node_xpath: "//hl7:investigationCharacteristic/hl7:value[@xsi:type='BL']",
		value_attr: "value",
		required_code: "ICH.XML.INV_CHAR_BL.NULLFLAVOR.REQUIRED",
		required_message:
			"investigationCharacteristic BL missing value; nullFlavor is required",
		forbidden_code: Some("ICH.XML.INV_CHAR_BL.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"investigationCharacteristic BL has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
];

pub const ICH_PROFILE_ATTR_OR_NULL_RULES: &[AttrOrNullFlavorRequiredRuleSpec] = &[];

pub const ICH_PROFILE_ATTR_OR_TEXT_OR_NULL_RULES:
	&[AttrOrTextOrNullRequiredRuleSpec] = &[];

pub const ICH_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_OR_NULL_RULES:
	&[CodeOrCodeSystemOrTextOrNullRequiredRuleSpec] = &[];

pub const ICH_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_REQUIRED_WITH_FORBIDDEN_NULLFLAVOR_RULES: &[CodeOrCodeSystemOrTextRequiredForbiddenRuleSpec] =
	&[CodeOrCodeSystemOrTextRequiredForbiddenRuleSpec {
		node_xpath: "//hl7:code",
		required_code: "ICH.XML.CODE.NULLFLAVOR.REQUIRED",
		required_message:
			"code missing code/codeSystem; nullFlavor is required when originalText is absent",
		forbidden_code: "ICH.XML.CODE.NULLFLAVOR.FORBIDDEN",
		forbidden_message:
			"code has value and nullFlavor; nullFlavor must be absent when value present",
	}];

pub const ICH_STRUCTURAL_WHEN_CHILD_PRESENT_RULES:
	&[WhenChildPresentRequireAnyChildrenRuleSpec] =
	&[WhenChildPresentRequireAnyChildrenRuleSpec {
		node_xpath: "//hl7:effectiveTime",
		trigger_child_name: "width",
		required_child_names: &["low", "high"],
		rule_code: "ICH.XML.EFFECTIVETIME.WIDTH.REQUIRES_BOUND",
		fallback_message: "effectiveTime has width but missing low/high",
	}];

pub const ICH_STRUCTURAL_REQUIRED_CHILD_RULES: &[RequiredChildRuleSpec] = &[
	RequiredChildRuleSpec {
		parent_xpath: "//hl7:effectiveTime[@xsi:type='SXPR_TS']",
		required_child_name: "comp",
		rule_code: "ICH.XML.SXPR_TS.COMP.REQUIRED",
		fallback_message: "SXPR_TS must include comp elements",
	},
	RequiredChildRuleSpec {
		parent_xpath: "//hl7:comp[@xsi:type='PIVL_TS']",
		required_child_name: "period",
		rule_code: "ICH.XML.PIVL_TS.PERIOD.REQUIRED",
		fallback_message: "PIVL_TS must include period",
	},
];

pub const ICH_STRUCTURAL_REQUIRED_ATTRS_RULES: &[RequiredAttrsRuleSpec] = &[
	RequiredAttrsRuleSpec {
		node_xpath: "//hl7:comp[@xsi:type='PIVL_TS']/hl7:period",
		required_attrs: &["value", "unit"],
		rule_code: "ICH.XML.PIVL_TS.PERIOD.VALUE_UNIT.REQUIRED",
		fallback_message: "PIVL_TS period must include value and unit",
	},
	RequiredAttrsRuleSpec {
		node_xpath: "//hl7:doseQuantity",
		required_attrs: &["value", "unit"],
		rule_code: "ICH.XML.DOSE_QUANTITY.VALUE_UNIT.REQUIRED",
		fallback_message: "doseQuantity must include value and unit",
	},
	RequiredAttrsRuleSpec {
		node_xpath: "//hl7:period",
		required_attrs: &["value", "unit"],
		rule_code: "ICH.XML.PERIOD.VALUE_UNIT.REQUIRED",
		fallback_message: "period must include value and unit",
	},
];

pub const ICH_STRUCTURAL_WHEN_ATTR_EQUALS_RULES:
	&[WhenAttrEqualsRequireAnyChildrenRuleSpec] =
	&[WhenAttrEqualsRequireAnyChildrenRuleSpec {
		node_xpath: "//hl7:comp[@xsi:type='IVL_TS']",
		attr_name: "operator",
		expected_attr_value: "A",
		required_child_names: &["low", "high", "width"],
		rule_code: "ICH.XML.IVL_TS.OPERATOR_A.BOUND_REQUIRED",
		fallback_message: "IVL_TS operator='A' must include low, high, or width",
	}];

pub const ICH_STRUCTURAL_TYPED_CHILDREN_RULES:
	&[TypedChildrenAttrsOrNullFlavorRuleSpec] = &[];

pub const ICH_STRUCTURAL_SUPPORTED_XSI_TYPES_RULES: &[SupportedXsiTypesRuleSpec] =
	&[];

pub const ICH_STRUCTURAL_NORMALIZED_CODE_RULES: &[NormalizedCodeRuleSpec] = &[
	NormalizedCodeRuleSpec {
		rule_code: "ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED",
		message_prefix: "MedDRA code must be 8 digits",
		extra_required_attr: Some((
			"codeSystemVersion",
			"ICH.XML.MEDDRA.VERSION.REQUIRED",
			"MedDRA code missing codeSystemVersion",
		)),
	},
	NormalizedCodeRuleSpec {
		rule_code: "ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED",
		message_prefix: "ISO country code must be 2 letters",
		extra_required_attr: None,
	},
];

#[cfg(test)]
mod tests {
	use super::*;
	use crate::find_canonical_rule;
	use crate::xml::sections::{
		c::{
			FDA_C_ICH_C13_CONDITIONAL_RULE_CODE,
			FDA_C_LOCAL_CRITERIA_CONDITIONAL_RULE_CODE,
			FDA_C_PREANDA_FORBIDDEN_RULE_CODE, FDA_C_PREANDA_REQUIRED_RULE_CODE,
			FDA_C_REPORTER_EMAIL_RULE_CODE, FDA_C_STATIC_VALUE_NODE_RULES,
			ICH_C_CASE_HISTORY_RULE_CODE, ICH_C_IDENTITY_ATTR_NULL_FLAVOR_RULES,
			ICH_C_IDENTITY_ATTR_OR_NULL_RULES, ICH_C_IDENTITY_ATTR_PREFIX_RULES,
			ICH_C_IDENTITY_TEXT_NULL_FLAVOR_RULES,
			ICH_C_PROFILE_TEXT_NULL_FLAVOR_RULES,
		},
		d::{
			FDA_D_STATIC_VALUE_NODE_RULES, ICH_D_IDENTITY_ATTR_NULL_FLAVOR_RULES,
			ICH_D_MEDICAL_HISTORY_RULE_CODE, ICH_D_PROFILE_ATTR_NULL_FLAVOR_RULES,
			ICH_D_PROFILE_ATTR_OR_NULL_RULES, ICH_D_PROFILE_TEXT_NULL_FLAVOR_RULES,
		},
		e::{
			FDA_E_STATIC_VALUE_NODE_RULES, ICH_E_PROFILE_ATTR_NULL_FLAVOR_RULES,
			ICH_E_PROFILE_ATTR_OR_NULL_RULES, ICH_E_PROFILE_TEXT_NULL_FLAVOR_RULES,
			ICH_E_REACTION_TEMPORAL_RULE_CODE,
		},
		f::{
			ICH_F_STRUCTURAL_REQUIRED_ATTRS_RULES,
			ICH_F_STRUCTURAL_SUPPORTED_XSI_TYPES_RULES,
			ICH_F_STRUCTURAL_TYPED_CHILDREN_RULES,
		},
		g::{
			FDA_G_GK10A_RULE_CODE, ICH_G_DRUG_TEMPORAL_RULE_CODE,
			ICH_G_IDENTITY_TEXT_NULL_FLAVOR_RULES,
			ICH_G_PROFILE_ATTR_NULL_FLAVOR_RULES, ICH_G_PROFILE_ATTR_OR_NULL_RULES,
			ICH_G_PROFILE_ATTR_OR_TEXT_OR_NULL_RULES,
			ICH_G_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_OR_NULL_RULES,
		},
		n::FDA_N_BATCH_RECEIVER_RULE_CODE,
	};
	use std::collections::HashSet;

	fn collect_registered_codes() -> Vec<&'static str> {
		let mut codes = Vec::new();
		for rule in ICH_IDENTITY_ATTR_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_IDENTITY_TEXT_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_IDENTITY_ATTR_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_PROFILE_TEXT_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_PROFILE_ATTR_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_PROFILE_ATTR_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_PROFILE_ATTR_OR_TEXT_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_REQUIRED_WITH_FORBIDDEN_NULLFLAVOR_RULES {
			codes.push(rule.required_code);
			codes.push(rule.forbidden_code);
		}
		for rule in ICH_STRUCTURAL_WHEN_CHILD_PRESENT_RULES {
			codes.push(rule.rule_code);
		}
		for rule in ICH_STRUCTURAL_REQUIRED_CHILD_RULES {
			codes.push(rule.rule_code);
		}
		for rule in ICH_STRUCTURAL_REQUIRED_ATTRS_RULES {
			codes.push(rule.rule_code);
		}
		for rule in ICH_STRUCTURAL_WHEN_ATTR_EQUALS_RULES {
			codes.push(rule.rule_code);
		}
		for rule in ICH_STRUCTURAL_TYPED_CHILDREN_RULES {
			codes.push(rule.component_required_rule_code);
			codes.push(rule.attr_rule_code);
		}
		for rule in ICH_STRUCTURAL_SUPPORTED_XSI_TYPES_RULES {
			codes.push(rule.rule_code);
		}
		for rule in ICH_STRUCTURAL_NORMALIZED_CODE_RULES {
			codes.push(rule.rule_code);
			if let Some((_, code, _)) = rule.extra_required_attr {
				codes.push(code);
			}
		}
		for rule in ICH_C_IDENTITY_ATTR_PREFIX_RULES {
			codes.push(rule.rule_code);
		}
		for rule in ICH_C_IDENTITY_ATTR_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_C_IDENTITY_TEXT_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_C_IDENTITY_ATTR_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_C_PROFILE_TEXT_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_D_IDENTITY_ATTR_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_D_PROFILE_TEXT_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_D_PROFILE_ATTR_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_D_PROFILE_ATTR_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_E_PROFILE_TEXT_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_E_PROFILE_ATTR_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_E_PROFILE_ATTR_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_F_STRUCTURAL_REQUIRED_ATTRS_RULES {
			codes.push(rule.rule_code);
		}
		for rule in ICH_F_STRUCTURAL_TYPED_CHILDREN_RULES {
			codes.push(rule.component_required_rule_code);
			codes.push(rule.attr_rule_code);
		}
		for rule in ICH_F_STRUCTURAL_SUPPORTED_XSI_TYPES_RULES {
			codes.push(rule.rule_code);
		}
		for rule in ICH_G_IDENTITY_TEXT_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_G_PROFILE_ATTR_NULL_FLAVOR_RULES {
			codes.push(rule.required_code);
			if let Some(code) = rule.forbidden_code {
				codes.push(code);
			}
		}
		for rule in ICH_G_PROFILE_ATTR_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_G_PROFILE_ATTR_OR_TEXT_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in ICH_G_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_OR_NULL_RULES {
			codes.push(rule.required_code);
		}
		for rule in FDA_C_STATIC_VALUE_NODE_RULES {
			codes.push(rule.rule_code);
		}
		for rule in FDA_D_STATIC_VALUE_NODE_RULES {
			codes.push(rule.rule_code);
		}
		for rule in FDA_E_STATIC_VALUE_NODE_RULES {
			codes.push(rule.rule_code);
		}
		codes.extend([
			ICH_C_CASE_HISTORY_RULE_CODE,
			ICH_D_MEDICAL_HISTORY_RULE_CODE,
			ICH_E_REACTION_TEMPORAL_RULE_CODE,
			ICH_G_DRUG_TEMPORAL_RULE_CODE,
			FDA_N_BATCH_RECEIVER_RULE_CODE,
			FDA_C_LOCAL_CRITERIA_CONDITIONAL_RULE_CODE,
			FDA_G_GK10A_RULE_CODE,
			FDA_C_REPORTER_EMAIL_RULE_CODE,
			FDA_C_ICH_C13_CONDITIONAL_RULE_CODE,
			FDA_C_PREANDA_REQUIRED_RULE_CODE,
			FDA_C_PREANDA_FORBIDDEN_RULE_CODE,
		]);
		codes
	}

	#[test]
	fn registry_codes_are_catalog_backed_and_unique() {
		let mut seen = HashSet::new();
		for code in collect_registered_codes() {
			assert!(seen.insert(code), "duplicate detector rule code: {code}");
			assert!(
				find_canonical_rule(code).is_some(),
				"detector code missing in catalog: {code}"
			);
		}
	}
}
