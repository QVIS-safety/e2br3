// Drug sub-resources REST endpoints (G.k.2.3.r, G.k.4.r, G.k.6.r)

use lib_core::model;
use lib_core::model::drug::{
	DosageInformation, DosageInformationBmc, DosageInformationFilter,
	DosageInformationForCreate, DosageInformationForUpdate, DrugActiveSubstance,
	DrugActiveSubstanceBmc, DrugActiveSubstanceFilter, DrugActiveSubstanceForCreate,
	DrugActiveSubstanceForUpdate, DrugDeviceCharacteristic,
	DrugDeviceCharacteristicBmc, DrugDeviceCharacteristicFilter,
	DrugDeviceCharacteristicForCreate, DrugDeviceCharacteristicForUpdate,
	DrugIndication, DrugIndicationBmc, DrugIndicationFilter,
	DrugIndicationForCreate, DrugIndicationForUpdate, DrugInformationBmc,
	FdaDeviceCode, FdaDeviceCodeBmc, FdaDeviceCodeFilter, FdaDeviceCodeForCreate,
	FdaDeviceCodeForUpdate, FdaDeviceInformation, FdaDeviceInformationBmc,
	FdaDeviceInformationFilter, FdaDeviceInformationForCreate,
	FdaDeviceInformationForUpdate,
};
use lib_rest_core::Result;
use serde::Deserialize;
use uuid::Uuid;

