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

pub const ICH_IDENTITY_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:ingredientSubstance/hl7:name",
		required_code: "ICH.G.k.2.3.NAME.NULLFLAVOR.REQUIRED",
		required_message: "ingredientSubstance/name is empty; nullFlavor is required",
		forbidden_code: Some("ICH.G.k.2.3.NAME.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"ingredientSubstance/name has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
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

pub const ICH_IDENTITY_ATTR_OR_NULL_RULES: &[AttrOrNullFlavorRequiredRuleSpec] = &[AttrOrNullFlavorRequiredRuleSpec {
	node_xpath: "//hl7:primaryRole/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.6']",
	value_attr: "extension",
	required_code: "ICH.C.2.r.1.ID.ROOT_3_6.NULLFLAVOR.REQUIRED",
	required_message:
		"primaryRole/id with root 2.16.840.1.113883.3.989.2.1.3.6 requires extension or nullFlavor",
}];

pub const ICH_PROFILE_TEXT_NULL_FLAVOR_RULES: &[TextNullFlavorPairRuleSpec] = &[
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:associatedPerson//hl7:name/*",
		required_code: "ICH.D.PARENT.NAME.NULLFLAVOR.REQUIRED",
		required_message: "associatedPerson name element is empty; nullFlavor is required",
		forbidden_code: Some("ICH.D.PARENT.NAME.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"associatedPerson name element has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
	TextNullFlavorPairRuleSpec {
		node_xpath: "//hl7:researchStudy/hl7:title",
		required_code: "ICH.C.5.TITLE.NULLFLAVOR.REQUIRED",
		required_message: "researchStudy/title is empty; nullFlavor is required",
		forbidden_code: Some("ICH.C.5.TITLE.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"researchStudy/title has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
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

pub const ICH_PROFILE_ATTR_NULL_FLAVOR_RULES: &[AttrNullFlavorPairRuleSpec] = &[
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
		required_message: "investigationCharacteristic BL missing value; nullFlavor is required",
		forbidden_code: Some("ICH.XML.INV_CHAR_BL.NULLFLAVOR.FORBIDDEN"),
		forbidden_message: Some(
			"investigationCharacteristic BL has value and nullFlavor; nullFlavor must be absent when value present",
		),
	},
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

pub const ICH_PROFILE_ATTR_OR_NULL_RULES: &[AttrOrNullFlavorRequiredRuleSpec] = &[
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath:
			"//hl7:observation[hl7:code[@code='29']]/hl7:effectiveTime/hl7:low | //hl7:observation[hl7:code[@code='29']]/hl7:effectiveTime/hl7:high",
		value_attr: "value",
		required_code: "ICH.E.i.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		required_message: "reaction effectiveTime low/high missing value; nullFlavor is required",
	},
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath:
			"//hl7:substanceAdministration/hl7:effectiveTime//hl7:low | //hl7:substanceAdministration/hl7:effectiveTime//hl7:high",
		value_attr: "value",
		required_code: "ICH.G.k.4.r.4-5.LOW_HIGH.NULLFLAVOR.REQUIRED",
		required_message: "drug effectiveTime low/high missing value; nullFlavor is required",
	},
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath:
			"//hl7:primaryRole//hl7:effectiveTime//hl7:low | //hl7:primaryRole//hl7:effectiveTime//hl7:high",
		value_attr: "value",
		required_code: "ICH.D.EFFECTIVETIME.LOW_HIGH.NULLFLAVOR.REQUIRED",
		required_message: "patient effectiveTime low/high missing value; nullFlavor is required",
	},
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath: "//hl7:locatedPlace/hl7:code",
		value_attr: "code",
		required_code: "ICH.E.i.9.COUNTRY.NULLFLAVOR.REQUIRED",
		required_message: "reaction country missing code; nullFlavor is required",
	},
	AttrOrNullFlavorRequiredRuleSpec {
		node_xpath: "//hl7:administrativeGenderCode",
		value_attr: "code",
		required_code: "ICH.D.5.SEX.CONDITIONAL",
		required_message: "administrativeGenderCode missing code; nullFlavor is required",
	},
];

pub const ICH_PROFILE_ATTR_OR_TEXT_OR_NULL_RULES:
	&[AttrOrTextOrNullRequiredRuleSpec] = &[AttrOrTextOrNullRequiredRuleSpec {
	node_xpath: "//hl7:routeCode",
	value_attr: "code",
	required_code: "ICH.G.k.4.r.11.NULLFLAVOR.REQUIRED",
	required_message:
		"routeCode missing code; originalText or nullFlavor is required",
}];

pub const ICH_PROFILE_CODE_OR_CODESYSTEM_OR_TEXT_OR_NULL_RULES:
	&[CodeOrCodeSystemOrTextOrNullRequiredRuleSpec] =
	&[CodeOrCodeSystemOrTextOrNullRequiredRuleSpec {
		node_xpath: "//hl7:formCode",
		required_code: "ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED",
		required_message:
			"formCode missing code/codeSystem/originalText; nullFlavor is required",
	}];

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
		node_xpath: "//hl7:organizer[hl7:code[@code='3']]/hl7:component/hl7:observation/hl7:value[@xsi:type='PQ']",
		required_attrs: &["value", "unit"],
		rule_code: "ICH.XML.TESTRESULT.PQ.VALUE_UNIT.REQUIRED",
		fallback_message: "PQ must include value and unit",
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

pub const ICH_STRUCTURAL_TYPED_CHILDREN_RULES: &[TypedChildrenAttrsOrNullFlavorRuleSpec] =
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

pub const ICH_STRUCTURAL_SUPPORTED_XSI_TYPES_RULES: &[SupportedXsiTypesRuleSpec] =
	&[SupportedXsiTypesRuleSpec {
		node_xpath:
			"//hl7:organizer[hl7:code[@code='3']]/hl7:component/hl7:observation/hl7:value",
		allowed_types: &["IVL_PQ", "PQ", "ED", "ST", "BL", "CE"],
		rule_code: "ICH.XML.TESTRESULT.XSI_TYPE.UNSUPPORTED",
		fallback_message_prefix: "Unsupported test result xsi:type",
	}];

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

pub const FDA_STATIC_VALUE_NODE_RULES: &[ValueNodeRuleSpec] = &[
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
	ValueNodeRuleSpec {
		xpath: "//hl7:observation[hl7:code[@code='29' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]//hl7:outboundRelationship2/hl7:observation[hl7:code[@code='7' and @codeSystem='2.16.840.1.113883.3.989.5.1.2.2.1.3']]/hl7:value",
		value_attr: "value",
		rule_code: "FDA.E.i.3.2h.REQUIRED",
		fallback_message:
			"FDA.E.i.3.2h required intervention missing value; nullFlavor is required",
	},
];

pub const FDA_LOCAL_CRITERIA_VALUE_XPATH: &str =
	"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value";
pub const FDA_GK10A_VALUE_XPATH: &str =
	"//hl7:organizer[hl7:code[@code='4' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.20']]/hl7:component/hl7:substanceAdministration/hl7:outboundRelationship2[@typeCode='REFR']/hl7:observation[hl7:code[@code='9']]/hl7:value";
pub const FDA_REPORT_TYPE_VALUE_XPATH: &str =
	"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value";
pub const FDA_FACT_BATCH_RECEIVER_XPATH: &str =
	"/hl7:MCCI_IN200100UV01/hl7:receiver/hl7:device/hl7:id/@extension";
pub const FDA_FACT_MSG_RECEIVER_XPATH: &str =
	"/hl7:MCCI_IN200100UV01/hl7:PORR_IN049016UV/hl7:receiver/hl7:device/hl7:id/@extension";
pub const FDA_FACT_COMBINATION_PRODUCT_XPATH: &str =
	"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.5.1.2.2.1.3']]/hl7:value/@value";
pub const FDA_FACT_FULFIL_EXPEDITED_XPATH: &str =
	"//hl7:component/hl7:observationEvent[hl7:code[@code='23' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.19']]/hl7:value/@value";
pub const FDA_FACT_PREANDA_XPATH: &str =
	"//hl7:researchStudy/hl7:authorization/hl7:studyRegistration/hl7:id[@root='2.16.840.1.113883.3.989.5.1.2.2.1.2.2']/@extension";
pub const FDA_FACT_STUDY_TYPE_XPATH: &str = "//hl7:researchStudy/hl7:code/@code";
pub const FDA_FACT_TYPE_OF_REPORT_XPATH: &str =
	"//hl7:investigationEvent/hl7:subjectOf2/hl7:investigationCharacteristic[hl7:code[@code='1' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value/@code";
pub const FDA_FACT_PRIMARY_SOURCE_NODE_XPATH: &str =
	"//hl7:outboundRelationship[@typeCode='SPRT']/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
pub const FDA_FACT_PRIMARY_SOURCE_EMAIL_XPATH: &str =
	"//hl7:outboundRelationship[@typeCode='SPRT']/hl7:relatedInvestigation/hl7:subjectOf2/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:telecom/@value";

pub const ICH_REACTION_TEMPORAL_RULE_CODE: &str = "ICH.E.i.4-6.CONDITIONAL";
pub const ICH_REACTION_TEMPORAL_RULE_MESSAGE: &str =
	"Reaction requires start, end, or duration";
pub const ICH_DRUG_TEMPORAL_RULE_CODE: &str = "ICH.G.k.4.r.4-8.CONDITIONAL";
pub const ICH_DRUG_TEMPORAL_RULE_MESSAGE: &str =
	"Drug requires start, end, or duration";
pub const ICH_CASE_HISTORY_RULE_CODE: &str = "ICH.C.1.9.1.CONDITIONAL";
pub const ICH_CASE_HISTORY_RULE_MESSAGE: &str =
	"C.1.9.1 is true but C.1.9.1.r.1/.r.2 are missing";
pub const ICH_MEDICAL_HISTORY_RULE_CODE: &str = "ICH.D.7.2.CONDITIONAL";
pub const ICH_MEDICAL_HISTORY_RULE_MESSAGE: &str =
	"D.7.2 must be provided when D.7.1.r.1b is not provided";

pub const FDA_BATCH_RECEIVER_RULE_CODE: &str = "FDA.N.1.4.REQUIRED";
pub const FDA_BATCH_RECEIVER_RULE_MESSAGE: &str =
	"FDA.N.1.4 batch receiver identifier missing";
pub const FDA_LOCAL_CRITERIA_CONDITIONAL_RULE_CODE: &str = "FDA.C.1.7.1.REQUIRED";
pub const FDA_LOCAL_CRITERIA_CONDITIONAL_RULE_MESSAGE: &str =
	"FDA.C.1.7.1 local criteria report type is invalid for current expedited/combination product facts";
pub const FDA_GK10A_RULE_CODE: &str = "FDA.G.k.10a.REQUIRED";
pub const FDA_GK10A_REQUIRED_MESSAGE: &str =
	"FDA.G.k.10a missing: required when FDA.C.5.5b is present";
pub const FDA_GK10A_VALUE_MESSAGE: &str =
	"FDA.G.k.10a must be code 1/2 or nullFlavor NA when FDA.C.5.5b is present";
pub const FDA_REPORTER_EMAIL_RULE_CODE: &str = "FDA.C.2.r.2.EMAIL.REQUIRED";
pub const FDA_REPORTER_EMAIL_RULE_MESSAGE: &str =
	"FDA requires reporter email when primary source is present";
pub const FDA_ICH_C13_CONDITIONAL_RULE_CODE: &str = "ICH.C.1.3.CONDITIONAL";
pub const FDA_ICH_C13_CONDITIONAL_RULE_MESSAGE: &str =
	"C.1.3 must be 2 when premarket receiver and FDA.C.5.5b present with study type 1/2/3";
pub const FDA_PREANDA_REQUIRED_RULE_CODE: &str = "FDA.C.5.5b.REQUIRED";
pub const FDA_PREANDA_REQUIRED_RULE_MESSAGE: &str =
	"FDA.C.5.5b required when C.1.3=2 and N.2.r.3=CDER_IND_EXEMPT_BA_BE";
pub const FDA_PREANDA_FORBIDDEN_RULE_CODE: &str = "FDA.C.5.5b.FORBIDDEN";
pub const FDA_PREANDA_FORBIDDEN_RULE_MESSAGE: &str =
	"FDA.C.5.5b must not be provided for postmarket (N.1.4=ZZFDA, N.2.r.3=CDER/CBER)";

#[cfg(test)]
mod tests {
	use super::*;
	use crate::xml::validate::find_canonical_rule;
	use std::collections::HashSet;

	fn collect_registered_codes() -> Vec<&'static str> {
		let mut codes = Vec::new();
		for rule in ICH_IDENTITY_ATTR_PREFIX_RULES {
			codes.push(rule.rule_code);
		}
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
		for rule in FDA_STATIC_VALUE_NODE_RULES {
			codes.push(rule.rule_code);
		}
		codes.extend([
			ICH_REACTION_TEMPORAL_RULE_CODE,
			ICH_DRUG_TEMPORAL_RULE_CODE,
			ICH_CASE_HISTORY_RULE_CODE,
			ICH_MEDICAL_HISTORY_RULE_CODE,
			FDA_BATCH_RECEIVER_RULE_CODE,
			FDA_LOCAL_CRITERIA_CONDITIONAL_RULE_CODE,
			FDA_GK10A_RULE_CODE,
			FDA_REPORTER_EMAIL_RULE_CODE,
			FDA_ICH_C13_CONDITIONAL_RULE_CODE,
			FDA_PREANDA_REQUIRED_RULE_CODE,
			FDA_PREANDA_FORBIDDEN_RULE_CODE,
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
