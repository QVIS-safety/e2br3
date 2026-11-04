//! Case editor REST handlers, split by editor surface.
//!
//! Shared DTO/import helpers live in `common`; handler modules are re-exported
//! so existing router paths (`case_editor_rest::<handler>`) stay unchanged.

mod common;
mod handler_macros;

mod ae;
mod dg;
mod dh;
mod direct;
mod input_contract_save;
mod lb;
mod lr;
mod shell;

pub use ae::*;
pub use dg::*;
pub use dh::*;
pub use direct::*;
pub(crate) use input_contract_save::validate_row_payload;
pub use lb::*;
pub use lr::*;
pub use shell::*;

use crate::web::rest::case_editor_dto::{
	CaseEditorPagePatchRequest, CaseEditorPageRowBatchOperation,
	CaseEditorPageRowBatchRequest, CaseEditorPageRowBatchResponse,
	CaseEditorPageRowBatchResult, CaseFollowUpPagePatchRequest,
};
use axum::{
	extract::{Path, State},
	http::StatusCode,
	Json,
};
use lib_core::{ctx::Ctx, model::ModelManager};
use lib_rest_core::{Error, Result};
use lib_web::middleware::{
	mw_auth::CtxW, mw_authorization_snapshot::AuthorizationSnapshotW,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

macro_rules! repeatable_page_batch_handler {
	($fn_name:ident, $page_id:literal) => {
		pub async fn $fn_name(
			State(mm): State<ModelManager>,
			ctx_w: CtxW,
			snapshot: AuthorizationSnapshotW,
			Path(case_id): Path<Uuid>,
			Json(request): Json<CaseEditorPageRowBatchRequest>,
		) -> Result<(StatusCode, Json<CaseEditorPageRowBatchResponse>)> {
			let ctx = ctx_w.0;
			lib_rest_core::with_authorized_case_child_mutation(
				&ctx,
				&snapshot,
				&mm,
				case_id,
				concat!("editor/", $page_id, "/rows"),
				move |ctx, mm| {
					Box::pin(apply_editor_page_row_batch(
						ctx, mm, case_id, $page_id, request,
					))
				},
			)
			.await
		}
	};
}

repeatable_page_batch_handler!(patch_editor_lr_page_rows, "LR");
repeatable_page_batch_handler!(patch_editor_dh_page_rows, "DH");
repeatable_page_batch_handler!(patch_editor_ae_page_rows, "AE");
repeatable_page_batch_handler!(patch_editor_lb_page_rows, "LB");
repeatable_page_batch_handler!(patch_editor_dg_page_rows, "DG");

async fn apply_editor_page_row_batch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &'static str,
	request: CaseEditorPageRowBatchRequest,
) -> Result<(StatusCode, Json<CaseEditorPageRowBatchResponse>)> {
	if request.operations.is_empty() {
		return Err(Error::BadRequest {
			message: "repeatable page batch requires at least one operation"
				.to_string(),
		});
	}
	let requested_authorities =
		common::validate_request_projection_context(request.authorities.as_deref())?;
	let mut seen_row_ids = HashSet::new();
	for operation in &request.operations {
		let row_id = match operation {
			CaseEditorPageRowBatchOperation::Update { row_id, .. }
			| CaseEditorPageRowBatchOperation::Delete { row_id, .. } => Some(*row_id),
			CaseEditorPageRowBatchOperation::Create { .. } => None,
		};
		if let Some(row_id) = row_id {
			if !seen_row_ids.insert(row_id) {
				return Err(Error::BadRequest {
					message: format!("duplicate repeatable page row ID '{row_id}'"),
				});
			}
		}
	}

	let mut results = Vec::with_capacity(request.operations.len());
	for operation in request.operations {
		let (operation_name, client_row_index, row_id) = match operation {
			CaseEditorPageRowBatchOperation::Create {
				client_row_index,
				rows,
			} => {
				let patch = CaseEditorPagePatchRequest {
					authorities: request.authorities.clone(),
					rows,
				};
				let row_id =
					apply_editor_page_row_create(ctx, mm, case_id, page_id, &patch)
						.await?;
				("create", client_row_index, row_id)
			}
			CaseEditorPageRowBatchOperation::Update {
				row_id,
				client_row_index,
				rows,
			} => {
				let patch = CaseEditorPagePatchRequest {
					authorities: request.authorities.clone(),
					rows,
				};
				apply_editor_page_row_patch(
					ctx, mm, case_id, page_id, row_id, &patch,
				)
				.await?;
				("update", client_row_index, row_id)
			}
			CaseEditorPageRowBatchOperation::Delete {
				row_id,
				client_row_index,
			} => {
				apply_editor_page_row_delete(ctx, mm, case_id, page_id, row_id)
					.await?;
				("delete", client_row_index, row_id)
			}
		};
		results.push(CaseEditorPageRowBatchResult {
			operation: operation_name,
			client_row_index,
			row_id,
		});
	}
	common::mark_editor_validation_summary_stale(
		ctx,
		mm,
		case_id,
		requested_authorities,
	)
	.await?;
	Ok((
		StatusCode::OK,
		Json(CaseEditorPageRowBatchResponse {
			case_id,
			page_id,
			results,
		}),
	))
}

