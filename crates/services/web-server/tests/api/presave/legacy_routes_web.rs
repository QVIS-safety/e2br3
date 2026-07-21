use super::helpers::*;
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::{Method, StatusCode};
use lib_auth::token::generate_web_token;
use uuid::Uuid;

#[tokio::test]
async fn test_legacy_presave_templates_route_is_removed() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let app = web_server::app(mm.clone());

	for uri in [
		"/api/presave-templates".to_string(),
		format!("/api/presave-templates/{}", Uuid::new_v4()),
		format!("/api/presave-templates/{}/audit", Uuid::new_v4()),
		format!("/api/presaves/products/{}/substances", Uuid::new_v4()),
	] {
		let (status, value) =
			request_json(&app, &admin_cookie, Method::GET, uri, None).await?;
		assert_eq!(status, StatusCode::NOT_FOUND, "{value:?}");
	}

	Ok(())
}
