//! Shared imports, scope guards, and parent-scope helpers
//! used across the presave section entity modules.

pub(super) use axum::extract::{Path, Query, State};
pub(super) use axum::http::StatusCode;
pub(super) use lib_core::authorization::EnforcedScopeFilter;
pub(super) use lib_core::model::authorization::PresaveAuthorizationKind;
pub(super) use lib_core::model::presave::{
	NarrativePresave, NarrativePresaveBmc, NarrativePresaveForCreate,
	NarrativePresaveForUpdate, ProductPresave, ProductPresaveActiveSubstance,
	ProductPresaveActiveSubstanceBmc, ProductPresaveActiveSubstanceForCreate,
	ProductPresaveActiveSubstanceForUpdate, ProductPresaveBmc,
	ProductPresaveForCreate, ProductPresaveForUpdate, ReceiverPresave,
	ReceiverPresaveBmc, ReceiverPresaveConsignee, ReceiverPresaveConsigneeBmc,
	ReceiverPresaveConsigneeForCreate, ReceiverPresaveConsigneeForUpdate,
	ReceiverPresaveForCreate, ReceiverPresaveForUpdate, ReceiverPresaveRouteBmc,
	ReceiverPresaveRouteForCreate, ReceiverPresaveRouteForUpdate, ReporterPresave,
	ReporterPresaveBmc, ReporterPresaveForCreate, ReporterPresaveForUpdate,
	SenderPresave, SenderPresaveBmc, SenderPresaveForCreate, SenderPresaveForUpdate,
	SenderPresaveGateway, SenderPresaveGatewayBmc, SenderPresaveGatewayForCreate,
	SenderPresaveGatewayForUpdate, SenderPresaveResponsiblePerson,
	SenderPresaveResponsiblePersonBmc, SenderPresaveResponsiblePersonForCreate,
	SenderPresaveResponsiblePersonForUpdate, StudyPresave, StudyPresaveBmc,
	StudyPresaveFdaCrossReportedIndNumberBmc,
	StudyPresaveFdaCrossReportedIndNumberForCreate,
	StudyPresaveFdaCrossReportedIndNumberForUpdate, StudyPresaveForCreate,
	StudyPresaveForUpdate, StudyPresaveProduct, StudyPresaveProductBmc,
	StudyPresaveProductForCreate, StudyPresaveProductForUpdate,
	StudyPresaveRegistrationNumber, StudyPresaveRegistrationNumberBmc,
	StudyPresaveRegistrationNumberForCreate,
	StudyPresaveRegistrationNumberForUpdate, StudyPresaveReporter,
	StudyPresaveReporterBmc, StudyPresaveReporterForCreate,
	StudyPresaveReporterForUpdate,
};
pub(super) use lib_core::model::presave_lifecycle::{
	PresaveKind, PresaveLifecycleService,
};
pub(super) use lib_core::model::{self, ModelManager};
pub(super) use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate};
pub(super) use lib_rest_core::rest_result::DataRestResult;
pub(super) use lib_rest_core::{
	with_authorized_presave_atomic_create, with_authorized_presave_atomic_update,
	with_authorized_presave_collection, with_authorized_presave_collection_action,
	with_authorized_presave_create, with_authorized_presave_read,
	with_authorized_presave_update, Error, Result,
};
pub(super) use lib_web::middleware::mw_auth::CtxW;
pub(super) use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
pub(super) use serde::{Deserialize, Serialize};
pub(super) use std::collections::HashSet;
pub(super) use uuid::Uuid;

pub struct Json<T>(pub T);

impl<S, T> axum::extract::FromRequest<S> for Json<T>
where
	S: Send + Sync,
	T: serde::de::DeserializeOwned,
{
	type Rejection = Error;

	async fn from_request(
		req: axum::extract::Request,
		state: &S,
	) -> core::result::Result<Self, Self::Rejection> {
		axum::Json::<T>::from_request(req, state)
			.await
			.map(|axum::Json(value)| Self(value))
			.map_err(|rejection| {
				let body = rejection.body_text();
				Error::ConstraintViolation(lib_rest_core::ConstraintViolation {
					rule_code: "INPUT.JSON.INVALID".into(),
					path: json_rejection_path(&body).into(),
					message: "request JSON does not match the expected input type"
						.into(),
				})
			})
	}
}

impl<T: Serialize> axum::response::IntoResponse for Json<T> {
	fn into_response(self) -> axum::response::Response {
		axum::Json(self.0).into_response()
	}
}

