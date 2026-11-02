use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use lib_core::authorization::{
	authorize_contextual_mutation, authorize_contextual_read,
	existing_organization_mutation_context, existing_organization_read_context,
	organization_collection_context, policy_registry, proposed_organization_context,
	Collection, Existing, OrganizationCreateProposal, OrganizationResource,
	Proposed,
};
use lib_core::model::organization::{
	Organization, OrganizationBmc, OrganizationFilter, OrganizationForCreate,
	OrganizationForUpdate,
};
use lib_core::model::ModelManager;
use lib_rest_core::rest_params::{ParamsForCreate, ParamsForUpdate, ParamsList};
use lib_rest_core::rest_result::DataRestResult;
use lib_rest_core::{
	authorization_denied, rls_ctx_for_authorized_mutation,
	rls_ctx_for_authorized_read, Error, Result,
};
use lib_web::middleware::mw_auth::CtxW;
use lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW;
use uuid::Uuid;

fn normalize_required_org_type(org_type: Option<String>) -> Result<String> {
	let org_type = org_type
		.as_deref()
		.and_then(OrganizationBmc::normalize_org_type)
		.ok_or_else(|| Error::BadRequest {
			message: "organization type must be CRO or Pharmaceutical company"
				.to_string(),
		})?;
	Ok(org_type.to_string())
}

fn normalize_optional_org_type(org_type: Option<String>) -> Result<Option<String>> {
	org_type
		.map(|value| {
			OrganizationBmc::normalize_org_type(&value)
				.map(str::to_string)
				.ok_or_else(|| Error::BadRequest {
					message:
						"organization type must be CRO or Pharmaceutical company"
							.to_string(),
				})
		})
		.transpose()
}

pub async fn create_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Json(params): Json<ParamsForCreate<OrganizationForCreate>>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	let action = policy_registry()
		.context_action::<Proposed<OrganizationCreateProposal>>(
			"organization.create",
		)
		.expect("organization.create policy");
	let permit = authorize_contextual_mutation(
		action,
		&snapshot,
		proposed_organization_context(),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_mutation(&ctx, &snapshot, &permit)?;
	let ParamsForCreate { mut data } = params;
	data.org_type = Some(normalize_required_org_type(data.org_type)?);
	let id = OrganizationBmc::create(&db_ctx, &mm, data).await?;
	let entity = OrganizationBmc::get(&db_ctx, &mm, id).await?;
	Ok((StatusCode::CREATED, Json(DataRestResult { data: entity })))
}

pub async fn get_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	let action = policy_registry()
		.context_action::<Existing<OrganizationResource>>("organization.read")
		.expect("organization.read policy");
	let permit = authorize_contextual_read(
		action,
		&snapshot,
		existing_organization_read_context(id),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_read(&ctx, &snapshot, &permit)?;
	let entity = OrganizationBmc::get(&db_ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn list_organizations(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Result<(StatusCode, Json<DataRestResult<Vec<Organization>>>)> {
	let ctx = ctx_w.0;
	let action = policy_registry()
		.context_action::<Collection<OrganizationResource>>("organization.list")
		.expect("organization.list policy");
	let permit = authorize_contextual_read(
		action,
		&snapshot,
		organization_collection_context(),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_read(&ctx, &snapshot, &permit)?;
	let params =
		ParamsList::<OrganizationFilter>::from_raw_query(raw_query.as_deref())
			.map_err(|message| Error::BadRequest { message })?;
	let entities =
		OrganizationBmc::list(&db_ctx, &mm, params.filters, params.list_options)
			.await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entities })))
}

pub async fn update_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
	Json(params): Json<ParamsForUpdate<OrganizationForUpdate>>,
) -> Result<(StatusCode, Json<DataRestResult<Organization>>)> {
	let ctx = ctx_w.0;
	let action = policy_registry()
		.context_action::<Existing<OrganizationResource>>("organization.update")
		.expect("organization.update policy");
	let permit = authorize_contextual_mutation(
		action,
		&snapshot,
		existing_organization_mutation_context(id),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_mutation(&ctx, &snapshot, &permit)?;
	let ParamsForUpdate { mut data } = params;
	data.org_type = normalize_optional_org_type(data.org_type)?;
	OrganizationBmc::update(&db_ctx, &mm, id, data).await?;
	let entity = OrganizationBmc::get(&db_ctx, &mm, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}

pub async fn delete_organization(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(id): Path<Uuid>,
) -> Result<StatusCode> {
	let ctx = ctx_w.0;
	let action = policy_registry()
		.context_action::<Existing<OrganizationResource>>("organization.delete")
		.expect("organization.delete policy");
	let permit = authorize_contextual_mutation(
		action,
		&snapshot,
		existing_organization_mutation_context(id),
	)
	.map_err(authorization_denied)?;
	let db_ctx = rls_ctx_for_authorized_mutation(&ctx, &snapshot, &permit)?;
	OrganizationBmc::deactivate(&db_ctx, &mm, id).await?;
	Ok(StatusCode::NO_CONTENT)
}
