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

include!("rule_fixtures/common.rs");

fn assert_edge(
	code: &str,
	path: &str,
	invalid: Vec<ValidationIssue>,
	repaired: Vec<ValidationIssue>,
) {
	crate::rule_assertions::assert_rule_result(code, &invalid, &repaired);
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

include!("rule_fixtures/ich.rs");

include!("rule_fixtures/fda.rs");

include!("rule_fixtures/mfds.rs");

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
