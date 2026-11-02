use super::common::{
	bool_field, direct_page_projection_response,
	mark_editor_validation_summary_stale, validate_direct_rows,
	validate_request_projection_context, BTreeMap, CaseEditorPagePatchRequest,
	CaseEditorPageProjectionResponse, CtxW, Error, Json, ModelManager, Path,
	PatientIdentifierBmc, PatientInformationBmc, Result, State, Uuid, Value,
};

mod ci;
mod dm;
mod nr;
mod rp;
mod sd;
mod si;

pub use ci::{
	get_editor_ci, get_editor_ci_page_projection, patch_editor_ci_page_projection,
};
pub use dm::{get_editor_dm, get_editor_dm_page_projection};
pub use nr::{get_editor_nr, get_editor_nr_page_projection};
pub use rp::{get_editor_rp, get_editor_rp_page_projection};
pub use sd::{get_editor_sd, get_editor_sd_page_projection};
pub use si::{get_editor_si, get_editor_si_page_projection};

use ci::apply_ci_rows_patch;
use dm::{apply_dm_page_rows_patch, load_editor_dm_data};
use nr::{apply_nr_page_rows_patch, load_editor_nr_data};
use rp::{apply_rp_page_rows_patch, load_editor_rp_data};
use sd::{apply_sd_page_rows_patch, load_editor_sd_data};
use si::{apply_si_page_rows_patch, load_editor_si_data};

pub async fn patch_editor_rp_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, snapshot, case_id, "RP", request).await
}

pub async fn patch_editor_sd_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, snapshot, case_id, "SD", request).await
}

pub async fn patch_editor_si_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, snapshot, case_id, "SI", request).await
}

pub async fn patch_editor_dm_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, snapshot, case_id, "DM", request).await
}

pub async fn patch_editor_nr_page_projection(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagePatchRequest>,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	patch_direct_page_projection(mm, ctx_w, snapshot, case_id, "NR", request).await
}

async fn patch_direct_page_projection(
	mm: ModelManager,
	ctx_w: CtxW,
	snapshot: lib_web::middleware::mw_authorization_snapshot::AuthorizationSnapshotW,
	case_id: Uuid,
	page_id: &'static str,
	request: CaseEditorPagePatchRequest,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let ctx = ctx_w.0;
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx,
		&snapshot,
		&mm,
		case_id,
		format!("editor/{page_id}"),
		move |ctx, mm| {
			Box::pin(patch_direct_page_projection_authorized(
				ctx, mm, case_id, page_id, request,
			))
		},
	)
	.await
}

async fn patch_direct_page_projection_authorized(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	request: CaseEditorPagePatchRequest,
) -> Result<(
	axum::http::StatusCode,
	Json<CaseEditorPageProjectionResponse>,
)> {
	let has_rows = !request.rows.is_empty();
	let requested_authorities =
		apply_editor_direct_page_patch(ctx, mm, case_id, page_id, request).await?;
	if has_rows {
		mark_editor_validation_summary_stale(
			ctx,
			mm,
			case_id,
			requested_authorities.clone(),
		)
		.await?;
	}

	let data = match page_id {
		"RP" => load_editor_rp_data(ctx, mm, case_id).await?,
		"SD" => load_editor_sd_data(ctx, mm, case_id).await?,
		"SI" => load_editor_si_data(ctx, mm, case_id).await?,
		"DM" => load_editor_dm_data(ctx, mm, case_id).await?,
		"NR" => load_editor_nr_data(ctx, mm, case_id).await?,
		_ => {
			return Err(Error::BadRequest {
				message: format!("unsupported direct page '{page_id}'"),
			})
		}
	};
	let projection = direct_page_projection_response(
		ctx,
		mm,
		case_id,
		page_id,
		requested_authorities,
		data,
	)
	.await?;
	Ok((axum::http::StatusCode::OK, Json(projection)))
}

