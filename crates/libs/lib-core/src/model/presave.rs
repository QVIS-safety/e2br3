use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
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

trait IntoOrgScopedCreate {
	type Insert: HasSeaFields;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert;
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct SenderPresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub name: String,
	pub comments: Option<String>,
	pub deleted: bool,
	pub is_default: bool,
	pub sender_type: Option<String>,
	pub organization_name: Option<String>,
	pub organization_name_notation: Option<String>,
	pub person_given_name: Option<String>,
	pub department: Option<String>,
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
	pub name: String,
	pub comments: Option<String>,
	pub is_default: Option<bool>,
	pub sender_type: Option<String>,
	pub organization_name: Option<String>,
	pub organization_name_notation: Option<String>,
	pub person_given_name: Option<String>,
	pub department: Option<String>,
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
	name: String,
	comments: Option<String>,
	is_default: Option<bool>,
	sender_type: Option<String>,
	organization_name: Option<String>,
	organization_name_notation: Option<String>,
	person_given_name: Option<String>,
	department: Option<String>,
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
			name: self.name,
			comments: self.comments,
			is_default: self.is_default,
			sender_type: self.sender_type,
			organization_name: self.organization_name,
			organization_name_notation: self.organization_name_notation,
			person_given_name: self.person_given_name,
			department: self.department,
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
	pub name: Option<String>,
	pub comments: Option<String>,
	pub deleted: Option<bool>,
	pub is_default: Option<bool>,
	pub sender_type: Option<String>,
	pub organization_name: Option<String>,
	pub organization_name_notation: Option<String>,
	pub person_given_name: Option<String>,
	pub department: Option<String>,
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
		Self::validate_identity(
			data.sender_type.as_deref(),
			data.organization_name.as_deref(),
			data.person_given_name.as_deref(),
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
			Self::ensure_not_referenced_by_products(ctx, mm, id).await?;
		} else {
			let current = Self::get(ctx, mm, id).await?;
			let sender_type = data
				.sender_type
				.as_deref()
				.or(current.sender_type.as_deref());
			let organization_name = data
				.organization_name
				.as_deref()
				.or(current.organization_name.as_deref());
			let person_given_name = data
				.person_given_name
				.as_deref()
				.or(current.person_given_name.as_deref());
			Self::validate_identity(
				sender_type,
				organization_name,
				person_given_name,
			)?;
			Self::ensure_unique_identity(
				ctx,
				mm,
				Some(id),
				sender_type,
				organization_name,
			)
			.await?;
		}
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		Self::ensure_not_referenced_by_products(ctx, mm, id).await?;
		base_uuid::delete::<Self>(ctx, mm, id).await
	}

	fn validate_identity(
		sender_type: Option<&str>,
		organization_name: Option<&str>,
		person_given_name: Option<&str>,
	) -> Result<()> {
		require_identity(
			normalized_text(sender_type).is_some()
				&& normalized_text(organization_name).is_some()
				&& normalized_text(person_given_name).is_some(),
			"sender presave requires sender_type, organization_name, and person_given_name",
		)
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

	async fn ensure_not_referenced_by_products(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<()> {
		let referenced = ProductPresaveBmc::list(ctx, mm, None)
			.await?
			.into_iter()
			.any(|row| !row.deleted && row.sender_presave_id == Some(id));
		if referenced {
			Err(relationship_conflict(
				"sender presave is used by product presaves",
			))
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
	pub name: String,
	pub comments: Option<String>,
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
	pub name: String,
	pub comments: Option<String>,
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
	name: String,
	comments: Option<String>,
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
			name: self.name,
			comments: self.comments,
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
	pub name: Option<String>,
	pub comments: Option<String>,
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
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ReceiverPresaveForCreate,
	) -> Result<Uuid> {
		Self::validate_identity(
			data.receiver_type.as_deref(),
			data.organization_name.as_deref(),
		)?;
		Self::ensure_unique_identity(
			ctx,
			mm,
			None,
			data.receiver_type.as_deref(),
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
		if data.deleted != Some(true) {
			let current = Self::get(ctx, mm, id).await?;
			let receiver_type = data
				.receiver_type
				.as_deref()
				.or(current.receiver_type.as_deref());
			let organization_name = data
				.organization_name
				.as_deref()
				.or(current.organization_name.as_deref());
			Self::validate_identity(receiver_type, organization_name)?;
			Self::ensure_unique_identity(
				ctx,
				mm,
				Some(id),
				receiver_type,
				organization_name,
			)
			.await?;
		}
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}

	fn validate_identity(
		receiver_type: Option<&str>,
		organization_name: Option<&str>,
	) -> Result<()> {
		require_identity(
			normalized_text(receiver_type).is_some()
				&& normalized_text(organization_name).is_some(),
			"receiver presave requires receiver_type and organization_name",
		)
	}

	async fn ensure_unique_identity(
		ctx: &Ctx,
		mm: &ModelManager,
		excluding_id: Option<Uuid>,
		receiver_type: Option<&str>,
		organization_name: Option<&str>,
	) -> Result<()> {
		let receiver_type = normalized_text(receiver_type);
		let organization_name = normalized_text(organization_name);
		let duplicate = Self::list(ctx, mm, None).await?.into_iter().any(|row| {
			!row.deleted
				&& Some(row.id) != excluding_id
				&& normalized_text(row.receiver_type.as_deref()) == receiver_type
				&& normalized_text(row.organization_name.as_deref())
					== organization_name
		});
		if duplicate {
			Err(duplicate_identity("receiver presave duplicate identity"))
		} else {
			Ok(())
		}
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
pub struct ProductPresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub name: String,
	pub comments: Option<String>,
	pub deleted: bool,
	pub sender_presave_id: Option<Uuid>,
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
pub struct ProductPresaveForCreate {
	pub name: String,
	pub comments: Option<String>,
	pub sender_presave_id: Option<Uuid>,
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
	name: String,
	comments: Option<String>,
	sender_presave_id: Option<Uuid>,
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
			name: self.name,
			comments: self.comments,
			sender_presave_id: self.sender_presave_id,
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
pub struct ProductPresaveForUpdate {
	pub name: Option<String>,
	pub comments: Option<String>,
	pub deleted: Option<bool>,
	pub sender_presave_id: Option<Uuid>,
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
			Self::ensure_not_referenced_by_studies(ctx, mm, id).await?;
		} else {
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
		Self::ensure_not_referenced_by_studies(ctx, mm, id).await?;
		base_uuid::delete::<Self>(ctx, mm, id).await
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

	async fn ensure_not_referenced_by_studies(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<()> {
		let referenced = StudyPresaveBmc::list(ctx, mm, None)
			.await?
			.into_iter()
			.any(|row| !row.deleted && row.product_presave_id == Some(id));
		if referenced {
			Err(relationship_conflict(
				"product presave is used by study presaves",
			))
		} else {
			Ok(())
		}
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ProductPresaveSubstance {
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
pub struct ProductPresaveSubstanceForCreate {
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
pub struct ProductPresaveSubstanceForUpdate {
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
	ProductPresaveSubstanceBmc,
	ProductPresaveSubstance,
	ProductPresaveSubstanceForCreate,
	ProductPresaveSubstanceForUpdate,
	"product_presave_substances",
	"product_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ProductPresaveMfdsDeviceItem {
	pub id: Uuid,
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub code: Option<String>,
	pub value_code: Option<String>,
	pub value_value: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ProductPresaveMfdsDeviceItemForCreate {
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub code: Option<String>,
	pub value_code: Option<String>,
	pub value_value: Option<String>,
}

#[derive(Default, Fields, Deserialize)]
pub struct ProductPresaveMfdsDeviceItemForUpdate {
	pub sequence_number: Option<i32>,
	pub code: Option<String>,
	pub value_code: Option<String>,
	pub value_value: Option<String>,
}

pub struct ProductPresaveMfdsDeviceItemBmc;

impl DbBmc for ProductPresaveMfdsDeviceItemBmc {
	const TABLE: &'static str = "product_presave_mfds_device_items";
}

impl ProductPresaveMfdsDeviceItemBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ProductPresaveMfdsDeviceItemForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ProductPresaveMfdsDeviceItem> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ProductPresaveMfdsDeviceItem>> {
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
		data: ProductPresaveMfdsDeviceItemForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}

	pub async fn list_by_parent(
		ctx: &Ctx,
		mm: &ModelManager,
		parent_id: Uuid,
	) -> Result<Vec<ProductPresaveMfdsDeviceItem>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) =
			crate::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
		{
			dbx.rollback_txn().await?;
			return Err(err);
		}

		let sql = format!(
			"SELECT * FROM {} WHERE product_presave_id = $1 ORDER BY sequence_number ASC, id ASC",
			Self::TABLE
		);
		let rows = match dbx
			.fetch_all(
				sqlx::query_as::<_, ProductPresaveMfdsDeviceItem>(&sql)
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
pub struct ReporterPresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub name: String,
	pub comments: Option<String>,
	pub deleted: bool,
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
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ReporterPresaveForCreate {
	pub name: String,
	pub comments: Option<String>,
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

#[derive(Fields)]
struct ReporterPresaveForInsert {
	organization_id: Uuid,
	name: String,
	comments: Option<String>,
	reporter_title: Option<String>,
	reporter_given_name: Option<String>,
	reporter_middle_name: Option<String>,
	reporter_family_name: Option<String>,
	organization: Option<String>,
	department: Option<String>,
	street: Option<String>,
	city: Option<String>,
	state: Option<String>,
	postcode: Option<String>,
	telephone: Option<String>,
	country_code: Option<String>,
	email: Option<String>,
	qualification: Option<String>,
	qualification_kr1: Option<String>,
	primary_source_regulatory: Option<String>,
}

impl IntoOrgScopedCreate for ReporterPresaveForCreate {
	type Insert = ReporterPresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		ReporterPresaveForInsert {
			organization_id,
			name: self.name,
			comments: self.comments,
			reporter_title: self.reporter_title,
			reporter_given_name: self.reporter_given_name,
			reporter_middle_name: self.reporter_middle_name,
			reporter_family_name: self.reporter_family_name,
			organization: self.organization,
			department: self.department,
			street: self.street,
			city: self.city,
			state: self.state,
			postcode: self.postcode,
			telephone: self.telephone,
			country_code: self.country_code,
			email: self.email,
			qualification: self.qualification,
			qualification_kr1: self.qualification_kr1,
			primary_source_regulatory: self.primary_source_regulatory,
		}
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct ReporterPresaveForUpdate {
	pub name: Option<String>,
	pub comments: Option<String>,
	pub deleted: Option<bool>,
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
		Self::validate_identity(
			data.reporter_given_name.as_deref(),
			data.organization.as_deref(),
			data.qualification.as_deref(),
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
				organization,
				qualification,
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
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}

	fn validate_identity(
		reporter_given_name: Option<&str>,
		organization: Option<&str>,
		qualification: Option<&str>,
	) -> Result<()> {
		require_identity(
			normalized_text(reporter_given_name).is_some()
				&& normalized_text(organization).is_some()
				&& normalized_text(qualification).is_some(),
			"reporter presave requires reporter_given_name, organization, and qualification",
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
	pub name: String,
	pub comments: Option<String>,
	pub deleted: bool,
	pub product_presave_id: Option<Uuid>,
	pub study_name: Option<String>,
	pub study_name_notation: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_kind: Option<String>,
	pub study_type_reaction: Option<String>,
	pub study_type_reaction_kr1: Option<String>,
	pub mfds_study_number: Option<String>,
	pub mfds_protocol_number: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub edc_sync: Option<bool>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct StudyPresaveForCreate {
	pub name: String,
	pub comments: Option<String>,
	pub product_presave_id: Option<Uuid>,
	pub study_name: Option<String>,
	pub study_name_notation: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_kind: Option<String>,
	pub study_type_reaction: Option<String>,
	pub study_type_reaction_kr1: Option<String>,
	pub mfds_study_number: Option<String>,
	pub mfds_protocol_number: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub edc_sync: Option<bool>,
}

#[derive(Fields)]
struct StudyPresaveForInsert {
	organization_id: Uuid,
	name: String,
	comments: Option<String>,
	product_presave_id: Option<Uuid>,
	study_name: Option<String>,
	study_name_notation: Option<String>,
	sponsor_study_number: Option<String>,
	sponsor_study_number_kind: Option<String>,
	study_type_reaction: Option<String>,
	study_type_reaction_kr1: Option<String>,
	mfds_study_number: Option<String>,
	mfds_protocol_number: Option<String>,
	fda_ind_number_occurred: Option<String>,
	fda_pre_anda_number_occurred: Option<String>,
	edc_sync: Option<bool>,
}

impl IntoOrgScopedCreate for StudyPresaveForCreate {
	type Insert = StudyPresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		StudyPresaveForInsert {
			organization_id,
			name: self.name,
			comments: self.comments,
			product_presave_id: self.product_presave_id,
			study_name: self.study_name,
			study_name_notation: self.study_name_notation,
			sponsor_study_number: self.sponsor_study_number,
			sponsor_study_number_kind: self.sponsor_study_number_kind,
			study_type_reaction: self.study_type_reaction,
			study_type_reaction_kr1: self.study_type_reaction_kr1,
			mfds_study_number: self.mfds_study_number,
			mfds_protocol_number: self.mfds_protocol_number,
			fda_ind_number_occurred: self.fda_ind_number_occurred,
			fda_pre_anda_number_occurred: self.fda_pre_anda_number_occurred,
			edc_sync: self.edc_sync,
		}
	}
}

impl StudyPresaveForCreate {
	fn validate_fields(&self) -> Result<()> {
		validate_sponsor_study_number_kind(self.sponsor_study_number_kind.as_deref())
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct StudyPresaveForUpdate {
	pub name: Option<String>,
	pub comments: Option<String>,
	pub deleted: Option<bool>,
	pub product_presave_id: Option<Uuid>,
	pub study_name: Option<String>,
	pub study_name_notation: Option<String>,
	pub sponsor_study_number: Option<String>,
	pub sponsor_study_number_kind: Option<String>,
	pub study_type_reaction: Option<String>,
	pub study_type_reaction_kr1: Option<String>,
	pub mfds_study_number: Option<String>,
	pub mfds_protocol_number: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub edc_sync: Option<bool>,
}

impl StudyPresaveForUpdate {
	fn validate_fields(&self) -> Result<()> {
		validate_sponsor_study_number_kind(self.sponsor_study_number_kind.as_deref())
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
		base_uuid::delete::<Self>(ctx, mm, id).await
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

impl_child_bmc!(
	StudyPresaveRegistrationNumberBmc,
	StudyPresaveRegistrationNumber,
	StudyPresaveRegistrationNumberForCreate,
	StudyPresaveRegistrationNumberForUpdate,
	"study_presave_registration_numbers",
	"study_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct StudyPresaveFdaCrossReportedInd {
	pub id: Uuid,
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub ind_number: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct StudyPresaveFdaCrossReportedIndForCreate {
	pub study_presave_id: Uuid,
	pub sequence_number: i32,
	pub ind_number: Option<String>,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct StudyPresaveFdaCrossReportedIndForUpdate {
	pub sequence_number: Option<i32>,
	pub ind_number: Option<String>,
	pub deleted: Option<bool>,
}

pub struct StudyPresaveFdaCrossReportedIndBmc;

impl DbBmc for StudyPresaveFdaCrossReportedIndBmc {
	const TABLE: &'static str = "study_presave_fda_cross_reported_inds";
}

impl StudyPresaveFdaCrossReportedIndBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: StudyPresaveFdaCrossReportedIndForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<StudyPresaveFdaCrossReportedInd> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<StudyPresaveFdaCrossReportedInd>> {
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
		data: StudyPresaveFdaCrossReportedIndForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}

	pub async fn list_by_parent(
		ctx: &Ctx,
		mm: &ModelManager,
		parent_id: Uuid,
	) -> Result<Vec<StudyPresaveFdaCrossReportedInd>> {
		let dbx = mm.dbx();
		dbx.begin_txn().await?;
		if let Err(err) =
			crate::model::store::set_full_context_from_ctx_dbx(dbx, ctx).await
		{
			dbx.rollback_txn().await?;
			return Err(err);
		}

		let sql = format!(
			"SELECT * FROM {} WHERE study_presave_id = $1 ORDER BY sequence_number ASC, id ASC",
			Self::TABLE
		);
		let rows = match dbx
			.fetch_all(
				sqlx::query_as::<_, StudyPresaveFdaCrossReportedInd>(&sql)
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
pub struct NarrativePresave {
	pub id: Uuid,
	pub organization_id: Uuid,
	pub name: String,
	pub comments: Option<String>,
	pub deleted: bool,
	pub case_narrative: Option<String>,
	pub case_narrative_notation: Option<String>,
	pub reporter_comments: Option<String>,
	pub sender_comments: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct NarrativePresaveForCreate {
	pub name: String,
	pub comments: Option<String>,
	pub case_narrative: Option<String>,
	pub case_narrative_notation: Option<String>,
	pub reporter_comments: Option<String>,
	pub sender_comments: Option<String>,
}

#[derive(Fields)]
struct NarrativePresaveForInsert {
	organization_id: Uuid,
	name: String,
	comments: Option<String>,
	case_narrative: Option<String>,
	case_narrative_notation: Option<String>,
	reporter_comments: Option<String>,
	sender_comments: Option<String>,
}

impl IntoOrgScopedCreate for NarrativePresaveForCreate {
	type Insert = NarrativePresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		NarrativePresaveForInsert {
			organization_id,
			name: self.name,
			comments: self.comments,
			case_narrative: self.case_narrative,
			case_narrative_notation: self.case_narrative_notation,
			reporter_comments: self.reporter_comments,
			sender_comments: self.sender_comments,
		}
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct NarrativePresaveForUpdate {
	pub name: Option<String>,
	pub comments: Option<String>,
	pub deleted: Option<bool>,
	pub case_narrative: Option<String>,
	pub case_narrative_notation: Option<String>,
	pub reporter_comments: Option<String>,
	pub sender_comments: Option<String>,
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
		Self::validate_identity(Some(data.name.as_str()))?;
		Self::ensure_unique_identity(ctx, mm, None, Some(data.name.as_str()))
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
		if data.deleted != Some(true) {
			let current = Self::get(ctx, mm, id).await?;
			let name = data.name.as_deref().or(Some(current.name.as_str()));
			Self::validate_identity(name)?;
			Self::ensure_unique_identity(ctx, mm, Some(id), name).await?;
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
			!row.deleted
				&& Some(row.id) != excluding_id
				&& normalized_text(Some(row.name.as_str())) == name
		});
		if duplicate {
			Err(duplicate_identity("narrative presave duplicate identity"))
		} else {
			Ok(())
		}
	}
}

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct NarrativePresaveSenderDiagnosis {
	pub id: Uuid,
	pub narrative_presave_id: Uuid,
	pub sequence_number: i32,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct NarrativePresaveSenderDiagnosisForCreate {
	pub narrative_presave_id: Uuid,
	pub sequence_number: i32,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct NarrativePresaveSenderDiagnosisForUpdate {
	pub sequence_number: Option<i32>,
	pub diagnosis_meddra_version: Option<String>,
	pub diagnosis_meddra_code: Option<String>,
	pub deleted: Option<bool>,
}

impl_child_bmc!(
	NarrativePresaveSenderDiagnosisBmc,
	NarrativePresaveSenderDiagnosis,
	NarrativePresaveSenderDiagnosisForCreate,
	NarrativePresaveSenderDiagnosisForUpdate,
	"narrative_presave_sender_diagnoses",
	"narrative_presave_id"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct NarrativePresaveCaseSummary {
	pub id: Uuid,
	pub narrative_presave_id: Uuid,
	pub sequence_number: i32,
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
	pub deleted: bool,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct NarrativePresaveCaseSummaryForCreate {
	pub narrative_presave_id: Uuid,
	pub sequence_number: i32,
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
	pub deleted: Option<bool>,
}

#[derive(Default, Fields, Deserialize)]
pub struct NarrativePresaveCaseSummaryForUpdate {
	pub sequence_number: Option<i32>,
	pub summary_type: Option<String>,
	pub language_code: Option<String>,
	pub summary_text: Option<String>,
	pub deleted: Option<bool>,
}

impl_child_bmc!(
	NarrativePresaveCaseSummaryBmc,
	NarrativePresaveCaseSummary,
	NarrativePresaveCaseSummaryForCreate,
	NarrativePresaveCaseSummaryForUpdate,
	"narrative_presave_case_summaries",
	"narrative_presave_id"
);
