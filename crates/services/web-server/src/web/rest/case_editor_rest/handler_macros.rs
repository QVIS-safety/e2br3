macro_rules! repeatable_page_row_read_handler {
	($fn_name:ident, $build_response:ident $(,)?) => {
		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, row_id)): axum::extract::Path<(uuid::Uuid, uuid::Uuid)>,
			axum::extract::Query(query): axum::extract::Query<$crate::web::rest::case_editor_rest::common::CaseEditorPageProjectionQuery>,
		) -> lib_rest_core::Result<(axum::http::StatusCode, axum::Json<serde_json::Value>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_read(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("editor/{}/{}", stringify!($fn_name), row_id),
				move |ctx, mm| Box::pin(async move {
					let response = $build_response(
						ctx,
						mm,
						case_id,
						row_id,
						$crate::web::rest::case_editor_rest::common::query_authorities_csv(&query)?,
					)
					.await?;
					Ok((axum::http::StatusCode::OK, axum::Json(response)))
				}),
			)
			.await
		}
	};
}

macro_rules! repeatable_page_row_create_handler {
	(
		$fn_name:ident,
		apply: $apply_fn:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		aliases: $aliases:expr,
		extras_fn: $extras_fn:ident,
		build_response: $build_response:ident $(,)?
	) => {
		pub(crate) async fn $apply_fn(
			ctx: &lib_core::ctx::Ctx,
			mm: &lib_core::model::ModelManager,
			case_id: uuid::Uuid,
			request: &$crate::web::rest::case_editor_dto::CaseEditorPagePatchRequest,
		) -> lib_rest_core::Result<(uuid::Uuid, Option<String>)> {
			let requested_authorities =
				$crate::web::rest::case_editor_rest::common::validate_request_projection_context(request.authorities.as_deref())?;
			let row = $crate::web::rest::case_editor_rest::common::required_row_object($section, &request.rows, $row_key)?;
			$crate::web::rest::case_editor_rest::common::validate_row_payload($section, $row_key, row, None)?;
			let extras = $extras_fn(ctx, mm, case_id, row).await?;
			let value = $crate::web::rest::case_editor_rest::common::row_model_value($section, "", row, $aliases, &extras);
			let create = $crate::web::rest::case_editor_rest::common::parse_row_model::<$model>($section, $row_key, value)?;
			let row_id = $bmc::create(ctx, mm, create).await?;
			Ok((row_id, requested_authorities))
		}

		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
			axum::Json(request): axum::Json<$crate::web::rest::case_editor_dto::CaseEditorPagePatchRequest>,
		) -> lib_rest_core::Result<(axum::http::StatusCode, axum::Json<serde_json::Value>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				concat!("editor/", $section, "/", $row_key),
				move |ctx, mm| Box::pin(async move {
					let (row_id, requested_authorities) =
						$apply_fn(ctx, mm, case_id, &request).await?;
					$crate::web::rest::case_editor_rest::common::mark_editor_validation_summary_stale(
						ctx,
						mm,
						case_id,
						requested_authorities.clone(),
					)
					.await?;
					let response =
						$build_response(ctx, mm, case_id, row_id, requested_authorities)
							.await?;
					Ok((axum::http::StatusCode::CREATED, axum::Json(response)))
				}),
			)
			.await
		}
	};
	(
		$fn_name:ident,
		apply: $apply_fn:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		aliases: $aliases:expr,
		extras: |$case_id:ident, $row:ident| $extras:expr,
		build_response: $build_response:ident $(,)?
	) => {
		pub(crate) async fn $apply_fn(
			ctx: &lib_core::ctx::Ctx,
			mm: &lib_core::model::ModelManager,
			case_id: uuid::Uuid,
			request: &$crate::web::rest::case_editor_dto::CaseEditorPagePatchRequest,
		) -> lib_rest_core::Result<(uuid::Uuid, Option<String>)> {
			let requested_authorities =
				$crate::web::rest::case_editor_rest::common::validate_request_projection_context(request.authorities.as_deref())?;
			let row = $crate::web::rest::case_editor_rest::common::required_row_object($section, &request.rows, $row_key)?;
			$crate::web::rest::case_editor_rest::common::validate_row_payload($section, $row_key, row, None)?;
			let extras = {
				let $case_id = case_id;
				let $row = row;
				$extras
			};
			let value = $crate::web::rest::case_editor_rest::common::row_model_value($section, "", row, $aliases, &extras);
			let create = $crate::web::rest::case_editor_rest::common::parse_row_model::<$model>($section, $row_key, value)?;
			let row_id = $bmc::create(ctx, mm, create).await?;
			Ok((row_id, requested_authorities))
		}

		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
			axum::Json(request): axum::Json<$crate::web::rest::case_editor_dto::CaseEditorPagePatchRequest>,
		) -> lib_rest_core::Result<(axum::http::StatusCode, axum::Json<serde_json::Value>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				concat!("editor/", $section, "/", $row_key),
				move |ctx, mm| Box::pin(async move {
					let (row_id, requested_authorities) =
						$apply_fn(ctx, mm, case_id, &request).await?;
					$crate::web::rest::case_editor_rest::common::mark_editor_validation_summary_stale(
						ctx,
						mm,
						case_id,
						requested_authorities.clone(),
					)
					.await?;
					let response =
						$build_response(ctx, mm, case_id, row_id, requested_authorities)
							.await?;
					Ok((axum::http::StatusCode::CREATED, axum::Json(response)))
				}),
			)
			.await
		}
	};
}

