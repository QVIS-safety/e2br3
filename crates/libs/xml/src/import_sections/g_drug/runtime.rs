use super::helpers as g_helpers;
use crate::error::Error;
use crate::import_sections::shared::ImportIdMap;
use crate::Result;
use lib_core::ctx::Ctx;
use lib_core::model;
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationForCreate, DrugActiveSubstanceBmc,
	DrugActiveSubstanceForCreate, DrugIndicationBmc, DrugIndicationForCreate,
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
	FdaDeviceCodeBmc, FdaDeviceCodeForCreate, FdaDeviceInformationBmc,
	FdaDeviceInformationForCreate,
};
use lib_core::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	RelatednessAssessmentBmc, RelatednessAssessmentForCreate,
	RelatednessAssessmentForUpdate,
};
use lib_core::model::ModelManager;
use sqlx::types::Uuid;
use std::collections::HashMap;

pub(crate) async fn import_section_g(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	reaction_map: &ImportIdMap,
	product_presave_id: Uuid,
) -> Result<ImportIdMap> {
	let drug_map = import_drugs(ctx, mm, xml, case_id, product_presave_id).await?;
	import_drug_reaction_assessments(ctx, mm, xml, &drug_map, reaction_map).await?;
	Ok(drug_map)
}

async fn import_drugs(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	product_presave_id: Uuid,
) -> Result<ImportIdMap> {
	let imports = super::parse_g_drugs(xml)?;
	let mut map = ImportIdMap::default();

	for (index, drug) in imports.into_iter().enumerate() {
		let fda_specialized_product_category =
			g_helpers::import_fda_specialized_product_category(
				&drug,
				&drug.characteristics,
			);
		let drug_id = DrugInformationBmc::create(
			ctx,
			mm,
			DrugInformationForCreate {
				case_id,
				source_product_presave_id: (index == 0)
					.then_some(product_presave_id),
				source_product_id: None,
				sequence_number: drug.sequence_number,
				drug_characterization: drug.drug_characterization.clone(),
				medicinal_product: drug.medicinal_product.clone(),
				..Default::default()
			},
		)
		.await?;

		DrugInformationBmc::update(
			ctx,
			mm,
			drug_id,
			DrugInformationForUpdate {
				source_product_presave_id: (index == 0)
					.then_some(product_presave_id),
				source_product_id: None,
				medicinal_product: Some(drug.medicinal_product),
				drug_characterization: Some(drug.drug_characterization),
				// FDA.G.k.2.2.1 intentionally unsupported until a verified
				// canonical XML source path or fixture exists locally.
				drug_authorization_number: drug.drug_authorization_number,
				manufacturer_name: drug.manufacturer_name,
				manufacturer_country: drug.manufacturer_country,
				batch_lot_number: drug.batch_lot_number,
				cumulative_dose_first_reaction_value: drug
					.cumulative_dose_first_reaction_value,
				cumulative_dose_first_reaction_unit: drug
					.cumulative_dose_first_reaction_unit,
				gestation_period_exposure_value: drug
					.gestation_period_exposure_value,
				gestation_period_exposure_unit: drug.gestation_period_exposure_unit,
				action_taken: drug.action_taken,
				investigational_product_blinded: drug
					.investigational_product_blinded,
				mpid: drug.mpid,
				mpid_version: drug.mpid_version,
				mpid_source_code_system: drug.mpid_source_code_system,
				mpid_source_code_system_version: drug
					.mpid_source_code_system_version,
				mfds_mpid_version: drug.mfds_mpid_version,
				mfds_mpid: drug.mfds_mpid,
				phpid: drug.phpid,
				phpid_version: drug.phpid_version,
				obtain_drug_country: drug.obtain_drug_country,
				fda_additional_info_coded: drug.fda_additional_info_coded,
				fda_additional_info_coded_null_flavor: drug
					.fda_additional_info_coded_null_flavor,
				drug_additional_info_codes_json: None,
				drug_additional_information: drug.drug_additional_information,
				fda_specialized_product_category,
				fda_other_characterization: drug.fda_other_characterization,
			},
		)
		.await?;

		for (device_index, device) in drug.devices.into_iter().enumerate() {
			let device_id = FdaDeviceInformationBmc::create(
				ctx,
				mm,
				FdaDeviceInformationForCreate {
					drug_id,
					sequence_number: (device_index + 1) as i32,
					malfunction: device.malfunction,
					device_brand_name: device.device_brand_name,
					device_brand_name_null_flavor: device
						.device_brand_name_null_flavor,
					common_device_name: device.common_device_name,
					common_device_name_null_flavor: device
						.common_device_name_null_flavor,
					device_product_code: device.device_product_code,
					manufacturer_name: device.manufacturer_name,
					manufacturer_address: device.manufacturer_address,
					manufacturer_city: device.manufacturer_city,
					manufacturer_state: device.manufacturer_state,
					manufacturer_country: device.manufacturer_country,
					device_usage: device.device_usage,
					device_lot_number: device.device_lot_number,
					operator_of_device: device.operator_of_device,
				},
			)
			.await?;
			let mut sequences: HashMap<&'static str, i32> = HashMap::new();
			for code in device.codes {
				let sequence = sequences.entry(code.element).or_insert(0);
				*sequence += 1;
				FdaDeviceCodeBmc::create(
					ctx,
					mm,
					FdaDeviceCodeForCreate {
						device_id,
						element: code.element.to_string(),
						sequence_number: *sequence,
						value_code: code.value_code,
					},
				)
				.await?;
			}
		}

		for (sidx, sub) in drug.substances.into_iter().enumerate() {
			let _ = DrugActiveSubstanceBmc::create(
				ctx,
				mm,
				DrugActiveSubstanceForCreate {
					drug_id,
					sequence_number: (sidx + 1) as i32,
					substance_name: sub.substance_name,
					substance_termid: sub.substance_termid,
					substance_termid_version: sub.substance_termid_version,
					substance_termid_code_system: sub.substance_termid_code_system,
					mfds_version: sub.mfds_version,
					mfds_id: sub.mfds_id,
					strength_value: sub.strength_value,
					strength_unit: sub.strength_unit,
				},
			)
			.await?;
		}

		for (didx, dose) in drug.dosages.into_iter().enumerate() {
			let _ = DosageInformationBmc::create(
				ctx,
				mm,
				DosageInformationForCreate {
					drug_id,
					sequence_number: (didx + 1) as i32,
					dose_value: dose.dose_value,
					dose_unit: dose.dose_unit,
					number_of_units: dose.number_of_units,
					frequency_unit: dose.frequency_unit,
					first_administration_date: dose.start_date,
					first_administration_date_raw: dose.start_date_raw,
					last_administration_date: dose.end_date,
					last_administration_date_raw: dose.end_date_raw,
					duration_value: dose.duration_value,
					duration_unit: dose.duration_unit,
					continuing: None,
					batch_lot_number: dose.batch_lot,
					batch_lot_number_null_flavor: dose.batch_lot_null_flavor,
					dosage_text: dose.dosage_text,
					dose_form: dose.dose_form,
					dose_form_null_flavor: dose.dose_form_null_flavor,
					dose_form_termid: dose.dose_form_termid,
					dose_form_termid_version: dose.dose_form_termid_version,
					route_of_administration: dose.route,
					route_of_administration_null_flavor: dose.route_null_flavor,
					route_termid: dose.route_termid,
					route_termid_version: dose.route_termid_version,
					route_termid_code_system: dose.route_termid_code_system,
					parent_route: dose.parent_route,
					parent_route_null_flavor: dose.parent_route_null_flavor,
					parent_route_termid: dose.parent_route_termid,
					parent_route_termid_version: dose.parent_route_termid_version,
					parent_route_termid_code_system: dose
						.parent_route_termid_code_system,
					first_administration_date_null_flavor: dose
						.start_date_null_flavor,
					last_administration_date_null_flavor: dose.end_date_null_flavor,
				},
			)
			.await?;
		}

		for (iidx, ind) in drug.indications.into_iter().enumerate() {
			let _ = DrugIndicationBmc::create(
				ctx,
				mm,
				DrugIndicationForCreate {
					drug_id,
					sequence_number: (iidx + 1) as i32,
					indication_text: ind.text,
					indication_text_null_flavor: ind.text_null_flavor,
					indication_meddra_version: ind.version,
					indication_meddra_code: ind.code,
				},
			)
			.await?;
		}

		if !drug.characteristics.is_empty() {
			for (cidx, ch) in drug.characteristics.into_iter().enumerate() {
				mm.dbx()
					.execute(
						sqlx::query(
							"INSERT INTO drug_device_characteristics (
								drug_id,
								sequence_number,
								code,
								code_system,
								code_display_name,
								value_type,
								value_value,
								value_code,
								value_code_system,
								value_display_name,
								created_at,
								updated_at,
								created_by
							) VALUES (
								$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,NOW(),NOW(),$11
							)",
						)
						.bind(drug_id)
						.bind((cidx + 1) as i32)
						.bind(ch.code)
						.bind(ch.code_system)
						.bind(ch.code_display_name)
						.bind(ch.value_type)
						.bind(ch.value_value)
						.bind(ch.value_code)
						.bind(ch.value_code_system)
						.bind(ch.value_display_name)
						.bind(ctx.user_id()),
					)
					.await
					.map_err(model::Error::from)?;
			}
		}

		if let Some(xml_id) = drug.xml_id {
			map.insert_xml_id(xml_id, drug_id);
		}
		map.push_sequence(drug_id);
	}

	Ok(map)
}

