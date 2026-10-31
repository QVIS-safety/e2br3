fn mfds_past(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let parent_rule = code.contains("D.10.8");
	if parent_rule {
		let mut bad = crate::ParentPastDrugByCase {
			parent_id: Uuid::nil(),
			sequence_number: 1,
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			mfds_medicinal_product_id: None,
			mfds_medicinal_product_version: None,
		};
		let mut good;
		let path = if code.contains("KR.1a") {
			bad.mfds_medicinal_product_id = Some("product".into());
			good = bad.clone();
			good.mfds_medicinal_product_version = Some("v1".into());
			"patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductVersion"
		} else if code.contains("KR.1b.VOCABULARY") {
			bad.mfds_medicinal_product_id = Some("bad".into());
			bad.mfds_medicinal_product_version = Some("v1".into());
			good = bad.clone();
			good.mfds_medicinal_product_id = Some("product".into());
			"patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductId"
		} else if code.contains("KR.1b.REQUIRED") {
			good = bad.clone();
			good.mfds_medicinal_product_id = Some("product".into());
			"patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductId"
		} else if code.contains("r.2b") {
			bad.mpid_version = Some("1".into());
			good = bad.clone();
			good.mpid = Some("m".into());
			"patientInformation.parents.0.pastDrugs.0.mpid"
		} else if code.contains("r.3b") {
			bad.phpid_version = Some("1".into());
			good = bad.clone();
			good.phpid = Some("p".into());
			"patientInformation.parents.0.pastDrugs.0.phpid"
		} else {
			panic!("missing MFDS parent past-drug fixture: {code}")
		};
		let mut bad_ctx = empty_ctx();
		let mut good_ctx = empty_ctx();
		if code.ends_with(".VOCABULARY") {
			bad_ctx.vocabulary =
				crate::context::VocabularyContext::for_whodrug(&[("v1", "product")]);
			good_ctx.vocabulary = bad_ctx.vocabulary.clone();
		}
		let run = |row: &crate::ParentPastDrugByCase, ctx: &ValidationContext| {
			let mut i = Vec::new();
			if code.contains("KR.1a") {
				mfds_d_10_8_r_1_kr_1a(0, 0, row, true, &mut i)
			} else if code.contains("KR.1b") {
				mfds_d_10_8_r_1_kr_1b(0, 0, row, true, Some("FR"), ctx, &mut i)
			} else {
				mfds_d_10_8_identifier_companions(0, 0, row, &mut i)
			}
			i
		};
		(path, run(&bad, &bad_ctx), run(&good, &good_ctx))
	} else {
		let mut bad = crate::PastDrugByCase {
			drug_name_null_flavor: None,
			mpid: None,
			mpid_version: None,
			phpid: None,
			phpid_version: None,
			mfds_medicinal_product_id: None,
			mfds_medicinal_product_version: None,
		};
		let mut good;
		let path = if code.contains("KR.1a") {
			bad.mfds_medicinal_product_id = Some("product".into());
			good = bad.clone();
			good.mfds_medicinal_product_version = Some("v1".into());
			"patientInformation.pastDrugHistory.0.mfdsMedicinalProductVersion"
		} else if code.contains("KR.1b.VOCABULARY") {
			bad.mfds_medicinal_product_id = Some("bad".into());
			bad.mfds_medicinal_product_version = Some("v1".into());
			good = bad.clone();
			good.mfds_medicinal_product_id = Some("product".into());
			"patientInformation.pastDrugHistory.0.mfdsMedicinalProductId"
		} else if code.contains("KR.1b.REQUIRED") {
			good = bad.clone();
			good.mfds_medicinal_product_id = Some("product".into());
			"patientInformation.pastDrugHistory.0.mfdsMedicinalProductId"
		} else if code.contains("r.2a") {
			bad.mpid = Some("m".into());
			good = bad.clone();
			good.mpid_version = Some("1".into());
			"patientInformation.pastDrugHistory.0.mpidVersion"
		} else if code.contains("r.2b") {
			bad.mpid_version = Some("1".into());
			good = bad.clone();
			good.mpid = Some("m".into());
			"patientInformation.pastDrugHistory.0.mpid"
		} else if code.contains("r.3a") {
			bad.phpid = Some("p".into());
			good = bad.clone();
			good.phpid_version = Some("1".into());
			"patientInformation.pastDrugHistory.0.phpidVersion"
		} else if code.contains("r.3b") {
			bad.phpid_version = Some("1".into());
			good = bad.clone();
			good.phpid = Some("p".into());
			"patientInformation.pastDrugHistory.0.phpid"
		} else {
			panic!("missing MFDS past-drug fixture: {code}")
		};
		let mut bad_ctx = empty_ctx();
		let mut good_ctx = empty_ctx();
		if code.ends_with(".VOCABULARY") {
			bad_ctx.vocabulary =
				crate::context::VocabularyContext::for_whodrug(&[("v1", "product")]);
			good_ctx.vocabulary = bad_ctx.vocabulary.clone();
		}
		let run = |row: &crate::PastDrugByCase, ctx: &ValidationContext| {
			let mut i = Vec::new();
			if code.contains("KR.1a") {
				mfds_d_8_r_1_kr_1a(0, row, true, &mut i)
			} else if code.contains("KR.1b") {
				mfds_d_8_r_1_kr_1b(0, row, true, Some("FR"), ctx, &mut i)
			} else {
				mfds_d_8_identifier_companions(0, row, &mut i)
			}
			i
		};
		(path, run(&bad, &bad_ctx), run(&good, &good_ctx))
	}
}

fn mfds_required_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let bad = empty_ctx();
	let mut good = empty_ctx();
	let mut p = patient();
	p.age_at_time_of_onset = Some(Decimal::ONE);
	p.age_unit = Some("a".into());
	p.sex = Some("1".into());
	good.patient = Some(p);
	good.patient_identifiers
		.push(patient_identifier("4", "STUDY"));
	let path = match code {
		"MFDS.D.1.1.4.REQUIRED" => "patientInformation.patientStudyNumber",
		"MFDS.D.2.2a.REQUIRED" => "patientInformation.ageAtTimeOfOnset",
		"MFDS.D.2.2b.REQUIRED" => "patientInformation.ageUnit",
		"MFDS.D.5.REQUIRED" => "patientInformation.sex",
		_ => panic!("missing MFDS required fixture: {code}"),
	};
	let run = |ctx: &ValidationContext| {
		let mut i = Vec::new();
		mfds_d_ct_cu(ctx, &mut i);
		i
	};
	(path, run(&bad), run(&good))
}
