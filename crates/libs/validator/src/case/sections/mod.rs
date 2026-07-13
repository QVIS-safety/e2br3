pub(crate) mod c;
pub(crate) mod d;
pub(crate) mod e;
pub(crate) mod f;
pub(crate) mod g;
pub(crate) mod h;
pub(crate) mod n;
pub(crate) mod rule_table;

use crate::{
	FdaValidationContext, MfdsValidationContext, RegulatoryAuthority,
	ValidationContext, ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::{ModelManager, Result};
#[cfg(test)]
use std::collections::BTreeSet;

#[cfg(test)]
pub(crate) fn implemented_allowed_value_rule_codes() -> BTreeSet<&'static str> {
	[
		c::constraint_rule_codes(),
		d::constraint_rule_codes(),
		e::constraint_rule_codes(),
		f::constraint_rule_codes(),
		g::constraint_rule_codes(),
		n::constraint_rule_codes(),
	]
	.into_iter()
	.flatten()
	.collect()
}

pub(crate) async fn collect_section_issues(
	ctx: &Ctx,
	authority: RegulatoryAuthority,
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) -> Result<Vec<ValidationIssue>> {
	let mut issues = Vec::new();
	c::collect(
		&mut issues,
		authority,
		mm,
		ctx,
		validation_ctx,
		fda_ctx,
		mfds_ctx,
	)
	.await?;
	d::collect(&mut issues, authority, validation_ctx, fda_ctx, mfds_ctx);
	e::collect(&mut issues, authority, validation_ctx, fda_ctx);
	f::collect(&mut issues, authority, validation_ctx);
	g::collect(
		&mut issues,
		authority,
		mm,
		ctx,
		validation_ctx,
		fda_ctx,
		mfds_ctx,
	)
	.await?;
	h::collect(&mut issues, authority, validation_ctx);
	n::collect(&mut issues, authority, validation_ctx);
	Ok(issues)
}

pub(crate) fn normalize_validation_field_path(path: &str) -> String {
	path.replace("[]", ".0")
}

pub(crate) fn resolve_validation_field_path(path: Option<&str>) -> Option<String> {
	path.map(normalize_validation_field_path)
}

