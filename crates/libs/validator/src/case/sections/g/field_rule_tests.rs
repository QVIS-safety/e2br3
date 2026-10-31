use super::*;

#[test]
fn drug_rules_cover_domestic_foreign_and_unrelated_contexts() {
	let mut issues = Vec::new();
	let vocabulary = crate::context::VocabularyContext::default();
	for (
		index,
		mpid,
		mpid_version,
		product_required,
		version_required,
		domestic_required,
		foreign_required,
	) in [
		(1, None, None, true, false, true, false),
		(2, Some("product"), None, true, true, false, true),
		(3, None, None, false, false, false, false),
	] {
		mfds_g_k_2_1_kr_1b(
			index,
			mpid,
			mpid_version,
			None,
			product_required,
			domestic_required,
			foreign_required,
			&vocabulary,
			&mut issues,
		);
		mfds_g_k_2_1_kr_1a(index, mpid_version, version_required, &mut issues);
	}

	assert_eq!(
		issues
			.iter()
			.map(|issue| issue.code.as_str())
			.collect::<Vec<_>>(),
		[
			"MFDS.G.k.2.1.KR.1b.REQUIRED",
			"MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED",
			"MFDS.G.k.2.1.KR.1a.REQUIRED",
		]
	);
	assert!(issues
		.iter()
		.all(|issue| issue.field_path.as_deref() != Some("drugs.3.mfdsMpid")));
}

#[test]
fn substance_rules_preserve_resolved_drug_and_substance_indices() {
	let mut issues = Vec::new();
	for (
		drug_index,
		substance_index,
		id,
		version,
		domestic_required,
		code_required,
		version_required,
	) in [
		(1, 2, None, None, true, true, false),
		(3, 4, Some("ingredient"), None, false, true, true),
	] {
		mfds_g_k_2_3_r_1_kr_1b(
			drug_index,
			substance_index,
			id,
			version,
			None,
			domestic_required,
			code_required,
			&crate::context::VocabularyContext::default(),
			&mut issues,
		);
		mfds_g_k_2_3_r_1_kr_1a(
			drug_index,
			substance_index,
			version,
			version_required,
			&mut issues,
		);
	}

	assert_eq!(issues.len(), 3);
	assert_eq!(
		issues
			.iter()
			.map(|issue| issue.field_path.as_deref().unwrap())
			.collect::<Vec<_>>(),
		[
			"drugs.1.activeSubstances.2.mfdsId",
			"drugs.1.activeSubstances.2.mfdsId",
			"drugs.3.activeSubstances.4.mfdsVersion",
		]
	);
}

#[test]
fn substance_code_required_rules_keep_domestic_alias_distinct() {
	let mut issues = Vec::new();
	for (drug_index, domestic_required, report_required) in
		[(0, true, false), (1, false, true), (2, false, false)]
	{
		mfds_g_k_2_3_r_1_kr_1b(
			drug_index,
			0,
			None,
			None,
			None,
			domestic_required,
			report_required,
			&crate::context::VocabularyContext::default(),
			&mut issues,
		);
	}

	assert_eq!(
		issues
			.iter()
			.map(|issue| (issue.code.as_str(), issue.field_path.as_deref()))
			.collect::<Vec<_>>(),
		[
			(
				"MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED",
				Some("drugs.0.activeSubstances.0.mfdsId"),
			),
			(
				"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",
				Some("drugs.1.activeSubstances.0.mfdsId"),
			),
		]
	);
}

#[test]
fn relatedness_rules_cover_method_results_while_source_remains_optional() {
	let mut issues = Vec::new();
	for (
		assessment_index,
		method,
		result_kr1,
		result_kr1_null_flavor,
		result_kr2,
		receiver_is_ct_or_cu,
		receiver_is_kr,
		receiver_is_fr,
		method_required,
		kr1_required,
		kr2_required,
	) in [
		(
			2, None, None, None, None, false, false, false, true, false, false,
		),
		(
			3,
			Some("1"),
			None,
			None,
			None,
			false,
			false,
			false,
			true,
			true,
			false,
		),
		(
			4,
			Some("2"),
			None,
			None,
			None,
			true,
			false,
			false,
			true,
			false,
			true,
		),
		(
			5,
			Some("1"),
			None,
			None,
			None,
			false,
			true,
			false,
			false,
			false,
			false,
		),
		(
			6,
			Some("1"),
			None,
			Some("NA"),
			None,
			false,
			false,
			false,
			false,
			true,
			false,
		),
	] {
		mfds_g_k_9_i_2_r_2_kr_1(
			1,
			assessment_index,
			method,
			method_required,
			receiver_is_ct_or_cu,
			receiver_is_kr,
			receiver_is_fr,
			&mut issues,
		);
		mfds_g_k_9_i_2_r_3_kr_1(
			1,
			assessment_index,
			result_kr1,
			result_kr1_null_flavor,
			method,
			kr1_required,
			&mut issues,
		);
		mfds_g_k_9_i_2_r_3_kr_2(
			1,
			assessment_index,
			result_kr2,
			kr2_required,
			&mut issues,
		);
	}

	assert_eq!(
		issues
			.iter()
			.map(|issue| issue.code.as_str())
			.collect::<Vec<_>>(),
		[
			"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",
			"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED",
			"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
		]
	);
}

#[test]
fn mfds_who_umc_kr1_result_is_scoped_to_domestic_reports() {
	assert!(mfds_kr1_result_required(true, true, true));
	assert!(!mfds_kr1_result_required(false, true, true));
	assert!(!mfds_kr1_result_required(true, false, true));
	assert!(!mfds_kr1_result_required(true, true, false));
}
