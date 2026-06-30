use lib_core::model::acs::{
	DRUG_CREATE, DRUG_DELETE, DRUG_LIST, DRUG_READ, DRUG_UPDATE,
};
use lib_core::model::drug::{
	DrugInformationBmc, DrugInformationForCreate, DrugInformationForUpdate,
};
use lib_rest_core::prelude::*;
use lib_web::middleware::mw_auth::CtxW;

// Case-scoped CRUD functions:
// - create_drug_information
// - get_drug_information
// - list_drug_informations
// - update_drug_information
// - delete_drug_information
generate_case_rest_fns! {
	Bmc: DrugInformationBmc,
	Entity: lib_core::model::drug::DrugInformation,
	ForCreate: DrugInformationForCreate,
	ForUpdate: DrugInformationForUpdate,
	Suffix: drug_information,
	PermCreate: DRUG_CREATE,
	PermRead: DRUG_READ,
	PermUpdate: DRUG_UPDATE,
	PermDelete: DRUG_DELETE,
	PermList: DRUG_LIST
}

pub async fn restore_drug_information(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_core::model::drug::DrugInformation>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, DRUG_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	DrugInformationBmc::get_in_case_with_deleted(&ctx, &mm, case_id, id, true)
		.await?;
	DrugInformationBmc::restore_in_case(&ctx, &mm, case_id, id).await?;
	let data = DrugInformationBmc::get_in_case(&ctx, &mm, case_id, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data })))
}
