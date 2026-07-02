// Section C - Safety Report Identification

use crate::ctx::{Ctx, SYSTEM_USER_ID};
use crate::e2b::null_flavor::NullFlavor;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValBool, OpValsBool, OpValsValue};
use serde::{Deserialize, Serialize};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Uuid;
use sqlx::FromRow;

fn validation_error(message: &str) -> crate::model::Error {
	crate::model::Error::Validation {
		message: message.to_string(),
	}
}

fn validate_primary_source_null_flavor_set(
	field: &str,
	value: Option<&str>,
	allowed: &[NullFlavor],
) -> Result<()> {
	let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
		return Ok(());
	};
	let parsed: NullFlavor = value
		.parse()
		.map_err(|err| validation_error(&format!("{field}: {err}")))?;
	if parsed.is_one_of(allowed) {
		Ok(())
	} else {
		Err(validation_error(&format!(
			"{field}: nullFlavor {parsed} is not allowed"
		)))
	}
}

// -- SafetyReportIdentification

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SafetyReportIdentification {
	pub id: Uuid,
	pub case_id: Uuid,

	// C.1.1 - Sender's (case) Safety Report Unique Identifier (MANDATORY)
	pub safety_report_id: Option<String>,
	pub version: i32,

	// C.1.2 - Date of Creation (MANDATORY)
	pub transmission_date: Option<String>,

	// C.1.3 - Type of Report (MANDATORY)
	pub report_type: Option<String>,

	// C.1.4 - Date Report Was First Received from Source (MANDATORY)
	pub date_first_received_from_source: Option<Date>,

	// C.1.5 - Date of Most Recent Information (MANDATORY)
	pub date_of_most_recent_information: Option<Date>,

	// C.1.7 - Fulfils Expedited Criteria (MANDATORY)
	pub fulfil_expedited_criteria: Option<bool>,
	pub fulfil_expedited_criteria_null_flavor: Option<String>,

	// FDA.C.1.7.1 - Local Criteria Report Type (FDA)
	pub local_criteria_report_type: Option<String>,

	// FDA.C.1.12 - Combination Product Report Indicator (FDA)
	pub combination_product_report_indicator: Option<String>,

	// C.1.8.1 - Worldwide Unique Case Identification
	pub worldwide_unique_id: Option<String>,

	// C.1.8.2 - First Sender of This Case
	pub first_sender_type: Option<String>,

	// C.1.6.1 - Are Additional Documents Available?
	pub additional_documents_available: Option<bool>,

	// C.1.9.1 - Other Case Identifiers in Previous Transmissions
	pub other_case_identifiers_exist: Option<bool>,
	pub other_case_identifiers_exist_null_flavor: Option<String>,

	// C.1.11.1 - Nullification/Amendment Code
	pub nullification_code: Option<String>,

	// C.1.11.2 - Nullification Reason
	pub nullification_reason: Option<String>,

	// Receiver Organization
	pub receiver_organization: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyReportIdentificationForCreate {
	pub case_id: Uuid,
	pub safety_report_id: Option<String>,
	pub version: Option<i32>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_e2b_datetime"
	)]
	pub transmission_date: Option<String>,
	pub report_type: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_first_received_from_source: Option<Date>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_of_most_recent_information: Option<Date>,
	pub fulfil_expedited_criteria: Option<bool>,
	pub fulfil_expedited_criteria_null_flavor: Option<String>,
	pub local_criteria_report_type: Option<String>,
	pub combination_product_report_indicator: Option<String>,
	pub first_sender_type: Option<String>,
	pub additional_documents_available: Option<bool>,
	pub other_case_identifiers_exist: Option<bool>,
	pub other_case_identifiers_exist_null_flavor: Option<String>,
	pub worldwide_unique_id: Option<String>,
	pub nullification_code: Option<String>,
	pub nullification_reason: Option<String>,
	pub receiver_organization: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum PatchValue<T> {
	#[default]
	Missing,
	Null,
	Value(T),
}

impl<T> PatchValue<T> {
	fn into_parts(self) -> (Option<T>, bool) {
		match self {
			Self::Missing => (None, false),
			Self::Null => (None, true),
			Self::Value(value) => (Some(value), false),
		}
	}
}

