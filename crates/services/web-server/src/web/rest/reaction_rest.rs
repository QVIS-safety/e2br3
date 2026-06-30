use lib_core::model::acs::{
	REACTION_CREATE, REACTION_DELETE, REACTION_LIST, REACTION_READ, REACTION_UPDATE,
};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate, ReactionForUpdate};
use lib_rest_core::prelude::*;

// Case-scoped CRUD functions:
// - create_reaction
// - get_reaction
// - list_reactions
// - update_reaction
// - delete_reaction
generate_case_rest_fns! {
	Bmc: ReactionBmc,
	Entity: lib_core::model::reaction::Reaction,
	ForCreate: ReactionForCreate,
	ForUpdate: ReactionForUpdate,
	Suffix: reaction,
	PermCreate: REACTION_CREATE,
	PermRead: REACTION_READ,
	PermUpdate: REACTION_UPDATE,
	PermDelete: REACTION_DELETE,
	PermList: REACTION_LIST
}

pub async fn restore_reaction(
	State(mm): State<ModelManager>,
	ctx_w: lib_web::middleware::mw_auth::CtxW,
	Path((case_id, id)): Path<(Uuid, Uuid)>,
) -> Result<(
	StatusCode,
	Json<DataRestResult<lib_core::model::reaction::Reaction>>,
)> {
	let ctx = ctx_w.0;
	require_permission(&ctx, REACTION_UPDATE)?;
	require_case_write_allowed(&ctx, &mm, case_id).await?;
	ReactionBmc::get_in_case_with_deleted(&ctx, &mm, case_id, id, true).await?;
	ReactionBmc::restore_in_case(&ctx, &mm, case_id, id).await?;
	let entity = ReactionBmc::get_in_case(&ctx, &mm, case_id, id).await?;
	Ok((StatusCode::OK, Json(DataRestResult { data: entity })))
}
