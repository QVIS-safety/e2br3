pub(crate) mod c;
pub(crate) mod d;
pub(crate) mod e;
pub(crate) mod f;
pub(crate) mod g;
pub(crate) mod h;
pub(crate) mod helpers;

use crate::{
	FdaValidationContext, MfdsValidationContext, RegulatoryAuthority,
	ValidationContext, ValidationIssue,
};
use lib_core::ctx::Ctx;
use lib_core::model::{ModelManager, Result};
use std::collections::{HashMap, HashSet};

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
	collect_meddra_version_issues(validation_ctx, &mut issues);
	retain_case_business_rules(&mut issues);
	Ok(issues)
}

fn retain_case_business_rules(issues: &mut Vec<ValidationIssue>) {
	issues.retain(|issue| {
		matches!(
			issue.code.as_str(),
			"ICH.C.1.6.1.r.2.ALLOWED.VALUE"
				| "ICH.C.4.r.2.ALLOWED.VALUE"
				| "ICH.D.6.NULLFLAVOR.ALLOWED"
				| "ICH.D.7.1.r.1a.ALLOWED.VALUE"
				| "ICH.D.10.7.1.r.1a.ALLOWED.VALUE"
				| "ICH.E.i.2.1a.ALLOWED.VALUE"
				| "ICH.F.r.2.2a.ALLOWED.VALUE"
				| "ICH.G.k.7.r.2a.ALLOWED.VALUE"
				| "ICH.H.3.r.1a.ALLOWED.VALUE"
		) || (!issue.code.ends_with(".LENGTH.MAX")
			&& !issue.code.ends_with(".ALLOWED.VALUE")
			&& !issue.code.ends_with(".NULLFLAVOR.ALLOWED"))
	});
}

fn has_multiple_values<'a>(values: impl IntoIterator<Item = &'a str>) -> bool {
	values
		.into_iter()
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.collect::<HashSet<_>>()
		.len() > 1
}

