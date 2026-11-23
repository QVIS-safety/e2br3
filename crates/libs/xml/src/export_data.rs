use crate::error::Error;
use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model;
use lib_core::model::drug::{
	derive_drug_characteristics, DosageInformation, DrugActiveSubstance,
	DrugDeviceCharacteristic, DrugIndication, DrugInformation, DrugInformationBmc,
	FdaDeviceCode, FdaDeviceCodeBmc, FdaDeviceCodeFilter, FdaDeviceInformation,
	FdaDeviceInformationBmc, FdaDeviceInformationFilter,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessment, RelatednessAssessment,
};
use lib_core::model::ModelManager;
use modql::filter::{ListOptions, OpValValue, OpValsValue};
use serde_json::json;

pub(crate) struct DrugExportBundle {
	pub(crate) drugs: Vec<DrugInformation>,
	pub(crate) substances: Vec<DrugActiveSubstance>,
	pub(crate) dosages: Vec<DosageInformation>,
	pub(crate) indications: Vec<DrugIndication>,
	pub(crate) characteristics: Vec<DrugDeviceCharacteristic>,
	pub(crate) devices: Vec<FdaDeviceInformation>,
	pub(crate) device_codes: Vec<FdaDeviceCode>,
	pub(crate) assessments: Vec<DrugReactionAssessment>,
	pub(crate) relatedness: Vec<RelatednessAssessment>,
}

pub(crate) async fn load_drug_export_bundle(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: sqlx::types::Uuid,
) -> Result<DrugExportBundle> {
	let drugs = DrugInformationBmc::list_by_case(ctx, mm, case_id).await?;
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
						.is_some_and(|value| !value.trim().is_empty())
			})
			.cloned()
			.collect();
		raw_rows.sort_by_key(|row| row.sequence_number);
		if raw_rows.is_empty() {
			characteristics.extend(derive_drug_characteristics(drug)?);
		} else {
			let seen_codes: std::collections::HashSet<_> = raw_rows
				.iter()
				.filter_map(|row| row.code.as_deref().map(str::trim))
				.filter(|value| !value.is_empty())
				.map(str::to_string)
				.collect();
			characteristics.extend(raw_rows);
			characteristics.extend(
				derive_drug_characteristics(drug)?
					.into_iter()
					.filter(|row| {
						row.code
							.as_deref()
							.map(str::trim)
							.filter(|value| !value.is_empty())
							.is_none_or(|code| !seen_codes.contains(code))
					}),
			);
		}
	}

	let mut devices = if drug_ids.is_empty() {
		Vec::new()
	} else {
		FdaDeviceInformationBmc::list(
			ctx,
			mm,
			Some(vec![FdaDeviceInformationFilter {
				drug_id: Some(OpValsValue::from(vec![OpValValue::In(
					drug_ids.iter().map(|id| json!(id.to_string())).collect(),
				)])),
				..Default::default()
			}]),
			Some(ListOptions {
				limit: Some(5000),
				..Default::default()
			}),
		)
		.await?
	};
	devices.sort_by_key(|device| (device.drug_id, device.sequence_number));
	let device_ids: Vec<_> = devices.iter().map(|device| device.id).collect();
	let mut device_codes = if device_ids.is_empty() {
		Vec::new()
	} else {
		FdaDeviceCodeBmc::list(
			ctx,
			mm,
			Some(vec![FdaDeviceCodeFilter {
				device_id: Some(OpValsValue::from(vec![OpValValue::In(
					device_ids.iter().map(|id| json!(id.to_string())).collect(),
				)])),
				..Default::default()
			}]),
			Some(ListOptions {
				limit: Some(5000),
				..Default::default()
			}),
		)
		.await?
	};
	device_codes.sort_by_key(|code| {
		(code.device_id, code.element.clone(), code.sequence_number)
	});

	let assessments = if drug_ids.is_empty() {
		Vec::new()
	} else {
		mm.dbx()
			.fetch_all(
				sqlx::query_as::<_, DrugReactionAssessment>(
					"SELECT * FROM drug_reaction_assessments WHERE drug_id = ANY($1)",
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
		devices,
		device_codes,
		assessments,
		relatedness,
	})
}
