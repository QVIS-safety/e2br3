use super::*;
use crate::model::patient::PatientInformation;
use crate::model::patient::{AutopsyCauseOfDeath, ReportedCauseOfDeath};
use crate::xml::export::roundtrip::{
	patch_d_patient, DPatientDeathCausePatch, DPatientPatch,
};

pub(crate) async fn export_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
	raw_xml: &[u8],
) -> Result<String> {
	let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	let death_info = fetch_death_info(mm, patient.id).await?;
	let reported_causes =
		fetch_reported_causes(mm, death_info.as_ref().map(|death| death.id)).await?;
	let autopsy_causes =
		fetch_autopsy_causes(mm, death_info.as_ref().map(|death| death.id)).await?;
	export_d_patient_patch(
		raw_xml,
		&patient,
		death_info.as_ref(),
		&reported_causes,
		&autopsy_causes,
	)
}

pub(crate) async fn export_build(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<String> {
	let patient = PatientInformationBmc::get_by_case(ctx, mm, case_id)
		.await
		.map_err(Error::from)?;
	export_d_patient_xml(&patient)
}

async fn fetch_death_info(
	mm: &ModelManager,
	patient_id: sqlx::types::Uuid,
) -> Result<Option<PatientDeathInformation>> {
	mm.dbx()
		.fetch_optional(
			sqlx::query_as::<_, PatientDeathInformation>(
				"SELECT * FROM patient_death_information WHERE patient_id = $1 LIMIT 1",
			)
			.bind(patient_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

async fn fetch_reported_causes(
	mm: &ModelManager,
	death_info_id: Option<sqlx::types::Uuid>,
) -> Result<Vec<ReportedCauseOfDeath>> {
	let Some(death_info_id) = death_info_id else {
		return Ok(Vec::new());
	};
	mm.dbx()
		.fetch_all(
				sqlx::query_as::<_, ReportedCauseOfDeath>(
				"SELECT * FROM reported_causes_of_death WHERE death_info_id = $1 AND deleted = false ORDER BY sequence_number",
			)
			.bind(death_info_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

async fn fetch_autopsy_causes(
	mm: &ModelManager,
	death_info_id: Option<sqlx::types::Uuid>,
) -> Result<Vec<AutopsyCauseOfDeath>> {
	let Some(death_info_id) = death_info_id else {
		return Ok(Vec::new());
	};
	mm.dbx()
		.fetch_all(
				sqlx::query_as::<_, AutopsyCauseOfDeath>(
				"SELECT * FROM autopsy_causes_of_death WHERE death_info_id = $1 AND deleted = false ORDER BY sequence_number",
			)
			.bind(death_info_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)
}

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
