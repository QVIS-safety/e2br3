use super::*;
use crate::VocabularyScope;

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

fn assert_rule(code: &str, expected_path: &str) {
	let (path, invalid, repaired) =
		if code.starts_with("MFDS.D.8") || code.starts_with("MFDS.D.10.8") {
			mfds_past(code)
		} else {
			mfds_required_edge(code)
		};
	assert_eq!(expected_path, path, "stale declared path for {code}");
	assert_edge(code, path, invalid, repaired);
}

macro_rules! rule_tests {
	($($name:ident: ($code:literal, $path:literal),)+) => {
		$(
			#[test]
			fn $name() {
				assert_rule($code, $path);
			}
		)+
	};
}

rule_tests! {
	rule_mfds_d_10_8_r_1_kr_1b_required: ("MFDS.D.10.8.r.1.KR.1b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductId"),
	rule_mfds_d_10_8_r_2b_required: ("MFDS.D.10.8.r.2b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.mpid"),
	rule_mfds_d_10_8_r_3b_required: ("MFDS.D.10.8.r.3b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.phpid"),
	rule_mfds_d_8_r_1_kr_1b_required: ("MFDS.D.8.r.1.KR.1b.REQUIRED", "patientInformation.pastDrugHistory.0.mfdsMedicinalProductId"),
	rule_mfds_d_8_r_2a_required: ("MFDS.D.8.r.2a.REQUIRED", "patientInformation.pastDrugHistory.0.mpidVersion"),
	rule_mfds_d_8_r_2b_required: ("MFDS.D.8.r.2b.REQUIRED", "patientInformation.pastDrugHistory.0.mpid"),
	rule_mfds_d_8_r_3a_required: ("MFDS.D.8.r.3a.REQUIRED", "patientInformation.pastDrugHistory.0.phpidVersion"),
	rule_mfds_d_8_r_3b_required: ("MFDS.D.8.r.3b.REQUIRED", "patientInformation.pastDrugHistory.0.phpid"),
}

fn empty_mfds_context() -> MfdsValidationContext {
	MfdsValidationContext {
		senders: Vec::new(),
		studies: Vec::new(),
		active_substances: Vec::new(),
		relatedness: Vec::new(),
		past_drugs: Vec::new(),
		parent_past_drugs: Vec::new(),
	}
}

fn mfds_route_issues(
	receiver: &str,
	repair: impl FnOnce(&mut ValidationContext),
) -> Vec<ValidationIssue> {
	let mut ctx = empty_ctx();
	ctx.message_header = Some(message(receiver));
	repair(&mut ctx);
	let mut issues = Vec::new();
	collect_mfds_issues(&ctx, &empty_mfds_context(), &mut issues);
	issues
}

fn assert_mfds_ct_cu_route(code: &str, repair: impl Fn(&mut ValidationContext)) {
	for receiver in ["MFDS-O-CT", "MFDS-O-CU", "MFDS-O-KR", "MFDS-O-FR"] {
		let invalid = mfds_route_issues(receiver, |_| {});
		assert_eq!(
			invalid.iter().any(|issue| issue.code == code),
			matches!(receiver, "MFDS-O-CT" | "MFDS-O-CU"),
			"unexpected {code} routing for {receiver}: {invalid:?}"
		);
		let repaired = mfds_route_issues(receiver, |ctx| repair(ctx));
		assert!(
			!repaired.iter().any(|issue| issue.code == code),
			"{code} remained after repair for {receiver}: {repaired:?}"
		);
	}
}

#[test]
fn rule_mfds_d_1_1_4_required() {
	assert_rule(
		"MFDS.D.1.1.4.REQUIRED",
		"patientInformation.patientStudyNumber",
	);
	assert_mfds_ct_cu_route("MFDS.D.1.1.4.REQUIRED", |ctx| {
		ctx.patient_identifiers
			.push(patient_identifier("4", "STUDY"));
	});
}