fn deserialize_patch_value<'de, D, T>(
	deserializer: D,
) -> std::result::Result<PatchValue<T>, D::Error>
where
	D: serde::Deserializer<'de>,
	T: Deserialize<'de>,
{
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum PatchInput<T> {
		Null(()),
		Value(T),
	}

	let value = PatchInput::<T>::deserialize(deserializer)?;
	Ok(match value {
		PatchInput::Null(()) => PatchValue::Null,
		PatchInput::Value(value) => PatchValue::Value(value),
	})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyReportIdentificationForUpdate {
	pub safety_report_id: Option<String>,
	pub version: Option<i32>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_e2b_datetime"
	)]
	pub transmission_date: Option<String>,
	#[serde(default, deserialize_with = "deserialize_patch_value")]
	pub report_type: PatchValue<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_first_received_from_source: Option<Date>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_of_most_recent_information: Option<Date>,
	#[serde(default, deserialize_with = "deserialize_patch_value")]
	pub fulfil_expedited_criteria: PatchValue<bool>,
	pub fulfil_expedited_criteria_null_flavor: Option<String>,
	#[serde(default, deserialize_with = "deserialize_patch_value")]
	pub local_criteria_report_type: PatchValue<String>,
	#[serde(default, deserialize_with = "deserialize_patch_value")]
	pub combination_product_report_indicator: PatchValue<String>,
	pub worldwide_unique_id: Option<String>,
	pub first_sender_type: Option<String>,
	pub additional_documents_available: Option<bool>,
	pub other_case_identifiers_exist: Option<bool>,
	pub other_case_identifiers_exist_null_flavor: Option<String>,
	pub nullification_code: Option<String>,
	pub nullification_reason: Option<String>,
	pub receiver_organization: Option<String>,
}

