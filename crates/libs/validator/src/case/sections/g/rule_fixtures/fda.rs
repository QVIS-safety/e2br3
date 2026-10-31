fn verify_fda(code: &str) {
	let (path, b, g) = match code {
		"FDA.G.K.12.REQUIRED" => {
			let mut b = vec![];
			fda_g_k_12(0, true, false, true, &mut b);
			let mut g = vec![];
			fda_g_k_12(0, true, true, true, &mut g);
			("drugs.0.fdaDevices.0.malfunction", b, g)
		}
		"FDA.G.k.12.COLLECTION.REQUIRED" => {
			let mut b = vec![];
			fda_g_k_12(0, true, false, false, &mut b);
			let mut g = vec![];
			fda_g_k_12(0, true, true, true, &mut g);
			("drugs.0.fdaDevices", b, g)
		}
		"FDA.G.k.12.r.4-6.AT_LEAST_ONE" => {
			let mut b = vec![];
			fda_g_k_12_r_4_6(0, 0, [None; 3], true, &mut b);
			let mut g = vec![];
			fda_g_k_12_r_4_6(0, 0, [Some("brand"), None, None], true, &mut g);
			("drugs.0.fdaDevices.0.deviceBrandName", b, g)
		}
		"FDA.R0072" => {
			let mut b = vec![];
			fda_g_k_1_a(0, None, true, true, "4", true, &mut b);
			let mut g = vec![];
			fda_g_k_1_a(0, Some("1"), true, true, "4", true, &mut g);
			("drugs.0.fdaOtherCharacterization", b, g)
		}
		"FDA.W0006" => {
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
	let mut b = vec![];
	fda_g_k_9(&x, true, &mut b);
	if c == "FDA.G.k.9.REQUIRED" {
		x.drug_reaction_assessments.push(fixture::assessment())
	}
	let mut r = fixture::relatedness();
	r.source_of_assessment = Some("Sponsor".into());
	r.method_of_assessment = Some("FDA".into());
	r.result_of_assessment = Some("Suspected".into());
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
	let mut b = vec![];
	fda_device_code_rules(
		0,
		0,
		&d,
		&[],
		if c == "FDA.W0007" { Some("4") } else { None },
		&mut b,
	);
	let element = if c == "FDA.W0007" {
		"remedial_action"
	} else {
		"device_problem"
	};
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
