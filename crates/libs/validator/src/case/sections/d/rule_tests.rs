use super::golden_companion_tests::{
	autopsy_cause, death_info, empty_ctx, medhist, parent, parent_medhist,
	parent_past_drug, past_drug, patient, patient_identifier, reported_cause,
};
use super::*;
use lib_core::model::case_identifiers::LinkedReportNumber;
use lib_core::model::message_header::MessageHeader;
use lib_core::model::reaction::Reaction;
use lib_core::model::safety_report::{SafetyReportIdentification, StudyInformation};
use sqlx::types::time::Date;
use sqlx::types::time::OffsetDateTime;
use time::Month;

fn invoke<T>(
	value: &T,
	function: impl Fn(&T, &mut Vec<ValidationIssue>),
) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	function(value, &mut issues);
	issues
}

fn run_ich(ctx: &ValidationContext) -> Vec<ValidationIssue> {
	let mut issues = Vec::new();
	collect_ich_issues(ctx, &mut issues);
	super::super::collect_meddra_version_issues(ctx, &mut issues);
	issues
}

fn future_date() -> Date {
	Date::from_calendar_date(2999, Month::January, 1).expect("valid fixture date")
}

fn past_date() -> Date {
	Date::from_calendar_date(2020, Month::January, 1).expect("valid fixture date")
}

