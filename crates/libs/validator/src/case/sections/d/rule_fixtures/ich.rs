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
			super::super::collect_meddra_version_issues(&global_ctx, &mut issues);
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
	} else if code == "ICH.D.8.r.2a.REQUIRED" {
		bad.mpid = Some("mpid".into());
		good = bad.clone();
		good.mpid_version = Some("1".into());
		"patientInformation.pastDrugs.0.mpidVersion"
	} else if code == "ICH.D.8.r.3a.REQUIRED" {
		bad.phpid = Some("phpid".into());
		good = bad.clone();
		good.phpid_version = Some("1".into());
		"patientInformation.pastDrugs.0.phpidVersion"
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
	} else if code.contains(".r.2a.") {
		bad.mpid = Some("m".into());
		"mpidVersion"
	} else if code.contains(".r.3a.") {
		bad.phpid = Some("p".into());
		"phpidVersion"
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
		"mpidVersion" => good.mpid_version = Some("1".into()),
		"phpidVersion" => good.phpid_version = Some("1".into()),
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
