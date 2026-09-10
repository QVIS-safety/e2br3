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

fn deserialize_patch_decimal<'de, D>(
	deserializer: D,
) -> std::result::Result<Option<Option<Decimal>>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	Option::<Decimal>::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrManyStrings {
	One(String),
	Many(Vec<String>),
}

fn deserialize_string_vec<'de, D>(
	deserializer: D,
) -> std::result::Result<Vec<String>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	Ok(
		match Option::<OneOrManyStrings>::deserialize(deserializer)? {
			Some(OneOrManyStrings::One(value)) => vec![value],
			Some(OneOrManyStrings::Many(values)) => values,
			None => Vec::new(),
		},
	)
}

fn deserialize_patch_string_vec<'de, D>(
	deserializer: D,
) -> std::result::Result<Option<Vec<String>>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	Ok(
		match Option::<OneOrManyStrings>::deserialize(deserializer)? {
			Some(OneOrManyStrings::One(value)) => Some(vec![value]),
			Some(OneOrManyStrings::Many(values)) => Some(values),
			None => None,
		},
	)
}

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
	pub height_cm: Option<Decimal>,
	pub sex: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	pub birth_date_null_flavor: Option<String>,
	pub sex_null_flavor: Option<String>,

	// FDA.D.11.r.1 / FDA.D.12 - Race (repeating) / Ethnicity (FDA)
	pub race_codes: Vec<String>,
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

