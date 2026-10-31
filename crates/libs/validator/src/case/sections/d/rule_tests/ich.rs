use super::*;

fn patient_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut bad = patient();
	let mut good = bad.clone();
	let (path, function): (
		&str,
		fn(&PatientInformation, &mut Vec<ValidationIssue>),
	) = match code {
		"ICH.D.2.EXCLUSIVE" => {
			bad.birth_date_null_flavor = Some("UNK".into());
			bad.age_group = Some("3".into());
			good.birth_date_null_flavor = Some("UNK".into());
			good.age_group = None;
			("patientInformation.patientBirthDate", d_2)
		}
		"ICH.D.2.1.FUTURE_DATE.FORBIDDEN" => {
			bad.birth_date = Some(future_date());
			good.birth_date = Some(past_date());
			("patientInformation.patientBirthDate", d_2_1)
		}
		"ICH.D.2.2a.REQUIRED" => {
			bad.age_unit = Some("a".into());
			good.age_unit = Some("a".into());
			good.age_at_time_of_onset = Some(Decimal::ONE);
			("patientInformation.ageAtTimeOfOnset", d_2_2a)
		}
		"ICH.D.2.2b.REQUIRED" => {
			bad.age_at_time_of_onset = Some(Decimal::ONE);
			good.age_at_time_of_onset = Some(Decimal::ONE);
			good.age_unit = Some("a".into());
			("patientInformation.ageUnit", d_2_2b)
		}
		"ICH.D.2.2.1a.REQUIRED" => {
			bad.gestation_period_unit = Some("wk".into());
			good.gestation_period_unit = Some("wk".into());
			good.gestation_period = Some(Decimal::ONE);
			("patientInformation.gestationPeriod", d_2_2_1a)
		}
		"ICH.D.2.2.1b.REQUIRED" => {
			bad.gestation_period = Some(Decimal::ONE);
			good.gestation_period = Some(Decimal::ONE);
			good.gestation_period_unit = Some("wk".into());
			("patientInformation.gestationPeriodUnit", d_2_2_1b)
		}
		"ICH.D.4.INTEGER" => {
			bad.height_cm = Some(Decimal::new(1755, 1));
			good.height_cm = Some(Decimal::new(175, 0));
			("patientInformation.heightCm", d_4_integer)
		}
		"ICH.D.6.FUTURE_DATE.FORBIDDEN" => {
			bad.last_menstrual_period_date = Some(future_date());
			good.last_menstrual_period_date = Some(past_date());
			("patientInformation.lastMenstrualPeriodDate", d_6)
		}
		"ICH.D.6.NULLFLAVOR.ALLOWED" => {
			bad.last_menstrual_period_date_null_flavor = Some("NA".into());
			good.last_menstrual_period_date_null_flavor = Some("MSK".into());
			("patientInformation.lastMenstrualPeriodDateNullFlavor", d_6)
		}
		"ICH.D.7.2.REQUIRED" => {
			bad.medical_history_text = None;
			good.medical_history_text = Some("history".into());
			("patientInformation.medicalHistoryText", |p, i| {
				d_7_2(p, true, i)
			})
		}
		_ => panic!("missing patient fixture: {code}"),
	};
	(path, invoke(&bad, function), invoke(&good, function))
}

fn medhist_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut bad = medhist(None, None);
	let mut good;
	let mut bad_ctx = empty_ctx();
	let mut good_ctx = empty_ctx();
	let (path, kind) = if code.contains("1a.REQUIRED") {
		bad.meddra_code = Some("10000001".into());
		good = bad.clone();
		good.meddra_version = Some("26.1".into());
		("patientInformation.medicalHistory.0.meddraVersion", 0)
	} else if code.contains("1b.REQUIRED") {
		bad.meddra_version = Some("26.1".into());
		good = bad.clone();
		good.meddra_code = Some("10000001".into());
		("patientInformation.medicalHistory.0.meddraCode", 1)
	} else if code.contains("1a.ALLOWED") {
		bad.meddra_version = Some("bad".into());
		bad.meddra_code = Some("10000001".into());
		good = bad.clone();
		good.meddra_version = Some("26.1".into());
		("patientInformation.medicalHistory.0.meddraVersion", 2)
	} else if code.contains("1a.VOCABULARY") {
		bad.meddra_version = Some("27.0".into());
		bad.meddra_code = Some("10000001".into());
		good = bad.clone();
		good.meddra_version = Some("26.1".into());
		bad_ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
		good_ctx.vocabulary = bad_ctx.vocabulary.clone();
		("patientInformation.medicalHistory.0.meddraVersion", 2)
	} else if code.contains("1b.VOCABULARY") {
		bad.meddra_version = Some("26.1".into());
		bad.meddra_code = Some("99999999".into());
		good = bad.clone();
		good.meddra_code = Some("10000001".into());
		bad_ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
		good_ctx.vocabulary = bad_ctx.vocabulary.clone();
		("patientInformation.medicalHistory.0.meddraCode", 2)
	} else if code.contains("PARENT_DUPLICATE") {
		bad.meddra_code = Some("10000001".into());
		bad.family_history = Some(true);
		good = bad.clone();
		let invalid_history =
			vec![parent_medhist(Uuid::nil(), Some("10000001"), Some("26.1"))];
		let repaired_history =
			vec![parent_medhist(Uuid::nil(), Some("10000002"), Some("26.1"))];
		return (
			"patientInformation.medicalHistory.0.familyHistory",
			invoke(&bad, |v, i| {
				d_7_1_r_6_parent_duplicate(0, v, &invalid_history, i)
			}),
			invoke(&good, |v, i| {
				d_7_1_r_6_parent_duplicate(0, v, &repaired_history, i)
			}),
		);
	} else if code == "ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN" {
		bad.start_date = Some("29990101".into());
		good = bad.clone();
		good.start_date = Some("20200101".into());
		("patientInformation.medicalHistoryEpisodes.0.dateRange", 3)
	} else {
		panic!("missing medical-history fixture: {code}")
	};
	let run = |row: &MedicalHistoryEpisode, ctx: &ValidationContext| {
		let mut issues = Vec::new();
		match kind {
			0 => d_7_1_r_1a(0, row, &mut issues),
			1 => d_7_1_r_1b(0, row, &mut issues),
			2 => d_7_1_r_1_meddra(0, row, ctx, &mut issues),
			3 => d_7_1_r(0, row, &mut issues),
			_ => panic!("invalid medical-history fixture kind: {kind}"),
		}
		if code == "ICH.D.7.1.r.1a.VOCABULARY" {
			let mut global_ctx = ctx.clone();
			global_ctx.medical_history.push(row.clone());
			super::super::super::collect_meddra_version_issues(
				&global_ctx,
				&mut issues,
			);
		}
		issues
	};
	(path, run(&bad, &bad_ctx), run(&good, &good_ctx))
}

