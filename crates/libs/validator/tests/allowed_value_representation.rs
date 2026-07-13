use lib_core::model::drug::{
	DosageInformationForUpdate, DrugActiveSubstanceForUpdate,
	DrugInformationForUpdate,
};
use lib_core::model::drug_reaction_assessment::DrugReactionAssessmentForUpdate;
use lib_core::model::message_header::MessageHeaderForUpdate;
use lib_core::model::parent_history::{
	ParentMedicalHistoryForUpdate, ParentPastDrugHistoryForUpdate,
};
use lib_core::model::patient::{
	MedicalHistoryEpisodeForUpdate, ParentInformationForUpdate,
	PastDrugHistoryForUpdate, PatientDeathInformationForUpdate,
	PatientInformationForUpdate,
};
use lib_core::model::reaction::ReactionForUpdate;
use lib_core::model::safety_report::SafetyReportIdentificationForUpdate;
use lib_core::model::test_result::TestResultForUpdate;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use validator::representation_enforced_rule_codes;

struct BoundaryCase {
	code: &'static str,
	rejects: fn() -> bool,
}

fn rejects<T: DeserializeOwned>(field: &str, invalid: Value) -> bool {
	let mut value = Value::Object(Default::default());
	value[field] = invalid;
	serde_json::from_value::<T>(value).is_err()
}

macro_rules! boundary_case {
	($code:literal, $type:ty, $field:literal, $invalid:expr) => {
		BoundaryCase {
			code: $code,
			rejects: || rejects::<$type>($field, $invalid),
		}
	};
}