fn ensure_drug_scope(
	path_drug_id: Uuid,
	entity_drug_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	if path_drug_id != entity_drug_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

// -- FDA Device Information (FDA.G.k.12.r)

#[derive(Deserialize)]
pub struct FdaDeviceCodeInput {
	#[serde(alias = "valueCode")]
	pub value_code: String,
}

#[derive(Deserialize)]
pub struct FdaDeviceReplaceInput {
	pub malfunction: Option<bool>,
	#[serde(alias = "deviceBrandName")]
	pub device_brand_name: Option<String>,
	#[serde(alias = "deviceBrandNameNullFlavor")]
	pub device_brand_name_null_flavor: Option<String>,
	#[serde(alias = "commonDeviceName")]
	pub common_device_name: Option<String>,
	#[serde(alias = "commonDeviceNameNullFlavor")]
	pub common_device_name_null_flavor: Option<String>,
	#[serde(alias = "deviceProductCode")]
	pub device_product_code: Option<String>,
	#[serde(alias = "manufacturerName")]
	pub manufacturer_name: Option<String>,
	#[serde(alias = "manufacturerAddress")]
	pub manufacturer_address: Option<String>,
	#[serde(alias = "manufacturerCity")]
	pub manufacturer_city: Option<String>,
	#[serde(alias = "manufacturerState")]
	pub manufacturer_state: Option<String>,
	#[serde(alias = "manufacturerCountry")]
	pub manufacturer_country: Option<String>,
	#[serde(alias = "deviceUsage")]
	pub device_usage: Option<String>,
	#[serde(alias = "deviceLotNumber")]
	pub device_lot_number: Option<String>,
	#[serde(alias = "operatorOfDevice")]
	pub operator_of_device: Option<String>,
	#[serde(default)]
	#[serde(alias = "followUpTypes")]
	pub follow_up_types: Vec<FdaDeviceCodeInput>,
	#[serde(default)]
	#[serde(alias = "deviceProblemCodes")]
	pub device_problem_codes: Vec<FdaDeviceCodeInput>,
	#[serde(default)]
	#[serde(alias = "remedialActions")]
	pub remedial_actions: Vec<FdaDeviceCodeInput>,
}

#[derive(Deserialize)]
pub struct ReplaceFdaDevicesInput {
	#[serde(default)]
	pub devices: Vec<FdaDeviceReplaceInput>,
}

pub(crate) async fn persist_fda_devices(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	drug_id: Uuid,
	devices: Vec<FdaDeviceReplaceInput>,
) -> Result<Vec<FdaDeviceInformation>> {
	let mut filter = FdaDeviceInformationFilter::default();
	filter.drug_id = Some(modql::filter::OpValsValue::from(vec![
		modql::filter::OpValValue::Eq(serde_json::json!(drug_id.to_string())),
	]));
	for existing in FdaDeviceInformationBmc::list(
		ctx,
		mm,
		Some(vec![filter]),
		Some(modql::filter::ListOptions::default()),
	)
	.await?
	{
		FdaDeviceInformationBmc::delete(ctx, mm, existing.id).await?;
	}

	let mut created = Vec::with_capacity(devices.len());
	for (device_index, device) in devices.into_iter().enumerate() {
		let id = FdaDeviceInformationBmc::create(
			ctx,
			mm,
			FdaDeviceInformationForCreate {
				drug_id,
				sequence_number: device_index as i32 + 1,
				malfunction: device.malfunction,
				device_brand_name: device.device_brand_name,
				device_brand_name_null_flavor: device.device_brand_name_null_flavor,
				common_device_name: device.common_device_name,
				common_device_name_null_flavor: device
					.common_device_name_null_flavor,
				device_product_code: device.device_product_code,
				manufacturer_name: device.manufacturer_name,
				manufacturer_address: device.manufacturer_address,
				manufacturer_city: device.manufacturer_city,
				manufacturer_state: device.manufacturer_state,
				manufacturer_country: device.manufacturer_country,
				device_usage: device.device_usage,
				device_lot_number: device.device_lot_number,
				operator_of_device: device.operator_of_device,
			},
		)
		.await?;
		for (element, codes) in [
			("follow_up_type", device.follow_up_types),
			("device_problem", device.device_problem_codes),
			("remedial_action", device.remedial_actions),
		] {
			for (code_index, code) in codes.into_iter().enumerate() {
				FdaDeviceCodeBmc::create(
					ctx,
					mm,
					FdaDeviceCodeForCreate {
						device_id: id,
						element: element.to_string(),
						sequence_number: code_index as i32 + 1,
						value_code: code.value_code,
					},
				)
				.await?;
			}
		}
		created.push(FdaDeviceInformationBmc::get(ctx, mm, id).await?);
	}
	Ok(created)
}

pub async fn replace_fda_devices(
	axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
	ctx_w: lib_web::middleware::mw_auth::CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	axum::extract::Path((case_id, drug_id)): axum::extract::Path<(Uuid, Uuid)>,
	axum::Json(params): axum::Json<
		lib_rest_core::rest_params::ParamsForUpdate<ReplaceFdaDevicesInput>,
	>,
) -> Result<(
	axum::http::StatusCode,
	axum::Json<
		lib_rest_core::rest_result::DataRestResult<Vec<FdaDeviceInformation>>,
	>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("fda_device_information:replace:drug:{drug_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				DrugInformationBmc::get_in_case(ctx, mm, case_id, drug_id).await?;
				let created =
					persist_fda_devices(ctx, mm, drug_id, params.data.devices)
						.await?;
				Ok((
					axum::http::StatusCode::OK,
					axum::Json(lib_rest_core::rest_result::DataRestResult {
						data: created,
					}),
				))
			})
		},
	)
	.await
}

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: FdaDeviceInformationBmc,
	Entity: FdaDeviceInformation,
	ForCreate: FdaDeviceInformationForCreate,
	ForUpdate: FdaDeviceInformationForUpdate,
	Filter: FdaDeviceInformationFilter,
	CreateFn: create_fda_device_information,
	ListFn: list_fda_device_information,
	GetFn: get_fda_device_information,
	UpdateFn: update_fda_device_information,
	DeleteFn: delete_fda_device_information,
	RestoreFn: restore_fda_device_information,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "fda_device_information"
}

async fn ensure_device_scope(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	drug_id: Uuid,
	device_id: Uuid,
) -> Result<()> {
	let device = FdaDeviceInformationBmc::get(ctx, mm, device_id).await?;
	ensure_drug_scope(drug_id, device.drug_id, device_id, "fda_device_information")
}

