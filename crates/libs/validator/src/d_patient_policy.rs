use lib_core::model::patient::PatientInformation;

// Shared Section D policy used by exporter + case validators.

pub fn has_patient_payload(patient: &PatientInformation) -> bool {
	super::has_text(patient.patient_initials.as_deref())
		|| patient.birth_date.is_some()
		|| patient.age_at_time_of_onset.is_some()
		|| patient.sex.is_some()
}

pub fn should_require_patient_initials(_patient: &PatientInformation) -> bool {
	true
}

pub fn has_patient_initials(patient: &PatientInformation) -> bool {
	super::has_text(patient.patient_initials.as_deref())
		|| super::has_text(patient.patient_initials_null_flavor.as_deref())
}

pub fn should_require_fda_race(patient: &PatientInformation) -> bool {
	has_patient_payload(patient)
}

pub fn should_require_fda_ethnicity(patient: &PatientInformation) -> bool {
	has_patient_payload(patient)
}

pub fn has_fda_race(patient: &PatientInformation) -> bool {
	patient
		.race_codes
		.iter()
		.any(|value| super::has_text(Some(value.as_str())))
}

pub fn has_fda_ethnicity(patient: &PatientInformation) -> bool {
	super::has_text(patient.ethnicity_code.as_deref())
}

#[cfg(test)]
#[path = "tests/d_patient_policy.rs"]
mod tests;
