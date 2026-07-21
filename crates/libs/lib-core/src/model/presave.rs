use crate::ctx::Ctx;
use crate::e2b::null_flavor::NullFlavor;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::presave_lifecycle::{PresaveKind, PresaveLifecycleService};
use crate::model::store::set_full_context_from_ctx_dbx;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::{Fields, HasSeaFields};
use modql::filter::{FilterNodes, ListOptions, OpValsBool};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
use sqlx::FromRow;

#[derive(FilterNodes, Deserialize, Default)]
pub struct PresaveListFilter {
	pub deleted: Option<OpValsBool>,
}

fn clean_presave_text(value: Option<&str>) -> Option<String> {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(str::to_string)
}

fn join_presave_name_parts(parts: &[Option<&str>]) -> String {
	parts
		.iter()
		.filter_map(|value| clean_presave_text(*value))
		.collect::<Vec<_>>()
		.join(" / ")
}

fn narrative_presave_identity(
	case_narrative: Option<&str>,
	additional_information: Option<&str>,
) -> Option<String> {
	let parts = [case_narrative, additional_information];
	let name = join_presave_name_parts(&parts);
	if name.is_empty() {
		None
	} else if name.len() > 80 {
		Some(format!("{}...", name.chars().take(77).collect::<String>()))
	} else {
		Some(name)
	}
}

macro_rules! impl_child_bmc {
	(
		$bmc:ident,
		$model:ty,
		$create:ty,
		$update:ty,
		$table:literal,
		$parent_col:literal
	) => {
		pub struct $bmc;

		impl DbBmc for $bmc {
			const TABLE: &'static str = $table;
		}

		impl $bmc {
			pub async fn create(
				ctx: &Ctx,
				mm: &ModelManager,
				data: $create,
			) -> Result<Uuid> {
				base_uuid::create::<Self, _>(ctx, mm, data).await
			}

			pub async fn get(
				ctx: &Ctx,
				mm: &ModelManager,
				id: Uuid,
			) -> Result<$model> {
				base_uuid::get::<Self, _>(ctx, mm, id).await
			}

			pub async fn list(
				ctx: &Ctx,
				mm: &ModelManager,
				list_options: Option<ListOptions>,
			) -> Result<Vec<$model>> {
				base_uuid::list::<Self, _, Vec<PresaveListFilter>>(
					ctx,
					mm,
					None,
					list_options,
				)
				.await
			}

			pub async fn update(
				ctx: &Ctx,
				mm: &ModelManager,
				id: Uuid,
				data: $update,
			) -> Result<()> {
				base_uuid::update::<Self, _>(ctx, mm, id, data).await
			}

			pub async fn delete(
				ctx: &Ctx,
				mm: &ModelManager,
				id: Uuid,
			) -> Result<()> {
				base_uuid::delete::<Self>(ctx, mm, id).await
			}

			pub async fn list_by_parent(
				ctx: &Ctx,
				mm: &ModelManager,
				parent_id: Uuid,
			) -> Result<Vec<$model>> {
				let dbx = mm.dbx();
				dbx.begin_txn().await?;
				if let Err(err) =
					crate::model::store::set_full_context_from_ctx_dbx(dbx, ctx)
						.await
				{
					dbx.rollback_txn().await?;
					return Err(err);
				}

				let sql = format!(
					"SELECT * FROM {} WHERE {} = $1 ORDER BY sequence_number ASC, id ASC",
					Self::TABLE,
					$parent_col
				);
				let rows = match dbx
					.fetch_all(sqlx::query_as::<_, $model>(&sql).bind(parent_id))
					.await
				{
					Ok(rows) => rows,
					Err(err) => {
						dbx.rollback_txn().await?;
						return Err(err.into());
					}
				};
				dbx.commit_txn().await?;
				Ok(rows)
			}
		}
	};
}

const STUDY_PRESAVE_SPONSOR_STUDY_NUMBER_MAX_LEN: usize = 50;
const STUDY_PRESAVE_REGISTRATION_NUMBER_MAX_LEN: usize = 50;

fn validate_allowed_optional_text(
	entity: &str,
	field: &str,
	value: Option<&str>,
	allowed_values: &[&str],
) -> Result<()> {
	if let Some(value) = value {
		if !allowed_values.contains(&value) {
			return Err(crate::model::Error::Store(format!(
				"{entity} field `{field}` must be one of: {}",
				allowed_values.join(", ")
			)));
		}
	}
	Ok(())
}

fn validate_optional_text_max_len(
	entity: &str,
	field: &str,
	value: Option<&str>,
	max_len: usize,
) -> Result<()> {
	if let Some(value) = value {
		if value.chars().count() > max_len {
			return Err(crate::model::Error::Validation {
				message: format!(
					"{entity} field `{field}` must be at most {max_len} characters"
				),
			});
		}
	}
	Ok(())
}

fn normalized_text(value: Option<&str>) -> Option<String> {
	value
		.map(str::trim)
		.filter(|value| !value.is_empty())
		.map(|value| value.to_ascii_lowercase())
}

fn require_identity(condition: bool, message: &str) -> Result<()> {
	if condition {
		Ok(())
	} else {
		Err(crate::model::Error::Validation {
			message: message.to_string(),
		})
	}
}

fn duplicate_identity(message: &str) -> crate::model::Error {
	crate::model::Error::Conflict {
		message: message.to_string(),
	}
}

fn relationship_conflict(message: &str) -> crate::model::Error {
	crate::model::Error::Conflict {
		message: message.to_string(),
	}
}

fn validation_error(message: &str) -> crate::model::Error {
	crate::model::Error::Validation {
		message: message.to_string(),
	}
}