macro_rules! repeatable_page_row_patch_handler {
	(@restore [], $row:ident, $bmc:ident, $ctx:ident, $mm:ident, $row_id:ident) => {};
	(@restore [true], $row:ident, $bmc:ident, $ctx:ident, $mm:ident, $row_id:ident) => {
		if $row.get("deleted").and_then(serde_json::Value::as_bool) == Some(false) {
			let current = $bmc::get($ctx, $mm, $row_id).await?;
			if current.deleted {
				$bmc::restore($ctx, $mm, $row_id).await?;
			}
		}
	};
	(@restore_in_case [], $row:ident, $bmc:ident, $ctx:ident, $mm:ident, $case_id:ident, $row_id:ident) => {};
	(@restore_in_case [true], $row:ident, $bmc:ident, $ctx:ident, $mm:ident, $case_id:ident, $row_id:ident) => {
		if $row.get("deleted").and_then(serde_json::Value::as_bool) == Some(false) {
			let current = $bmc::get_in_case_with_deleted($ctx, $mm, $case_id, $row_id, true).await?;
			if current.deleted {
				$bmc::restore_in_case($ctx, $mm, $case_id, $row_id).await?;
			}
		}
	};
	(
		$fn_name:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		verify: $verify_fn:ident,
		aliases: $aliases:expr,
		base_patch: true,
		$(restore_on_false: $restore_on_false:tt,)?
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, row_id)): axum::extract::Path<(uuid::Uuid, uuid::Uuid)>,
			axum::Json(request): axum::Json<$crate::web::rest::case_editor_dto::CaseEditorPagePatchRequest>,
		) -> lib_rest_core::Result<(axum::http::StatusCode, axum::Json<serde_json::Value>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("editor/{}/{}/{}", $section, $row_key, row_id),
				move |ctx, mm| Box::pin(async move {
					let requested_authorities =
						$crate::web::rest::case_editor_rest::common::validate_request_projection_context(request.authorities.as_deref())?;
					$verify_fn(ctx, mm, case_id, row_id).await?;
					let row = $crate::web::rest::case_editor_rest::common::required_row_object($section, &request.rows, $row_key)?;
					$crate::web::rest::case_editor_rest::common::validate_row_payload($section, $row_key, row, None)?;
					let clear_fields = $crate::web::rest::case_editor_rest::common::explicit_null_model_fields(row, $aliases);
					let value = $crate::web::rest::case_editor_rest::common::row_model_value($section, "", row, $aliases, &[]);
					let update = $crate::web::rest::case_editor_rest::common::parse_row_model::<$model>($section, $row_key, value)?;
					lib_core::model::update_uuid_patch::<$bmc, $model>(
						ctx,
						mm,
						row_id,
						update,
						&clear_fields,
					)
					.await?;
					repeatable_page_row_patch_handler!(
						@restore [$($restore_on_false)?], row, $bmc, ctx, mm, row_id
					);
					$crate::web::rest::case_editor_rest::common::mark_editor_validation_summary_stale(
						ctx, mm, case_id, requested_authorities.clone(),
					)
					.await?;
					let response =
						$build_response(ctx, mm, case_id, row_id, requested_authorities)
							.await?;
					Ok((axum::http::StatusCode::OK, axum::Json(response)))
				}),
			)
			.await
		}
	};
	(
		$fn_name:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		verify: $verify_fn:ident,
		aliases: $aliases:expr,
		$(restore_on_false: $restore_on_false:tt,)?
		$(restore_in_case_on_false: $restore_in_case_on_false:tt,)?
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, row_id)): axum::extract::Path<(uuid::Uuid, uuid::Uuid)>,
			axum::Json(request): axum::Json<$crate::web::rest::case_editor_dto::CaseEditorPagePatchRequest>,
		) -> lib_rest_core::Result<(axum::http::StatusCode, axum::Json<serde_json::Value>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("editor/{}/{}/{}", $section, $row_key, row_id),
				move |ctx, mm| Box::pin(async move {
					let requested_authorities =
						$crate::web::rest::case_editor_rest::common::validate_request_projection_context(request.authorities.as_deref())?;
					$verify_fn(ctx, mm, case_id, row_id).await?;
					let row = $crate::web::rest::case_editor_rest::common::required_row_object($section, &request.rows, $row_key)?;
					$crate::web::rest::case_editor_rest::common::validate_row_payload($section, $row_key, row, None)?;
					let clear_fields = $crate::web::rest::case_editor_rest::common::explicit_null_model_fields(row, $aliases);
					let value = $crate::web::rest::case_editor_rest::common::row_model_value($section, "", row, $aliases, &[]);
					let update = $crate::web::rest::case_editor_rest::common::parse_row_model::<$model>($section, $row_key, value)?;
					$bmc::update_patch(ctx, mm, row_id, update, &clear_fields)
					.await?;
					repeatable_page_row_patch_handler!(
						@restore [$($restore_on_false)?], row, $bmc, ctx, mm, row_id
					);
					repeatable_page_row_patch_handler!(
						@restore_in_case [$($restore_in_case_on_false)?], row, $bmc, ctx, mm, case_id, row_id
					);
					$crate::web::rest::case_editor_rest::common::mark_editor_validation_summary_stale(
						ctx, mm, case_id, requested_authorities.clone(),
					)
					.await?;
					let response =
						$build_response(ctx, mm, case_id, row_id, requested_authorities)
							.await?;
					Ok((axum::http::StatusCode::OK, axum::Json(response)))
				}),
			)
			.await
		}
	};
	(
		$fn_name:ident,
		section: $section:expr,
		row_key: $row_key:expr,
		bmc: $bmc:ident,
		model: $model:ty,
		aliases: $aliases:expr,
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, row_id)): axum::extract::Path<(uuid::Uuid, uuid::Uuid)>,
			axum::Json(request): axum::Json<$crate::web::rest::case_editor_dto::CaseEditorPagePatchRequest>,
		) -> lib_rest_core::Result<(axum::http::StatusCode, axum::Json<serde_json::Value>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("editor/{}/{}/{}", $section, $row_key, row_id),
				move |ctx, mm| Box::pin(async move {
					let requested_authorities =
						$crate::web::rest::case_editor_rest::common::validate_request_projection_context(request.authorities.as_deref())?;
					$bmc::get_in_case(ctx, mm, case_id, row_id).await?;
					let row = $crate::web::rest::case_editor_rest::common::required_row_object($section, &request.rows, $row_key)?;
					$crate::web::rest::case_editor_rest::common::validate_row_payload($section, $row_key, row, None)?;
					let clear_fields = $crate::web::rest::case_editor_rest::common::explicit_null_model_fields(row, $aliases);
					let value = $crate::web::rest::case_editor_rest::common::row_model_value($section, "", row, $aliases, &[]);
					let update = $crate::web::rest::case_editor_rest::common::parse_row_model::<$model>($section, $row_key, value)?;
					$bmc::update_patch(ctx, mm, row_id, update, &clear_fields)
					.await?;
					$crate::web::rest::case_editor_rest::common::mark_editor_validation_summary_stale(
						ctx, mm, case_id, requested_authorities.clone(),
					)
					.await?;
					let response =
						$build_response(ctx, mm, case_id, row_id, requested_authorities)
							.await?;
					Ok((axum::http::StatusCode::OK, axum::Json(response)))
				}),
			)
			.await
		}
	};
}