fn json_rejection_path(message: &str) -> &str {
	message
		.strip_prefix("Failed to deserialize the JSON body into the target type: ")
		.and_then(|detail| detail.split_once(':'))
		.map_or("$", |(path, _)| path)
}

pub(super) fn parse_scope_filter(
	raw: Option<&str>,
	field: &str,
) -> Result<Option<HashSet<Uuid>>> {
	let Some(raw) = raw else {
		return Ok(None);
	};
	let mut ids = HashSet::new();
	for value in raw
		.split(',')
		.map(str::trim)
		.filter(|value| !value.is_empty())
	{
		let id = Uuid::parse_str(value).map_err(|_| Error::BadRequest {
			message: format!("{field} accepts UUID values only"),
		})?;
		ids.insert(id);
	}
	Ok((!ids.is_empty()).then_some(ids))
}

macro_rules! generate_single_row_presave_rest_fns {
	(
		Bmc: $bmc:ident,
		Entity: $entity:ident,
		ForCreate: $for_create:ident,
		ForUpdate: $for_update:ident,
		Row: $row:ident,
		Details: $details:ident,
		Rows: $rows:ident,
		CreateRequest: $create_request:ident,
		CreateRows: $create_rows:ident,
		UpdateRequest: $update_request:ident,
		UpdateRows: $update_rows:ident,
		CreateFn: $create_fn:ident,
		ListFn: $list_fn:ident,
		GetFn: $get_fn:ident,
		UpdateFn: $update_fn:ident,
		DeleteFn: $delete_fn:ident,
		Kind: $kind:ident,
		ValidateCreate: $validate_create:path,
		ValidateUpdate: $validate_update:path
	) => {
		#[derive(Debug, Serialize)]
		pub struct $rows {
			pub $row: $entity,
		}
		#[derive(Debug, Serialize)]
		pub struct $details {
			pub rows: $rows,
		}
		#[derive(Deserialize)]
		#[serde(rename_all = "camelCase", deny_unknown_fields)]
		pub struct $create_request {
			pub rows: $create_rows,
		}
		#[derive(Deserialize)]
		#[serde(rename_all = "camelCase", deny_unknown_fields)]
		pub struct $create_rows {
			pub $row: $for_create,
		}
		#[derive(Deserialize)]
		#[serde(rename_all = "camelCase", deny_unknown_fields)]
		pub struct $update_request {
			pub rows: $update_rows,
		}
		#[derive(Deserialize)]
		#[serde(rename_all = "camelCase", deny_unknown_fields)]
		pub struct $update_rows {
			pub $row: $for_update,
		}

		fn details(entity: $entity) -> $details {
			$details {
				rows: $rows { $row: entity },
			}
		}

		pub async fn $create_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Json(params): Json<ParamsForCreate<$create_request>>,
		) -> Result<(StatusCode, Json<DataRestResult<$details>>)> {
			let ctx = ctx_w.0;
			with_authorized_presave_create(
				&ctx,
				&snapshot,
				&mm,
				stringify!($kind),
				move |ctx, mm| {
					Box::pin(async move {
						let data = params.data.rows.$row;
						$validate_create(&data)?;
						let id = $bmc::create(ctx, mm, data).await?;
						Ok(rest_created(details($bmc::get(ctx, mm, id).await?)))
					})
				},
			)
			.await
		}

		pub async fn $list_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
		) -> Result<(StatusCode, Json<DataRestResult<Vec<$details>>>)> {
			let ctx = ctx_w.0;
			with_authorized_presave_collection(
				&ctx,
				&snapshot,
				&mm,
				|ctx, mm, _scope| {
					Box::pin(async move {
						Ok(rest_ok(
							$bmc::list(ctx, mm, None)
								.await?
								.into_iter()
								.map(details)
								.collect(),
						))
					})
				},
			)
			.await
		}

		pub async fn $get_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path(id): Path<Uuid>,
		) -> Result<(StatusCode, Json<DataRestResult<$details>>)> {
			let ctx = ctx_w.0;
			with_authorized_presave_read(
				&ctx,
				&snapshot,
				&mm,
				PresaveAuthorizationKind::$kind,
				id,
				|ctx, mm| {
					Box::pin(async move {
						Ok(rest_ok(details($bmc::get(ctx, mm, id).await?)))
					})
				},
			)
			.await
		}

		pub async fn $update_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path(id): Path<Uuid>,
			Json(params): Json<ParamsForUpdate<$update_request>>,
		) -> Result<(StatusCode, Json<DataRestResult<$details>>)> {
			let ctx = ctx_w.0;
			with_authorized_presave_atomic_update(
				&ctx,
				&snapshot,
				&mm,
				PresaveAuthorizationKind::$kind,
				id,
				move |ctx, mm| {
					Box::pin(async move {
						let data = params.data.rows.$row;
						$validate_update(&data)?;
						if data.deleted == Some(true) {
							PresaveLifecycleService::archive(
								ctx,
								mm,
								PresaveKind::$kind,
								id,
							)
							.await?;
						} else {
							$bmc::update(ctx, mm, id, data).await?;
						}
						Ok(rest_ok(details($bmc::get(ctx, mm, id).await?)))
					})
				},
			)
			.await
		}

		pub async fn $delete_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path(id): Path<Uuid>,
		) -> Result<StatusCode> {
			let ctx = ctx_w.0;
			with_authorized_presave_update(
				&ctx,
				&snapshot,
				&mm,
				PresaveAuthorizationKind::$kind,
				id,
				|ctx, mm| {
					Box::pin(async move {
						PresaveLifecycleService::archive(
							ctx,
							mm,
							PresaveKind::$kind,
							id,
						)
						.await?;
						Ok(StatusCode::NO_CONTENT)
					})
				},
			)
			.await
		}
	};
}

