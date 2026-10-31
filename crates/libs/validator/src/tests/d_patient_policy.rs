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
		height_cm: None,
		sex: None,
		patient_initials_null_flavor: None,
		birth_date_null_flavor: None,
		sex_null_flavor: None,
		race_codes: Vec::new(),
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
