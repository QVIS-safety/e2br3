use crate::ctx::Ctx;
use crate::model;
use crate::model::parent_history::{
	ParentMedicalHistoryBmc, ParentMedicalHistoryForCreate,
	ParentMedicalHistoryForUpdate, ParentPastDrugHistoryBmc,
	ParentPastDrugHistoryForCreate, ParentPastDrugHistoryForUpdate,
};
use crate::model::patient::{
	AutopsyCauseOfDeathBmc, AutopsyCauseOfDeathForCreate,
	AutopsyCauseOfDeathForUpdate, MedicalHistoryEpisodeBmc,
	MedicalHistoryEpisodeForCreate, MedicalHistoryEpisodeForUpdate,
	ParentInformationBmc, ParentInformationForCreate, ParentInformationForUpdate,
	PastDrugHistoryBmc, PastDrugHistoryForCreate, PastDrugHistoryForUpdate,
	PatientDeathInformationBmc, PatientDeathInformationForCreate,
	PatientDeathInformationForUpdate, PatientIdentifierBmc,
	PatientIdentifierForCreate, PatientIdentifierForUpdate, PatientInformationBmc,
	PatientInformationForCreate, PatientInformationForUpdate,
	ReportedCauseOfDeathBmc, ReportedCauseOfDeathForCreate,
	ReportedCauseOfDeathForUpdate,
};
use crate::model::ModelManager;
use crate::xml::import_runtime::helpers::d as d_helpers;
use crate::xml::Result;
use sqlx::types::Uuid;

pub(crate) async fn import_section_d(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<()> {
	let patient_id = import_patient_information(ctx, mm, xml, case_id).await?;
	if let Some(patient_id) = patient_id {
		import_patient_identifiers(ctx, mm, xml, patient_id).await?;
		import_medical_history(ctx, mm, xml, patient_id).await?;
		import_past_drug_history(ctx, mm, xml, patient_id).await?;
		import_patient_death(ctx, mm, xml, patient_id).await?;
		import_parent_information(ctx, mm, xml, patient_id).await?;
	}
	Ok(())
}

async fn import_patient_identifiers(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let ids = d_helpers::parse_patient_identifiers(xml)?;
	for (idx, entry) in ids.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM patient_identifiers WHERE patient_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(patient_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = PatientIdentifierBmc::update(
				ctx,
				mm,
				id,
				PatientIdentifierForUpdate {
					identifier_type_code: Some(entry.identifier_type_code),
					identifier_value: Some(entry.identifier_value),
				},
			)
			.await;
		} else {
			let _ = PatientIdentifierBmc::create(
				ctx,
				mm,
				PatientIdentifierForCreate {
					patient_id,
					sequence_number: seq,
					identifier_type_code: entry.identifier_type_code,
					identifier_value: entry.identifier_value,
				},
			)
			.await?;
		}
	}
	Ok(())
}

async fn import_medical_history(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let episodes = d_helpers::parse_medical_history(xml)?;
	for (idx, entry) in episodes.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM medical_history_episodes WHERE patient_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(patient_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = MedicalHistoryEpisodeBmc::update(
				ctx,
				mm,
				id,
				MedicalHistoryEpisodeForUpdate {
					meddra_version: entry.meddra_version,
					meddra_code: entry.meddra_code.clone(),
					start_date: entry.start_date,
					start_date_null_flavor: None,
					continuing: entry.continuing,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					comments: entry.comments,
					family_history: entry.family_history,
				},
			)
			.await;
		} else {
			let id = MedicalHistoryEpisodeBmc::create(
				ctx,
				mm,
				MedicalHistoryEpisodeForCreate {
					patient_id,
					sequence_number: seq,
					meddra_code: entry.meddra_code.clone(),
					start_date_null_flavor: None,
					end_date_null_flavor: None,
				},
			)
			.await?;
			let _ = MedicalHistoryEpisodeBmc::update(
				ctx,
				mm,
				id,
				MedicalHistoryEpisodeForUpdate {
					meddra_version: entry.meddra_version,
					meddra_code: entry.meddra_code.clone(),
					start_date: entry.start_date,
					start_date_null_flavor: None,
					continuing: entry.continuing,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					comments: entry.comments,
					family_history: entry.family_history,
				},
			)
			.await;
		}
	}
	Ok(())
}