pub(super) use generate_single_row_presave_rest_fns;

macro_rules! delete_presave_child {
	(hard, $bmc:ident, $for_update:ident, $ctx:ident, $mm:ident, $id:ident) => {
		$bmc::delete(&$ctx, &$mm, $id).await?;
	};
	(soft, $bmc:ident, $for_update:ident, $ctx:ident, $mm:ident, $id:ident) => {
		$bmc::update(
			&$ctx,
			&$mm,
			$id,
			$for_update {
				deleted: Some(true),
				..Default::default()
			},
		)
		.await?;
	};
}

pub(super) use delete_presave_child;

macro_rules! generate_presave_child_rest_fns {
	(
		Bmc: $bmc:ident,
		Entity: $entity:ident,
		RestCreate: $rest_create:ident,
		ForUpdate: $for_update:ident,
		CreateFn: $create_fn:ident,
		ListFn: $list_fn:ident,
		GetFn: $get_fn:ident,
		UpdateFn: $update_fn:ident,
		DeleteFn: $delete_fn:ident,
		ParentField: $parent_field:ident,
		ParentKind: $parent_kind:ident,
		EntityName: $entity_name:literal,
		DeleteMode: $delete_mode:ident,
		ValidateCreate: $validate_create:path,
		ValidateUpdate: $validate_update:path
	) => {
		pub async fn $create_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path(parent_id): Path<Uuid>,
			Json(params): Json<ParamsForCreate<$rest_create>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			with_authorized_presave_atomic_update(
				&ctx,
				&snapshot,
				&mm,
				PresaveAuthorizationKind::$parent_kind,
				parent_id,
				move |ctx, mm| {
					Box::pin(async move {
						let ParamsForCreate { data } = params;
						super::shared::ensure_presave_parent_active(
							ctx,
							mm,
							PresaveAuthorizationKind::$parent_kind,
							parent_id,
						)
						.await?;
						$validate_create(&data)?;
						let id =
							$bmc::create(ctx, mm, data.into_core(parent_id)).await?;
						Ok(rest_created($bmc::get(ctx, mm, id).await?))
					})
				},
			)
			.await
		}

		pub async fn $list_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path(parent_id): Path<Uuid>,
		) -> Result<(StatusCode, Json<DataRestResult<Vec<$entity>>>)> {
			let ctx = ctx_w.0;
			with_authorized_presave_read(
				&ctx,
				&snapshot,
				&mm,
				PresaveAuthorizationKind::$parent_kind,
				parent_id,
				|ctx, mm| {
					Box::pin(async move {
						Ok(rest_ok($bmc::list_by_parent(ctx, mm, parent_id).await?))
					})
				},
			)
			.await
		}

		pub async fn $get_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path((parent_id, id)): Path<(Uuid, Uuid)>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			with_authorized_presave_read(
				&ctx,
				&snapshot,
				&mm,
				PresaveAuthorizationKind::$parent_kind,
				parent_id,
				|ctx, mm| {
					Box::pin(async move {
						let entity = $bmc::get(ctx, mm, id).await?;
						ensure_parent_scope(
							parent_id,
							entity.$parent_field,
							id,
							$entity_name,
						)?;
						Ok(rest_ok(entity))
					})
				},
			)
			.await
		}

		pub async fn $update_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path((parent_id, id)): Path<(Uuid, Uuid)>,
			Json(params): Json<ParamsForUpdate<$for_update>>,
		) -> Result<(StatusCode, Json<DataRestResult<$entity>>)> {
			let ctx = ctx_w.0;
			with_authorized_presave_update(
				&ctx,
				&snapshot,
				&mm,
				PresaveAuthorizationKind::$parent_kind,
				parent_id,
				move |ctx, mm| {
					Box::pin(async move {
						let ParamsForUpdate { data } = params;
						let entity = $bmc::get(ctx, mm, id).await?;
						ensure_parent_scope(
							parent_id,
							entity.$parent_field,
							id,
							$entity_name,
						)?;
						$validate_update(&data)?;
						$bmc::update(ctx, mm, id, data).await?;
						Ok(rest_ok($bmc::get(ctx, mm, id).await?))
					})
				},
			)
			.await
		}

		pub async fn $delete_fn(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path((parent_id, id)): Path<(Uuid, Uuid)>,
		) -> Result<StatusCode> {
			let ctx = ctx_w.0;
			with_authorized_presave_update(
				&ctx,
				&snapshot,
				&mm,
				PresaveAuthorizationKind::$parent_kind,
				parent_id,
				|ctx, mm| {
					Box::pin(async move {
						let entity = $bmc::get(ctx, mm, id).await?;
						ensure_parent_scope(
							parent_id,
							entity.$parent_field,
							id,
							$entity_name,
						)?;
						delete_presave_child!(
							$delete_mode,
							$bmc,
							$for_update,
							ctx,
							mm,
							id
						);
						Ok(StatusCode::NO_CONTENT)
					})
				},
			)
			.await
		}
	};
}

