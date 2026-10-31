use super::*;

#[test]
fn fda_report_rules_emit_and_pass() {
	let mut issues = Vec::new();
	fda_c_1_7_1(None, true, &mut issues);
	fda_c_1_12(None, None, &mut issues);
	assert_eq!(
		issues
			.iter()
			.map(|issue| issue.code.as_str())
			.collect::<Vec<_>>(),
		["FDA.C.1.7.1.REQUIRED", "FDA.C.1.12.REQUIRED",]
	);

	issues.clear();
	fda_c_1_7_1(Some("1"), true, &mut issues);
	fda_c_1_12(Some("true"), None, &mut issues);
	assert!(issues.is_empty());
}

#[test]
fn mfds_rules_preserve_nonzero_paths_and_conditions() {
	let mut issues = Vec::new();
	mfds_c_3_1_kr_1(2, None, true, &mut issues);
	mfds_c_2_r_4_kr_1(3, None, true, &mut issues);
	mfds_c_5_4_kr_1(4, None, true, &mut issues);

	assert_eq!(issues.len(), 3);
	assert_eq!(
		issues
			.iter()
			.map(|issue| issue.field_path.as_deref().unwrap())
			.collect::<Vec<_>>(),
		[
			"senderInformation.2.healthProfessionalTypeKr1",
			"primarySources.3.qualificationKr1",
			"studyInformation.4.studyTypeReactionKr1",
		]
	);

	issues.clear();
	mfds_c_5_4_kr_1(4, None, false, &mut issues);
	assert!(issues.is_empty());
}
