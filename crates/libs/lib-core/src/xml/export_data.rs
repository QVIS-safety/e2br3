use crate::model;
use crate::model::drug::{
	derive_fda_device_characteristics, DosageInformation, DrugActiveSubstance,
	DrugDeviceCharacteristic, DrugIndication, DrugInformation,
};
use crate::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::Result;

pub(crate) struct DrugExportBundle {
	pub(crate) drugs: Vec<DrugInformation>,
	pub(crate) substances: Vec<DrugActiveSubstance>,
	pub(crate) dosages: Vec<DosageInformation>,
	pub(crate) indications: Vec<DrugIndication>,
	pub(crate) characteristics: Vec<DrugDeviceCharacteristic>,
	pub(crate) assessments: Vec<DrugReactionAssessment>,
	pub(crate) relatedness: Vec<RelatednessAssessment>,
}

pub(crate) async fn load_drug_export_bundle(
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<DrugExportBundle> {
	let drugs = mm
		.dbx()
		.fetch_all(
			sqlx::query_as::<_, DrugInformation>(
				"SELECT * FROM drug_information WHERE case_id = $1 AND deleted = false ORDER BY sequence_number",
			)
			.bind(case_id),
		)
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)?;
	let drug_ids: Vec<_> = drugs.iter().map(|d| d.id).collect();

	let substances = if drug_ids.is_empty() {
		Vec::new()
	} else {
		mm.dbx()
			.fetch_all(
				sqlx::query_as::<_, DrugActiveSubstance>(
					"SELECT * FROM drug_active_substances WHERE drug_id = ANY($1) AND deleted = false ORDER BY sequence_number",
				)
				.bind(&drug_ids),
			)
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?
	};

	let dosages = if drug_ids.is_empty() {
		Vec::new()
	} else {
		mm.dbx()
			.fetch_all(
				sqlx::query_as::<_, DosageInformation>(
					"SELECT * FROM dosage_information WHERE drug_id = ANY($1) AND deleted = false ORDER BY sequence_number",
				)
				.bind(&drug_ids),
			)
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?
	};

	let indications = if drug_ids.is_empty() {
		Vec::new()
	} else {
		mm.dbx()
			.fetch_all(
				sqlx::query_as::<_, DrugIndication>(
					"SELECT * FROM drug_indications WHERE drug_id = ANY($1) AND deleted = false ORDER BY sequence_number",
				)
				.bind(&drug_ids),
			)
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?
	};

	let raw_characteristics = if drug_ids.is_empty() {
		Vec::new()
	} else {
		mm.dbx()
			.fetch_all(
				sqlx::query_as::<_, DrugDeviceCharacteristic>(
					"SELECT * FROM drug_device_characteristics WHERE drug_id = ANY($1) AND deleted = false ORDER BY sequence_number",
				)
				.bind(&drug_ids),
			)
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?
	};
	let mut characteristics = Vec::new();
	for drug in &drugs {
		let mut raw_rows: Vec<_> = raw_characteristics
			.iter()
			.filter(|row| {
				row.drug_id == drug.id
					&& row
						.code
						.as_deref()
						.map(|value| !value.trim().is_empty())
						.unwrap_or(false)
			})
			.cloned()
			.collect();
		raw_rows.sort_by_key(|row| row.sequence_number);
		if raw_rows.is_empty() {
			characteristics.extend(derive_fda_device_characteristics(drug));
		} else {
			let seen_codes: std::collections::HashSet<_> = raw_rows
				.iter()
				.filter_map(|row| row.code.as_deref().map(str::trim))
				.filter(|value| !value.is_empty())
				.map(str::to_string)
				.collect();
			characteristics.extend(raw_rows);
			characteristics.extend(
				derive_fda_device_characteristics(drug)
					.into_iter()
					.filter(|row| {
						row.code
							.as_deref()
							.map(str::trim)
							.filter(|value| !value.is_empty())
							.map(|code| !seen_codes.contains(code))
							.unwrap_or(true)
					}),
			);
		}
	}

	let assessments = if drug_ids.is_empty() {
		Vec::new()
	} else {
		mm.dbx()
			.fetch_all(
				sqlx::query_as::<_, DrugReactionAssessment>(
					"SELECT * FROM drug_reaction_assessments WHERE drug_id = ANY($1) AND deleted = false",
				)
				.bind(&drug_ids),
			)
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?
	};
	let assessment_ids: Vec<_> = assessments.iter().map(|a| a.id).collect();
	let relatedness = if assessment_ids.is_empty() {
		Vec::new()
	} else {
		mm.dbx()
			.fetch_all(
				sqlx::query_as::<_, RelatednessAssessment>(
					"SELECT * FROM relatedness_assessments WHERE drug_reaction_assessment_id = ANY($1) AND deleted = false ORDER BY sequence_number",
				)
				.bind(&assessment_ids),
			)
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?
	};

	Ok(DrugExportBundle {
		drugs,
		substances,
		dosages,
		indications,
		characteristics,
		assessments,
		relatedness,
	})
}
