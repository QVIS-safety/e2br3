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

macro_rules! impl_parent_bmc {
	(
		$bmc:ident,
		$model:ty,
		$create:ty,
		$update:ty,
		$table:literal
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
		}
	};
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

impl_parent_bmc!(
	SenderPresaveBmc,
	SenderPresave,
	SenderPresaveForCreate,
	SenderPresaveForUpdate,
	"sender_presaves"
);

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

impl_parent_bmc!(
	ReceiverPresaveBmc,
	ReceiverPresave,
	ReceiverPresaveForCreate,
	ReceiverPresaveForUpdate,
	"receiver_presaves"
);

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
	pub drug_characterization: Option<String>,
	pub medicinal_product: Option<String>,
	pub medicinal_product_notation: Option<String>,
	pub preapproval_ip_name: Option<String>,
	pub brand_name: Option<String>,
	pub drug_generic_name: Option<String>,
	pub manufacturer_name: Option<String>,
	pub product_description: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub obtain_drug_country: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub drug_authorization_country: Option<String>,
	pub drug_authorization_holder: Option<String>,
	pub holder_applicant_name_notation: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub mfds_domestic_product_code: Option<String>,
	pub mfds_domestic_ingredient_code: Option<String>,
	pub mfds_udl_product_code: Option<String>,
	pub mfds_udl_ingredient_code: Option<String>,
	pub mfds_udl_manufacturer_code: Option<String>,
	pub mfds_udl_manufacturer_name: Option<String>,
	pub mfds_foreign_ich_product_code: Option<String>,
	pub mfds_foreign_ich_ingredient_code: Option<String>,
	pub mfds_foreign_ich_holder_code: Option<String>,
	pub mfds_foreign_ich_holder_name: Option<String>,
	pub mfds_foreign_e2b_product_code: Option<String>,
	pub mfds_foreign_e2b_ingredient_code: Option<String>,
	pub mfds_foreign_e2b_holder_code: Option<String>,
	pub mfds_foreign_e2b_holder_name: Option<String>,
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
	pub drug_characterization: Option<String>,
	pub medicinal_product: Option<String>,
	pub medicinal_product_notation: Option<String>,
	pub preapproval_ip_name: Option<String>,
	pub brand_name: Option<String>,
	pub drug_generic_name: Option<String>,
	pub manufacturer_name: Option<String>,
	pub product_description: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub obtain_drug_country: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub drug_authorization_country: Option<String>,
	pub drug_authorization_holder: Option<String>,
	pub holder_applicant_name_notation: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub mfds_domestic_product_code: Option<String>,
	pub mfds_domestic_ingredient_code: Option<String>,
	pub mfds_udl_product_code: Option<String>,
	pub mfds_udl_ingredient_code: Option<String>,
	pub mfds_udl_manufacturer_code: Option<String>,
	pub mfds_udl_manufacturer_name: Option<String>,
	pub mfds_foreign_ich_product_code: Option<String>,
	pub mfds_foreign_ich_ingredient_code: Option<String>,
	pub mfds_foreign_ich_holder_code: Option<String>,
	pub mfds_foreign_ich_holder_name: Option<String>,
	pub mfds_foreign_e2b_product_code: Option<String>,
	pub mfds_foreign_e2b_ingredient_code: Option<String>,
	pub mfds_foreign_e2b_holder_code: Option<String>,
	pub mfds_foreign_e2b_holder_name: Option<String>,
}

#[derive(Fields)]
struct ProductPresaveForInsert {
	organization_id: Uuid,
	name: String,
	comments: Option<String>,
	sender_presave_id: Option<Uuid>,
	drug_characterization: Option<String>,
	medicinal_product: Option<String>,
	medicinal_product_notation: Option<String>,
	preapproval_ip_name: Option<String>,
	brand_name: Option<String>,
	drug_generic_name: Option<String>,
	manufacturer_name: Option<String>,
	product_description: Option<String>,
	mpid: Option<String>,
	mpid_version: Option<String>,
	phpid: Option<String>,
	phpid_version: Option<String>,
	investigational_product_blinded: Option<bool>,
	obtain_drug_country: Option<String>,
	drug_authorization_number: Option<String>,
	drug_authorization_country: Option<String>,
	drug_authorization_holder: Option<String>,
	holder_applicant_name_notation: Option<String>,
	fda_ind_number_occurred: Option<String>,
	fda_pre_anda_number_occurred: Option<String>,
	mfds_domestic_product_code: Option<String>,
	mfds_domestic_ingredient_code: Option<String>,
	mfds_udl_product_code: Option<String>,
	mfds_udl_ingredient_code: Option<String>,
	mfds_udl_manufacturer_code: Option<String>,
	mfds_udl_manufacturer_name: Option<String>,
	mfds_foreign_ich_product_code: Option<String>,
	mfds_foreign_ich_ingredient_code: Option<String>,
	mfds_foreign_ich_holder_code: Option<String>,
	mfds_foreign_ich_holder_name: Option<String>,
	mfds_foreign_e2b_product_code: Option<String>,
	mfds_foreign_e2b_ingredient_code: Option<String>,
	mfds_foreign_e2b_holder_code: Option<String>,
	mfds_foreign_e2b_holder_name: Option<String>,
}

