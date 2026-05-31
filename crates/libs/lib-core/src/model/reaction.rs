// Section E - Reaction/Event

use crate::ctx::Ctx;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, OpValsBool, OpValsValue};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- Reaction

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct Reaction {
	pub id: Uuid,
	pub case_id: Uuid,
	pub sequence_number: i32,

	// E.i.1.1 - Reaction as reported
	pub primary_source_reaction: String,
	// E.i.1.2 - Reaction/Event as reported by primary source for translation
	pub primary_source_reaction_translation: Option<String>,
	pub reaction_language: Option<String>,

	// E.i.2.1 - MedDRA coding
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,

	// E.i.3 - Term Highlighted by Reporter
	pub term_highlighted: Option<bool>,

	// E.i.3.1 - Seriousness (MANDATORY if serious)
	pub serious: Option<bool>,

	// E.i.3.2 - Seriousness Criteria
	pub criteria_death: bool,
	pub criteria_death_null_flavor: Option<String>,
	pub criteria_life_threatening: bool,
	pub criteria_life_threatening_null_flavor: Option<String>,
	pub criteria_hospitalization: bool,
	pub criteria_hospitalization_null_flavor: Option<String>,
	pub criteria_disabling: bool,
	pub criteria_disabling_null_flavor: Option<String>,
	pub criteria_congenital_anomaly: bool,
	pub criteria_congenital_anomaly_null_flavor: Option<String>,
	pub criteria_other_medically_important: bool,
	pub criteria_other_medically_important_null_flavor: Option<String>,
	// FDA.E.i.3.2h - Required Intervention (FDA)
	pub required_intervention: Option<String>,

	pub included_in_ema_ime_list: Option<bool>,
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
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,

	// E.i.7 - Outcome
	pub outcome: Option<String>,

	// E.i.8 - Medical Confirmation
	pub medical_confirmation: Option<bool>,

	// E.i.9 - Country
	pub country_code: Option<String>,

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
	pub primary_source_reaction: String,
	pub primary_source_reaction_translation: Option<String>,
	pub reaction_language: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub term_highlighted: Option<bool>,
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
	pub required_intervention: Option<String>,
	pub included_in_ema_ime_list: Option<bool>,
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
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub outcome: Option<String>,
	pub medical_confirmation: Option<bool>,
	pub country_code: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct ReactionForUpdate {
	pub primary_source_reaction: Option<String>,
	pub primary_source_reaction_translation: Option<String>,
	pub reaction_language: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub term_highlighted: Option<bool>,
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
	pub required_intervention: Option<String>,
	pub included_in_ema_ime_list: Option<bool>,
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
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub outcome: Option<String>,
	pub medical_confirmation: Option<bool>,
	pub country_code: Option<String>,
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
			 included_in_ema_ime_list, expectedness, severity, mfds_device_ae_classification,
			 mfds_device_ae_outcome, mfds_device_cause_medical_device,
			 mfds_device_cause_procedure_issue, mfds_device_cause_patient_condition,
			 mfds_device_cause_unable_to_assess, mfds_device_cause_other,
			 mfds_device_action_reason, mfds_device_action_recall, mfds_device_action_repair,
			 mfds_device_action_inspection, mfds_device_action_replacement,
			 mfds_device_action_improvement, mfds_device_action_monitoring,
			 mfds_device_action_notification, mfds_device_action_label_change,
			 mfds_device_action_other, start_date,
			 start_date_null_flavor, end_date, end_date_null_flavor, duration_value, duration_unit,
			 outcome, medical_confirmation, country_code, created_at, updated_at, created_by
			)
			 VALUES (
			 $1, $2, $3, $4,
			 $5, $6, $7, $8,
			 $9, COALESCE($10, false), $11, COALESCE($12, false),
			 $13, COALESCE($14, false),
			 $15, COALESCE($16, false),
			 $17, COALESCE($18, false),
			 $19, COALESCE($20, false),
			 $21, $22, $23, $24,
			 $25, $26, $27, $28,
			 $29, $30, $31, $32, $33,
			 $34, $35, $36,
			 $37, $38, $39,
			 $40, $41, $42,
			 $43, $44, $45, $46, $47,
			 $48, $49, $50, $51, now(), now(), $52
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
					.bind(reaction_c.included_in_ema_ime_list)
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
		reaction_u: ReactionForUpdate,
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
			"UPDATE {}
			 SET primary_source_reaction = COALESCE($2, primary_source_reaction),
			     primary_source_reaction_translation = COALESCE($3, primary_source_reaction_translation),
			     reaction_language = COALESCE($4, reaction_language),
			     reaction_meddra_code = COALESCE($5, reaction_meddra_code),
			     reaction_meddra_version = COALESCE($6, reaction_meddra_version),
			     term_highlighted = COALESCE($7, term_highlighted),
			     serious = COALESCE($8, serious),
			     criteria_death = COALESCE($9, criteria_death),
			     criteria_death_null_flavor = COALESCE($10, criteria_death_null_flavor),
			     criteria_life_threatening = COALESCE($11, criteria_life_threatening),
			     criteria_life_threatening_null_flavor = COALESCE($12, criteria_life_threatening_null_flavor),
			     criteria_hospitalization = COALESCE($13, criteria_hospitalization),
			     criteria_hospitalization_null_flavor = COALESCE($14, criteria_hospitalization_null_flavor),
			     criteria_disabling = COALESCE($15, criteria_disabling),
			     criteria_disabling_null_flavor = COALESCE($16, criteria_disabling_null_flavor),
			     criteria_congenital_anomaly = COALESCE($17, criteria_congenital_anomaly),
			     criteria_congenital_anomaly_null_flavor = COALESCE($18, criteria_congenital_anomaly_null_flavor),
			     criteria_other_medically_important = COALESCE($19, criteria_other_medically_important),
			     criteria_other_medically_important_null_flavor = COALESCE($20, criteria_other_medically_important_null_flavor),
			     required_intervention = COALESCE($21, required_intervention),
			     included_in_ema_ime_list = COALESCE($22, included_in_ema_ime_list),
			     expectedness = COALESCE($23, expectedness),
			     severity = COALESCE($24, severity),
			     mfds_device_ae_classification = COALESCE($25, mfds_device_ae_classification),
			     mfds_device_ae_outcome = COALESCE($26, mfds_device_ae_outcome),
			     mfds_device_cause_medical_device = COALESCE($27, mfds_device_cause_medical_device),
			     mfds_device_cause_procedure_issue = COALESCE($28, mfds_device_cause_procedure_issue),
			     mfds_device_cause_patient_condition = COALESCE($29, mfds_device_cause_patient_condition),
			     mfds_device_cause_unable_to_assess = COALESCE($30, mfds_device_cause_unable_to_assess),
			     mfds_device_cause_other = COALESCE($31, mfds_device_cause_other),
			     mfds_device_action_reason = COALESCE($32, mfds_device_action_reason),
			     mfds_device_action_recall = COALESCE($33, mfds_device_action_recall),
			     mfds_device_action_repair = COALESCE($34, mfds_device_action_repair),
			     mfds_device_action_inspection = COALESCE($35, mfds_device_action_inspection),
			     mfds_device_action_replacement = COALESCE($36, mfds_device_action_replacement),
			     mfds_device_action_improvement = COALESCE($37, mfds_device_action_improvement),
			     mfds_device_action_monitoring = COALESCE($38, mfds_device_action_monitoring),
			     mfds_device_action_notification = COALESCE($39, mfds_device_action_notification),
			     mfds_device_action_label_change = COALESCE($40, mfds_device_action_label_change),
			     mfds_device_action_other = COALESCE($41, mfds_device_action_other),
			     start_date = CASE WHEN $43 IS NOT NULL THEN NULL ELSE COALESCE($42, start_date) END,
			     start_date_null_flavor = CASE WHEN $42 IS NOT NULL THEN NULL ELSE COALESCE($43, start_date_null_flavor) END,
			     end_date = CASE WHEN $45 IS NOT NULL THEN NULL ELSE COALESCE($44, end_date) END,
			     end_date_null_flavor = CASE WHEN $44 IS NOT NULL THEN NULL ELSE COALESCE($45, end_date_null_flavor) END,
			     duration_value = COALESCE($46, duration_value),
			     duration_unit = COALESCE($47, duration_unit),
			     outcome = COALESCE($48, outcome),
			     medical_confirmation = COALESCE($49, medical_confirmation),
			     country_code = COALESCE($50, country_code),
			     updated_at = now(),
			     updated_by = $51
			 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(reaction_u.primary_source_reaction)
					.bind(reaction_u.primary_source_reaction_translation)
					.bind(reaction_u.reaction_language)
					.bind(reaction_u.reaction_meddra_code)
					.bind(reaction_u.reaction_meddra_version)
					.bind(reaction_u.term_highlighted)
					.bind(reaction_u.serious)
					.bind(reaction_u.criteria_death)
					.bind(reaction_u.criteria_death_null_flavor)
					.bind(reaction_u.criteria_life_threatening)
					.bind(reaction_u.criteria_life_threatening_null_flavor)
					.bind(reaction_u.criteria_hospitalization)
					.bind(reaction_u.criteria_hospitalization_null_flavor)
					.bind(reaction_u.criteria_disabling)
					.bind(reaction_u.criteria_disabling_null_flavor)
					.bind(reaction_u.criteria_congenital_anomaly)
					.bind(reaction_u.criteria_congenital_anomaly_null_flavor)
					.bind(reaction_u.criteria_other_medically_important)
					.bind(reaction_u.criteria_other_medically_important_null_flavor)
					.bind(reaction_u.required_intervention)
					.bind(reaction_u.included_in_ema_ime_list)
					.bind(reaction_u.expectedness)
					.bind(reaction_u.severity)
					.bind(reaction_u.mfds_device_ae_classification)
					.bind(reaction_u.mfds_device_ae_outcome)
					.bind(reaction_u.mfds_device_cause_medical_device)
					.bind(reaction_u.mfds_device_cause_procedure_issue)
					.bind(reaction_u.mfds_device_cause_patient_condition)
					.bind(reaction_u.mfds_device_cause_unable_to_assess)
					.bind(reaction_u.mfds_device_cause_other)
					.bind(reaction_u.mfds_device_action_reason)
					.bind(reaction_u.mfds_device_action_recall)
					.bind(reaction_u.mfds_device_action_repair)
					.bind(reaction_u.mfds_device_action_inspection)
					.bind(reaction_u.mfds_device_action_replacement)
					.bind(reaction_u.mfds_device_action_improvement)
					.bind(reaction_u.mfds_device_action_monitoring)
					.bind(reaction_u.mfds_device_action_notification)
					.bind(reaction_u.mfds_device_action_label_change)
					.bind(reaction_u.mfds_device_action_other)
					.bind(reaction_u.start_date)
					.bind(reaction_u.start_date_null_flavor)
					.bind(reaction_u.end_date)
					.bind(reaction_u.end_date_null_flavor)
					.bind(reaction_u.duration_value)
					.bind(reaction_u.duration_unit)
					.bind(reaction_u.outcome)
					.bind(reaction_u.medical_confirmation)
					.bind(reaction_u.country_code)
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

	pub async fn list_by_case(
		_ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<Reaction>> {
		let sql = format!(
			"SELECT * FROM {} WHERE case_id = $1 ORDER BY sequence_number",
			Self::TABLE
		);
		let reactions = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, Reaction>(&sql).bind(case_id))
			.await?;
		Ok(reactions)
	}

	pub async fn get_in_case(
		_ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<Reaction> {
		let sql = format!(
			"SELECT * FROM {} WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let reaction = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, Reaction>(&sql).bind(id).bind(case_id),
			)
			.await?
			.ok_or(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			})?;
		Ok(reaction)
	}

	pub async fn update_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		reaction_u: ReactionForUpdate,
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
			"UPDATE {}
			 SET primary_source_reaction = COALESCE($3, primary_source_reaction),
			     primary_source_reaction_translation = COALESCE($4, primary_source_reaction_translation),
			     reaction_language = COALESCE($5, reaction_language),
			     reaction_meddra_code = COALESCE($6, reaction_meddra_code),
			     reaction_meddra_version = COALESCE($7, reaction_meddra_version),
			     term_highlighted = COALESCE($8, term_highlighted),
			     serious = COALESCE($9, serious),
			     criteria_death = COALESCE($10, criteria_death),
			     criteria_death_null_flavor = COALESCE($11, criteria_death_null_flavor),
			     criteria_life_threatening = COALESCE($12, criteria_life_threatening),
			     criteria_life_threatening_null_flavor = COALESCE($13, criteria_life_threatening_null_flavor),
			     criteria_hospitalization = COALESCE($14, criteria_hospitalization),
			     criteria_hospitalization_null_flavor = COALESCE($15, criteria_hospitalization_null_flavor),
			     criteria_disabling = COALESCE($16, criteria_disabling),
			     criteria_disabling_null_flavor = COALESCE($17, criteria_disabling_null_flavor),
			     criteria_congenital_anomaly = COALESCE($18, criteria_congenital_anomaly),
			     criteria_congenital_anomaly_null_flavor = COALESCE($19, criteria_congenital_anomaly_null_flavor),
			     criteria_other_medically_important = COALESCE($20, criteria_other_medically_important),
			     criteria_other_medically_important_null_flavor = COALESCE($21, criteria_other_medically_important_null_flavor),
			     required_intervention = COALESCE($22, required_intervention),
			     included_in_ema_ime_list = COALESCE($23, included_in_ema_ime_list),
			     expectedness = COALESCE($24, expectedness),
			     severity = COALESCE($25, severity),
			     mfds_device_ae_classification = COALESCE($26, mfds_device_ae_classification),
			     mfds_device_ae_outcome = COALESCE($27, mfds_device_ae_outcome),
			     mfds_device_cause_medical_device = COALESCE($28, mfds_device_cause_medical_device),
			     mfds_device_cause_procedure_issue = COALESCE($29, mfds_device_cause_procedure_issue),
			     mfds_device_cause_patient_condition = COALESCE($30, mfds_device_cause_patient_condition),
			     mfds_device_cause_unable_to_assess = COALESCE($31, mfds_device_cause_unable_to_assess),
			     mfds_device_cause_other = COALESCE($32, mfds_device_cause_other),
			     mfds_device_action_reason = COALESCE($33, mfds_device_action_reason),
			     mfds_device_action_recall = COALESCE($34, mfds_device_action_recall),
			     mfds_device_action_repair = COALESCE($35, mfds_device_action_repair),
			     mfds_device_action_inspection = COALESCE($36, mfds_device_action_inspection),
			     mfds_device_action_replacement = COALESCE($37, mfds_device_action_replacement),
			     mfds_device_action_improvement = COALESCE($38, mfds_device_action_improvement),
			     mfds_device_action_monitoring = COALESCE($39, mfds_device_action_monitoring),
			     mfds_device_action_notification = COALESCE($40, mfds_device_action_notification),
			     mfds_device_action_label_change = COALESCE($41, mfds_device_action_label_change),
			     mfds_device_action_other = COALESCE($42, mfds_device_action_other),
			     start_date = CASE WHEN $44 IS NOT NULL THEN NULL ELSE COALESCE($43, start_date) END,
			     start_date_null_flavor = CASE WHEN $43 IS NOT NULL THEN NULL ELSE COALESCE($44, start_date_null_flavor) END,
			     end_date = CASE WHEN $46 IS NOT NULL THEN NULL ELSE COALESCE($45, end_date) END,
			     end_date_null_flavor = CASE WHEN $45 IS NOT NULL THEN NULL ELSE COALESCE($46, end_date_null_flavor) END,
			     duration_value = COALESCE($47, duration_value),
			     duration_unit = COALESCE($48, duration_unit),
			     outcome = COALESCE($49, outcome),
			     medical_confirmation = COALESCE($50, medical_confirmation),
			     country_code = COALESCE($51, country_code),
			     updated_at = now(),
			     updated_by = $52
			 WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(case_id)
					.bind(reaction_u.primary_source_reaction)
					.bind(reaction_u.primary_source_reaction_translation)
					.bind(reaction_u.reaction_language)
					.bind(reaction_u.reaction_meddra_code)
					.bind(reaction_u.reaction_meddra_version)
					.bind(reaction_u.term_highlighted)
					.bind(reaction_u.serious)
					.bind(reaction_u.criteria_death)
					.bind(reaction_u.criteria_death_null_flavor)
					.bind(reaction_u.criteria_life_threatening)
					.bind(reaction_u.criteria_life_threatening_null_flavor)
					.bind(reaction_u.criteria_hospitalization)
					.bind(reaction_u.criteria_hospitalization_null_flavor)
					.bind(reaction_u.criteria_disabling)
					.bind(reaction_u.criteria_disabling_null_flavor)
					.bind(reaction_u.criteria_congenital_anomaly)
					.bind(reaction_u.criteria_congenital_anomaly_null_flavor)
					.bind(reaction_u.criteria_other_medically_important)
					.bind(reaction_u.criteria_other_medically_important_null_flavor)
					.bind(reaction_u.required_intervention)
					.bind(reaction_u.included_in_ema_ime_list)
					.bind(reaction_u.expectedness)
					.bind(reaction_u.severity)
					.bind(reaction_u.mfds_device_ae_classification)
					.bind(reaction_u.mfds_device_ae_outcome)
					.bind(reaction_u.mfds_device_cause_medical_device)
					.bind(reaction_u.mfds_device_cause_procedure_issue)
					.bind(reaction_u.mfds_device_cause_patient_condition)
					.bind(reaction_u.mfds_device_cause_unable_to_assess)
					.bind(reaction_u.mfds_device_cause_other)
					.bind(reaction_u.mfds_device_action_reason)
					.bind(reaction_u.mfds_device_action_recall)
					.bind(reaction_u.mfds_device_action_repair)
					.bind(reaction_u.mfds_device_action_inspection)
					.bind(reaction_u.mfds_device_action_replacement)
					.bind(reaction_u.mfds_device_action_improvement)
					.bind(reaction_u.mfds_device_action_monitoring)
					.bind(reaction_u.mfds_device_action_notification)
					.bind(reaction_u.mfds_device_action_label_change)
					.bind(reaction_u.mfds_device_action_other)
					.bind(reaction_u.start_date)
					.bind(reaction_u.start_date_null_flavor)
					.bind(reaction_u.end_date)
					.bind(reaction_u.end_date_null_flavor)
					.bind(reaction_u.duration_value)
					.bind(reaction_u.duration_unit)
					.bind(reaction_u.outcome)
					.bind(reaction_u.medical_confirmation)
					.bind(reaction_u.country_code)
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

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!("DELETE FROM {} WHERE id = $1", Self::TABLE);
		let result = mm.dbx().execute(sqlx::query(&sql).bind(id)).await?;
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
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql =
			format!("DELETE FROM {} WHERE id = $1 AND case_id = $2", Self::TABLE);
		let result = mm
			.dbx()
			.execute(sqlx::query(&sql).bind(id).bind(case_id))
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