async fn apply_editor_page_row_create(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &str,
	request: &CaseEditorPagePatchRequest,
) -> Result<Uuid> {
	let (row_id, _) = match page_id {
		"LR" => {
			lr::apply_editor_lr_page_row_create(ctx, mm, case_id, request).await?
		}
		"DH" => {
			dh::apply_editor_dh_page_row_create(ctx, mm, case_id, request).await?
		}
		"AE" => {
			ae::apply_editor_ae_page_row_create(ctx, mm, case_id, request).await?
		}
		"LB" => {
			lb::apply_editor_lb_page_row_create(ctx, mm, case_id, request).await?
		}
		"DG" => {
			dg::apply_editor_dg_page_row_create(ctx, mm, case_id, request).await?
		}
		_ => {
			return Err(Error::BadRequest {
				message: format!("unsupported repeatable page '{page_id}'"),
			})
		}
	};
	Ok(row_id)
}

async fn apply_editor_page_row_patch(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &str,
	row_id: Uuid,
	request: &CaseEditorPagePatchRequest,
) -> Result<()> {
	match page_id {
		"LR" => {
			lr::apply_editor_lr_page_row_patch(ctx, mm, case_id, row_id, request)
				.await?
		}
		"DH" => {
			dh::apply_editor_dh_page_row_patch(ctx, mm, case_id, row_id, request)
				.await?
		}
		"AE" => {
			ae::apply_editor_ae_page_row_patch(ctx, mm, case_id, row_id, request)
				.await?
		}
		"LB" => {
			lb::apply_editor_lb_page_row_patch(ctx, mm, case_id, row_id, request)
				.await?
		}
		"DG" => {
			dg::apply_editor_dg_page_row_patch(ctx, mm, case_id, row_id, request)
				.await?
		}
		_ => {
			return Err(Error::BadRequest {
				message: format!("unsupported repeatable page '{page_id}'"),
			})
		}
	};
	Ok(())
}

async fn apply_editor_page_row_delete(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	page_id: &str,
	row_id: Uuid,
) -> Result<()> {
	match page_id {
		"LR" => {
			lr::apply_editor_lr_page_row_delete(ctx, mm, case_id, row_id).await?
		}
		"DH" => {
			dh::apply_editor_dh_page_row_delete(ctx, mm, case_id, row_id).await?
		}
		"AE" => {
			ae::apply_editor_ae_page_row_delete(ctx, mm, case_id, row_id).await?
		}
		"LB" => {
			lb::apply_editor_lb_page_row_delete(ctx, mm, case_id, row_id).await?
		}
		"DG" => {
			dg::apply_editor_dg_page_row_delete(ctx, mm, case_id, row_id).await?
		}
		_ => {
			return Err(Error::BadRequest {
				message: format!("unsupported repeatable page '{page_id}'"),
			})
		}
	}
	Ok(())
}