fn past_drug_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let mut bad = past_drug();
	let mut good;
	let path = if code == "ICH.D.8.r.1.REQUIRED" {
		bad.mpid = Some("mpid".into());
		good = bad.clone();
		good.drug_name = Some("Drug".into());
		"patientInformation.pastDrugs.0.drugName"
	} else if code == "ICH.D.8.MPID_PHPID.EXCLUSIVE" {
		bad.mpid = Some("mpid".into());
		bad.phpid = Some("phpid".into());
		good = bad.clone();
		good.phpid = None;
		"patientInformation.pastDrugs.0.mpid"
	} else if code == "ICH.D.8.r.FUTURE_DATE.FORBIDDEN" {
		bad.start_date = Some(future_date());
		good = bad.clone();
		good.start_date = Some(past_date());
		"patientInformation.pastDrugs.0.dateRange"
	} else if code.starts_with("ICH.D.8.r.6") || code.starts_with("ICH.D.8.r.7") {
		let indication = code.contains(".r.6");
		let version_rule = code.contains("a.");
		let vocabulary = code.ends_with(".VOCABULARY");
		let bad_value = if vocabulary {
			if version_rule {
				"27.0"
			} else {
				"99999999"
			}
		} else if version_rule {
			""
		} else {
			""
		};
		if indication {
			if version_rule {
				bad.indication_meddra_code = Some("10000001".into());
				bad.indication_meddra_version =
					(!bad_value.is_empty()).then(|| bad_value.into());
			} else {
				bad.indication_meddra_version = Some("26.1".into());
				bad.indication_meddra_code =
					(!bad_value.is_empty()).then(|| bad_value.into());
			}
		} else if version_rule {
			bad.reaction_meddra_code = Some("10000001".into());
			bad.reaction_meddra_version =
				(!bad_value.is_empty()).then(|| bad_value.into());
		} else {
			bad.reaction_meddra_version = Some("26.1".into());
			bad.reaction_meddra_code =
				(!bad_value.is_empty()).then(|| bad_value.into());
		}
		good = bad.clone();
		if indication {
			if version_rule {
				good.indication_meddra_version = Some("26.1".into())
			} else {
				good.indication_meddra_code = Some("10000001".into())
			}
		} else if version_rule {
			good.reaction_meddra_version = Some("26.1".into())
		} else {
			good.reaction_meddra_code = Some("10000001".into())
		}
		match (indication, version_rule) {
			(true, true) => "patientInformation.pastDrugs.0.indicationMeddraVersion",
			(true, false) => "patientInformation.pastDrugs.0.indicationMeddraCode",
			(false, true) => "patientInformation.pastDrugs.0.reactionMeddraVersion",
			(false, false) => "patientInformation.pastDrugs.0.reactionMeddraCode",
		}
	} else {
		panic!("missing past-drug fixture: {code}")
	};
	let path = match code {
		"ICH.D.8.r.6a.VOCABULARY" => {
			"patientInformation.pastDrugHistory.0.indicationMeddraVersion"
		}
		"ICH.D.8.r.7a.VOCABULARY" => {
			"patientInformation.pastDrugHistory.0.reactionMeddraVersion"
		}
		_ => path,
	};
	let mut bad_ctx = empty_ctx();
	let mut good_ctx = empty_ctx();
	if code.ends_with(".VOCABULARY") {
		bad_ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
		good_ctx.vocabulary = bad_ctx.vocabulary.clone();
	}
	bad_ctx.past_drugs.push(bad);
	good_ctx.past_drugs.push(good);
	(path, run_ich(&bad_ctx), run_ich(&good_ctx))
}

fn death_cause_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let autopsy = code.contains(".9.4.");
	let version = code.contains(".1a.");
	let comments = code.contains(".2.REQUIRED");
	let vocabulary = code.ends_with(".VOCABULARY");
	let (mut bad_ctx, mut good_ctx) = (empty_ctx(), empty_ctx());
	let (
		bad_version,
		bad_code,
		bad_comments,
		good_version,
		good_code,
		good_comments,
	) = if comments {
		(
			Some("26.1"),
			Some("10000001"),
			None,
			Some("26.1"),
			Some("10000001"),
			Some("comment"),
		)
	} else if version {
		(
			if vocabulary { Some("27.0") } else { None },
			Some("10000001"),
			Some("comment"),
			Some("26.1"),
			Some("10000001"),
			Some("comment"),
		)
	} else {
		(
			Some("26.1"),
			if vocabulary { Some("99999999") } else { None },
			Some("comment"),
			Some("26.1"),
			Some("10000001"),
			Some("comment"),
		)
	};
	if vocabulary {
		bad_ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
		good_ctx.vocabulary = bad_ctx.vocabulary.clone();
	}
	let stem = if autopsy {
		"patientInformation.death.autopsyCauses.0"
	} else {
		"patientInformation.death.reportedCauses.0"
	};
	if autopsy {
		bad_ctx.autopsy_causes_of_death.push(autopsy_cause(
			bad_code,
			bad_version,
			bad_comments,
		));
		good_ctx.autopsy_causes_of_death.push(autopsy_cause(
			good_code,
			good_version,
			good_comments,
		));
	} else {
		bad_ctx.reported_causes_of_death.push(reported_cause(
			bad_code,
			bad_version,
			bad_comments,
		));
		good_ctx.reported_causes_of_death.push(reported_cause(
			good_code,
			good_version,
			good_comments,
		));
	}
	let field = if comments {
		"comments"
	} else if version {
		"meddraVersion"
	} else {
		"meddraCode"
	};
	(
		Box::leak(format!("{stem}.{field}").into_boxed_str()),
		run_ich(&bad_ctx),
		run_ich(&good_ctx),
	)
}

fn parent_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let id = Uuid::from_u128(1);
	let mut bad = parent(id);
	let mut good = bad.clone();
	let (path, function): (
		&str,
		fn(usize, &ParentInformation, &mut Vec<ValidationIssue>),
	) = match code {
		"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN" => {
			bad.parent_birth_date = Some(future_date());
			good.parent_birth_date = Some(past_date());
			("patientInformation.parents.0.parentBirthDate", d_10_2_1)
		}
		"ICH.D.10.2.2a.REQUIRED" => {
			bad.parent_age_unit = Some("a".into());
			good.parent_age_unit = Some("a".into());
			good.parent_age = Some(Decimal::ONE);
			("patientInformation.parents.0.parentAge", d_10_2_2a)
		}
		"ICH.D.10.2.2b.REQUIRED" => {
			bad.parent_age = Some(Decimal::ONE);
			good.parent_age = Some(Decimal::ONE);
			good.parent_age_unit = Some("a".into());
			("patientInformation.parents.0.parentAgeUnit", d_10_2_2b)
		}
		"ICH.D.10.2.EXCLUSIVE" => {
			bad.parent_birth_date_null_flavor = Some("UNK".into());
			bad.parent_age = Some(Decimal::ONE);
			good.parent_birth_date_null_flavor = Some("UNK".into());
			good.parent_age = None;
			("patientInformation.parents.0.parentBirthDate", d_10_2)
		}
		"ICH.D.10.3.FUTURE_DATE.FORBIDDEN" => {
			bad.last_menstrual_period_date = Some(future_date());
			good.last_menstrual_period_date = Some(past_date());
			(
				"patientInformation.parents.0.lastMenstrualPeriodDate",
				d_10_3,
			)
		}
		"ICH.D.10.5.INTEGER" => {
			bad.height_cm = Some(Decimal::new(1755, 1));
			good.height_cm = Some(Decimal::new(175, 0));
			("patientInformation.parents.0.heightCm", d_10_5_integer)
		}
		"ICH.D.10.6.REQUIRED" => {
			bad.parent_age = Some(Decimal::ONE);
			good.parent_age = Some(Decimal::ONE);
			good.sex = Some("1".into());
			("patientInformation.parents.0.sex", |idx, p, i| {
				d_10_6(idx, p, false, i)
			})
		}
		_ => panic!("missing parent fixture: {code}"),
	};
	(
		path,
		invoke(&bad, |v, i| function(0, v, i)),
		invoke(&good, |v, i| function(0, v, i)),
	)
}

