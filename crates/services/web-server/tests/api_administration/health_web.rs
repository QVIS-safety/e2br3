use crate::common::{init_test_mm, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn health_is_public_no_content_with_empty_body() -> Result<()> {
	let app = web_server::app(init_test_mm().await?);
	let response = app
		.oneshot(Request::builder().uri("/health").body(Body::empty())?)
		.await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
	assert!(to_bytes(response.into_body(), usize::MAX).await?.is_empty());
	Ok(())
}
