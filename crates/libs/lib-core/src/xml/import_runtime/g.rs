use crate::ctx::Ctx;
use crate::model;
use crate::model::drug::{
	DosageInformationBmc, DosageInformationForCreate, DrugActiveSubstanceBmc,
	DrugActiveSubstanceForCreate, DrugIndicationBmc, DrugIndicationForCreate,
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
use crate::model::drug_reaction_assessment::{
	DrugReactionAssessmentBmc, DrugReactionAssessmentForCreate,
	RelatednessAssessmentBmc, RelatednessAssessmentForCreate,
	RelatednessAssessmentForUpdate,
};
use crate::model::store::set_full_context_dbx;
use crate::model::ModelManager;
use crate::xml::error::Error;
use crate::xml::import_runtime::helpers::g as g_helpers;
use crate::xml::import_runtime::shared::ImportIdMap;
use crate::xml::Result;
use sqlx::types::Uuid;
use std::collections::HashMap;

pub(crate) async fn import_section_g(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
	reaction_map: &ImportIdMap,
	product_presave_id: Option<Uuid>,
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
	product_presave_id: Option<Uuid>,
) -> Result<ImportIdMap> {
	let imports = crate::xml::import_sections::g_drug::parse_g_drugs(xml)?
		.into_iter()
		.map(|entry| g_helpers::DrugImport {
			xml_id: entry.xml_id,
			sequence_number: entry.sequence_number,
			medicinal_product: entry.medicinal_product,
			drug_characterization: entry.drug_characterization,
			mpid: entry.mpid,
			mpid_version: entry.mpid_version,
			phpid: entry.phpid,
			phpid_version: entry.phpid_version,
			investigational_product_blinded: entry.investigational_product_blinded,
			obtain_drug_country: entry.obtain_drug_country,
			drug_authorization_number: entry.drug_authorization_number,
			manufacturer_name: entry.manufacturer_name,
			manufacturer_country: entry.manufacturer_country,
			batch_lot_number: entry.batch_lot_number,
			cumulative_dose_first_reaction_value: entry
				.cumulative_dose_first_reaction_value,
			cumulative_dose_first_reaction_unit: entry
				.cumulative_dose_first_reaction_unit,
			gestation_period_exposure_value: entry.gestation_period_exposure_value,
			gestation_period_exposure_unit: entry.gestation_period_exposure_unit,
			dosage_text: entry.dosage_text,
			action_taken: entry.action_taken,
			fda_additional_info_coded: entry.fda_additional_info_coded,
			fda_specialized_product_category: entry.fda_specialized_product_category,
			fda_device_brand_name: entry.fda_device_brand_name,
			fda_common_device_name: entry.fda_common_device_name,
			fda_device_product_code: entry.fda_device_product_code,
			fda_device_manufacturer_name: entry.fda_device_manufacturer_name,
			fda_device_manufacturer_address: entry.fda_device_manufacturer_address,
			fda_device_manufacturer_city: entry.fda_device_manufacturer_city,
			fda_device_manufacturer_state: entry.fda_device_manufacturer_state,
			fda_device_manufacturer_country: entry.fda_device_manufacturer_country,
			fda_device_lot_number: entry.fda_device_lot_number,
			fda_operator_of_device: entry.fda_operator_of_device,
			substances: entry
				.substances
				.into_iter()
				.map(|sub| g_helpers::DrugSubstanceImport {
					substance_name: sub.substance_name,
					substance_termid: sub.substance_termid,
					substance_termid_version: sub.substance_termid_version,
					strength_value: sub.strength_value,
					strength_unit: sub.strength_unit,
				})
				.collect(),
			dosages: entry
				.dosages
				.into_iter()
				.map(|dose| g_helpers::DrugDosageImport {
					dosage_text: dose.dosage_text,
					frequency_value: dose.frequency_value,
					frequency_unit: dose.frequency_unit,
					number_of_units: dose.number_of_units,
					start_date: dose.start_date,
					start_date_null_flavor: dose.start_date_null_flavor,
					end_date: dose.end_date,
					end_date_null_flavor: dose.end_date_null_flavor,
					duration_value: dose.duration_value,
					duration_unit: dose.duration_unit,
					dose_value: dose.dose_value,
					dose_unit: dose.dose_unit,
					route: dose.route,
					route_termid: None,
					route_termid_version: dose.route_termid_version,
					dose_form: dose.dose_form,
					dose_form_termid: dose.dose_form_termid,
					dose_form_termid_version: dose.dose_form_termid_version,
					batch_lot: dose.batch_lot,
					parent_route_termid: dose.parent_route_termid,
					parent_route_termid_version: dose.parent_route_termid_version,
					parent_route: dose.parent_route,
				})
				.collect(),
			indications: entry
				.indications
				.into_iter()
				.map(|ind| g_helpers::DrugIndicationImport {
					text: ind.text,
					version: ind.version,
					code: ind.code,
				})
				.collect(),
			characteristics: entry
				.characteristics
				.into_iter()
				.map(|ch| g_helpers::DrugDeviceCharacteristicImport {
					code: ch.code,
					code_system: ch.code_system,
					code_display_name: ch.code_display_name,
					value_type: ch.value_type,
					value_value: ch.value_value,
					value_code: ch.value_code,
					value_code_system: ch.value_code_system,
					value_display_name: ch.value_display_name,
				})
				.collect(),
		})
		.collect::<Vec<_>>();
	let mut map = ImportIdMap::default();

	for (index, drug) in imports.into_iter().enumerate() {
		let (fda_specialized_product_category, fda_device_info_json) =
			g_helpers::import_fda_device_info(&drug, &drug.characteristics);
		let drug_additional_info_codes_json =
			g_helpers::build_drug_additional_info_codes_json(
				drug.fda_additional_info_coded.as_deref(),
			);
		let drug_id = DrugInformationBmc::create(
			ctx,
			mm,
			DrugInformationForCreate {
				case_id,
				source_product_presave_id: (index == 0)
					.then_some(product_presave_id)
					.flatten(),
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
					.then_some(product_presave_id)
					.flatten(),
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
				dosage_text: drug.dosage_text,
				action_taken: drug.action_taken,
				investigational_product_blinded: drug
					.investigational_product_blinded,
				mpid: drug.mpid,
				mpid_version: drug.mpid_version,
				// MFDS G product XML mapping is deferred until a verified local
				// canonical element path exists; do not alias base MPID values.
				mfds_mpid_version: None,
				mfds_mpid: None,
				phpid: drug.phpid,
				phpid_version: drug.phpid_version,
				obtain_drug_country: drug.obtain_drug_country,
				fda_additional_info_coded: drug.fda_additional_info_coded,
				drug_additional_info_codes_json,
				drug_additional_information: None,
				fda_specialized_product_category,
				fda_device_info_json,
				fda_other_characterization: None,
			},
		)
		.await?;

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
					// MFDS G substance XML mapping is deferred until a verified
					// local canonical element path exists; do not alias base terms.
					mfds_version: None,
					mfds_id: None,
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
					frequency_value: dose.frequency_value,
					frequency_unit: dose.frequency_unit,
					first_administration_date: dose.start_date,
					last_administration_date: dose.end_date,
					duration_value: dose.duration_value,
					duration_unit: dose.duration_unit,
					continuing: None,
					batch_lot_number: dose.batch_lot,
					batch_lot_number_null_flavor: None,
					dosage_text: dose.dosage_text,
					dose_form: dose.dose_form,
					dose_form_termid: dose.dose_form_termid,
					dose_form_termid_version: dose.dose_form_termid_version,
					route_of_administration: dose.route,
					route_termid: dose.route_termid,
					route_termid_version: dose.route_termid_version,
					parent_route: dose.parent_route,
					parent_route_termid: dose.parent_route_termid,
					parent_route_termid_version: dose.parent_route_termid_version,
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
					indication_text_null_flavor: None,
					indication_meddra_version: ind.version,
					indication_meddra_code: ind.code,
				},
			)
			.await?;
		}

		if !drug.characteristics.is_empty() {
			mm.dbx().begin_txn().await.map_err(model::Error::from)?;
			if let Err(err) = set_full_context_dbx(
				mm.dbx(),
				ctx.user_id(),
				ctx.organization_id(),
				ctx.role(),
			)
			.await
			{
				let _ = mm.dbx().rollback_txn().await;
				return Err(Error::Model(err));
			}
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
			mm.dbx().commit_txn().await.map_err(model::Error::from)?;
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
		let drug_id = drug_map.resolve(obs.drug_xml_id, Some(obs.drug_sequence));
		let reaction_id = reaction_map.resolve(obs.reaction_xml_id, None);
		let (Some(drug_id), Some(reaction_id)) = (drug_id, reaction_id) else {
			continue;
		};

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
		let drug_id = drug_map.resolve(rel.drug_xml_id, None);
		let reaction_id = reaction_map.resolve(rel.reaction_xml_id, None);
		let (Some(drug_id), Some(reaction_id)) = (drug_id, reaction_id) else {
			continue;
		};

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
			let _ = RelatednessAssessmentBmc::update(
				ctx,
				mm,
				id,
				RelatednessAssessmentForUpdate {
					source_of_assessment: rel.source_of_assessment,
					method_of_assessment: rel.method_of_assessment,
					result_of_assessment: rel.result_of_assessment,
					// KR.2 remains intentionally unsupported in XML import until a
					// canonical MFDS XML source path or fixture is available locally.
					result_of_assessment_kr2: None,
				},
			)
			.await;
		} else {
			let id = RelatednessAssessmentBmc::create(
				ctx,
				mm,
				RelatednessAssessmentForCreate {
					drug_reaction_assessment_id: assessment_id,
					sequence_number: *seq,
					source_of_assessment: rel.source_of_assessment.clone(),
					method_of_assessment: rel.method_of_assessment.clone(),
					result_of_assessment: rel.result_of_assessment.clone(),
					// KR.2 remains intentionally unsupported in XML import until a
					// canonical MFDS XML source path or fixture is available locally.
					result_of_assessment_kr2: None,
				},
			)
			.await?;
			let _ = RelatednessAssessmentBmc::update(
				ctx,
				mm,
				id,
				RelatednessAssessmentForUpdate {
					source_of_assessment: rel.source_of_assessment,
					method_of_assessment: rel.method_of_assessment,
					result_of_assessment: rel.result_of_assessment,
					// KR.2 remains intentionally unsupported in XML import until a
					// canonical MFDS XML source path or fixture is available locally.
					result_of_assessment_kr2: None,
				},
			)
			.await;
		}
	}

	Ok(())
}