pub async fn create_fda_device_code(
	axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
	ctx_w: lib_web::middleware::mw_auth::CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	axum::extract::Path((case_id, drug_id, device_id)): axum::extract::Path<(
		Uuid,
		Uuid,
		Uuid,
	)>,
	axum::Json(params): axum::Json<
		lib_rest_core::rest_params::ParamsForCreate<FdaDeviceCodeForCreate>,
	>,
) -> Result<(
	axum::http::StatusCode,
	axum::Json<lib_rest_core::rest_result::DataRestResult<FdaDeviceCode>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("fda_device_codes:new:device:{device_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_device_scope(ctx, mm, drug_id, device_id).await?;
				let mut data = params.data;
				data.device_id = device_id;
				let id = FdaDeviceCodeBmc::create(ctx, mm, data).await?;
				let data = FdaDeviceCodeBmc::get(ctx, mm, id).await?;
				Ok((
					axum::http::StatusCode::CREATED,
					axum::Json(lib_rest_core::rest_result::DataRestResult { data }),
				))
			})
		},
	)
	.await
}

pub async fn list_fda_device_codes(
	axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
	ctx_w: lib_web::middleware::mw_auth::CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	axum::extract::Path((case_id, drug_id, device_id)): axum::extract::Path<(
		Uuid,
		Uuid,
		Uuid,
	)>,
) -> Result<(
	axum::http::StatusCode,
	axum::Json<lib_rest_core::rest_result::DataRestResult<Vec<FdaDeviceCode>>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("fda_device_codes:list:device:{device_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				ensure_device_scope(ctx, mm, drug_id, device_id).await?;
				let mut filter = FdaDeviceCodeFilter::default();
				filter.device_id = Some(modql::filter::OpValsValue::from(vec![
					modql::filter::OpValValue::Eq(serde_json::json!(
						device_id.to_string()
					)),
				]));
				let data = FdaDeviceCodeBmc::list(
					ctx,
					mm,
					Some(vec![filter]),
					Some(modql::filter::ListOptions::default()),
				)
				.await?;
				Ok((
					axum::http::StatusCode::OK,
					axum::Json(lib_rest_core::rest_result::DataRestResult { data }),
				))
			})
		},
	)
	.await
}

async fn scoped_fda_device_code(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	drug_id: Uuid,
	device_id: Uuid,
	id: Uuid,
) -> Result<FdaDeviceCode> {
	ensure_device_scope(ctx, mm, drug_id, device_id).await?;
	let code = FdaDeviceCodeBmc::get(ctx, mm, id).await?;
	if code.device_id != device_id {
		return Err(model::Error::EntityUuidNotFound {
			entity: "fda_device_codes",
			id,
		}
		.into());
	}
	Ok(code)
}

pub async fn get_fda_device_code(
	axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
	ctx_w: lib_web::middleware::mw_auth::CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	axum::extract::Path((case_id, drug_id, device_id, id)): axum::extract::Path<(
		Uuid,
		Uuid,
		Uuid,
		Uuid,
	)>,
) -> Result<(
	axum::http::StatusCode,
	axum::Json<lib_rest_core::rest_result::DataRestResult<FdaDeviceCode>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_read(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("fda_device_codes:{id}:device:{device_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let data =
					scoped_fda_device_code(ctx, mm, drug_id, device_id, id).await?;
				Ok((
					axum::http::StatusCode::OK,
					axum::Json(lib_rest_core::rest_result::DataRestResult { data }),
				))
			})
		},
	)
	.await
}

pub async fn update_fda_device_code(
	axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
	ctx_w: lib_web::middleware::mw_auth::CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	axum::extract::Path((case_id, drug_id, device_id, id)): axum::extract::Path<(
		Uuid,
		Uuid,
		Uuid,
		Uuid,
	)>,
	axum::Json(params): axum::Json<
		lib_rest_core::rest_params::ParamsForUpdate<FdaDeviceCodeForUpdate>,
	>,
) -> Result<(
	axum::http::StatusCode,
	axum::Json<lib_rest_core::rest_result::DataRestResult<FdaDeviceCode>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("fda_device_codes:{id}:device:{device_id}"),
		move |ctx, mm| {
			Box::pin(async move {
				scoped_fda_device_code(ctx, mm, drug_id, device_id, id).await?;
				FdaDeviceCodeBmc::update(ctx, mm, id, params.data).await?;
				let data = FdaDeviceCodeBmc::get(ctx, mm, id).await?;
				Ok((
					axum::http::StatusCode::OK,
					axum::Json(lib_rest_core::rest_result::DataRestResult { data }),
				))
			})
		},
	)
	.await
}