async fn import_drug_reaction_assessments(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	drug_map: &ImportIdMap,
	reaction_map: &ImportIdMap,
) -> Result<()> {
	let observations = g_helpers::parse_drug_observations(xml)?;
	let mut assessment_map: HashMap<(Uuid, Uuid), Uuid> = HashMap::new();
	for obs in &observations {
		let drug_id = drug_map
			.resolve(obs.drug_xml_id.clone(), Some(obs.drug_sequence))
			.ok_or_else(|| Error::InvalidXml {
				message: format!(
					"ICH.G.k.9.i: unresolved drug reference {:?}",
					obs.drug_xml_id
				),
				line: None,
				column: None,
			})?;
		let reaction_id = reaction_map
			.resolve(obs.reaction_xml_id.clone(), None)
			.ok_or_else(|| Error::InvalidXml {
				message: format!(
					"ICH.G.k.9.i: unresolved reaction reference {:?}",
					obs.reaction_xml_id
				),
				line: None,
				column: None,
			})?;

		let key = (drug_id, reaction_id);
		let _assessment_id = if let Some(id) = assessment_map.get(&key) {
			*id
		} else if let Some(existing) =
			DrugReactionAssessmentBmc::get_by_drug_and_reaction(
				ctx,
				mm,
				drug_id,
				reaction_id,
			)
			.await?
		{
			assessment_map.insert(key, existing.id);
			existing.id
		} else {
			let id = DrugReactionAssessmentBmc::create(
				ctx,
				mm,
				DrugReactionAssessmentForCreate {
					drug_id,
					reaction_id,
					administration_start_interval_value: obs
						.administration_start_interval_value,
					administration_start_interval_unit: obs
						.administration_start_interval_unit
						.clone(),
					last_dose_interval_value: obs.last_dose_interval_value,
					last_dose_interval_unit: obs.last_dose_interval_unit.clone(),
					recurrence_action: obs.rechallenge_action.clone(),
					reaction_recurred: obs.reaction_recurred.clone(),
					dechallenge_result: None,
					expectedness: None,
				},
			)
			.await?;
			assessment_map.insert(key, id);
			id
		};
	}

	let relatedness = g_helpers::parse_relatedness_assessments(xml)?;
	let mut seq_map: HashMap<(Uuid, Uuid), i32> = HashMap::new();
	for rel in relatedness {
		let drug_id =
			drug_map
				.resolve(rel.drug_xml_id.clone(), None)
				.ok_or_else(|| Error::InvalidXml {
					message: format!(
						"ICH.G.k.9.i.2: unresolved drug reference {:?}",
						rel.drug_xml_id
					),
					line: None,
					column: None,
				})?;
		let reaction_id = reaction_map
			.resolve(rel.reaction_xml_id.clone(), None)
			.ok_or_else(|| Error::InvalidXml {
				message: format!(
					"ICH.G.k.9.i.2: unresolved reaction reference {:?}",
					rel.reaction_xml_id
				),
				line: None,
				column: None,
			})?;

		let key = (drug_id, reaction_id);
		let assessment_id = if let Some(id) = assessment_map.get(&key) {
			*id
		} else if let Some(existing) =
			DrugReactionAssessmentBmc::get_by_drug_and_reaction(
				ctx,
				mm,
				drug_id,
				reaction_id,
			)
			.await?
		{
			assessment_map.insert(key, existing.id);
			existing.id
		} else {
			let id = DrugReactionAssessmentBmc::create(
				ctx,
				mm,
				DrugReactionAssessmentForCreate {
					drug_id,
					reaction_id,
					administration_start_interval_value: None,
					administration_start_interval_unit: None,
					last_dose_interval_value: None,
					last_dose_interval_unit: None,
					recurrence_action: None,
					reaction_recurred: None,
					dechallenge_result: None,
					expectedness: None,
				},
			)
			.await?;
			assessment_map.insert(key, id);
			id
		};

		let seq = seq_map
			.entry((drug_id, reaction_id))
			.and_modify(|v| *v += 1)
			.or_insert(1);

		let existing: Option<Uuid> = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, (Uuid,)>(
					"SELECT id FROM relatedness_assessments WHERE drug_reaction_assessment_id = $1 AND sequence_number = $2 LIMIT 1",
				)
				.bind(assessment_id)
				.bind(*seq),
			)
			.await
			.map_err(model::Error::from)?
			.map(|v| v.0);

		if let Some(id) = existing {
			RelatednessAssessmentBmc::update(
				ctx,
				mm,
				id,
				RelatednessAssessmentForUpdate {
					source_of_assessment: rel.source_of_assessment,
					method_of_assessment: rel.method_of_assessment,
					method_of_assessment_kr1: rel.method_of_assessment_kr1,
					result_of_assessment: rel.result_of_assessment,
					result_of_assessment_kr1: rel.result_of_assessment_kr1,
					result_of_assessment_kr1_null_flavor: rel
						.result_of_assessment_kr1_null_flavor,
					result_of_assessment_kr2: rel.result_of_assessment_kr2,
				},
			)
			.await?;
		} else {
			let id = RelatednessAssessmentBmc::create(
				ctx,
				mm,
				RelatednessAssessmentForCreate {
					drug_reaction_assessment_id: assessment_id,
					sequence_number: *seq,
					source_of_assessment: rel.source_of_assessment.clone(),
					method_of_assessment: rel.method_of_assessment.clone(),
					method_of_assessment_kr1: rel.method_of_assessment_kr1.clone(),
					result_of_assessment: rel.result_of_assessment.clone(),
					result_of_assessment_kr1: rel.result_of_assessment_kr1.clone(),
					result_of_assessment_kr1_null_flavor: rel
						.result_of_assessment_kr1_null_flavor
						.clone(),
					result_of_assessment_kr2: rel.result_of_assessment_kr2.clone(),
				},
			)
			.await?;
			RelatednessAssessmentBmc::update(
				ctx,
				mm,
				id,
				RelatednessAssessmentForUpdate {
					source_of_assessment: rel.source_of_assessment,
					method_of_assessment: rel.method_of_assessment,
					method_of_assessment_kr1: rel.method_of_assessment_kr1,
					result_of_assessment: rel.result_of_assessment,
					result_of_assessment_kr1: rel.result_of_assessment_kr1,
					result_of_assessment_kr1_null_flavor: rel
						.result_of_assessment_kr1_null_flavor,
					result_of_assessment_kr2: rel.result_of_assessment_kr2,
				},
			)
			.await?;
		}
	}

	Ok(())
}