fn collect_meddra_version_issues(
	validation_ctx: &ValidationContext,
	issues: &mut Vec<ValidationIssue>,
) {
	let mut versions = Vec::new();
	for (idx, value) in validation_ctx.medical_history.iter().enumerate() {
		versions.push((
			"ICH.D.7.1.r.1a.MEDDRA.VERSION.CONSISTENT",
			format!("patientInformation.medicalHistory.{idx}.meddraVersion"),
			value.meddra_version.as_deref(),
		));
	}
	for (idx, value) in validation_ctx.past_drugs.iter().enumerate() {
		versions.extend([
			(
				"ICH.D.8.r.6a.MEDDRA.VERSION.CONSISTENT",
				format!("patientInformation.pastDrugHistory.{idx}.indicationMeddraVersion"),
				value.indication_meddra_version.as_deref(),
			),
			(
				"ICH.D.8.r.7a.MEDDRA.VERSION.CONSISTENT",
				format!("patientInformation.pastDrugHistory.{idx}.reactionMeddraVersion"),
				value.reaction_meddra_version.as_deref(),
			),
		]);
	}
	for (idx, value) in validation_ctx.reported_causes_of_death.iter().enumerate() {
		versions.push((
			"ICH.D.9.2.r.1a.MEDDRA.VERSION.CONSISTENT",
			format!("patientInformation.death.reportedCauses.{idx}.meddraVersion"),
			value.meddra_version.as_deref(),
		));
	}
	for (idx, value) in validation_ctx.autopsy_causes_of_death.iter().enumerate() {
		versions.push((
			"ICH.D.9.4.r.1a.MEDDRA.VERSION.CONSISTENT",
			format!("patientInformation.death.autopsyCauses.{idx}.meddraVersion"),
			value.meddra_version.as_deref(),
		));
	}
	let parent_indices = validation_ctx
		.parents
		.iter()
		.enumerate()
		.map(|(idx, parent)| (parent.id, idx))
		.collect::<HashMap<_, _>>();
	let mut parent_medical_history_indices = HashMap::new();
	for value in &validation_ctx.parent_medical_history {
		let Some(parent_idx) = parent_indices.get(&value.parent_id).copied() else {
			continue;
		};
		let child_idx = parent_medical_history_indices
			.entry(value.parent_id)
			.or_insert(0);
		let idx = *child_idx;
		*child_idx += 1;
		versions.push((
			"ICH.D.10.7.1.r.1a.MEDDRA.VERSION.CONSISTENT",
			format!("patientInformation.parents.{parent_idx}.medicalHistory.{idx}.meddraVersion"),
			value.meddra_version.as_deref(),
		));
	}
	let mut parent_past_drug_indices = HashMap::new();
	for value in &validation_ctx.parent_past_drugs {
		let Some(parent_idx) = parent_indices.get(&value.parent_id).copied() else {
			continue;
		};
		let child_idx = parent_past_drug_indices.entry(value.parent_id).or_insert(0);
		let idx = *child_idx;
		*child_idx += 1;
		versions.extend([
			(
				"ICH.D.10.8.r.6a.MEDDRA.VERSION.CONSISTENT",
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.indicationMeddraVersion"),
				value.indication_meddra_version.as_deref(),
			),
			(
				"ICH.D.10.8.r.7a.MEDDRA.VERSION.CONSISTENT",
				format!("patientInformation.parents.{parent_idx}.pastDrugs.{idx}.reactionMeddraVersion"),
				value.reaction_meddra_version.as_deref(),
			),
		]);
	}
	for (idx, value) in validation_ctx.reactions.iter().enumerate() {
		versions.push((
			"ICH.E.i.2.1a.MEDDRA.VERSION.CONSISTENT",
			format!("reactions.{idx}.reactionMeddraVersion"),
			value.reaction_meddra_version.as_deref(),
		));
	}
	for (idx, value) in validation_ctx.tests.iter().enumerate() {
		versions.push((
			"ICH.F.r.2.2a.MEDDRA.VERSION.CONSISTENT",
			format!("testResults.{idx}.testMeddraVersion"),
			value.test_meddra_version.as_deref(),
		));
	}
	let drug_indices = validation_ctx
		.drugs
		.iter()
		.enumerate()
		.map(|(idx, drug)| (drug.id, idx))
		.collect::<HashMap<_, _>>();
	let mut indication_indices = HashMap::new();
	for value in &validation_ctx.indications {
		let Some(drug_idx) = drug_indices.get(&value.drug_id).copied() else {
			continue;
		};
		let child_idx = indication_indices.entry(value.drug_id).or_insert(0);
		let idx = *child_idx;
		*child_idx += 1;
		versions.push((
			"ICH.G.k.7.r.2a.MEDDRA.VERSION.CONSISTENT",
			format!("drugs.{drug_idx}.indications.{idx}.indicationMeddraVersion"),
			value.indication_meddra_version.as_deref(),
		));
	}
	for (idx, value) in validation_ctx.sender_diagnoses.iter().enumerate() {
		versions.push((
			"ICH.H.3.r.1a.MEDDRA.VERSION.CONSISTENT",
			format!("narrative.senderDiagnoses.{idx}.diagnosisMeddraVersion"),
			value.diagnosis_meddra_version.as_deref(),
		));
	}
	collect_unavailable_meddra_version_issues(
		&validation_ctx.vocabulary,
		&versions,
		issues,
	);

	if !has_multiple_values(versions.iter().filter_map(|(_, _, value)| *value)) {
		return;
	}
	for (code, path, value) in versions {
		if value.is_some_and(|value| !value.trim().is_empty()) {
			crate::push_business_issue(
				issues,
				code,
				path,
				"Only one MedDRA version may be used in an ICSR",
			);
		}
	}
}

fn collect_unavailable_meddra_version_issues(
	vocabulary: &crate::context::VocabularyContext,
	versions: &[(&str, String, Option<&str>)],
	issues: &mut Vec<ValidationIssue>,
) {
	for (_, path, version) in versions {
		let Some(version) = version.map(str::trim).filter(|value| !value.is_empty())
		else {
			continue;
		};
		if helpers::valid_dotted_version(Some(version))
			&& !vocabulary.contains_meddra_version(version)
		{
			crate::push_business_issue(
				issues,
				"ICH.MEDDRA.VERSION.UNAVAILABLE",
				path,
				format!("MedDRA {version} is not loaded"),
			);
		}
	}
}

pub(crate) fn normalize_validation_field_path(path: &str) -> String {
	path.replace("[]", ".0")
}

pub(crate) fn resolve_validation_field_path(path: Option<&str>) -> Option<String> {
	path.map(normalize_validation_field_path)
}

