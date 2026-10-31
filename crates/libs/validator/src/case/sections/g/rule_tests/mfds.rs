use super::*;

fn verify_mfds(code: &str) {
	let (path, b, g) = match code {
		"MFDS.G.k.2.1.1a.REQUIRED"
		| "MFDS.G.k.2.1.1b.REQUIRED"
		| "MFDS.G.k.2.1.2a.REQUIRED"
		| "MFDS.G.k.2.1.2b.REQUIRED" => {
			let mut d = fixture::drug();
			match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => d.mpid = Some("id".into()),
				"MFDS.G.k.2.1.1b.REQUIRED" => d.mpid_version = Some("v".into()),
				"MFDS.G.k.2.1.2a.REQUIRED" => d.phpid = Some("id".into()),
				_ => d.phpid_version = Some("v".into()),
			};
			let mut b = vec![];
			mfds_g_k_2_1_companions(0, &d, &mut b);
			match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => d.mpid_version = Some("v".into()),
				"MFDS.G.k.2.1.1b.REQUIRED" => d.mpid = Some("id".into()),
				"MFDS.G.k.2.1.2a.REQUIRED" => d.phpid_version = Some("v".into()),
				_ => d.phpid = Some("id".into()),
			};
			let mut g = vec![];
			mfds_g_k_2_1_companions(0, &d, &mut g);
			let p = match code {
				"MFDS.G.k.2.1.1a.REQUIRED" => "drugs.0.mpidVersion",
				"MFDS.G.k.2.1.1b.REQUIRED" => "drugs.0.mpid",
				"MFDS.G.k.2.1.2a.REQUIRED" => "drugs.0.phpidVersion",
				_ => "drugs.0.phpid",
			};
			(p, b, g)
		}
		"MFDS.G.k.2.1.KR.1a.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_2_1_kr_1a(0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_2_1_kr_1a(0, Some("v"), true, &mut g);
			("drugs.0.mfdsMpidVersion", b, g)
		}
		"MFDS.G.k.2.1.KR.1b.VOCABULARY" => {
			let bad = crate::context::VocabularyContext::for_active_codes(&[(
				"MFDS_PRODUCT",
				VocabularyScope::ItemSeq,
				"good",
			)]);
			let mut b = vec![];
			mfds_g_k_2_1_kr_1b(0, Some("bad"), None, Some("KR"), &bad, &mut b);
			let mut g = vec![];
			mfds_g_k_2_1_kr_1b(0, Some("good"), None, Some("KR"), &bad, &mut g);
			("drugs.0.mfdsMpid", b, g)
		}
		"MFDS.G.k.2.3.r.2b.REQUIRED" => {
			let mut s = fixture::substance();
			s.substance_termid_version = Some("v".into());
			let mut b = vec![];
			mfds_g_k_2_3_r_2b(0, 0, &s, &mut b);
			s.substance_termid = Some("id".into());
			let mut g = vec![];
			mfds_g_k_2_3_r_2b(0, 0, &s, &mut g);
			("drugs.0.activeSubstances.0.substanceTermId", b, g)
		}
		"MFDS.G.k.2.3.r.2a.REQUIRED" => {
			let mut substance = fixture::substance();
			let mut not_triggered = Vec::new();
			mfds_g_k_2_3_r_2a(0, 0, &substance, &mut not_triggered);
			assert!(!not_triggered.iter().any(|issue| issue.code == code));
			substance.substance_termid = Some("id".into());
			let mut b = Vec::new();
			mfds_g_k_2_3_r_2a(0, 0, &substance, &mut b);
			substance.substance_termid_version = Some("v".into());
			let mut g = Vec::new();
			mfds_g_k_2_3_r_2a(0, 0, &substance, &mut g);
			("drugs.0.activeSubstances.0.substanceTermIdVersion", b, g)
		}
		"MFDS.G.k.4.r.9.2a.REQUIRED" | "MFDS.G.k.4.r.10.2a.REQUIRED" => {
			let mut dosage = fixture::dosage();
			let mut not_triggered = Vec::new();
			mfds_g_k_4_termid_versions(0, 0, &dosage, &mut not_triggered);
			assert!(!not_triggered.iter().any(|issue| issue.code == code));
			if code.contains("r.9") {
				dosage.dose_form_termid = Some("id".into());
			} else {
				dosage.route_termid = Some("id".into());
			}
			let mut b = Vec::new();
			mfds_g_k_4_termid_versions(0, 0, &dosage, &mut b);
			if code.contains("r.9") {
				dosage.dose_form_termid_version = Some("v".into());
			} else {
				dosage.route_termid_version = Some("v".into());
			}
			let mut g = Vec::new();
			mfds_g_k_4_termid_versions(0, 0, &dosage, &mut g);
			(
				if code.contains("r.9") {
					"drugs.0.dosageInformation.0.doseFormTermIdVersion"
				} else {
					"drugs.0.dosageInformation.0.routeTermIdVersion"
				},
				b,
				g,
			)
		}
		"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED" => {
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1a(0, 0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1a(0, 0, Some("v"), true, &mut g);
			("drugs.0.activeSubstances.0.mfdsVersion", b, g)
		}
		"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED" => {
			let v = Default::default();
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1b(0, 0, None, None, None, true, &v, &mut b);
			let mut ctx = fixture::empty_ctx();
			ctx.message_header = Some(header("MFDS-O-KR"));
			ctx.drugs.push(fixture::drug());
			collect_mfds_issues(&ctx, &mfds_empty(), &mut b);
			let collect_paths = |ids: &[Option<&str>]| {
				let mut case_ctx = fixture::empty_ctx();
				case_ctx.message_header = Some(header("MFDS-O-KR"));
				case_ctx.drugs.push(fixture::drug());
				let mut mfds = mfds_empty();
				for id in ids {
					let mut substance = fixture::substance();
					substance.mfds_id = id.map(str::to_string);
					mfds.active_substances.push(substance);
				}
				let mut issues = Vec::new();
				collect_mfds_issues(&case_ctx, &mfds, &mut issues);
				issues
					.into_iter()
					.filter(|issue| issue.code == code)
					.map(|issue| issue.path)
					.collect::<Vec<_>>()
			};
			assert_eq!(collect_paths(&[]), ["drugs.0.activeSubstances".to_string()]);
			assert_eq!(
				collect_paths(&[None]),
				["drugs.0.activeSubstances.0.mfdsId".to_string()]
			);
			assert_eq!(
				collect_paths(&[None, None]),
				[
					"drugs.0.activeSubstances.0.mfdsId".to_string(),
					"drugs.0.activeSubstances.1.mfdsId".to_string(),
				]
			);
			assert_eq!(
				collect_paths(&[Some("ingredient"), None]),
				["drugs.0.activeSubstances.1.mfdsId".to_string()]
			);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1b(0, 0, Some("id"), None, None, true, &v, &mut g);
			("drugs.0.activeSubstances.0.mfdsId", b, g)
		}
		"MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY" => {
			let v = crate::context::VocabularyContext::for_active_codes(&[(
				"MFDS_SUBSTANCE",
				VocabularyScope::All,
				"good",
			)]);
			let mut b = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0,
				0,
				Some("bad"),
				None,
				Some("KR"),
				false,
				&v,
				&mut b,
			);
			let mut g = vec![];
			mfds_g_k_2_3_r_1_kr_1b(
				0,
				0,
				Some("good"),
				None,
				Some("KR"),
				false,
				&v,
				&mut g,
			);
			("drugs.0.activeSubstances.0.mfdsId", b, g)
		}
		"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED" => {
			let source_only = mfds_causality_issues(
				"MFDS-O-KR",
				"1",
				Some("reporter"),
				None,
				None,
			);
			assert!(!source_only.iter().any(|issue| issue.code == code));
			for (value, ct_or_cu, kr, fr, rejected) in [
				(Some("2"), true, false, false, false),
				(Some("1"), true, false, false, true),
				(Some("1"), false, true, false, false),
				(Some("1"), false, false, true, true),
			] {
				let mut route_issues = Vec::new();
				mfds_g_k_9_i_2_r_2_kr_1(
					0,
					0,
					value,
					false,
					ct_or_cu,
					kr,
					fr,
					&mut route_issues,
				);
				assert_eq!(
					route_issues.iter().any(|issue| issue.code == code),
					rejected
				);
			}
			let mut b = vec![];
			mfds_g_k_9_i_2_r_2_kr_1(0, 0, None, true, false, false, false, &mut b);
			let mut g = vec![];
			mfds_g_k_9_i_2_r_2_kr_1(
				0,
				0,
				Some("1"),
				true,
				false,
				true,
				false,
				&mut g,
			);
			(
				"drugs.0.drugReactionAssessments.0.methodOfAssessmentKr1",
				b,
				g,
			)
		}
		"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED" => {
			for (receiver, method, required) in [
				("MFDS-O-CT", Some("2"), true),
				("MFDS-O-CU", Some("2"), true),
				("MFDS-O-KR", Some("2"), false),
				("MFDS-O-FR", Some("2"), false),
				("MFDS-O-CT", Some("1"), false),
				("MFDS-O-CU", None, false),
			] {
				let issues = mfds_causality_issues(
					receiver,
					"2",
					Some("Sponsor"),
					method,
					None,
				);
				assert_eq!(
					issues.iter().any(|issue| issue.code == code),
					required,
					"receiver={receiver}, method={method:?}: {issues:?}"
				);
			}
			let clinical =
				mfds_causality_issues("MFDS-O-CT", "1", None, Some("2"), None);
			assert!(clinical.iter().any(|issue| issue.code == code));
			let mut b = vec![];
			mfds_g_k_9_i_2_r_3_kr_2(0, 0, None, true, &mut b);
			let mut g = vec![];
			mfds_g_k_9_i_2_r_3_kr_2(0, 0, Some("1"), true, &mut g);
			(
				"drugs.0.drugReactionAssessments.0.resultOfAssessmentKr2",
				b,
				g,
			)
		}
		"MFDS.G.k.8.REQUIRED" => {
			let mut c = fixture::empty_ctx();
			c.drugs.push(fixture::drug());
			for receiver in ["MFDS-O-KR", "MFDS-O-FR"] {
				c.message_header = Some(header(receiver));
				let mut optional = Vec::new();
				collect_mfds_issues(&c, &mfds_empty(), &mut optional);
				assert!(!optional.iter().any(|issue| issue.code == code));
			}
			c.message_header = Some(header("MFDS-O-CT"));
			let m = mfds_empty();
			let mut b = vec![];
			collect_mfds_issues(&c, &m, &mut b);
			c.drugs[0].action_taken = Some("1".into());
			let mut g = vec![];
			collect_mfds_issues(&c, &m, &mut g);
			c.message_header = Some(header("MFDS-O-CU"));
			c.drugs[0].action_taken = None;
			let mut compassionate = Vec::new();
			collect_mfds_issues(&c, &m, &mut compassionate);
			assert!(compassionate.iter().any(|issue| issue.code == code));
			("drugs.0.actionTaken", b, g)
		}
		_ => panic!("unsupported {code}"),
	};
	edge(code, path, b, g)
}

