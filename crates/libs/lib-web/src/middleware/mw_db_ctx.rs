use crate::error::{Error, Result};
use crate::middleware::mw_auth::CtxW;
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use lib_core::model::store::set_full_context_from_ctx_dbx;
use lib_core::model::{self, ModelManager};
use lib_rest_core as rest;
use std::sync::Arc;

pub async fn mw_ctx_require_and_set_dbx(
	State(mm): State<ModelManager>,
	ctx: Result<CtxW>,
	req: Request<Body>,
	next: Next,
) -> Result<Response> {
	let ctx = ctx?;
	let dbx = mm.dbx();

	dbx.begin_txn()
		.await
		.map_err(model::Error::from)
		.map_err(Error::from)?;

	if let Err(err) = set_full_context_from_ctx_dbx(dbx, &ctx.0).await {
		let _ = dbx.rollback_txn().await;
		return Err(Error::from(err));
	}

	let res = next.run(req).await;

	let has_error = res.extensions().get::<Arc<Error>>().is_some()
		|| res.extensions().get::<Arc<rest::Error>>().is_some()
		|| res.status().is_client_error()
		|| res.status().is_server_error();
	if has_error {
		let _ = dbx.rollback_txn().await;
	} else {
		dbx.commit_txn()
			.await
			.map_err(model::Error::from)
			.map_err(Error::from)?;
	}

	Ok(res)
}
