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
	pub transmission_date: Date,

	// C.1.3 - Type of Report (MANDATORY)
	pub report_type: String,

	// C.1.4 - Date Report Was First Received from Source (MANDATORY)
	pub date_first_received_from_source: Date,

	// C.1.5 - Date of Most Recent Information (MANDATORY)
	pub date_of_most_recent_information: Date,

	// C.1.7 - Fulfils Expedited Criteria (MANDATORY)
	pub fulfil_expedited_criteria: bool,

	// FDA.C.1.7.1 - Local Criteria Report Type (FDA)
	pub local_criteria_report_type: Option<String>,

	// FDA.C.1.12 - Combination Product Report Indicator (FDA)
	pub combination_product_report_indicator: Option<String>,

	// C.1.8.1 - Worldwide Unique Case Identification
	pub worldwide_unique_id: Option<String>,

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

#[derive(Fields, Deserialize)]
pub struct SafetyReportIdentificationForCreate {
	pub case_id: Uuid,
	#[serde(deserialize_with = "crate::serde::flex_date::deserialize_date")]
	pub transmission_date: Date,
	pub report_type: String,
	#[serde(deserialize_with = "crate::serde::flex_date::deserialize_date")]
	pub date_first_received_from_source: Date,
	#[serde(deserialize_with = "crate::serde::flex_date::deserialize_date")]
	pub date_of_most_recent_information: Date,
	pub fulfil_expedited_criteria: bool,
}

#[derive(Fields, Deserialize)]
pub struct SafetyReportIdentificationForUpdate {
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub transmission_date: Option<Date>,
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
	pub local_criteria_report_type: Option<String>,
	pub combination_product_report_indicator: Option<String>,
	pub worldwide_unique_id: Option<String>,
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
	pub sender_type: String,

	// C.3.2 - Sender's Organisation (MANDATORY)
	pub organization_name: String,
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
	pub sender_type: String,
	pub organization_name: String,
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
	pub qualification: Option<String>,
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
}

#[derive(Fields, Deserialize)]
pub struct StudyInformationForUpdate {
	pub study_name: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub study_type_reaction: Option<String>,
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
			"INSERT INTO {} (case_id, transmission_date, report_type, date_first_received_from_source, date_of_most_recent_information, fulfil_expedited_criteria, created_at, updated_at, created_by)
			 VALUES ($1, $2, $3, $4, $5, $6, now(), now(), $7)
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(data.case_id)
					.bind(data.transmission_date)
					.bind(data.report_type)
					.bind(data.date_first_received_from_source)
					.bind(data.date_of_most_recent_information)
					.bind(data.fulfil_expedited_criteria)
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
		let should_mark_nullified = data
			.nullification_code
			.as_deref()
			.map(str::trim)
			.map(|v| !v.is_empty())
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
			 SET transmission_date = COALESCE($2, transmission_date),
			     report_type = COALESCE($3, report_type),
			     date_first_received_from_source = COALESCE($4, date_first_received_from_source),
			     date_of_most_recent_information = COALESCE($5, date_of_most_recent_information),
			     fulfil_expedited_criteria = COALESCE($6, fulfil_expedited_criteria),
			     local_criteria_report_type = COALESCE($7, local_criteria_report_type),
			     combination_product_report_indicator = COALESCE($8, combination_product_report_indicator),
			     worldwide_unique_id = COALESCE($9, worldwide_unique_id),
			     nullification_code = COALESCE($10, nullification_code),
			     nullification_reason = COALESCE($11, nullification_reason),
			     receiver_organization = COALESCE($12, receiver_organization),
			     updated_at = now(),
			     updated_by = $13
			 WHERE case_id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(case_id)
					.bind(data.transmission_date)
					.bind(data.report_type)
					.bind(data.date_first_received_from_source)
					.bind(data.date_of_most_recent_information)
					.bind(data.fulfil_expedited_criteria)
					.bind(data.local_criteria_report_type)
					.bind(data.combination_product_report_indicator)
					.bind(data.worldwide_unique_id)
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
