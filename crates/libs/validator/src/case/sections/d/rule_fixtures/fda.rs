fn fda_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	match code {
		"FDA.D.11.REQUIRED" | "FDA.D.11.r.1.REQUIRED" => {
			let mut good = patient();
			good.race_code_null_flavor = Some("UNK".into());
			let vaers = code == "FDA.D.11.REQUIRED";
			(
				"patientInformation.raceCodes",
				invoke(&None, |p, i| fda_d_11(p.as_ref(), vaers, i)),
				invoke(&Some(good), |p, i| fda_d_11(p.as_ref(), vaers, i)),
			)
		}
		"FDA.D.11.NULLFLAVOR.ROUTE" => {
			let mut bad = patient();
			bad.race_code_null_flavor = Some("NA".into());
			let mut good = bad.clone();
			good.race_code_null_flavor = Some("UNK".into());
			(
				"patientInformation.raceCodeNullFlavor",
				invoke(&bad, |p, i| fda_d_11(Some(p), true, i)),
				invoke(&good, |p, i| fda_d_11(Some(p), true, i)),
			)
		}
		"FDA.D.12.REQUIRED" => {
			let mut good = patient();
			good.ethnicity_code_null_flavor = Some("UNK".into());
			(
				"patientInformation.ethnicityCode",
				invoke(&None, |p, i| fda_d_12(p.as_ref(), true, i)),
				invoke(&Some(good), |p, i| fda_d_12(p.as_ref(), true, i)),
			)
		}
		"FDA.D.12.NULLFLAVOR.ROUTE" => {
			let mut bad = patient();
			bad.ethnicity_code_null_flavor = Some("NA".into());
			let mut good = bad.clone();
			good.ethnicity_code_null_flavor = Some("UNK".into());
			(
				"patientInformation.ethnicityCodeNullFlavor",
				invoke(&bad, |p, i| fda_d_12(Some(p), true, i)),
				invoke(&good, |p, i| fda_d_12(Some(p), true, i)),
			)
		}
		"FDA.D.2.REQUIRED" => {
			let mut good = patient();
			good.age_group = Some("3".into());
			(
				"patientInformation.patientBirthDate",
				invoke(&None, |p, i| fda_d_2(p.as_ref(), i)),
				invoke(&Some(good), |p, i| fda_d_2(p.as_ref(), i)),
			)
		}
		"FDA.D.9.1.REQUIRED" => {
			let mut bad = empty_ctx();
			bad.safety_report = Some(report("2"));
			bad.message_header = Some(message("CDER_IND"));
			bad.reactions.push(death_reaction());
			let mut good = bad.clone();
			let mut death = death_info();
			death.date_of_death_null_flavor = Some("UNK".into());
			good.death_info = Some(death);
			let run = |ctx: &ValidationContext| {
				let mut i = Vec::new();
				fda_d_9_1(ctx, &mut i);
				i
			};
			(
				"patientInformation.death.dateOfDeath",
				run(&bad),
				run(&good),
			)
		}
		"FDA.W0010" => {
			let mut bad = empty_ctx();
			bad.safety_report = Some(report("2"));
			bad.message_header = Some(message("CDER_IND"));
			bad.patient = Some(patient());
			bad.linked_report_numbers.push(linked_report());
			let fda = FdaValidationContext {
				studies: vec![ind_study()],
				cross_reported_inds: Vec::new(),
				has_prior_submission: false,
			};
			let mut good = bad.clone();
			good.patient
				.as_mut()
				.expect("patient fixture")
				.patient_initials = Some("AGGREGATE".into());
			let run = |ctx: &ValidationContext| {
				let mut i = Vec::new();
				fda_d_1(ctx, Some(&fda), &mut i);
				i
			};
			("patientInformation.patientInitials", run(&bad), run(&good))
		}
		"FDA.W0003" | "FDA.W0004" => {
			let mut bad = patient();
			bad.patient_initials = Some("AGGREGATE".into());
			let mut good = bad.clone();
			let path = if code == "FDA.W0003" {
				good.race_code_null_flavor = Some("NA".into());
				"patientInformation.raceCodeNullFlavor"
			} else {
				good.ethnicity_code_null_flavor = Some("NA".into());
				"patientInformation.ethnicityCodeNullFlavor"
			};
			(
				path,
				invoke(&bad, fda_d_11_d_12_na),
				invoke(&good, fda_d_11_d_12_na),
			)
		}
		_ => panic!("missing FDA fixture: {code}"),
	}
}