async fn import_past_drug_history(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let items = d_helpers::parse_past_drug_history(xml)?;
	for (idx, entry) in items.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM past_drug_history WHERE patient_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(patient_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = PastDrugHistoryBmc::update(
				ctx,
				mm,
				id,
				PastDrugHistoryForUpdate {
					drug_name: entry.drug_name,
					drug_name_null_flavor: None,
					mfds_medicinal_product_version: entry
						.mfds_medicinal_product_version,
					mfds_medicinal_product_id: entry.mfds_medicinal_product_id,
					mpid: entry.mpid,
					mpid_version: entry.mpid_version,
					phpid: entry.phpid,
					phpid_version: entry.phpid_version,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					indication_meddra_version: entry.indication_meddra_version,
					indication_meddra_code: entry.indication_meddra_code,
					reaction_meddra_version: entry.reaction_meddra_version,
					reaction_meddra_code: entry.reaction_meddra_code,
				},
			)
			.await;
		} else {
			let _ = PastDrugHistoryBmc::create(
				ctx,
				mm,
				PastDrugHistoryForCreate {
					patient_id,
					sequence_number: seq,
					drug_name: entry.drug_name,
					drug_name_null_flavor: None,
					mfds_medicinal_product_version: entry
						.mfds_medicinal_product_version,
					mfds_medicinal_product_id: entry.mfds_medicinal_product_id,
					mpid: entry.mpid,
					mpid_version: entry.mpid_version,
					phpid: entry.phpid,
					phpid_version: entry.phpid_version,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					indication_meddra_version: entry.indication_meddra_version,
					indication_meddra_code: entry.indication_meddra_code,
					reaction_meddra_version: entry.reaction_meddra_version,
					reaction_meddra_code: entry.reaction_meddra_code,
				},
			)
			.await?;
		}
	}
	Ok(())
}

async fn import_patient_death(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let Some(death) = d_helpers::parse_patient_death(xml)? else {
		return Ok(());
	};

	let death_id = if let Some((id,)) = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM patient_death_information WHERE patient_id = $1 LIMIT 1",
			)
			.bind(patient_id),
		)
		.await
		.map_err(model::Error::from)?
	{
		id
	} else {
		PatientDeathInformationBmc::create(
			ctx,
			mm,
			PatientDeathInformationForCreate {
				patient_id,
				date_of_death: death.date_of_death,
				date_of_death_null_flavor: death.date_of_death_null_flavor.clone(),
				autopsy_performed: death.autopsy_performed,
			},
		)
		.await?
	};

	let _ = PatientDeathInformationBmc::update(
		ctx,
		mm,
		death_id,
		PatientDeathInformationForUpdate {
			date_of_death: death.date_of_death,
			date_of_death_null_flavor: death.date_of_death_null_flavor,
			autopsy_performed: death.autopsy_performed,
		},
	)
	.await;

	for (idx, cause) in death.reported_causes.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM reported_causes_of_death WHERE death_info_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(death_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = ReportedCauseOfDeathBmc::update(
				ctx,
				mm,
				id,
				ReportedCauseOfDeathForUpdate {
					meddra_version: cause.meddra_version,
					meddra_code: cause.meddra_code.clone(),
					comments: cause.comments.clone(),
				},
			)
			.await;
		} else {
			ReportedCauseOfDeathBmc::create(
				ctx,
				mm,
				ReportedCauseOfDeathForCreate {
					death_info_id: death_id,
					sequence_number: seq,
					meddra_version: cause.meddra_version.clone(),
					meddra_code: cause.meddra_code.clone(),
					comments: cause.comments.clone(),
				},
			)
			.await?;
		}
	}

	for (idx, cause) in death.autopsy_causes.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM autopsy_causes_of_death WHERE death_info_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(death_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = AutopsyCauseOfDeathBmc::update(
				ctx,
				mm,
				id,
				AutopsyCauseOfDeathForUpdate {
					meddra_version: cause.meddra_version,
					meddra_code: cause.meddra_code.clone(),
					comments: cause.comments.clone(),
				},
			)
			.await;
		} else {
			AutopsyCauseOfDeathBmc::create(
				ctx,
				mm,
				AutopsyCauseOfDeathForCreate {
					death_info_id: death_id,
					sequence_number: seq,
					meddra_version: cause.meddra_version.clone(),
					meddra_code: cause.meddra_code.clone(),
					comments: cause.comments.clone(),
				},
			)
			.await?;
		}
	}

	Ok(())
}

