use lib_core::model::test_result::{
	TestResultBmc, TestResultForCreate, TestResultForUpdate,
};
use lib_rest_core::prelude::*;
use lib_web::middleware::mw_auth::CtxW;

mod generated {
	use super::*;
	generate_case_rest_fns! {
		Bmc: TestResultBmc,
		Entity: lib_core::model::test_result::TestResult,
		ForCreate: TestResultForCreate,
		ForUpdate: TestResultForUpdate,
		Suffix: test_result
	}
}
pub use generated::{
	create_test_result, delete_test_result, get_test_result, list_test_results,
};

pub async fn update_test_result(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
	Json(params): Json<ParamsForUpdate<serde_json::Value>>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_core::model::test_result::TestResult>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("test_result:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				let ParamsForUpdate { data } = params;
				let update: TestResultForUpdate =
					serde_json::from_value(data.clone()).map_err(|_| {
						lib_rest_core::Error::BadRequest {
							message: "Invalid test result fields.".into(),
						}
					})?;
				let clear_fields: Vec<&str> = [
					"test_name",
					"test_date",
					"test_date_null_flavor",
					"test_meddra_version",
					"test_meddra_code",
					"test_result_code",
					"test_result_value",
					"test_result_qualifier",
					"test_result_unit",
					"result_unstructured",
					"normal_low_value",
					"normal_high_value",
					"comments",
					"more_info_available",
				]
				.into_iter()
				.filter(|field| {
					data.get(*field).is_some_and(serde_json::Value::is_null)
				})
				.collect();
				TestResultBmc::update_in_case_patch(
					ctx,
					mm,
					case_id,
					id,
					update,
					&clear_fields,
				)
				.await?;
				let data = TestResultBmc::get_in_case_with_deleted(
					ctx, mm, case_id, id, true,
				)
				.await?;
				Ok((StatusCode::OK, Json(DataRestResult { data })))
			})
		},
	)
	.await
}

pub async fn restore_test_result(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_core::model::test_result::TestResult>>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("test_result:{id}"),
		move |ctx, mm| {
			Box::pin(async move {
				TestResultBmc::get_in_case_with_deleted(ctx, mm, case_id, id, true)
					.await?;
				TestResultBmc::restore_in_case(ctx, mm, case_id, id).await?;
				let data = TestResultBmc::get_in_case(ctx, mm, case_id, id).await?;
				Ok((StatusCode::OK, Json(DataRestResult { data })))
			})
		},
	)
	.await
}