pub(crate) fn resolve_validation_subsection(
	code: &str,
	path: Option<&str>,
) -> String {
	if code == "ICH.C.1"
		|| code.starts_with("ICH.C.1.")
		|| code == "FDA.C.1"
		|| code.starts_with("FDA.C.1.")
	{
		return "C.1".to_string();
	}
	if code.starts_with("ICH.C.2.")
		|| code.starts_with("FDA.C.2.")
		|| code.starts_with("MFDS.C.2.")
	{
		return "C.2".to_string();
	}
	if code.starts_with("ICH.C.3.") || code.starts_with("MFDS.C.3.") {
		return "C.3".to_string();
	}
	if code.starts_with("ICH.C.5.")
		|| code.starts_with("FDA.C.5.")
		|| code.starts_with("MFDS.C.5.")
	{
		return "C.5".to_string();
	}
	if code.starts_with("ICH.D.10.") || code.starts_with("MFDS.D.10.") {
		return "D.10".to_string();
	}
	if code.starts_with("ICH.D.1.") || code == "ICH.D.1.REQUIRED" {
		return "D.1".to_string();
	}
	if code.starts_with("ICH.D.2.") {
		return "D.2".to_string();
	}
	if code.starts_with("ICH.D.7.1.r.") {
		return "D.7.1.r".to_string();
	}
	if code.starts_with("ICH.D.8.") || code.starts_with("MFDS.D.8.") {
		return "D.8.r".to_string();
	}
	if code.starts_with("ICH.D.")
		|| code.starts_with("FDA.D.")
		|| code.starts_with("MFDS.D.")
	{
		return "D".to_string();
	}
	if code.starts_with("ICH.E.") || code.starts_with("FDA.E.") {
		return "E.i".to_string();
	}
	if code.starts_with("ICH.F.") {
		return "F.r".to_string();
	}
	if code.starts_with("ICH.G.k.4.") {
		return "G.k.4.r".to_string();
	}
	if code.starts_with("ICH.G.")
		|| code.starts_with("FDA.G.")
		|| code.starts_with("MFDS.G.")
		|| code.starts_with("MFDS.KR.")
	{
		return "G.k".to_string();
	}
	if code.starts_with("ICH.H.") {
		return "H".to_string();
	}
	if code.starts_with("ICH.N.") || code.starts_with("FDA.N.") {
		return "N".to_string();
	}

	path.and_then(|value| value.split('.').next())
		.unwrap_or("unknown")
		.to_string()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{canonical_rules_for_phase, find_canonical_rule, ValidationPhase};
	use std::collections::BTreeSet;

	#[test]
	fn implemented_allowed_value_registry_contains_all_current_tables() {
		let codes = implemented_allowed_value_rule_codes();
		assert_eq!(codes.len(), 40);
		assert!(codes.contains("ICH.C.1.3.ALLOWED.VALUE"));
		assert!(codes.contains("ICH.G.k.9.i.4.ALLOWED.VALUE"));
		assert!(codes.contains("ICH.E.i.3.2f.ALLOWED.VALUE"));
		assert!(codes.is_disjoint(&crate::representation_enforced_rule_codes()));
	}

	fn source_rule_codes(source: &str, section_letter: char) -> BTreeSet<String> {
		let prefixes = [
			format!("ICH.{section_letter}."),
			format!("FDA.{section_letter}."),
			format!("MFDS.{section_letter}."),
		];
		source
			.split('"')
			.filter(|segment| {
				prefixes.iter().any(|prefix| segment.starts_with(prefix))
					&& find_canonical_rule(segment).is_some()
			})
			.map(str::to_string)
			.collect()
	}

	fn expected_case_rule_codes(section_letter: char) -> BTreeSet<String> {
		let prefixes = [
			format!("ICH.{section_letter}."),
			format!("FDA.{section_letter}."),
			format!("MFDS.{section_letter}."),
		];
		canonical_rules_for_phase(ValidationPhase::CaseValidate)
			.into_iter()
			.filter(|rule| {
				prefixes.iter().any(|prefix| rule.code.starts_with(prefix))
			})
			.map(|rule| rule.code.to_string())
			.collect()
	}

	#[test]
	fn case_section_sources_cover_catalog_codes_in_canonical_sections() {
		let actual = [
			source_rule_codes(include_str!("c.rs"), 'C'),
			source_rule_codes(include_str!("d.rs"), 'D'),
			source_rule_codes(include_str!("e.rs"), 'E'),
			source_rule_codes(include_str!("f.rs"), 'F'),
			source_rule_codes(include_str!("g.rs"), 'G'),
			source_rule_codes(include_str!("h.rs"), 'H'),
			source_rule_codes(include_str!("n.rs"), 'N'),
		];
		let expected = [
			expected_case_rule_codes('C'),
			expected_case_rule_codes('D'),
			expected_case_rule_codes('E'),
			expected_case_rule_codes('F'),
			expected_case_rule_codes('G'),
			expected_case_rule_codes('H'),
			expected_case_rule_codes('N'),
		];
		for (actual, expected) in actual.into_iter().zip(expected) {
			assert_eq!(actual, expected);
		}
	}

	#[test]
	fn normalizes_array_paths() {
		assert_eq!(
			normalize_validation_field_path("reactions[].outcome"),
			"reactions.0.outcome"
		);
	}

	#[test]
	fn resolves_field_path_from_the_issue_path_only() {
		assert_eq!(resolve_validation_field_path(None), None);
		assert_eq!(
			resolve_validation_field_path(Some(
				"senderInformation.organizationName"
			)),
			Some("senderInformation.organizationName".to_string())
		);
		assert_eq!(
			resolve_validation_field_path(Some("messageHeader[]")),
			Some("messageHeader.0".to_string())
		);
	}

	#[test]
	fn preserves_concrete_indexed_issue_paths_as_field_paths() {
		assert_eq!(
			resolve_validation_field_path(Some(
				"patientInformation.medicalHistory.1.meddraVersion",
			)),
			Some("patientInformation.medicalHistory.1.meddraVersion".to_string())
		);
		assert_eq!(
			resolve_validation_field_path(Some(
				"patientInformation.parents.1.pastDrugs.0.mpidVersion",
			)),
			Some("patientInformation.parents.1.pastDrugs.0.mpidVersion".to_string())
		);
	}

	#[test]
	fn resolves_validation_subsection_from_rule_code() {
		assert_eq!(
			resolve_validation_subsection("ICH.C.1.REQUIRED", None),
			"C.1"
		);
		assert_eq!(
			resolve_validation_subsection("ICH.C.1.2.REQUIRED", None),
			"C.1"
		);
		assert_eq!(
			resolve_validation_subsection("MFDS.C.2.r.1.REQUIRED", None),
			"C.2"
		);
		assert_eq!(
			resolve_validation_subsection("FDA.C.5.5a.REQUIRED", None),
			"C.5"
		);
		assert_eq!(
			resolve_validation_subsection("MFDS.D.10.7.1.r.1.REQUIRED", None),
			"D.10"
		);
		assert_eq!(
			resolve_validation_subsection("ICH.D.2.1.FUTURE_DATE.FORBIDDEN", None),
			"D.2"
		);
		assert_eq!(
			resolve_validation_subsection("ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN", None),
			"D.7.1.r"
		);
		assert_eq!(
			resolve_validation_subsection(
				"ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED",
				None
			),
			"G.k.4.r"
		);
		assert_eq!(
			resolve_validation_subsection("FDA.G.K.12.REQUIRED", None),
			"G.k"
		);
		assert_eq!(
			resolve_validation_subsection(
				"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
				None
			),
			"G.k"
		);
		assert_eq!(
			resolve_validation_subsection("MFDS.KR.FOREIGN.WHOMPID.REQUIRED", None),
			"G.k"
		);
		assert_eq!(
			resolve_validation_subsection(
				"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
				None
			),
			"G.k"
		);
		assert_eq!(resolve_validation_subsection("ICH.N.REQUIRED", None), "N");
	}
}
