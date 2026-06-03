use crate::error::Result;
use crate::middleware::mw_auth::CtxW;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn mw_ctx_require_and_set_dbx(
	ctx: Result<CtxW>,
	req: Request<Body>,
	next: Next,
) -> Result<Response> {
	ctx?;
	Ok(next.run(req).await)
}
