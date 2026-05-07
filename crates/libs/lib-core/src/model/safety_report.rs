// Section C - Safety Report Identification

use crate::ctx::{Ctx, SYSTEM_USER_ID};
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValsValue};
use serde::{Deserialize, Serialize};
use sqlx::types::time::{Date, OffsetDateTime};
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- SafetyReportIdentification

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SafetyReportIdentification {
	pub id: Uuid,
	pub case_id: Uuid,

	// C.1.2 - Date of Creation (MANDATORY)
	pub transmission_date: Option<Date>,
	pub transmission_date_null_flavor: Option<String>,

	// C.1.3 - Type of Report (MANDATORY)
	pub report_type: Option<String>,

	// C.1.4 - Date Report Was First Received from Source (MANDATORY)
	pub date_first_received_from_source: Option<Date>,
	pub date_first_received_from_source_null_flavor: Option<String>,

	// C.1.5 - Date of Most Recent Information (MANDATORY)
	pub date_of_most_recent_information: Option<Date>,
	pub date_of_most_recent_information_null_flavor: Option<String>,

	// C.1.7 - Fulfils Expedited Criteria (MANDATORY)
	pub fulfil_expedited_criteria: Option<bool>,

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
pub struct SafetyReportIdentificationForCreate {
	pub case_id: Uuid,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub transmission_date: Option<Date>,
	pub transmission_date_null_flavor: Option<String>,
	pub report_type: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_first_received_from_source: Option<Date>,
	pub date_first_received_from_source_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_of_most_recent_information: Option<Date>,
	pub date_of_most_recent_information_null_flavor: Option<String>,
	pub fulfil_expedited_criteria: Option<bool>,
	pub local_criteria_report_type: Option<String>,
	pub combination_product_report_indicator: Option<String>,
	pub first_sender_type: Option<String>,
	pub additional_documents_available: Option<bool>,
	pub other_case_identifiers_exist: Option<bool>,
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
pub struct SafetyReportIdentificationForUpdate {
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub transmission_date: Option<Date>,
	pub transmission_date_null_flavor: Option<String>,
	#[serde(default, deserialize_with = "deserialize_patch_value")]
	pub report_type: PatchValue<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_first_received_from_source: Option<Date>,
	pub date_first_received_from_source_null_flavor: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub date_of_most_recent_information: Option<Date>,
	pub date_of_most_recent_information_null_flavor: Option<String>,
	#[serde(default, deserialize_with = "deserialize_patch_value")]
	pub fulfil_expedited_criteria: PatchValue<bool>,
	pub local_criteria_report_type: Option<String>,
	pub combination_product_report_indicator: Option<String>,
	pub worldwide_unique_id: Option<String>,
	pub first_sender_type: Option<String>,
	pub additional_documents_available: Option<bool>,
	pub other_case_identifiers_exist: Option<bool>,
	pub nullification_code: Option<String>,
	pub nullification_reason: Option<String>,
	pub receiver_organization: Option<String>,
}

// -- SenderInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SenderInformation {
	pub id: Uuid,
	pub case_id: Uuid,

	// C.3.1 - Sender Type (MANDATORY)
	pub sender_type: Option<String>,

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
	pub sender_type: Option<String>,
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
	pub sender_type: Option<String>,
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
	pub sequence_number: i32,

	// C.2.r.1 - Reporter's Name
	pub reporter_title: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_family_name: Option<String>,

	// C.2.r.2 - Reporter's Address
	pub organization: Option<String>,
	pub department: Option<String>,
	pub street: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub telephone: Option<String>,

	// C.2.r.3 - Country Code
	pub country_code: Option<String>,
	pub email: Option<String>,

	// C.2.r.4 - Qualification (MANDATORY within primary source)
	pub qualification: Option<String>,
	// MFDS.C.2.r.4.KR.1 - Other health professional type
	pub qualification_kr1: Option<String>,

	// C.2.r.5 - Primary Source for Regulatory Purposes (MANDATORY)
	pub primary_source_regulatory: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct PrimarySourceForCreate {
	pub case_id: Uuid,
	pub sequence_number: i32,
	pub reporter_title: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_family_name: Option<String>,
	pub organization: Option<String>,
	pub department: Option<String>,
	pub street: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub telephone: Option<String>,
	pub country_code: Option<String>,
	pub email: Option<String>,
	pub qualification: Option<String>,
	pub qualification_kr1: Option<String>,
	pub primary_source_regulatory: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct PrimarySourceForUpdate {
	pub reporter_title: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_family_name: Option<String>,
	pub organization: Option<String>,
	pub department: Option<String>,
	pub street: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub telephone: Option<String>,
	pub country_code: Option<String>,
	pub email: Option<String>,
	pub qualification: Option<String>,
	pub qualification_kr1: Option<String>,
	pub primary_source_regulatory: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct PrimarySourceFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub case_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- LiteratureReference

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct LiteratureReference {
	pub id: Uuid,
	pub case_id: Uuid,
	pub reference_text: String,
	pub sequence_number: i32,
	pub document_base64: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct LiteratureReferenceForCreate {
	pub case_id: Uuid,
	pub reference_text: String,
	pub sequence_number: i32,
	pub document_base64: Option<String>,
	pub media_type: Option<String>,
	pub representation: Option<String>,
	pub compression: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct LiteratureReferenceForUpdate {
	pub reference_text: Option<String>,
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
}
// -- StudyInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyInformation {
	pub id: Uuid,
	pub case_id: Uuid,

	pub study_name: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub study_type_reaction: Option<String>,
	// MFDS.C.5.4.KR.1 - Other studies type
	pub study_type_reaction_kr1: Option<String>,

	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyInformationForCreate {
	pub case_id: Uuid,
	pub study_name: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub study_type_reaction: Option<String>,
	pub study_type_reaction_kr1: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct StudyInformationForUpdate {
	pub study_name: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub study_type_reaction: Option<String>,
	pub study_type_reaction_kr1: Option<String>,
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
	pub country_code: Option<String>,
	pub sequence_number: i32,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyRegistrationNumberForCreate {
	pub study_information_id: Uuid,
	pub registration_number: String,
	pub country_code: Option<String>,
	pub sequence_number: i32,
}

#[derive(Fields, Deserialize)]
pub struct StudyRegistrationNumberForUpdate {
	pub registration_number: Option<String>,
	pub country_code: Option<String>,
	pub sequence_number: Option<i32>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct StudyRegistrationNumberFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub study_information_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- BMCs (Business Model Controllers)

pub struct SafetyReportIdentificationBmc;
impl DbBmc for SafetyReportIdentificationBmc {
	const TABLE: &'static str = "safety_report_identification";
}

impl SafetyReportIdentificationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: SafetyReportIdentificationForCreate,
	) -> Result<Uuid> {
		mm.dbx().begin_txn().await?;
		set_full_context_from_ctx_dbx(mm.dbx(), ctx).await?;

		let sql = format!(
			"INSERT INTO {} (case_id, transmission_date, transmission_date_null_flavor, report_type, date_first_received_from_source, date_first_received_from_source_null_flavor, date_of_most_recent_information, date_of_most_recent_information_null_flavor, fulfil_expedited_criteria, local_criteria_report_type, combination_product_report_indicator, worldwide_unique_id, first_sender_type, additional_documents_available, other_case_identifiers_exist, nullification_code, nullification_reason, receiver_organization, created_at, updated_at, created_by)
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, now(), now(), $19)
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(data.case_id)
					.bind(data.transmission_date)
					.bind(data.transmission_date_null_flavor)
					.bind(data.report_type)
					.bind(data.date_first_received_from_source)
					.bind(data.date_first_received_from_source_null_flavor)
					.bind(data.date_of_most_recent_information)
					.bind(data.date_of_most_recent_information_null_flavor)
					.bind(data.fulfil_expedited_criteria)
					.bind(data.local_criteria_report_type)
					.bind(data.combination_product_report_indicator)
					.bind(data.worldwide_unique_id)
					.bind(data.first_sender_type)
					.bind(data.additional_documents_available)
					.bind(data.other_case_identifiers_exist)
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
		_ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<SafetyReportIdentification> {
		let sql = format!("SELECT * FROM {} WHERE case_id = $1", Self::TABLE);
		let report = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, SafetyReportIdentification>(&sql).bind(case_id),
			)
			.await?;
		report.ok_or(crate::model::Error::EntityUuidNotFound {
			entity: Self::TABLE,
			id: case_id,
		})
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
		let should_mark_nullified = data
			.nullification_code
			.as_deref()
			.map(str::trim)
			.map(|v| v == "1")
			.unwrap_or(false);

		if should_mark_nullified {
			let row = mm
				.dbx()
				.fetch_optional(
					sqlx::query_as::<_, (String,)>(
						"SELECT status FROM cases WHERE id = $1",
					)
					.bind(case_id),
				)
				.await?;
			let (current_status,) =
				row.ok_or(crate::model::Error::EntityUuidNotFound {
					entity: "cases",
					id: case_id,
				})?;
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
			 SET transmission_date = CASE WHEN $3 IS NOT NULL THEN NULL ELSE COALESCE($2, transmission_date) END,
			     transmission_date_null_flavor = CASE WHEN $2 IS NOT NULL THEN NULL ELSE COALESCE($3, transmission_date_null_flavor) END,
			     report_type = CASE WHEN $4 THEN NULL ELSE COALESCE($5, report_type) END,
			     date_first_received_from_source = CASE WHEN $7 IS NOT NULL THEN NULL ELSE COALESCE($6, date_first_received_from_source) END,
			     date_first_received_from_source_null_flavor = CASE WHEN $6 IS NOT NULL THEN NULL ELSE COALESCE($7, date_first_received_from_source_null_flavor) END,
			     date_of_most_recent_information = CASE WHEN $9 IS NOT NULL THEN NULL ELSE COALESCE($8, date_of_most_recent_information) END,
			     date_of_most_recent_information_null_flavor = CASE WHEN $8 IS NOT NULL THEN NULL ELSE COALESCE($9, date_of_most_recent_information_null_flavor) END,
			     fulfil_expedited_criteria = CASE WHEN $10 THEN NULL ELSE COALESCE($11, fulfil_expedited_criteria) END,
			     local_criteria_report_type = COALESCE($12, local_criteria_report_type),
			     combination_product_report_indicator = COALESCE($13, combination_product_report_indicator),
			     worldwide_unique_id = COALESCE($14, worldwide_unique_id),
			     first_sender_type = COALESCE($15, first_sender_type),
			     additional_documents_available = COALESCE($16, additional_documents_available),
			     other_case_identifiers_exist = COALESCE($17, other_case_identifiers_exist),
			     nullification_code = COALESCE($18, nullification_code),
			     nullification_reason = COALESCE($19, nullification_reason),
			     receiver_organization = COALESCE($20, receiver_organization),
			     updated_at = now(),
			     updated_by = $21
			 WHERE case_id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(case_id)
					.bind(data.transmission_date)
					.bind(data.transmission_date_null_flavor)
					.bind(clear_report_type)
					.bind(report_type)
					.bind(data.date_first_received_from_source)
					.bind(data.date_first_received_from_source_null_flavor)
					.bind(data.date_of_most_recent_information)
					.bind(data.date_of_most_recent_information_null_flavor)
					.bind(clear_fulfil_expedited_criteria)
					.bind(fulfil_expedited_criteria)
					.bind(data.local_criteria_report_type)
					.bind(data.combination_product_report_indicator)
					.bind(data.worldwide_unique_id)
					.bind(data.first_sender_type)
					.bind(data.additional_documents_available)
					.bind(data.other_case_identifiers_exist)
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
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: PrimarySourceForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
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
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
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
		base_uuid::delete::<Self>(ctx, mm, id).await
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
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
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
		base_uuid::delete::<Self>(ctx, mm, id).await
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
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
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
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}