pub(super) use generate_presave_child_rest_fns;

pub(super) async fn ensure_presave_parent_active(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	kind: PresaveAuthorizationKind,
	id: Uuid,
) -> Result<()> {
	let deleted = match kind {
		PresaveAuthorizationKind::Sender => {
			SenderPresaveBmc::get(ctx, mm, id).await?.deleted
		}
		PresaveAuthorizationKind::Receiver => {
			ReceiverPresaveBmc::get(ctx, mm, id).await?.deleted
		}
		PresaveAuthorizationKind::Product => {
			ProductPresaveBmc::get(ctx, mm, id).await?.deleted
		}
		PresaveAuthorizationKind::Reporter => {
			ReporterPresaveBmc::get(ctx, mm, id).await?.deleted
		}
		PresaveAuthorizationKind::Study => {
			StudyPresaveBmc::get(ctx, mm, id).await?.deleted
		}
		PresaveAuthorizationKind::Narrative => {
			NarrativePresaveBmc::get(ctx, mm, id).await?.deleted
		}
	};
	if deleted {
		return Err(lib_core::model::Error::Conflict {
			message: "archived presave cannot accept new child rows".to_string(),
		}
		.into());
	}
	Ok(())
}

pub(super) fn no_input_contract<T>(_: &T) -> Result<()> {
	Ok(())
}

pub(super) fn normalized_set(values: &[String]) -> HashSet<String> {
	values
		.iter()
		.map(|value| value.trim().to_ascii_lowercase())
		.filter(|value| !value.is_empty())
		.collect()
}

fn identifier_allowed(allowed: &[String], identifier: Uuid) -> bool {
	let allowed = normalized_set(allowed);
	if allowed.is_empty() {
		return true;
	}
	allowed.contains(&identifier.to_string().to_ascii_lowercase())
}

fn product_allowed_by_scope(
	sender_ids: &[String],
	product_ids: &[String],
	product_id: Uuid,
	product_sender_id: Option<Uuid>,
) -> bool {
	identifier_allowed(product_ids, product_id)
		&& (sender_ids.is_empty()
			|| product_sender_id
				.is_some_and(|sender_id| identifier_allowed(sender_ids, sender_id)))
}

pub(crate) fn product_presave_allowed(
	scope: &EnforcedScopeFilter,
	entity: &ProductPresave,
) -> bool {
	product_allowed_by_scope(
		scope.sender_ids(),
		scope.product_ids(),
		entity.id,
		entity.sender_presave_id,
	)
}

pub(super) fn can_manage_all_presaves(snapshot: &AuthorizationSnapshotW) -> bool {
	snapshot.identity().is_platform_administrator()
}

pub(super) fn rest_ok<T: Serialize>(
	data: T,
) -> (StatusCode, Json<DataRestResult<T>>) {
	(StatusCode::OK, Json(DataRestResult { data }))
}

