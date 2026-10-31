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

	collect_unavailable_meddra_version_issues(&vocabulary, &versions, &mut issues);

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
		"MFDS.D.2.3.ALLOWED.VALUE",
		"ICH.C.4.r.2.ALLOWED.VALUE",
		"ICH.D.6.NULLFLAVOR.ALLOWED",
		"ICH.D.7.1.r.6.ALLOWED.VALUE",
		"ICH.D.7.3.ALLOWED.VALUE",
		"ICH.D.10.2.2b.ALLOWED.VALUE",
		"ICH.D.7.1.r.1a.ALLOWED.VALUE",
		"ICH.D.10.7.1.r.1a.ALLOWED.VALUE",
		"ICH.E.i.2.1a.ALLOWED.VALUE",
		"ICH.F.r.2.2a.ALLOWED.VALUE",
		"ICH.G.k.2.5.ALLOWED.VALUE",
		"ICH.G.k.7.r.2a.ALLOWED.VALUE",
		"ICH.H.3.r.1a.ALLOWED.VALUE",
	] {
		crate::push_field_issue(&mut issues, code, "field", "section", code);
	}
	retain_case_business_rules(&mut issues);
	assert_eq!(issues.len(), 14);
}

#[test]
fn resolves_field_path_from_the_issue_path_only() {
	assert_eq!(resolve_validation_field_path(None), None);
	assert_eq!(
		resolve_validation_field_path(Some("senderInformation.organizationName")),
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
		resolve_validation_subsection("ICH.G.k.4.r.10.NULLFLAVOR.REQUIRED", None),
		"G.k.4.r"
	);
	assert_eq!(
		resolve_validation_subsection("FDA.G.K.12.REQUIRED", None),
		"G.k"
	);
	assert_eq!(
		resolve_validation_subsection("MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED", None),
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