impl IntoOrgScopedCreate for ProductPresaveForCreate {
	type Insert = ProductPresaveForInsert;

	fn into_insert(self, organization_id: Uuid) -> Self::Insert {
		ProductPresaveForInsert {
			organization_id,
			name: self.name,
			comments: self.comments,
			sender_presave_id: self.sender_presave_id,
			drug_characterization: self.drug_characterization,
			medicinal_product: self.medicinal_product,
			medicinal_product_notation: self.medicinal_product_notation,
			preapproval_ip_name: self.preapproval_ip_name,
			brand_name: self.brand_name,
			drug_generic_name: self.drug_generic_name,
			manufacturer_name: self.manufacturer_name,
			product_description: self.product_description,
			mpid: self.mpid,
			mpid_version: self.mpid_version,
			phpid: self.phpid,
			phpid_version: self.phpid_version,
			investigational_product_blinded: self.investigational_product_blinded,
			obtain_drug_country: self.obtain_drug_country,
			drug_authorization_number: self.drug_authorization_number,
			drug_authorization_country: self.drug_authorization_country,
			drug_authorization_holder: self.drug_authorization_holder,
			holder_applicant_name_notation: self.holder_applicant_name_notation,
			fda_ind_number_occurred: self.fda_ind_number_occurred,
			fda_pre_anda_number_occurred: self.fda_pre_anda_number_occurred,
			mfds_domestic_product_code: self.mfds_domestic_product_code,
			mfds_domestic_ingredient_code: self.mfds_domestic_ingredient_code,
			mfds_udl_product_code: self.mfds_udl_product_code,
			mfds_udl_ingredient_code: self.mfds_udl_ingredient_code,
			mfds_udl_manufacturer_code: self.mfds_udl_manufacturer_code,
			mfds_udl_manufacturer_name: self.mfds_udl_manufacturer_name,
			mfds_foreign_ich_product_code: self.mfds_foreign_ich_product_code,
			mfds_foreign_ich_ingredient_code: self.mfds_foreign_ich_ingredient_code,
			mfds_foreign_ich_holder_code: self.mfds_foreign_ich_holder_code,
			mfds_foreign_ich_holder_name: self.mfds_foreign_ich_holder_name,
			mfds_foreign_e2b_product_code: self.mfds_foreign_e2b_product_code,
			mfds_foreign_e2b_ingredient_code: self.mfds_foreign_e2b_ingredient_code,
			mfds_foreign_e2b_holder_code: self.mfds_foreign_e2b_holder_code,
			mfds_foreign_e2b_holder_name: self.mfds_foreign_e2b_holder_name,
		}
	}
}

#[derive(Default, Fields, Deserialize)]
pub struct ProductPresaveForUpdate {
	pub name: Option<String>,
	pub comments: Option<String>,
	pub deleted: Option<bool>,
	pub sender_presave_id: Option<Uuid>,
	pub drug_characterization: Option<String>,
	pub medicinal_product: Option<String>,
	pub medicinal_product_notation: Option<String>,
	pub preapproval_ip_name: Option<String>,
	pub brand_name: Option<String>,
	pub drug_generic_name: Option<String>,
	pub manufacturer_name: Option<String>,
	pub product_description: Option<String>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub obtain_drug_country: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub drug_authorization_country: Option<String>,
	pub drug_authorization_holder: Option<String>,
	pub holder_applicant_name_notation: Option<String>,
	pub fda_ind_number_occurred: Option<String>,
	pub fda_pre_anda_number_occurred: Option<String>,
	pub mfds_domestic_product_code: Option<String>,
	pub mfds_domestic_ingredient_code: Option<String>,
	pub mfds_udl_product_code: Option<String>,
	pub mfds_udl_ingredient_code: Option<String>,
	pub mfds_udl_manufacturer_code: Option<String>,
	pub mfds_udl_manufacturer_name: Option<String>,
	pub mfds_foreign_ich_product_code: Option<String>,
	pub mfds_foreign_ich_ingredient_code: Option<String>,
	pub mfds_foreign_ich_holder_code: Option<String>,
	pub mfds_foreign_ich_holder_name: Option<String>,
	pub mfds_foreign_e2b_product_code: Option<String>,
	pub mfds_foreign_e2b_ingredient_code: Option<String>,
	pub mfds_foreign_e2b_holder_code: Option<String>,
	pub mfds_foreign_e2b_holder_name: Option<String>,
}

