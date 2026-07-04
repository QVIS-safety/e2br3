pub const EXPORT_RULE_MEDDRA_CODE_FORMAT_REQUIRED: &str =
	"ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED";
pub const EXPORT_RULE_COUNTRY_CODE_FORMAT_REQUIRED: &str =
	"ICH.XML.COUNTRY.CODE.FORMAT.REQUIRED";
pub const EXPORT_RULE_XSI_TYPE_NORMALIZE: &str = "ICH.XML.XSI_TYPE.NORMALIZE";
pub const EXPORT_RULE_OPTIONAL_PATH_EMPTY_PRUNE: &str =
	"ICH.XML.OPTIONAL.PATH.EMPTY.PRUNE";
pub const EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE: &str =
	"ICH.XML.STRUCTURAL.EMPTY.PRUNE";
pub const EXPORT_RULE_PLACEHOLDER_VALUE_PRUNE: &str =
	"ICH.XML.PLACEHOLDER.VALUE.PRUNE";
pub const EXPORT_RULE_PLACEHOLDER_CODESYSTEMVERSION_PRUNE: &str =
	"ICH.XML.PLACEHOLDER.CODESYSTEMVERSION.PRUNE";
pub const EXPORT_RULE_RACE_NI_PRUNE: &str = "ICH.XML.RACE.NI.PRUNE";
pub const EXPORT_RULE_RACE_EMPTY_PRUNE: &str = "ICH.XML.RACE.EMPTY.PRUNE";
pub const EXPORT_RULE_GK11_EMPTY_PRUNE: &str = "ICH.XML.GK11.EMPTY.PRUNE";
pub const EXPORT_RULE_DOCUMENT_TEXT_COMPRESSION_FORBIDDEN: &str =
	"ICH.XML.DOCUMENT.TEXT.COMPRESSION.FORBIDDEN";
pub const EXPORT_RULE_SUMMARY_LANGUAGE_JA_FORBIDDEN: &str =
	"ICH.XML.SUMMARY.LANGUAGE.JA.FORBIDDEN";
pub const EXPORT_RULE_FDA_REQUIRED_INTERVENTION: &str = "FDA.E.i.3.2h.REQUIRED";
pub const EXPORT_RULE_FDA_LOCAL_CRITERIA_REQUIRED: &str = "FDA.C.1.7.1.REQUIRED";
pub const EXPORT_RULE_FDA_COMBINATION_PRODUCT_REQUIRED: &str = "FDA.C.1.12.REQUIRED";