async fn set_fda_device_code_deleted(
	ctx: &lib_core::ctx::Ctx,
	mm: &lib_core::model::ModelManager,
	drug_id: Uuid,
	device_id: Uuid,
	id: Uuid,
	deleted: bool,
) -> Result<axum::http::StatusCode> {
	scoped_fda_device_code(ctx, mm, drug_id, device_id, id).await?;
	if deleted {
		FdaDeviceCodeBmc::delete(ctx, mm, id).await?;
	} else {
		FdaDeviceCodeBmc::restore(ctx, mm, id).await?;
	}
	Ok(axum::http::StatusCode::NO_CONTENT)
}

macro_rules! fda_device_code_delete_handler {
	($name:ident, $deleted:literal) => {
		pub async fn $name(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, drug_id, device_id, id)): axum::extract::Path<(
				Uuid,
				Uuid,
				Uuid,
				Uuid,
			)>,
		) -> Result<axum::http::StatusCode> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("fda_device_codes:{id}:device:{device_id}"),
				move |ctx, mm| {
					Box::pin(set_fda_device_code_deleted(
						ctx, mm, drug_id, device_id, id, $deleted,
					))
				},
			)
			.await
		}
	};
}

fda_device_code_delete_handler!(delete_fda_device_code, true);
fda_device_code_delete_handler!(restore_fda_device_code, false);

// -- Drug Active Substances (G.k.2.3.r)

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: DrugActiveSubstanceBmc,
	Entity: DrugActiveSubstance,
	ForCreate: DrugActiveSubstanceForCreate,
	ForUpdate: DrugActiveSubstanceForUpdate,
	Filter: DrugActiveSubstanceFilter,
	CreateFn: create_drug_active_substance,
	ListFn: list_drug_active_substances,
	GetFn: get_drug_active_substance,
	UpdateFn: update_drug_active_substance,
	DeleteFn: delete_drug_active_substance,
	RestoreFn: restore_drug_active_substance,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "drug_active_substances"
}

// -- Dosage Information (G.k.4.r)

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: DosageInformationBmc,
	Entity: DosageInformation,
	ForCreate: DosageInformationForCreate,
	ForUpdate: DosageInformationForUpdate,
	Filter: DosageInformationFilter,
	CreateFn: create_dosage_information,
	ListFn: list_dosage_information,
	GetFn: get_dosage_information,
	UpdateFn: update_dosage_information,
	DeleteFn: delete_dosage_information,
	RestoreFn: restore_dosage_information,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "dosage_information"
}

// -- Drug Indications (G.k.6.r)

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: DrugIndicationBmc,
	Entity: DrugIndication,
	ForCreate: DrugIndicationForCreate,
	ForUpdate: DrugIndicationForUpdate,
	Filter: DrugIndicationFilter,
	CreateFn: create_drug_indication,
	ListFn: list_drug_indications,
	GetFn: get_drug_indication,
	UpdateFn: update_drug_indication,
	DeleteFn: delete_drug_indication,
	RestoreFn: restore_drug_indication,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "drug_indications"
}

// -- Drug Device Characteristics (FDA device authority)

lib_rest_core::generate_drug_child_rest_fns! {
	Bmc: DrugDeviceCharacteristicBmc,
	Entity: DrugDeviceCharacteristic,
	ForCreate: DrugDeviceCharacteristicForCreate,
	ForUpdate: DrugDeviceCharacteristicForUpdate,
	Filter: DrugDeviceCharacteristicFilter,
	CreateFn: create_drug_device_characteristic,
	ListFn: list_drug_device_characteristics,
	GetFn: get_drug_device_characteristic,
	UpdateFn: update_drug_device_characteristic,
	DeleteFn: delete_drug_device_characteristic,
	RestoreFn: restore_drug_device_characteristic,
	ParentField: drug_id,
	ScopeFn: ensure_drug_scope,
	EntityName: "drug_device_characteristics"
}
