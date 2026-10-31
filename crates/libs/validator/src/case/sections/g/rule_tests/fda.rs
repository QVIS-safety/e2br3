use super::*;

fn verify_fda(code: &str) {
	let (path, b, g) = match code {
		"FDA.G.K.12.REQUIRED" => {
			let mut collector_ctx = fixture::empty_ctx();
			let mut other = fixture::drug();
			other.id = Uuid::from_u128(2);
			other.drug_characterization = "2".into();
			let mut suspect = fixture::drug();
			suspect.id = Uuid::from_u128(1);
			suspect.drug_characterization = "1".into();
			collector_ctx.drugs = vec![other, suspect];
			collector_ctx.safety_report = Some(report("1"));
			collector_ctx
				.safety_report
				.as_mut()
				.unwrap()
				.local_criteria_report_type = Some("5".into());
			collector_ctx.message_header =
				Some(header_batch(crate::FDA_BATCH_RECEIVER_POSTMARKET, "OTHER"));
			let mut other_device = device();
			other_device.drug_id = Uuid::from_u128(2);
			let mut suspect_device = device();
			suspect_device.drug_id = Uuid::from_u128(1);
			suspect_device.malfunction = Some(false);
			let mut wrong_owner = Vec::new();
			collect_fda_rules(
				&collector_ctx,
				None,
				&[other_device.clone(), suspect_device.clone()],
				&[],
				&mut wrong_owner,
			);
			assert!(wrong_owner.iter().any(|issue| {
				issue.code == code
					&& issue.path == "drugs.1.fdaDevices.0.malfunction"
			}));

			let mut other_criteria = collector_ctx.clone();
			other_criteria
				.safety_report
				.as_mut()
				.unwrap()
				.local_criteria_report_type = Some("4".into());
			let mut not_malfunction_only = Vec::new();
			collect_fda_rules(
				&other_criteria,
				None,
				&[other_device.clone(), suspect_device.clone()],
				&[],
				&mut not_malfunction_only,
			);
			assert!(!not_malfunction_only.iter().any(|issue| issue.code == code));

			suspect_device.malfunction = Some(true);
			let mut collector_repaired = Vec::new();
			collect_fda_rules(
				&collector_ctx,
				None,
				&[other_device, suspect_device],
				&[],
				&mut collector_repaired,
			);
			assert!(!collector_repaired.iter().any(|issue| issue.code == code));

			let mut not_malfunction_only = Vec::new();
			fda_g_k_12(0, false, false, false, &mut not_malfunction_only);
			assert!(!not_malfunction_only.iter().any(|issue| issue.code == code));
			let mut b = vec![];
			fda_g_k_12(0, true, false, true, &mut b);
			let mut g = vec![];
			fda_g_k_12(0, true, true, true, &mut g);
			("drugs.0.fdaDevices.0.malfunction", b, g)
		}
		"FDA.G.k.12.COLLECTION.REQUIRED" => {
			let mut collector_ctx = fixture::empty_ctx();
			collector_ctx.drugs.push(fixture::drug());
			collector_ctx.safety_report = Some(report("1"));
			collector_ctx
				.safety_report
				.as_mut()
				.unwrap()
				.combination_product_report_indicator = Some("1".into());
			collector_ctx.message_header =
				Some(header_batch(crate::FDA_BATCH_RECEIVER_POSTMARKET, "OTHER"));
			let mut collector_missing = Vec::new();
			collect_fda_rules(
				&collector_ctx,
				None,
				&[],
				&[],
				&mut collector_missing,
			);
			assert!(collector_missing.iter().any(|issue| issue.code == code));

			let mut not_combination = collector_ctx.clone();
			not_combination
				.safety_report
				.as_mut()
				.unwrap()
				.combination_product_report_indicator = Some("0".into());
			let mut not_combination_issues = Vec::new();
			collect_fda_rules(
				&not_combination,
				None,
				&[],
				&[],
				&mut not_combination_issues,
			);
			assert!(!not_combination_issues
				.iter()
				.any(|issue| issue.code == code));

			let mut not_postmarket = collector_ctx.clone();
			not_postmarket.message_header = Some(header_batch("OTHER", "OTHER"));
			let mut not_postmarket_issues = Vec::new();
			collect_fda_rules(
				&not_postmarket,
				None,
				&[],
				&[],
				&mut not_postmarket_issues,
			);
			assert!(!not_postmarket_issues.iter().any(|issue| issue.code == code));

			let mut present_device = device();
			present_device.drug_id = collector_ctx.drugs[0].id;
			let mut collector_present = Vec::new();
			collect_fda_rules(
				&collector_ctx,
				None,
				&[present_device],
				&[],
				&mut collector_present,
			);
			assert!(!collector_present.iter().any(|issue| issue.code == code));

			let mut not_required = Vec::new();
			fda_g_k_12(0, false, false, false, &mut not_required);
			assert!(!not_required.iter().any(|issue| issue.code == code));
			let mut b = vec![];
			fda_g_k_12(0, true, false, false, &mut b);
			let mut g = vec![];
			fda_g_k_12(0, true, true, true, &mut g);
			("drugs.0.fdaDevices", b, g)
		}
		"FDA.G.k.12.r.4-6.AT_LEAST_ONE" => {
			for names in
				[[None, Some("common"), None], [None, None, Some("product")]]
			{
				let mut alternative = Vec::new();
				fda_g_k_12_r_4_6(0, 0, names, true, &mut alternative);
				assert!(!alternative.iter().any(|issue| issue.code == code));
			}
			let mut not_required = Vec::new();
			fda_g_k_12_r_4_6(0, 0, [None; 3], false, &mut not_required);
			assert!(!not_required.iter().any(|issue| issue.code == code));
			let mut b = vec![];
			fda_g_k_12_r_4_6(0, 0, [None; 3], true, &mut b);
			let mut g = vec![];
			fda_g_k_12_r_4_6(0, 0, [Some("brand"), None, None], true, &mut g);
			("drugs.0.fdaDevices.0.deviceBrandName", b, g)
		}
		"FDA.R0072" => {
			for (combination, malfunction, role) in
				[(false, true, "4"), (true, false, "4"), (true, true, "2")]
			{
				let mut rejected = Vec::new();
				fda_g_k_1_a(
					0,
					Some("1"),
					combination,
					malfunction,
					role,
					true,
					&mut rejected,
				);
				assert!(rejected.iter().any(|issue| issue.code == code));
			}
			let mut receiver_ctx = fixture::empty_ctx();
			receiver_ctx.safety_report = Some(report("1"));
			receiver_ctx
				.safety_report
				.as_mut()
				.unwrap()
				.combination_product_report_indicator = Some("1".into());
			let mut role_four = fixture::drug();
			role_four.drug_characterization = "4".into();
			receiver_ctx.drugs.push(role_four);
			let mut malfunctioning = device();
			malfunctioning.drug_id = receiver_ctx.drugs[0].id;
			receiver_ctx.message_header = Some(header(crate::FDA_MSG_RECEIVER_CDER));
			let mut aems = Vec::new();
			collect_fda_rules(
				&receiver_ctx,
				None,
				&[malfunctioning.clone()],
				&[],
				&mut aems,
			);
			assert!(aems.iter().any(|issue| issue.code == code));
			receiver_ctx.message_header = Some(header("CBER_VAERS"));
			let mut vaers = Vec::new();
			collect_fda_rules(
				&receiver_ctx,
				None,
				&[malfunctioning],
				&[],
				&mut vaers,
			);
			assert!(!vaers.iter().any(|issue| issue.code == code));

			let mut b = vec![];
			fda_g_k_1_a(0, None, true, true, "4", true, &mut b);
			let mut g = vec![];
			fda_g_k_1_a(0, Some("1"), true, true, "4", true, &mut g);
			("drugs.0.fdaOtherCharacterization", b, g)
		}
		"FDA.W0006" => {
			for (value, null_flavor) in [(Some("2"), None), (None, Some("NA"))] {
				let mut accepted = Vec::new();
				fda_g_k_10a(0, value, null_flavor, true, &mut accepted);
				assert!(!accepted.iter().any(|issue| issue.code == code));
			}
			let mut not_preanda = Vec::new();
			fda_g_k_10a(0, None, None, false, &mut not_preanda);
			assert!(!not_preanda.iter().any(|issue| issue.code == code));
			let mut b = vec![];
			fda_g_k_10a(0, None, None, true, &mut b);
			let mut g = vec![];
			fda_g_k_10a(0, Some("1"), None, true, &mut g);
			("drugs.0.fdaAdditionalInfoCoded", b, g)
		}
		"FDA.G.K.12.R.3.REQUIRED" | "FDA.W0007" => fda_device_pair(code),
		"FDA.D.1.R0027" => {
			let mut x = fixture::empty_ctx();
			let mut r = reaction();
			r.reaction_meddra_code = Some("10067482".into());
			x.reactions.push(r);
			let mut b = vec![];
			fda_d_1_malfunction(&x, true, true, &mut b);
			for (combination, malfunction) in [(false, true), (true, false)] {
				let mut not_triggered = Vec::new();
				fda_d_1_malfunction(
					&x,
					combination,
					malfunction,
					&mut not_triggered,
				);
				assert!(!not_triggered.iter().any(|issue| issue.code == code));
			}
			x.reactions[0].reaction_meddra_code = Some("10000001".into());
			let mut other_reaction = Vec::new();
			fda_d_1_malfunction(&x, true, true, &mut other_reaction);
			assert!(!other_reaction.iter().any(|issue| issue.code == code));
			x.reactions[0].reaction_meddra_code = Some("10067482".into());
			let mut p = patient();
			p.patient_initials_null_flavor = Some("NA".into());
			x.patient = Some(p);
			let mut g = vec![];
			fda_d_1_malfunction(&x, true, true, &mut g);
			("patientInformation.patientInitialsNullFlavor", b, g)
		}
		"FDA.R0069" | "FDA.R0070" | "FDA.R0071" | "FDA.G.k.1.ROUTE" => {
			fda_route_pair(code)
		}
		"FDA.G.k.9.REQUIRED"
		| "FDA.G.k.9.i.2.r.1.REQUIRED"
		| "FDA.G.k.9.i.2.r.2.REQUIRED"
		| "FDA.G.k.9.i.2.r.3.REQUIRED" => fda_assessment_pair(code),
		_ => panic!("unsupported FDA {code}"),
	};
	edge(code, path, b, g)
}