pub const EXPORT_NORMALIZE_INVALID_CODE_RULES: &[&str] = &[
	EXPORT_RULE_MEDDRA_CODE_FORMAT_REQUIRED,
	EXPORT_RULE_COUNTRY_CODE_FORMAT_REQUIRED,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPolicyDirective {
	RequiredInterventionNullFlavorNi,
	ClearNullFlavorWhenValuePresent,
	NormalizeInvalidCodeToNullFlavorNi,
	NormalizeTypeAttributeToXsiType,
	RemoveDocumentTextCompression,
	RemoveSummaryLanguageJa,
	RemovePlaceholderValueNodes,
	RemovePlaceholderCodeSystemVersion,
	RemoveRaceNiNodes,
	RemoveRaceEmptyNodes,
	RemoveEmptyGk11Relationships,
	RemoveOptionalPathEmptyNodes,
	RemoveEmptyStructuralNodes,
}

impl ExportPolicyDirective {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::RequiredInterventionNullFlavorNi => {
				"required_intervention_null_flavor_ni"
			}
			Self::ClearNullFlavorWhenValuePresent => {
				"clear_null_flavor_when_value_present"
			}
			Self::NormalizeInvalidCodeToNullFlavorNi => {
				"normalize_invalid_code_to_null_flavor_ni"
			}
			Self::NormalizeTypeAttributeToXsiType => {
				"normalize_type_attribute_to_xsi_type"
			}
			Self::RemoveDocumentTextCompression => {
				"remove_document_text_compression"
			}
			Self::RemoveSummaryLanguageJa => "remove_summary_language_ja",
			Self::RemovePlaceholderValueNodes => "remove_placeholder_value_nodes",
			Self::RemovePlaceholderCodeSystemVersion => {
				"remove_placeholder_code_system_version"
			}
			Self::RemoveRaceNiNodes => "remove_race_ni_nodes",
			Self::RemoveRaceEmptyNodes => "remove_race_empty_nodes",
			Self::RemoveEmptyGk11Relationships => "remove_empty_gk11_relationships",
			Self::RemoveOptionalPathEmptyNodes => "remove_optional_path_empty_nodes",
			Self::RemoveEmptyStructuralNodes => "remove_empty_structural_nodes",
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportNormalizeKind {
	AsciiDigitsLen(usize),
	AsciiUpperLen(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportNormalizationSpec {
	pub xpath: &'static str,
	pub attribute: &'static str,
	pub kind: ExportNormalizeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportAttributeStripSpec {
	pub xpath: &'static str,
	pub attribute: &'static str,
}

pub fn export_policy_directive_for_rule(
	code: &str,
) -> Option<ExportPolicyDirective> {
	match code {
		EXPORT_RULE_FDA_REQUIRED_INTERVENTION => {
			Some(ExportPolicyDirective::RequiredInterventionNullFlavorNi)
		}
		EXPORT_RULE_FDA_LOCAL_CRITERIA_REQUIRED
		| EXPORT_RULE_FDA_COMBINATION_PRODUCT_REQUIRED => {
			Some(ExportPolicyDirective::ClearNullFlavorWhenValuePresent)
		}
		EXPORT_RULE_MEDDRA_CODE_FORMAT_REQUIRED
		| EXPORT_RULE_COUNTRY_CODE_FORMAT_REQUIRED => {
			Some(ExportPolicyDirective::NormalizeInvalidCodeToNullFlavorNi)
		}
		EXPORT_RULE_XSI_TYPE_NORMALIZE => {
			Some(ExportPolicyDirective::NormalizeTypeAttributeToXsiType)
		}
		EXPORT_RULE_DOCUMENT_TEXT_COMPRESSION_FORBIDDEN => {
			Some(ExportPolicyDirective::RemoveDocumentTextCompression)
		}
		EXPORT_RULE_SUMMARY_LANGUAGE_JA_FORBIDDEN => {
			Some(ExportPolicyDirective::RemoveSummaryLanguageJa)
		}
		EXPORT_RULE_PLACEHOLDER_VALUE_PRUNE => {
			Some(ExportPolicyDirective::RemovePlaceholderValueNodes)
		}
		EXPORT_RULE_PLACEHOLDER_CODESYSTEMVERSION_PRUNE => {
			Some(ExportPolicyDirective::RemovePlaceholderCodeSystemVersion)
		}
		EXPORT_RULE_RACE_NI_PRUNE => Some(ExportPolicyDirective::RemoveRaceNiNodes),
		EXPORT_RULE_RACE_EMPTY_PRUNE => {
			Some(ExportPolicyDirective::RemoveRaceEmptyNodes)
		}
		EXPORT_RULE_GK11_EMPTY_PRUNE => {
			Some(ExportPolicyDirective::RemoveEmptyGk11Relationships)
		}
		EXPORT_RULE_OPTIONAL_PATH_EMPTY_PRUNE => {
			Some(ExportPolicyDirective::RemoveOptionalPathEmptyNodes)
		}
		EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE => {
			Some(ExportPolicyDirective::RemoveEmptyStructuralNodes)
		}
		_ => None,
	}
}

pub fn has_export_policy_directive(
	code: &str,
	directive: ExportPolicyDirective,
) -> bool {
	export_policy_directive_for_rule(code) == Some(directive)
}

pub fn should_clear_null_flavor_on_value(code: &str) -> bool {
	has_export_policy_directive(
		code,
		ExportPolicyDirective::ClearNullFlavorWhenValuePresent,
	)
}

pub fn export_normalization_spec_for_rule(
	code: &str,
) -> Option<ExportNormalizationSpec> {
	match code {
		EXPORT_RULE_MEDDRA_CODE_FORMAT_REQUIRED => Some(ExportNormalizationSpec {
			xpath: "//hl7:value[@codeSystem='2.16.840.1.113883.6.163']",
			attribute: "code",
			kind: ExportNormalizeKind::AsciiDigitsLen(8),
		}),
		EXPORT_RULE_COUNTRY_CODE_FORMAT_REQUIRED => Some(ExportNormalizationSpec {
			xpath: "//hl7:code[@codeSystem='1.0.3166.1.2.2']",
			attribute: "code",
			kind: ExportNormalizeKind::AsciiUpperLen(2),
		}),
		_ => None,
	}
}

pub fn export_xpath_for_rule(code: &str) -> Option<&'static str> {
	match code {
		EXPORT_RULE_RACE_NI_PRUNE => Some("//hl7:observation[hl7:code[@code='C17049' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value[@code='NI']"),
		EXPORT_RULE_RACE_EMPTY_PRUNE => Some("//hl7:observation[hl7:code[@code='C17049' and @codeSystem='2.16.840.1.113883.3.26.1.1']]/hl7:value[not(@code) or @nullFlavor]"),
		EXPORT_RULE_GK11_EMPTY_PRUNE => Some("//hl7:outboundRelationship2[hl7:observation/hl7:code[@code='2'] and (not(hl7:observation/hl7:value) or normalize-space(hl7:observation/hl7:value)='')]"),
		EXPORT_RULE_DOCUMENT_TEXT_COMPRESSION_FORBIDDEN => {
			Some("//hl7:document/hl7:text[@compression]")
		}
		EXPORT_RULE_SUMMARY_LANGUAGE_JA_FORBIDDEN => Some(
			"//hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:value[@language='JA']",
		),
		EXPORT_RULE_FDA_REQUIRED_INTERVENTION => {
			Some("//hl7:observation[hl7:code[@code='7']]/hl7:value")
		}
		_ => None,
	}
}

pub fn export_xpaths_for_rule(code: &str) -> &'static [&'static str] {
	match code {
		EXPORT_RULE_PLACEHOLDER_VALUE_PRUNE => &[
			"//hl7:observation/hl7:value[@code='G.k.10.r']",
			"//hl7:investigationCharacteristic[hl7:code[@code='3' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value[@code='C.1.11.1']",
			"//hl7:observation/hl7:value[@code='D.2.3']",
			"//hl7:observation/hl7:value[@unit='D.2.2.1b']",
		],
		EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE => &[
			"//hl7:outboundRelationship2",
			"//hl7:component",
			"//hl7:component1",
			"//hl7:subjectOf2",
			"//hl7:observation",
			"//hl7:organizer",
		],
		_ => &[],
	}
}

pub fn export_attribute_strip_spec_for_rule(
	code: &str,
) -> Option<ExportAttributeStripSpec> {
	match code {
		EXPORT_RULE_PLACEHOLDER_CODESYSTEMVERSION_PRUNE => {
			Some(ExportAttributeStripSpec {
				xpath: "//hl7:observation/hl7:value[@codeSystemVersion='D.8.r.6a' or @codeSystemVersion='D.8.r.7a' or @codeSystemVersion='D.9.2.r.1a' or @codeSystemVersion='D.9.4.r.1a']",
				attribute: "codeSystemVersion",
			})
		}
		EXPORT_RULE_DOCUMENT_TEXT_COMPRESSION_FORBIDDEN => {
			Some(ExportAttributeStripSpec {
				xpath: "//hl7:document/hl7:text[@compression]",
				attribute: "compression",
			})
		}
		EXPORT_RULE_SUMMARY_LANGUAGE_JA_FORBIDDEN => Some(ExportAttributeStripSpec {
			xpath: "//hl7:component/hl7:observationEvent[hl7:code[@code='36']]/hl7:value[@language='JA']",
			attribute: "language",
		}),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn export_policy_exposes_prune_and_normalize_rules() {
		assert_eq!(
			export_policy_directive_for_rule("ICH.XML.STRUCTURAL.EMPTY.PRUNE"),
			Some(ExportPolicyDirective::RemoveEmptyStructuralNodes)
		);
		assert_eq!(
			export_policy_directive_for_rule("ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED"),
			Some(ExportPolicyDirective::NormalizeInvalidCodeToNullFlavorNi)
		);
		assert!(export_xpaths_for_rule("ICH.XML.STRUCTURAL.EMPTY.PRUNE")
			.contains(&"//hl7:observation"));
		assert!(export_normalization_spec_for_rule(
			"ICH.XML.MEDDRA.CODE.FORMAT.REQUIRED"
		)
		.is_some());
	}
}