fn mfds_causality_issues(
	receiver: &str,
	report_type: &str,
	source: Option<&str>,
	method: Option<&str>,
	result_null_flavor: Option<&str>,
) -> Vec<ValidationIssue> {
	let mut ctx = fixture::empty_ctx();
	ctx.message_header = Some(header(receiver));
	ctx.safety_report = Some(report(report_type));
	ctx.drugs.push(fixture::drug());
	ctx.drug_reaction_assessments.push(fixture::assessment());
	let mut mfds = mfds_empty();
	mfds.relatedness
		.push(crate::mfds_context::RelatednessWithDrug {
			drug_id: Uuid::nil(),
			drug_reaction_assessment_id: Uuid::nil(),
			relatedness_sequence_number: 1,
			source_of_assessment: source.map(str::to_string),
			method_of_assessment: None,
			method_of_assessment_kr1: method.map(str::to_string),
			result_of_assessment: None,
			result_of_assessment_kr1: None,
			result_of_assessment_kr1_null_flavor: result_null_flavor
				.map(str::to_string),
			result_of_assessment_kr2: None,
		});
	let mut issues = Vec::new();
	collect_mfds_issues(&ctx, &mfds, &mut issues);
	issues
}

macro_rules! tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_mfds($c);})+};}
tests! {
	rule_mfds_g_k_2_1_1a_required=>"MFDS.G.k.2.1.1a.REQUIRED",rule_mfds_g_k_2_1_1b_required=>"MFDS.G.k.2.1.1b.REQUIRED",rule_mfds_g_k_2_1_2a_required=>"MFDS.G.k.2.1.2a.REQUIRED",rule_mfds_g_k_2_1_2b_required=>"MFDS.G.k.2.1.2b.REQUIRED",rule_mfds_g_k_2_1_kr_1a_required=>"MFDS.G.k.2.1.KR.1a.REQUIRED",rule_mfds_g_k_2_1_kr_1b_vocabulary=>"MFDS.G.k.2.1.KR.1b.VOCABULARY",rule_mfds_g_k_2_3_r_1_kr_1a_required=>"MFDS.G.k.2.3.r.1.KR.1a.REQUIRED",rule_mfds_g_k_2_3_r_1_kr_1b_required=>"MFDS.G.k.2.3.r.1.KR.1b.REQUIRED",rule_mfds_g_k_2_3_r_1_kr_1b_vocabulary=>"MFDS.G.k.2.3.r.1.KR.1b.VOCABULARY",rule_mfds_g_k_2_3_r_2b_required=>"MFDS.G.k.2.3.r.2b.REQUIRED",rule_mfds_g_k_2_3_r_2a_required=>"MFDS.G.k.2.3.r.2a.REQUIRED",rule_mfds_g_k_4_r_9_2a_required=>"MFDS.G.k.4.r.9.2a.REQUIRED",rule_mfds_g_k_4_r_10_2a_required=>"MFDS.G.k.4.r.10.2a.REQUIRED",rule_mfds_g_k_8_required=>"MFDS.G.k.8.REQUIRED",rule_mfds_g_k_9_i_2_r_2_kr_1_required=>"MFDS.G.k.9.i.2.r.2.KR.1.REQUIRED",rule_mfds_g_k_9_i_2_r_3_kr_2_required=>"MFDS.G.k.9.i.2.r.3.KR.2.REQUIRED",
}

