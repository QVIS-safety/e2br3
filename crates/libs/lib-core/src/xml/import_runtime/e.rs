use crate::ctx::Ctx;
use crate::model::reaction::{ReactionBmc, ReactionForCreate};
use crate::model::ModelManager;
use crate::xml::import_runtime::shared::ImportIdMap;
use crate::xml::Result;
use sqlx::types::Uuid;

pub(crate) async fn import_section_e(
	ctx: &Ctx,
	mm: &ModelManager,
	xml: &[u8],
	case_id: Uuid,
) -> Result<ImportIdMap> {
	let reactions = crate::xml::import_sections::e_reaction::parse_e_reactions(xml)?;
	let mut map = ImportIdMap::default();

	for (idx, reaction) in reactions.into_iter().enumerate() {
		let rec_id = ReactionBmc::create(
			ctx,
			mm,
			ReactionForCreate {
				case_id,
				sequence_number: (idx + 1) as i32,
				primary_source_reaction: reaction.primary_source_reaction.clone(),
				primary_source_reaction_translation: reaction
					.primary_source_reaction_translation
					.clone(),
				reaction_language: reaction.reaction_language.clone(),
				reaction_meddra_code: reaction.reaction_meddra_code.clone(),
				reaction_meddra_version: reaction.reaction_meddra_version.clone(),
				term_highlighted: reaction.term_highlighted,
				serious: reaction.serious,
				criteria_death: reaction.criteria_death,
				criteria_death_null_flavor: reaction
					.criteria_death_null_flavor
					.clone(),
				criteria_life_threatening: reaction.criteria_life_threatening,
				criteria_life_threatening_null_flavor: reaction
					.criteria_life_threatening_null_flavor
					.clone(),
				criteria_hospitalization: reaction.criteria_hospitalization,
				criteria_hospitalization_null_flavor: reaction
					.criteria_hospitalization_null_flavor
					.clone(),
				criteria_disabling: reaction.criteria_disabling,
				criteria_disabling_null_flavor: reaction
					.criteria_disabling_null_flavor
					.clone(),
				criteria_congenital_anomaly: reaction.criteria_congenital_anomaly,
				criteria_congenital_anomaly_null_flavor: reaction
					.criteria_congenital_anomaly_null_flavor
					.clone(),
				criteria_other_medically_important: reaction
					.criteria_other_medically_important,
				criteria_other_medically_important_null_flavor: reaction
					.criteria_other_medically_important_null_flavor
					.clone(),
				required_intervention: reaction.required_intervention.clone(),
				required_intervention_null_flavor: None,
				included_in_ema_ime_list: reaction.included_in_ema_ime_list,
				expectedness: reaction.expectedness.clone(),
				severity: reaction.severity.clone(),
				mfds_device_ae_classification: reaction
					.mfds_device_ae_classification
					.clone(),
				mfds_device_ae_outcome: reaction.mfds_device_ae_outcome.clone(),
				mfds_device_cause_medical_device: reaction
					.mfds_device_cause_medical_device,
				mfds_device_cause_procedure_issue: reaction
					.mfds_device_cause_procedure_issue,
				mfds_device_cause_patient_condition: reaction
					.mfds_device_cause_patient_condition,
				mfds_device_cause_unable_to_assess: reaction
					.mfds_device_cause_unable_to_assess,
				mfds_device_cause_other: reaction.mfds_device_cause_other.clone(),
				mfds_device_action_reason: reaction
					.mfds_device_action_reason
					.clone(),
				mfds_device_action_recall: reaction.mfds_device_action_recall,
				mfds_device_action_repair: reaction.mfds_device_action_repair,
				mfds_device_action_inspection: reaction
					.mfds_device_action_inspection,
				mfds_device_action_replacement: reaction
					.mfds_device_action_replacement,
				mfds_device_action_improvement: reaction
					.mfds_device_action_improvement,
				mfds_device_action_monitoring: reaction
					.mfds_device_action_monitoring,
				mfds_device_action_notification: reaction
					.mfds_device_action_notification,
				mfds_device_action_label_change: reaction
					.mfds_device_action_label_change,
				mfds_device_action_other: reaction.mfds_device_action_other.clone(),
				start_date: reaction.start_date,
				start_date_null_flavor: reaction.start_date_null_flavor.clone(),
				end_date: reaction.end_date,
				end_date_null_flavor: reaction.end_date_null_flavor.clone(),
				duration_value: reaction.duration_value,
				duration_unit: reaction.duration_unit.clone(),
				outcome: reaction.outcome.clone(),
				medical_confirmation: reaction.medical_confirmation,
				country_code: reaction.country_code.clone(),
				deleted: Some(false),
			},
		)
		.await?;
		if let Some(xml_id) = reaction.xml_id {
			map.insert_xml_id(xml_id, rec_id);
		}
		map.push_sequence(rec_id);
	}

	Ok(map)
}
