use crate::model::patient::{
	AutopsyCauseOfDeath, PatientDeathInformation, PatientInformation,
	ReportedCauseOfDeath,
};
use crate::xml::raw::patch::{
	patch_d_patient, DPatientDeathCausePatch, DPatientPatch,
};
use crate::xml::Result;
use libxml::parser::Parser;

pub fn export_d_patient_patch(
	raw_xml: &[u8],
	patient: &PatientInformation,
	death_info: Option<&PatientDeathInformation>,
	reported_causes: &[ReportedCauseOfDeath],
	autopsy_causes: &[AutopsyCauseOfDeath],
) -> Result<String> {
	let patient_name = build_patient_name(patient);
	let age_value = patient.age_at_time_of_onset.as_ref().map(|v| v.to_string());
	let weight_kg = patient.weight_kg.as_ref().map(|v| v.to_string());
	let height_cm = patient.height_cm.as_ref().map(|v| v.to_string());
	let reported_cause_patches: Vec<DPatientDeathCausePatch<'_>> = reported_causes
		.iter()
		.map(|cause| DPatientDeathCausePatch {
			meddra_version: cause.meddra_version.as_deref(),
			meddra_code: cause.meddra_code.as_deref(),
			comments: cause.comments.as_deref(),
		})
		.collect();
	let autopsy_cause_patches: Vec<DPatientDeathCausePatch<'_>> = autopsy_causes
		.iter()
		.map(|cause| DPatientDeathCausePatch {
			meddra_version: cause.meddra_version.as_deref(),
			meddra_code: cause.meddra_code.as_deref(),
			comments: cause.comments.as_deref(),
		})
		.collect();

	let patch = DPatientPatch {
		patient_name: patient_name.as_deref(),
		sex: patient.sex.as_deref(),
		birth_date: patient.birth_date,
		age_value: age_value.as_deref(),
		age_unit: patient.age_unit.as_deref(),
		weight_kg: weight_kg.as_deref(),
		height_cm: height_cm.as_deref(),
		date_of_death: death_info.and_then(|death| death.date_of_death),
		autopsy_performed: death_info.and_then(|death| death.autopsy_performed),
		reported_causes: &reported_cause_patches,
		autopsy_causes: &autopsy_cause_patches,
	};

	patch_d_patient(raw_xml, &patch)
}

/// Build a minimal ICSR XML skeleton and populate Section D using mapping-driven patching.
pub fn export_d_patient_xml(patient: &PatientInformation) -> Result<String> {
	let base_xml = base_d_patient_skeleton();
	let parser = Parser::default();
	let doc = parser.parse_string(base_xml).map_err(|err| {
		crate::xml::error::Error::InvalidXml {
			message: format!("XML parse error (base skeleton): {err}"),
			line: None,
			column: None,
		}
	})?;
	let raw = doc.to_string();
	export_d_patient_patch(raw.as_bytes(), patient, None, &[], &[])
}

fn base_d_patient_skeleton() -> &'static str {
	"<?xml version=\"1.0\" encoding=\"utf-8\"?>\
<MCCI_IN200100UV01 xmlns=\"urn:hl7-org:v3\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ITSVersion=\"XML_1.0\">\
\t<PORR_IN049016UV>\
\t\t<controlActProcess classCode=\"CACT\" moodCode=\"EVN\">\
\t\t\t<code code=\"PORR_TE049016UV\" codeSystem=\"2.16.840.1.113883.1.18\"/>\
\t\t\t<subject>\
\t\t\t\t<investigationEvent classCode=\"INVSTG\" moodCode=\"EVN\">\
\t\t\t\t\t<component typeCode=\"COMP\">\
\t\t\t\t\t\t<adverseEventAssessment classCode=\"INVSTG\" moodCode=\"EVN\"/>\
\t\t\t\t\t</component>\
\t\t\t\t</investigationEvent>\
\t\t\t</subject>\
\t\t</controlActProcess>\
\t</PORR_IN049016UV>\
</MCCI_IN200100UV01>"
}

fn build_patient_name(patient: &PatientInformation) -> Option<String> {
	let given = patient.patient_given_name.as_deref().unwrap_or("").trim();
	let family = patient.patient_family_name.as_deref().unwrap_or("").trim();
	if !given.is_empty() || !family.is_empty() {
		let mut name = String::new();
		if !given.is_empty() {
			name.push_str(given);
		}
		if !family.is_empty() {
			if !name.is_empty() {
				name.push(' ');
			}
			name.push_str(family);
		}
		return Some(name);
	}
	patient.patient_initials.clone()
}