pub(crate) async fn apply_follow_up_page_patches(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	mut pages: CaseFollowUpPagePatchRequest,
) -> Result<()> {
	let authorities = pages.ci.authorities.clone();
	for request in [&pages.rp, &pages.sd, &pages.si, &pages.dm, &pages.nr]
		.into_iter()
		.chain(
			pages
				.lr
				.iter()
				.chain(&pages.dh)
				.chain(&pages.ae)
				.chain(&pages.lb)
				.chain(&pages.dg)
				.map(|row| &row.request),
		) {
		if request.authorities != authorities {
			return Err(Error::BadRequest {
				message: "follow-up page authorities must match".to_string(),
			});
		}
	}

	let requested_authorities =
		direct::apply_editor_direct_page_patch(ctx, mm, case_id, "CI", pages.ci)
			.await?;
	for (page_id, request) in [
		("RP", pages.rp),
		("SD", pages.sd),
		("SI", pages.si),
		("DM", pages.dm),
		("NR", pages.nr),
	] {
		direct::apply_editor_direct_page_patch(ctx, mm, case_id, page_id, request)
			.await?;
	}
	for row in &pages.lr {
		lr::apply_editor_lr_page_row_create(ctx, mm, case_id, &row.request).await?;
	}
	for row in &pages.dh {
		dh::apply_editor_dh_page_row_create(ctx, mm, case_id, &row.request).await?;
	}

	let mut reaction_ids = HashMap::new();
	for row in &pages.ae {
		let client_row_id = row
			.client_row_id
			.as_deref()
			.map(str::trim)
			.filter(|value| value.starts_with("index-"))
			.ok_or_else(|| Error::BadRequest {
				message: "follow-up AE row clientRowId must start with 'index-'"
					.to_string(),
			})?;
		let (row_id, _) =
			ae::apply_editor_ae_page_row_create(ctx, mm, case_id, &row.request)
				.await?;
		if reaction_ids
			.insert(client_row_id.to_string(), row_id)
			.is_some()
		{
			return Err(Error::BadRequest {
				message: format!(
					"duplicate follow-up AE clientRowId '{client_row_id}'"
				),
			});
		}
	}
	for row in &pages.lb {
		lb::apply_editor_lb_page_row_create(ctx, mm, case_id, &row.request).await?;
	}
	for row in &mut pages.dg {
		remap_follow_up_dg_reaction_ids(&mut row.request, &reaction_ids)?;
		dg::apply_editor_dg_page_row_create(ctx, mm, case_id, &row.request).await?;
	}

	common::mark_editor_validation_summary_stale(
		ctx,
		mm,
		case_id,
		requested_authorities,
	)
	.await
}

fn remap_follow_up_dg_reaction_ids(
	request: &mut CaseEditorPagePatchRequest,
	reaction_ids: &HashMap<String, Uuid>,
) -> Result<()> {
	let Some(assessments) = request
		.rows
		.get_mut("drug")
		.and_then(serde_json::Value::as_object_mut)
		.and_then(|drug| drug.get_mut("drugReactionAssessments"))
		.and_then(serde_json::Value::as_array_mut)
	else {
		return Ok(());
	};
	for (index, assessment) in assessments.iter_mut().enumerate() {
		let Some(reaction_id) = assessment
			.as_object_mut()
			.and_then(|row| row.get_mut("reactionId"))
		else {
			continue;
		};
		let Some(client_row_id) = reaction_id.as_str() else {
			continue;
		};
		if !client_row_id.starts_with("index-") {
			continue;
		}
		let persisted_id =
			reaction_ids
				.get(client_row_id)
				.ok_or_else(|| Error::BadRequest {
					message: format!(
						"missing follow-up AE row '{client_row_id}' for DG assessment {index}"
					),
				})?;
		*reaction_id = serde_json::Value::String(persisted_id.to_string());
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn remaps_follow_up_dg_reaction_reference_exactly() {
		let persisted_id = Uuid::new_v4();
		let mut request = serde_json::from_value(json!({
			"authorities": ["mfds"],
			"rows": { "drug": { "drugReactionAssessments": [
				{ "reactionId": "index-0" }
			] } }
		}))
		.unwrap();

		remap_follow_up_dg_reaction_ids(
			&mut request,
			&HashMap::from([("index-0".to_string(), persisted_id)]),
		)
		.unwrap();

		assert_eq!(
			request.rows["drug"]["drugReactionAssessments"][0]["reactionId"],
			persisted_id.to_string()
		);
	}
}
