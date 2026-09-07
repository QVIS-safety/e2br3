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
        EntityName: $entity_name:literal
    ) => {
		pub async fn $create_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
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
			$crate::with_authorized_case_child_mutation(
				&ctx, &snapshot, &mm, case_id,
				format!("{}:new:drug:{drug_id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
			let $crate::rest_params::ParamsForCreate { data } = params;
			let mut data = data;
			data.$parent_field = drug_id;
			let id = $bmc::create(&ctx, &mm, data).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::CREATED,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
				})
			).await
		}

		pub async fn $list_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, drug_id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<Vec<$entity>>>,
		)> {
			let ctx = ctx_w.0;
			$crate::with_authorized_case_child_read(
				&ctx, &snapshot, &mm, case_id,
				format!("{}:list:drug:{drug_id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
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
				})
			).await
		}

		pub async fn $get_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
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
			$crate::with_authorized_case_child_read(
				&ctx, &snapshot, &mm, case_id,
				format!("{}:{id}:drug:{drug_id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(drug_id, entity.$parent_field, id, $entity_name)?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
				})
			).await
		}

		pub async fn $update_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
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
			$crate::with_authorized_case_child_mutation(
				&ctx, &snapshot, &mm, case_id,
				format!("{}:{id}:drug:{drug_id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
			let $crate::rest_params::ParamsForUpdate { data } = params;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(drug_id, entity.$parent_field, id, $entity_name)?;
			$bmc::update(&ctx, &mm, id, data).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
				})
			).await
		}

		pub async fn $delete_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, drug_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<axum::http::StatusCode> {
			let ctx = ctx_w.0;
			$crate::with_authorized_case_child_mutation(
				&ctx, &snapshot, &mm, case_id,
				format!("{}:{id}:drug:{drug_id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(drug_id, entity.$parent_field, id, $entity_name)?;
			$bmc::delete(&ctx, &mm, id).await?;
			Ok(axum::http::StatusCode::NO_CONTENT)
				})
			).await
		}

		pub async fn $restore_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
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
			$crate::with_authorized_case_child_mutation(
				&ctx, &snapshot, &mm, case_id,
				format!("{}:{id}:drug:{drug_id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(drug_id, entity.$parent_field, id, $entity_name)?;
			$bmc::restore(&ctx, &mm, id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
				})
			).await
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
		DeleteResponse: $delete_response:ident
	) => {
		pub async fn $create_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
			axum::Json(params): axum::Json<
				$crate::rest_params::ParamsForCreate<$for_create>,
			>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::with_authorized_case_child_mutation(
				&ctx, &snapshot, &mm, case_id, format!("{}:new", $entity_name),
				move |ctx, mm| Box::pin(async move {
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
				})
			).await
		}

		pub async fn $list_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path(case_id): axum::extract::Path<uuid::Uuid>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<Vec<$entity>>>,
		)> {
			let ctx = ctx_w.0;
			$crate::with_authorized_case_child_read(
				&ctx, &snapshot, &mm, case_id, format!("{}:list", $entity_name),
				move |ctx, mm| Box::pin(async move {
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
				})
			).await
		}

		pub async fn $get_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::with_authorized_case_child_read(
				&ctx, &snapshot, &mm, case_id, format!("{}:{id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(&ctx, &mm, case_id, entity.$parent_field, id, $entity_name)
				.await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
				})
			).await
		}

		pub async fn $update_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
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
			$crate::with_authorized_case_child_mutation(
				&ctx, &snapshot, &mm, case_id, format!("{}:{id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
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
				})
			).await
		}

		pub async fn $delete_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<$delete_result> {
			let ctx = ctx_w.0;
			$crate::with_authorized_case_child_mutation(
				&ctx, &snapshot, &mm, case_id, format!("{}:{id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(&ctx, &mm, case_id, entity.$parent_field, id, $entity_name)
				.await?;
			$bmc::delete(&ctx, &mm, id).await?;
			Ok($crate::__patient_child_delete_response!(
				$delete_response,
				entity
			))
				})
			).await
		}

		pub async fn $restore_fn(
			axum::extract::State(mm): axum::extract::State<
				lib_core::model::ModelManager,
			>,
			ctx_w: lib_web::middleware::mw_auth::CtxW,
			snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
			axum::extract::Path((case_id, id)): axum::extract::Path<(
				uuid::Uuid,
				uuid::Uuid,
			)>,
		) -> $crate::Result<(
			axum::http::StatusCode,
			axum::Json<$crate::rest_result::DataRestResult<$entity>>,
		)> {
			let ctx = ctx_w.0;
			$crate::with_authorized_case_child_mutation(
				&ctx, &snapshot, &mm, case_id, format!("{}:{id}", $entity_name),
				move |ctx, mm| Box::pin(async move {
			let entity = $bmc::get(&ctx, &mm, id).await?;
			$scope_fn(&ctx, &mm, case_id, entity.$parent_field, id, $entity_name)
				.await?;
			$bmc::restore(&ctx, &mm, id).await?;
			let entity = $bmc::get(&ctx, &mm, id).await?;
			Ok((
				axum::http::StatusCode::OK,
				axum::Json($crate::rest_result::DataRestResult { data: entity }),
			))
				})
			).await
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
        Suffix: $suffix:ident
    ) => {
        paste! {
            pub async fn [<create_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
                Path(case_id): Path<Uuid>,
                Json(params): Json<ParamsForCreate<$for_create>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::with_authorized_case_child_mutation(
                    &ctx, &snapshot, &mm, case_id, concat!(stringify!($suffix), ":new"),
                    move |ctx, mm| Box::pin(async move {
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
                    })
                ).await
            }

            pub async fn [<get_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
                Path((case_id, id)): Path<(Uuid, Uuid)>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::with_authorized_case_child_read(
                    &ctx, &snapshot, &mm, case_id, format!("{}:{id}", stringify!($suffix)),
                    move |ctx, mm| Box::pin(async move {
                tracing::debug!(
                    "{:<12} - rest get {} case_id={} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id,
                    id
                );
                let entity = $bmc::get_in_case(&ctx, &mm, case_id, id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entity })))
                    })
                ).await
            }

            pub async fn [<list_ $suffix s>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
                Path(case_id): Path<Uuid>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
                let ctx = ctx_w.0;
                $crate::with_authorized_case_child_read(
                    &ctx, &snapshot, &mm, case_id, concat!(stringify!($suffix), ":list"),
                    move |ctx, mm| Box::pin(async move {
                tracing::debug!(
                    "{:<12} - rest list {}s case_id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id
                );
                let entities = $bmc::list_by_case(&ctx, &mm, case_id).await?;
                Ok((axum::http::StatusCode::OK, Json(DataRestResult { data: entities })))
                    })
                ).await
            }

            pub async fn [<update_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
                Path((case_id, id)): Path<(Uuid, Uuid)>,
                Json(params): Json<ParamsForUpdate<$for_update>>,
            ) -> Result<(axum::http::StatusCode, Json<DataRestResult<$entity>>)> {
                let ctx = ctx_w.0;
                $crate::with_authorized_case_child_mutation(
                    &ctx, &snapshot, &mm, case_id, format!("{}:{id}", stringify!($suffix)),
                    move |ctx, mm| Box::pin(async move {
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
                    })
                ).await
            }

            pub async fn [<delete_ $suffix>](
                State(mm): State<ModelManager>,
                ctx_w: lib_web::middleware::mw_auth::CtxW,
                snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
                Path((case_id, id)): Path<(Uuid, Uuid)>,
            ) -> Result<axum::http::StatusCode> {
                let ctx = ctx_w.0;
                $crate::with_authorized_case_child_mutation(
                    &ctx, &snapshot, &mm, case_id, format!("{}:{id}", stringify!($suffix)),
                    move |ctx, mm| Box::pin(async move {
                tracing::debug!(
                    "{:<12} - rest delete {} case_id={} id={}",
                    "HANDLER",
                    stringify!($suffix),
                    case_id,
                    id
                );
                $bmc::delete_in_case(&ctx, &mm, case_id, id).await?;
                Ok(axum::http::StatusCode::NO_CONTENT)
                    })
                ).await
            }
        }
    };
}