async fn import_parent_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	patient_id: Uuid,
) -> Result<()> {
	let Some(parent) = d_helpers::parse_parent_information(xml)? else {
		return Ok(());
	};

	let parent_id = if let Some((id,)) = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM parent_information WHERE patient_id = $1 LIMIT 1",
			)
			.bind(patient_id),
		)
		.await
		.map_err(model::Error::from)?
	{
		id
	} else {
		ParentInformationBmc::create(
			ctx,
			mm,
			ParentInformationForCreate {
				patient_id,
				parent_identification: parent.parent_identification.clone(),
				parent_birth_date: parent.parent_birth_date,
				parent_birth_date_null_flavor: parent
					.parent_birth_date_null_flavor
					.clone(),
				parent_age: parent.parent_age,
				parent_age_null_flavor: parent.parent_age_null_flavor.clone(),
				parent_age_unit: parent.parent_age_unit.clone(),
				last_menstrual_period_date: parent.last_menstrual_period_date,
				last_menstrual_period_date_null_flavor: parent
					.last_menstrual_period_date_null_flavor
					.clone(),
				weight_kg: parent.weight_kg,
				height_cm: parent.height_cm,
				sex: parent.sex.clone(),
				medical_history_text: parent.medical_history_text.clone(),
			},
		)
		.await?
	};

	let _ = ParentInformationBmc::update(
		ctx,
		mm,
		parent_id,
		ParentInformationForUpdate {
			parent_identification: parent.parent_identification,
			parent_birth_date: parent.parent_birth_date,
			parent_birth_date_null_flavor: parent.parent_birth_date_null_flavor,
			parent_age: parent.parent_age,
			parent_age_null_flavor: parent.parent_age_null_flavor,
			parent_age_unit: parent.parent_age_unit,
			last_menstrual_period_date: parent.last_menstrual_period_date,
			last_menstrual_period_date_null_flavor: parent
				.last_menstrual_period_date_null_flavor,
			weight_kg: parent.weight_kg,
			height_cm: parent.height_cm,
			sex: parent.sex,
			medical_history_text: parent.medical_history_text,
		},
	)
	.await;

	for (idx, entry) in parent.medical_history.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM parent_medical_history WHERE parent_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(parent_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = ParentMedicalHistoryBmc::update(
				ctx,
				mm,
				id,
				ParentMedicalHistoryForUpdate {
					meddra_version: entry.meddra_version,
					meddra_code: entry.meddra_code,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					continuing: entry.continuing,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					comments: entry.comments,
				},
			)
			.await;
		} else {
			let meddra_code = entry.meddra_code.clone();
			let id = ParentMedicalHistoryBmc::create(
				ctx,
				mm,
				ParentMedicalHistoryForCreate {
					parent_id,
					sequence_number: seq,
					meddra_code,
					start_date_null_flavor: None,
					end_date_null_flavor: None,
				},
			)
			.await?;
			let _ = ParentMedicalHistoryBmc::update(
				ctx,
				mm,
				id,
				ParentMedicalHistoryForUpdate {
					meddra_version: entry.meddra_version,
					meddra_code: entry.meddra_code,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					continuing: entry.continuing,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					comments: entry.comments,
				},
			)
			.await;
		}
	}

	for (idx, entry) in parent.past_drugs.into_iter().enumerate() {
		let seq = (idx + 1) as i32;
		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM parent_past_drug_history WHERE parent_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(parent_id)
				.bind(seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);
		if let Some(id) = existing {
			let _ = ParentPastDrugHistoryBmc::update(
				ctx,
				mm,
				id,
				ParentPastDrugHistoryForUpdate {
					drug_name: entry.drug_name,
					drug_name_null_flavor: None,
					mpid: entry.mpid,
					mpid_version: entry.mpid_version,
					mfds_medicinal_product_version: entry
						.mfds_medicinal_product_version,
					mfds_medicinal_product_id: entry.mfds_medicinal_product_id,
					phpid: entry.phpid,
					phpid_version: entry.phpid_version,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					indication_meddra_version: entry.indication_meddra_version,
					indication_meddra_code: entry.indication_meddra_code,
					reaction_meddra_version: entry.reaction_meddra_version,
					reaction_meddra_code: entry.reaction_meddra_code,
				},
			)
			.await;
		} else {
			let drug_name = entry.drug_name.clone();
			let id = ParentPastDrugHistoryBmc::create(
				ctx,
				mm,
				ParentPastDrugHistoryForCreate {
					parent_id,
					sequence_number: seq,
					drug_name,
					drug_name_null_flavor: None,
					mfds_medicinal_product_version: entry
						.mfds_medicinal_product_version
						.clone(),
					mfds_medicinal_product_id: entry
						.mfds_medicinal_product_id
						.clone(),
					start_date_null_flavor: None,
					end_date_null_flavor: None,
				},
			)
			.await?;
			let _ = ParentPastDrugHistoryBmc::update(
				ctx,
				mm,
				id,
				ParentPastDrugHistoryForUpdate {
					drug_name: entry.drug_name,
					drug_name_null_flavor: None,
					mpid: entry.mpid,
					mpid_version: entry.mpid_version,
					mfds_medicinal_product_version: entry
						.mfds_medicinal_product_version,
					mfds_medicinal_product_id: entry.mfds_medicinal_product_id,
					phpid: entry.phpid,
					phpid_version: entry.phpid_version,
					start_date: entry.start_date,
					start_date_null_flavor: None,
					end_date: entry.end_date,
					end_date_null_flavor: None,
					indication_meddra_version: entry.indication_meddra_version,
					indication_meddra_code: entry.indication_meddra_code,
					reaction_meddra_version: entry.reaction_meddra_version,
					reaction_meddra_code: entry.reaction_meddra_code,
				},
			)
			.await;
		}
	}

	Ok(())
}

