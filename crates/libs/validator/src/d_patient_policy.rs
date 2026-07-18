use super::{is_rule_condition_satisfied, RuleFacts};
use lib_core::model::patient::PatientInformation;

// Shared Section D policy used by exporter + case validators.

pub fn has_patient_payload(patient: &PatientInformation) -> bool {
	super::has_text(patient.patient_initials.as_deref())
		|| patient.birth_date.is_some()
		|| patient.age_at_time_of_onset.is_some()
		|| patient.sex.is_some()
}

pub fn should_require_patient_initials(_patient: &PatientInformation) -> bool {
	false
}

pub fn has_patient_initials(patient: &PatientInformation) -> bool {
	super::has_text(patient.patient_initials.as_deref())
}

pub fn should_require_fda_race(patient: &PatientInformation) -> bool {
	is_rule_condition_satisfied(
		"FDA.D.11.REQUIRED",
		RuleFacts {
			fda_patient_payload_present: Some(has_patient_payload(patient)),
			..RuleFacts::default()
		},
	)
}

pub fn should_require_fda_ethnicity(patient: &PatientInformation) -> bool {
	is_rule_condition_satisfied(
		"FDA.D.12.REQUIRED",
		RuleFacts {
			fda_patient_payload_present: Some(has_patient_payload(patient)),
			..RuleFacts::default()
		},
	)
}

pub fn has_fda_race(patient: &PatientInformation) -> bool {
	super::has_text(patient.race_code.as_deref())
}

pub fn has_fda_ethnicity(patient: &PatientInformation) -> bool {
	super::has_text(patient.ethnicity_code.as_deref())
}

#[cfg(test)]
mod tests {
	use super::*;
	use sqlx::types::Uuid;
	use time::OffsetDateTime;

	fn empty_patient() -> PatientInformation {
		PatientInformation {
			id: Uuid::new_v4(),
			case_id: Uuid::new_v4(),
			patient_initials: None,
			birth_date: None,
			age_at_time_of_onset: None,
			age_unit: None,
			gestation_period: None,
			gestation_period_unit: None,
			age_group: None,
			weight_kg: None,
			weight_kg_null_flavor: None,
			height_cm: None,
			height_cm_null_flavor: None,
			sex: None,
			patient_initials_null_flavor: None,
			birth_date_null_flavor: None,
			age_at_time_of_onset_null_flavor: None,
			sex_null_flavor: None,
			race_code: None,
			race_code_null_flavor: None,
			ethnicity_code: None,
			ethnicity_code_null_flavor: None,
			last_menstrual_period_date: None,
			last_menstrual_period_date_null_flavor: None,
			medical_history_text: None,
			medical_history_text_null_flavor: None,
			concomitant_therapy: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::new_v4(),
			updated_by: None,
		}
	}

	#[test]
	fn payload_detection_is_false_for_empty_patient() {
		let patient = empty_patient();
		assert!(!has_patient_payload(&patient));
		assert!(!should_require_patient_initials(&patient));
	}

	#[test]
	fn payload_detection_is_false_when_only_sex_present() {
		let mut patient = empty_patient();
		patient.sex = Some("1".to_string());
		assert!(has_patient_payload(&patient));
		assert!(!should_require_patient_initials(&patient));
	}
}
