// Section E - Reaction/Event

use crate::ctx::Ctx;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, OpValsBool, OpValsValue};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- Reaction

fn deserialize_term_highlighted<'de, D>(
	deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
	D: Deserializer<'de>,
{
	let value = Option::<String>::deserialize(deserializer)?;
	match value.as_deref() {
		None | Some("1" | "2" | "3" | "4") => Ok(value),
		Some(_) => Err(serde::de::Error::custom(
			"term_highlighted must be one of 1, 2, 3, 4",
		)),
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct Reaction {
	pub id: Uuid,
	pub case_id: Uuid,
	pub sequence_number: i32,

	// E.i.1.1 - Reaction as reported
	pub primary_source_reaction: Option<String>,
	// E.i.1.2 - Reaction/Event as reported by primary source for translation
	pub primary_source_reaction_translation: Option<String>,
	pub reaction_language: Option<String>,

	// E.i.2.1 - MedDRA coding
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,

	// E.i.3 - Term Highlighted by Reporter
	pub term_highlighted: Option<String>,

	// E.i.3.1 - Seriousness (MANDATORY if serious)
	pub serious: Option<bool>,

	// E.i.3.2 - Seriousness Criteria
	pub criteria_death: Option<bool>,
	pub criteria_death_null_flavor: Option<String>,
	pub criteria_life_threatening: Option<bool>,
	pub criteria_life_threatening_null_flavor: Option<String>,
	pub criteria_hospitalization: Option<bool>,
	pub criteria_hospitalization_null_flavor: Option<String>,
	pub criteria_disabling: Option<bool>,
	pub criteria_disabling_null_flavor: Option<String>,
	pub criteria_congenital_anomaly: Option<bool>,
	pub criteria_congenital_anomaly_null_flavor: Option<String>,
	pub criteria_other_medically_important: Option<bool>,
	pub criteria_other_medically_important_null_flavor: Option<String>,
	// FDA.E.i.3.2h - Required Intervention (FDA)
	pub required_intervention: Option<bool>,
	pub required_intervention_null_flavor: Option<String>,

	pub expectedness: Option<String>,
	pub severity: Option<String>,
	pub mfds_device_ae_classification: Option<String>,
	pub mfds_device_ae_outcome: Option<String>,
	pub mfds_device_cause_medical_device: Option<bool>,
	pub mfds_device_cause_procedure_issue: Option<bool>,
	pub mfds_device_cause_patient_condition: Option<bool>,
	pub mfds_device_cause_unable_to_assess: Option<bool>,
	pub mfds_device_cause_other: Option<String>,
	pub mfds_device_action_reason: Option<String>,
	pub mfds_device_action_recall: Option<bool>,
	pub mfds_device_action_repair: Option<bool>,
	pub mfds_device_action_inspection: Option<bool>,
	pub mfds_device_action_replacement: Option<bool>,
	pub mfds_device_action_improvement: Option<bool>,
	pub mfds_device_action_monitoring: Option<bool>,
	pub mfds_device_action_notification: Option<bool>,
	pub mfds_device_action_label_change: Option<bool>,
	pub mfds_device_action_other: Option<String>,

	// E.i.4-6 - Timing
	pub start_date: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub end_date: Option<String>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<String>,
	pub duration_unit: Option<String>,

	// E.i.7 - Outcome
	pub outcome: Option<String>,

	// E.i.8 - Medical Confirmation
	pub medical_confirmation: Option<bool>,

	// E.i.9 - Country
	pub country_code: Option<String>,

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ReactionForCreate {
	pub case_id: Uuid,
	pub sequence_number: i32,
	pub primary_source_reaction: Option<String>,
	pub primary_source_reaction_translation: Option<String>,
	pub reaction_language: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub reaction_meddra_version: Option<String>,
	#[serde(default, deserialize_with = "deserialize_term_highlighted")]
	pub term_highlighted: Option<String>,
	pub serious: Option<bool>,
	pub criteria_death: Option<bool>,
	pub criteria_death_null_flavor: Option<String>,
	pub criteria_life_threatening: Option<bool>,
	pub criteria_life_threatening_null_flavor: Option<String>,
	pub criteria_hospitalization: Option<bool>,
	pub criteria_hospitalization_null_flavor: Option<String>,
	pub criteria_disabling: Option<bool>,
	pub criteria_disabling_null_flavor: Option<String>,
	pub criteria_congenital_anomaly: Option<bool>,
	pub criteria_congenital_anomaly_null_flavor: Option<String>,
	pub criteria_other_medically_important: Option<bool>,
	pub criteria_other_medically_important_null_flavor: Option<String>,
	pub required_intervention: Option<bool>,
	pub required_intervention_null_flavor: Option<String>,
	pub expectedness: Option<String>,
	pub severity: Option<String>,
	pub mfds_device_ae_classification: Option<String>,
	pub mfds_device_ae_outcome: Option<String>,
	pub mfds_device_cause_medical_device: Option<bool>,
	pub mfds_device_cause_procedure_issue: Option<bool>,
	pub mfds_device_cause_patient_condition: Option<bool>,
	pub mfds_device_cause_unable_to_assess: Option<bool>,
	pub mfds_device_cause_other: Option<String>,
	pub mfds_device_action_reason: Option<String>,
	pub mfds_device_action_recall: Option<bool>,
	pub mfds_device_action_repair: Option<bool>,
	pub mfds_device_action_inspection: Option<bool>,
	pub mfds_device_action_replacement: Option<bool>,
	pub mfds_device_action_improvement: Option<bool>,
	pub mfds_device_action_monitoring: Option<bool>,
	pub mfds_device_action_notification: Option<bool>,
	pub mfds_device_action_label_change: Option<bool>,
	pub mfds_device_action_other: Option<String>,
	pub start_date: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub end_date: Option<String>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<String>,
	pub duration_unit: Option<String>,
	pub outcome: Option<String>,
	pub medical_confirmation: Option<bool>,
	pub country_code: Option<String>,
	pub deleted: Option<bool>,
}

#[derive(Fields, Deserialize)]
pub struct ReactionForUpdate {
	pub primary_source_reaction: Option<String>,
	pub primary_source_reaction_translation: Option<String>,
	pub reaction_language: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub reaction_meddra_version: Option<String>,
	#[serde(default, deserialize_with = "deserialize_term_highlighted")]
	pub term_highlighted: Option<String>,
	pub serious: Option<bool>,
	pub criteria_death: Option<bool>,
	pub criteria_death_null_flavor: Option<String>,
	pub criteria_life_threatening: Option<bool>,
	pub criteria_life_threatening_null_flavor: Option<String>,
	pub criteria_hospitalization: Option<bool>,
	pub criteria_hospitalization_null_flavor: Option<String>,
	pub criteria_disabling: Option<bool>,
	pub criteria_disabling_null_flavor: Option<String>,
	pub criteria_congenital_anomaly: Option<bool>,
	pub criteria_congenital_anomaly_null_flavor: Option<String>,
	pub criteria_other_medically_important: Option<bool>,
	pub criteria_other_medically_important_null_flavor: Option<String>,
	pub required_intervention: Option<bool>,
	pub required_intervention_null_flavor: Option<String>,
	pub expectedness: Option<String>,
	pub severity: Option<String>,
	pub mfds_device_ae_classification: Option<String>,
	pub mfds_device_ae_outcome: Option<String>,
	pub mfds_device_cause_medical_device: Option<bool>,
	pub mfds_device_cause_procedure_issue: Option<bool>,
	pub mfds_device_cause_patient_condition: Option<bool>,
	pub mfds_device_cause_unable_to_assess: Option<bool>,
	pub mfds_device_cause_other: Option<String>,
	pub mfds_device_action_reason: Option<String>,
	pub mfds_device_action_recall: Option<bool>,
	pub mfds_device_action_repair: Option<bool>,
	pub mfds_device_action_inspection: Option<bool>,
	pub mfds_device_action_replacement: Option<bool>,
	pub mfds_device_action_improvement: Option<bool>,
	pub mfds_device_action_monitoring: Option<bool>,
	pub mfds_device_action_notification: Option<bool>,
	pub mfds_device_action_label_change: Option<bool>,
	pub mfds_device_action_other: Option<String>,
	pub start_date: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub end_date: Option<String>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<String>,
	pub duration_unit: Option<String>,
	pub outcome: Option<String>,
	pub medical_confirmation: Option<bool>,
	pub country_code: Option<String>,
	pub deleted: Option<bool>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct ReactionFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
	pub serious: Option<OpValsBool>,
}

// -- BMC

pub struct ReactionBmc;
impl DbBmc for ReactionBmc {
	const TABLE: &'static str = "reactions";
}

impl ReactionBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		reaction_c: ReactionForCreate,
	) -> Result<Uuid> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"INSERT INTO {} (
			 case_id, sequence_number, primary_source_reaction, primary_source_reaction_translation,
			 reaction_language, reaction_meddra_code, reaction_meddra_version, term_highlighted,
			 serious, criteria_death, criteria_death_null_flavor, criteria_life_threatening,
			 criteria_life_threatening_null_flavor, criteria_hospitalization,
			 criteria_hospitalization_null_flavor, criteria_disabling,
			 criteria_disabling_null_flavor, criteria_congenital_anomaly,
			 criteria_congenital_anomaly_null_flavor, criteria_other_medically_important,
			 criteria_other_medically_important_null_flavor, required_intervention,
			 expectedness, severity, mfds_device_ae_classification,
			 mfds_device_ae_outcome, mfds_device_cause_medical_device,
			 mfds_device_cause_procedure_issue, mfds_device_cause_patient_condition,
			 mfds_device_cause_unable_to_assess, mfds_device_cause_other,
			 mfds_device_action_reason, mfds_device_action_recall, mfds_device_action_repair,
			 mfds_device_action_inspection, mfds_device_action_replacement,
			 mfds_device_action_improvement, mfds_device_action_monitoring,
			 mfds_device_action_notification, mfds_device_action_label_change,
			 mfds_device_action_other, start_date,
			 start_date_null_flavor, end_date, end_date_null_flavor, duration_value, duration_unit,
			 outcome, medical_confirmation, country_code, required_intervention_null_flavor, created_at, updated_at, created_by
			)
			 VALUES (
			 $1, $2, $3, $4,
			 $5, $6, $7, $8,
			 $9, $10, $11, $12,
			 $13, $14,
			 $15, $16,
			 $17, $18,
			 $19, $20,
			 $21, $22, $23, $24,
			 $25, $26, $27, $28,
			 $29, $30, $31, $32,
			 $33, $34, $35,
			 $36, $37, $38,
			 $39, $40, $41,
			 $42, $43, $44, $45, $46,
			 $47, $48, $49, $50, $51, now(), now(), $52
			)
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(reaction_c.case_id)
					.bind(reaction_c.sequence_number)
					.bind(reaction_c.primary_source_reaction)
					.bind(reaction_c.primary_source_reaction_translation)
					.bind(reaction_c.reaction_language)
					.bind(reaction_c.reaction_meddra_code)
					.bind(reaction_c.reaction_meddra_version)
					.bind(reaction_c.term_highlighted)
					.bind(reaction_c.serious)
					.bind(reaction_c.criteria_death)
					.bind(reaction_c.criteria_death_null_flavor)
					.bind(reaction_c.criteria_life_threatening)
					.bind(reaction_c.criteria_life_threatening_null_flavor)
					.bind(reaction_c.criteria_hospitalization)
					.bind(reaction_c.criteria_hospitalization_null_flavor)
					.bind(reaction_c.criteria_disabling)
					.bind(reaction_c.criteria_disabling_null_flavor)
					.bind(reaction_c.criteria_congenital_anomaly)
					.bind(reaction_c.criteria_congenital_anomaly_null_flavor)
					.bind(reaction_c.criteria_other_medically_important)
					.bind(reaction_c.criteria_other_medically_important_null_flavor)
					.bind(reaction_c.required_intervention)
					.bind(reaction_c.expectedness)
					.bind(reaction_c.severity)
					.bind(reaction_c.mfds_device_ae_classification)
					.bind(reaction_c.mfds_device_ae_outcome)
					.bind(reaction_c.mfds_device_cause_medical_device)
					.bind(reaction_c.mfds_device_cause_procedure_issue)
					.bind(reaction_c.mfds_device_cause_patient_condition)
					.bind(reaction_c.mfds_device_cause_unable_to_assess)
					.bind(reaction_c.mfds_device_cause_other)
					.bind(reaction_c.mfds_device_action_reason)
					.bind(reaction_c.mfds_device_action_recall)
					.bind(reaction_c.mfds_device_action_repair)
					.bind(reaction_c.mfds_device_action_inspection)
					.bind(reaction_c.mfds_device_action_replacement)
					.bind(reaction_c.mfds_device_action_improvement)
					.bind(reaction_c.mfds_device_action_monitoring)
					.bind(reaction_c.mfds_device_action_notification)
					.bind(reaction_c.mfds_device_action_label_change)
					.bind(reaction_c.mfds_device_action_other)
					.bind(reaction_c.start_date)
					.bind(reaction_c.start_date_null_flavor)
					.bind(reaction_c.end_date)
					.bind(reaction_c.end_date_null_flavor)
					.bind(reaction_c.duration_value)
					.bind(reaction_c.duration_unit)
					.bind(reaction_c.outcome)
					.bind(reaction_c.medical_confirmation)
					.bind(reaction_c.country_code)
					.bind(reaction_c.required_intervention_null_flavor)
					.bind(ctx.user_id()),
			)
			.await?;
		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get(_ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<Reaction> {
		let sql = format!("SELECT * FROM {} WHERE id = $1", Self::TABLE);
		let reaction = mm
			.dbx()
			.fetch_optional(sqlx::query_as::<_, Reaction>(&sql).bind(id))
			.await?
			.ok_or(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			})?;
		Ok(reaction)
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ReactionForUpdate,
	) -> Result<()> {
		Self::update_patch(ctx, mm, id, data, &[]).await
	}

	pub async fn update_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ReactionForUpdate,
		clear_fields: &[&str],
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let clears: Vec<String> =
			clear_fields.iter().map(|field| (*field).into()).collect();

		let sql = format!(
			"UPDATE {}
			 SET primary_source_reaction = CASE WHEN 'primary_source_reaction' = ANY($53) THEN NULL ELSE COALESCE($2, primary_source_reaction) END,
			     primary_source_reaction_translation = CASE WHEN 'primary_source_reaction_translation' = ANY($53) THEN NULL ELSE COALESCE($3, primary_source_reaction_translation) END,
			     reaction_language = CASE WHEN 'reaction_language' = ANY($53) THEN NULL ELSE COALESCE($4, reaction_language) END,
			     reaction_meddra_code = CASE WHEN 'reaction_meddra_code' = ANY($53) THEN NULL ELSE COALESCE($5, reaction_meddra_code) END,
			     reaction_meddra_version = CASE WHEN 'reaction_meddra_version' = ANY($53) THEN NULL ELSE COALESCE($6, reaction_meddra_version) END,
			     term_highlighted = CASE WHEN 'term_highlighted' = ANY($53) THEN NULL ELSE COALESCE($7, term_highlighted) END,
			     serious = CASE WHEN 'serious' = ANY($53) THEN NULL ELSE COALESCE($8, serious) END,
			     criteria_death = CASE WHEN 'criteria_death' = ANY($53) THEN NULL ELSE COALESCE($9, criteria_death) END,
			     criteria_death_null_flavor = CASE WHEN 'criteria_death_null_flavor' = ANY($53) THEN NULL ELSE COALESCE($10, criteria_death_null_flavor) END,
			     criteria_life_threatening = CASE WHEN 'criteria_life_threatening' = ANY($53) THEN NULL ELSE COALESCE($11, criteria_life_threatening) END,
			     criteria_life_threatening_null_flavor = CASE WHEN 'criteria_life_threatening_null_flavor' = ANY($53) THEN NULL ELSE COALESCE($12, criteria_life_threatening_null_flavor) END,
			     criteria_hospitalization = CASE WHEN 'criteria_hospitalization' = ANY($53) THEN NULL ELSE COALESCE($13, criteria_hospitalization) END,
			     criteria_hospitalization_null_flavor = CASE WHEN 'criteria_hospitalization_null_flavor' = ANY($53) THEN NULL ELSE COALESCE($14, criteria_hospitalization_null_flavor) END,
			     criteria_disabling = CASE WHEN 'criteria_disabling' = ANY($53) THEN NULL ELSE COALESCE($15, criteria_disabling) END,
			     criteria_disabling_null_flavor = CASE WHEN 'criteria_disabling_null_flavor' = ANY($53) THEN NULL ELSE COALESCE($16, criteria_disabling_null_flavor) END,
			     criteria_congenital_anomaly = CASE WHEN 'criteria_congenital_anomaly' = ANY($53) THEN NULL ELSE COALESCE($17, criteria_congenital_anomaly) END,
			     criteria_congenital_anomaly_null_flavor = CASE WHEN 'criteria_congenital_anomaly_null_flavor' = ANY($53) THEN NULL ELSE COALESCE($18, criteria_congenital_anomaly_null_flavor) END,
			     criteria_other_medically_important = CASE WHEN 'criteria_other_medically_important' = ANY($53) THEN NULL ELSE COALESCE($19, criteria_other_medically_important) END,
			     criteria_other_medically_important_null_flavor = CASE WHEN 'criteria_other_medically_important_null_flavor' = ANY($53) THEN NULL ELSE COALESCE($20, criteria_other_medically_important_null_flavor) END,
			     required_intervention = CASE WHEN 'required_intervention' = ANY($53) THEN NULL ELSE CASE WHEN $50 IS NOT NULL THEN NULL ELSE COALESCE($21, required_intervention) END END,
			     expectedness = CASE WHEN 'expectedness' = ANY($53) THEN NULL ELSE COALESCE($22, expectedness) END,
			     severity = CASE WHEN 'severity' = ANY($53) THEN NULL ELSE COALESCE($23, severity) END,
			     mfds_device_ae_classification = CASE WHEN 'mfds_device_ae_classification' = ANY($53) THEN NULL ELSE COALESCE($24, mfds_device_ae_classification) END,
			     mfds_device_ae_outcome = CASE WHEN 'mfds_device_ae_outcome' = ANY($53) THEN NULL ELSE COALESCE($25, mfds_device_ae_outcome) END,
			     mfds_device_cause_medical_device = CASE WHEN 'mfds_device_cause_medical_device' = ANY($53) THEN NULL ELSE COALESCE($26, mfds_device_cause_medical_device) END,
			     mfds_device_cause_procedure_issue = CASE WHEN 'mfds_device_cause_procedure_issue' = ANY($53) THEN NULL ELSE COALESCE($27, mfds_device_cause_procedure_issue) END,
			     mfds_device_cause_patient_condition = CASE WHEN 'mfds_device_cause_patient_condition' = ANY($53) THEN NULL ELSE COALESCE($28, mfds_device_cause_patient_condition) END,
			     mfds_device_cause_unable_to_assess = CASE WHEN 'mfds_device_cause_unable_to_assess' = ANY($53) THEN NULL ELSE COALESCE($29, mfds_device_cause_unable_to_assess) END,
			     mfds_device_cause_other = CASE WHEN 'mfds_device_cause_other' = ANY($53) THEN NULL ELSE COALESCE($30, mfds_device_cause_other) END,
			     mfds_device_action_reason = CASE WHEN 'mfds_device_action_reason' = ANY($53) THEN NULL ELSE COALESCE($31, mfds_device_action_reason) END,
			     mfds_device_action_recall = CASE WHEN 'mfds_device_action_recall' = ANY($53) THEN NULL ELSE COALESCE($32, mfds_device_action_recall) END,
			     mfds_device_action_repair = CASE WHEN 'mfds_device_action_repair' = ANY($53) THEN NULL ELSE COALESCE($33, mfds_device_action_repair) END,
			     mfds_device_action_inspection = CASE WHEN 'mfds_device_action_inspection' = ANY($53) THEN NULL ELSE COALESCE($34, mfds_device_action_inspection) END,
			     mfds_device_action_replacement = CASE WHEN 'mfds_device_action_replacement' = ANY($53) THEN NULL ELSE COALESCE($35, mfds_device_action_replacement) END,
			     mfds_device_action_improvement = CASE WHEN 'mfds_device_action_improvement' = ANY($53) THEN NULL ELSE COALESCE($36, mfds_device_action_improvement) END,
			     mfds_device_action_monitoring = CASE WHEN 'mfds_device_action_monitoring' = ANY($53) THEN NULL ELSE COALESCE($37, mfds_device_action_monitoring) END,
			     mfds_device_action_notification = CASE WHEN 'mfds_device_action_notification' = ANY($53) THEN NULL ELSE COALESCE($38, mfds_device_action_notification) END,
			     mfds_device_action_label_change = CASE WHEN 'mfds_device_action_label_change' = ANY($53) THEN NULL ELSE COALESCE($39, mfds_device_action_label_change) END,
			     mfds_device_action_other = CASE WHEN 'mfds_device_action_other' = ANY($53) THEN NULL ELSE COALESCE($40, mfds_device_action_other) END,
			     start_date = CASE WHEN 'start_date' = ANY($53) THEN NULL ELSE CASE WHEN $42 IS NOT NULL THEN NULL ELSE COALESCE($41, start_date) END END,
			     start_date_null_flavor = CASE WHEN 'start_date_null_flavor' = ANY($53) THEN NULL ELSE CASE WHEN $41 IS NOT NULL THEN NULL ELSE COALESCE($42, start_date_null_flavor) END END,
			     end_date = CASE WHEN 'end_date' = ANY($53) THEN NULL ELSE CASE WHEN $44 IS NOT NULL THEN NULL ELSE COALESCE($43, end_date) END END,
			     end_date_null_flavor = CASE WHEN 'end_date_null_flavor' = ANY($53) THEN NULL ELSE CASE WHEN $43 IS NOT NULL THEN NULL ELSE COALESCE($44, end_date_null_flavor) END END,
			     duration_value = CASE WHEN 'duration_value' = ANY($53) THEN NULL ELSE COALESCE($45, duration_value) END,
			     duration_unit = CASE WHEN 'duration_unit' = ANY($53) THEN NULL ELSE COALESCE($46, duration_unit) END,
			     outcome = CASE WHEN 'outcome' = ANY($53) THEN NULL ELSE COALESCE($47, outcome) END,
			     medical_confirmation = CASE WHEN 'medical_confirmation' = ANY($53) THEN NULL ELSE COALESCE($48, medical_confirmation) END,
			     country_code = CASE WHEN 'country_code' = ANY($53) THEN NULL ELSE COALESCE($49, country_code) END,
			     required_intervention_null_flavor = CASE WHEN 'required_intervention_null_flavor' = ANY($53) THEN NULL ELSE CASE WHEN $21 IS NOT NULL THEN NULL ELSE COALESCE($50, required_intervention_null_flavor) END END,
			     deleted = CASE WHEN 'deleted' = ANY($53) THEN NULL ELSE COALESCE($51, deleted) END,
			     updated_at = now(),
			     updated_by = $52
			 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(data.primary_source_reaction)
					.bind(data.primary_source_reaction_translation)
					.bind(data.reaction_language)
					.bind(data.reaction_meddra_code)
					.bind(data.reaction_meddra_version)
					.bind(data.term_highlighted)
					.bind(data.serious)
					.bind(data.criteria_death)
					.bind(data.criteria_death_null_flavor)
					.bind(data.criteria_life_threatening)
					.bind(data.criteria_life_threatening_null_flavor)
					.bind(data.criteria_hospitalization)
					.bind(data.criteria_hospitalization_null_flavor)
					.bind(data.criteria_disabling)
					.bind(data.criteria_disabling_null_flavor)
					.bind(data.criteria_congenital_anomaly)
					.bind(data.criteria_congenital_anomaly_null_flavor)
					.bind(data.criteria_other_medically_important)
					.bind(data.criteria_other_medically_important_null_flavor)
					.bind(data.required_intervention)
					.bind(data.expectedness)
					.bind(data.severity)
					.bind(data.mfds_device_ae_classification)
					.bind(data.mfds_device_ae_outcome)
					.bind(data.mfds_device_cause_medical_device)
					.bind(data.mfds_device_cause_procedure_issue)
					.bind(data.mfds_device_cause_patient_condition)
					.bind(data.mfds_device_cause_unable_to_assess)
					.bind(data.mfds_device_cause_other)
					.bind(data.mfds_device_action_reason)
					.bind(data.mfds_device_action_recall)
					.bind(data.mfds_device_action_repair)
					.bind(data.mfds_device_action_inspection)
					.bind(data.mfds_device_action_replacement)
					.bind(data.mfds_device_action_improvement)
					.bind(data.mfds_device_action_monitoring)
					.bind(data.mfds_device_action_notification)
					.bind(data.mfds_device_action_label_change)
					.bind(data.mfds_device_action_other)
					.bind(data.start_date)
					.bind(data.start_date_null_flavor)
					.bind(data.end_date)
					.bind(data.end_date_null_flavor)
					.bind(data.duration_value)
					.bind(data.duration_unit)
					.bind(data.outcome)
					.bind(data.medical_confirmation)
					.bind(data.country_code)
					.bind(data.required_intervention_null_flavor)
					.bind(data.deleted)
					.bind(ctx.user_id())
					.bind(clears),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn list_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<Reaction>> {
		Self::list_by_case_with_deleted(ctx, mm, case_id, false).await
	}

	pub async fn list_by_case_with_deleted(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		include_deleted: bool,
	) -> Result<Vec<Reaction>> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let deleted_filter = if include_deleted {
			""
		} else {
			" AND deleted = false"
		};
		let sql = format!(
			"SELECT * FROM {} WHERE case_id = $1{} ORDER BY sequence_number",
			Self::TABLE,
			deleted_filter
		);
		let result = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, Reaction>(&sql).bind(case_id))
			.await;
		match result {
			Ok(reactions) => {
				mm.dbx().commit_txn().await?;
				Ok(reactions)
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn get_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<Reaction> {
		Self::get_in_case_with_deleted(ctx, mm, case_id, id, false).await
	}

	pub async fn get_in_case_with_deleted(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		include_deleted: bool,
	) -> Result<Reaction> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let deleted_filter = if include_deleted {
			""
		} else {
			" AND deleted = false"
		};
		let sql = format!(
			"SELECT * FROM {} WHERE id = $1 AND case_id = $2{}",
			Self::TABLE,
			deleted_filter
		);
		let result = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, Reaction>(&sql).bind(id).bind(case_id),
			)
			.await;
		match result {
			Ok(Some(reaction)) => {
				mm.dbx().commit_txn().await?;
				Ok(reaction)
			}
			Ok(None) => {
				mm.dbx().commit_txn().await?;
				Err(crate::model::Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id,
				})
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn update_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		data: ReactionForUpdate,
	) -> Result<()> {
		Self::update_in_case_patch(ctx, mm, case_id, id, data, &[]).await
	}

	pub async fn update_in_case_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		data: ReactionForUpdate,
		clear_fields: &[&str],
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let clears: Vec<String> =
			clear_fields.iter().map(|field| (*field).into()).collect();

		let sql = format!(
			"UPDATE {}
			 SET primary_source_reaction = CASE WHEN 'primary_source_reaction' = ANY($54) THEN NULL ELSE COALESCE($3, primary_source_reaction) END,
			     primary_source_reaction_translation = CASE WHEN 'primary_source_reaction_translation' = ANY($54) THEN NULL ELSE COALESCE($4, primary_source_reaction_translation) END,
			     reaction_language = CASE WHEN 'reaction_language' = ANY($54) THEN NULL ELSE COALESCE($5, reaction_language) END,
			     reaction_meddra_code = CASE WHEN 'reaction_meddra_code' = ANY($54) THEN NULL ELSE COALESCE($6, reaction_meddra_code) END,
			     reaction_meddra_version = CASE WHEN 'reaction_meddra_version' = ANY($54) THEN NULL ELSE COALESCE($7, reaction_meddra_version) END,
			     term_highlighted = CASE WHEN 'term_highlighted' = ANY($54) THEN NULL ELSE COALESCE($8, term_highlighted) END,
			     serious = CASE WHEN 'serious' = ANY($54) THEN NULL ELSE COALESCE($9, serious) END,
			     criteria_death = CASE WHEN 'criteria_death' = ANY($54) THEN NULL ELSE COALESCE($10, criteria_death) END,
			     criteria_death_null_flavor = CASE WHEN 'criteria_death_null_flavor' = ANY($54) THEN NULL ELSE COALESCE($11, criteria_death_null_flavor) END,
			     criteria_life_threatening = CASE WHEN 'criteria_life_threatening' = ANY($54) THEN NULL ELSE COALESCE($12, criteria_life_threatening) END,
			     criteria_life_threatening_null_flavor = CASE WHEN 'criteria_life_threatening_null_flavor' = ANY($54) THEN NULL ELSE COALESCE($13, criteria_life_threatening_null_flavor) END,
			     criteria_hospitalization = CASE WHEN 'criteria_hospitalization' = ANY($54) THEN NULL ELSE COALESCE($14, criteria_hospitalization) END,
			     criteria_hospitalization_null_flavor = CASE WHEN 'criteria_hospitalization_null_flavor' = ANY($54) THEN NULL ELSE COALESCE($15, criteria_hospitalization_null_flavor) END,
			     criteria_disabling = CASE WHEN 'criteria_disabling' = ANY($54) THEN NULL ELSE COALESCE($16, criteria_disabling) END,
			     criteria_disabling_null_flavor = CASE WHEN 'criteria_disabling_null_flavor' = ANY($54) THEN NULL ELSE COALESCE($17, criteria_disabling_null_flavor) END,
			     criteria_congenital_anomaly = CASE WHEN 'criteria_congenital_anomaly' = ANY($54) THEN NULL ELSE COALESCE($18, criteria_congenital_anomaly) END,
			     criteria_congenital_anomaly_null_flavor = CASE WHEN 'criteria_congenital_anomaly_null_flavor' = ANY($54) THEN NULL ELSE COALESCE($19, criteria_congenital_anomaly_null_flavor) END,
			     criteria_other_medically_important = CASE WHEN 'criteria_other_medically_important' = ANY($54) THEN NULL ELSE COALESCE($20, criteria_other_medically_important) END,
			     criteria_other_medically_important_null_flavor = CASE WHEN 'criteria_other_medically_important_null_flavor' = ANY($54) THEN NULL ELSE COALESCE($21, criteria_other_medically_important_null_flavor) END,
			     required_intervention = CASE WHEN 'required_intervention' = ANY($54) THEN NULL ELSE CASE WHEN $51 IS NOT NULL THEN NULL ELSE COALESCE($22, required_intervention) END END,
			     expectedness = CASE WHEN 'expectedness' = ANY($54) THEN NULL ELSE COALESCE($23, expectedness) END,
			     severity = CASE WHEN 'severity' = ANY($54) THEN NULL ELSE COALESCE($24, severity) END,
			     mfds_device_ae_classification = CASE WHEN 'mfds_device_ae_classification' = ANY($54) THEN NULL ELSE COALESCE($25, mfds_device_ae_classification) END,
			     mfds_device_ae_outcome = CASE WHEN 'mfds_device_ae_outcome' = ANY($54) THEN NULL ELSE COALESCE($26, mfds_device_ae_outcome) END,
			     mfds_device_cause_medical_device = CASE WHEN 'mfds_device_cause_medical_device' = ANY($54) THEN NULL ELSE COALESCE($27, mfds_device_cause_medical_device) END,
			     mfds_device_cause_procedure_issue = CASE WHEN 'mfds_device_cause_procedure_issue' = ANY($54) THEN NULL ELSE COALESCE($28, mfds_device_cause_procedure_issue) END,
			     mfds_device_cause_patient_condition = CASE WHEN 'mfds_device_cause_patient_condition' = ANY($54) THEN NULL ELSE COALESCE($29, mfds_device_cause_patient_condition) END,
			     mfds_device_cause_unable_to_assess = CASE WHEN 'mfds_device_cause_unable_to_assess' = ANY($54) THEN NULL ELSE COALESCE($30, mfds_device_cause_unable_to_assess) END,
			     mfds_device_cause_other = CASE WHEN 'mfds_device_cause_other' = ANY($54) THEN NULL ELSE COALESCE($31, mfds_device_cause_other) END,
			     mfds_device_action_reason = CASE WHEN 'mfds_device_action_reason' = ANY($54) THEN NULL ELSE COALESCE($32, mfds_device_action_reason) END,
			     mfds_device_action_recall = CASE WHEN 'mfds_device_action_recall' = ANY($54) THEN NULL ELSE COALESCE($33, mfds_device_action_recall) END,
			     mfds_device_action_repair = CASE WHEN 'mfds_device_action_repair' = ANY($54) THEN NULL ELSE COALESCE($34, mfds_device_action_repair) END,
			     mfds_device_action_inspection = CASE WHEN 'mfds_device_action_inspection' = ANY($54) THEN NULL ELSE COALESCE($35, mfds_device_action_inspection) END,
			     mfds_device_action_replacement = CASE WHEN 'mfds_device_action_replacement' = ANY($54) THEN NULL ELSE COALESCE($36, mfds_device_action_replacement) END,
			     mfds_device_action_improvement = CASE WHEN 'mfds_device_action_improvement' = ANY($54) THEN NULL ELSE COALESCE($37, mfds_device_action_improvement) END,
			     mfds_device_action_monitoring = CASE WHEN 'mfds_device_action_monitoring' = ANY($54) THEN NULL ELSE COALESCE($38, mfds_device_action_monitoring) END,
			     mfds_device_action_notification = CASE WHEN 'mfds_device_action_notification' = ANY($54) THEN NULL ELSE COALESCE($39, mfds_device_action_notification) END,
			     mfds_device_action_label_change = CASE WHEN 'mfds_device_action_label_change' = ANY($54) THEN NULL ELSE COALESCE($40, mfds_device_action_label_change) END,
			     mfds_device_action_other = CASE WHEN 'mfds_device_action_other' = ANY($54) THEN NULL ELSE COALESCE($41, mfds_device_action_other) END,
			     start_date = CASE WHEN 'start_date' = ANY($54) THEN NULL ELSE CASE WHEN $43 IS NOT NULL THEN NULL ELSE COALESCE($42, start_date) END END,
			     start_date_null_flavor = CASE WHEN 'start_date_null_flavor' = ANY($54) THEN NULL ELSE CASE WHEN $42 IS NOT NULL THEN NULL ELSE COALESCE($43, start_date_null_flavor) END END,
			     end_date = CASE WHEN 'end_date' = ANY($54) THEN NULL ELSE CASE WHEN $45 IS NOT NULL THEN NULL ELSE COALESCE($44, end_date) END END,
			     end_date_null_flavor = CASE WHEN 'end_date_null_flavor' = ANY($54) THEN NULL ELSE CASE WHEN $44 IS NOT NULL THEN NULL ELSE COALESCE($45, end_date_null_flavor) END END,
			     duration_value = CASE WHEN 'duration_value' = ANY($54) THEN NULL ELSE COALESCE($46, duration_value) END,
			     duration_unit = CASE WHEN 'duration_unit' = ANY($54) THEN NULL ELSE COALESCE($47, duration_unit) END,
			     outcome = CASE WHEN 'outcome' = ANY($54) THEN NULL ELSE COALESCE($48, outcome) END,
			     medical_confirmation = CASE WHEN 'medical_confirmation' = ANY($54) THEN NULL ELSE COALESCE($49, medical_confirmation) END,
			     country_code = CASE WHEN 'country_code' = ANY($54) THEN NULL ELSE COALESCE($50, country_code) END,
			     required_intervention_null_flavor = CASE WHEN 'required_intervention_null_flavor' = ANY($54) THEN NULL ELSE CASE WHEN $22 IS NOT NULL THEN NULL ELSE COALESCE($51, required_intervention_null_flavor) END END,
			     deleted = CASE WHEN 'deleted' = ANY($54) THEN NULL ELSE COALESCE($52, deleted) END,
			     updated_at = now(),
			     updated_by = $53
			 WHERE id = $1 AND case_id = $2 AND (deleted = false OR $52 = false)",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(case_id)
					.bind(data.primary_source_reaction)
					.bind(data.primary_source_reaction_translation)
					.bind(data.reaction_language)
					.bind(data.reaction_meddra_code)
					.bind(data.reaction_meddra_version)
					.bind(data.term_highlighted)
					.bind(data.serious)
					.bind(data.criteria_death)
					.bind(data.criteria_death_null_flavor)
					.bind(data.criteria_life_threatening)
					.bind(data.criteria_life_threatening_null_flavor)
					.bind(data.criteria_hospitalization)
					.bind(data.criteria_hospitalization_null_flavor)
					.bind(data.criteria_disabling)
					.bind(data.criteria_disabling_null_flavor)
					.bind(data.criteria_congenital_anomaly)
					.bind(data.criteria_congenital_anomaly_null_flavor)
					.bind(data.criteria_other_medically_important)
					.bind(data.criteria_other_medically_important_null_flavor)
					.bind(data.required_intervention)
					.bind(data.expectedness)
					.bind(data.severity)
					.bind(data.mfds_device_ae_classification)
					.bind(data.mfds_device_ae_outcome)
					.bind(data.mfds_device_cause_medical_device)
					.bind(data.mfds_device_cause_procedure_issue)
					.bind(data.mfds_device_cause_patient_condition)
					.bind(data.mfds_device_cause_unable_to_assess)
					.bind(data.mfds_device_cause_other)
					.bind(data.mfds_device_action_reason)
					.bind(data.mfds_device_action_recall)
					.bind(data.mfds_device_action_repair)
					.bind(data.mfds_device_action_inspection)
					.bind(data.mfds_device_action_replacement)
					.bind(data.mfds_device_action_improvement)
					.bind(data.mfds_device_action_monitoring)
					.bind(data.mfds_device_action_notification)
					.bind(data.mfds_device_action_label_change)
					.bind(data.mfds_device_action_other)
					.bind(data.start_date)
					.bind(data.start_date_null_flavor)
					.bind(data.end_date)
					.bind(data.end_date_null_flavor)
					.bind(data.duration_value)
					.bind(data.duration_unit)
					.bind(data.outcome)
					.bind(data.medical_confirmation)
					.bind(data.country_code)
					.bind(data.required_intervention_null_flavor)
					.bind(data.deleted)
					.bind(ctx.user_id())
					.bind(clears),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		Self::set_deleted(ctx, mm, id, true).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		Self::set_deleted(ctx, mm, id, false).await
	}

	async fn set_deleted(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		deleted: bool,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {} SET deleted = $2, updated_at = now(), updated_by = $3 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(sqlx::query(&sql).bind(id).bind(deleted).bind(ctx.user_id()))
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<()> {
		Self::set_deleted_in_case(ctx, mm, case_id, id, true).await
	}

	pub async fn restore_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<()> {
		Self::set_deleted_in_case(ctx, mm, case_id, id, false).await
	}

	async fn set_deleted_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		deleted: bool,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {} SET deleted = $3, updated_at = now(), updated_by = $4 WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(case_id)
					.bind(deleted)
					.bind(ctx.user_id()),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}
}