#[test]
fn mfds_who_umc_result_is_optional_at_the_business_rule_layer() {
	for null_flavor in [None, Some("NA")] {
		let issues =
			mfds_causality_issues("MFDS-O-KR", "1", None, None, null_flavor);
		assert!(!issues.iter().any(|issue| matches!(
			issue.code.as_str(),
			"MFDS.G.k.9.i.2.r.3.KR.1.REQUIRED"
				| "MFDS.G.k.9.i.2.r.3.KR.1.ALLOWED.VALUE"
		)));
	}
}

#[test]
fn product_or_ingredient_acceptance_does_not_depend_on_obtain_country() {
	for country in ["KR", "US"] {
		let mut ctx = fixture::empty_ctx();
		ctx.message_header = Some(header("MFDS-O-KR"));
		let mut drug = fixture::drug();
		drug.obtain_drug_country = Some(country.into());
		ctx.drugs.push(drug);
		let mut mfds = mfds_empty();
		let mut substance = fixture::substance();
		substance.mfds_id = Some("ingredient".into());
		mfds.active_substances.push(substance);
		let mut issues = Vec::new();
		collect_mfds_issues(&ctx, &mfds, &mut issues);
		assert!(!issues.iter().any(|issue| matches!(
			issue.code.as_str(),
			"MFDS.G.k.2.1.KR.1b.REQUIRED"
				| "MFDS.KR.DOMESTIC.PRODUCTCODE.REQUIRED"
				| "MFDS.KR.FOREIGN.WHOMPID.REQUIRED"
				| "MFDS.KR.DOMESTIC.INGREDIENTCODE.REQUIRED"
				| "MFDS.G.k.2.3.r.1.KR.1b.REQUIRED"
		)));
	}
}