#[test]
fn rule_mfds_d_2_2a_required() {
	assert_rule(
		"MFDS.D.2.2a.REQUIRED",
		"patientInformation.ageAtTimeOfOnset",
	);
	assert_mfds_ct_cu_route("MFDS.D.2.2a.REQUIRED", |ctx| {
		let mut value = patient();
		value.age_at_time_of_onset = Some(Decimal::ONE);
		value.age_unit = Some("a".into());
		value.sex = Some("1".into());
		ctx.patient = Some(value);
	});
}

#[test]
fn rule_mfds_d_2_2b_required() {
	assert_rule("MFDS.D.2.2b.REQUIRED", "patientInformation.ageUnit");
	assert_mfds_ct_cu_route("MFDS.D.2.2b.REQUIRED", |ctx| {
		let mut value = patient();
		value.age_at_time_of_onset = Some(Decimal::ONE);
		value.age_unit = Some("a".into());
		value.sex = Some("1".into());
		ctx.patient = Some(value);
	});
}

#[test]
fn rule_mfds_d_5_required() {
	assert_rule("MFDS.D.5.REQUIRED", "patientInformation.sex");
	assert_mfds_ct_cu_route("MFDS.D.5.REQUIRED", |ctx| {
		let mut value = patient();
		value.age_at_time_of_onset = Some(Decimal::ONE);
		value.age_unit = Some("a".into());
		value.sex = Some("1".into());
		ctx.patient = Some(value);
	});
}

#[test]
fn rule_mfds_d_8_r_1_kr_1a_required() {
	assert_rule(
		"MFDS.D.8.r.1.KR.1a.REQUIRED",
		"patientInformation.pastDrugHistory.0.mfdsMedicinalProductVersion",
	);
	let mut row = crate::PastDrugByCase {
		drug_name_null_flavor: None,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		mfds_medicinal_product_id: Some("product".into()),
		mfds_medicinal_product_version: None,
	};
	for receiver_is_fr in [false, true] {
		let issues = invoke(&row, |row, issues| {
			mfds_d_8_r_1_kr_1a(0, row, receiver_is_fr, issues)
		});
		assert_eq!(
			issues
				.iter()
				.any(|issue| issue.code == "MFDS.D.8.r.1.KR.1a.REQUIRED"),
			receiver_is_fr
		);
	}
	row.mfds_medicinal_product_id = None;
	assert!(
		!invoke(&row, |row, issues| mfds_d_8_r_1_kr_1a(0, row, true, issues))
			.iter()
			.any(|issue| issue.code == "MFDS.D.8.r.1.KR.1a.REQUIRED")
	);
}

#[test]
fn rule_mfds_d_10_8_r_1_kr_1a_required() {
	assert_rule(
		"MFDS.D.10.8.r.1.KR.1a.REQUIRED",
		"patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductVersion",
	);
	let mut row = crate::ParentPastDrugByCase {
		parent_id: Uuid::nil(),
		sequence_number: 1,
		mpid: None,
		mpid_version: None,
		phpid: None,
		phpid_version: None,
		mfds_medicinal_product_id: Some("product".into()),
		mfds_medicinal_product_version: None,
	};
	for receiver_is_fr in [false, true] {
		let issues = invoke(&row, |row, issues| {
			mfds_d_10_8_r_1_kr_1a(0, 0, row, receiver_is_fr, issues)
		});
		assert_eq!(
			issues
				.iter()
				.any(|issue| issue.code == "MFDS.D.10.8.r.1.KR.1a.REQUIRED"),
			receiver_is_fr
		);
	}
	row.mfds_medicinal_product_id = None;
	assert!(!invoke(&row, |row, issues| mfds_d_10_8_r_1_kr_1a(
		0, 0, row, true, issues
	))
	.iter()
	.any(|issue| issue.code == "MFDS.D.10.8.r.1.KR.1a.REQUIRED"));
}

