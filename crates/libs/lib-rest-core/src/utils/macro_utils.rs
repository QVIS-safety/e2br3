/// Create the base crud rpc functions following the common pattern.
/// - `create_...`
/// - `get_...`
///
/// NOTE: Make sure to import the Ctx, ModelManager, ... in the model that uses this macro.
///
#[macro_export]
macro_rules! generate_common_rest_fns {
    (
        Bmc: $bmc:ident,
        Entity: $entity:ty,
        ForCreate: $for_create:ty,
        ForUpdate: $for_update:ty,
        Filter: $filter:ty,
        Suffix: $suffix:ident,
        PermCreate: $perm_create:path,
        PermRead: $perm_read:path,
        PermUpdate: $perm_update:path,
        PermDelete: $perm_delete:path,
        PermList: $perm_list:path
    ) => {
        paste! {
            pub async fn [<create_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Json(params): Json<ParamsForCreate<$for_create>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_create)?;
                tracing::debug!("{:<12} - rest create {}", "HANDLER", stringify!($suffix));
                let ParamsForCreate { data } = params;
                let id = $bmc::create(&ctx, &mm, data).await?;
                let entity = $bmc::get(&ctx, &mm, id).await?;
                Ok((axum::http::StatusCode::CREATED, Json(DataRestResult { data: entity })))
            }

            pub async fn [<get_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(id): Path<Uuid>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_read)?;
                tracing::debug!(
                    "{:<12} - rest get {} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    id
                );
                let entity = $bmc::get(&ctx, &mm, id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            // Note: for now just add `s` after the suffix.
            pub async fn [<list_ $suffix s>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_list)?;
                tracing::debug!("{:<12} - rest list {}s", "HANDLER", stringify!($suffix));
                let params = ParamsList::<$filter>::from_raw_query(raw_query.as_deref())
                    .map_err(|message| $crate::Error::BadRequest { message })?;
                let entities = $bmc::list(&ctx, &mm, params.filters, params.list_options).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entities })))
            }

            pub async fn [<update_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(id): Path<Uuid>,
                Json(params): Json<ParamsForUpdate<$for_update>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_update)?;
                tracing::debug!(
                    "{:<12} - rest update {} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    id
                );
                let ParamsForUpdate { data } = params;
                $bmc::update(&ctx, &mm, id, data).await?;
                let entity = $bmc::get(&ctx, &mm, id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            pub async fn [<delete_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(id): Path<Uuid>,
            ) -> Result<axum::http::StatusCode> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_delete)?;
                tracing::debug!(
                    "{:<12} - rest delete {} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    id
                );
                $bmc::delete(&ctx, &mm, id).await?;
                Ok(axum::http::StatusCode::NO_CONTENT)
            }
        }
    };

    // Variant without ForUpdate (immutable entities)
    (
        Bmc: $bmc:ident,
        Entity: $entity:ty,
        ForCreate: $for_create:ty,
        Filter: $filter:ty,
        Suffix: $suffix:ident,
        PermCreate: $perm_create:path,
        PermRead: $perm_read:path,
        PermUpdate: $perm_update:path,
        PermDelete: $perm_delete:path,
        PermList: $perm_list:path
    ) => {
        paste! {
            pub async fn [<create_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Json(params): Json<ParamsForCreate<$for_create>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_create)?;
                tracing::debug!("{:<12} - rest create {}", "HANDLER", stringify!($suffix));
                let ParamsForCreate { data } = params;
                let id = $bmc::create(&ctx, &mm, data).await?;
                let entity = $bmc::get(&ctx, &mm, id).await?;
                Ok((axum::http::StatusCode::CREATED, Json(DataRestResult { data: entity })))
            }

            pub async fn [<get_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(id): Path<Uuid>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_read)?;
                tracing::debug!(
                    "{:<12} - rest get {} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    id
                );
                let entity = $bmc::get(&ctx, &mm, id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            pub async fn [<list_ $suffix s>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_list)?;
                tracing::debug!("{:<12} - rest list {}s", "HANDLER", stringify!($suffix));
                let params = ParamsList::<$filter>::from_raw_query(raw_query.as_deref())
                    .map_err(|message| $crate::Error::BadRequest { message })?;
                let entities = $bmc::list(&ctx, &mm, params.filters, params.list_options).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entities })))
            }

            pub async fn [<delete_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(id): Path<Uuid>,
            ) -> Result<axum::http::StatusCode> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_delete)?;
                tracing::debug!(
                    "{:<12} - rest delete {} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    id
                );
                $bmc::delete(&ctx, &mm, id).await?;
                Ok(axum::http::StatusCode::NO_CONTENT)
            }
        }
    };

    // Variant without Filter (no list filtering)
    (
        Bmc: $bmc:ident,
        Entity: $entity:ty,
        ForCreate: $for_create:ty,
        ForUpdate: $for_update:ty,
        Suffix: $suffix:ident,
        PermCreate: $perm_create:path,
        PermRead: $perm_read:path,
        PermUpdate: $perm_update:path,
        PermDelete: $perm_delete:path,
        PermList: $perm_list:path
    ) => {
        paste! {
            pub async fn [<create_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Json(params): Json<ParamsForCreate<$for_create>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_create)?;
                tracing::debug!("{:<12} - rest create {}", "HANDLER", stringify!($suffix));
                let ParamsForCreate { data } = params;
                let id = $bmc::create(&ctx, &mm, data).await?;
                let entity = $bmc::get(&ctx, &mm, id).await?;
                Ok((axum::http::StatusCode::CREATED, Json(DataRestResult { data: entity })))
            }

            pub async fn [<get_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(id): Path<Uuid>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_read)?;
                tracing::debug!(
                    "{:<12} - rest get {} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    id
                );
                let entity = $bmc::get(&ctx, &mm, id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            pub async fn [<list_ $suffix s>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_list)?;
                tracing::debug!("{:<12} - rest list {}s", "HANDLER", stringify!($suffix));
                let entities = $bmc::list(&ctx, &mm, None, None).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entities })))
            }

            pub async fn [<update_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(id): Path<Uuid>,
                Json(params): Json<ParamsForUpdate<$for_update>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_update)?;
                tracing::debug!(
                    "{:<12} - rest update {} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    id
                );
                let ParamsForUpdate { data } = params;
                $bmc::update(&ctx, &mm, id, data).await?;
                let entity = $bmc::get(&ctx, &mm, id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            pub async fn [<delete_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(id): Path<Uuid>,
            ) -> Result<axum::http::StatusCode> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_delete)?;
                tracing::debug!(
                    "{:<12} - rest delete {} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    id
                );
                $bmc::delete(&ctx, &mm, id).await?;
                Ok(axum::http::StatusCode::NO_CONTENT)
            }
        }
    };
}

