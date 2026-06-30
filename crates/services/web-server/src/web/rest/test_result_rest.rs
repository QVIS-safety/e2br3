use lib_core::model::acs::{
	TEST_RESULT_CREATE, TEST_RESULT_DELETE, TEST_RESULT_LIST, TEST_RESULT_READ,
	TEST_RESULT_UPDATE,
};
use lib_core::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};
use lib_rest_core::prelude::*;
use lib_web::middleware::mw_auth::CtxW;

// Case-scoped CRUD functions:
// - create_test_result
// - get_test_result
// - list_test_results
// - update_test_result
// - delete_test_result
generate_case_rest_fns! {
	Bmc: TestResultBmc,
	Entity: lib_core::model::test_result::TestResult,
	ForCreate: TestResultForCreate,
	ForUpdate: TestResultForUpdate,
	Suffix: test_result,
	PermCreate: TEST_RESULT_CREATE,
	PermRead: TEST_RESULT_READ,
	PermUpdate: TEST_RESULT_UPDATE,
	PermDelete: TEST_RESULT_DELETE,
	PermList: TEST_RESULT_LIST
}

pub async fn restore_test_result(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_core::model::test_result::TestResult>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, TEST_RESULT_UPDATE)?;
	lib_rest_core::require_case_write_allowed(&ctx, &mm, case_id).await?;
	TestResultBmc::get_in_case_with_deleted(&ctx, &mm, case_id, id, true).await?;
	TestResultBmc::restore_in_case(&ctx, &mm, case_id, id).await?;
	let data = TestResultBmc::get_in_case(&ctx, &mm, case_id, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data })))
}