#[test]
fn rule_mfds_d_8_r_1_kr_1b_vocabulary() {
	assert_rule(
		"MFDS.D.8.r.1.KR.1b.VOCABULARY",
		"patientInformation.pastDrugHistory.0.mfdsMedicinalProductId",
	);
	for receiver in ["MFDS-O-KR", "MFDS-O-FR"] {
		let run = |product: &str| {
			let mut ctx = empty_ctx();
			ctx.message_header = Some(message(receiver));
			ctx.vocabulary = if receiver == "MFDS-O-KR" {
				crate::context::VocabularyContext::for_active_codes(&[(
					"MFDS_PRODUCT",
					VocabularyScope::ItemSeq,
					"product",
				)])
			} else {
				crate::context::VocabularyContext::for_whodrug(&[("v1", "product")])
			};
			let mut mfds_ctx = empty_mfds_context();
			mfds_ctx.past_drugs.push(crate::PastDrugByCase {
				drug_name_null_flavor: None,
				mpid: None,
				mpid_version: None,
				phpid: None,
				phpid_version: None,
				mfds_medicinal_product_id: Some(product.into()),
				mfds_medicinal_product_version: Some("v1".into()),
			});
			let mut issues = Vec::new();
			collect_mfds_issues(&ctx, &mfds_ctx, &mut issues);
			issues
		};
		let invalid = run("bad");
		assert!(invalid
			.iter()
			.any(|issue| issue.code == "MFDS.D.8.r.1.KR.1b.VOCABULARY"));
		let repaired = run("product");
		assert!(!repaired
			.iter()
			.any(|issue| issue.code == "MFDS.D.8.r.1.KR.1b.VOCABULARY"));
	}
}

#[test]
fn rule_mfds_d_10_8_r_1_kr_1b_vocabulary() {
	assert_rule(
		"MFDS.D.10.8.r.1.KR.1b.VOCABULARY",
		"patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductId",
	);
	for receiver in ["MFDS-O-KR", "MFDS-O-FR"] {
		let run = |product: &str| {
			let parent_id = Uuid::nil();
			let mut ctx = empty_ctx();
			ctx.message_header = Some(message(receiver));
			ctx.vocabulary = if receiver == "MFDS-O-KR" {
				crate::context::VocabularyContext::for_active_codes(&[(
					"MFDS_PRODUCT",
					VocabularyScope::ItemSeq,
					"product",
				)])
			} else {
				crate::context::VocabularyContext::for_whodrug(&[("v1", "product")])
			};
			ctx.parents.push(parent(parent_id));
			let mut mfds_ctx = empty_mfds_context();
			mfds_ctx
				.parent_past_drugs
				.push(crate::ParentPastDrugByCase {
					parent_id,
					sequence_number: 1,
					mpid: None,
					mpid_version: None,
					phpid: None,
					phpid_version: None,
					mfds_medicinal_product_id: Some(product.into()),
					mfds_medicinal_product_version: Some("v1".into()),
				});
			let mut issues = Vec::new();
			collect_mfds_issues(&ctx, &mfds_ctx, &mut issues);
			issues
		};
		let invalid = run("bad");
		assert!(invalid
			.iter()
			.any(|issue| issue.code == "MFDS.D.10.8.r.1.KR.1b.VOCABULARY"));
		let repaired = run("product");
		assert!(!repaired
			.iter()
			.any(|issue| issue.code == "MFDS.D.10.8.r.1.KR.1b.VOCABULARY"));
	}
}

#[test]
fn rule_mfds_d_2_3_allowed_value() {
	for allowed_value in ["1", "6"] {
		let mut value = patient();
		value.age_group = Some(allowed_value.into());
		let issues = invoke(&Some(value), |patient, issues| {
			mfds_d_2_3(patient.as_ref(), issues)
		});
		assert!(
			!issues
				.iter()
				.any(|issue| issue.code == "MFDS.D.2.3.ALLOWED.VALUE"),
			"allowed MFDS age group {allowed_value} was rejected: {issues:?}"
		);
	}
	let mut invalid = patient();
	invalid.age_group = Some("0".into());
	assert_edge(
		"MFDS.D.2.3.ALLOWED.VALUE",
		"patientInformation.patientAgeGroup",
		invoke(&Some(invalid), |patient, issues| {
			mfds_d_2_3(patient.as_ref(), issues)
		}),
		invoke(&None, |patient: &Option<PatientInformation>, issues| {
			mfds_d_2_3(patient.as_ref(), issues)
		}),
	);
}
