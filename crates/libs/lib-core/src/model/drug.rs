// Section G - Drug/Biological Information

use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValsValue};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::time::{Date, OffsetDateTime, Time};
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- DrugInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugInformation {
	pub id: Uuid,
	pub case_id: Uuid,
	pub sequence_number: i32,

	// G.k.1 - Drug role (MANDATORY)
	pub drug_characterization: String,

	// G.k.2.2 - Product name
	pub medicinal_product: String,

	// G.k.2.4-5 - Product identifiers
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	// G.k.2.5 - Investigational Product Blinded
	pub investigational_product_blinded: Option<bool>,

	// G.k.3.1 - Obtain Drug Country
	pub obtain_drug_country: Option<String>,

	// G.k.3.2 - Brand Name
	pub brand_name: Option<String>,

	// G.k.3.3 - Manufacturer
	pub manufacturer_name: Option<String>,
	pub manufacturer_country: Option<String>,

	// G.k.3.4 - Batch/Lot Number
	pub batch_lot_number: Option<String>,

	// G.k.5 - Dosage Text
	pub dosage_text: Option<String>,

	// G.k.7 - Action taken
	pub action_taken: Option<String>,

	// G.k.8 - Rechallenge/Recurrence
	pub rechallenge: Option<String>,

	// G.k.10 - Parent Route
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,

	// G.k.11 - Parent Dosage
	pub parent_dosage_text: Option<String>,

	// FDA.G.k.10a - Additional Information on Drug (coded)
	pub fda_additional_info_coded: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DrugInformationForCreate {
	pub case_id: Uuid,
	pub sequence_number: i32,
	pub drug_characterization: String,
	pub medicinal_product: String,
}

#[derive(Fields, Deserialize)]
pub struct DrugInformationForUpdate {
	pub medicinal_product: Option<String>,
	pub drug_characterization: Option<String>,
	pub brand_name: Option<String>,
	pub manufacturer_name: Option<String>,
	pub manufacturer_country: Option<String>,
	pub batch_lot_number: Option<String>,
	pub dosage_text: Option<String>,
	pub action_taken: Option<String>,
	pub rechallenge: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub obtain_drug_country: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub parent_dosage_text: Option<String>,
	pub fda_additional_info_coded: Option<String>,
}

// -- DrugActiveSubstance

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugActiveSubstance {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,

	// G.k.2.3.r.1 - Substance Name
	pub substance_name: Option<String>,

	// G.k.2.3.r.2 - Substance TermID
	pub substance_termid: Option<String>,
	pub substance_termid_version: Option<String>,

	// G.k.2.3.r.3 - Strength
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DrugActiveSubstanceForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub substance_name: Option<String>,
	pub substance_termid: Option<String>,
	pub substance_termid_version: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DrugActiveSubstanceForUpdate {
	pub substance_name: Option<String>,
	pub substance_termid: Option<String>,
	pub substance_termid_version: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DrugActiveSubstanceFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- DosageInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DosageInformation {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,

	// G.k.4.r.1 - Dose
	pub dose_value: Option<Decimal>,
	pub dose_unit: Option<String>,

	// G.k.4.r.2 - Number of Separate Dosages
	pub number_of_units: Option<i32>,

	// G.k.4.r.3 - Dose Frequency
	pub frequency_value: Option<Decimal>,
	pub frequency_unit: Option<String>,

	// G.k.4.r.4 - Date/Time of First Administration
	pub first_administration_date: Option<Date>,
	pub first_administration_time: Option<Time>,

	// G.k.4.r.5 - Date/Time of Last Administration
	pub last_administration_date: Option<Date>,
	pub last_administration_time: Option<Time>,

	// G.k.4.r.6 - Duration
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,

	// G.k.4.r.7 - Batch/Lot Number
	pub batch_lot_number: Option<String>,

	// G.k.4.r.8 - Dosage Text
	pub dosage_text: Option<String>,

	// G.k.4.r.9.1 - Pharmaceutical Dose Form
	pub dose_form: Option<String>,
	pub dose_form_termid: Option<String>,
	pub dose_form_termid_version: Option<String>,

	// G.k.4.r.10 - Route of Administration
	pub route_of_administration: Option<String>,

	// G.k.4.r.11 - Parent Route
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DosageInformationForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub dose_value: Option<Decimal>,
	pub dose_unit: Option<String>,
	pub number_of_units: Option<i32>,
	pub frequency_value: Option<Decimal>,
	pub frequency_unit: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub first_administration_date: Option<Date>,
	pub first_administration_time: Option<Time>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub last_administration_date: Option<Date>,
	pub last_administration_time: Option<Time>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub batch_lot_number: Option<String>,
	pub dosage_text: Option<String>,
	pub dose_form: Option<String>,
	pub dose_form_termid: Option<String>,
	pub dose_form_termid_version: Option<String>,
	pub route_of_administration: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub first_administration_date_null_flavor: Option<String>,
	pub last_administration_date_null_flavor: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DosageInformationForUpdate {
	pub dose_value: Option<Decimal>,
	pub dose_unit: Option<String>,
	pub number_of_units: Option<i32>,
	pub frequency_value: Option<Decimal>,
	pub frequency_unit: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub first_administration_date: Option<Date>,
	pub first_administration_time: Option<Time>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub last_administration_date: Option<Date>,
	pub last_administration_time: Option<Time>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub batch_lot_number: Option<String>,
	pub dosage_text: Option<String>,
	pub dose_form: Option<String>,
	pub dose_form_termid: Option<String>,
	pub dose_form_termid_version: Option<String>,
	pub route_of_administration: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub first_administration_date_null_flavor: Option<String>,
	pub last_administration_date_null_flavor: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DosageInformationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- DrugIndication

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugIndication {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,

	// G.k.6.r.1 - Indication (free text)
	pub indication_text: Option<String>,

	// G.k.6.r.2 - Indication (MedDRA coded)
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DrugIndicationForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub indication_text: Option<String>,
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DrugIndicationForUpdate {
	pub indication_text: Option<String>,
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DrugIndicationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

#[derive(Fields, Deserialize)]
pub struct DrugDeviceCharacteristicForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub code: Option<String>,
	pub code_system: Option<String>,
	pub code_display_name: Option<String>,
	pub value_type: Option<String>,
	pub value_value: Option<String>,
	pub value_code: Option<String>,
	pub value_code_system: Option<String>,
	pub value_display_name: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DrugDeviceCharacteristicForUpdate {
	pub code: Option<String>,
	pub code_system: Option<String>,
	pub code_display_name: Option<String>,
	pub value_type: Option<String>,
	pub value_value: Option<String>,
	pub value_code: Option<String>,
	pub value_code_system: Option<String>,
	pub value_display_name: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DrugDeviceCharacteristicFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- DrugDeviceCharacteristic (FDA Scenario 7)

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugDeviceCharacteristic {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub code: Option<String>,
	pub code_system: Option<String>,
	pub code_display_name: Option<String>,
	pub value_type: Option<String>,
	pub value_value: Option<String>,
	pub value_code: Option<String>,
	pub value_code_system: Option<String>,
	pub value_display_name: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

// -- BMCs

pub struct DrugInformationBmc;
impl DbBmc for DrugInformationBmc {
	const TABLE: &'static str = "drug_information";
}

impl DrugInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		drug_c: DrugInformationForCreate,
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
			"INSERT INTO {} (case_id, sequence_number, drug_characterization, medicinal_product, created_at, updated_at, created_by)
			 VALUES ($1, $2, $3, $4, now(), now(), $5)
			 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(drug_c.case_id)
					.bind(drug_c.sequence_number)
					.bind(drug_c.drug_characterization)
					.bind(drug_c.medicinal_product)
					.bind(ctx.user_id()),
			)
			.await?;

		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get(
		_ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugInformation> {
		let sql = format!("SELECT * FROM {} WHERE id = $1", Self::TABLE);
		let drug = mm
			.dbx()
			.fetch_optional(sqlx::query_as::<_, DrugInformation>(&sql).bind(id))
			.await?
			.ok_or(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			})?;
		Ok(drug)
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		drug_u: DrugInformationForUpdate,
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
			 SET medicinal_product = COALESCE($2, medicinal_product),
			     drug_characterization = COALESCE($3, drug_characterization),
			     brand_name = COALESCE($4, brand_name),
			     manufacturer_name = COALESCE($5, manufacturer_name),
			     manufacturer_country = COALESCE($6, manufacturer_country),
			     batch_lot_number = COALESCE($7, batch_lot_number),
			     dosage_text = COALESCE($8, dosage_text),
			     action_taken = COALESCE($9, action_taken),
			     rechallenge = COALESCE($10, rechallenge),
			     investigational_product_blinded = COALESCE($11, investigational_product_blinded),
			     mpid = COALESCE($12, mpid),
			     mpid_version = COALESCE($13, mpid_version),
			     phpid = COALESCE($14, phpid),
			     phpid_version = COALESCE($15, phpid_version),
			     obtain_drug_country = COALESCE($16, obtain_drug_country),
			     parent_route = COALESCE($17, parent_route),
			     parent_route_termid = COALESCE($18, parent_route_termid),
			     parent_route_termid_version = COALESCE($19, parent_route_termid_version),
			     parent_dosage_text = COALESCE($20, parent_dosage_text),
			     fda_additional_info_coded = COALESCE($21, fda_additional_info_coded),
			     updated_at = now(),
			     updated_by = $22
			 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(drug_u.medicinal_product)
					.bind(drug_u.drug_characterization)
					.bind(drug_u.brand_name)
					.bind(drug_u.manufacturer_name)
					.bind(drug_u.manufacturer_country)
					.bind(drug_u.batch_lot_number)
					.bind(drug_u.dosage_text)
					.bind(drug_u.action_taken)
					.bind(drug_u.rechallenge)
					.bind(drug_u.investigational_product_blinded)
					.bind(drug_u.mpid)
					.bind(drug_u.mpid_version)
					.bind(drug_u.phpid)
					.bind(drug_u.phpid_version)
					.bind(drug_u.obtain_drug_country)
					.bind(drug_u.parent_route)
					.bind(drug_u.parent_route_termid)
					.bind(drug_u.parent_route_termid_version)
					.bind(drug_u.parent_dosage_text)
					.bind(drug_u.fda_additional_info_coded)
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
	) -> Result<Vec<DrugInformation>> {
		let sql = format!(
			"SELECT * FROM {} WHERE case_id = $1 ORDER BY sequence_number",
			Self::TABLE
		);
		let drugs = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, DrugInformation>(&sql).bind(case_id))
			.await?;
		Ok(drugs)
	}

	pub async fn get_in_case(
		_ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<DrugInformation> {
		let sql = format!(
			"SELECT * FROM {} WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let drug = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, DrugInformation>(&sql)
					.bind(id)
					.bind(case_id),
			)
			.await?
			.ok_or(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			})?;
		Ok(drug)
	}

	pub async fn update_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		drug_u: DrugInformationForUpdate,
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
			 SET medicinal_product = COALESCE($3, medicinal_product),
			     drug_characterization = COALESCE($4, drug_characterization),
			     brand_name = COALESCE($5, brand_name),
			     manufacturer_name = COALESCE($6, manufacturer_name),
			     manufacturer_country = COALESCE($7, manufacturer_country),
			     batch_lot_number = COALESCE($8, batch_lot_number),
			     dosage_text = COALESCE($9, dosage_text),
			     action_taken = COALESCE($10, action_taken),
			     rechallenge = COALESCE($11, rechallenge),
			     investigational_product_blinded = COALESCE($12, investigational_product_blinded),
			     mpid = COALESCE($13, mpid),
			     mpid_version = COALESCE($14, mpid_version),
			     phpid = COALESCE($15, phpid),
			     phpid_version = COALESCE($16, phpid_version),
			     obtain_drug_country = COALESCE($17, obtain_drug_country),
			     parent_route = COALESCE($18, parent_route),
			     parent_route_termid = COALESCE($19, parent_route_termid),
			     parent_route_termid_version = COALESCE($20, parent_route_termid_version),
			     parent_dosage_text = COALESCE($21, parent_dosage_text),
			     fda_additional_info_coded = COALESCE($22, fda_additional_info_coded),
			     updated_at = now(),
			     updated_by = $23
			 WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(case_id)
					.bind(drug_u.medicinal_product)
					.bind(drug_u.drug_characterization)
					.bind(drug_u.brand_name)
					.bind(drug_u.manufacturer_name)
					.bind(drug_u.manufacturer_country)
					.bind(drug_u.batch_lot_number)
					.bind(drug_u.dosage_text)
					.bind(drug_u.action_taken)
					.bind(drug_u.rechallenge)
					.bind(drug_u.investigational_product_blinded)
					.bind(drug_u.mpid)
					.bind(drug_u.mpid_version)
					.bind(drug_u.phpid)
					.bind(drug_u.phpid_version)
					.bind(drug_u.obtain_drug_country)
					.bind(drug_u.parent_route)
					.bind(drug_u.parent_route_termid)
					.bind(drug_u.parent_route_termid_version)
					.bind(drug_u.parent_dosage_text)
					.bind(drug_u.fda_additional_info_coded)
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

pub struct DrugActiveSubstanceBmc;
impl DbBmc for DrugActiveSubstanceBmc {
	const TABLE: &'static str = "drug_active_substances";
}

impl DrugActiveSubstanceBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DrugActiveSubstanceForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugActiveSubstance> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DrugActiveSubstanceFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DrugActiveSubstance>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DrugActiveSubstanceForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

pub struct DosageInformationBmc;
impl DbBmc for DosageInformationBmc {
	const TABLE: &'static str = "dosage_information";
}

impl DosageInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DosageInformationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DosageInformation> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DosageInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DosageInformation>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DosageInformationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

pub struct DrugIndicationBmc;
impl DbBmc for DrugIndicationBmc {
	const TABLE: &'static str = "drug_indications";
}

impl DrugIndicationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DrugIndicationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugIndication> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DrugIndicationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DrugIndication>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DrugIndicationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

// -- DrugDeviceCharacteristic BMC

pub struct DrugDeviceCharacteristicBmc;
impl DbBmc for DrugDeviceCharacteristicBmc {
	const TABLE: &'static str = "drug_device_characteristics";
}

impl DrugDeviceCharacteristicBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DrugDeviceCharacteristicForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugDeviceCharacteristic> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DrugDeviceCharacteristicFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DrugDeviceCharacteristic>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DrugDeviceCharacteristicForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}
