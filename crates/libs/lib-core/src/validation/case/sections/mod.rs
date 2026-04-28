pub(crate) mod c;
pub(crate) mod d;
pub(crate) mod e;
pub(crate) mod f;
pub(crate) mod g;
pub(crate) mod h;
pub(crate) mod n;

use crate::model::{ModelManager, Result};
use crate::validation::{
	FdaValidationContext, MfdsValidationContext, ValidationContext, ValidationIssue,
	ValidationProfile,
};

pub(crate) async fn collect_section_issues(
	profile: ValidationProfile,
	mm: &ModelManager,
	validation_ctx: &ValidationContext,
	fda_ctx: Option<&FdaValidationContext>,
	mfds_ctx: Option<&MfdsValidationContext>,
) -> Result<Vec<ValidationIssue>> {
	let mut issues = Vec::new();
	c::collect(&mut issues, profile, mm, validation_ctx, fda_ctx, mfds_ctx).await?;
	d::collect(&mut issues, profile, validation_ctx, fda_ctx, mfds_ctx);
	e::collect(&mut issues, profile, validation_ctx, fda_ctx);
	f::collect(&mut issues, profile, validation_ctx);
	g::collect(&mut issues, profile, mm, validation_ctx, fda_ctx, mfds_ctx).await?;
	h::collect(&mut issues, profile, validation_ctx);
	n::collect(&mut issues, profile, validation_ctx);
	Ok(issues)
}

pub(crate) fn normalize_validation_field_path(path: &str) -> String {
	path.replace("[]", ".0")
}

pub(crate) fn canonical_field_path_for_rule(code: &str) -> Option<&'static str> {
	c::field_path_for_rule(code)
		.or_else(|| d::field_path_for_rule(code))
		.or_else(|| e::field_path_for_rule(code))
		.or_else(|| f::field_path_for_rule(code))
		.or_else(|| g::field_path_for_rule(code))
		.or_else(|| h::field_path_for_rule(code))
		.or_else(|| n::field_path_for_rule(code))
}

pub(crate) fn resolve_validation_field_path(
	code: &str,
	path: Option<&str>,
) -> Option<String> {
	canonical_field_path_for_rule(code)
		.map(str::to_string)
		.or_else(|| path.map(normalize_validation_field_path))
}

pub(crate) fn resolve_validation_subsection(
	code: &str,
	path: Option<&str>,
) -> String {
	if code.starts_with("ICH.C.1.") || code.starts_with("FDA.C.1.") {
		return "C.1".to_string();
	}
	if code.starts_with("ICH.C.2.") || code.starts_with("FDA.C.2.") {
		return "C.2".to_string();
	}
	if code.starts_with("ICH.C.3.") || code.starts_with("MFDS.C.3.") {
		return "C.3".to_string();
	}
	if code.starts_with("ICH.C.5.") || code.starts_with("FDA.C.5.") {
		return "C.5".to_string();
	}
	if code.starts_with("ICH.D.10.") {
		return "D.10".to_string();
	}
	if code.starts_with("ICH.D.") || code.starts_with("FDA.D.") {
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
	use crate::validation::{
		canonical_rules_for_phase, find_canonical_rule, ValidationPhase,
	};
	use std::collections::BTreeSet;

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
	fn resolves_canonical_field_path_from_section_owners() {
		assert_eq!(
			resolve_validation_field_path("ICH.C.1.1.REQUIRED", None),
			Some("safetyReportIdentification.safetyReportId".to_string())
		);
		assert_eq!(
			canonical_field_path_for_rule("ICH.N.REQUIRED"),
			Some("messageHeader.messageNumber")
		);
		assert_eq!(
			resolve_validation_field_path(
				"ICH.C.3.2.REQUIRED",
				Some("senderInformation.organizationName"),
			),
			Some("safetyReportIdentification.senderOrganization".to_string())
		);
	}

	#[test]
	fn resolves_validation_subsection_from_rule_code() {
		assert_eq!(
			resolve_validation_subsection("ICH.C.1.2.REQUIRED", None),
			"C.1"
		);
		assert_eq!(
			resolve_validation_subsection("FDA.C.5.5a.REQUIRED", None),
			"C.5"
		);
		assert_eq!(
			resolve_validation_subsection(
				"ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED",
				None
			),
			"G.k.4.r"
		);
	}
}