#[derive(Default, Fields, Deserialize)]
#[serde(deny_unknown_fields)]
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
	pub age_unit: Option<String>,
	pub gestation_period: Option<Decimal>,
	pub gestation_period_unit: Option<String>,
	pub age_group: Option<String>,
	pub weight_kg: Option<Decimal>,
	pub height_cm: Option<Decimal>,
	pub sex: Option<String>,
	pub sex_null_flavor: Option<String>,
	#[serde(
		default,
		alias = "race_code",
		deserialize_with = "deserialize_string_vec"
	)]
	pub race_codes: Vec<String>,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatientInformationForUpdate {
	pub patient_initials: Option<String>,
	pub patient_initials_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub birth_date: Option<Date>,
	pub birth_date_null_flavor: Option<String>,
	#[serde(default, deserialize_with = "deserialize_patch_decimal")]
	pub age_at_time_of_onset: Option<Option<Decimal>>,
	pub age_unit: Option<String>,
	pub gestation_period: Option<Decimal>,
	pub gestation_period_unit: Option<String>,
	pub age_group: Option<String>,
	#[serde(default, deserialize_with = "deserialize_patch_decimal")]
	pub weight_kg: Option<Option<Decimal>>,
	#[serde(default, deserialize_with = "deserialize_patch_decimal")]
	pub height_cm: Option<Option<Decimal>>,
	pub sex: Option<String>,
	pub sex_null_flavor: Option<String>,
	#[serde(
		default,
		alias = "race_code",
		deserialize_with = "deserialize_patch_string_vec"
	)]
	pub race_codes: Option<Vec<String>>,
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
	pub start_date: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub continuing: Option<bool>,
	pub continuing_null_flavor: Option<String>,
	pub end_date: Option<String>,
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
	pub start_date: Option<String>,
	pub start_date_null_flavor: Option<String>,
	pub continuing: Option<bool>,
	pub continuing_null_flavor: Option<String>,
	pub end_date: Option<String>,
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
	#[serde(skip_serializing)]
	pub mpid_source_code_system: Option<String>,
	#[serde(skip_serializing)]
	pub mpid_source_code_system_version: Option<String>,
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
	#[serde(skip_deserializing)]
	pub mpid_source_code_system: Option<String>,
	#[serde(skip_deserializing)]
	pub mpid_source_code_system_version: Option<String>,
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
	#[serde(skip_deserializing)]
	pub mpid_source_code_system: Option<String>,
	#[serde(skip_deserializing)]
	pub mpid_source_code_system_version: Option<String>,
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
	pub parent_identification_null_flavor: Option<String>,
	pub parent_birth_date: Option<Date>,
	pub parent_birth_date_null_flavor: Option<String>,
	pub parent_age: Option<Decimal>,
	pub parent_age_unit: Option<String>,
	pub last_menstrual_period_date: Option<Date>,
	pub last_menstrual_period_date_null_flavor: Option<String>,
	pub weight_kg: Option<Decimal>,
	pub height_cm: Option<Decimal>,
	pub sex: Option<String>,
	pub sex_null_flavor: Option<String>,
	pub medical_history_text: Option<String>,
	pub deleted: bool,

	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParentInformationForCreate {
	pub patient_id: Uuid,
	pub parent_identification: Option<String>,
	pub parent_identification_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub parent_birth_date: Option<Date>,
	pub parent_birth_date_null_flavor: Option<String>,
	pub parent_age: Option<Decimal>,
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
	pub sex_null_flavor: Option<String>,
	pub medical_history_text: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParentInformationForUpdate {
	pub parent_identification: Option<String>,
	pub parent_identification_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub parent_birth_date: Option<Date>,
	pub parent_birth_date_null_flavor: Option<String>,
	#[serde(default, deserialize_with = "deserialize_patch_decimal")]
	pub parent_age: Option<Option<Decimal>>,
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
	pub sex_null_flavor: Option<String>,
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
	/// Used by case-editor child saves; preserves an existing patient's values.
	pub async fn get_or_create_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Uuid> {
		match Self::get_by_case(ctx, mm, case_id).await {
			Ok(patient) => Ok(patient.id),
			Err(crate::model::Error::EntityUuidNotFound { .. }) => {
				Self::create(
					ctx,
					mm,
					PatientInformationForCreate {
						case_id,
						..Default::default()
					},
				)
				.await
			}
			Err(error) => Err(error),
		}
	}

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
				age_unit,
				gestation_period,
				gestation_period_unit,
				age_group,
				weight_kg,
				height_cm,
				sex,
				sex_null_flavor,
				race_codes,
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
			  $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
			  $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, now(), now(), $24
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
					.bind(data.patient_initials_null_flavor)
					.bind(data.birth_date)
					.bind(data.birth_date_null_flavor)
					.bind(data.age_at_time_of_onset)
					.bind(data.age_unit)
					.bind(data.gestation_period)
					.bind(data.gestation_period_unit)
					.bind(data.age_group)
					.bind(data.weight_kg)
					.bind(data.height_cm)
					.bind(data.sex)
					.bind(data.sex_null_flavor)
					.bind(data.race_codes)
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
		Self::update_patch(ctx, mm, id, data, &[]).await
	}

	pub async fn update_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PatientInformationForUpdate,
		clear_fields: &[&str],
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			mm.dbx().rollback_txn().await?;
			return Err(err);
		}
		let age_clear = matches!(data.age_at_time_of_onset, Some(None))
			|| clear_fields.contains(&"age_at_time_of_onset");
		let weight_clear = matches!(data.weight_kg, Some(None))
			|| clear_fields.contains(&"weight_kg");
		let height_clear = matches!(data.height_cm, Some(None))
			|| clear_fields.contains(&"height_cm");
		let age_at_time_of_onset = data.age_at_time_of_onset.flatten();
		let weight_kg = data.weight_kg.flatten();
		let height_cm = data.height_cm.flatten();
		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE patient_information SET
					 age_at_time_of_onset = CASE WHEN $2 THEN NULL ELSE age_at_time_of_onset END,
					 weight_kg = CASE WHEN $3 THEN NULL ELSE weight_kg END,
					 height_cm = CASE WHEN $4 THEN NULL ELSE height_cm END
					 WHERE id = $1",
				)
				.bind(id)
				.bind(age_clear)
				.bind(weight_clear)
				.bind(height_clear),
			)
			.await?;
		let clears: Vec<String> =
			clear_fields.iter().map(|field| (*field).into()).collect();

		let sql = format!(
			"UPDATE {}
			 SET patient_initials = CASE WHEN 'patient_initials' = ANY($25) THEN NULL ELSE CASE WHEN $3 IS NOT NULL THEN NULL ELSE COALESCE($2, patient_initials) END END,
			     patient_initials_null_flavor = CASE WHEN 'patient_initials_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $2 IS NOT NULL THEN NULL ELSE COALESCE($3, patient_initials_null_flavor) END END,
			     birth_date = CASE WHEN 'birth_date' = ANY($25) THEN NULL ELSE CASE WHEN $5 IS NOT NULL THEN NULL ELSE COALESCE($4, birth_date) END END,
			     birth_date_null_flavor = CASE WHEN 'birth_date_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $4 IS NOT NULL THEN NULL ELSE COALESCE($5, birth_date_null_flavor) END END,
			     age_at_time_of_onset = CASE WHEN 'age_at_time_of_onset' = ANY($25) THEN NULL ELSE COALESCE($6, age_at_time_of_onset) END,
			     age_unit = CASE WHEN 'age_unit' = ANY($25) THEN NULL ELSE COALESCE($7, age_unit) END,
			     gestation_period = CASE WHEN 'gestation_period' = ANY($25) THEN NULL ELSE COALESCE($8, gestation_period) END,
			     gestation_period_unit = CASE WHEN 'gestation_period_unit' = ANY($25) THEN NULL ELSE COALESCE($9, gestation_period_unit) END,
			     age_group = CASE WHEN 'age_group' = ANY($25) THEN NULL ELSE COALESCE($10, age_group) END,
			     weight_kg = CASE WHEN 'weight_kg' = ANY($25) THEN NULL ELSE COALESCE($11, weight_kg) END,
			     height_cm = CASE WHEN 'height_cm' = ANY($25) THEN NULL ELSE COALESCE($12, height_cm) END,
			     sex = CASE WHEN 'sex' = ANY($25) THEN NULL ELSE CASE WHEN $14 IS NOT NULL THEN NULL ELSE COALESCE($13, sex) END END,
			     sex_null_flavor = CASE WHEN 'sex_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $13 IS NOT NULL THEN NULL ELSE COALESCE($14, sex_null_flavor) END END,
			     race_codes = CASE WHEN 'race_codes' = ANY($25) THEN NULL ELSE CASE WHEN $16 IS NOT NULL THEN '{{}}'::VARCHAR(10)[] ELSE COALESCE($15, race_codes) END END,
			     race_code_null_flavor = CASE WHEN 'race_code_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN COALESCE(cardinality($15), 0) > 0 THEN NULL ELSE COALESCE($16, race_code_null_flavor) END END,
			     ethnicity_code = CASE WHEN 'ethnicity_code' = ANY($25) THEN NULL ELSE CASE WHEN $18 IS NOT NULL THEN NULL ELSE COALESCE($17, ethnicity_code) END END,
			     ethnicity_code_null_flavor = CASE WHEN 'ethnicity_code_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $17 IS NOT NULL THEN NULL ELSE COALESCE($18, ethnicity_code_null_flavor) END END,
			     last_menstrual_period_date = CASE WHEN 'last_menstrual_period_date' = ANY($25) THEN NULL ELSE CASE WHEN $20 IS NOT NULL THEN NULL ELSE COALESCE($19, last_menstrual_period_date) END END,
			     last_menstrual_period_date_null_flavor = CASE WHEN 'last_menstrual_period_date_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $19 IS NOT NULL THEN NULL ELSE COALESCE($20, last_menstrual_period_date_null_flavor) END END,
			     medical_history_text = CASE WHEN 'medical_history_text' = ANY($25) THEN NULL ELSE CASE WHEN $22 IS NOT NULL THEN NULL ELSE COALESCE($21, medical_history_text) END END,
			     medical_history_text_null_flavor = CASE WHEN 'medical_history_text_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $21 IS NOT NULL THEN NULL ELSE COALESCE($22, medical_history_text_null_flavor) END END,
			     concomitant_therapy = CASE WHEN 'concomitant_therapy' = ANY($25) THEN NULL ELSE COALESCE($23, concomitant_therapy) END,
			     updated_at = now(),
			     updated_by = $24
			 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(data.patient_initials)
					.bind(data.patient_initials_null_flavor)
					.bind(data.birth_date)
					.bind(data.birth_date_null_flavor)
					.bind(age_at_time_of_onset)
					.bind(data.age_unit)
					.bind(data.gestation_period)
					.bind(data.gestation_period_unit)
					.bind(data.age_group)
					.bind(weight_kg)
					.bind(height_cm)
					.bind(data.sex)
					.bind(data.sex_null_flavor)
					.bind(data.race_codes)
					.bind(data.race_code_null_flavor)
					.bind(data.ethnicity_code)
					.bind(data.ethnicity_code_null_flavor)
					.bind(data.last_menstrual_period_date)
					.bind(data.last_menstrual_period_date_null_flavor)
					.bind(data.medical_history_text)
					.bind(data.medical_history_text_null_flavor)
					.bind(data.concomitant_therapy)
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
		Self::update_by_case_patch(ctx, mm, case_id, data, &[]).await
	}

	pub async fn update_by_case_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		data: PatientInformationForUpdate,
		clear_fields: &[&str],
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			mm.dbx().rollback_txn().await?;
			return Err(err);
		}
		let age_clear = matches!(data.age_at_time_of_onset, Some(None))
			|| clear_fields.contains(&"age_at_time_of_onset");
		let weight_clear = matches!(data.weight_kg, Some(None))
			|| clear_fields.contains(&"weight_kg");
		let height_clear = matches!(data.height_cm, Some(None))
			|| clear_fields.contains(&"height_cm");
		let age_at_time_of_onset = data.age_at_time_of_onset.flatten();
		let weight_kg = data.weight_kg.flatten();
		let height_cm = data.height_cm.flatten();
		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE patient_information SET
					 age_at_time_of_onset = CASE WHEN $2 THEN NULL ELSE age_at_time_of_onset END,
					 weight_kg = CASE WHEN $3 THEN NULL ELSE weight_kg END,
					 height_cm = CASE WHEN $4 THEN NULL ELSE height_cm END
					 WHERE case_id = $1",
				)
				.bind(case_id)
				.bind(age_clear)
				.bind(weight_clear)
				.bind(height_clear),
			)
			.await?;
		let clears: Vec<String> =
			clear_fields.iter().map(|field| (*field).into()).collect();

		let sql = format!(
			"UPDATE {}
			 SET patient_initials = CASE WHEN 'patient_initials' = ANY($25) THEN NULL ELSE CASE WHEN $3 IS NOT NULL THEN NULL ELSE COALESCE($2, patient_initials) END END,
			     patient_initials_null_flavor = CASE WHEN 'patient_initials_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $2 IS NOT NULL THEN NULL ELSE COALESCE($3, patient_initials_null_flavor) END END,
			     birth_date = CASE WHEN 'birth_date' = ANY($25) THEN NULL ELSE CASE WHEN $5 IS NOT NULL THEN NULL ELSE COALESCE($4, birth_date) END END,
			     birth_date_null_flavor = CASE WHEN 'birth_date_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $4 IS NOT NULL THEN NULL ELSE COALESCE($5, birth_date_null_flavor) END END,
			     age_at_time_of_onset = CASE WHEN 'age_at_time_of_onset' = ANY($25) THEN NULL ELSE COALESCE($6, age_at_time_of_onset) END,
			     age_unit = CASE WHEN 'age_unit' = ANY($25) THEN NULL ELSE COALESCE($7, age_unit) END,
			     gestation_period = CASE WHEN 'gestation_period' = ANY($25) THEN NULL ELSE COALESCE($8, gestation_period) END,
			     gestation_period_unit = CASE WHEN 'gestation_period_unit' = ANY($25) THEN NULL ELSE COALESCE($9, gestation_period_unit) END,
			     age_group = CASE WHEN 'age_group' = ANY($25) THEN NULL ELSE COALESCE($10, age_group) END,
			     weight_kg = CASE WHEN 'weight_kg' = ANY($25) THEN NULL ELSE COALESCE($11, weight_kg) END,
			     height_cm = CASE WHEN 'height_cm' = ANY($25) THEN NULL ELSE COALESCE($12, height_cm) END,
			     sex = CASE WHEN 'sex' = ANY($25) THEN NULL ELSE CASE WHEN $14 IS NOT NULL THEN NULL ELSE COALESCE($13, sex) END END,
			     sex_null_flavor = CASE WHEN 'sex_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $13 IS NOT NULL THEN NULL ELSE COALESCE($14, sex_null_flavor) END END,
			     race_codes = CASE WHEN 'race_codes' = ANY($25) THEN NULL ELSE CASE WHEN $16 IS NOT NULL THEN '{{}}'::VARCHAR(10)[] ELSE COALESCE($15, race_codes) END END,
			     race_code_null_flavor = CASE WHEN 'race_code_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN COALESCE(cardinality($15), 0) > 0 THEN NULL ELSE COALESCE($16, race_code_null_flavor) END END,
			     ethnicity_code = CASE WHEN 'ethnicity_code' = ANY($25) THEN NULL ELSE CASE WHEN $18 IS NOT NULL THEN NULL ELSE COALESCE($17, ethnicity_code) END END,
			     ethnicity_code_null_flavor = CASE WHEN 'ethnicity_code_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $17 IS NOT NULL THEN NULL ELSE COALESCE($18, ethnicity_code_null_flavor) END END,
			     last_menstrual_period_date = CASE WHEN 'last_menstrual_period_date' = ANY($25) THEN NULL ELSE CASE WHEN $20 IS NOT NULL THEN NULL ELSE COALESCE($19, last_menstrual_period_date) END END,
			     last_menstrual_period_date_null_flavor = CASE WHEN 'last_menstrual_period_date_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $19 IS NOT NULL THEN NULL ELSE COALESCE($20, last_menstrual_period_date_null_flavor) END END,
			     medical_history_text = CASE WHEN 'medical_history_text' = ANY($25) THEN NULL ELSE CASE WHEN $22 IS NOT NULL THEN NULL ELSE COALESCE($21, medical_history_text) END END,
			     medical_history_text_null_flavor = CASE WHEN 'medical_history_text_null_flavor' = ANY($25) THEN NULL ELSE CASE WHEN $21 IS NOT NULL THEN NULL ELSE COALESCE($22, medical_history_text_null_flavor) END END,
			     concomitant_therapy = CASE WHEN 'concomitant_therapy' = ANY($25) THEN NULL ELSE COALESCE($23, concomitant_therapy) END,
			     updated_at = now(),
			     updated_by = $24
			 WHERE case_id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(case_id)
					.bind(data.patient_initials)
					.bind(data.patient_initials_null_flavor)
					.bind(data.birth_date)
					.bind(data.birth_date_null_flavor)
					.bind(age_at_time_of_onset)
					.bind(data.age_unit)
					.bind(data.gestation_period)
					.bind(data.gestation_period_unit)
					.bind(data.age_group)
					.bind(weight_kg)
					.bind(height_cm)
					.bind(data.sex)
					.bind(data.sex_null_flavor)
					.bind(data.race_codes)
					.bind(data.race_code_null_flavor)
					.bind(data.ethnicity_code)
					.bind(data.ethnicity_code_null_flavor)
					.bind(data.last_menstrual_period_date)
					.bind(data.last_menstrual_period_date_null_flavor)
					.bind(data.medical_history_text)
					.bind(data.medical_history_text_null_flavor)
					.bind(data.concomitant_therapy)
					.bind(ctx.user_id())
					.bind(clears),
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
		if filters.is_empty() {
			filters.push(PatientIdentifierFilter::default());
		}
		for filter in &mut filters {
			filter.deleted = Some(OpValBool::Eq(false).into());
		}
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PatientIdentifierForUpdate,
	) -> Result<()> {
		Self::update_patch(ctx, mm, id, data, &[]).await
	}

	pub async fn update_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PatientIdentifierForUpdate,
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

		let sql = format!(
			"UPDATE {} SET
			 identifier_type_code = COALESCE($1, identifier_type_code),
			 identifier_value = CASE
			  WHEN 'identifier_value' = ANY($4::text[]) THEN NULL
			  WHEN $3::varchar IS NOT NULL THEN NULL
			  ELSE COALESCE($2, identifier_value)
			 END,
			 identifier_value_null_flavor = CASE
			  WHEN 'identifier_value_null_flavor' = ANY($4::text[]) THEN NULL
			  WHEN $3::varchar IS NOT NULL THEN $3
			  WHEN $2::varchar IS NOT NULL THEN NULL
			  ELSE identifier_value_null_flavor
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
					.bind(data.identifier_type_code)
					.bind(data.identifier_value)
					.bind(data.identifier_value_null_flavor)
					.bind(clear_fields)
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
		if filters.is_empty() {
			filters.push(MedicalHistoryEpisodeFilter::default());
		}
		for filter in &mut filters {
			filter.deleted = Some(OpValBool::Eq(false).into());
		}
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
		if filters.is_empty() {
			filters.push(PastDrugHistoryFilter::default());
		}
		for filter in &mut filters {
			filter.deleted = Some(OpValBool::Eq(false).into());
		}
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PastDrugHistoryForUpdate,
	) -> Result<()> {
		Self::update_patch(ctx, mm, id, data, &[]).await
	}

	pub async fn update_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PastDrugHistoryForUpdate,
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
			"UPDATE {} SET
			 drug_name = CASE WHEN 'drug_name' = ANY($21) THEN NULL ELSE CASE WHEN $1::varchar IS NOT NULL THEN NULL ELSE COALESCE($2, drug_name) END END,
			 drug_name_null_flavor = CASE WHEN 'drug_name_null_flavor' = ANY($21) THEN NULL ELSE CASE WHEN $2::varchar IS NOT NULL THEN NULL ELSE COALESCE($1, drug_name_null_flavor) END END,
			 mfds_medicinal_product_version = CASE WHEN 'mfds_medicinal_product_version' = ANY($21) THEN NULL ELSE COALESCE($3, mfds_medicinal_product_version) END,
			 mfds_medicinal_product_id = CASE WHEN 'mfds_medicinal_product_id' = ANY($21) THEN NULL ELSE COALESCE($4, mfds_medicinal_product_id) END,
			 mpid = CASE WHEN 'mpid' = ANY($21) THEN NULL ELSE COALESCE($5, mpid) END,
			 mpid_version = CASE WHEN 'mpid_version' = ANY($21) THEN NULL ELSE COALESCE($6, mpid_version) END,
			 mpid_source_code_system = CASE WHEN 'mpid_source_code_system' = ANY($21) OR (($5 IS NOT NULL OR $6 IS NOT NULL) AND $17 IS NULL AND $18 IS NULL) THEN NULL ELSE COALESCE($17, mpid_source_code_system) END,
			 mpid_source_code_system_version = CASE WHEN 'mpid_source_code_system_version' = ANY($21) OR (($5 IS NOT NULL OR $6 IS NOT NULL) AND $17 IS NULL AND $18 IS NULL) THEN NULL ELSE COALESCE($18, mpid_source_code_system_version) END,
			 phpid = CASE WHEN 'phpid' = ANY($21) THEN NULL ELSE COALESCE($7, phpid) END,
			 phpid_version = CASE WHEN 'phpid_version' = ANY($21) THEN NULL ELSE COALESCE($8, phpid_version) END,
			 start_date = CASE WHEN 'start_date' = ANY($21) THEN NULL ELSE CASE WHEN $10::varchar IS NOT NULL THEN NULL ELSE COALESCE($9, start_date) END END,
			 start_date_null_flavor = CASE WHEN 'start_date_null_flavor' = ANY($21) THEN NULL ELSE CASE WHEN $9::date IS NOT NULL THEN NULL ELSE COALESCE($10, start_date_null_flavor) END END,
			 end_date = CASE WHEN 'end_date' = ANY($21) THEN NULL ELSE CASE WHEN $12::varchar IS NOT NULL THEN NULL ELSE COALESCE($11, end_date) END END,
			 end_date_null_flavor = CASE WHEN 'end_date_null_flavor' = ANY($21) THEN NULL ELSE CASE WHEN $11::date IS NOT NULL THEN NULL ELSE COALESCE($12, end_date_null_flavor) END END,
			 indication_meddra_version = CASE WHEN 'indication_meddra_version' = ANY($21) THEN NULL ELSE COALESCE($13, indication_meddra_version) END,
			 indication_meddra_code = CASE WHEN 'indication_meddra_code' = ANY($21) THEN NULL ELSE COALESCE($14, indication_meddra_code) END,
			 reaction_meddra_version = CASE WHEN 'reaction_meddra_version' = ANY($21) THEN NULL ELSE COALESCE($15, reaction_meddra_version) END,
			 reaction_meddra_code = CASE WHEN 'reaction_meddra_code' = ANY($21) THEN NULL ELSE COALESCE($16, reaction_meddra_code) END,
			 updated_at = now(),
			 updated_by = $19
			 WHERE id = $20",
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
					.bind(data.mpid_source_code_system)
					.bind(data.mpid_source_code_system_version)
					.bind(ctx.user_id())
					.bind(id)
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
		Self::update_patch(ctx, mm, id, data, &[]).await
	}

	pub async fn update_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PatientDeathInformationForUpdate,
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
			"UPDATE {} SET
			 date_of_death = CASE WHEN 'date_of_death' = ANY($7) THEN NULL ELSE CASE
			 	WHEN $1::varchar IS NOT NULL THEN NULL
			 	ELSE COALESCE($2, date_of_death)
			 END END,
			 date_of_death_null_flavor = CASE WHEN 'date_of_death_null_flavor' = ANY($7) THEN NULL ELSE CASE
			 	WHEN $1::varchar IS NOT NULL THEN $1
			 	WHEN $2::date IS NOT NULL THEN NULL
			 	ELSE date_of_death_null_flavor
			 END END,
			 autopsy_performed = CASE WHEN 'autopsy_performed' = ANY($7) THEN NULL ELSE CASE
			  WHEN $3::varchar IS NOT NULL THEN NULL
			  ELSE COALESCE($4, autopsy_performed)
			 END END,
			 autopsy_performed_null_flavor = CASE WHEN 'autopsy_performed_null_flavor' = ANY($7) THEN NULL ELSE CASE
			  WHEN $3::varchar IS NOT NULL THEN $3
			  WHEN $4::boolean IS NOT NULL THEN NULL
			  ELSE autopsy_performed_null_flavor
			 END END,
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
					.bind(id)
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
		if filters.is_empty() {
			filters.push(ReportedCauseOfDeathFilter::default());
		}
		for filter in &mut filters {
			filter.deleted = Some(OpValBool::Eq(false).into());
		}
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
		if filters.is_empty() {
			filters.push(AutopsyCauseOfDeathFilter::default());
		}
		for filter in &mut filters {
			filter.deleted = Some(OpValBool::Eq(false).into());
		}
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
		if filters.is_empty() {
			filters.push(ParentInformationFilter::default());
		}
		for filter in &mut filters {
			filter.deleted = Some(OpValBool::Eq(false).into());
		}
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ParentInformationForUpdate,
	) -> Result<()> {
		Self::update_patch(ctx, mm, id, data, &[]).await
	}

	pub async fn update_patch(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ParentInformationForUpdate,
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
		let parent_age_clear = matches!(data.parent_age, Some(None))
			|| clear_fields.contains(&"parent_age");
		let parent_age = data.parent_age.flatten();
		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE parent_information SET parent_age = CASE WHEN $2 THEN NULL ELSE parent_age END WHERE id = $1",
				)
				.bind(id)
				.bind(parent_age_clear),
			)
			.await?;

		let sql = format!(
			"UPDATE {} SET
			 parent_identification = CASE WHEN 'parent_identification' = ANY($16) THEN NULL ELSE CASE
			   WHEN $1::varchar IS NOT NULL THEN NULL
			   ELSE COALESCE($2, parent_identification)
			 END END,
			 parent_identification_null_flavor = CASE WHEN 'parent_identification_null_flavor' = ANY($16) THEN NULL ELSE CASE
			   WHEN $1::varchar IS NOT NULL THEN $1
			   WHEN $2::varchar IS NOT NULL THEN NULL
			   ELSE parent_identification_null_flavor
			 END END,
			 parent_birth_date = CASE WHEN 'parent_birth_date' = ANY($16) THEN NULL ELSE CASE
			   WHEN $3::varchar IS NOT NULL THEN NULL
			   ELSE COALESCE($4, parent_birth_date)
			 END END,
			 parent_birth_date_null_flavor = CASE WHEN 'parent_birth_date_null_flavor' = ANY($16) THEN NULL ELSE CASE
			   WHEN $3::varchar IS NOT NULL THEN $3
			   WHEN $4::date IS NOT NULL THEN NULL
			   ELSE parent_birth_date_null_flavor
			 END END,
			 parent_age = CASE WHEN 'parent_age' = ANY($16) THEN NULL ELSE COALESCE($5, parent_age) END,
			 parent_age_unit = CASE WHEN 'parent_age_unit' = ANY($16) THEN NULL ELSE COALESCE($6, parent_age_unit) END,
			 last_menstrual_period_date = CASE WHEN 'last_menstrual_period_date' = ANY($16) THEN NULL ELSE CASE
			   WHEN $7::varchar IS NOT NULL THEN NULL
			   ELSE COALESCE($8, last_menstrual_period_date)
			 END END,
			 last_menstrual_period_date_null_flavor = CASE WHEN 'last_menstrual_period_date_null_flavor' = ANY($16) THEN NULL ELSE CASE
			   WHEN $7::varchar IS NOT NULL THEN $7
			   WHEN $8::date IS NOT NULL THEN NULL
			   ELSE last_menstrual_period_date_null_flavor
			 END END,
			 weight_kg = CASE WHEN 'weight_kg' = ANY($16) THEN NULL ELSE COALESCE($9, weight_kg) END,
			 height_cm = CASE WHEN 'height_cm' = ANY($16) THEN NULL ELSE COALESCE($10, height_cm) END,
			 sex = CASE WHEN 'sex' = ANY($16) THEN NULL ELSE CASE
			   WHEN $11::varchar IS NOT NULL THEN NULL
			   ELSE COALESCE($12, sex)
			 END END,
			 sex_null_flavor = CASE WHEN 'sex_null_flavor' = ANY($16) THEN NULL ELSE CASE
			   WHEN $11::varchar IS NOT NULL THEN $11
			   WHEN $12::varchar IS NOT NULL THEN NULL
			   ELSE sex_null_flavor
			 END END,
			 medical_history_text = CASE WHEN 'medical_history_text' = ANY($16) THEN NULL ELSE COALESCE($13, medical_history_text) END,
			 updated_at = now(),
			 updated_by = $14
			 WHERE id = $15",
			Self::TABLE
		);

		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(data.parent_identification_null_flavor)
					.bind(data.parent_identification)
					.bind(data.parent_birth_date_null_flavor)
					.bind(data.parent_birth_date)
					.bind(parent_age)
					.bind(data.parent_age_unit)
					.bind(data.last_menstrual_period_date_null_flavor)
					.bind(data.last_menstrual_period_date)
					.bind(data.weight_kg)
					.bind(data.height_cm)
					.bind(data.sex_null_flavor)
					.bind(data.sex)
					.bind(data.medical_history_text)
					.bind(ctx.user_id())
					.bind(id)
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
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn numeric_updates_distinguish_omitted_null_and_value() {
		let omitted: PatientInformationForUpdate =
			serde_json::from_value(serde_json::json!({})).unwrap();
		assert_eq!(omitted.weight_kg, None);

		let cleared: PatientInformationForUpdate =
			serde_json::from_value(serde_json::json!({"weight_kg": null})).unwrap();
		assert_eq!(cleared.weight_kg, Some(None));

		let set: PatientInformationForUpdate =
			serde_json::from_value(serde_json::json!({"weight_kg": 42.5})).unwrap();
		assert_eq!(set.weight_kg, Some(Some(Decimal::new(425, 1))));

		let parent_cleared: ParentInformationForUpdate =
			serde_json::from_value(serde_json::json!({"parent_age": null})).unwrap();
		assert_eq!(parent_cleared.parent_age, Some(None));
	}
}