impl_parent_bmc!(
	ProductPresaveBmc,
	ProductPresave,
	ProductPresaveForCreate,
	ProductPresaveForUpdate,
	"product_presaves"
);

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct ProductPresaveSubstance {
	pub id: Uuid,
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub substance_name: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid: Option<String>,
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
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

#[derive(Default, Fields, Deserialize)]
pub struct ProductPresaveSubstanceForUpdate {
	pub sequence_number: Option<i32>,
	pub substance_name: Option<String>,
	pub substance_termid_version: Option<String>,
	pub substance_termid: Option<String>,
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
pub struct ProductPresaveFdaCrossReportedInd {
	pub id: Uuid,
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub ind_number: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ProductPresaveFdaCrossReportedIndForCreate {
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub ind_number: Option<String>,
}

#[derive(Default, Fields, Deserialize)]
pub struct ProductPresaveFdaCrossReportedIndForUpdate {
	pub sequence_number: Option<i32>,
	pub ind_number: Option<String>,
}

pub struct ProductPresaveFdaCrossReportedIndBmc;

impl DbBmc for ProductPresaveFdaCrossReportedIndBmc {
	const TABLE: &'static str = "product_presave_fda_cross_reported_inds";
}

impl ProductPresaveFdaCrossReportedIndBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ProductPresaveFdaCrossReportedIndForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ProductPresaveFdaCrossReportedInd> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ProductPresaveFdaCrossReportedInd>> {
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
		data: ProductPresaveFdaCrossReportedIndForUpdate,
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
	) -> Result<Vec<ProductPresaveFdaCrossReportedInd>> {
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
				sqlx::query_as::<_, ProductPresaveFdaCrossReportedInd>(&sql)
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
pub struct ProductPresaveMfdsRegionalItem {
	pub id: Uuid,
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub item_type: Option<String>,
	pub item_value: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct ProductPresaveMfdsRegionalItemForCreate {
	pub product_presave_id: Uuid,
	pub sequence_number: i32,
	pub item_type: Option<String>,
	pub item_value: Option<String>,
}

#[derive(Default, Fields, Deserialize)]
pub struct ProductPresaveMfdsRegionalItemForUpdate {
	pub sequence_number: Option<i32>,
	pub item_type: Option<String>,
	pub item_value: Option<String>,
}

pub struct ProductPresaveMfdsRegionalItemBmc;

impl DbBmc for ProductPresaveMfdsRegionalItemBmc {
	const TABLE: &'static str = "product_presave_mfds_regional_items";
}

impl ProductPresaveMfdsRegionalItemBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: ProductPresaveMfdsRegionalItemForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<ProductPresaveMfdsRegionalItem> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		list_options: Option<ListOptions>,
	) -> Result<Vec<ProductPresaveMfdsRegionalItem>> {
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
		data: ProductPresaveMfdsRegionalItemForUpdate,
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
	) -> Result<Vec<ProductPresaveMfdsRegionalItem>> {
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
				sqlx::query_as::<_, ProductPresaveMfdsRegionalItem>(&sql)
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

impl_parent_bmc!(
	ReporterPresaveBmc,
	ReporterPresave,
	ReporterPresaveForCreate,
	ReporterPresaveForUpdate,
	"reporter_presaves"
);

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
		&["study_no", "protocol_no"],
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
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
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

impl_parent_bmc!(
	NarrativePresaveBmc,
	NarrativePresave,
	NarrativePresaveForCreate,
	NarrativePresaveForUpdate,
	"narrative_presaves"
);

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