fn boundary_cases() -> Vec<BoundaryCase> {
	vec![
		// Boolean-backed fields.
		boundary_case!(
			"ICH.C.1.6.1.ALLOWED.VALUE",
			SafetyReportIdentificationForUpdate,
			"additional_documents_available",
			json!("1")
		),
		boundary_case!(
			"ICH.C.1.7.ALLOWED.VALUE",
			SafetyReportIdentificationForUpdate,
			"fulfil_expedited_criteria",
			json!("1")
		),
		boundary_case!(
			"ICH.D.7.1.r.3.ALLOWED.VALUE",
			MedicalHistoryEpisodeForUpdate,
			"continuing",
			json!("1")
		),
		boundary_case!(
			"ICH.D.9.3.ALLOWED.VALUE",
			PatientDeathInformationForUpdate,
			"autopsy_performed",
			json!("1")
		),
		boundary_case!(
			"ICH.D.10.7.1.r.3.ALLOWED.VALUE",
			ParentMedicalHistoryForUpdate,
			"continuing",
			json!("1")
		),
		boundary_case!(
			"ICH.E.i.8.ALLOWED.VALUE",
			ReactionForUpdate,
			"medical_confirmation",
			json!("1")
		),
		boundary_case!(
			"ICH.F.r.7.ALLOWED.VALUE",
			TestResultForUpdate,
			"more_info_available",
			json!("1")
		),
		boundary_case!(
			"ICH.E.i.3.1.ALLOWED.VALUE",
			ReactionForUpdate,
			"term_highlighted",
			json!("1")
		),
		// Decimal/integer-backed fields.
		boundary_case!(
			"ICH.D.2.2a.ALLOWED.VALUE",
			PatientInformationForUpdate,
			"age_at_time_of_onset",
			json!("12mg")
		),
		boundary_case!(
			"ICH.D.2.2.1a.ALLOWED.VALUE",
			PatientInformationForUpdate,
			"gestation_period",
			json!("12mg")
		),
		boundary_case!(
			"ICH.D.3.ALLOWED.VALUE",
			PatientInformationForUpdate,
			"weight_kg",
			json!("12mg")
		),
		boundary_case!(
			"ICH.D.4.ALLOWED.VALUE",
			PatientInformationForUpdate,
			"height_cm",
			json!("12mg")
		),
		boundary_case!(
			"ICH.D.10.2.2a.ALLOWED.VALUE",
			ParentInformationForUpdate,
			"parent_age",
			json!("12mg")
		),
		boundary_case!(
			"ICH.D.10.4.ALLOWED.VALUE",
			ParentInformationForUpdate,
			"weight_kg",
			json!("12mg")
		),
		boundary_case!(
			"ICH.D.10.5.ALLOWED.VALUE",
			ParentInformationForUpdate,
			"height_cm",
			json!("12mg")
		),
		boundary_case!(
			"ICH.E.i.6a.ALLOWED.VALUE",
			ReactionForUpdate,
			"duration_value",
			json!("12mg")
		),
		boundary_case!(
			"ICH.G.k.2.3.r.3a.ALLOWED.VALUE",
			DrugActiveSubstanceForUpdate,
			"strength_value",
			json!("12mg")
		),
		boundary_case!(
			"ICH.G.k.4.r.1a.ALLOWED.VALUE",
			DosageInformationForUpdate,
			"dose_value",
			json!("12mg")
		),
		boundary_case!(
			"ICH.G.k.4.r.2.ALLOWED.VALUE",
			DosageInformationForUpdate,
			"number_of_units",
			json!(1.5)
		),
		boundary_case!(
			"ICH.G.k.4.r.6a.ALLOWED.VALUE",
			DosageInformationForUpdate,
			"duration_value",
			json!("12mg")
		),
		boundary_case!(
			"ICH.G.k.5a.ALLOWED.VALUE",
			DrugInformationForUpdate,
			"cumulative_dose_first_reaction_value",
			json!("12mg")
		),
		boundary_case!(
			"ICH.G.k.6a.ALLOWED.VALUE",
			DrugInformationForUpdate,
			"gestation_period_exposure_value",
			json!("12mg")
		),
		boundary_case!(
			"ICH.G.k.9.i.3.1a.ALLOWED.VALUE",
			DrugReactionAssessmentForUpdate,
			"administration_start_interval_value",
			json!("12mg")
		),
		boundary_case!(
			"ICH.G.k.9.i.3.2a.ALLOWED.VALUE",
			DrugReactionAssessmentForUpdate,
			"last_dose_interval_value",
			json!("12mg")
		),
		// Date/OffsetDateTime-backed fields.
		boundary_case!(
			"ICH.N.1.5.ALLOWED.VALUE",
			MessageHeaderForUpdate,
			"batch_transmission_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.C.1.4.ALLOWED.VALUE",
			SafetyReportIdentificationForUpdate,
			"date_first_received_from_source",
			json!("20230230")
		),
		boundary_case!(
			"ICH.C.1.5.ALLOWED.VALUE",
			SafetyReportIdentificationForUpdate,
			"date_of_most_recent_information",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.2.1.ALLOWED.VALUE",
			PatientInformationForUpdate,
			"birth_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.6.ALLOWED.VALUE",
			PatientInformationForUpdate,
			"last_menstrual_period_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.7.1.r.2.ALLOWED.VALUE",
			MedicalHistoryEpisodeForUpdate,
			"start_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.7.1.r.4.ALLOWED.VALUE",
			MedicalHistoryEpisodeForUpdate,
			"end_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.8.r.4.ALLOWED.VALUE",
			PastDrugHistoryForUpdate,
			"start_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.8.r.5.ALLOWED.VALUE",
			PastDrugHistoryForUpdate,
			"end_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.9.1.ALLOWED.VALUE",
			PatientDeathInformationForUpdate,
			"date_of_death",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.10.2.1.ALLOWED.VALUE",
			ParentInformationForUpdate,
			"parent_birth_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.10.3.ALLOWED.VALUE",
			ParentInformationForUpdate,
			"last_menstrual_period_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.10.7.1.r.2.ALLOWED.VALUE",
			ParentMedicalHistoryForUpdate,
			"start_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.10.7.1.r.4.ALLOWED.VALUE",
			ParentMedicalHistoryForUpdate,
			"end_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.10.8.r.4.ALLOWED.VALUE",
			ParentPastDrugHistoryForUpdate,
			"start_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.D.10.8.r.5.ALLOWED.VALUE",
			ParentPastDrugHistoryForUpdate,
			"end_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.E.i.4.ALLOWED.VALUE",
			ReactionForUpdate,
			"start_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.E.i.5.ALLOWED.VALUE",
			ReactionForUpdate,
			"end_date",
			json!("20230230")
		),
		boundary_case!(
			"ICH.F.r.1.ALLOWED.VALUE",
			TestResultForUpdate,
			"test_date",
			json!("20230230")
		),
	]
}

#[test]
fn every_representation_enforced_code_rejects_invalid_input() {
	let cases = boundary_cases();
	assert_eq!(cases.len(), 43);
	let accepted = cases
		.iter()
		.filter(|case| !(case.rejects)())
		.map(|case| case.code)
		.collect::<Vec<_>>();
	assert!(
		accepted.is_empty(),
		"invalid representations accepted: {accepted:?}"
	);

	let tested = cases.iter().map(|case| case.code).collect::<BTreeSet<_>>();
	assert_eq!(tested, representation_enforced_rule_codes());
}