macro_rules! fda_tests{($($n:ident=>$c:literal),+$(,)?)=>{$(#[test]fn $n(){verify_fda($c);})+};}

#[test]
fn fda_device_code_rules_preserve_row_identity_and_conditions() {
	let device = device();
	let mut mismatched = device_code("device_problem");
	mismatched.device_id = Uuid::from_u128(2);
	let mut issues = Vec::new();
	fda_device_code_rules(0, 0, &device, &[mismatched], None, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.G.K.12.R.3.REQUIRED"
			&& issue.path == "drugs.0.fdaDevices.0.deviceProblemCodes"));
	let mut blank = device_code("device_problem");
	blank.value_code = " ".into();
	issues.clear();
	fda_device_code_rules(0, 0, &device, &[blank], None, &mut issues);
	assert!(issues
		.iter()
		.any(|issue| issue.code == "FDA.G.K.12.R.3.REQUIRED"
			&& issue.path == "drugs.0.fdaDevices.0.deviceProblemCodes.0.valueCode"));
	let mut not_malfunctioning = device;
	not_malfunctioning.malfunction = Some(false);
	issues.clear();
	fda_device_code_rules(0, 0, &not_malfunctioning, &[], Some("4"), &mut issues);
	assert!(issues.is_empty());
}

fda_tests! {rule_fda_d_1_r0027=>"FDA.D.1.R0027",rule_fda_g_k_12_r_3_required=>"FDA.G.K.12.R.3.REQUIRED",rule_fda_g_k_12_required=>"FDA.G.K.12.REQUIRED",rule_fda_g_k_1_route=>"FDA.G.k.1.ROUTE",rule_fda_g_k_12_collection_required=>"FDA.G.k.12.COLLECTION.REQUIRED",rule_fda_g_k_12_r_4_6_at_least_one=>"FDA.G.k.12.r.4-6.AT_LEAST_ONE",rule_fda_g_k_9_required=>"FDA.G.k.9.REQUIRED",rule_fda_g_k_9_i_2_r_1_required=>"FDA.G.k.9.i.2.r.1.REQUIRED",rule_fda_g_k_9_i_2_r_2_required=>"FDA.G.k.9.i.2.r.2.REQUIRED",rule_fda_g_k_9_i_2_r_3_required=>"FDA.G.k.9.i.2.r.3.REQUIRED",rule_fda_r0072=>"FDA.R0072",rule_fda_w0006=>"FDA.W0006",rule_fda_w0007=>"FDA.W0007"}

#[test]
fn rule_fda_r0069() {
	verify_fda("FDA.R0069");
	assert_fda_route_cases(
		"FDA.R0069",
		crate::FDA_MSG_RECEIVER_CDER,
		&[&[2, 1], &[1, 2], &[2, 3], &[2, 4]],
		&[&[2, 2]],
	);
}

#[test]
fn rule_fda_r0070() {
	verify_fda("FDA.R0070");
	assert_fda_route_cases(
		"FDA.R0070",
		crate::FDA_MSG_RECEIVER_CDER_IND,
		&[&[2, 1], &[1, 2], &[2, 3]],
		&[&[2, 2], &[1, 4]],
	);
}

#[test]
fn rule_fda_r0071() {
	verify_fda("FDA.R0071");
	assert_fda_route_cases(
		"FDA.R0071",
		crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE,
		&[&[2, 1], &[1, 2], &[2, 3], &[2, 4]],
		&[&[2, 2]],
	);
}

fn assert_fda_route_cases(
	code: &str,
	receiver: &str,
	accepted_roles: &[&[u8]],
	rejected_roles: &[&[u8]],
) {
	for roles in accepted_roles {
		let issues = fda_route_issues(receiver, roles);
		assert!(
			!issues.iter().any(|issue| issue.code == code),
			"{code} unexpectedly rejected roles {roles:?}: {issues:?}"
		);
	}
	for roles in rejected_roles {
		let issues = fda_route_issues(receiver, roles);
		assert!(
			issues.iter().any(|issue| issue.code == code),
			"{code} unexpectedly accepted roles {roles:?}"
		);
	}
}

fn fda_route_issues(receiver: &str, roles: &[u8]) -> Vec<ValidationIssue> {
	let mut ctx = fixture::empty_ctx();
	ctx.safety_report = Some(report("2"));
	ctx.message_header = Some(header(receiver));
	for role in roles {
		let mut drug = fixture::drug();
		drug.drug_characterization = role.to_string();
		ctx.drugs.push(drug);
	}
	let mut issues = Vec::new();
	fda_g_k_1_route(&ctx, false, &mut issues);
	issues
}
fn fda_route_pair(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut x = fixture::empty_ctx();
	x.safety_report = Some(report("2"));
	let mut d = fixture::drug();
	d.drug_characterization = match c {
		"FDA.R0070" => "4",
		"FDA.R0071" => "2",
		_ => "2",
	}
	.into();
	x.drugs.push(d);
	x.message_header = Some(match c {
		"FDA.R0069" => header(crate::FDA_MSG_RECEIVER_CDER),
		"FDA.R0070" => header(crate::FDA_MSG_RECEIVER_CDER_IND),
		"FDA.R0071" => header(crate::FDA_MSG_RECEIVER_CDER_IND_EXEMPT_BA_BE),
		_ => header_batch(crate::FDA_BATCH_RECEIVER_POSTMARKET, "OTHER"),
	});
	let mut b = vec![];
	fda_g_k_1_route(&x, false, &mut b);
	x.drugs[0].drug_characterization = "1".into();
	let mut g = vec![];
	fda_g_k_1_route(&x, false, &mut g);
	("drugs.0.drugCharacterization", b, g)
}
fn fda_assessment_pair(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut x = fixture::empty_ctx();
	x.safety_report = Some(report("2"));
	let mut d = fixture::drug();
	d.drug_characterization = "1".into();
	x.drugs.push(d);
	if c != "FDA.G.k.9.REQUIRED" {
		x.drug_reaction_assessments.push(fixture::assessment());
		if c != "FDA.G.k.9.i.2.r.1.REQUIRED"
			&& c != "FDA.G.k.9.i.2.r.2.REQUIRED"
			&& c != "FDA.G.k.9.i.2.r.3.REQUIRED"
		{
			unreachable!()
		}
	}
	let mut not_ind = Vec::new();
	fda_g_k_9(&x, false, &mut not_ind);
	assert!(!not_ind.iter().any(|issue| issue.code == c));
	let study_report = x.safety_report.clone();
	x.safety_report = Some(report("1"));
	let mut not_study = Vec::new();
	fda_g_k_9(&x, true, &mut not_study);
	assert!(!not_study.iter().any(|issue| issue.code == c));
	x.safety_report = study_report;
	let mut b = vec![];
	fda_g_k_9(&x, true, &mut b);
	if c == "FDA.G.k.9.REQUIRED" {
		x.drug_reaction_assessments.push(fixture::assessment())
	}
	let mut r = fixture::relatedness();
	r.source_of_assessment = Some("Sponsor".into());
	r.method_of_assessment = Some("FDA".into());
	r.result_of_assessment = Some("Suspected".into());
	if c == "FDA.G.k.9.i.2.r.3.REQUIRED" {
		let mut not_suspected = r.clone();
		not_suspected.result_of_assessment = Some("Not Suspected".into());
		x.relatedness_assessments.push(not_suspected);
		let mut accepted = Vec::new();
		fda_g_k_9(&x, true, &mut accepted);
		assert!(!accepted.iter().any(|issue| issue.code == c));
		x.relatedness_assessments.clear();
	}
	let suspect_assessment = if c == "FDA.G.k.9.REQUIRED" {
		Some(x.drug_reaction_assessments.remove(0))
	} else {
		None
	};
	let mut other_drug = fixture::drug();
	other_drug.id = Uuid::from_u128(2);
	other_drug.drug_characterization = "2".into();
	x.drugs.push(other_drug);
	let mut other_assessment = fixture::assessment();
	other_assessment.id = Uuid::from_u128(2);
	other_assessment.drug_id = Uuid::from_u128(2);
	x.drug_reaction_assessments.push(other_assessment);
	let mut other_result = r.clone();
	other_result.drug_reaction_assessment_id = Uuid::from_u128(2);
	x.relatedness_assessments.push(other_result);
	let mut wrong_owner = Vec::new();
	fda_g_k_9(&x, true, &mut wrong_owner);
	assert!(wrong_owner.iter().any(|issue| issue.code == c));
	x.drugs.pop();
	x.drug_reaction_assessments.pop();
	if let Some(suspect_assessment) = suspect_assessment {
		x.drug_reaction_assessments.push(suspect_assessment);
	}
	x.relatedness_assessments.clear();
	x.relatedness_assessments.push(r);
	let mut g = vec![];
	fda_g_k_9(&x, true, &mut g);
	(
		match c {
			"FDA.G.k.9.REQUIRED" => "drugs.0.drugReactionAssessments",
			"FDA.G.k.9.i.2.r.1.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.sourceOfAssessment"
			}
			"FDA.G.k.9.i.2.r.2.REQUIRED" => {
				"drugs.0.drugReactionAssessments.0.methodOfAssessment"
			}
			_ => "drugs.0.drugReactionAssessments.0.resultOfAssessment",
		},
		b,
		g,
	)
}
fn fda_device_pair(
	c: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let d = device();
	for malfunction in [Some(false), None] {
		let mut not_malfunctioning = d.clone();
		not_malfunctioning.malfunction = malfunction;
		let mut not_triggered = Vec::new();
		fda_device_code_rules(
			0,
			0,
			&not_malfunctioning,
			&[],
			Some("4"),
			&mut not_triggered,
		);
		assert!(!not_triggered.iter().any(|issue| issue.code == c));
	}
	let element = if c == "FDA.W0007" {
		"remedial_action"
	} else {
		"device_problem"
	};
	let mut wrong_owner_code = device_code(element);
	wrong_owner_code.device_id = Uuid::from_u128(2);
	let mut wrong_owner = Vec::new();
	fda_device_code_rules(
		0,
		0,
		&d,
		&[wrong_owner_code],
		if c == "FDA.W0007" { Some("4") } else { None },
		&mut wrong_owner,
	);
	assert!(wrong_owner.iter().any(|issue| issue.code == c));
	if c == "FDA.W0007" {
		let mut not_local_criteria = Vec::new();
		fda_device_code_rules(0, 0, &d, &[], None, &mut not_local_criteria);
		assert!(!not_local_criteria.iter().any(|issue| issue.code == c));
	}
	let mut b = vec![];
	fda_device_code_rules(
		0,
		0,
		&d,
		&[],
		if c == "FDA.W0007" { Some("4") } else { None },
		&mut b,
	);
	let codes = vec![device_code(element)];
	let mut g = vec![];
	fda_device_code_rules(
		0,
		0,
		&d,
		&codes,
		if c == "FDA.W0007" { Some("4") } else { None },
		&mut g,
	);
	(
		if c == "FDA.W0007" {
			"drugs.0.fdaDevices.0.remedialActions"
		} else {
			"drugs.0.fdaDevices.0.deviceProblemCodes"
		},
		b,
		g,
	)
}