pub(crate) async fn apply_editor_direct_page_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	mut request: CaseEditorPagePatchRequest,
) -> Result<Option<String>> {
	let requested_authorities =
		validate_request_projection_context(request.authorities.as_deref())?;
	let fda = request
		.authorities
		.as_deref()
		.unwrap_or_default()
		.iter()
		.any(|authority| authority.eq_ignore_ascii_case("fda"));
	if page_id == "SI"
		&& !request
			.authorities
			.as_deref()
			.unwrap_or_default()
			.iter()
			.any(|authority| authority.eq_ignore_ascii_case("mfds"))
	{
		if let Some(study) = request
			.rows
			.get_mut("studyInformation")
			.and_then(Value::as_object_mut)
		{
			study.remove("studyTypeReactionKr1");
			study.remove("study_type_reaction_kr1");
		}
	}
	resolve_dm_identifier_types(ctx, mm, case_id, page_id, &mut request.rows)
		.await?;
	validate_direct_rows(page_id, &request.rows, fda)?;
	if !request.rows.is_empty() {
		if page_id == "CI" {
			apply_ci_rows_patch(ctx, mm, case_id, &request.rows).await?;
		} else {
			apply_direct_page_rows_patch(ctx, mm, case_id, page_id, &request.rows)
				.await?;
		}
	}
	Ok(requested_authorities)
}

async fn resolve_dm_identifier_types(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &str,
	rows: &mut BTreeMap<String, Value>,
) -> Result<()> {
	if page_id != "DM" {
		return Ok(());
	}
	let Some(identifiers) = rows
		.get_mut("patientIdentifiers")
		.and_then(Value::as_array_mut)
	else {
		return Ok(());
	};
	for value in identifiers {
		let identifier = value.as_object_mut().ok_or_else(|| Error::BadRequest {
			message: format!("{page_id}.patientIdentifiers must contain objects"),
		})?;
		if !identifier.contains_key("deleted") {
			if let Some(deleted) = identifier.get("_delete").and_then(Value::as_bool)
			{
				identifier.insert("deleted".into(), Value::Bool(deleted));
			}
		}
		let deleted = bool_field(identifier, &["deleted", "_delete"]) == Some(true);
		let id = super::common::uuid_field(identifier, &["id"])?;
		if deleted && id.is_none() {
			continue;
		}
		let supplied_type = identifier
			.get("identifierTypeCode")
			.or_else(|| identifier.get("identifier_type_code"))
			.cloned();
		if !deleted {
			if let Some(value) =
				supplied_type.as_ref().filter(|value| !value.is_null())
			{
				if !matches!(value.as_str(), Some("1" | "2" | "3" | "4")) {
					return Err(Error::BadRequest {
						message: format!(
							"{page_id}.patientIdentifiers.identifierTypeCode must be one of 1, 2, 3, or 4"
						),
					});
				}
				identifier.insert("identifierTypeCode".into(), value.clone());
			}
		}
		let needs_type =
			!deleted && supplied_type.as_ref().is_none_or(Value::is_null);
		let Some(id) = id else {
			if needs_type {
				return Err(Error::BadRequest {
					message: format!(
						"{page_id}.patientIdentifiers.identifierTypeCode is required"
					),
				});
			}
			continue;
		};
		let current = PatientIdentifierBmc::get(ctx, mm, id).await?;
		let patient =
			PatientInformationBmc::get(ctx, mm, current.patient_id).await?;
		if patient.case_id != case_id {
			return Err(lib_core::model::Error::EntityUuidNotFound {
				entity: "patient_identifiers",
				id,
			}
			.into());
		}
		if needs_type {
			identifier.insert(
				"identifierTypeCode".into(),
				Value::String(current.identifier_type_code),
			);
		}
	}
	Ok(())
}

async fn apply_direct_page_rows_patch(
	ctx: &lib_core::ctx::Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	rows: &BTreeMap<String, Value>,
) -> Result<()> {
	match page_id {
		"RP" => apply_rp_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"SD" => apply_sd_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"SI" => apply_si_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"DM" => apply_dm_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		"NR" => apply_nr_page_rows_patch(ctx, mm, case_id, page_id, rows).await,
		_ => Err(Error::BadRequest {
			message: format!("unsupported direct page '{page_id}'"),
		}),
	}
}