fn section_and_subsection_from_path(path: &str) -> (&'static str, &'static str) {
	if path.starts_with("messageHeader") {
		("N", "N")
	} else if path.starts_with("safetyReportIdentification") {
		("C", "C.1")
	} else if path.starts_with("primarySources") {
		("C", "C.2")
	} else if path.starts_with("senderInformation") {
		("C", "C.3")
	} else if path.starts_with("documentsHeldBySender")
		|| path.starts_with("literatureReferences")
	{
		("C", "C.4")
	} else if path.starts_with("studyInformation") {
		("C", "C.5")
	} else if path.starts_with("patientInformation.parents") {
		("D", "D.10")
	} else if path.starts_with("patientInformation.medicalHistory") {
		("D", "D.7.1.r")
	} else if path.starts_with("patientInformation.pastDrugHistory") {
		("D", "D.8.r")
	} else if path.starts_with("patientInformation") {
		("D", "D")
	} else if path.starts_with("reactions") {
		("E", "E.i")
	} else if path.starts_with("testResults") || path.starts_with("tests") {
		("F", "F.r")
	} else if path.starts_with("drugs") {
		("G", "G.k")
	} else if path.starts_with("narrative") {
		("H", "H")
	} else {
		("unknown", "unknown")
	}
}

pub(crate) fn resolve_validation_section(code: &str, path: Option<&str>) -> String {
	code.split('.')
		.nth(1)
		.filter(|section| {
			matches!(*section, "C" | "D" | "E" | "F" | "G" | "H" | "N")
		})
		.or_else(|| {
			path.map(section_and_subsection_from_path)
				.map(|value| value.0)
		})
		.unwrap_or("unknown")
		.to_string()
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

	path.map(section_and_subsection_from_path)
		.map(|value| value.1)
		.unwrap_or("unknown")
		.to_string()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalizes_array_paths() {
		assert_eq!(
			normalize_validation_field_path("reactions[].outcome"),
			"reactions.0.outcome"
		);
	}

	#[test]
	fn detects_multiple_non_empty_meddra_versions() {
		assert!(!has_multiple_values(["26.1", " 26.1 ", ""]));
		assert!(has_multiple_values(["26.1", "27.0"]));
	}

	#[test]
	fn unavailable_meddra_release_blocks_submission_for_each_field() {
		let vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("28.1", "10000001")]);
		let versions = vec![
			(
				"ignored",
				"reactions.0.reactionMeddraVersion".to_string(),
				Some("12.0"),
			),
			(
				"ignored",
				"narrative.senderDiagnoses.0.diagnosisMeddraVersion".to_string(),
				Some("12.0"),
			),
			(
				"ignored",
				"testResults.0.testMeddraVersion".to_string(),
				Some("28.1"),
			),
		];
		let mut issues = Vec::new();

		collect_unavailable_meddra_version_issues(
			&vocabulary,
			&versions,
			&mut issues,
		);

		assert_eq!(issues.len(), 2);
		assert!(issues
			.iter()
			.all(|issue| issue.message == "MedDRA 12.0 is not loaded"));
		assert_eq!(issues[0].section, "E");
		assert_eq!(issues[1].section, "H");
	}

	#[test]
	fn case_output_excludes_input_contract_rules() {
		let mut issues = Vec::new();
		for code in [
			"ICH.C.1.1.LENGTH.MAX",
			"ICH.C.1.3.ALLOWED.VALUE",
			"ICH.D.1.NULLFLAVOR.ALLOWED",
			"ICH.C.1.3.REQUIRED",
		] {
			crate::push_field_issue(
				&mut issues,
				code,
				"safetyReportIdentification.reportType",
				"case-identification",
				code,
			);
		}
		retain_case_business_rules(&mut issues);
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].code, "ICH.C.1.3.REQUIRED");
	}

	#[test]
	fn case_output_keeps_migrated_business_rules() {
		let mut issues = Vec::new();
		for code in [
			"ICH.C.1.6.1.r.2.ALLOWED.VALUE",
			"ICH.C.4.r.2.ALLOWED.VALUE",
			"ICH.D.6.NULLFLAVOR.ALLOWED",
			"ICH.D.7.1.r.1a.ALLOWED.VALUE",
			"ICH.D.10.7.1.r.1a.ALLOWED.VALUE",
			"ICH.E.i.2.1a.ALLOWED.VALUE",
			"ICH.F.r.2.2a.ALLOWED.VALUE",
			"ICH.G.k.7.r.2a.ALLOWED.VALUE",
			"ICH.H.3.r.1a.ALLOWED.VALUE",
		] {
			crate::push_field_issue(&mut issues, code, "field", "section", code);
		}
		retain_case_business_rules(&mut issues);
		assert_eq!(issues.len(), 9);
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
		assert_eq!(
			resolve_validation_section(
				"FDA.R0011",
				Some("safetyReportIdentification.fulfilExpeditedCriteria")
			),
			"C"
		);
		assert_eq!(
			resolve_validation_subsection(
				"FDA.R0011",
				Some("safetyReportIdentification.fulfilExpeditedCriteria")
			),
			"C.1"
		);
	}
}