fn parent_history_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let id = Uuid::from_u128(1);
	let mut bad = parent_medhist(id, None, None);
	let mut good;
	let version = code.contains("1a.");
	let vocabulary = code.ends_with(".VOCABULARY");
	if version {
		bad.meddra_code = Some("10000001".into());
		if vocabulary {
			bad.meddra_version = Some("27.0".into());
		} else if code.ends_with(".ALLOWED.VALUE") {
			bad.meddra_version = Some("bad".into());
		}
	} else {
		bad.meddra_version = Some("26.1".into());
		if vocabulary {
			bad.meddra_code = Some("99999999".into());
		}
	}
	good = bad.clone();
	if version {
		good.meddra_version = Some("26.1".into())
	} else {
		good.meddra_code = Some("10000001".into())
	}
	if code.contains("FUTURE_DATE") {
		bad.start_date = Some(future_date());
		good = bad.clone();
		good.start_date = Some(past_date());
	}
	let mut bad_ctx = empty_ctx();
	let mut good_ctx = empty_ctx();
	bad_ctx.parents.push(parent(id));
	good_ctx.parents.push(parent(id));
	if vocabulary {
		bad_ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
		good_ctx.vocabulary = bad_ctx.vocabulary.clone();
	}
	bad_ctx.parent_medical_history.push(bad);
	good_ctx.parent_medical_history.push(good);
	let field = if code.contains("FUTURE_DATE") {
		"dateRange"
	} else if version {
		"meddraVersion"
	} else {
		"meddraCode"
	};
	let path = Box::leak(
		format!("patientInformation.parents.0.medicalHistory.0.{field}")
			.into_boxed_str(),
	);
	(path, run_ich(&bad_ctx), run_ich(&good_ctx))
}

fn parent_past_edge(
	code: &str,
) -> (&'static str, Vec<ValidationIssue>, Vec<ValidationIssue>) {
	let id = Uuid::from_u128(1);
	let mut bad = parent_past_drug(id, None, None);
	let mut good;
	let field = if code.contains("MPID_PHPID") {
		bad.mpid = Some("m".into());
		bad.phpid = Some("p".into());
		"mpid"
	} else if code.contains("FUTURE_DATE") {
		bad.start_date = Some(future_date());
		"dateRange"
	} else {
		let indication = code.contains(".r.6");
		let version = code.contains("a.");
		let vocabulary = code.ends_with(".VOCABULARY");
		if indication {
			if version {
				bad.indication_meddra_code = Some("10000001".into());
				if vocabulary {
					bad.indication_meddra_version = Some("27.0".into())
				}
			} else {
				bad.indication_meddra_version = Some("26.1".into());
				if vocabulary {
					bad.indication_meddra_code = Some("99999999".into())
				}
			}
		} else if version {
			bad.reaction_meddra_code = Some("10000001".into());
			if vocabulary {
				bad.reaction_meddra_version = Some("27.0".into())
			}
		} else {
			bad.reaction_meddra_version = Some("26.1".into());
			if vocabulary {
				bad.reaction_meddra_code = Some("99999999".into())
			}
		}
		match (indication, version) {
			(true, true) => "indicationMeddraVersion",
			(true, false) => "indicationMeddraCode",
			(false, true) => "reactionMeddraVersion",
			(false, false) => "reactionMeddraCode",
		}
	};
	good = bad.clone();
	match field {
		"mpid" => good.phpid = None,
		"dateRange" => good.start_date = Some(past_date()),
		"indicationMeddraVersion" => {
			good.indication_meddra_version = Some("26.1".into())
		}
		"indicationMeddraCode" => {
			good.indication_meddra_code = Some("10000001".into())
		}
		"reactionMeddraVersion" => {
			good.reaction_meddra_version = Some("26.1".into())
		}
		"reactionMeddraCode" => good.reaction_meddra_code = Some("10000001".into()),
		_ => panic!("invalid parent past-drug field: {field}"),
	}
	let mut bad_ctx = empty_ctx();
	let mut good_ctx = empty_ctx();
	bad_ctx.parents.push(parent(id));
	good_ctx.parents.push(parent(id));
	if code.ends_with(".VOCABULARY") {
		bad_ctx.vocabulary =
			crate::context::VocabularyContext::for_meddra(&[("26.1", "10000001")]);
		good_ctx.vocabulary = bad_ctx.vocabulary.clone();
	}
	bad_ctx.parent_past_drugs.push(bad);
	good_ctx.parent_past_drugs.push(good);
	let path = Box::leak(
		format!("patientInformation.parents.0.pastDrugs.0.{field}").into_boxed_str(),
	);
	(path, run_ich(&bad_ctx), run_ich(&good_ctx))
}