/// Generate CRUD REST handlers for a resource nested below a drug.
#[macro_export]
macro_rules! generate_drug_child_rest_fns {
	(
        Bmc: $bmc:ident,
        Entity: $entity:ty,
        ForCreate: $for_create:ty,
        ForUpdate: $for_update:ty,
        Filter: $filter:ty,
        CreateFn: $create_fn:ident,
        ListFn: $list_fn:ident,
        GetFn: $get_fn:ident,
        UpdateFn: $update_fn:ident,
        DeleteFn: $delete_fn:ident,
        RestoreFn: $restore_fn:ident,
        ParentField: $parent_field:ident,
        ScopeFn: $scope_fn:ident,
        EntityName: $entity_name:literal,
        PermCreate: $perm_create:path,
        PermList: $perm_list:path,
        PermRead: $perm_read:path,
        PermUpdate: $perm_update:path,
        PermDelete: $perm_delete:path
    ) => {
		pub async fn $create_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, drug_id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
			axum::Json(params): axum::Json<
				$crate::rest_params::ParamsForCreate<$for_create>,
			>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_create)?;
			$crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let $crate::rest_params::ParamsForCreate { data } = params;
			let mut data = data;
			data.$parent_field = drug_id;
			let id = $bmc::create(&ctx, &mm, data).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::CREATED,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
		}

		pub async fn $list_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, drug_id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<Vec<$entity>>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_list)?;
			$crate::require_case_read_allowed(&ctx, &mm, case_id).await?;
			let mut filter: $filter = Default::default();
			filter.$parent_field = Some(modql::filter::OpValsValue::from(vec![
				modql::filter::OpValValue::Eq(
					serde_json::json!(drug_id.to_string()),
				),
			]));
			let entities = $bmc::list(
				&ctx,
				&mm,
				Some(vec![filter]),
				Some(modql::filter::ListOptions::default()),
			)
			.await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entities }),
			))
		}

		pub async fn $get_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, drug_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_read)?;
			$crate::require_case_read_allowed(&ctx, &mm, case_id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(drug_id, entity.$parent_field, id, $entity_name)?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
		}

		pub async fn $update_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, drug_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
				uuid::Uuid,
			)>,
			axum::Json(params): axum::Json<
				$crate::rest_params::ParamsForUpdate<$for_update>,
			>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_update)?;
			$crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let $crate::rest_params::ParamsForUpdate { data } = params;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(drug_id, entity.$parent_field, id, $entity_name)?;
			$bmc::update(&ctx, &mm, id, data).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
		}

		pub async fn $delete_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, drug_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<axum::http::StatusCode> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_delete)?;
			$crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(drug_id, entity.$parent_field, id, $entity_name)?;
			$bmc::delete(&ctx, &mm, id).await?;
			Ok(axum::http::StatusCode::NO_CONTENT)
		}

		pub async fn $restore_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, drug_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_update)?;
			$crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(drug_id, entity.$parent_field, id, $entity_name)?;
			$bmc::restore(&ctx, &mm, id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
		}
	};
}