fn validate_null_flavor_set(
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

trait IntoOrgScopedCreate {
	type Insert: HasSeaFields;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert;
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SenderPresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub deleted: bool,
	pub is_default: bool,
	pub sender_type: Option<String>,
	pub organization_name: Option<String>,
	pub organization_name_notation: Option<String>,
	pub street_address: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub telephone: Option<String>,
	pub fax: Option<String>,
	pub email: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct SenderPresaveForCreate {
	pub is_default: Option<bool>,
	pub sender_type: Option<String>,
	pub organization_name: Option<String>,
	pub organization_name_notation: Option<String>,
	pub street_address: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub telephone: Option<String>,
	pub fax: Option<String>,
	pub email: Option<String>,
}

#[derive(Fields)]
struct SenderPresaveForInsert {
	organization_id: Uuid,
	is_default: Option<bool>,
	sender_type: Option<String>,
	organization_name: Option<String>,
	organization_name_notation: Option<String>,
	street_address: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	country_code: Option<String>,
	telephone: Option<String>,
	fax: Option<String>,
	email: Option<String>,
}

impl IntoOrgScopedCreate for SenderPresaveForCreate {
	type Insert = SenderPresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		SenderPresaveForInsert {
			organization_id,
			is_default: self.is_default,
			sender_type: self.sender_type,
			organization_name: self.organization_name,
			organization_name_notation: self.organization_name_notation,
			street_address: self.street_address,
			city: self.city,
			state: self.state,
			postcode: self.postcode,
			country_code: self.country_code,
			telephone: self.telephone,
			fax: self.fax,
			email: self.email,
		}
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct SenderPresaveForUpdate {
	pub deleted: Option<bool>,
	pub is_default: Option<bool>,
	pub sender_type: Option<String>,
	pub organization_name: Option<String>,
	pub organization_name_notation: Option<String>,
	pub street_address: Option<String>,
	pub city: Option<String>,
	pub state: Option<String>,
	pub postcode: Option<String>,
	pub country_code: Option<String>,
	pub telephone: Option<String>,
	pub fax: Option<String>,
	pub email: Option<String>,
}

pub struct SenderPresaveBmc;

impl DbBmc for SenderPresaveBmc {
	const TABLE: &'static str = "sender_presaves";
}

impl SenderPresaveBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: SenderPresaveForCreate,
	) -> Result<Uuid> {
		Self::ensure_sender_count_allowed(ctx, mm).await?;
		Self::validate_identity(
			data.sender_type.as_deref(),
			data.organization_name.as_deref(),
		)?;
		Self::ensure_unique_identity(
			ctx,
			mm,
			None,
			data.sender_type.as_deref(),
			data.organization_name.as_deref(),
		)
		.await?;
		base_uuid::create::<Self, _>(
			ctx,
			mm,
			data.into_insert(ctx.organization_id()),
		)
		.await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<SenderPresave> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<SenderPresave>> {
		base_uuid::list::<Self, _, Vec<PresaveListFilter>>(
			ctx,
			mm,
			None,
			list_options,
		)
		.await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: SenderPresaveForUpdate,
	) -> Result<()> {
		if data.deleted == Some(true) {
			return Err(validation_error(
				"presave deletion must use lifecycle service",
			));
		}
		let current = Self::get(ctx, mm, id).await?;
		let sender_type = data
			.sender_type
			.as_deref()
			.or(current.sender_type.as_deref());
		let organization_name = data
			.organization_name
			.as_deref()
			.or(current.organization_name.as_deref());
		Self::validate_identity(sender_type, organization_name)?;
		Self::ensure_unique_identity(
			ctx,
			mm,
			Some(id),
			sender_type,
			organization_name,
		)
		.await?;
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		PresaveLifecycleService::hard_delete(ctx, mm, PresaveKind::Sender, id).await
	}

	fn validate_identity(
		sender_type: Option<&str>,
		organization_name: Option<&str>,
	) -> Result<()> {
		require_identity(
			normalized_text(sender_type).is_some()
				&& normalized_text(organization_name).is_some(),
			"sender presave requires sender_type and organization_name",
		)
	}

	async fn ensure_sender_count_allowed(
		ctx: &Ctx,
		mm: &ModelManager,
	) -> Result<()> {
		if !ctx.is_company_sponsor_admin() {
			return Ok(());
		}

		let active_sender_count = Self::list(ctx, mm, None)
			.await?
			.into_iter()
			.filter(|row| !row.deleted)
			.count();
		if active_sender_count >= 1 {
			return Err(relationship_conflict(
				"pharmaceutical company sponsor administrators can register only one active sender presave",
			));
		}

		Ok(())
	}

	async fn ensure_unique_identity(
		ctx: &Ctx,
		mm: &ModelManager,
		excluding_id: Option<Uuid>,
		sender_type: Option<&str>,
		organization_name: Option<&str>,
	) -> Result<()> {
		let sender_type = normalized_text(sender_type);
		let organization_name = normalized_text(organization_name);
		let duplicate = Self::list(ctx, mm, None).await?.into_iter().any(|row| {
			!row.deleted
				&& Some(row.id) != excluding_id
				&& normalized_text(row.sender_type.as_deref()) == sender_type
				&& normalized_text(row.organization_name.as_deref())
					== organization_name
		});
		if duplicate {
			Err(duplicate_identity("sender presave duplicate identity"))
		} else {
			Ok(())
		}
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SenderPresaveGateway {
	pub id: Uuid,
	pub sender_presave_id: Uuid,
	pub sequence_number: i32,
	pub gateway_authority: String,
	pub sender_identifier: Option<String>,
	pub routing_identifier: Option<String>,
	pub cde_sender_identifier: Option<String>,
	pub cdr_sender_identifier: Option<String>,
	pub is_default_for_authority: bool,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct SenderPresaveGatewayForCreate {
	pub sender_presave_id: Uuid,
	pub sequence_number: i32,
	pub gateway_authority: String,
	pub sender_identifier: Option<String>,
	pub routing_identifier: Option<String>,
	pub cde_sender_identifier: Option<String>,
	pub cdr_sender_identifier: Option<String>,
	pub is_default_for_authority: Option<bool>,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct SenderPresaveGatewayForUpdate {
	pub sequence_number: Option<i32>,
	pub gateway_authority: Option<String>,
	pub sender_identifier: Option<String>,
	pub routing_identifier: Option<String>,
	pub cde_sender_identifier: Option<String>,
	pub cdr_sender_identifier: Option<String>,
	pub is_default_for_authority: Option<bool>,
	pub deleted: Option<bool>,
}

impl_child_bmc!(
	SenderPresaveGatewayBmc,
	SenderPresaveGateway,
	SenderPresaveGatewayForCreate,
	SenderPresaveGatewayForUpdate,
	"sender_presave_gateways",
	"sender_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SenderPresaveResponsiblePerson {
	pub id: Uuid,
	pub sender_presave_id: Uuid,
	pub sequence_number: i32,
	pub department: Option<String>,
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,
	pub is_default: bool,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct SenderPresaveResponsiblePersonForCreate {
	pub sender_presave_id: Uuid,
	pub sequence_number: i32,
	pub department: Option<String>,
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,
	pub is_default: Option<bool>,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct SenderPresaveResponsiblePersonForUpdate {
	pub sequence_number: Option<i32>,
	pub department: Option<String>,
	pub person_title: Option<String>,
	pub person_given_name: Option<String>,
	pub person_middle_name: Option<String>,
	pub person_family_name: Option<String>,
	pub is_default: Option<bool>,
	pub deleted: Option<bool>,
}

impl_child_bmc!(
	SenderPresaveResponsiblePersonBmc,
	SenderPresaveResponsiblePerson,
	SenderPresaveResponsiblePersonForCreate,
	SenderPresaveResponsiblePersonForUpdate,
	"sender_presave_responsible_persons",
	"sender_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ReceiverPresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub deleted: bool,
	pub receiver_type: Option<String>,
	pub organization_name: Option<String>,
	pub receiver_identifier: Option<String>,
	pub day_count_rule: Option<String>,
	pub nsae_solicited_day_count: Option<i32>,
	pub nsae_solicited_not_applicable: Option<bool>,
	pub nsae_non_solicited_day_count: Option<i32>,
	pub nsae_non_solicited_not_applicable: Option<bool>,
	pub sae_solicited_day_count: Option<i32>,
	pub sae_solicited_not_applicable: Option<bool>,
	pub sae_non_solicited_day_count: Option<i32>,
	pub sae_non_solicited_not_applicable: Option<bool>,
	pub description: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ReceiverPresaveForCreate {
	pub receiver_type: Option<String>,
	pub organization_name: Option<String>,
	pub receiver_identifier: Option<String>,
	pub day_count_rule: Option<String>,
	pub nsae_solicited_day_count: Option<i32>,
	pub nsae_solicited_not_applicable: Option<bool>,
	pub nsae_non_solicited_day_count: Option<i32>,
	pub nsae_non_solicited_not_applicable: Option<bool>,
	pub sae_solicited_day_count: Option<i32>,
	pub sae_solicited_not_applicable: Option<bool>,
	pub sae_non_solicited_day_count: Option<i32>,
	pub sae_non_solicited_not_applicable: Option<bool>,
	pub description: Option<String>,
}

#[derive(Fields)]
struct ReceiverPresaveForInsert {
	organization_id: Uuid,
	receiver_type: Option<String>,
	organization_name: Option<String>,
	receiver_identifier: Option<String>,
	day_count_rule: Option<String>,
	nsae_solicited_day_count: Option<i32>,
	nsae_solicited_not_applicable: Option<bool>,
	nsae_non_solicited_day_count: Option<i32>,
	nsae_non_solicited_not_applicable: Option<bool>,
	sae_solicited_day_count: Option<i32>,
	sae_solicited_not_applicable: Option<bool>,
	sae_non_solicited_day_count: Option<i32>,
	sae_non_solicited_not_applicable: Option<bool>,
	description: Option<String>,
}

impl IntoOrgScopedCreate for ReceiverPresaveForCreate {
	type Insert = ReceiverPresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		ReceiverPresaveForInsert {
			organization_id,
			receiver_type: self.receiver_type,
			organization_name: self.organization_name,
			receiver_identifier: self.receiver_identifier,
			day_count_rule: self.day_count_rule,
			nsae_solicited_day_count: self.nsae_solicited_day_count,
			nsae_solicited_not_applicable: self.nsae_solicited_not_applicable,
			nsae_non_solicited_day_count: self.nsae_non_solicited_day_count,
			nsae_non_solicited_not_applicable: self
				.nsae_non_solicited_not_applicable,
			sae_solicited_day_count: self.sae_solicited_day_count,
			sae_solicited_not_applicable: self.sae_solicited_not_applicable,
			sae_non_solicited_day_count: self.sae_non_solicited_day_count,
			sae_non_solicited_not_applicable: self.sae_non_solicited_not_applicable,
			description: self.description,
		}
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct ReceiverPresaveForUpdate {
	pub deleted: Option<bool>,
	pub receiver_type: Option<String>,
	pub organization_name: Option<String>,
	pub receiver_identifier: Option<String>,
	pub day_count_rule: Option<String>,
	pub nsae_solicited_day_count: Option<i32>,
	pub nsae_solicited_not_applicable: Option<bool>,
	pub nsae_non_solicited_day_count: Option<i32>,
	pub nsae_non_solicited_not_applicable: Option<bool>,
	pub sae_solicited_day_count: Option<i32>,
	pub sae_solicited_not_applicable: Option<bool>,
	pub sae_non_solicited_day_count: Option<i32>,
	pub sae_non_solicited_not_applicable: Option<bool>,
	pub description: Option<String>,
}

pub struct ReceiverPresaveBmc;

impl DbBmc for ReceiverPresaveBmc {
	const TABLE: &'static str = "receiver_presaves";
}

impl ReceiverPresaveBmc {
	const RECEIVER_TYPE_ORIGINAL_MANUFACTURER: &'static str =
		"Original Manufacturer";
	const RECEIVER_TYPE_REGULATORY_AUTHORITY: &'static str = "Regulatory Authority";

	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ReceiverPresaveForCreate,
	) -> Result<Uuid> {
		Self::validate_identity(
			data.receiver_type.as_deref(),
			data.organization_name.as_deref(),
		)?;
		Self::validate_timeline(
			data.nsae_non_solicited_day_count,
			data.nsae_non_solicited_not_applicable,
			data.sae_non_solicited_day_count,
			data.sae_non_solicited_not_applicable,
			data.nsae_solicited_day_count,
			data.nsae_solicited_not_applicable,
			data.sae_solicited_day_count,
			data.sae_solicited_not_applicable,
		)?;
		Self::ensure_unique_identity(
			ctx,
			mm,
			None,
			data.organization_name.as_deref(),
		)
		.await?;
		base_uuid::create::<Self, _>(
			ctx,
			mm,
			data.into_insert(ctx.organization_id()),
		)
		.await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ReceiverPresave> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ReceiverPresave>> {
		base_uuid::list::<Self, _, Vec<PresaveListFilter>>(
			ctx,
			mm,
			None,
			list_options,
		)
		.await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ReceiverPresaveForUpdate,
	) -> Result<()> {
		if data.deleted == Some(true) {
			return Err(validation_error(
				"presave deletion must use lifecycle service",
			));
		}
		let current = Self::get(ctx, mm, id).await?;
		let receiver_type = data.receiver_type.as_deref();
		let current_receiver_type = current.receiver_type.as_deref();
		let organization_name = data
			.organization_name
			.as_deref()
			.or(current.organization_name.as_deref());
		Self::validate_update_identity(
			receiver_type,
			current_receiver_type,
			organization_name,
		)?;
		let clear_nsae_non_solicited_day_count =
			data.nsae_non_solicited_not_applicable == Some(true);
		let clear_sae_non_solicited_day_count =
			data.sae_non_solicited_not_applicable == Some(true);
		let clear_nsae_solicited_day_count =
			data.nsae_solicited_not_applicable == Some(true);
		let clear_sae_solicited_day_count =
			data.sae_solicited_not_applicable == Some(true);
		Self::validate_timeline(
			if clear_nsae_non_solicited_day_count {
				None
			} else {
				data.nsae_non_solicited_day_count
					.or(current.nsae_non_solicited_day_count)
			},
			data.nsae_non_solicited_not_applicable
				.or(current.nsae_non_solicited_not_applicable),
			if clear_sae_non_solicited_day_count {
				None
			} else {
				data.sae_non_solicited_day_count
					.or(current.sae_non_solicited_day_count)
			},
			data.sae_non_solicited_not_applicable
				.or(current.sae_non_solicited_not_applicable),
			if clear_nsae_solicited_day_count {
				None
			} else {
				data.nsae_solicited_day_count
					.or(current.nsae_solicited_day_count)
			},
			data.nsae_solicited_not_applicable
				.or(current.nsae_solicited_not_applicable),
			if clear_sae_solicited_day_count {
				None
			} else {
				data.sae_solicited_day_count
					.or(current.sae_solicited_day_count)
			},
			data.sae_solicited_not_applicable
				.or(current.sae_solicited_not_applicable),
		)?;
		Self::ensure_unique_identity(ctx, mm, Some(id), organization_name).await?;
		base_uuid::update::<Self, _>(ctx, mm, id, data).await?;
		Self::clear_not_applicable_day_counts(
			ctx,
			mm,
			id,
			clear_nsae_non_solicited_day_count,
			clear_sae_non_solicited_day_count,
			clear_nsae_solicited_day_count,
			clear_sae_solicited_day_count,
		)
		.await?;
		Ok(())
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		PresaveLifecycleService::hard_delete(ctx, mm, PresaveKind::Receiver, id)
			.await
	}

	fn validate_identity(
		receiver_type: Option<&str>,
		organization_name: Option<&str>,
	) -> Result<()> {
		require_identity(
			normalized_text(receiver_type).is_some()
				&& normalized_text(organization_name).is_some(),
			"receiver presave requires receiver_type and organization_name",
		)?;
		match receiver_type.map(str::trim) {
			Some(Self::RECEIVER_TYPE_REGULATORY_AUTHORITY)
			| Some(Self::RECEIVER_TYPE_ORIGINAL_MANUFACTURER) => Ok(()),
			_ => Err(validation_error(
				"receiver_type must be Regulatory Authority or Original Manufacturer",
			)),
		}
	}

	fn validate_update_identity(
		receiver_type: Option<&str>,
		current_receiver_type: Option<&str>,
		organization_name: Option<&str>,
	) -> Result<()> {
		let effective_receiver_type = receiver_type.or(current_receiver_type);
		if Self::is_unchanged_legacy_receiver_type(
			receiver_type,
			current_receiver_type,
		) {
			require_identity(
				normalized_text(effective_receiver_type).is_some()
					&& normalized_text(organization_name).is_some(),
				"receiver presave requires receiver_type and organization_name",
			)?;
			return Ok(());
		}
		Self::validate_identity(effective_receiver_type, organization_name)
	}

	fn is_unchanged_legacy_receiver_type(
		receiver_type: Option<&str>,
		current_receiver_type: Option<&str>,
	) -> bool {
		let Some(current_receiver_type) = current_receiver_type.map(str::trim)
		else {
			return false;
		};
		if !Self::is_legacy_receiver_type_code(current_receiver_type) {
			return false;
		}
		match receiver_type.map(str::trim) {
			Some(receiver_type) => receiver_type == current_receiver_type,
			None => true,
		}
	}

	fn is_legacy_receiver_type_code(receiver_type: &str) -> bool {
		matches!(receiver_type, "1" | "2" | "3" | "4" | "5" | "6")
	}

	fn validate_timeline_category(
		label: &str,
		day_count: Option<i32>,
		not_applicable: Option<bool>,
	) -> Result<()> {
		if day_count.is_some_and(|value| value < 0) {
			return Err(validation_error(&format!(
				"{label} day count must be zero or greater"
			)));
		}
		if day_count.is_some() && not_applicable == Some(true) {
			return Err(validation_error(&format!(
				"{label} cannot have both day count and Not Applicable"
			)));
		}
		Ok(())
	}

	fn validate_timeline(
		nsae_spontaneous_day_count: Option<i32>,
		nsae_spontaneous_not_applicable: Option<bool>,
		sae_spontaneous_day_count: Option<i32>,
		sae_spontaneous_not_applicable: Option<bool>,
		nsae_solicited_day_count: Option<i32>,
		nsae_solicited_not_applicable: Option<bool>,
		sae_solicited_day_count: Option<i32>,
		sae_solicited_not_applicable: Option<bool>,
	) -> Result<()> {
		Self::validate_timeline_category(
			"Non-SAE Spontaneous",
			nsae_spontaneous_day_count,
			nsae_spontaneous_not_applicable,
		)?;
		Self::validate_timeline_category(
			"SAE Spontaneous",
			sae_spontaneous_day_count,
			sae_spontaneous_not_applicable,
		)?;
		Self::validate_timeline_category(
			"Non-SAE Solicited",
			nsae_solicited_day_count,
			nsae_solicited_not_applicable,
		)?;
		Self::validate_timeline_category(
			"SAE Solicited",
			sae_solicited_day_count,
			sae_solicited_not_applicable,
		)
	}

	async fn ensure_unique_identity(
		ctx: &Ctx,
		mm: &ModelManager,
		excluding_id: Option<Uuid>,
		organization_name: Option<&str>,
	) -> Result<()> {
		let organization_name = normalized_text(organization_name);
		let duplicate = Self::list(ctx, mm, None).await?.into_iter().any(|row| {
			!row.deleted
				&& Some(row.id) != excluding_id
				&& normalized_text(row.organization_name.as_deref())
					== organization_name
		});
		if duplicate {
			Err(duplicate_identity("receiver presave duplicate identity"))
		} else {
			Ok(())
		}
	}

	async fn clear_not_applicable_day_counts(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		clear_nsae_non_solicited_day_count: bool,
		clear_sae_non_solicited_day_count: bool,
		clear_nsae_solicited_day_count: bool,
		clear_sae_solicited_day_count: bool,
	) -> Result<()> {
		if !clear_nsae_non_solicited_day_count
			&& !clear_sae_non_solicited_day_count
			&& !clear_nsae_solicited_day_count
			&& !clear_sae_solicited_day_count
		{
			return Ok(());
		}

		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) = set_full_context_from_ctx_dbx(dbx, ctx).await {
			dbx.rollback_txn().await?;
			return Err(err);
		}
		let result = dbx
			.execute(
				sqlx::query(
					"UPDATE receiver_presaves SET \
					 nsae_non_solicited_day_count = CASE WHEN $2 THEN NULL ELSE nsae_non_solicited_day_count END, \
					 sae_non_solicited_day_count = CASE WHEN $3 THEN NULL ELSE sae_non_solicited_day_count END, \
					 nsae_solicited_day_count = CASE WHEN $4 THEN NULL ELSE nsae_solicited_day_count END, \
					 sae_solicited_day_count = CASE WHEN $5 THEN NULL ELSE sae_solicited_day_count END, \
					 updated_by = $6, updated_at = NOW() \
					 WHERE id = $1",
				)
				.bind(id)
				.bind(clear_nsae_non_solicited_day_count)
				.bind(clear_sae_non_solicited_day_count)
				.bind(clear_nsae_solicited_day_count)
				.bind(clear_sae_solicited_day_count)
				.bind(ctx.user_id()),
			)
			.await;
		let count = match result {
			Ok(count) => count,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(err.into());
			}
		};
		if count == 0 {
			dbx.rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		dbx.commit_txn().await?;
		Ok(())
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ReceiverPresaveConsignee {
	pub id: Uuid,
	pub receiver_presave_id: Uuid,
	pub sequence_number: i32,
	pub name: Option<String>,
	pub phone: Option<String>,
	pub email: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ReceiverPresaveConsigneeForCreate {
	pub receiver_presave_id: Uuid,
	pub sequence_number: i32,
	pub name: Option<String>,
	pub phone: Option<String>,
	pub email: Option<String>,
}

#[derive(Default, Fields, Deserialize)]
pub struct ReceiverPresaveConsigneeForUpdate {
	pub sequence_number: Option<i32>,
	pub name: Option<String>,
	pub phone: Option<String>,
	pub email: Option<String>,
}

impl_child_bmc!(
	ReceiverPresaveConsigneeBmc,
	ReceiverPresaveConsignee,
	ReceiverPresaveConsigneeForCreate,
	ReceiverPresaveConsigneeForUpdate,
	"receiver_presave_consignees",
	"receiver_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ReceiverPresaveRoute {
	pub id: Uuid,
	pub receiver_presave_id: Uuid,
	pub sequence_number: i32,
	pub authority: String,
	pub receiver_label: String,
	pub batch_receiver_identifier: Option<String>,
	pub message_receiver_identifier: String,
	pub condition_page: String,
	pub condition_field_code: String,
	pub condition_operator: String,
	pub condition_value_code: String,
	pub condition_value_label: String,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ReceiverPresaveRouteForCreate {
	pub receiver_presave_id: Uuid,
	pub sequence_number: i32,
	pub authority: String,
	pub receiver_label: String,
	pub batch_receiver_identifier: Option<String>,
	pub message_receiver_identifier: String,
	pub condition_page: String,
	pub condition_field_code: String,
	pub condition_operator: String,
	pub condition_value_code: String,
	pub condition_value_label: String,
}

#[derive(Default, Fields, Deserialize)]
pub struct ReceiverPresaveRouteForUpdate {
	pub sequence_number: Option<i32>,
	pub authority: Option<String>,
	pub receiver_label: Option<String>,
	pub batch_receiver_identifier: Option<String>,
	pub message_receiver_identifier: Option<String>,
	pub condition_page: Option<String>,
	pub condition_field_code: Option<String>,
	pub condition_operator: Option<String>,
	pub condition_value_code: Option<String>,
	pub condition_value_label: Option<String>,
}

impl_child_bmc!(
	ReceiverPresaveRouteBmc,
	ReceiverPresaveRoute,
	ReceiverPresaveRouteForCreate,
	ReceiverPresaveRouteForUpdate,
	"receiver_presave_routes",
	"receiver_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ProductPresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub deleted: bool,
	pub sender_presave_id: Option<Uuid>,
	pub receiver_presave_id: Option<Uuid>,
	pub product_id: Option<String>,
	pub medicinal_product: Option<String>,
	pub medicinal_product_notation: Option<String>,
	pub preapproval_ip_name: Option<String>,
	pub brand_name: Option<String>,
	pub original_manufacturer: Option<String>,
	pub product_description: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_mpid: Option<String>,
	pub mfds_mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub obtain_drug_country: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub drug_authorization_country: Option<String>,
	pub drug_authorization_holder: Option<String>,
	pub holder_applicant_name_notation: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductPresaveForCreate {
	pub sender_presave_id: Option<Uuid>,
	pub receiver_presave_id: Option<Uuid>,
	pub product_id: Option<String>,
	pub medicinal_product: Option<String>,
	pub medicinal_product_notation: Option<String>,
	pub preapproval_ip_name: Option<String>,
	pub brand_name: Option<String>,
	pub original_manufacturer: Option<String>,
	pub product_description: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_mpid: Option<String>,
	pub mfds_mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub obtain_drug_country: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub drug_authorization_country: Option<String>,
	pub drug_authorization_holder: Option<String>,
	pub holder_applicant_name_notation: Option<String>,
}

#[derive(Fields)]
struct ProductPresaveForInsert {
	organization_id: Uuid,
	sender_presave_id: Option<Uuid>,
	receiver_presave_id: Option<Uuid>,
	product_id: Option<String>,
	medicinal_product: Option<String>,
	medicinal_product_notation: Option<String>,
	preapproval_ip_name: Option<String>,
	brand_name: Option<String>,
	original_manufacturer: Option<String>,
	product_description: Option<String>,
	mpid: Option<String>,
	mpid_version: Option<String>,
	mfds_mpid: Option<String>,
	mfds_mpid_version: Option<String>,
	phpid: Option<String>,
	phpid_version: Option<String>,
	investigational_product_blinded: Option<bool>,
	obtain_drug_country: Option<String>,
	drug_authorization_number: Option<String>,
	drug_authorization_country: Option<String>,
	drug_authorization_holder: Option<String>,
	holder_applicant_name_notation: Option<String>,
}

impl IntoOrgScopedCreate for ProductPresaveForCreate {
	type Insert = ProductPresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		ProductPresaveForInsert {
			organization_id,
			sender_presave_id: self.sender_presave_id,
			receiver_presave_id: self.receiver_presave_id,
			product_id: self.product_id,
			medicinal_product: self.medicinal_product,
			medicinal_product_notation: self.medicinal_product_notation,
			preapproval_ip_name: self.preapproval_ip_name,
			brand_name: self.brand_name,
			original_manufacturer: self.original_manufacturer,
			product_description: self.product_description,
			mpid: self.mpid,
			mpid_version: self.mpid_version,
			mfds_mpid: self.mfds_mpid,
			mfds_mpid_version: self.mfds_mpid_version,
			phpid: self.phpid,
			phpid_version: self.phpid_version,
			investigational_product_blinded: self.investigational_product_blinded,
			obtain_drug_country: self.obtain_drug_country,
			drug_authorization_number: self.drug_authorization_number,
			drug_authorization_country: self.drug_authorization_country,
			drug_authorization_holder: self.drug_authorization_holder,
			holder_applicant_name_notation: self.holder_applicant_name_notation,
		}
	}
}

#[derive(Default, Fields, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductPresaveForUpdate {
	pub deleted: Option<bool>,
	pub sender_presave_id: Option<Uuid>,
	pub receiver_presave_id: Option<Uuid>,
	pub product_id: Option<String>,
	pub medicinal_product: Option<String>,
	pub medicinal_product_notation: Option<String>,
	pub preapproval_ip_name: Option<String>,
	pub brand_name: Option<String>,
	pub original_manufacturer: Option<String>,
	pub product_description: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_mpid: Option<String>,
	pub mfds_mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub obtain_drug_country: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub drug_authorization_country: Option<String>,
	pub drug_authorization_holder: Option<String>,
	pub holder_applicant_name_notation: Option<String>,
}

pub struct ProductPresaveBmc;

impl DbBmc for ProductPresaveBmc {
	const TABLE: &'static str = "product_presaves";
}

impl ProductPresaveBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ProductPresaveForCreate,
	) -> Result<Uuid> {
		Self::ensure_sender_assignment_allowed(ctx, data.sender_presave_id)?;
		Self::ensure_receiver_assignment_allowed(ctx, mm, data.receiver_presave_id)
			.await?;
		Self::validate_identity(
			data.sender_presave_id,
			data.product_id.as_deref(),
			data.preapproval_ip_name.as_deref(),
		)?;
		Self::ensure_unique_identity(
			ctx,
			mm,
			None,
			data.sender_presave_id,
			data.product_id.as_deref(),
			data.preapproval_ip_name.as_deref(),
		)
		.await?;
		base_uuid::create::<Self, _>(
			ctx,
			mm,
			data.into_insert(ctx.organization_id()),
		)
		.await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ProductPresave> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ProductPresave>> {
		base_uuid::list::<Self, _, Vec<PresaveListFilter>>(
			ctx,
			mm,
			None,
			list_options,
		)
		.await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ProductPresaveForUpdate,
	) -> Result<()> {
		if data.deleted == Some(true) {
			return Err(validation_error(
				"presave deletion must use lifecycle service",
			));
		}
		Self::ensure_sender_assignment_allowed(ctx, data.sender_presave_id)?;
		Self::ensure_receiver_assignment_allowed(ctx, mm, data.receiver_presave_id)
			.await?;
		{
			let current = Self::get(ctx, mm, id).await?;
			let sender_presave_id =
				data.sender_presave_id.or(current.sender_presave_id);
			let product_id =
				data.product_id.as_deref().or(current.product_id.as_deref());
			let preapproval_ip_name = data
				.preapproval_ip_name
				.as_deref()
				.or(current.preapproval_ip_name.as_deref());
			Self::validate_identity(
				sender_presave_id,
				product_id,
				preapproval_ip_name,
			)?;
			Self::ensure_unique_identity(
				ctx,
				mm,
				Some(id),
				sender_presave_id,
				product_id,
				preapproval_ip_name,
			)
			.await?;
		}
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		PresaveLifecycleService::hard_delete(ctx, mm, PresaveKind::Product, id).await
	}

	fn validate_identity(
		sender_presave_id: Option<Uuid>,
		product_id: Option<&str>,
		preapproval_ip_name: Option<&str>,
	) -> Result<()> {
		require_identity(
			sender_presave_id.is_some()
				&& (normalized_text(product_id).is_some()
					|| normalized_text(preapproval_ip_name).is_some()),
			"product presave requires sender_presave_id and product_id or preapproval_ip_name",
		)
	}

	fn ensure_sender_assignment_allowed(
		ctx: &Ctx,
		sender_presave_id: Option<Uuid>,
	) -> Result<()> {
		if sender_presave_id.is_some() && !ctx.is_cro_sponsor_admin() {
			return Err(relationship_conflict(
				"only CRO sponsor administrators can set product sender presaves",
			));
		}

		Ok(())
	}

	async fn ensure_receiver_assignment_allowed(
		ctx: &Ctx,
		mm: &ModelManager,
		receiver_presave_id: Option<Uuid>,
	) -> Result<()> {
		let Some(receiver_id) = receiver_presave_id else {
			return Ok(());
		};
		let receiver = ReceiverPresaveBmc::get(ctx, mm, receiver_id)
			.await
			.map_err(|_| {
				relationship_conflict(
					"product requires an active receiver presave in the same organization",
				)
			})?;
		if receiver.deleted || receiver.organization_id != ctx.organization_id() {
			return Err(relationship_conflict(
				"product requires an active receiver presave in the same organization",
			));
		}
		Ok(())
	}

	async fn ensure_unique_identity(
		ctx: &Ctx,
		mm: &ModelManager,
		excluding_id: Option<Uuid>,
		sender_presave_id: Option<Uuid>,
		product_id: Option<&str>,
		preapproval_ip_name: Option<&str>,
	) -> Result<()> {
		let product_id = normalized_text(product_id);
		let preapproval_ip_name = normalized_text(preapproval_ip_name);
		let duplicate = Self::list(ctx, mm, None).await?.into_iter().any(|row| {
			!row.deleted
				&& Some(row.id) != excluding_id
				&& row.sender_presave_id == sender_presave_id
				&& ((product_id.is_some()
					&& normalized_text(row.product_id.as_deref()) == product_id)
					|| (preapproval_ip_name.is_some()
						&& normalized_text(row.preapproval_ip_name.as_deref())
							== preapproval_ip_name))
		});
		if duplicate {
			Err(duplicate_identity("product presave duplicate identity"))
		} else {
			Ok(())
		}
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ProductPresaveActiveSubstance {
	pub id: Uuid,
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub substance_name: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductPresaveActiveSubstanceForCreate {
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub substance_name: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

#[derive(Default, Fields, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductPresaveActiveSubstanceForUpdate {
	pub sequence_number: Option<i32>,
	pub substance_name: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

impl_child_bmc!(
	ProductPresaveActiveSubstanceBmc,
	ProductPresaveActiveSubstance,
	ProductPresaveActiveSubstanceForCreate,
	ProductPresaveActiveSubstanceForUpdate,
	"product_presave_active_substances",
	"product_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ReporterPresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub deleted: bool,
	pub reporter_title: Option<String>,
	pub reporter_title_null_flavor: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_given_name_null_flavor: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_middle_name_null_flavor: Option<String>,
	pub reporter_family_name: Option<String>,
	pub reporter_family_name_null_flavor: Option<String>,
	pub organization: Option<String>,
	pub organization_null_flavor: Option<String>,
	pub department: Option<String>,
	pub department_null_flavor: Option<String>,
	pub street: Option<String>,
	pub street_null_flavor: Option<String>,
	pub city: Option<String>,
	pub city_null_flavor: Option<String>,
	pub state: Option<String>,
	pub state_null_flavor: Option<String>,
	pub postcode: Option<String>,
	pub postcode_null_flavor: Option<String>,
	pub telephone: Option<String>,
	pub telephone_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub qualification: Option<String>,
	// MFDS.C.2.r.4.KR.1 - Other health professional type
	pub qualification_kr1: Option<String>,
	pub primary_source_regulatory: Option<String>,
	pub country_code_null_flavor: Option<String>,
	pub qualification_null_flavor: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Default, Deserialize)]
pub struct ReporterPresaveForCreate {
	pub reporter_title: Option<String>,
	pub reporter_title_null_flavor: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_given_name_null_flavor: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_middle_name_null_flavor: Option<String>,
	pub reporter_family_name: Option<String>,
	pub reporter_family_name_null_flavor: Option<String>,
	pub organization: Option<String>,
	pub organization_null_flavor: Option<String>,
	pub department: Option<String>,
	pub department_null_flavor: Option<String>,
	pub street: Option<String>,
	pub street_null_flavor: Option<String>,
	pub city: Option<String>,
	pub city_null_flavor: Option<String>,
	pub state: Option<String>,
	pub state_null_flavor: Option<String>,
	pub postcode: Option<String>,
	pub postcode_null_flavor: Option<String>,
	pub telephone: Option<String>,
	pub telephone_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub qualification: Option<String>,
	// MFDS.C.2.r.4.KR.1 - Other health professional type
	pub qualification_kr1: Option<String>,
	pub primary_source_regulatory: Option<String>,
	pub country_code_null_flavor: Option<String>,
	pub qualification_null_flavor: Option<String>,
}

#[derive(Fields)]
struct ReporterPresaveForInsert {
	organization_id: Uuid,
	reporter_title: Option<String>,
	reporter_title_null_flavor: Option<String>,
	reporter_given_name: Option<String>,
	reporter_given_name_null_flavor: Option<String>,
	reporter_middle_name: Option<String>,
	reporter_middle_name_null_flavor: Option<String>,
	reporter_family_name: Option<String>,
	reporter_family_name_null_flavor: Option<String>,
	organization: Option<String>,
	organization_null_flavor: Option<String>,
	department: Option<String>,
	department_null_flavor: Option<String>,
	street: Option<String>,
	street_null_flavor: Option<String>,
	city: Option<String>,
	city_null_flavor: Option<String>,
	state: Option<String>,
	state_null_flavor: Option<String>,
	postcode: Option<String>,
	postcode_null_flavor: Option<String>,
	telephone: Option<String>,
	telephone_null_flavor: Option<String>,
	country_code: Option<String>,
	qualification: Option<String>,
	qualification_kr1: Option<String>,
	primary_source_regulatory: Option<String>,
	country_code_null_flavor: Option<String>,
	qualification_null_flavor: Option<String>,
}

impl IntoOrgScopedCreate for ReporterPresaveForCreate {
	type Insert = ReporterPresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		ReporterPresaveForInsert {
			organization_id,
			reporter_title: self.reporter_title,
			reporter_title_null_flavor: self.reporter_title_null_flavor,
			reporter_given_name: self.reporter_given_name,
			reporter_given_name_null_flavor: self.reporter_given_name_null_flavor,
			reporter_middle_name: self.reporter_middle_name,
			reporter_middle_name_null_flavor: self.reporter_middle_name_null_flavor,
			reporter_family_name: self.reporter_family_name,
			reporter_family_name_null_flavor: self.reporter_family_name_null_flavor,
			organization: self.organization,
			organization_null_flavor: self.organization_null_flavor,
			department: self.department,
			department_null_flavor: self.department_null_flavor,
			street: self.street,
			street_null_flavor: self.street_null_flavor,
			city: self.city,
			city_null_flavor: self.city_null_flavor,
			state: self.state,
			state_null_flavor: self.state_null_flavor,
			postcode: self.postcode,
			postcode_null_flavor: self.postcode_null_flavor,
			telephone: self.telephone,
			telephone_null_flavor: self.telephone_null_flavor,
			country_code: self.country_code,
			qualification: self.qualification,
			qualification_kr1: self.qualification_kr1,
			primary_source_regulatory: self.primary_source_regulatory,
			country_code_null_flavor: self.country_code_null_flavor,
			qualification_null_flavor: self.qualification_null_flavor,
		}
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct ReporterPresaveForUpdate {
	pub deleted: Option<bool>,
	pub reporter_title: Option<String>,
	pub reporter_title_null_flavor: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_given_name_null_flavor: Option<String>,
	pub reporter_middle_name: Option<String>,
	pub reporter_middle_name_null_flavor: Option<String>,
	pub reporter_family_name: Option<String>,
	pub reporter_family_name_null_flavor: Option<String>,
	pub organization: Option<String>,
	pub organization_null_flavor: Option<String>,
	pub department: Option<String>,
	pub department_null_flavor: Option<String>,
	pub street: Option<String>,
	pub street_null_flavor: Option<String>,
	pub city: Option<String>,
	pub city_null_flavor: Option<String>,
	pub state: Option<String>,
	pub state_null_flavor: Option<String>,
	pub postcode: Option<String>,
	pub postcode_null_flavor: Option<String>,
	pub telephone: Option<String>,
	pub telephone_null_flavor: Option<String>,
	pub country_code: Option<String>,
	pub qualification: Option<String>,
	// MFDS.C.2.r.4.KR.1 - Other health professional type
	pub qualification_kr1: Option<String>,
	pub primary_source_regulatory: Option<String>,
	pub country_code_null_flavor: Option<String>,
	pub qualification_null_flavor: Option<String>,
}

pub struct ReporterPresaveBmc;

impl DbBmc for ReporterPresaveBmc {
	const TABLE: &'static str = "reporter_presaves";
}

impl ReporterPresaveBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ReporterPresaveForCreate,
	) -> Result<Uuid> {
		Self::validate_null_flavors(
			data.reporter_title_null_flavor.as_deref(),
			data.reporter_given_name_null_flavor.as_deref(),
			data.reporter_middle_name_null_flavor.as_deref(),
			data.reporter_family_name_null_flavor.as_deref(),
			data.organization_null_flavor.as_deref(),
			data.department_null_flavor.as_deref(),
			data.street_null_flavor.as_deref(),
			data.city_null_flavor.as_deref(),
			data.state_null_flavor.as_deref(),
			data.postcode_null_flavor.as_deref(),
			data.telephone_null_flavor.as_deref(),
			data.country_code_null_flavor.as_deref(),
			data.qualification_null_flavor.as_deref(),
		)?;
		Self::validate_identity(
			data.reporter_given_name.as_deref(),
			data.reporter_given_name_null_flavor.as_deref(),
			data.organization.as_deref(),
			data.organization_null_flavor.as_deref(),
			data.qualification.as_deref(),
			data.qualification_null_flavor.as_deref(),
		)?;
		Self::ensure_unique_identity(
			ctx,
			mm,
			None,
			data.reporter_given_name.as_deref(),
			data.organization.as_deref(),
			data.qualification.as_deref(),
		)
		.await?;
		base_uuid::create::<Self, _>(
			ctx,
			mm,
			data.into_insert(ctx.organization_id()),
		)
		.await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ReporterPresave> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ReporterPresave>> {
		base_uuid::list::<Self, _, Vec<PresaveListFilter>>(
			ctx,
			mm,
			None,
			list_options,
		)
		.await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: ReporterPresaveForUpdate,
	) -> Result<()> {
		if data.deleted == Some(true) {
			return Err(validation_error(
				"presave deletion must use lifecycle service",
			));
		}
		Self::validate_null_flavors(
			data.reporter_title_null_flavor.as_deref(),
			data.reporter_given_name_null_flavor.as_deref(),
			data.reporter_middle_name_null_flavor.as_deref(),
			data.reporter_family_name_null_flavor.as_deref(),
			data.organization_null_flavor.as_deref(),
			data.department_null_flavor.as_deref(),
			data.street_null_flavor.as_deref(),
			data.city_null_flavor.as_deref(),
			data.state_null_flavor.as_deref(),
			data.postcode_null_flavor.as_deref(),
			data.telephone_null_flavor.as_deref(),
			data.country_code_null_flavor.as_deref(),
			data.qualification_null_flavor.as_deref(),
		)?;
		if data.deleted != Some(true) {
			let current = Self::get(ctx, mm, id).await?;
			let reporter_given_name = data
				.reporter_given_name
				.as_deref()
				.or(current.reporter_given_name.as_deref());
			let organization = data
				.organization
				.as_deref()
				.or(current.organization.as_deref());
			let qualification = data
				.qualification
				.as_deref()
				.or(current.qualification.as_deref());
			Self::validate_identity(
				reporter_given_name,
				data.reporter_given_name_null_flavor
					.as_deref()
					.or(current.reporter_given_name_null_flavor.as_deref()),
				organization,
				data.organization_null_flavor
					.as_deref()
					.or(current.organization_null_flavor.as_deref()),
				qualification,
				data.qualification_null_flavor
					.as_deref()
					.or(current.qualification_null_flavor.as_deref()),
			)?;
			Self::ensure_unique_identity(
				ctx,
				mm,
				Some(id),
				reporter_given_name,
				organization,
				qualification,
			)
			.await?;
		}
		base_uuid::update::<Self, _>(ctx, mm, id, data).await?;
		Ok(())
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		PresaveLifecycleService::hard_delete(ctx, mm, PresaveKind::Reporter, id)
			.await
	}

	fn validate_identity(
		reporter_given_name: Option<&str>,
		reporter_given_name_null_flavor: Option<&str>,
		organization: Option<&str>,
		organization_null_flavor: Option<&str>,
		qualification: Option<&str>,
		qualification_null_flavor: Option<&str>,
	) -> Result<()> {
		require_identity(
			(normalized_text(reporter_given_name).is_some()
				|| normalized_text(reporter_given_name_null_flavor).is_some())
				&& (normalized_text(organization).is_some()
					|| normalized_text(organization_null_flavor).is_some())
				&& (normalized_text(qualification).is_some()
					|| normalized_text(qualification_null_flavor).is_some()),
			"reporter presave requires reporter_given_name, organization, and qualification values or nullFlavors",
		)
	}

	fn validate_null_flavors(
		reporter_title_null_flavor: Option<&str>,
		reporter_given_name_null_flavor: Option<&str>,
		reporter_middle_name_null_flavor: Option<&str>,
		reporter_family_name_null_flavor: Option<&str>,
		organization_null_flavor: Option<&str>,
		department_null_flavor: Option<&str>,
		street_null_flavor: Option<&str>,
		city_null_flavor: Option<&str>,
		state_null_flavor: Option<&str>,
		postcode_null_flavor: Option<&str>,
		telephone_null_flavor: Option<&str>,
		country_code_null_flavor: Option<&str>,
		qualification_null_flavor: Option<&str>,
	) -> Result<()> {
		const ELEMENT_ALLOWED: &[NullFlavor] =
			&[NullFlavor::MSK, NullFlavor::ASKU, NullFlavor::NASK];
		const TITLE_ALLOWED: &[NullFlavor] = &[
			NullFlavor::MSK,
			NullFlavor::UNK,
			NullFlavor::ASKU,
			NullFlavor::NASK,
		];
		// C.2.r.3 country additionally permits UNK per the ICH dictionary.
		const COUNTRY_ALLOWED: &[NullFlavor] = &[
			NullFlavor::MSK,
			NullFlavor::UNK,
			NullFlavor::ASKU,
			NullFlavor::NASK,
		];
		const QUALIFICATION_ALLOWED: &[NullFlavor] = &[NullFlavor::UNK];

		validate_null_flavor_set(
			"reporter_title_null_flavor",
			reporter_title_null_flavor,
			TITLE_ALLOWED,
		)?;
		for (field, value) in [
			(
				"reporter_given_name_null_flavor",
				reporter_given_name_null_flavor,
			),
			(
				"reporter_middle_name_null_flavor",
				reporter_middle_name_null_flavor,
			),
			(
				"reporter_family_name_null_flavor",
				reporter_family_name_null_flavor,
			),
			("organization_null_flavor", organization_null_flavor),
			("department_null_flavor", department_null_flavor),
			("street_null_flavor", street_null_flavor),
			("city_null_flavor", city_null_flavor),
			("state_null_flavor", state_null_flavor),
			("postcode_null_flavor", postcode_null_flavor),
			("telephone_null_flavor", telephone_null_flavor),
		] {
			validate_null_flavor_set(field, value, ELEMENT_ALLOWED)?;
		}
		validate_null_flavor_set(
			"country_code_null_flavor",
			country_code_null_flavor,
			COUNTRY_ALLOWED,
		)?;
		validate_null_flavor_set(
			"qualification_null_flavor",
			qualification_null_flavor,
			QUALIFICATION_ALLOWED,
		)
	}

	async fn ensure_unique_identity(
		ctx: &Ctx,
		mm: &ModelManager,
		excluding_id: Option<Uuid>,
		reporter_given_name: Option<&str>,
		organization: Option<&str>,
		qualification: Option<&str>,
	) -> Result<()> {
		let reporter_given_name = normalized_text(reporter_given_name);
		let organization = normalized_text(organization);
		let qualification = normalized_text(qualification);
		let duplicate = Self::list(ctx, mm, None).await?.into_iter().any(|row| {
			!row.deleted
				&& Some(row.id) != excluding_id
				&& normalized_text(row.reporter_given_name.as_deref())
					== reporter_given_name
				&& normalized_text(row.organization.as_deref()) == organization
				&& normalized_text(row.qualification.as_deref()) == qualification
		});
		if duplicate {
			Err(duplicate_identity("reporter presave duplicate identity"))
		} else {
			Ok(())
		}
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyPresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub deleted: bool,
	pub product_presave_id: Option<Uuid>,
	pub study_name: Option<String>,
	pub study_name_notation: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_kind: Option<String>,
	pub study_type_reaction: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub edc_sync: Option<bool>,
	pub exclude_case_key_from_sync: Option<bool>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct StudyPresaveForCreate {
	pub product_presave_id: Option<Uuid>,
	pub study_name: Option<String>,
	pub study_name_notation: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_kind: Option<String>,
	pub study_type_reaction: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub edc_sync: Option<bool>,
	pub exclude_case_key_from_sync: Option<bool>,
}

#[derive(Fields)]
struct StudyPresaveForInsert {
	organization_id: Uuid,
	product_presave_id: Option<Uuid>,
	study_name: Option<String>,
	study_name_notation: Option<String>,
	sponsor_study_number: Option<String>,
	sponsor_study_number_kind: Option<String>,
	study_type_reaction: Option<String>,
	fda_ind_number_occurred: Option<String>,
	fda_pre_anda_number_occurred: Option<String>,
	edc_sync: Option<bool>,
	exclude_case_key_from_sync: Option<bool>,
}

impl IntoOrgScopedCreate for StudyPresaveForCreate {
	type Insert = StudyPresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		StudyPresaveForInsert {
			organization_id,
			product_presave_id: self.product_presave_id,
			study_name: self.study_name,
			study_name_notation: self.study_name_notation,
			sponsor_study_number: self.sponsor_study_number,
			sponsor_study_number_kind: self.sponsor_study_number_kind,
			study_type_reaction: self.study_type_reaction,
			fda_ind_number_occurred: self.fda_ind_number_occurred,
			fda_pre_anda_number_occurred: self.fda_pre_anda_number_occurred,
			edc_sync: self.edc_sync,
			exclude_case_key_from_sync: self.exclude_case_key_from_sync,
		}
	}
}

impl StudyPresaveForCreate {
	fn validate_fields(&self) -> Result<()> {
		validate_sponsor_study_number_kind(
			self.sponsor_study_number_kind.as_deref(),
		)?;
		validate_optional_text_max_len(
			"study presave",
			"sponsor_study_number",
			self.sponsor_study_number.as_deref(),
			STUDY_PRESAVE_SPONSOR_STUDY_NUMBER_MAX_LEN,
		)
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct StudyPresaveForUpdate {
	pub deleted: Option<bool>,
	pub product_presave_id: Option<Uuid>,
	pub study_name: Option<String>,
	pub study_name_notation: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_kind: Option<String>,
	pub study_type_reaction: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub edc_sync: Option<bool>,
	pub exclude_case_key_from_sync: Option<bool>,
}

impl StudyPresaveForUpdate {
	fn validate_fields(&self) -> Result<()> {
		validate_sponsor_study_number_kind(
			self.sponsor_study_number_kind.as_deref(),
		)?;
		validate_optional_text_max_len(
			"study presave",
			"sponsor_study_number",
			self.sponsor_study_number.as_deref(),
			STUDY_PRESAVE_SPONSOR_STUDY_NUMBER_MAX_LEN,
		)
	}
}

fn validate_sponsor_study_number_kind(value: Option<&str>) -> Result<()> {
	validate_allowed_optional_text(
		"study presave",
		"sponsor_study_number_kind",
		value,
		&["STUDY_NO", "PROTOCOL_NO"],
	)
}

pub struct StudyPresaveBmc;

impl DbBmc for StudyPresaveBmc {
	const TABLE: &'static str = "study_presaves";
}

impl StudyPresaveBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: StudyPresaveForCreate,
	) -> Result<Uuid> {
		data.validate_fields()?;
		Self::validate_identity(
			data.product_presave_id,
			data.sponsor_study_number.as_deref(),
			data.study_name.as_deref(),
			data.study_type_reaction.as_deref(),
		)?;
		Self::ensure_unique_identity(
			ctx,
			mm,
			None,
			data.product_presave_id,
			data.sponsor_study_number.as_deref(),
		)
		.await?;
		base_uuid::create::<Self, _>(
			ctx,
			mm,
			data.into_insert(ctx.organization_id()),
		)
		.await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<StudyPresave> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<StudyPresave>> {
		base_uuid::list::<Self, _, Vec<PresaveListFilter>>(
			ctx,
			mm,
			None,
			list_options,
		)
		.await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: StudyPresaveForUpdate,
	) -> Result<()> {
		data.validate_fields()?;
		if data.deleted == Some(true) {
			return Err(validation_error(
				"presave deletion must use lifecycle service",
			));
		}
		if data.deleted != Some(true) {
			let current = Self::get(ctx, mm, id).await?;
			let product_presave_id =
				data.product_presave_id.or(current.product_presave_id);
			let sponsor_study_number = data
				.sponsor_study_number
				.as_deref()
				.or(current.sponsor_study_number.as_deref());
			let study_name =
				data.study_name.as_deref().or(current.study_name.as_deref());
			let study_type_reaction = data
				.study_type_reaction
				.as_deref()
				.or(current.study_type_reaction.as_deref());
			Self::validate_identity(
				product_presave_id,
				sponsor_study_number,
				study_name,
				study_type_reaction,
			)?;
			Self::ensure_unique_identity(
				ctx,
				mm,
				Some(id),
				product_presave_id,
				sponsor_study_number,
			)
			.await?;
		}
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		PresaveLifecycleService::hard_delete(ctx, mm, PresaveKind::Study, id).await
	}

	fn validate_identity(
		product_presave_id: Option<Uuid>,
		sponsor_study_number: Option<&str>,
		study_name: Option<&str>,
		study_type_reaction: Option<&str>,
	) -> Result<()> {
		require_identity(
			product_presave_id.is_some()
				&& normalized_text(sponsor_study_number).is_some()
				&& normalized_text(study_name).is_some()
				&& normalized_text(study_type_reaction).is_some(),
			"study presave requires product_presave_id, sponsor_study_number, study_name, and study_type_reaction",
		)
	}

	async fn ensure_unique_identity(
		ctx: &Ctx,
		mm: &ModelManager,
		excluding_id: Option<Uuid>,
		product_presave_id: Option<Uuid>,
		sponsor_study_number: Option<&str>,
	) -> Result<()> {
		let sponsor_study_number = normalized_text(sponsor_study_number);
		let duplicate = Self::list(ctx, mm, None).await?.into_iter().any(|row| {
			!row.deleted
				&& Some(row.id) != excluding_id
				&& row.product_presave_id == product_presave_id
				&& normalized_text(row.sponsor_study_number.as_deref())
					== sponsor_study_number
		});
		if duplicate {
			Err(duplicate_identity("study presave duplicate identity"))
		} else {
			Ok(())
		}
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyPresaveRegistrationNumber {
	pub id: Uuid,
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub registration_number: Option<String>,
	pub country_code: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyPresaveRegistrationNumberForCreate {
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub registration_number: Option<String>,
	pub country_code: Option<String>,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct StudyPresaveRegistrationNumberForUpdate {
	pub sequence_number: Option<i32>,
	pub registration_number: Option<String>,
	pub country_code: Option<String>,
	pub deleted: Option<bool>,
}

impl StudyPresaveRegistrationNumberForCreate {
	fn validate_fields(&self) -> Result<()> {
		validate_optional_text_max_len(
			"study presave registration number",
			"registration_number",
			self.registration_number.as_deref(),
			STUDY_PRESAVE_REGISTRATION_NUMBER_MAX_LEN,
		)
	}
}

impl StudyPresaveRegistrationNumberForUpdate {
	fn validate_fields(&self) -> Result<()> {
		validate_optional_text_max_len(
			"study presave registration number",
			"registration_number",
			self.registration_number.as_deref(),
			STUDY_PRESAVE_REGISTRATION_NUMBER_MAX_LEN,
		)
	}
}

pub struct StudyPresaveRegistrationNumberBmc;

impl DbBmc for StudyPresaveRegistrationNumberBmc {
	const TABLE: &'static str = "study_presave_registration_numbers";
}

impl StudyPresaveRegistrationNumberBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: StudyPresaveRegistrationNumberForCreate,
	) -> Result<Uuid> {
		data.validate_fields()?;
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<StudyPresaveRegistrationNumber> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<StudyPresaveRegistrationNumber>> {
		base_uuid::list::<Self, _, Vec<PresaveListFilter>>(
			ctx,
			mm,
			None,
			list_options,
		)
		.await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: StudyPresaveRegistrationNumberForUpdate,
	) -> Result<()> {
		data.validate_fields()?;
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		PresaveLifecycleService::hard_delete(ctx, mm, PresaveKind::Narrative, id)
			.await
	}

	pub async fn list_by_parent(
		ctx: &Ctx,
		mm: &ModelManager,
		parent_id: Uuid,
	) -> Result<Vec<StudyPresaveRegistrationNumber>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) =
			crate::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
		{
			dbx.rollback_txn().await?;
			return Err(err);
		}

		let sql = "SELECT * FROM study_presave_registration_numbers WHERE study_presave_id = $1 ORDER BY sequence_number ASC, id ASC";
		let rows = match dbx
			.fetch_all(
				sqlx::query_as::<_, StudyPresaveRegistrationNumber>(sql)
					.bind(parent_id),
			)
			.await
		{
			Ok(rows) => rows,
			Err(err) => {
				dbx.rollback_txn().await?;
				return Err(err.into());
			}
		};
		dbx.commit_txn().await?;
		Ok(rows)
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyPresaveFdaCrossReportedIndNumber {
	pub id: Uuid,
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub ind_number: String,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyPresaveFdaCrossReportedIndNumberForCreate {
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub ind_number: String,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct StudyPresaveFdaCrossReportedIndNumberForUpdate {
	pub sequence_number: Option<i32>,
	pub ind_number: Option<String>,
	pub deleted: Option<bool>,
}

impl_child_bmc!(
	StudyPresaveFdaCrossReportedIndNumberBmc,
	StudyPresaveFdaCrossReportedIndNumber,
	StudyPresaveFdaCrossReportedIndNumberForCreate,
	StudyPresaveFdaCrossReportedIndNumberForUpdate,
	"study_presave_fda_cross_reported_ind_numbers",
	"study_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyPresaveProduct {
	pub id: Uuid,
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub product_presave_id: Option<Uuid>,
	pub product_name: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyPresaveProductForCreate {
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub product_presave_id: Option<Uuid>,
	pub product_name: Option<String>,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct StudyPresaveProductForUpdate {
	pub sequence_number: Option<i32>,
	pub product_presave_id: Option<Uuid>,
	pub product_name: Option<String>,
	pub deleted: Option<bool>,
}

impl_child_bmc!(
	StudyPresaveProductBmc,
	StudyPresaveProduct,
	StudyPresaveProductForCreate,
	StudyPresaveProductForUpdate,
	"study_presave_products",
	"study_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyPresaveReporter {
	pub id: Uuid,
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub reporter_presave_id: Option<Uuid>,
	pub reporter_organization: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_qualification: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyPresaveReporterForCreate {
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub reporter_presave_id: Option<Uuid>,
	pub reporter_organization: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_qualification: Option<String>,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct StudyPresaveReporterForUpdate {
	pub sequence_number: Option<i32>,
	pub reporter_presave_id: Option<Uuid>,
	pub reporter_organization: Option<String>,
	pub reporter_given_name: Option<String>,
	pub reporter_qualification: Option<String>,
	pub deleted: Option<bool>,
}

impl_child_bmc!(
	StudyPresaveReporterBmc,
	StudyPresaveReporter,
	StudyPresaveReporterForCreate,
	StudyPresaveReporterForUpdate,
	"study_presave_reporters",
	"study_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct NarrativePresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub deleted: bool,
	pub case_narrative: Option<String>,
	pub case_narrative_notation: Option<String>,
	pub additional_information: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct NarrativePresaveForCreate {
	pub case_narrative: Option<String>,
	pub case_narrative_notation: Option<String>,
	pub additional_information: Option<String>,
}

#[derive(Fields)]
struct NarrativePresaveForInsert {
	organization_id: Uuid,
	case_narrative: Option<String>,
	case_narrative_notation: Option<String>,
	additional_information: Option<String>,
}

impl IntoOrgScopedCreate for NarrativePresaveForCreate {
	type Insert = NarrativePresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		NarrativePresaveForInsert {
			organization_id,
			case_narrative: self.case_narrative,
			case_narrative_notation: self.case_narrative_notation,
			additional_information: self.additional_information,
		}
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct NarrativePresaveForUpdate {
	pub deleted: Option<bool>,
	pub case_narrative: Option<String>,
	pub case_narrative_notation: Option<String>,
	pub additional_information: Option<String>,
}

pub struct NarrativePresaveBmc;

impl DbBmc for NarrativePresaveBmc {
	const TABLE: &'static str = "narrative_presaves";
}

impl NarrativePresaveBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: NarrativePresaveForCreate,
	) -> Result<Uuid> {
		let identity = narrative_presave_identity(
			data.case_narrative.as_deref(),
			data.additional_information.as_deref(),
		);
		Self::validate_identity(identity.as_deref())?;
		Self::ensure_unique_identity(ctx, mm, None, identity.as_deref()).await?;
		base_uuid::create::<Self, _>(
			ctx,
			mm,
			data.into_insert(ctx.organization_id()),
		)
		.await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<NarrativePresave> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<NarrativePresave>> {
		base_uuid::list::<Self, _, Vec<PresaveListFilter>>(
			ctx,
			mm,
			None,
			list_options,
		)
		.await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: NarrativePresaveForUpdate,
	) -> Result<()> {
		if data.deleted == Some(true) {
			return Err(validation_error(
				"presave deletion must use lifecycle service",
			));
		}
		if data.deleted != Some(true) {
			let current = Self::get(ctx, mm, id).await?;
			let case_narrative = data
				.case_narrative
				.as_deref()
				.or(current.case_narrative.as_deref());
			let additional_information = data
				.additional_information
				.as_deref()
				.or(current.additional_information.as_deref());
			let identity =
				narrative_presave_identity(case_narrative, additional_information);
			Self::validate_identity(identity.as_deref())?;
			Self::ensure_unique_identity(ctx, mm, Some(id), identity.as_deref())
				.await?;
		}
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}

	fn validate_identity(name: Option<&str>) -> Result<()> {
		require_identity(
			normalized_text(name).is_some(),
			"narrative presave requires name",
		)
	}

	async fn ensure_unique_identity(
		ctx: &Ctx,
		mm: &ModelManager,
		excluding_id: Option<Uuid>,
		name: Option<&str>,
	) -> Result<()> {
		let name = normalized_text(name);
		let duplicate = Self::list(ctx, mm, None).await?.into_iter().any(|row| {
			let row_identity = narrative_presave_identity(
				row.case_narrative.as_deref(),
				row.additional_information.as_deref(),
			);
			!row.deleted
				&& Some(row.id) != excluding_id
				&& normalized_text(row_identity.as_deref()) == name
		});
		if duplicate {
			Err(duplicate_identity("narrative presave duplicate identity"))
		} else {
			Ok(())
		}
	}
}
