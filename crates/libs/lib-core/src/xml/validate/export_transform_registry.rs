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

pub const EXPORT_POSTPROCESS_RULE_CODES: &[&str] = &[
	EXPORT_RULE_MEDDRA_CODE_FORMAT_REQUIRED,
	EXPORT_RULE_COUNTRY_CODE_FORMAT_REQUIRED,
	EXPORT_RULE_XSI_TYPE_NORMALIZE,
	EXPORT_RULE_OPTIONAL_PATH_EMPTY_PRUNE,
	EXPORT_RULE_STRUCTURAL_EMPTY_PRUNE,
	EXPORT_RULE_PLACEHOLDER_VALUE_PRUNE,
	EXPORT_RULE_PLACEHOLDER_CODESYSTEMVERSION_PRUNE,
	EXPORT_RULE_RACE_NI_PRUNE,
	EXPORT_RULE_RACE_EMPTY_PRUNE,
	EXPORT_RULE_GK11_EMPTY_PRUNE,
	EXPORT_RULE_DOCUMENT_TEXT_COMPRESSION_FORBIDDEN,
	EXPORT_RULE_SUMMARY_LANGUAGE_JA_FORBIDDEN,
	EXPORT_RULE_FDA_REQUIRED_INTERVENTION,
	EXPORT_RULE_FDA_LOCAL_CRITERIA_REQUIRED,
	EXPORT_RULE_FDA_COMBINATION_PRODUCT_REQUIRED,
];

#[cfg(test)]
mod tests {
	use super::*;
	use crate::xml::validate::{
		canonical_rules_for_phase, find_canonical_rule_for_phase, ValidationPhase,
	};
	use std::collections::HashSet;

	#[test]
	fn export_registry_codes_are_catalog_backed_and_unique() {
		let mut seen = HashSet::new();
		for code in EXPORT_POSTPROCESS_RULE_CODES {
			assert!(seen.insert(*code), "duplicate export registry code: {code}");
			assert!(
				find_canonical_rule_for_phase(code, ValidationPhase::Export)
					.is_some(),
				"export registry code missing export-phase catalog rule: {code}"
			);
		}
	}

	#[test]
	fn export_registry_exactly_matches_catalog_export_phase_codes() {
		let registry_codes: HashSet<&str> =
			EXPORT_POSTPROCESS_RULE_CODES.iter().copied().collect();
		let catalog_codes: HashSet<&str> =
			canonical_rules_for_phase(ValidationPhase::Export)
				.into_iter()
				.map(|rule| rule.code)
				.collect();
		assert_eq!(
			registry_codes, catalog_codes,
			"export registry/catalog phase drift detected"
		);
	}
}