async fn import_patient_information(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<Option<Uuid>> {
	let Some(patient) = crate::xml::import_sections::d_patient::parse_d_patient(
		xml,
	)?
	.map(|patient| d_helpers::PatientImport {
		patient_initials: patient.patient_initials,
		patient_given_name: patient.patient_given_name,
		patient_family_name: patient.patient_family_name,
		patient_initials_null_flavor: patient.patient_initials_null_flavor,
		birth_date: patient.birth_date,
		birth_date_null_flavor: patient.birth_date_null_flavor,
		sex: patient.sex,
		sex_null_flavor: patient.sex_null_flavor,
		age_at_time_of_onset: patient.age_at_time_of_onset,
		age_at_time_of_onset_null_flavor: patient.age_at_time_of_onset_null_flavor,
		age_unit: patient.age_unit,
		gestation_period: patient.gestation_period,
		gestation_period_unit: patient.gestation_period_unit,
		age_group: patient.age_group,
		weight_kg: patient.weight_kg,
		height_cm: patient.height_cm,
		race_code: patient.race_code,
		race_code_null_flavor: patient.race_code_null_flavor,
		ethnicity_code: patient.ethnicity_code,
		ethnicity_code_null_flavor: patient.ethnicity_code_null_flavor,
		last_menstrual_period_date: patient.last_menstrual_period_date,
		last_menstrual_period_date_null_flavor: patient
			.last_menstrual_period_date_null_flavor,
		medical_history_text: patient.medical_history_text,
		concomitant_therapy: patient.concomitant_therapy,
	}) else {
		return Ok(None);
	};

	let existing_id: Option<Uuid> = mm
		.dbx()
		.fetch_optional(
			sqlx::query_as::<_, (Uuid,)>(
				"SELECT id FROM patient_information WHERE case_id = $1 LIMIT 1",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)?
		.map(|v| v.0);

	let patient_id = if let Some(id) = existing_id {
		id
	} else {
		PatientInformationBmc::create(
			ctx,
			mm,
			PatientInformationForCreate {
				case_id,
				patient_initials: patient.patient_initials.clone(),
				patient_given_name: patient.patient_given_name.clone(),
				patient_family_name: patient.patient_family_name.clone(),
				patient_initials_null_flavor: patient
					.patient_initials_null_flavor
					.clone(),
				birth_date: patient.birth_date,
				birth_date_null_flavor: patient.birth_date_null_flavor.clone(),
				age_at_time_of_onset: patient.age_at_time_of_onset,
				age_at_time_of_onset_null_flavor: patient
					.age_at_time_of_onset_null_flavor
					.clone(),
				age_unit: patient.age_unit.clone(),
				gestation_period: patient.gestation_period,
				gestation_period_unit: patient.gestation_period_unit.clone(),
				age_group: patient.age_group.clone(),
				weight_kg: patient.weight_kg,
				height_cm: patient.height_cm,
				sex: patient.sex.clone(),
				sex_null_flavor: patient.sex_null_flavor.clone(),
				race_code: patient.race_code.clone(),
				race_code_null_flavor: patient.race_code_null_flavor.clone(),
				ethnicity_code: patient.ethnicity_code.clone(),
				ethnicity_code_null_flavor: patient
					.ethnicity_code_null_flavor
					.clone(),
				last_menstrual_period_date: patient.last_menstrual_period_date,
				last_menstrual_period_date_null_flavor: patient
					.last_menstrual_period_date_null_flavor
					.clone(),
				medical_history_text: patient.medical_history_text.clone(),
				concomitant_therapy: patient.concomitant_therapy,
			},
		)
		.await?
	};

	if existing_id.is_some() {
		PatientInformationBmc::update(
			ctx,
			mm,
			patient_id,
			PatientInformationForUpdate {
				patient_initials: patient.patient_initials,
				patient_given_name: patient.patient_given_name,
				patient_family_name: patient.patient_family_name,
				patient_initials_null_flavor: patient.patient_initials_null_flavor,
				birth_date: patient.birth_date,
				birth_date_null_flavor: patient.birth_date_null_flavor,
				age_at_time_of_onset: patient.age_at_time_of_onset,
				age_at_time_of_onset_null_flavor: patient
					.age_at_time_of_onset_null_flavor,
				age_unit: patient.age_unit,
				gestation_period: patient.gestation_period,
				gestation_period_unit: patient.gestation_period_unit,
				age_group: patient.age_group,
				weight_kg: patient.weight_kg,
				height_cm: patient.height_cm,
				sex: patient.sex,
				sex_null_flavor: patient.sex_null_flavor,
				race_code: patient.race_code,
				race_code_null_flavor: patient.race_code_null_flavor,
				ethnicity_code: patient.ethnicity_code,
				ethnicity_code_null_flavor: patient.ethnicity_code_null_flavor,
				last_menstrual_period_date: patient.last_menstrual_period_date,
				last_menstrual_period_date_null_flavor: patient
					.last_menstrual_period_date_null_flavor,
				medical_history_text: patient.medical_history_text,
				concomitant_therapy: patient.concomitant_therapy,
			},
		)
		.await?;
	}

	Ok(Some(patient_id))
}