/// Generate CRUD REST handlers for a resource nested below a case patient.
#[macro_export]
#[doc(hidden)]
macro_rules! __patient_child_delete_response {
	(entity, $entity:expr) => {
		(
			axum::http::StatusCode::OK,
			axum::Json($crate::rest_result::DataRestResult { data: $entity }),
		)
	};
	(no_content, $entity:expr) => {{
		let _ = $entity;
		axum::http::StatusCode::NO_CONTENT
	}};
}

#[macro_export]
macro_rules! generate_patient_child_rest_fns {
	(
		Bmc: $bmc:ident,
		Entity: $entity:ty,
		ForCreate: $for_create:ty,
		ForUpdate: $for_update:ty,
		Filter: $filter:ty,
		CreateFn: $create_fn:ident,
		ListFn: $list_fn:ident,
		GetFn: $get_fn:ident,
		UpdateFn: $update_fn:ident,
		DeleteFn: $delete_fn:ident,
		RestoreFn: $restore_fn:ident,
		ParentField: $parent_field:ident,
		ResolveParentFn: $resolve_parent_fn:ident,
		ScopeFn: $scope_fn:ident,
		EntityName: $entity_name:literal,
		DeleteResult: $delete_result:ty,
		DeleteResponse: $delete_response:ident,
		PermCreate: $perm_create:path,
		PermList: $perm_list:path,
		PermRead: $perm_read:path,
		PermUpdate: $perm_update:path,
		PermDelete: $perm_delete:path
	) => {
		pub async fn $create_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
			axum::Json(params): axum::Json<
				$crate::rest_params::ParamsForCreate<$for_create>,
			>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_create)?;
			$crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let parent_id = $resolve_parent_fn(&ctx, &mm, case_id).await?;
			let $crate::rest_params::ParamsForCreate { data } = params;
			let mut data = data;
			data.$parent_field = parent_id;
			let id = $bmc::create(&ctx, &mm, data).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::CREATED,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
		}

		pub async fn $list_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<Vec<$entity>>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_list)?;
			let parent_id = $resolve_parent_fn(&ctx, &mm, case_id).await?;
			let mut filter: $filter = Default::default();
			filter.$parent_field = Some(modql::filter::OpValsValue::from(vec![
				modql::filter::OpValValue::Eq(serde_json::json!(
					parent_id.to_string()
				)),
			]));
			let entities = $bmc::list(
				&ctx,
				&mm,
				Some(vec![filter]),
				Some(modql::filter::ListOptions::default()),
			)
			.await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entities }),
			))
		}

		pub async fn $get_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_read)?;
			$crate::require_case_read_allowed(&ctx, &mm, case_id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(&ctx, &mm, case_id, entity.$parent_field, id, $entity_name)
				.await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
		}

		pub async fn $update_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
			axum::Json(params): axum::Json<
				$crate::rest_params::ParamsForUpdate<$for_update>,
			>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_update)?;
			$crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let $crate::rest_params::ParamsForUpdate { data } = params;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(&ctx, &mm, case_id, entity.$parent_field, id, $entity_name)
				.await?;
			$bmc::update(&ctx, &mm, id, data).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
		}

		pub async fn $delete_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<$delete_result> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_delete)?;
			$crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(&ctx, &mm, case_id, entity.$parent_field, id, $entity_name)
				.await?;
			$bmc::delete(&ctx, &mm, id).await?;
			Ok($crate::__patient_child_delete_response!(
				$delete_response,
				entity
			))
		}

		pub async fn $restore_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			axum::extract::Path((case_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::require_permission(&ctx, $perm_update)?;
			$crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(&ctx, &mm, case_id, entity.$parent_field, id, $entity_name)
				.await?;
			$bmc::restore(&ctx, &mm, id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
		}
	};
}

