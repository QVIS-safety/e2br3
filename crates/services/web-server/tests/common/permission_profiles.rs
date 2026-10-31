use super::Result;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

pub async fn create_empty_permission_profile(
	app: &axum::Router,
	admin_cookie: &str,
	name: String,
) -> Result<String> {
	let body = json!({
		"data": {
			"name": name,
			"privileges": []
		}
	});
	let req = Request::builder()
		.method("POST")
		.uri("/api/admin/permission-profiles")
		.header("cookie", admin_cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	assert_eq!(res.status(), StatusCode::CREATED);
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	let value: serde_json::Value = serde_json::from_slice(&bytes)?;
	Ok(value["id"]
		.as_str()
		.ok_or("missing permission profile id")?
		.to_string())
}
