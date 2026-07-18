// Section D - Patient Information

use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::{
	set_full_context_dbx_or_rollback, set_full_context_from_ctx_dbx,
};
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{
	FilterNodes, ListOptions, OpValBool, OpValsBool, OpValsString, OpValsValue,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- PatientInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct PatientInformation {
	pub id: Uuid,
	pub case_id: Uuid,

	// D.1 - Patient identification
	pub patient_initials: Option<String>,

	// D.2 - Age
	pub birth_date: Option<Date>,
	pub age_at_time_of_onset: Option<Decimal>,
	pub age_unit: Option<String>,
	pub gestation_period: Option<Decimal>,
	pub gestation_period_unit: Option<String>,
	pub age_group: Option<String>,

	// D.3-5 - Physical
	pub weight_kg: Option<Decimal>,
	pub weight_kg_null_flavor: Option<String>,
	pub height_cm: Option<Decimal>,
	pub height_cm_null_flavor: Option<String>,
	pub sex: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	pub birth_date_null_flavor: Option<String>,
	pub age_at_time_of_onset_null_flavor: Option<String>,
	pub sex_null_flavor: Option<String>,

	// FDA.D.11 / FDA.D.12 - Race / Ethnicity (FDA)
	pub race_code: Option<String>,
	pub race_code_null_flavor: Option<String>,
	pub ethnicity_code: Option<String>,
	pub ethnicity_code_null_flavor: Option<String>,

	// D.6 - Last Menstrual Period
	pub last_menstrual_period_date: Option<Date>,
	pub last_menstrual_period_date_null_flavor: Option<String>,

	// D.7.2 - Medical history
	pub medical_history_text: Option<String>,
	pub medical_history_text_null_flavor: Option<String>,
	// D.7.3 - Concomitant Therapies
	pub concomitant_therapy: Option<bool>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct PatientInformationForCreate {
	pub case_id: Uuid,
	pub patient_initials: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub birth_date: Option<Date>,
	pub birth_date_null_flavor: Option<String>,
	pub age_at_time_of_onset: Option<Decimal>,
	pub age_at_time_of_onset_null_flavor: Option<String>,
	pub age_unit: Option<String>,
	pub gestation_period: Option<Decimal>,
	pub gestation_period_unit: Option<String>,
	pub age_group: Option<String>,
	pub weight_kg: Option<Decimal>,
	pub weight_kg_null_flavor: Option<String>,
	pub height_cm: Option<Decimal>,
	pub height_cm_null_flavor: Option<String>,
	pub sex: Option<String>,
	pub sex_null_flavor: Option<String>,
	pub race_code: Option<String>,
	pub race_code_null_flavor: Option<String>,
	pub ethnicity_code: Option<String>,
	pub ethnicity_code_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub last_menstrual_period_date: Option<Date>,
	pub last_menstrual_period_date_null_flavor: Option<String>,
	pub medical_history_text: Option<String>,
	pub medical_history_text_null_flavor: Option<String>,
	pub concomitant_therapy: Option<bool>,
}

#[derive(Fields, Deserialize)]
pub struct PatientInformationForUpdate {
	pub patient_initials: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub birth_date: Option<Date>,
	pub birth_date_null_flavor: Option<String>,
	pub age_at_time_of_onset: Option<Decimal>,
	pub age_at_time_of_onset_null_flavor: Option<String>,
	pub age_unit: Option<String>,
	pub gestation_period: Option<Decimal>,
	pub gestation_period_unit: Option<String>,
	pub age_group: Option<String>,
	pub weight_kg: Option<Decimal>,
	pub weight_kg_null_flavor: Option<String>,
	pub height_cm: Option<Decimal>,
	pub height_cm_null_flavor: Option<String>,
	pub sex: Option<String>,
	pub sex_null_flavor: Option<String>,
	pub race_code: Option<String>,
	pub race_code_null_flavor: Option<String>,
	pub ethnicity_code: Option<String>,
	pub ethnicity_code_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub last_menstrual_period_date: Option<Date>,
	pub last_menstrual_period_date_null_flavor: Option<String>,
	pub medical_history_text: Option<String>,
	pub medical_history_text_null_flavor: Option<String>,
	pub concomitant_therapy: Option<bool>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct PatientInformationFilter {
	pub patient_initials: Option<OpValsString>,
	pub sex: Option<OpValsString>,
}

// -- PatientIdentifier (D.1.1.x)

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct PatientIdentifier {
	pub id: Uuid,
	pub patient_id: Uuid,
	pub sequence_number: i32,
	pub identifier_type_code: String,
	pub identifier_value: Option<String>,
	pub identifier_value_null_flavor: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct PatientIdentifierForCreate {
	pub patient_id: Uuid,
	pub sequence_number: i32,
	pub identifier_type_code: String,
	pub identifier_value: Option<String>,
	pub identifier_value_null_flavor: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct PatientIdentifierForUpdate {
	pub identifier_type_code: Option<String>,
	pub identifier_value: Option<String>,
	pub identifier_value_null_flavor: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct PatientIdentifierFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub patient_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- MedicalHistoryEpisode

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct MedicalHistoryEpisode {
	pub id: Uuid,
	pub patient_id: Uuid,
	pub sequence_number: i32,

	// D.7.1.r.1a - Disease/Surgical Procedure
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,

	// D.7.1.r.2-4
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	pub continuing: Option<bool>,
	pub continuing_null_flavor: Option<String>,
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub comments: Option<String>,
	pub family_history: Option<bool>,
	pub deleted: bool,

	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct MedicalHistoryEpisodeForCreate {
	pub patient_id: Uuid,
	pub sequence_number: i32,
	pub meddra_code: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub continuing_null_flavor: Option<String>,
	pub end_date_null_flavor: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct MedicalHistoryEpisodeForUpdate {
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	pub continuing: Option<bool>,
	pub continuing_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,
	pub comments: Option<String>,
	pub family_history: Option<bool>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct MedicalHistoryEpisodeFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub patient_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- PastDrugHistory

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct PastDrugHistory {
	pub id: Uuid,
	pub patient_id: Uuid,
	pub sequence_number: i32,

	// D.8.r.1 - Drug Name
	pub drug_name: Option<String>,
	pub drug_name_null_flavor: Option<String>,

	// D.8.r.1.KR.1a/b - MFDS product code fields
	pub mfds_medicinal_product_version: Option<String>,
	pub mfds_medicinal_product_id: Option<String>,

	// D.8.r.2-3 - Product IDs
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,

	// D.8.r.4-5 - Dates
	pub start_date: Option<Date>,
	pub start_date_null_flavor: Option<String>,
	pub end_date: Option<Date>,
	pub end_date_null_flavor: Option<String>,

	// D.8.r.6a - Indication
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,

	// D.8.r.7 - Reaction(s)
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
	pub deleted: bool,

	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct PastDrugHistoryForCreate {
	pub patient_id: Uuid,
	pub sequence_number: i32,
	pub drug_name: Option<String>,
	pub drug_name_null_flavor: Option<String>,
	pub mfds_medicinal_product_version: Option<String>,
	pub mfds_medicinal_product_id: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
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
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct PastDrugHistoryForUpdate {
	pub drug_name: Option<String>,
	pub drug_name_null_flavor: Option<String>,
	pub mfds_medicinal_product_version: Option<String>,
	pub mfds_medicinal_product_id: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
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
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,
	pub reaction_meddra_version: Option<String>,
	pub reaction_meddra_code: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct PastDrugHistoryFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub patient_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- PatientDeathInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct PatientDeathInformation {
	pub id: Uuid,
	pub patient_id: Uuid,

	// D.9.1 - Date of Death
	pub date_of_death: Option<Date>,
	pub date_of_death_null_flavor: Option<String>,

	// D.9.3 - Autopsy
	pub autopsy_performed: Option<bool>,
	pub autopsy_performed_null_flavor: Option<String>,

	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct PatientDeathInformationForCreate {
	pub patient_id: Uuid,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_of_death: Option<Date>,
	pub date_of_death_null_flavor: Option<String>,
	pub autopsy_performed: Option<bool>,
	pub autopsy_performed_null_flavor: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct PatientDeathInformationForUpdate {
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_of_death: Option<Date>,
	pub date_of_death_null_flavor: Option<String>,
	pub autopsy_performed: Option<bool>,
	pub autopsy_performed_null_flavor: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct PatientDeathInformationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub patient_id: Option<OpValsValue>,
}

// -- ReportedCauseOfDeath

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ReportedCauseOfDeath {
	pub id: Uuid,
	pub death_info_id: Uuid,
	pub sequence_number: i32,
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	pub comments: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ReportedCauseOfDeathForCreate {
	pub death_info_id: Uuid,
	pub sequence_number: i32,
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	pub comments: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct ReportedCauseOfDeathForUpdate {
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	pub comments: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct ReportedCauseOfDeathFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub death_info_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- AutopsyCauseOfDeath

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct AutopsyCauseOfDeath {
	pub id: Uuid,
	pub death_info_id: Uuid,
	pub sequence_number: i32,
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	pub comments: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct AutopsyCauseOfDeathForCreate {
	pub death_info_id: Uuid,
	pub sequence_number: i32,
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	pub comments: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct AutopsyCauseOfDeathForUpdate {
	pub meddra_version: Option<String>,
	pub meddra_code: Option<String>,
	pub comments: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct AutopsyCauseOfDeathFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub death_info_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- ParentInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ParentInformation {
	pub id: Uuid,
	pub patient_id: Uuid,

	pub parent_identification: Option<String>,
	pub parent_birth_date: Option<Date>,
	pub parent_birth_date_null_flavor: Option<String>,
	pub parent_age: Option<Decimal>,
	pub parent_age_null_flavor: Option<String>,
	pub parent_age_unit: Option<String>,
	pub last_menstrual_period_date: Option<Date>,
	pub last_menstrual_period_date_null_flavor: Option<String>,
	pub weight_kg: Option<Decimal>,
	pub height_cm: Option<Decimal>,
	pub sex: Option<String>,
	pub medical_history_text: Option<String>,
	pub deleted: bool,

	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ParentInformationForCreate {
	pub patient_id: Uuid,
	pub parent_identification: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub parent_birth_date: Option<Date>,
	pub parent_birth_date_null_flavor: Option<String>,
	pub parent_age: Option<Decimal>,
	pub parent_age_null_flavor: Option<String>,
	pub parent_age_unit: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub last_menstrual_period_date: Option<Date>,
	pub last_menstrual_period_date_null_flavor: Option<String>,
	pub weight_kg: Option<Decimal>,
	pub height_cm: Option<Decimal>,
	pub sex: Option<String>,
	pub medical_history_text: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct ParentInformationForUpdate {
	pub parent_identification: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub parent_birth_date: Option<Date>,
	pub parent_birth_date_null_flavor: Option<String>,
	pub parent_age: Option<Decimal>,
	pub parent_age_null_flavor: Option<String>,
	pub parent_age_unit: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub last_menstrual_period_date: Option<Date>,
	pub last_menstrual_period_date_null_flavor: Option<String>,
	pub weight_kg: Option<Decimal>,
	pub height_cm: Option<Decimal>,
	pub sex: Option<String>,
	pub medical_history_text: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct ParentInformationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub patient_id: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- BMCs

pub struct PatientInformationBmc;
impl DbBmc for PatientInformationBmc {
	const TABLE: &'static str = "patient_information";
}

impl PatientInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: PatientInformationForCreate,
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
				case_id,
				patient_initials,
				patient_initials_null_flavor,
				birth_date,
				birth_date_null_flavor,
				age_at_time_of_onset,
				age_at_time_of_onset_null_flavor,
				age_unit,
				gestation_period,
				gestation_period_unit,
				age_group,
				weight_kg,
				weight_kg_null_flavor,
				height_cm,
				height_cm_null_flavor,
				sex,
				sex_null_flavor,
				race_code,
				race_code_null_flavor,
				ethnicity_code,
				ethnicity_code_null_flavor,
				last_menstrual_period_date,
				last_menstrual_period_date_null_flavor,
				medical_history_text,
				medical_history_text_null_flavor,
				concomitant_therapy,
				created_at,
				updated_at,
				created_by
			)
			 VALUES (
			  $1, $2, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
			  $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, now(), now(), $29
			 )
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(data.case_id)
					.bind(data.patient_initials)
					.bind(Option::<String>::None)
					.bind(Option::<String>::None)
					.bind(data.patient_initials_null_flavor)
					.bind(data.birth_date)
					.bind(data.birth_date_null_flavor)
					.bind(data.age_at_time_of_onset)
					.bind(data.age_at_time_of_onset_null_flavor)
					.bind(data.age_unit)
					.bind(data.gestation_period)
					.bind(data.gestation_period_unit)
					.bind(data.age_group)
					.bind(data.weight_kg)
					.bind(data.weight_kg_null_flavor)
					.bind(data.height_cm)
					.bind(data.height_cm_null_flavor)
					.bind(data.sex)
					.bind(data.sex_null_flavor)
					.bind(data.race_code)
					.bind(data.race_code_null_flavor)
					.bind(data.ethnicity_code)
					.bind(data.ethnicity_code_null_flavor)
					.bind(data.last_menstrual_period_date)
					.bind(data.last_menstrual_period_date_null_flavor)
					.bind(data.medical_history_text)
					.bind(data.medical_history_text_null_flavor)
					.bind(data.concomitant_therapy)
					.bind(ctx.user_id()),
			)
			.await?;
		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<PatientInformation> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let sql = format!("SELECT * FROM {} WHERE id = $1", Self::TABLE);
		let result = mm
			.dbx()
			.fetch_optional(sqlx::query_as::<_, PatientInformation>(&sql).bind(id))
			.await;
		match result {
			Ok(Some(patient)) => {
				mm.dbx().commit_txn().await?;
				Ok(patient)
			}
			Ok(None) => {
				let _ = mm.dbx().rollback_txn().await;
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

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<PatientInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<PatientInformation>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PatientInformationForUpdate,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			mm.dbx().rollback_txn().await?;
			return Err(err);
		}

		let sql = format!(
			"UPDATE {}
			 SET patient_initials = CASE WHEN $5 IS NOT NULL THEN NULL ELSE COALESCE($2, patient_initials) END,
			     patient_initials_null_flavor = CASE WHEN $2 IS NOT NULL THEN NULL ELSE COALESCE($5, patient_initials_null_flavor) END,
			     birth_date = CASE WHEN $7 IS NOT NULL THEN NULL ELSE COALESCE($6, birth_date) END,
			     birth_date_null_flavor = CASE WHEN $6 IS NOT NULL THEN NULL ELSE COALESCE($7, birth_date_null_flavor) END,
			     age_at_time_of_onset = CASE WHEN $9 IS NOT NULL THEN NULL ELSE COALESCE($8, age_at_time_of_onset) END,
			     age_at_time_of_onset_null_flavor = CASE WHEN $8 IS NOT NULL THEN NULL ELSE COALESCE($9, age_at_time_of_onset_null_flavor) END,
			     age_unit = CASE WHEN $9 IS NOT NULL THEN NULL ELSE COALESCE($10, age_unit) END,
			     gestation_period = COALESCE($11, gestation_period),
			     gestation_period_unit = COALESCE($12, gestation_period_unit),
			     age_group = COALESCE($13, age_group),
			     weight_kg = CASE WHEN $15 IS NOT NULL THEN NULL ELSE COALESCE($14, weight_kg) END,
			     weight_kg_null_flavor = CASE WHEN $14 IS NOT NULL THEN NULL ELSE COALESCE($15, weight_kg_null_flavor) END,
			     height_cm = CASE WHEN $17 IS NOT NULL THEN NULL ELSE COALESCE($16, height_cm) END,
			     height_cm_null_flavor = CASE WHEN $16 IS NOT NULL THEN NULL ELSE COALESCE($17, height_cm_null_flavor) END,
			     sex = CASE WHEN $19 IS NOT NULL THEN NULL ELSE COALESCE($18, sex) END,
			     sex_null_flavor = CASE WHEN $18 IS NOT NULL THEN NULL ELSE COALESCE($19, sex_null_flavor) END,
			     race_code = CASE WHEN $21 IS NOT NULL THEN NULL ELSE COALESCE($20, race_code) END,
			     race_code_null_flavor = CASE WHEN $20 IS NOT NULL THEN NULL ELSE COALESCE($21, race_code_null_flavor) END,
			     ethnicity_code = CASE WHEN $23 IS NOT NULL THEN NULL ELSE COALESCE($22, ethnicity_code) END,
			     ethnicity_code_null_flavor = CASE WHEN $22 IS NOT NULL THEN NULL ELSE COALESCE($23, ethnicity_code_null_flavor) END,
			     last_menstrual_period_date = CASE WHEN $25 IS NOT NULL THEN NULL ELSE COALESCE($24, last_menstrual_period_date) END,
			     last_menstrual_period_date_null_flavor = CASE WHEN $24 IS NOT NULL THEN NULL ELSE COALESCE($25, last_menstrual_period_date_null_flavor) END,
			     medical_history_text = CASE WHEN $27 IS NOT NULL THEN NULL ELSE COALESCE($26, medical_history_text) END,
			     medical_history_text_null_flavor = CASE WHEN $26 IS NOT NULL THEN NULL ELSE COALESCE($27, medical_history_text_null_flavor) END,
			     concomitant_therapy = COALESCE($28, concomitant_therapy),
			     updated_at = now(),
			     updated_by = $29
			 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(data.patient_initials)
					.bind(Option::<String>::None)
					.bind(Option::<String>::None)
					.bind(data.patient_initials_null_flavor)
					.bind(data.birth_date)
					.bind(data.birth_date_null_flavor)
					.bind(data.age_at_time_of_onset)
					.bind(data.age_at_time_of_onset_null_flavor)
					.bind(data.age_unit)
					.bind(data.gestation_period)
					.bind(data.gestation_period_unit)
					.bind(data.age_group)
					.bind(data.weight_kg)
					.bind(data.weight_kg_null_flavor)
					.bind(data.height_cm)
					.bind(data.height_cm_null_flavor)
					.bind(data.sex)
					.bind(data.sex_null_flavor)
					.bind(data.race_code)
					.bind(data.race_code_null_flavor)
					.bind(data.ethnicity_code)
					.bind(data.ethnicity_code_null_flavor)
					.bind(data.last_menstrual_period_date)
					.bind(data.last_menstrual_period_date_null_flavor)
					.bind(data.medical_history_text)
					.bind(data.medical_history_text_null_flavor)
					.bind(data.concomitant_therapy)
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
			return Err(crate::model::Error::EntityNotFound {
				entity: Self::TABLE,
				id: 0,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn get_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<PatientInformation> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let sql = format!("SELECT * FROM {} WHERE case_id = $1", Self::TABLE);
		let result = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, PatientInformation>(&sql).bind(case_id),
			)
			.await;
		match result {
			Ok(Some(patient)) => {
				mm.dbx().commit_txn().await?;
				Ok(patient)
			}
			Ok(None) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(crate::model::Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id: case_id,
				})
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn update_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		data: PatientInformationForUpdate,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			mm.dbx().rollback_txn().await?;
			return Err(err);
		}

		let sql = format!(
			"UPDATE {}
			 SET patient_initials = CASE WHEN $5 IS NOT NULL THEN NULL ELSE COALESCE($2, patient_initials) END,
			     patient_initials_null_flavor = CASE WHEN $2 IS NOT NULL THEN NULL ELSE COALESCE($5, patient_initials_null_flavor) END,
			     birth_date = CASE WHEN $7 IS NOT NULL THEN NULL ELSE COALESCE($6, birth_date) END,
			     birth_date_null_flavor = CASE WHEN $6 IS NOT NULL THEN NULL ELSE COALESCE($7, birth_date_null_flavor) END,
			     age_at_time_of_onset = CASE WHEN $9 IS NOT NULL THEN NULL ELSE COALESCE($8, age_at_time_of_onset) END,
			     age_at_time_of_onset_null_flavor = CASE WHEN $8 IS NOT NULL THEN NULL ELSE COALESCE($9, age_at_time_of_onset_null_flavor) END,
			     age_unit = CASE WHEN $9 IS NOT NULL THEN NULL ELSE COALESCE($10, age_unit) END,
			     gestation_period = COALESCE($11, gestation_period),
			     gestation_period_unit = COALESCE($12, gestation_period_unit),
			     age_group = COALESCE($13, age_group),
			     weight_kg = CASE WHEN $15 IS NOT NULL THEN NULL ELSE COALESCE($14, weight_kg) END,
			     weight_kg_null_flavor = CASE WHEN $14 IS NOT NULL THEN NULL ELSE COALESCE($15, weight_kg_null_flavor) END,
			     height_cm = CASE WHEN $17 IS NOT NULL THEN NULL ELSE COALESCE($16, height_cm) END,
			     height_cm_null_flavor = CASE WHEN $16 IS NOT NULL THEN NULL ELSE COALESCE($17, height_cm_null_flavor) END,
			     sex = CASE WHEN $19 IS NOT NULL THEN NULL ELSE COALESCE($18, sex) END,
			     sex_null_flavor = CASE WHEN $18 IS NOT NULL THEN NULL ELSE COALESCE($19, sex_null_flavor) END,
			     race_code = CASE WHEN $21 IS NOT NULL THEN NULL ELSE COALESCE($20, race_code) END,
			     race_code_null_flavor = CASE WHEN $20 IS NOT NULL THEN NULL ELSE COALESCE($21, race_code_null_flavor) END,
			     ethnicity_code = CASE WHEN $23 IS NOT NULL THEN NULL ELSE COALESCE($22, ethnicity_code) END,
			     ethnicity_code_null_flavor = CASE WHEN $22 IS NOT NULL THEN NULL ELSE COALESCE($23, ethnicity_code_null_flavor) END,
			     last_menstrual_period_date = CASE WHEN $25 IS NOT NULL THEN NULL ELSE COALESCE($24, last_menstrual_period_date) END,
			     last_menstrual_period_date_null_flavor = CASE WHEN $24 IS NOT NULL THEN NULL ELSE COALESCE($25, last_menstrual_period_date_null_flavor) END,
			     medical_history_text = CASE WHEN $27 IS NOT NULL THEN NULL ELSE COALESCE($26, medical_history_text) END,
			     medical_history_text_null_flavor = CASE WHEN $26 IS NOT NULL THEN NULL ELSE COALESCE($27, medical_history_text_null_flavor) END,
			     concomitant_therapy = COALESCE($28, concomitant_therapy),
			     updated_at = now(),
			     updated_by = $29
			 WHERE case_id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(case_id)
					.bind(data.patient_initials)
					.bind(Option::<String>::None)
					.bind(Option::<String>::None)
					.bind(data.patient_initials_null_flavor)
					.bind(data.birth_date)
					.bind(data.birth_date_null_flavor)
					.bind(data.age_at_time_of_onset)
					.bind(data.age_at_time_of_onset_null_flavor)
					.bind(data.age_unit)
					.bind(data.gestation_period)
					.bind(data.gestation_period_unit)
					.bind(data.age_group)
					.bind(data.weight_kg)
					.bind(data.weight_kg_null_flavor)
					.bind(data.height_cm)
					.bind(data.height_cm_null_flavor)
					.bind(data.sex)
					.bind(data.sex_null_flavor)
					.bind(data.race_code)
					.bind(data.race_code_null_flavor)
					.bind(data.ethnicity_code)
					.bind(data.ethnicity_code_null_flavor)
					.bind(data.last_menstrual_period_date)
					.bind(data.last_menstrual_period_date_null_flavor)
					.bind(data.medical_history_text)
					.bind(data.medical_history_text_null_flavor)
					.bind(data.concomitant_therapy)
					.bind(ctx.user_id()),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id: case_id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!("DELETE FROM {} WHERE case_id = $1", Self::TABLE);
		let result = mm.dbx().execute(sqlx::query(&sql).bind(case_id)).await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id: case_id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}
}

pub struct PatientIdentifierBmc;
impl DbBmc for PatientIdentifierBmc {
	const TABLE: &'static str = "patient_identifiers";
}

impl PatientIdentifierBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: PatientIdentifierForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<PatientIdentifier> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<PatientIdentifierFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<PatientIdentifier>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(PatientIdentifierFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PatientIdentifierForUpdate,
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
			"UPDATE {} SET
			 identifier_type_code = COALESCE($1, identifier_type_code),
			 identifier_value = CASE
			  WHEN $3::varchar IS NOT NULL THEN NULL
			  ELSE COALESCE($2, identifier_value)
			 END,
			 identifier_value_null_flavor = CASE
			  WHEN $3::varchar IS NOT NULL THEN $3
			  WHEN $2::varchar IS NOT NULL THEN NULL
			  ELSE identifier_value_null_flavor
			 END,
			 updated_at = now(),
			 updated_by = $4
			 WHERE id = $5",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(data.identifier_type_code)
					.bind(data.identifier_value)
					.bind(data.identifier_value_null_flavor)
					.bind(ctx.user_id())
					.bind(id),
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
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}

pub struct MedicalHistoryEpisodeBmc;
impl DbBmc for MedicalHistoryEpisodeBmc {
	const TABLE: &'static str = "medical_history_episodes";
}

impl MedicalHistoryEpisodeBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: MedicalHistoryEpisodeForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<MedicalHistoryEpisode> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<MedicalHistoryEpisodeFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<MedicalHistoryEpisode>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(MedicalHistoryEpisodeFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: MedicalHistoryEpisodeForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}

pub struct PastDrugHistoryBmc;
impl DbBmc for PastDrugHistoryBmc {
	const TABLE: &'static str = "past_drug_history";
}

impl PastDrugHistoryBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: PastDrugHistoryForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<PastDrugHistory> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<PastDrugHistoryFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<PastDrugHistory>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(PastDrugHistoryFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PastDrugHistoryForUpdate,
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
			"UPDATE {} SET
			 drug_name = CASE WHEN $1::varchar IS NOT NULL THEN NULL ELSE COALESCE($2, drug_name) END,
			 drug_name_null_flavor = CASE WHEN $2::varchar IS NOT NULL THEN NULL ELSE COALESCE($1, drug_name_null_flavor) END,
			 mfds_medicinal_product_version = COALESCE($3, mfds_medicinal_product_version),
			 mfds_medicinal_product_id = COALESCE($4, mfds_medicinal_product_id),
			 mpid = COALESCE($5, mpid),
			 mpid_version = COALESCE($6, mpid_version),
			 phpid = COALESCE($7, phpid),
			 phpid_version = COALESCE($8, phpid_version),
			 start_date = CASE WHEN $10::varchar IS NOT NULL THEN NULL ELSE COALESCE($9, start_date) END,
			 start_date_null_flavor = CASE WHEN $9::date IS NOT NULL THEN NULL ELSE COALESCE($10, start_date_null_flavor) END,
			 end_date = CASE WHEN $12::varchar IS NOT NULL THEN NULL ELSE COALESCE($11, end_date) END,
			 end_date_null_flavor = CASE WHEN $11::date IS NOT NULL THEN NULL ELSE COALESCE($12, end_date_null_flavor) END,
			 indication_meddra_version = COALESCE($13, indication_meddra_version),
			 indication_meddra_code = COALESCE($14, indication_meddra_code),
			 reaction_meddra_version = COALESCE($15, reaction_meddra_version),
			 reaction_meddra_code = COALESCE($16, reaction_meddra_code),
			 updated_at = now(),
			 updated_by = $17
			 WHERE id = $18",
			Self::TABLE
		);

		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(data.drug_name_null_flavor)
					.bind(data.drug_name)
					.bind(data.mfds_medicinal_product_version)
					.bind(data.mfds_medicinal_product_id)
					.bind(data.mpid)
					.bind(data.mpid_version)
					.bind(data.phpid)
					.bind(data.phpid_version)
					.bind(data.start_date)
					.bind(data.start_date_null_flavor)
					.bind(data.end_date)
					.bind(data.end_date_null_flavor)
					.bind(data.indication_meddra_version)
					.bind(data.indication_meddra_code)
					.bind(data.reaction_meddra_version)
					.bind(data.reaction_meddra_code)
					.bind(ctx.user_id())
					.bind(id),
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
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}

pub struct PatientDeathInformationBmc;
impl DbBmc for PatientDeathInformationBmc {
	const TABLE: &'static str = "patient_death_information";
}

impl PatientDeathInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: PatientDeathInformationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<PatientDeathInformation> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<PatientDeathInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<PatientDeathInformation>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PatientDeathInformationForUpdate,
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
			"UPDATE {} SET
			 date_of_death = CASE
			 	WHEN $1::varchar IS NOT NULL THEN NULL
			 	ELSE COALESCE($2, date_of_death)
			 END,
			 date_of_death_null_flavor = CASE
			 	WHEN $1::varchar IS NOT NULL THEN $1
			 	WHEN $2::date IS NOT NULL THEN NULL
			 	ELSE date_of_death_null_flavor
			 END,
			 autopsy_performed = CASE
			  WHEN $3::varchar IS NOT NULL THEN NULL
			  ELSE COALESCE($4, autopsy_performed)
			 END,
			 autopsy_performed_null_flavor = CASE
			  WHEN $3::varchar IS NOT NULL THEN $3
			  WHEN $4::boolean IS NOT NULL THEN NULL
			  ELSE autopsy_performed_null_flavor
			 END,
			 updated_at = now(),
			 updated_by = $5
			 WHERE id = $6",
			Self::TABLE
		);

		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(data.date_of_death_null_flavor)
					.bind(data.date_of_death)
					.bind(data.autopsy_performed_null_flavor)
					.bind(data.autopsy_performed)
					.bind(ctx.user_id())
					.bind(id),
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
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

pub struct ReportedCauseOfDeathBmc;
impl DbBmc for ReportedCauseOfDeathBmc {
	const TABLE: &'static str = "reported_causes_of_death";
}

impl ReportedCauseOfDeathBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ReportedCauseOfDeathForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ReportedCauseOfDeath> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<ReportedCauseOfDeathFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ReportedCauseOfDeath>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(ReportedCauseOfDeathFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ReportedCauseOfDeathForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}

pub struct AutopsyCauseOfDeathBmc;
impl DbBmc for AutopsyCauseOfDeathBmc {
	const TABLE: &'static str = "autopsy_causes_of_death";
}

impl AutopsyCauseOfDeathBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: AutopsyCauseOfDeathForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<AutopsyCauseOfDeath> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<AutopsyCauseOfDeathFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<AutopsyCauseOfDeath>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(AutopsyCauseOfDeathFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: AutopsyCauseOfDeathForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}

pub struct ParentInformationBmc;
impl DbBmc for ParentInformationBmc {
	const TABLE: &'static str = "parent_information";
}

impl ParentInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ParentInformationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ParentInformation> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<ParentInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ParentInformation>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(ParentInformationFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ParentInformationForUpdate,
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
			"UPDATE {} SET
			 parent_identification = COALESCE($1, parent_identification),
			 parent_birth_date = CASE
			 	WHEN $2::varchar IS NOT NULL THEN NULL
			 	ELSE COALESCE($3, parent_birth_date)
			 END,
			 parent_birth_date_null_flavor = CASE
			 	WHEN $2::varchar IS NOT NULL THEN $2
			 	WHEN $3::date IS NOT NULL THEN NULL
			 	ELSE parent_birth_date_null_flavor
			 END,
			 parent_age = CASE
			 	WHEN $4::varchar IS NOT NULL THEN NULL
			 	ELSE COALESCE($5, parent_age)
			 END,
			 parent_age_null_flavor = CASE
			 	WHEN $4::varchar IS NOT NULL THEN $4
			 	WHEN $5::numeric IS NOT NULL THEN NULL
			 	ELSE parent_age_null_flavor
			 END,
			 parent_age_unit = CASE
			 	WHEN $4::varchar IS NOT NULL THEN NULL
			 	ELSE COALESCE($6, parent_age_unit)
			 END,
			 last_menstrual_period_date = CASE
			 	WHEN $7::varchar IS NOT NULL THEN NULL
			 	ELSE COALESCE($8, last_menstrual_period_date)
			 END,
			 last_menstrual_period_date_null_flavor = CASE
			 	WHEN $7::varchar IS NOT NULL THEN $7
			 	WHEN $8::date IS NOT NULL THEN NULL
			 	ELSE last_menstrual_period_date_null_flavor
			 END,
			 weight_kg = COALESCE($9, weight_kg),
			 height_cm = COALESCE($10, height_cm),
			 sex = COALESCE($11, sex),
			 medical_history_text = COALESCE($12, medical_history_text),
			 updated_at = now(),
			 updated_by = $13
			 WHERE id = $14",
			Self::TABLE
		);

		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(data.parent_identification)
					.bind(data.parent_birth_date_null_flavor)
					.bind(data.parent_birth_date)
					.bind(data.parent_age_null_flavor)
					.bind(data.parent_age)
					.bind(data.parent_age_unit)
					.bind(data.last_menstrual_period_date_null_flavor)
					.bind(data.last_menstrual_period_date)
					.bind(data.weight_kg)
					.bind(data.height_cm)
					.bind(data.sex)
					.bind(data.medical_history_text)
					.bind(ctx.user_id())
					.bind(id),
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
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}