// -- SenderInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SenderInformation {
	pub id: Uuid,
	pub case_id: Uuid,
	pub source_sender_presave_id: Option<Uuid>,

	// C.3.1 - Sender Type (MANDATORY)
	pub sender_type: Option<String>,
	// MFDS.C.3.1.KR.1 - Sender health professional type
	pub health_professional_type_kr1: Option<String>,

	// C.3.2 - Sender's Organisation (MANDATORY)
	pub organization_name: Option<String>,
	pub department: Option<String>,
	pub street_address: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,

	// C.3.3 - Person Responsible for Sending
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,

	// C.3.4 - Contact Information
	pub telephone: Option<String>,
	pub fax: Option<String>,
	pub email: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct SenderInformationForCreate {
	pub case_id: Uuid,
	pub source_sender_presave_id: Option<Uuid>,
	pub sender_type: Option<String>,
	pub health_professional_type_kr1: Option<String>,
	pub organization_name: Option<String>,
	pub department: Option<String>,
	pub street_address: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,
	pub telephone: Option<String>,
	pub fax: Option<String>,
	pub email: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct SenderInformationForUpdate {
	pub source_sender_presave_id: Option<Uuid>,
	pub sender_type: Option<String>,
	pub health_professional_type_kr1: Option<String>,
	pub organization_name: Option<String>,
	pub department: Option<String>,
	pub street_address: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,
	pub telephone: Option<String>,
	pub fax: Option<String>,
	pub email: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct SenderInformationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
}

// -- PrimarySource

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct PrimarySource {
	pub id: Uuid,
	pub case_id: Uuid,
	pub source_reporter_presave_id: Option<Uuid>,
	pub sequence_number: i32,

	// C.2.r.1 - Reporter's Name
	pub reporter_title: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_family_name: Option<String>,
	pub reporter_name_null_flavor: Option<String>,

	// C.2.r.2 - Reporter's Address
	pub organization: Option<String>,
	pub department: Option<String>,
	pub street: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub telephone: Option<String>,
	pub reporter_address_null_flavor: Option<String>,

	// C.2.r.3 - Country Code
	pub country_code: Option<String>,
	pub email: Option<String>,

	// C.2.r.4 - Qualification (MANDATORY within primary source)
	pub qualification: Option<String>,
	pub qualification_null_flavor: Option<String>,
	// MFDS.C.2.r.4.KR.1 - Other health professional type
	pub qualification_kr1: Option<String>,

	// C.2.r.5 - Primary Source for Regulatory Purposes (MANDATORY)
	pub primary_source_regulatory: Option<String>,

	pub deleted: bool,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct PrimarySourceForCreate {
	pub case_id: Uuid,
	pub source_reporter_presave_id: Option<Uuid>,
	pub sequence_number: i32,
	pub reporter_title: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_family_name: Option<String>,
	pub reporter_name_null_flavor: Option<String>,
	pub organization: Option<String>,
	pub department: Option<String>,
	pub street: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub telephone: Option<String>,
	pub reporter_address_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub email: Option<String>,
	pub qualification: Option<String>,
	pub qualification_null_flavor: Option<String>,
	pub qualification_kr1: Option<String>,
	pub primary_source_regulatory: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct PrimarySourceForUpdate {
	pub source_reporter_presave_id: Option<Uuid>,
	pub reporter_title: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_family_name: Option<String>,
	pub reporter_name_null_flavor: Option<String>,
	pub organization: Option<String>,
	pub department: Option<String>,
	pub street: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub telephone: Option<String>,
	pub reporter_address_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub email: Option<String>,
	pub qualification: Option<String>,
	pub qualification_null_flavor: Option<String>,
	pub qualification_kr1: Option<String>,
	pub primary_source_regulatory: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct PrimarySourceFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- LiteratureReference

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct LiteratureReference {
	pub id: Uuid,
	pub case_id: Uuid,
	pub reference_text: String,
	pub reference_text_null_flavor: Option<String>,
	pub sequence_number: i32,
	pub document_base64: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct LiteratureReferenceForCreate {
	pub case_id: Uuid,
	pub reference_text: String,
	pub reference_text_null_flavor: Option<String>,
	pub sequence_number: i32,
	pub document_base64: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct LiteratureReferenceForUpdate {
	pub reference_text: Option<String>,
	pub reference_text_null_flavor: Option<String>,
	pub sequence_number: Option<i32>,
	pub document_base64: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct LiteratureReferenceFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- DocumentsHeldBySender

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DocumentsHeldBySender {
	pub id: Uuid,
	pub case_id: Uuid,
	pub title: Option<String>,
	pub document_base64: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
	pub sequence_number: i32,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DocumentsHeldBySenderForCreate {
	pub case_id: Uuid,
	pub title: Option<String>,
	pub document_base64: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
	pub sequence_number: i32,
}

#[derive(Fields, Deserialize)]
pub struct DocumentsHeldBySenderForUpdate {
	pub title: Option<String>,
	pub document_base64: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
	pub sequence_number: Option<i32>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DocumentsHeldBySenderFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}
// -- StudyInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyInformation {
	pub id: Uuid,
	pub case_id: Uuid,
	pub source_study_presave_id: Option<Uuid>,

	pub study_name: Option<String>,
	pub study_name_null_flavor: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_null_flavor: Option<String>,
	pub study_type_reaction: Option<String>,
	// MFDS.C.5.4.KR.1 - Other studies type
	pub study_type_reaction_kr1: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,

	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyInformationForCreate {
	pub case_id: Uuid,
	pub source_study_presave_id: Option<Uuid>,
	pub study_name: Option<String>,
	pub study_name_null_flavor: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_null_flavor: Option<String>,
	pub study_type_reaction: Option<String>,
	pub study_type_reaction_kr1: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct StudyInformationForUpdate {
	pub source_study_presave_id: Option<Uuid>,
	pub study_name: Option<String>,
	pub study_name_null_flavor: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_null_flavor: Option<String>,
	pub study_type_reaction: Option<String>,
	pub study_type_reaction_kr1: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct StudyInformationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
}

// -- StudyRegistrationNumber

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyRegistrationNumber {
	pub id: Uuid,
	pub study_information_id: Uuid,
	pub registration_number: String,
	pub registration_number_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub country_code_null_flavor: Option<String>,
	pub sequence_number: i32,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyRegistrationNumberForCreate {
	pub study_information_id: Uuid,
	pub registration_number: String,
	pub registration_number_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub country_code_null_flavor: Option<String>,
	pub sequence_number: i32,
}

#[derive(Fields, Deserialize)]
pub struct StudyRegistrationNumberForUpdate {
	pub registration_number: Option<String>,
	pub registration_number_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub country_code_null_flavor: Option<String>,
	pub sequence_number: Option<i32>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct StudyRegistrationNumberFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub study_information_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- StudyFdaCrossReportedInd

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyFdaCrossReportedInd {
	pub id: Uuid,
	pub study_information_id: Uuid,
	pub ind_number: Option<String>,
	pub ind_number_null_flavor: Option<String>,
	pub sequence_number: i32,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyFdaCrossReportedIndForCreate {
	pub study_information_id: Uuid,
	pub ind_number: Option<String>,
	pub ind_number_null_flavor: Option<String>,
	pub sequence_number: i32,
}

#[derive(Fields, Deserialize)]
pub struct StudyFdaCrossReportedIndForUpdate {
	pub ind_number: Option<String>,
	pub ind_number_null_flavor: Option<String>,
	pub sequence_number: Option<i32>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct StudyFdaCrossReportedIndFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub study_information_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
	pub deleted: Option<OpValsBool>,
}

// -- BMCs (Business Model Controllers)

pub struct SafetyReportIdentificationBmc;
impl DbBmc for SafetyReportIdentificationBmc {
	const TABLE: &'static str = "safety_report_identification";
}

impl SafetyReportIdentificationBmc {
	pub async fn max_version_by_safety_report_id(
		ctx: &Ctx,
		mm: &ModelManager,
		safety_report_id: &str,
	) -> Result<i32> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let row = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Option<i32>,)>(
					"SELECT MAX(version) FROM safety_report_identification WHERE safety_report_id = $1",
				)
				.bind(safety_report_id),
			)
			.await;
		match row {
			Ok((value,)) => {
				mm.dbx().commit_txn().await?;
				Ok(value.unwrap_or(0))
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: SafetyReportIdentificationForCreate,
	) -> Result<Uuid> {
		mm.dbx().begin_txn().await?;
		set_full_context_from_ctx_dbx(mm.dbx(), ctx).await?;

		let sql = format!(
			"INSERT INTO {} (case_id, safety_report_id, version, transmission_date, report_type, date_first_received_from_source, date_of_most_recent_information, fulfil_expedited_criteria, fulfil_expedited_criteria_null_flavor, local_criteria_report_type, combination_product_report_indicator, worldwide_unique_id, first_sender_type, additional_documents_available, other_case_identifiers_exist, other_case_identifiers_exist_null_flavor, nullification_code, nullification_reason, receiver_organization, created_at, updated_at, created_by)
			 VALUES ($1, $2, COALESCE($3, 1), $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, now(), now(), $19)
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(data.case_id)
					.bind(data.safety_report_id)
					.bind(data.version)
					.bind(data.transmission_date)
					.bind(data.report_type)
					.bind(data.date_first_received_from_source)
					.bind(data.date_of_most_recent_information)
					.bind(data.fulfil_expedited_criteria)
					.bind(data.fulfil_expedited_criteria_null_flavor)
					.bind(data.local_criteria_report_type)
					.bind(data.combination_product_report_indicator)
					.bind(data.worldwide_unique_id)
					.bind(data.first_sender_type)
					.bind(data.additional_documents_available)
					.bind(data.other_case_identifiers_exist)
					.bind(data.other_case_identifiers_exist_null_flavor)
					.bind(data.nullification_code)
					.bind(data.nullification_reason)
					.bind(data.receiver_organization)
					.bind(ctx.user_id()),
			)
			.await?;
		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<SafetyReportIdentification> {
		mm.dbx().begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(mm.dbx(), ctx).await {
			let _ = mm.dbx().rollback_txn().await;
			return Err(err);
		}
		let sql = format!("SELECT * FROM {} WHERE case_id = $1", Self::TABLE);
		let result = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, SafetyReportIdentification>(&sql).bind(case_id),
			)
			.await;
		match result {
			Ok(Some(report)) => {
				mm.dbx().commit_txn().await?;
				Ok(report)
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
		data: SafetyReportIdentificationForUpdate,
	) -> Result<()> {
		let (report_type, clear_report_type) = data.report_type.into_parts();
		let (fulfil_expedited_criteria, clear_fulfil_expedited_criteria) =
			data.fulfil_expedited_criteria.into_parts();
		let (local_criteria_report_type, clear_local_criteria_report_type) =
			data.local_criteria_report_type.into_parts();
		let (
			combination_product_report_indicator,
			clear_combination_product_report_indicator,
		) = data.combination_product_report_indicator.into_parts();
		let should_mark_nullified = data
			.nullification_code
			.as_deref()
			.map(str::trim)
			.map(|v| v == "1")
			.unwrap_or(false);

		if should_mark_nullified {
			mm.dbx().begin_txn().await?;
			set_full_context_from_ctx_dbx(mm.dbx(), ctx).await?;
			let row = mm
				.dbx()
				.fetch_optional(
					sqlx::query_as::<_, (String,)>(
						"SELECT status FROM cases WHERE id = $1",
					)
					.bind(case_id),
				)
				.await?;
			let (current_status,) = match row {
				Some(row) => row,
				None => {
					mm.dbx().rollback_txn().await?;
					return Err(crate::model::Error::EntityUuidNotFound {
						entity: "cases",
						id: case_id,
					});
				}
			};
			mm.dbx().commit_txn().await?;
			let currently_nullified =
				current_status.trim().eq_ignore_ascii_case("nullified");
			let has_reason = ctx
				.change_reason()
				.map(|v| !v.trim().is_empty())
				.unwrap_or(false);
			let has_signature = ctx.e_signature_id().is_some();
			if !currently_nullified
				&& !is_system_context(ctx)
				&& !(has_reason && has_signature)
			{
				return Err(crate::model::Error::Store(
					"compliance context required for nullification status transition"
						.to_string(),
				));
			}
		}

		mm.dbx().begin_txn().await?;
		set_full_context_from_ctx_dbx(mm.dbx(), ctx).await?;

		let sql = format!(
			"UPDATE {}
			 SET safety_report_id = COALESCE($2, safety_report_id),
			     version = COALESCE($3, version),
			     transmission_date = COALESCE($4, transmission_date),
			     report_type = CASE WHEN $5 THEN NULL ELSE COALESCE($6, report_type) END,
			     date_first_received_from_source = COALESCE($7, date_first_received_from_source),
			     date_of_most_recent_information = COALESCE($8, date_of_most_recent_information),
			     fulfil_expedited_criteria = CASE WHEN $9 OR $11 IS NOT NULL THEN NULL ELSE COALESCE($10, fulfil_expedited_criteria) END,
			     fulfil_expedited_criteria_null_flavor = CASE WHEN $10 IS NOT NULL THEN NULL ELSE COALESCE($11, fulfil_expedited_criteria_null_flavor) END,
			     local_criteria_report_type = CASE WHEN $12 THEN NULL ELSE COALESCE($13, local_criteria_report_type) END,
			     combination_product_report_indicator = CASE WHEN $14 THEN NULL ELSE COALESCE($15, combination_product_report_indicator) END,
			     worldwide_unique_id = COALESCE($16, worldwide_unique_id),
			     first_sender_type = COALESCE($17, first_sender_type),
			     additional_documents_available = COALESCE($18, additional_documents_available),
			     other_case_identifiers_exist = CASE WHEN $20 IS NOT NULL THEN NULL ELSE COALESCE($19, other_case_identifiers_exist) END,
			     other_case_identifiers_exist_null_flavor = CASE WHEN $19 IS NOT NULL THEN NULL ELSE COALESCE($20, other_case_identifiers_exist_null_flavor) END,
			     nullification_code = COALESCE($21, nullification_code),
			     nullification_reason = COALESCE($22, nullification_reason),
			     receiver_organization = COALESCE($23, receiver_organization),
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
					.bind(data.safety_report_id)
					.bind(data.version)
					.bind(data.transmission_date)
					.bind(clear_report_type)
					.bind(report_type)
					.bind(data.date_first_received_from_source)
					.bind(data.date_of_most_recent_information)
					.bind(clear_fulfil_expedited_criteria)
					.bind(fulfil_expedited_criteria)
					.bind(data.fulfil_expedited_criteria_null_flavor)
					.bind(clear_local_criteria_report_type)
					.bind(local_criteria_report_type)
					.bind(clear_combination_product_report_indicator)
					.bind(combination_product_report_indicator)
					.bind(data.worldwide_unique_id)
					.bind(data.first_sender_type)
					.bind(data.additional_documents_available)
					.bind(data.other_case_identifiers_exist)
					.bind(data.other_case_identifiers_exist_null_flavor)
					.bind(data.nullification_code)
					.bind(data.nullification_reason)
					.bind(data.receiver_organization)
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

		if should_mark_nullified {
			mm.dbx()
				.execute(
					sqlx::query(
						"UPDATE cases
						 SET status = 'nullified',
						     updated_at = now(),
						     updated_by = $2
						 WHERE id = $1 AND status <> 'nullified'",
					)
					.bind(case_id)
					.bind(ctx.user_id()),
				)
				.await?;
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
		set_full_context_from_ctx_dbx(mm.dbx(), ctx).await?;

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

fn is_system_context(ctx: &Ctx) -> bool {
	ctx.user_id()
		== Uuid::parse_str(SYSTEM_USER_ID).expect("Invalid system user UUID")
}

pub struct SenderInformationBmc;
impl DbBmc for SenderInformationBmc {
	const TABLE: &'static str = "sender_information";
}

impl SenderInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: SenderInformationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<SenderInformation> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<SenderInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<SenderInformation>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: SenderInformationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

pub struct PrimarySourceBmc;
impl DbBmc for PrimarySourceBmc {
	const TABLE: &'static str = "primary_sources";
}

impl PrimarySourceBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: PrimarySourceForCreate,
	) -> Result<Uuid> {
		Self::validate_null_flavors(
			data.reporter_name_null_flavor.as_deref(),
			data.reporter_address_null_flavor.as_deref(),
			data.qualification_null_flavor.as_deref(),
		)?;
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<PrimarySource> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<PrimarySourceFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<PrimarySource>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(PrimarySourceFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PrimarySourceForUpdate,
	) -> Result<()> {
		Self::validate_null_flavors(
			data.reporter_name_null_flavor.as_deref(),
			data.reporter_address_null_flavor.as_deref(),
			data.qualification_null_flavor.as_deref(),
		)?;
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::soft_delete::<Self>(ctx, mm, id).await
	}

	pub async fn restore(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::restore::<Self>(ctx, mm, id).await
	}

	fn validate_null_flavors(
		reporter_name_null_flavor: Option<&str>,
		reporter_address_null_flavor: Option<&str>,
		qualification_null_flavor: Option<&str>,
	) -> Result<()> {
		const NAME_ADDRESS_ALLOWED: &[NullFlavor] =
			&[NullFlavor::MSK, NullFlavor::ASKU, NullFlavor::NASK];
		const QUALIFICATION_ALLOWED: &[NullFlavor] = &[NullFlavor::UNK];

		validate_primary_source_null_flavor_set(
			"reporter_name_null_flavor",
			reporter_name_null_flavor,
			NAME_ADDRESS_ALLOWED,
		)?;
		validate_primary_source_null_flavor_set(
			"reporter_address_null_flavor",
			reporter_address_null_flavor,
			NAME_ADDRESS_ALLOWED,
		)?;
		validate_primary_source_null_flavor_set(
			"qualification_null_flavor",
			qualification_null_flavor,
			QUALIFICATION_ALLOWED,
		)
	}
}

pub struct LiteratureReferenceBmc;
impl DbBmc for LiteratureReferenceBmc {
	const TABLE: &'static str = "literature_references";
}

impl LiteratureReferenceBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: LiteratureReferenceForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<LiteratureReference> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<LiteratureReferenceFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<LiteratureReference>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(LiteratureReferenceFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: LiteratureReferenceForUpdate,
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

pub struct DocumentsHeldBySenderBmc;
impl DbBmc for DocumentsHeldBySenderBmc {
	const TABLE: &'static str = "documents_held_by_sender";
}

impl DocumentsHeldBySenderBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DocumentsHeldBySenderForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DocumentsHeldBySender> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DocumentsHeldBySenderFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DocumentsHeldBySender>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(DocumentsHeldBySenderFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DocumentsHeldBySenderForUpdate,
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

pub struct StudyInformationBmc;
impl DbBmc for StudyInformationBmc {
	const TABLE: &'static str = "study_information";
}

impl StudyInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: StudyInformationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<StudyInformation> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<StudyInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<StudyInformation>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: StudyInformationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

pub struct StudyRegistrationNumberBmc;
impl DbBmc for StudyRegistrationNumberBmc {
	const TABLE: &'static str = "study_registration_numbers";
}

impl StudyRegistrationNumberBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: StudyRegistrationNumberForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<StudyRegistrationNumber> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<StudyRegistrationNumberFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<StudyRegistrationNumber>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(StudyRegistrationNumberFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: StudyRegistrationNumberForUpdate,
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

pub struct StudyFdaCrossReportedIndBmc;
impl DbBmc for StudyFdaCrossReportedIndBmc {
	const TABLE: &'static str = "study_fda_cross_reported_inds";
}

impl StudyFdaCrossReportedIndBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: StudyFdaCrossReportedIndForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<StudyFdaCrossReportedInd> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<StudyFdaCrossReportedIndFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<StudyFdaCrossReportedInd>> {
		let mut filters = filters.unwrap_or_default();
		filters.push(StudyFdaCrossReportedIndFilter {
			deleted: Some(OpValBool::Eq(false).into()),
			..Default::default()
		});
		base_uuid::list::<Self, _, _>(ctx, mm, Some(filters), list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: StudyFdaCrossReportedIndForUpdate,
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