macro_rules! repeatable_page_row_delete_handler {
	(
		$fn_name:ident,
		bmc: $bmc:ident,
		verify: $verify_fn:ident $(,)?
	) => {
		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, row_id)): axum::extract::Path<(uuid::Uuid, uuid::Uuid)>,
		) -> lib_rest_core::Result<axum::http::StatusCode> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("editor/{}/{}", stringify!($fn_name), row_id),
				move |ctx, mm| Box::pin(async move {
					$verify_fn(ctx, mm, case_id, row_id).await?;
					$bmc::delete(ctx, mm, row_id).await?;
					$crate::web::rest::case_editor_rest::common::mark_editor_validation_summary_stale(ctx, mm, case_id, None).await?;
					Ok(axum::http::StatusCode::NO_CONTENT)
				}),
			)
			.await
		}
	};
}

macro_rules! repeatable_list_handler {
	(
		$fn_name:ident,
		$row_dto:ty,
		$load_rows:ident,
		include_deleted
		$(,)?
	) => {
		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
		) -> lib_rest_core::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::web::rest::case_editor_dto::CaseEditorListResponse<$row_dto>>,
		)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_read(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				concat!("editor/", stringify!($fn_name)),
				move |ctx, mm| Box::pin(async move {
					let rows = $load_rows(ctx, mm, case_id, false).await?;
					Ok((
						axum::http::StatusCode::OK,
						axum::Json($crate::web::rest::case_editor_dto::CaseEditorListResponse { case_id, rows }),
					))
				}),
			)
			.await
		}
	};
	(
		$fn_name:ident,
		$row_dto:ty,
		$load_rows:ident
		$(,)?
	) => {
		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
		) -> lib_rest_core::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::web::rest::case_editor_dto::CaseEditorListResponse<$row_dto>>,
		)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_read(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				concat!("editor/", stringify!($fn_name)),
				move |ctx, mm| Box::pin(async move {
					let rows = $load_rows(ctx, mm, case_id).await?;
					Ok((
						axum::http::StatusCode::OK,
						axum::Json($crate::web::rest::case_editor_dto::CaseEditorListResponse { case_id, rows }),
					))
				}),
			)
			.await
		}
	};
}