pub(super) fn rest_created<T: Serialize>(
	data: T,
) -> (StatusCode, Json<DataRestResult<T>>) {
	(StatusCode::CREATED, Json(DataRestResult { data }))
}

pub(super) fn filter_sender_presaves_for_scope(
	scope: &EnforcedScopeFilter,
	entities: Vec<SenderPresave>,
	products: &[ProductPresave],
	studies: &[StudyPresave],
) -> Vec<SenderPresave> {
	entities
		.into_iter()
		.filter(|entity| {
			identifier_allowed(scope.sender_ids(), entity.id)
				&& (scope.product_ids().is_empty()
					|| products.iter().any(|product| {
						product.sender_presave_id == Some(entity.id)
							&& identifier_allowed(scope.product_ids(), product.id)
					})) && (scope.study_ids().is_empty()
				|| studies.iter().any(|study| {
					study.product_presave_id.is_some_and(|product_id| {
						products.iter().any(|product| {
							product.id == product_id
								&& product.sender_presave_id == Some(entity.id)
						})
					}) && identifier_allowed(scope.study_ids(), study.id)
				}))
		})
		.collect()
}

pub(super) fn filter_product_presaves_for_scope(
	scope: &EnforcedScopeFilter,
	entities: Vec<ProductPresave>,
	studies: &[StudyPresave],
) -> Vec<ProductPresave> {
	entities
		.into_iter()
		.filter(|entity| {
			product_presave_allowed(scope, entity)
				&& (scope.study_ids().is_empty()
					|| studies.iter().any(|study| {
						study.product_presave_id == Some(entity.id)
							&& identifier_allowed(scope.study_ids(), study.id)
					}))
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::{json_rejection_path, product_allowed_by_scope};
	use uuid::Uuid;

	#[test]
	fn product_scope_includes_linked_sender_scope() {
		let product_id = Uuid::new_v4();
		let allowed_sender_id = Uuid::new_v4();
		let blocked_sender_id = Uuid::new_v4();
		let sender_scope = vec![allowed_sender_id.to_string()];

		assert!(product_allowed_by_scope(
			&sender_scope,
			&[],
			product_id,
			Some(allowed_sender_id),
		));
		assert!(!product_allowed_by_scope(
			&sender_scope,
			&[],
			product_id,
			Some(blocked_sender_id),
		));
		assert!(!product_allowed_by_scope(
			&sender_scope,
			&[],
			product_id,
			None,
		));
		assert!(product_allowed_by_scope(&[], &[], product_id, None));
	}

	#[test]
	fn json_rejection_reports_the_input_path() {
		assert_eq!(
			json_rejection_path(
				"Failed to deserialize the JSON body into the target type: data.rows.product.flag: invalid type"
			),
			"data.rows.product.flag"
		);
		assert_eq!(json_rejection_path("expected JSON value"), "$");
	}
}

pub(super) fn filter_study_presaves_for_scope(
	scope: &EnforcedScopeFilter,
	entities: Vec<StudyPresave>,
	products: &[ProductPresave],
) -> Vec<StudyPresave> {
	entities
		.into_iter()
		.filter(|entity| {
			identifier_allowed(scope.study_ids(), entity.id)
				&& entity
					.product_presave_id
					.map(|product_id| {
						products
							.iter()
							.find(|product| product.id == product_id)
							.is_some_and(|product| {
								identifier_allowed(scope.product_ids(), product.id)
									&& (scope.sender_ids().is_empty()
										|| product.sender_presave_id.is_some_and(
											|sender_id| {
												identifier_allowed(
													scope.sender_ids(),
													sender_id,
												)
											},
										))
							})
					})
					.unwrap_or_else(|| {
						scope.product_ids().is_empty()
							&& scope.sender_ids().is_empty()
					})
		})
		.collect()
}

pub(super) fn ensure_parent_scope(
	path_parent_id: Uuid,
	entity_parent_id: Uuid,
	entity_id: Uuid,
	entity: &'static str,
) -> Result<()> {
	if path_parent_id != entity_parent_id {
		return Err(model::Error::EntityUuidNotFound {
			entity,
			id: entity_id,
		}
		.into());
	}
	Ok(())
}

pub(super) fn ensure_detail_parent_scope(
	path_parent_id: Uuid,
	entity_parent_id: Uuid,
	entity_id: Uuid,
	parent_name: &'static str,
	entity: &'static str,
) -> Result<()> {
	ensure_parent_scope(path_parent_id, entity_parent_id, entity_id, entity).map_err(
		|_| Error::BadRequest {
			message: format!(
				"{entity} child does not belong to {parent_name} {path_parent_id}"
			),
		},
	)
}