fn report(report_type: &str) -> SafetyReportIdentification {
	SafetyReportIdentification {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		safety_report_id: None,
		version: 1,
		transmission_date: None,
		report_type: Some(report_type.into()),
		date_first_received_from_source: None,
		date_of_most_recent_information: None,
		fulfil_expedited_criteria: None,
		fulfil_expedited_criteria_null_flavor: None,
		local_criteria_report_type: None,
		combination_product_report_indicator: None,
		combination_product_report_indicator_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		receiver_organization: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn message(receiver: &str) -> MessageHeader {
	MessageHeader {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		batch_number: None,
		batch_sender_identifier: None,
		batch_receiver_identifier: None,
		batch_transmission_date: None,
		message_type: "ichicsr".into(),
		message_format_version: "2.1".into(),
		message_format_release: "2.0".into(),
		message_number: "1".into(),
		message_sender_identifier: "sender".into(),
		message_receiver_identifier: receiver.into(),
		message_date_format: "204".into(),
		message_date: "20200101000000".into(),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn death_reaction() -> Reaction {
	Reaction {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		sequence_number: 1,
		primary_source_reaction: None,
		primary_source_reaction_translation: None,
		reaction_language: None,
		reaction_meddra_version: None,
		reaction_meddra_code: None,
		term_highlighted: None,
		serious: Some(true),
		criteria_death: Some(true),
		criteria_death_null_flavor: None,
		criteria_life_threatening: None,
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: None,
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: None,
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: None,
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: None,
		criteria_other_medically_important_null_flavor: None,
		required_intervention: None,
		required_intervention_null_flavor: None,
		expectedness: None,
		severity: None,
		mfds_device_ae_classification: None,
		mfds_device_ae_outcome: None,
		mfds_device_cause_medical_device: None,
		mfds_device_cause_procedure_issue: None,
		mfds_device_cause_patient_condition: None,
		mfds_device_cause_unable_to_assess: None,
		mfds_device_cause_other: None,
		mfds_device_action_reason: None,
		mfds_device_action_recall: None,
		mfds_device_action_repair: None,
		mfds_device_action_inspection: None,
		mfds_device_action_replacement: None,
		mfds_device_action_improvement: None,
		mfds_device_action_monitoring: None,
		mfds_device_action_notification: None,
		mfds_device_action_label_change: None,
		mfds_device_action_other: None,
		start_date: None,
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		duration_value: None,
		duration_unit: None,
		outcome: None,
		medical_confirmation: None,
		country_code: None,
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn ind_study() -> StudyInformation {
	StudyInformation {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		source_study_presave_id: None,
		study_name: None,
		study_name_null_flavor: None,
		sponsor_study_number: None,
		sponsor_study_number_null_flavor: None,
		study_type_reaction: None,
		study_type_reaction_kr1: None,
		fda_ind_number_occurred: Some("123".into()),
		fda_pre_anda_number_occurred: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn linked_report() -> LinkedReportNumber {
	LinkedReportNumber {
		id: Uuid::nil(),
		case_id: Uuid::nil(),
		sequence_number: 1,
		linked_report_number: "linked".into(),
		deleted: false,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::field_tests::record_rule_result(&invalid, &repaired);
	let legacy_version_code = matches!(
		code,
		"ICH.D.7.1.r.1a.VOCABULARY"
			| "ICH.D.8.r.6a.VOCABULARY"
			| "ICH.D.8.r.7a.VOCABULARY"
			| "ICH.D.9.2.r.1a.VOCABULARY"
			| "ICH.D.9.4.r.1a.VOCABULARY"
			| "ICH.D.10.7.1.r.1a.VOCABULARY"
			| "ICH.D.10.8.r.6a.VOCABULARY"
			| "ICH.D.10.8.r.7a.VOCABULARY"
	);
	let emitted_code = if legacy_version_code {
		"ICH.MEDDRA.VERSION.UNAVAILABLE"
	} else {
		code
	};
	assert!(
		invalid
			.iter()
			.any(|issue| issue.code == emitted_code && issue.path == path),
		"{emitted_code} missing at {path}: {invalid:?}"
	);
	assert!(
		!repaired.iter().any(|issue| issue.code == emitted_code),
		"{emitted_code} remained after repair: {repaired:?}"
	);
	if legacy_version_code {
		assert!(!invalid.iter().any(|issue| issue.code == code));
		assert!(!repaired.iter().any(|issue| issue.code == code));
	}
}

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
	let (path, invalid, repaired) = if code.starts_with("FDA.") {
		fda_edge(code)
	} else if code.starts_with("MFDS.D.8") || code.starts_with("MFDS.D.10.8") {
		mfds_past(code)
	} else if code.starts_with("MFDS.") {
		mfds_required_edge(code)
	} else if matches!(code, "ICH.D.1.REQUIRED" | "ICH.D.1.1.4.REQUIRED") {
		if code == "ICH.D.1.REQUIRED" {
			let mut good = patient();
			good.patient_initials = Some("AB".into());
			(
				"patientInformation.patientInitials",
				invoke(&None, |p, i| d_1(p.as_ref(), false, i)),
				invoke(&Some(good), |p, i| d_1(p.as_ref(), false, i)),
			)
		} else {
			let bad = Vec::new();
			let good = vec![patient_identifier("4", "STUDY")];
			(
				"patientInformation.patientStudyNumber",
				invoke(&bad, |v, i| d_1_1_4(v, true, i)),
				invoke(&good, |v, i| d_1_1_4(v, true, i)),
			)
		}
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
	rule_fda_d_11_nullflavor_route: ("FDA.D.11.NULLFLAVOR.ROUTE", "patientInformation.raceCodeNullFlavor"),
	rule_fda_d_11_required: ("FDA.D.11.REQUIRED", "patientInformation.raceCodes"),
	rule_fda_d_11_r_1_required: ("FDA.D.11.r.1.REQUIRED", "patientInformation.raceCodes"),
	rule_fda_d_12_nullflavor_route: ("FDA.D.12.NULLFLAVOR.ROUTE", "patientInformation.ethnicityCodeNullFlavor"),
	rule_fda_d_12_required: ("FDA.D.12.REQUIRED", "patientInformation.ethnicityCode"),
	rule_fda_d_2_required: ("FDA.D.2.REQUIRED", "patientInformation.patientBirthDate"),
	rule_fda_d_9_1_required: ("FDA.D.9.1.REQUIRED", "patientInformation.death.dateOfDeath"),
	rule_fda_w0003: ("FDA.W0003", "patientInformation.raceCodeNullFlavor"),
	rule_fda_w0004: ("FDA.W0004", "patientInformation.ethnicityCodeNullFlavor"),
	rule_fda_w0010: ("FDA.W0010", "patientInformation.patientInitials"),
	rule_ich_d_1_1_4_required: ("ICH.D.1.1.4.REQUIRED", "patientInformation.patientStudyNumber"),
	rule_ich_d_1_required: ("ICH.D.1.REQUIRED", "patientInformation.patientInitials"),
	rule_ich_d_10_2_1_future_date_forbidden: ("ICH.D.10.2.1.FUTURE_DATE.FORBIDDEN", "patientInformation.parents.0.parentBirthDate"),
	rule_ich_d_10_2_2a_required: ("ICH.D.10.2.2a.REQUIRED", "patientInformation.parents.0.parentAge"),
	rule_ich_d_10_2_2b_required: ("ICH.D.10.2.2b.REQUIRED", "patientInformation.parents.0.parentAgeUnit"),
	rule_ich_d_10_2_exclusive: ("ICH.D.10.2.EXCLUSIVE", "patientInformation.parents.0.parentBirthDate"),
	rule_ich_d_10_3_future_date_forbidden: ("ICH.D.10.3.FUTURE_DATE.FORBIDDEN", "patientInformation.parents.0.lastMenstrualPeriodDate"),
	rule_ich_d_10_5_integer: ("ICH.D.10.5.INTEGER", "patientInformation.parents.0.heightCm"),
	rule_ich_d_10_6_required: ("ICH.D.10.6.REQUIRED", "patientInformation.parents.0.sex"),
	rule_ich_d_10_7_1_r_1a_allowed_value: ("ICH.D.10.7.1.r.1a.ALLOWED.VALUE", "patientInformation.parents.0.medicalHistory.0.meddraVersion"),
	rule_ich_d_10_7_1_r_1a_required: ("ICH.D.10.7.1.r.1a.REQUIRED", "patientInformation.parents.0.medicalHistory.0.meddraVersion"),
	rule_ich_d_10_7_1_r_1a_vocabulary: ("ICH.D.10.7.1.r.1a.VOCABULARY", "patientInformation.parents.0.medicalHistory.0.meddraVersion"),
	rule_ich_d_10_7_1_r_1b_required: ("ICH.D.10.7.1.r.1b.REQUIRED", "patientInformation.parents.0.medicalHistory.0.meddraCode"),
	rule_ich_d_10_7_1_r_1b_vocabulary: ("ICH.D.10.7.1.r.1b.VOCABULARY", "patientInformation.parents.0.medicalHistory.0.meddraCode"),
	rule_ich_d_10_7_1_r_future_date_forbidden: ("ICH.D.10.7.1.r.FUTURE_DATE.FORBIDDEN", "patientInformation.parents.0.medicalHistory.0.dateRange"),
	rule_ich_d_10_8_mpid_phpid_exclusive: ("ICH.D.10.8.MPID_PHPID.EXCLUSIVE", "patientInformation.parents.0.pastDrugs.0.mpid"),
	rule_ich_d_10_8_r_2a_required: ("ICH.D.10.8.r.2a.REQUIRED", "patientInformation.parents.0.pastDrugs.0.mpidVersion"),
	rule_ich_d_10_8_r_3a_required: ("ICH.D.10.8.r.3a.REQUIRED", "patientInformation.parents.0.pastDrugs.0.phpidVersion"),
	rule_ich_d_10_8_r_6a_required: ("ICH.D.10.8.r.6a.REQUIRED", "patientInformation.parents.0.pastDrugs.0.indicationMeddraVersion"),
	rule_ich_d_10_8_r_6a_vocabulary: ("ICH.D.10.8.r.6a.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.indicationMeddraVersion"),
	rule_ich_d_10_8_r_6b_required: ("ICH.D.10.8.r.6b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.indicationMeddraCode"),
	rule_ich_d_10_8_r_6b_vocabulary: ("ICH.D.10.8.r.6b.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.indicationMeddraCode"),
	rule_ich_d_10_8_r_7a_required: ("ICH.D.10.8.r.7a.REQUIRED", "patientInformation.parents.0.pastDrugs.0.reactionMeddraVersion"),
	rule_ich_d_10_8_r_7a_vocabulary: ("ICH.D.10.8.r.7a.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.reactionMeddraVersion"),
	rule_ich_d_10_8_r_7b_required: ("ICH.D.10.8.r.7b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.reactionMeddraCode"),
	rule_ich_d_10_8_r_7b_vocabulary: ("ICH.D.10.8.r.7b.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.reactionMeddraCode"),
	rule_ich_d_10_8_r_future_date_forbidden: ("ICH.D.10.8.r.FUTURE_DATE.FORBIDDEN", "patientInformation.parents.0.pastDrugs.0.dateRange"),
	rule_ich_d_2_1_future_date_forbidden: ("ICH.D.2.1.FUTURE_DATE.FORBIDDEN", "patientInformation.patientBirthDate"),
	rule_ich_d_2_2_1a_required: ("ICH.D.2.2.1a.REQUIRED", "patientInformation.gestationPeriod"),
	rule_ich_d_2_2_1b_required: ("ICH.D.2.2.1b.REQUIRED", "patientInformation.gestationPeriodUnit"),
	rule_ich_d_2_2a_required: ("ICH.D.2.2a.REQUIRED", "patientInformation.ageAtTimeOfOnset"),
	rule_ich_d_2_2b_required: ("ICH.D.2.2b.REQUIRED", "patientInformation.ageUnit"),
	rule_ich_d_2_exclusive: ("ICH.D.2.EXCLUSIVE", "patientInformation.patientBirthDate"),
	rule_ich_d_4_integer: ("ICH.D.4.INTEGER", "patientInformation.heightCm"),
	rule_ich_d_6_future_date_forbidden: ("ICH.D.6.FUTURE_DATE.FORBIDDEN", "patientInformation.lastMenstrualPeriodDate"),
	rule_ich_d_6_nullflavor_allowed: ("ICH.D.6.NULLFLAVOR.ALLOWED", "patientInformation.lastMenstrualPeriodDateNullFlavor"),
	rule_ich_d_7_1_r_1a_allowed_value: ("ICH.D.7.1.r.1a.ALLOWED.VALUE", "patientInformation.medicalHistory.0.meddraVersion"),
	rule_ich_d_7_1_r_1a_required: ("ICH.D.7.1.r.1a.REQUIRED", "patientInformation.medicalHistory.0.meddraVersion"),
	rule_ich_d_7_1_r_1a_vocabulary: ("ICH.D.7.1.r.1a.VOCABULARY", "patientInformation.medicalHistory.0.meddraVersion"),
	rule_ich_d_7_1_r_1b_required: ("ICH.D.7.1.r.1b.REQUIRED", "patientInformation.medicalHistory.0.meddraCode"),
	rule_ich_d_7_1_r_1b_vocabulary: ("ICH.D.7.1.r.1b.VOCABULARY", "patientInformation.medicalHistory.0.meddraCode"),
	rule_ich_d_7_1_r_6_parent_duplicate: ("ICH.D.7.1.r.6.PARENT_DUPLICATE", "patientInformation.medicalHistory.0.familyHistory"),
	rule_ich_d_7_1_r_future_date_forbidden: ("ICH.D.7.1.r.FUTURE_DATE.FORBIDDEN", "patientInformation.medicalHistoryEpisodes.0.dateRange"),
	rule_ich_d_7_2_required: ("ICH.D.7.2.REQUIRED", "patientInformation.medicalHistoryText"),
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
	rule_ich_d_8_r_future_date_forbidden: ("ICH.D.8.r.FUTURE_DATE.FORBIDDEN", "patientInformation.pastDrugs.0.dateRange"),
	rule_ich_d_9_1_future_date_forbidden: ("ICH.D.9.1.FUTURE_DATE.FORBIDDEN", "patientInformation.death.dateOfDeath"),
	rule_ich_d_9_2_r_1a_required: ("ICH.D.9.2.r.1a.REQUIRED", "patientInformation.death.reportedCauses.0.meddraVersion"),
	rule_ich_d_9_2_r_1a_vocabulary: ("ICH.D.9.2.r.1a.VOCABULARY", "patientInformation.death.reportedCauses.0.meddraVersion"),
	rule_ich_d_9_2_r_1b_required: ("ICH.D.9.2.r.1b.REQUIRED", "patientInformation.death.reportedCauses.0.meddraCode"),
	rule_ich_d_9_2_r_1b_vocabulary: ("ICH.D.9.2.r.1b.VOCABULARY", "patientInformation.death.reportedCauses.0.meddraCode"),
	rule_ich_d_9_2_r_2_required: ("ICH.D.9.2.r.2.REQUIRED", "patientInformation.death.reportedCauses.0.comments"),
	rule_ich_d_9_3_required: ("ICH.D.9.3.REQUIRED", "patientInformation.death.autopsyPerformed"),
	rule_ich_d_9_4_r_1a_required: ("ICH.D.9.4.r.1a.REQUIRED", "patientInformation.death.autopsyCauses.0.meddraVersion"),
	rule_ich_d_9_4_r_1a_vocabulary: ("ICH.D.9.4.r.1a.VOCABULARY", "patientInformation.death.autopsyCauses.0.meddraVersion"),
	rule_ich_d_9_4_r_1b_required: ("ICH.D.9.4.r.1b.REQUIRED", "patientInformation.death.autopsyCauses.0.meddraCode"),
	rule_ich_d_9_4_r_1b_vocabulary: ("ICH.D.9.4.r.1b.VOCABULARY", "patientInformation.death.autopsyCauses.0.meddraCode"),
	rule_ich_d_9_4_r_2_required: ("ICH.D.9.4.r.2.REQUIRED", "patientInformation.death.autopsyCauses.0.comments"),
	rule_mfds_d_1_1_4_required: ("MFDS.D.1.1.4.REQUIRED", "patientInformation.patientStudyNumber"),
	rule_mfds_d_10_8_r_1_kr_1a_required: ("MFDS.D.10.8.r.1.KR.1a.REQUIRED", "patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductVersion"),
	rule_mfds_d_10_8_r_1_kr_1b_required: ("MFDS.D.10.8.r.1.KR.1b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductId"),
	rule_mfds_d_10_8_r_1_kr_1b_vocabulary: ("MFDS.D.10.8.r.1.KR.1b.VOCABULARY", "patientInformation.parents.0.pastDrugs.0.mfdsMedicinalProductId"),
	rule_mfds_d_10_8_r_2b_required: ("MFDS.D.10.8.r.2b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.mpid"),
	rule_mfds_d_10_8_r_3b_required: ("MFDS.D.10.8.r.3b.REQUIRED", "patientInformation.parents.0.pastDrugs.0.phpid"),
	rule_mfds_d_2_2a_required: ("MFDS.D.2.2a.REQUIRED", "patientInformation.ageAtTimeOfOnset"),
	rule_mfds_d_2_2b_required: ("MFDS.D.2.2b.REQUIRED", "patientInformation.ageUnit"),
	rule_mfds_d_5_required: ("MFDS.D.5.REQUIRED", "patientInformation.sex"),
	rule_mfds_d_8_r_1_kr_1a_required: ("MFDS.D.8.r.1.KR.1a.REQUIRED", "patientInformation.pastDrugHistory.0.mfdsMedicinalProductVersion"),
	rule_mfds_d_8_r_1_kr_1b_required: ("MFDS.D.8.r.1.KR.1b.REQUIRED", "patientInformation.pastDrugHistory.0.mfdsMedicinalProductId"),
	rule_mfds_d_8_r_1_kr_1b_vocabulary: ("MFDS.D.8.r.1.KR.1b.VOCABULARY", "patientInformation.pastDrugHistory.0.mfdsMedicinalProductId"),
	rule_mfds_d_8_r_2a_required: ("MFDS.D.8.r.2a.REQUIRED", "patientInformation.pastDrugHistory.0.mpidVersion"),
	rule_mfds_d_8_r_2b_required: ("MFDS.D.8.r.2b.REQUIRED", "patientInformation.pastDrugHistory.0.mpid"),
	rule_mfds_d_8_r_3a_required: ("MFDS.D.8.r.3a.REQUIRED", "patientInformation.pastDrugHistory.0.phpidVersion"),
	rule_mfds_d_8_r_3b_required: ("MFDS.D.8.r.3b.REQUIRED", "patientInformation.pastDrugHistory.0.phpid"),
}

include!("rule_test_bindings.rs");