macro_rules! repeatable_page_row_delete_restore_handlers {
	(
		delete: $delete_fn:ident,
		restore: $restore_fn:ident,
		bmc: $bmc:ident,
		build_response: $build_response:ident $(,)?
	) => {
		pub async fn $delete_fn(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, row_id)): axum::extract::Path<(uuid::Uuid, uuid::Uuid)>,
		) -> lib_rest_core::Result<axum::http::StatusCode> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("editor/{}/{}", stringify!($delete_fn), row_id),
				move |ctx, mm| Box::pin(async move {
					$bmc::get_in_case(ctx, mm, case_id, row_id).await?;
					$bmc::delete(ctx, mm, row_id).await?;
					$crate::web::rest::case_editor_rest::common::mark_editor_validation_summary_stale(ctx, mm, case_id, None).await?;
					Ok(axum::http::StatusCode::NO_CONTENT)
				}),
			)
			.await
		}

		pub async fn $restore_fn(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, row_id)): axum::extract::Path<(uuid::Uuid, uuid::Uuid)>,
		) -> lib_rest_core::Result<(axum::http::StatusCode, axum::Json<serde_json::Value>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				format!("editor/{}/{}", stringify!($restore_fn), row_id),
				move |ctx, mm| Box::pin(async move {
					$bmc::get_in_case_with_deleted(ctx, mm, case_id, row_id, true)
						.await?;
					$bmc::restore_in_case(ctx, mm, case_id, row_id).await?;
					$crate::web::rest::case_editor_rest::common::mark_editor_validation_summary_stale(ctx, mm, case_id, None).await?;
					let response = $build_response(ctx, mm, case_id, row_id, None).await?;
					Ok((axum::http::StatusCode::OK, axum::Json(response)))
				}),
			)
			.await
		}
	};
}

macro_rules! direct_page_projection_handler {
	(
		$fn_name:ident,
		$section:literal,
		$loader:ident
		$(,)?
	) => {
		pub async fn $fn_name(
			axum::extract::State(mm): axum::extract::State<lib_core::model::ModelManager>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
			axum::extract::Query(query): axum::extract::Query<$crate::web::rest::case_editor_rest::common::CaseEditorPageProjectionQuery>,
		) -> lib_rest_core::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::web::rest::case_editor_dto::CaseEditorPageProjectionResponse>,
		)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_read(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				concat!("editor/", $section),
				move |ctx, mm| Box::pin(async move {
					let projection = $crate::web::rest::case_editor_rest::common::direct_page_projection_response(
						ctx,
						mm,
						case_id,
						$section,
						$crate::web::rest::case_editor_rest::common::query_authorities_csv(&query)?,
						$loader(ctx, mm, case_id).await?,
					)
					.await?;
					Ok((axum::http::StatusCode::OK, axum::Json(projection)))
				}),
			)
			.await
		}
	};
}

pub(super) use direct_page_projection_handler;
pub(super) use repeatable_list_handler;
pub(super) use repeatable_page_row_create_handler;
pub(super) use repeatable_page_row_delete_handler;
pub(super) use repeatable_page_row_delete_restore_handlers;
pub(super) use repeatable_page_row_patch_handler;
pub(super) use repeatable_page_row_read_handler;
