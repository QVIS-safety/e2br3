use super::*;

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

fn assert_rule(code: &str, expected_path: &str) {
	let (path, invalid, repaired) = fda_edge(code);
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
	rule_fda_d_11_nullflavor_route: ("FDA.D.11.NULLFLAVOR.ROUTE", "patientInformation.raceCodeNullFlavor"),
	rule_fda_d_11_required: ("FDA.D.11.REQUIRED", "patientInformation.raceCodes"),
	rule_fda_d_11_r_1_required: ("FDA.D.11.r.1.REQUIRED", "patientInformation.raceCodes"),
	rule_fda_d_12_nullflavor_route: ("FDA.D.12.NULLFLAVOR.ROUTE", "patientInformation.ethnicityCodeNullFlavor"),
	rule_fda_d_12_required: ("FDA.D.12.REQUIRED", "patientInformation.ethnicityCode"),
}

#[test]
fn rule_fda_d_2_required() {
	assert_rule("FDA.D.2.REQUIRED", "patientInformation.patientBirthDate");
	let mut patient = patient();
	patient.birth_date_null_flavor = Some("UNK".into());
	let issues = invoke(&Some(patient), |patient, issues| {
		fda_d_2(patient.as_ref(), issues)
	});
	assert!(
		!issues.iter().any(|issue| issue.code == "FDA.D.2.REQUIRED"),
		"birth-date null flavor did not satisfy FDA.D.2: {issues:?}"
	);
}

#[test]
fn rule_fda_d_9_1_required() {
	assert_rule("FDA.D.9.1.REQUIRED", "patientInformation.death.dateOfDeath");

	for (report_type, receiver, expected) in [
		("1", "CDER_IND", false),
		("2", "CDER_IND", true),
		("2", "CDER", false),
	] {
		let mut ctx = empty_ctx();
		ctx.safety_report = Some(report(report_type));
		ctx.message_header = Some(message(receiver));
		ctx.reactions.push(death_reaction());
		let issues = invoke(&ctx, fda_d_9_1);
		assert_eq!(
			issues
				.iter()
				.any(|issue| issue.code == "FDA.D.9.1.REQUIRED"),
			expected,
			"unexpected death-date requirement for reportType={report_type}, receiver={receiver}: {issues:?}"
		);
	}
}

fn assert_fda_na_warning_cases(code: &str, path: &str) {
	assert_rule(code, path);
	for (initials, null_flavor) in [
		(Some("SUMMARY"), None),
		(None, Some("NA")),
		(Some("PATIENT"), None),
	] {
		let mut value = patient();
		value.patient_initials = initials.map(str::to_string);
		value.patient_initials_null_flavor = null_flavor.map(str::to_string);
		let issues = invoke(&value, fda_d_11_d_12_na);
		let expected = initials == Some("SUMMARY") || null_flavor == Some("NA");
		assert_eq!(
			issues.iter().any(|issue| issue.code == code),
			expected,
			"unexpected {code} result for initials={initials:?}, nullFlavor={null_flavor:?}: {issues:?}"
		);
	}
}

#[test]
fn rule_fda_w0003() {
	assert_fda_na_warning_cases(
		"FDA.W0003",
		"patientInformation.raceCodeNullFlavor",
	);
}

#[test]
fn rule_fda_w0004() {
	assert_fda_na_warning_cases(
		"FDA.W0004",
		"patientInformation.ethnicityCodeNullFlavor",
	);
}

#[test]
fn rule_fda_w0010() {
	assert_rule("FDA.W0010", "patientInformation.patientInitials");

	let mut base = empty_ctx();
	base.safety_report = Some(report("2"));
	base.message_header = Some(message("CDER_IND"));
	base.patient = Some(patient());
	base.linked_report_numbers.push(linked_report());
	let fda = FdaValidationContext {
		studies: vec![ind_study()],
		cross_reported_inds: Vec::new(),
		has_prior_submission: false,
	};
	for missing in ["reportType", "receiver", "ind", "linkedReport"] {
		let mut ctx = base.clone();
		let mut local_fda = fda.clone();
		match missing {
			"reportType" => {
				ctx.safety_report
					.as_mut()
					.expect("report fixture")
					.report_type = Some("1".into())
			}
			"receiver" => ctx.message_header = Some(message("CDER")),
			"ind" => local_fda.studies.clear(),
			"linkedReport" => ctx.linked_report_numbers.clear(),
			_ => unreachable!(),
		}
		let mut issues = Vec::new();
		fda_d_1(&ctx, Some(&local_fda), &mut issues);
		assert!(
			!issues.iter().any(|issue| issue.code == "FDA.W0010"),
			"FDA.W0010 emitted without {missing}: {issues:?}"
		);
	}
}