fn assert_rule(code: &str, expected_path: &str) {
	let (path, invalid, repaired) = if code == "ICH.D.1.REQUIRED" {
		let mut good = patient();
		good.patient_initials = Some("AB".into());
		(
			"patientInformation.patientInitials",
			invoke(&None, |p, i| d_1(p.as_ref(), i)),
			invoke(&Some(good), |p, i| d_1(p.as_ref(), i)),
		)
	} else if code.starts_with("ICH.D.7.1") {
		medhist_edge(code)
	} else if code.starts_with("ICH.D.8") {
		past_drug_edge(code)
	} else if code.starts_with("ICH.D.9.2") || code.starts_with("ICH.D.9.4") {
		death_cause_edge(code)
	} else if code == "ICH.D.9.1.FUTURE_DATE.FORBIDDEN" {
		let mut bad = death_info();
		bad.date_of_death = Some(future_date());
		let mut good = bad.clone();
		good.date_of_death = Some(past_date());
		(
			"patientInformation.death.dateOfDeath",
			invoke(&bad, d_9_1),
			invoke(&good, d_9_1),
		)
	} else if code == "ICH.D.9.3.REQUIRED" {
		let mut bad = death_info();
		bad.date_of_death = Some(future_date());
		bad.autopsy_performed = None;
		let mut good = bad.clone();
		good.autopsy_performed = Some(false);
		(
			"patientInformation.death.autopsyPerformed",
			invoke(&bad, d_9_3),
			invoke(&good, d_9_3),
		)
	} else if code.starts_with("ICH.D.10.7") {
		parent_history_edge(code)
	} else if code.starts_with("ICH.D.10.8") {
		parent_past_edge(code)
	} else if code.starts_with("ICH.D.10") {
		parent_edge(code)
	} else {
		patient_edge(code)
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
	rule_ich_d_10_5_integer: ("ICH.D.10.5.INTEGER", "patientInformation.parents.0.heightCm"),
	rule_ich_d_10_7_1_r_1a_allowed_value: ("ICH.D.10.7.1.r.1a.ALLOWED.VALUE", "patientInformation.parents.0.medicalHistory.0.meddraVersion"),
	rule_ich_d_10_7_1_r_1a_required: ("ICH.D.10.7.1.r.1a.REQUIRED", "patientInformation.parents.0.medicalHistory.0.meddraVersion"),
	rule_ich_d_10_7_1_r_1a_vocabulary: ("ICH.D.10.7.1.r.1a.VOCABULARY", "patientInformation.parents.0.medicalHistory.0.meddraVersion"),
	rule_ich_d_10_7_1_r_1b_required: ("ICH.D.10.7.1.r.1b.REQUIRED", "patientInformation.parents.0.medicalHistory.0.meddraCode"),
	rule_ich_d_10_7_1_r_1b_vocabulary: ("ICH.D.10.7.1.r.1b.VOCABULARY", "patientInformation.parents.0.medicalHistory.0.meddraCode"),
	rule_ich_d_10_8_mpid_phpid_exclusive: ("ICH.D.10.8.MPID_PHPID.EXCLUSIVE", "patientInformation.parents.0.pastDrugs.0.mpid"),
	rule_ich_d_10_8_r_6a_required: ("ICH.D.10.8.r.6a.REQUIRED", "patientInformation.parents.0.pastDrugs.0.indicationMeddraVersion"),
	rule_ich_d_10_8_r_6a_vocabulary: ("ICH.D.10.8.r.6a.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.indicationMeddraVersion"),
	rule_ich_d_10_8_r_6b_required: ("ICH.D.10.8.r.6b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.indicationMeddraCode"),
	rule_ich_d_10_8_r_6b_vocabulary: ("ICH.D.10.8.r.6b.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.indicationMeddraCode"),
	rule_ich_d_10_8_r_7a_required: ("ICH.D.10.8.r.7a.REQUIRED", "patientInformation.parents.0.pastDrugs.0.reactionMeddraVersion"),
	rule_ich_d_10_8_r_7a_vocabulary: ("ICH.D.10.8.r.7a.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.reactionMeddraVersion"),
	rule_ich_d_10_8_r_7b_required: ("ICH.D.10.8.r.7b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.reactionMeddraCode"),
	rule_ich_d_10_8_r_7b_vocabulary: ("ICH.D.10.8.r.7b.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.reactionMeddraCode"),
	rule_ich_d_4_integer: ("ICH.D.4.INTEGER", "patientInformation.heightCm"),
	rule_ich_d_6_nullflavor_allowed: ("ICH.D.6.NULLFLAVOR.ALLOWED", "patientInformation.lastMenstrualPeriodDateNullFlavor"),
	rule_ich_d_7_1_r_1a_allowed_value: ("ICH.D.7.1.r.1a.ALLOWED.VALUE", "patientInformation.medicalHistory.0.meddraVersion"),
	rule_ich_d_7_1_r_1a_required: ("ICH.D.7.1.r.1a.REQUIRED", "patientInformation.medicalHistory.0.meddraVersion"),
	rule_ich_d_7_1_r_1a_vocabulary: ("ICH.D.7.1.r.1a.VOCABULARY", "patientInformation.medicalHistory.0.meddraVersion"),
	rule_ich_d_7_1_r_1b_required: ("ICH.D.7.1.r.1b.REQUIRED", "patientInformation.medicalHistory.0.meddraCode"),
	rule_ich_d_7_1_r_1b_vocabulary: ("ICH.D.7.1.r.1b.VOCABULARY", "patientInformation.medicalHistory.0.meddraCode"),
	rule_ich_d_7_1_r_6_parent_duplicate: ("ICH.D.7.1.r.6.PARENT_DUPLICATE", "patientInformation.medicalHistory.0.familyHistory"),
	rule_ich_d_8_mpid_phpid_exclusive: ("ICH.D.8.MPID_PHPID.EXCLUSIVE", "patientInformation.pastDrugs.0.mpid"),
	rule_ich_d_8_r_1_required: ("ICH.D.8.r.1.REQUIRED", "patientInformation.pastDrugs.0.drugName"),
	rule_ich_d_8_r_6a_required: ("ICH.D.8.r.6a.REQUIRED", "patientInformation.pastDrugs.0.indicationMeddraVersion"),
	rule_ich_d_8_r_6a_vocabulary: ("ICH.D.8.r.6a.VOCABULARY", "patientInformation.pastDrugHistory.0.indicationMeddraVersion"),
	rule_ich_d_8_r_6b_required: ("ICH.D.8.r.6b.REQUIRED", "patientInformation.pastDrugs.0.indicationMeddraCode"),
	rule_ich_d_8_r_6b_vocabulary: ("ICH.D.8.r.6b.VOCABULARY", "patientInformation.pastDrugs.0.indicationMeddraCode"),
	rule_ich_d_8_r_7a_required: ("ICH.D.8.r.7a.REQUIRED", "patientInformation.pastDrugs.0.reactionMeddraVersion"),
	rule_ich_d_8_r_7a_vocabulary: ("ICH.D.8.r.7a.VOCABULARY", "patientInformation.pastDrugHistory.0.reactionMeddraVersion"),
	rule_ich_d_8_r_7b_required: ("ICH.D.8.r.7b.REQUIRED", "patientInformation.pastDrugs.0.reactionMeddraCode"),
	rule_ich_d_8_r_7b_vocabulary: ("ICH.D.8.r.7b.VOCABULARY", "patientInformation.pastDrugs.0.reactionMeddraCode"),
	rule_ich_d_9_2_r_1a_required: ("ICH.D.9.2.r.1a.REQUIRED", "patientInformation.death.reportedCauses.0.meddraVersion"),
	rule_ich_d_9_2_r_1a_vocabulary: ("ICH.D.9.2.r.1a.VOCABULARY", "patientInformation.death.reportedCauses.0.meddraVersion"),
	rule_ich_d_9_2_r_1b_required: ("ICH.D.9.2.r.1b.REQUIRED", "patientInformation.death.reportedCauses.0.meddraCode"),
	rule_ich_d_9_2_r_1b_vocabulary: ("ICH.D.9.2.r.1b.VOCABULARY", "patientInformation.death.reportedCauses.0.meddraCode"),
	rule_ich_d_9_4_r_1a_required: ("ICH.D.9.4.r.1a.REQUIRED", "patientInformation.death.autopsyCauses.0.meddraVersion"),
	rule_ich_d_9_4_r_1a_vocabulary: ("ICH.D.9.4.r.1a.VOCABULARY", "patientInformation.death.autopsyCauses.0.meddraVersion"),
	rule_ich_d_9_4_r_1b_required: ("ICH.D.9.4.r.1b.REQUIRED", "patientInformation.death.autopsyCauses.0.meddraCode"),
	rule_ich_d_9_4_r_1b_vocabulary: ("ICH.D.9.4.r.1b.VOCABULARY", "patientInformation.death.autopsyCauses.0.meddraCode"),
}

#[test]
fn rule_ich_d_1_required() {
	assert_rule("ICH.D.1.REQUIRED", "patientInformation.patientInitials");

	for (report_kind, report_type) in [("study", "2"), ("non-study", "1")] {
		let mut ctx = empty_ctx();
		ctx.patient = Some(patient());
		ctx.safety_report = Some(report(report_type));
		let issues = run_ich(&ctx);
		assert!(
			issues.iter().any(|issue| issue.code == "ICH.D.1.REQUIRED"),
			"{report_kind} report omitted required D.1 identifier: {issues:?}"
		);
	}
	for null_flavor in ["MSK", "ASKU", "NASK", "UNK", "NA", "other"] {
		let mut value = patient();
		value.patient_initials_null_flavor = Some(null_flavor.into());
		let issues = invoke(&Some(value), |patient, issues| {
			d_1(patient.as_ref(), issues)
		});
		assert!(
			!issues.iter().any(|issue| issue.code == "ICH.D.1.REQUIRED"),
			"present D.1 null flavor {null_flavor} did not satisfy REQUIRED: {issues:?}"
		);
	}
	for null_flavor in ["", " "] {
		let mut value = patient();
		value.patient_initials_null_flavor = Some(null_flavor.into());
		let issues = invoke(&Some(value), |patient, issues| {
			d_1(patient.as_ref(), issues)
		});
		assert!(
			issues.iter().any(|issue| issue.code == "ICH.D.1.REQUIRED"),
			"blank D.1 null flavor {null_flavor:?} satisfied REQUIRED: {issues:?}"
		);
	}
}

fn compact_date(date: Date) -> String {
	format!(
		"{:04}{:02}{:02}",
		date.year(),
		u8::from(date.month()),
		date.day()
	)
}

fn date_boundary_issues(
	code: &str,
	date: Date,
	end_date: bool,
) -> Vec<ValidationIssue> {
	match code {
		"ICH.D.2.1.FUTURE_DATE.FORBIDDEN" => {
			let mut value = patient();
			value.birth_date = Some(date);
			invoke(&value, d_2_1)
		}
		"ICH.D.6.FUTURE_DATE.FORBIDDEN" => {
			let mut value = patient();
			value.last_menstrual_period_date = Some(date);
			invoke(&value, d_6)
		}
		"ICH.D.9.1.FUTURE_DATE.FORBIDDEN" => {
			let mut value = death_info();
			value.date_of_death = Some(date);
			invoke(&value, d_9_1)
		}
		"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN" => {
			let mut value = parent(Uuid::nil());
			value.parent_birth_date = Some(date);
			invoke(&value, |parent, issues| d_10_2_1(0, parent, issues))
		}
		"ICH.D.10.3.FUTURE_DATE.FORBIDDEN" => {
			let mut value = parent(Uuid::nil());
			value.last_menstrual_period_date = Some(date);
			invoke(&value, |parent, issues| d_10_3(0, parent, issues))
		}
		"ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN" => {
			let mut value = medhist(None, None);
			if end_date {
				value.end_date = Some(compact_date(date));
			} else {
				value.start_date = Some(compact_date(date));
			}
			invoke(&value, |episode, issues| d_7_1_r(0, episode, issues))
		}
		"ICH.D.8.r.FUTURE_DATE.FORBIDDEN" => {
			let mut value = past_drug();
			if end_date {
				value.end_date = Some(date);
			} else {
				value.start_date = Some(date);
			}
			invoke(&value, |drug, issues| d_8_r(0, drug, issues))
		}
		"ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN" => {
			let mut value = parent_medhist(Uuid::nil(), None, None);
			if end_date {
				value.end_date = Some(date);
			} else {
				value.start_date = Some(date);
			}
			invoke(&value, |episode, issues| d_10_7_1_r(0, 0, episode, issues))
		}
		"ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN" => {
			let mut value = parent_past_drug(Uuid::nil(), None, None);
			if end_date {
				value.end_date = Some(date);
			} else {
				value.start_date = Some(date);
			}
			invoke(&value, |drug, issues| d_10_8_r(0, 0, drug, issues))
		}
		_ => panic!("unknown date-boundary rule: {code}"),
	}
}

fn assert_date_boundaries(code: &str, path: &str) {
	assert_rule(code, path);
	let today = OffsetDateTime::now_utc().date();
	let paired = matches!(
		code,
		"ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN"
			| "ICH.D.8.r.FUTURE_DATE.FORBIDDEN"
			| "ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN"
			| "ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN"
	);
	for end_date in [false, true].into_iter().take(if paired { 2 } else { 1 }) {
		let boundary = if end_date { "end" } else { "start" };
		for allowed in [today.previous_day().expect("previous date"), today] {
			let issues = date_boundary_issues(code, allowed, end_date);
			assert!(
				!issues.iter().any(|issue| issue.code == code),
				"{code} rejected non-future {boundary} date {allowed}: {issues:?}"
			);
		}
		let tomorrow = today.next_day().expect("next date");
		let issues = date_boundary_issues(code, tomorrow, end_date);
		assert!(
			issues
				.iter()
				.any(|issue| issue.code == code && issue.path == path),
			"{code} accepted future {boundary} date {tomorrow}: {issues:?}"
		);
	}
}

macro_rules! date_rule_test {
	($name:ident, $code:literal, $path:literal) => {
		#[test]
		fn $name() {
			assert_date_boundaries($code, $path);
		}
	};
}

date_rule_test!(
	rule_ich_d_2_1_future_date_forbidden,
	"ICH.D.2.1.FUTURE_DATE.FORBIDDEN",
	"patientInformation.patientBirthDate"
);
date_rule_test!(
	rule_ich_d_6_future_date_forbidden,
	"ICH.D.6.FUTURE_DATE.FORBIDDEN",
	"patientInformation.lastMenstrualPeriodDate"
);
date_rule_test!(
	rule_ich_d_7_1_r_future_date_forbidden,
	"ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN",
	"patientInformation.medicalHistoryEpisodes.0.dateRange"
);
date_rule_test!(
	rule_ich_d_8_r_future_date_forbidden,
	"ICH.D.8.r.FUTURE_DATE.FORBIDDEN",
	"patientInformation.pastDrugs.0.dateRange"
);
date_rule_test!(
	rule_ich_d_9_1_future_date_forbidden,
	"ICH.D.9.1.FUTURE_DATE.FORBIDDEN",
	"patientInformation.death.dateOfDeath"
);
date_rule_test!(
	rule_ich_d_10_2_1_future_date_forbidden,
	"ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN",
	"patientInformation.parents.0.parentBirthDate"
);
date_rule_test!(
	rule_ich_d_10_3_future_date_forbidden,
	"ICH.D.10.3.FUTURE_DATE.FORBIDDEN",
	"patientInformation.parents.0.lastMenstrualPeriodDate"
);
date_rule_test!(
	rule_ich_d_10_7_1_r_future_date_forbidden,
	"ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN",
	"patientInformation.parents.0.medicalHistory.0.dateRange"
);
date_rule_test!(
	rule_ich_d_10_8_r_future_date_forbidden,
	"ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN",
	"patientInformation.parents.0.pastDrugs.0.dateRange"
);

fn zero_pair_issues(code: &str, include_unit: bool) -> Vec<ValidationIssue> {
	if code.starts_with("ICH.D.10") {
		let mut value = parent(Uuid::nil());
		value.parent_age = Some(Decimal::ZERO);
		value.parent_age_unit = include_unit.then(|| "a".into());
		if code.contains("2.2a") {
			invoke(&value, |parent, issues| d_10_2_2a(0, parent, issues))
		} else {
			invoke(&value, |parent, issues| d_10_2_2b(0, parent, issues))
		}
	} else {
		let mut value = patient();
		let gestation = code.contains("2.2.1");
		if gestation {
			value.gestation_period = Some(Decimal::ZERO);
			value.gestation_period_unit = include_unit.then(|| "wk".into());
		} else {
			value.age_at_time_of_onset = Some(Decimal::ZERO);
			value.age_unit = include_unit.then(|| "a".into());
		}
		match code {
			"ICH.D.2.2a.REQUIRED" => invoke(&value, d_2_2a),
			"ICH.D.2.2b.REQUIRED" => invoke(&value, d_2_2b),
			"ICH.D.2.2.1a.REQUIRED" => invoke(&value, d_2_2_1a),
			"ICH.D.2.2.1b.REQUIRED" => invoke(&value, d_2_2_1b),
			_ => panic!("unknown zero-pair rule: {code}"),
		}
	}
}

fn assert_zero_pair(code: &str, path: &str, value_rule: bool) {
	assert_rule(code, path);
	let issues = zero_pair_issues(code, true);
	assert!(
		!issues.iter().any(|issue| issue.code == code),
		"zero was not treated as a populated value for {code}: {issues:?}"
	);
	let issues = zero_pair_issues(code, false);
	assert_eq!(
		issues.iter().any(|issue| issue.code == code),
		value_rule,
		"zero/missing-unit pairing mismatch for {code}: {issues:?}"
	);
}

macro_rules! zero_pair_test {
	($name:ident, $code:literal, $path:literal, $value_rule:literal) => {
		#[test]
		fn $name() {
			assert_zero_pair($code, $path, $value_rule);
		}
	};
}

zero_pair_test!(
	rule_ich_d_2_2a_required,
	"ICH.D.2.2a.REQUIRED",
	"patientInformation.ageAtTimeOfOnset",
	false
);
zero_pair_test!(
	rule_ich_d_2_2b_required,
	"ICH.D.2.2b.REQUIRED",
	"patientInformation.ageUnit",
	true
);
zero_pair_test!(
	rule_ich_d_2_2_1a_required,
	"ICH.D.2.2.1a.REQUIRED",
	"patientInformation.gestationPeriod",
	false
);
zero_pair_test!(
	rule_ich_d_2_2_1b_required,
	"ICH.D.2.2.1b.REQUIRED",
	"patientInformation.gestationPeriodUnit",
	true
);
zero_pair_test!(
	rule_ich_d_10_2_2a_required,
	"ICH.D.10.2.2a.REQUIRED",
	"patientInformation.parents.0.parentAge",
	false
);
zero_pair_test!(
	rule_ich_d_10_2_2b_required,
	"ICH.D.10.2.2b.REQUIRED",
	"patientInformation.parents.0.parentAgeUnit",
	true
);

#[test]
fn rule_ich_d_2_exclusive() {
	assert_rule("ICH.D.2.EXCLUSIVE", "patientInformation.patientBirthDate");
	for conflict in ["birthDate", "age", "ageGroup"] {
		let mut value = patient();
		value.gestation_period = Some(Decimal::ONE);
		match conflict {
			"birthDate" => value.birth_date = Some(past_date()),
			"age" => value.age_at_time_of_onset = Some(Decimal::ONE),
			"ageGroup" => value.age_group = Some("3".into()),
			_ => unreachable!(),
		}
		let issues = invoke(&value, d_2);
		assert!(issues.iter().any(|issue| issue.code == "ICH.D.2.EXCLUSIVE"));
	}
	let mut gestation_only = patient();
	gestation_only.gestation_period_unit = Some("wk".into());
	assert!(!invoke(&gestation_only, d_2)
		.iter()
		.any(|issue| issue.code == "ICH.D.2.EXCLUSIVE"));
}

#[test]
fn rule_ich_d_10_2_exclusive() {
	assert_rule(
		"ICH.D.10.2.EXCLUSIVE",
		"patientInformation.parents.0.parentBirthDate",
	);
	let mut age_only = parent(Uuid::nil());
	age_only.parent_age = Some(Decimal::ZERO);
	assert!(
		!invoke(&age_only, |parent, issues| d_10_2(0, parent, issues))
			.iter()
			.any(|issue| issue.code == "ICH.D.10.2.EXCLUSIVE")
	);
}

#[test]
fn rule_ich_d_7_2_required() {
	assert_rule(
		"ICH.D.7.2.REQUIRED",
		"patientInformation.medicalHistoryText",
	);
	let mut value = patient();
	value.medical_history_text = None;
	assert!(
		!invoke(&value, |patient, issues| d_7_2(patient, false, issues))
			.iter()
			.any(|issue| issue.code == "ICH.D.7.2.REQUIRED")
	);
	value.medical_history_text_null_flavor = Some("UNK".into());
	assert!(
		!invoke(&value, |patient, issues| d_7_2(patient, true, issues))
			.iter()
			.any(|issue| issue.code == "ICH.D.7.2.REQUIRED")
	);
}

fn assert_death_comments_pairing(code: &str, autopsy: bool) {
	assert_rule(
		code,
		if autopsy {
			"patientInformation.death.autopsyCauses.0.comments"
		} else {
			"patientInformation.death.reportedCauses.0.comments"
		},
	);
	for (version, meddra_code, expected) in [
		(Some("26.1"), None, true),
		(None, Some("10000001"), true),
		(None, None, false),
	] {
		let issues = if autopsy {
			invoke(
				&autopsy_cause(meddra_code, version, None),
				|cause, issues| d_9_4_r_2(0, cause, issues),
			)
		} else {
			invoke(
				&reported_cause(meddra_code, version, None),
				|cause, issues| d_9_2_r_2(0, cause, issues),
			)
		};
		assert_eq!(issues.iter().any(|issue| issue.code == code), expected);
	}
}

#[test]
fn rule_ich_d_9_2_r_2_required() {
	assert_death_comments_pairing("ICH.D.9.2.r.2.REQUIRED", false);
}

#[test]
fn rule_ich_d_9_4_r_2_required() {
	assert_death_comments_pairing("ICH.D.9.4.r.2.REQUIRED", true);
}

#[test]
fn ich_d_8_and_d_10_8_identifier_versions_are_optional() {
	let mut patient_mpid = past_drug();
	patient_mpid.mpid = Some("MPID".into());
	let mut issues = Vec::new();
	d_8_r_2a(0, &patient_mpid, &mut issues);
	assert!(
		issues.is_empty(),
		"optional patient MPID version was required: {issues:?}"
	);
	patient_mpid.mpid = None;
	patient_mpid.phpid = Some("PHPID".into());
	issues.clear();
	d_8_r_3a(0, &patient_mpid, &mut issues);
	assert!(
		issues.is_empty(),
		"optional patient PhPID version was required: {issues:?}"
	);

	let mut mpid = parent_past_drug(Uuid::nil(), Some("MPID"), None);
	let issues = invoke(&mpid, |drug, issues| d_10_8_r_2a(0, 0, drug, issues));
	assert!(
		issues.is_empty(),
		"optional parent MPID version was required: {issues:?}"
	);

	mpid.mpid = None;
	mpid.phpid = Some("PHPID".into());
	let issues = invoke(&mpid, |drug, issues| d_10_8_r_3a(0, 0, drug, issues));
	assert!(
		issues.is_empty(),
		"optional parent PhPID version was required: {issues:?}"
	);
}

#[test]
fn ich_d_1_1_4_is_optional_for_study_reports() {
	let issues = invoke(&Vec::new(), |identifiers, issues| {
		d_1_1_4(identifiers, issues)
	});
	assert!(
		!issues
			.iter()
			.any(|issue| issue.code == "ICH.D.1.1.4.REQUIRED"),
		"optional ICH D.1.1.4 was rejected: {issues:?}"
	);
}

#[test]
fn rule_ich_d_10_6_required() {
	assert_rule("ICH.D.10.6.REQUIRED", "patientInformation.parents.0.sex");

	let mut null_flavor_sex = parent(Uuid::nil());
	null_flavor_sex.parent_age = Some(Decimal::ONE);
	null_flavor_sex.sex_null_flavor = Some("UNK".into());
	let issues = invoke(&null_flavor_sex, |parent, issues| {
		d_10_6(0, parent, false, issues)
	});
	assert!(
		!issues
			.iter()
			.any(|issue| issue.code == "ICH.D.10.6.REQUIRED"),
		"allowed parent sex null flavor was rejected: {issues:?}"
	);

	let mut null_flavor_payload = parent(Uuid::nil());
	null_flavor_payload.parent_identification_null_flavor = Some("MSK".into());
	let issues = invoke(&null_flavor_payload, |parent, issues| {
		d_10_6(0, parent, false, issues)
	});
	assert!(
		issues
			.iter()
			.any(|issue| issue.code == "ICH.D.10.6.REQUIRED"),
		"null-flavor-only parent payload omitted required sex: {issues:?}"
	);
}

#[test]
fn rule_ich_d_9_3_required() {
	assert_rule(
		"ICH.D.9.3.REQUIRED",
		"patientInformation.death.autopsyPerformed",
	);

	for null_flavor in ["UNK", "ASKU", "NASK"] {
		let mut death = death_info();
		death.date_of_death = Some(past_date());
		death.autopsy_performed = None;
		death.autopsy_performed_null_flavor = Some(null_flavor.into());
		let issues = invoke(&death, d_9_3);
		assert!(
			!issues
				.iter()
				.any(|issue| issue.code == "ICH.D.9.3.REQUIRED"),
			"allowed autopsy null flavor {null_flavor} was rejected: {issues:?}"
		);
	}

	let mut null_flavor_death = death_info();
	null_flavor_death.date_of_death = None;
	null_flavor_death.date_of_death_null_flavor = Some("UNK".into());
	null_flavor_death.autopsy_performed = None;
	null_flavor_death.autopsy_performed_null_flavor = None;
	let missing = invoke(&null_flavor_death, d_9_3);
	assert!(
		missing
			.iter()
			.any(|issue| issue.code == "ICH.D.9.3.REQUIRED"),
		"D.9.1 nullFlavor did not trigger D.9.3: {missing:?}"
	);

	null_flavor_death.autopsy_performed = Some(false);
	let repaired = invoke(&null_flavor_death, d_9_3);
	assert!(
		!repaired
			.iter()
			.any(|issue| issue.code == "ICH.D.9.3.REQUIRED"),
		"populated D.9.3 did not repair nullFlavor-triggered requirement: {repaired:?}"
	);

	let mut blank_death = death_info();
	blank_death.date_of_death = None;
	blank_death.date_of_death_null_flavor = Some("  ".into());
	blank_death.autopsy_performed = None;
	let blank = invoke(&blank_death, d_9_3);
	assert!(
		!blank.iter().any(|issue| issue.code == "ICH.D.9.3.REQUIRED"),
		"blank D.9.1 nullFlavor triggered D.9.3: {blank:?}"
	);
}

fn assert_meddra_consistency(
	code: &str,
	path: &str,
	invalid: &ValidationContext,
	repaired: &ValidationContext,
) {
	let run = |ctx: &ValidationContext| {
		let mut issues = Vec::new();
		super::super::super::collect_meddra_version_issues(ctx, &mut issues);
		issues
	};
	let invalid = run(invalid);
	let repaired = run(repaired);
	assert!(
		invalid
			.iter()
			.any(|issue| issue.code == code && issue.path == path),
		"{code} missing at {path}: {invalid:?}"
	);
	assert!(
		!repaired.iter().any(|issue| issue.code == code),
		"{code} remained after repair: {repaired:?}"
	);
}

#[test]
fn rule_ich_d_7_1_r_1a_meddra_version_consistent() {
	let mut invalid = empty_ctx();
	invalid.medical_history = vec![
		medhist(None, Some("26.1")),
		medhist(None, Some("  ")),
		medhist(None, Some("27.0")),
	];
	let mut repaired = invalid.clone();
	repaired.medical_history[2].meddra_version = Some("26.1".into());
	assert_meddra_consistency(
		"ICH.D.7.1.r.1a.MEDDRA.VERSION.CONSISTENT",
		"patientInformation.medicalHistory.2.meddraVersion",
		&invalid,
		&repaired,
	);
}

#[test]
fn rule_ich_d_8_r_6a_meddra_version_consistent() {
	let mut invalid = empty_ctx();
	invalid.medical_history.push(medhist(None, Some("26.1")));
	invalid.past_drugs = vec![past_drug(), past_drug()];
	invalid.past_drugs[0].indication_meddra_version = Some(" ".into());
	invalid.past_drugs[1].indication_meddra_version = Some("27.0".into());
	let mut repaired = invalid.clone();
	repaired.past_drugs[1].indication_meddra_version = Some("26.1".into());
	assert_meddra_consistency(
		"ICH.D.8.r.6a.MEDDRA.VERSION.CONSISTENT",
		"patientInformation.pastDrugHistory.1.indicationMeddraVersion",
		&invalid,
		&repaired,
	);
}

#[test]
fn rule_ich_d_8_r_7a_meddra_version_consistent() {
	let mut invalid = empty_ctx();
	invalid.medical_history.push(medhist(None, Some("26.1")));
	invalid.past_drugs = vec![past_drug(), past_drug()];
	invalid.past_drugs[0].reaction_meddra_version = Some(" ".into());
	invalid.past_drugs[1].reaction_meddra_version = Some("27.0".into());
	let mut repaired = invalid.clone();
	repaired.past_drugs[1].reaction_meddra_version = Some("26.1".into());
	assert_meddra_consistency(
		"ICH.D.8.r.7a.MEDDRA.VERSION.CONSISTENT",
		"patientInformation.pastDrugHistory.1.reactionMeddraVersion",
		&invalid,
		&repaired,
	);
}

#[test]
fn rule_ich_d_9_2_r_1a_meddra_version_consistent() {
	let mut invalid = empty_ctx();
	invalid.medical_history.push(medhist(None, Some("26.1")));
	invalid.reported_causes_of_death = vec![
		reported_cause(None, Some(" "), None),
		reported_cause(None, Some("27.0"), None),
	];
	let mut repaired = invalid.clone();
	repaired.reported_causes_of_death[1].meddra_version = Some("26.1".into());
	assert_meddra_consistency(
		"ICH.D.9.2.r.1a.MEDDRA.VERSION.CONSISTENT",
		"patientInformation.death.reportedCauses.1.meddraVersion",
		&invalid,
		&repaired,
	);
}

#[test]
fn rule_ich_d_9_4_r_1a_meddra_version_consistent() {
	let mut invalid = empty_ctx();
	invalid.medical_history.push(medhist(None, Some("26.1")));
	invalid.autopsy_causes_of_death = vec![
		autopsy_cause(None, Some(" "), None),
		autopsy_cause(None, Some("27.0"), None),
	];
	let mut repaired = invalid.clone();
	repaired.autopsy_causes_of_death[1].meddra_version = Some("26.1".into());
	assert_meddra_consistency(
		"ICH.D.9.4.r.1a.MEDDRA.VERSION.CONSISTENT",
		"patientInformation.death.autopsyCauses.1.meddraVersion",
		&invalid,
		&repaired,
	);
}

fn parent_consistency_context(past_drug: bool, version: &str) -> ValidationContext {
	let first_parent = Uuid::from_u128(1);
	let second_parent = Uuid::from_u128(2);
	let mut ctx = empty_ctx();
	ctx.medical_history.push(medhist(None, Some("26.1")));
	ctx.parents = vec![parent(first_parent), parent(second_parent)];
	if past_drug {
		ctx.parent_past_drugs = vec![
			parent_past_drug(second_parent, None, None),
			parent_past_drug(first_parent, None, None),
			parent_past_drug(second_parent, None, None),
		];
		ctx.parent_past_drugs[0].indication_meddra_version = Some(" ".into());
		ctx.parent_past_drugs[2].indication_meddra_version = Some(version.into());
		ctx.parent_past_drugs[2].reaction_meddra_version = Some(version.into());
	} else {
		ctx.parent_medical_history = vec![
			parent_medhist(second_parent, None, Some(" ")),
			parent_medhist(first_parent, None, None),
			parent_medhist(second_parent, None, Some(version)),
		];
	}
	ctx
}

#[test]
fn rule_ich_d_10_7_1_r_1a_meddra_version_consistent() {
	let invalid = parent_consistency_context(false, "27.0");
	let repaired = parent_consistency_context(false, "26.1");
	assert_meddra_consistency(
		"ICH.D.10.7.1.r.1a.MEDDRA.VERSION.CONSISTENT",
		"patientInformation.parents.1.medicalHistory.1.meddraVersion",
		&invalid,
		&repaired,
	);
}

#[test]
fn rule_ich_d_10_8_r_6a_meddra_version_consistent() {
	let invalid = parent_consistency_context(true, "27.0");
	let repaired = parent_consistency_context(true, "26.1");
	assert_meddra_consistency(
		"ICH.D.10.8.r.6a.MEDDRA.VERSION.CONSISTENT",
		"patientInformation.parents.1.pastDrugs.1.indicationMeddraVersion",
		&invalid,
		&repaired,
	);
}

#[test]
fn rule_ich_d_10_8_r_7a_meddra_version_consistent() {
	let invalid = parent_consistency_context(true, "27.0");
	let repaired = parent_consistency_context(true, "26.1");
	assert_meddra_consistency(
		"ICH.D.10.8.r.7a.MEDDRA.VERSION.CONSISTENT",
		"patientInformation.parents.1.pastDrugs.1.reactionMeddraVersion",
		&invalid,
		&repaired,
	);
}

#[test]
fn rule_ich_d_7_1_r_6_allowed_value() {
	for allowed_value in [None, Some(true)] {
		let mut episode = medhist(None, None);
		episode.family_history = allowed_value;
		let issues =
			invoke(&episode, |episode, issues| d_7_1_r_6(2, episode, issues));
		assert!(
			!issues
				.iter()
				.any(|issue| issue.code == "ICH.D.7.1.r.6.ALLOWED.VALUE"),
			"allowed family history value was rejected: {issues:?}"
		);
	}
	let mut episode = medhist(None, None);
	episode.family_history = Some(false);
	assert_edge(
		"ICH.D.7.1.r.6.ALLOWED.VALUE",
		"patientInformation.medicalHistory.2.familyHistory",
		invoke(&episode, |episode, issues| d_7_1_r_6(2, episode, issues)),
		invoke(&medhist(None, None), |episode, issues| {
			d_7_1_r_6(2, episode, issues)
		}),
	);
}

#[test]
fn rule_ich_d_7_3_allowed_value() {
	for allowed_value in [None, Some(true)] {
		let mut value = patient();
		value.concomitant_therapy = allowed_value;
		let issues = invoke(&value, d_7_3);
		assert!(
			!issues
				.iter()
				.any(|issue| issue.code == "ICH.D.7.3.ALLOWED.VALUE"),
			"allowed concomitant therapy value was rejected: {issues:?}"
		);
	}
	let mut invalid = patient();
	invalid.concomitant_therapy = Some(false);
	assert_edge(
		"ICH.D.7.3.ALLOWED.VALUE",
		"patientInformation.concomitantTherapy",
		invoke(&invalid, d_7_3),
		invoke(&patient(), d_7_3),
	);
}

#[test]
fn rule_ich_d_10_2_2b_allowed_value() {
	for allowed_value in ["a", "10.a"] {
		let mut value = parent(Uuid::nil());
		value.parent_age_unit = Some(allowed_value.into());
		let issues = invoke(&value, |parent, issues| d_10_2_2b(0, parent, issues));
		assert!(
			!issues
				.iter()
				.any(|issue| issue.code == "ICH.D.10.2.2b.ALLOWED.VALUE"),
			"allowed parent age unit {allowed_value} was rejected: {issues:?}"
		);
	}
	let mut invalid = parent(Uuid::nil());
	invalid.parent_age_unit = Some("wk".into());
	assert_edge(
		"ICH.D.10.2.2b.ALLOWED.VALUE",
		"patientInformation.parents.0.parentAgeUnit",
		invoke(&invalid, |parent, issues| d_10_2_2b(0, parent, issues)),
		invoke(&parent(Uuid::nil()), |parent, issues| {
			d_10_2_2b(0, parent, issues)
		}),
	);
}