/// Generate CRUD REST handlers scoped to a case_id (nested resources).
#[macro_export]
macro_rules! generate_case_rest_fns {
    (
        Bmc: $bmc:ident,
        Entity: $entity:ty,
        ForCreate: $for_create:ty,
        ForUpdate: $for_update:ty,
        Suffix: $suffix:ident,
        PermCreate: $perm_create:path,
        PermRead: $perm_read:path,
        PermUpdate: $perm_update:path,
        PermDelete: $perm_delete:path,
        PermList: $perm_list:path
    ) => {
        paste! {
            pub async fn [<create_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(case_id): Path<Uuid>,
                Json(params): Json<ParamsForCreate<$for_create>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_create)?;
                $crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest create {} case_id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id
                );
                let ParamsForCreate { data } = params;
                let mut data = data;
                data.case_id = case_id;
                let id = $bmc::create(&ctx, &mm, data).await?;
                let entity = $bmc::get_in_case(&ctx, &mm, case_id, id).await?;
                Ok((axum::http::StatusCode::CREATED, Json(DataRestResult { data: entity })))
            }

            pub async fn [<get_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path((case_id, id)): Path<(Uuid, Uuid)>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_read)?;
                $crate::require_case_read_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest get {} case_id={} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id,
                    id
                );
                let entity = $bmc::get_in_case(&ctx, &mm, case_id, id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            pub async fn [<list_ $suffix s>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(case_id): Path<Uuid>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_list)?;
                $crate::require_case_read_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest list {}s case_id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id
                );
                let entities = $bmc::list_by_case(&ctx, &mm, case_id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entities })))
            }

            pub async fn [<update_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path((case_id, id)): Path<(Uuid, Uuid)>,
                Json(params): Json<ParamsForUpdate<$for_update>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_update)?;
                $crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest update {} case_id={} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id,
                    id
                );
                let ParamsForUpdate { data } = params;
                $bmc::update_in_case(&ctx, &mm, case_id, id, data).await?;
                let entity = $bmc::get_in_case(&ctx, &mm, case_id, id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            pub async fn [<delete_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path((case_id, id)): Path<(Uuid, Uuid)>,
            ) -> Result<axum::http::StatusCode> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_delete)?;
                $crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest delete {} case_id={} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id,
                    id
                );
                $bmc::delete_in_case(&ctx, &mm, case_id, id).await?;
                Ok(axum::http::StatusCode::NO_CONTENT)
            }
        }
    };
}

/// Generate CRUD REST handlers for a single resource per case (no list).
#[macro_export]
macro_rules! generate_case_single_rest_fns {
    (
        Bmc: $bmc:ident,
        Entity: $entity:ty,
        ForCreate: $for_create:ty,
        ForUpdate: $for_update:ty,
        Suffix: $suffix:ident,
        PermCreate: $perm_create:path,
        PermRead: $perm_read:path,
        PermUpdate: $perm_update:path,
        PermDelete: $perm_delete:path
    ) => {
        paste! {
            pub async fn [<create_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(case_id): Path<Uuid>,
                Json(params): Json<ParamsForCreate<$for_create>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_create)?;
                $crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest create {} case_id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id
                );
                let ParamsForCreate { data } = params;
                let mut data = data;
                data.case_id = case_id;
                let _id = $bmc::create(&ctx, &mm, data).await?;
                let entity = $bmc::get_by_case(&ctx, &mm, case_id).await?;
                Ok((axum::http::StatusCode::CREATED, Json(DataRestResult { data: entity })))
            }

            pub async fn [<get_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(case_id): Path<Uuid>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_read)?;
                $crate::require_case_read_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest get {} case_id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id
                );
                let entity = $bmc::get_by_case(&ctx, &mm, case_id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            pub async fn [<update_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(case_id): Path<Uuid>,
                Json(params): Json<ParamsForUpdate<$for_update>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_update)?;
                $crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest update {} case_id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id
                );
                let ParamsForUpdate { data } = params;
                $bmc::update_by_case(&ctx, &mm, case_id, data).await?;
                let entity = $bmc::get_by_case(&ctx, &mm, case_id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
            }

            pub async fn [<delete_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                Path(case_id): Path<Uuid>,
            ) -> Result<axum::http::StatusCode> {
                let ctx = ctx_w.0;
                $crate::require_permission(&ctx, $perm_delete)?;
                $crate::require_case_write_allowed(&ctx, &mm, case_id).await?;
                tracing::debug!(
                    "{:<12} - rest delete {} case_id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id
                );
                $bmc::delete_by_case(&ctx, &mm, case_id).await?;
                Ok(axum::http::StatusCode::NO_CONTENT)
            }
        }
    };
}
