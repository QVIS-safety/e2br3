use super::*;

#[test]
fn product_code_is_optional_while_foreign_version_companion_is_preserved() {
	let vocabulary = crate::context::VocabularyContext::default();
	let mut issues = Vec::new();
	mfds_g_k_2_1_kr_1b(0, None, None, Some("KR"), &vocabulary, &mut issues);
	mfds_g_k_2_1_kr_1b(1, None, None, Some("FR"), &vocabulary, &mut issues);
	assert!(issues.is_empty());
	mfds_g_k_2_1_kr_1a(1, None, true, &mut issues);
	assert_eq!(issues.len(), 1);
	assert_eq!(issues[0].code, "MFDS.G.k.2.1.KR.1a.REQUIRED");
}

#[test]
fn substance_rules_preserve_resolved_drug_and_substance_indices() {
	let mut issues = Vec::new();
	for (
		drug_index,
		substance_index,
		id,
		version,
		code_required,
		version_required,
	) in [
		(1, 2, None, None, true, false),
		(3, 4, Some("ingredient"), None, true, true),
	] {
		mfds_g_k_2_3_r_1_kr_1b(
			drug_index,
			substance_index,
			id,
			version,
			None,
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

	assert_eq!(issues.len(), 2);
	assert_eq!(
		issues
			.iter()
			.map(|issue| issue.field_path.as_deref().unwrap())
			.collect::<Vec<_>>(),
		[
			"drugs.1.activeSubstances.2.mfdsId",
			"drugs.3.activeSubstances.4.mfdsVersion",
		]
	);
}

#[test]
fn substance_code_fallback_uses_receiver_scope_without_country_alias() {
	let mut issues = Vec::new();
	mfds_g_k_2_3_r_1_kr_1b(
		1,
		0,
		None,
		None,
		Some("KR"),
		true,
		&crate::context::VocabularyContext::default(),
		&mut issues,
	);
	assert_eq!(issues.len(), 1);
	assert_eq!(issues[0].code, "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED");
	issues.clear();
	mfds_g_k_2_3_r_1_kr_1b(
		0,
		0,
		None,
		None,
		None,
		false,
		&crate::context::VocabularyContext::default(),
		&mut issues,
	);
	assert!(issues.is_empty());
}

#[test]
fn relatedness_rules_cover_method_results_while_source_remains_optional() {
	let mut issues = Vec::new();
	for (
		assessment_index,
		method,
		result_kr2,
		receiver_is_ct_or_cu,
		receiver_is_kr,
		receiver_is_fr,
		method_required,
		kr2_required,
	) in [
		(2, None, None, false, false, false, true, false),
		(3, Some("1"), None, false, false, false, true, false),
		(4, Some("2"), None, true, false, false, true, true),
		(5, Some("1"), None, false, true, false, false, false),
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
			"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
		]
	);
}
