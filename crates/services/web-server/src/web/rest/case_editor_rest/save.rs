use super::*;
use crate::web::rest::case_editor_dto::{
	CaseEditorPageSave, CaseEditorPagesSaveRequest, CaseEditorPagesSaveResponse,
};

impl CaseEditorPageSave {
	fn page_id(&self) -> &'static str {
		match self {
			Self::CI(_) => "CI",
			Self::RP(_) => "RP",
			Self::SD(_) => "SD",
			Self::SI(_) => "SI",
			Self::DM(_) => "DM",
			Self::NR(_) => "NR",
			Self::LR(_) => "LR",
			Self::DH(_) => "DH",
			Self::AE(_) => "AE",
			Self::LB(_) => "LB",
			Self::DG(_) => "DG",
		}
	}
	fn authorities(&self) -> &Option<Vec<String>> {
		match self {
			Self::CI(r)
			| Self::RP(r)
			| Self::SD(r)
			| Self::SI(r)
			| Self::DM(r)
			| Self::NR(r) => &r.authorities,
			Self::LR(r) | Self::DH(r) | Self::AE(r) | Self::LB(r) | Self::DG(r) => {
				&r.authorities
			}
		}
	}
}

pub async fn save_editor_pages(
	State(mm): State<ModelManager>,
	ctx_w: CtxW,
	snapshot: AuthorizationSnapshotW,
	Path(case_id): Path<Uuid>,
	Json(request): Json<CaseEditorPagesSaveRequest>,
) -> Result<(StatusCode, Json<CaseEditorPagesSaveResponse>)> {
	lib_rest_core::with_authorized_case_child_mutation(
		&ctx_w.0,
		&snapshot,
		&mm,
		case_id,
		"editor/pages",
		move |ctx, mm| Box::pin(apply_editor_pages_save(ctx, mm, case_id, request)),
	)
	.await
}

async fn apply_editor_pages_save(
	ctx: &Ctx,
	mm: &ModelManager,
	case_id: Uuid,
	mut request: CaseEditorPagesSaveRequest,
) -> Result<(StatusCode, Json<CaseEditorPagesSaveResponse>)> {
	let authorities = request
		.pages
		.first()
		.ok_or_else(|| Error::BadRequest {
			message: "editor save requires at least one page".to_string(),
		})?
		.authorities()
		.clone();
	match authorities.as_deref() {
		Some(values) if !values.is_empty() => {}
		_ => {
			return Err(Error::BadRequest {
				message: "editor save requires explicit authorities".to_string(),
			})
		}
	}
	let mut seen_pages = HashSet::new();
	for page in &request.pages {
		if !seen_pages.insert(page.page_id()) || page.authorities() != &authorities {
			return Err(Error::BadRequest {
				message:
					"editor save requires unique pages with matching authorities"
						.to_string(),
			});
		}
	}
	// Parents/new reactions must exist before dependent history/drug rows, within the same transaction.
	const ORDER: [&str; 11] = [
		"CI", "RP", "SD", "SI", "DM", "NR", "LR", "DH", "AE", "LB", "DG",
	];
	request
		.pages
		.sort_by_key(|page| ORDER.iter().position(|id| *id == page.page_id()));
	let mut pages = Vec::new();
	let mut reaction_ids = HashMap::new();
	for page in request.pages {
		let page_id = page.page_id();
		let result = match page {
			CaseEditorPageSave::CI(patch)
			| CaseEditorPageSave::RP(patch)
			| CaseEditorPageSave::SD(patch)
			| CaseEditorPageSave::SI(patch)
			| CaseEditorPageSave::DM(patch)
			| CaseEditorPageSave::NR(patch) => {
				if patch.rows.is_empty() {
					return Err(Error::BadRequest {
						message: format!("editor {page_id} save requires rows"),
					});
				}
				let requested = direct::apply_editor_direct_page_patch(
					ctx, mm, case_id, page_id, patch,
				)
				.await?;
				common::mark_editor_validation_summary_stale(
					ctx, mm, case_id, requested,
				)
				.await?;
				CaseEditorPageRowBatchResponse {
					case_id,
					page_id,
					results: Vec::new(),
				}
			}
			CaseEditorPageSave::LR(mut batch)
			| CaseEditorPageSave::DH(mut batch)
			| CaseEditorPageSave::AE(mut batch)
			| CaseEditorPageSave::LB(mut batch)
			| CaseEditorPageSave::DG(mut batch) => {
				let mut indexes = HashSet::new();
				let mut tokens = HashSet::new();
				let mut creates = HashMap::new();
				for operation in &mut batch.operations {
					let (index, rows) = match operation {
						CaseEditorPageRowBatchOperation::Create {
							client_row_index,
							client_row_id,
							rows,
						} => {
							let token = client_row_id
								.as_ref()
								.filter(|id| id.starts_with("index-"))
								.ok_or_else(|| {
									Error::BadRequest {
								message: format!("editor {page_id} create requires a temporary clientRowId"),
							}
								})?;
							if !tokens.insert(token.clone()) {
								return Err(Error::BadRequest {
									message: format!(
										"duplicate editor {page_id} clientRowId"
									),
								});
							}
							creates.insert(*client_row_index, token.clone());
							(*client_row_index, Some(rows))
						}
						CaseEditorPageRowBatchOperation::Update {
							client_row_index,
							rows,
							..
						} => (*client_row_index, Some(rows)),
						CaseEditorPageRowBatchOperation::Delete {
							client_row_index,
							..
						} => (*client_row_index, None),
					};
					if !indexes.insert(index) {
						return Err(Error::BadRequest {
							message: format!(
								"duplicate editor {page_id} clientRowIndex"
							),
						});
					}
					if page_id == "DG" {
						if let Some(rows) = rows {
							let mut patch = CaseEditorPagePatchRequest {
								authorities: batch.authorities.clone(),
								rows: std::mem::take(rows),
							};
							remap_follow_up_dg_reaction_ids(
								&mut patch,
								&reaction_ids,
							)?;
							*rows = patch.rows;
						}
					}
				}
				let collection = match page_id {
					"LR" => "literatureReferences",
					"DH" => "patientInformation.pastDrugHistory",
					"AE" => "reactions",
					"LB" => "testResults",
					"DG" => "drugs",
					_ => {
						return Err(Error::BadRequest {
							message: format!(
								"unsupported repeatable page {page_id}"
							),
						})
					}
				};
				let (_, Json(result)) = apply_editor_page_row_batch(
					ctx, mm, case_id, page_id, collection, batch,
				)
				.await?;
				if page_id == "AE" {
					for row in &result.results {
						if let Some(token) = creates.get(&row.client_row_index) {
							reaction_ids.insert(token.clone(), row.row_id);
						}
					}
				}
				result
			}
		};
		pages.push(result);
	}
	Ok((
		StatusCode::OK,
		Json(CaseEditorPagesSaveResponse { case_id, pages }),
	))
}
