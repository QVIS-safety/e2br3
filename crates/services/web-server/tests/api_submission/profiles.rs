use super::support::*;
use lib_core::ctx::Ctx;
use lib_core::model::organization_as2_profile::OrganizationAs2ProfileBmc as Profiles;

#[serial]
#[tokio::test]
async fn profile_admin_api_enforces_permission_and_omits_secrets() -> Result<()> {
	clear_submission_env();
	configure_as2_test_endpoint("http://127.0.0.1:9");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let admin = crate::common::insert_user(
		&mm,
		seed.org_id,
		lib_core::ctx::ROLE_SYSTEM_ADMIN,
		crate::common::system_user_id(),
		Some("systempwd"),
	)
	.await?;
	let app = web_server::app(mm).expect("configuration");
	let user_cookie = cookie_header(
		&generate_web_token(&seed.admin.email, seed.admin.token_salt)?.to_string(),
	);
	let admin_cookie = cookie_header(
		&generate_web_token(&admin.email, admin.token_salt)?.to_string(),
	);
	let collection = format!("/api/organizations/{}/as2-profiles", seed.org_id);
	let uri = format!("{collection}/fda");
	let input = profile_input();
	let body = json!({"endpointUrl":input.endpoint_url,
		"as2To":input.as2_to,"expectedAckAs2From":input.expected_ack_as2_from,
		"signingPkcs12Base64":input.signing_pkcs12_base64,"signingPkcs12Password":input.signing_pkcs12_password,
		"partnerCertificatePem":input.partner_certificate_pem});
	let (denied, _) = put_json(&app, &user_cookie, &uri, body.clone()).await?;
	assert_eq!(denied, StatusCode::FORBIDDEN);
	let mut duplicate_route = body.clone();
	duplicate_route["routingId"] = json!("MUST-NOT-BE-STORED-HERE");
	let (rejected, _) = put_json(&app, &admin_cookie, &uri, duplicate_route).await?;
	assert_eq!(rejected, StatusCode::UNPROCESSABLE_ENTITY);
	let (saved, value) = put_json(&app, &admin_cookie, &uri, body).await?;
	assert_eq!(saved, StatusCode::OK, "{value}");
	assert!(value["data"].get("signingPkcs12Password").is_none());
	let (listed, value) = get_json(&app, &admin_cookie, &collection).await?;
	assert_eq!(listed, StatusCode::OK, "{value}");
	assert!(!value.to_string().contains("changeit"));
	let deleted = app
		.oneshot(
			Request::builder()
				.method("DELETE")
				.uri(uri)
				.header("cookie", admin_cookie)
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
	clear_submission_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn organization_profile_isolation_encryption_rotation_and_revocation(
) -> Result<()> {
	clear_submission_env();
	configure_as2_test_endpoint("http://127.0.0.1:9");
	let mm = init_test_mm().await?;
	let first = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let second = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	crate::common::create_test_import_product(&mm, first.org_id, first.admin.id)
		.await?;
	crate::common::create_test_import_product(&mm, second.org_id, second.admin.id)
		.await?;
	let ctx = Ctx::new(
		first.admin.id,
		first.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	assert!(Profiles::list(&ctx, &mm, second.org_id).await?.is_empty());
	let profile =
		Profiles::active_for_submission(&ctx, &mm, first.org_id, "fda").await?;
	let public = serde_json::to_value(&profile)?;
	assert!(public.get("signingPkcs12Password").is_none());
	assert!(public.get("signingPkcs12Base64").is_none());
	let runtime = Profiles::runtime_profiles(&Ctx::root_ctx(), &mm).await?;
	let private = runtime
		.iter()
		.find(|row| row.id == profile.id)
		.ok_or("missing runtime profile")?;
	assert_eq!(private.signing_pkcs12_password, "changeit");
	assert_eq!(private.routing_id, format!("{}-fda", first.org_id));
	assert!(public.get("routingId").is_none());
	use base64::Engine;
	let decoded = base64::engine::general_purpose::STANDARD
		.decode(private.signing_pkcs12_base64.replace('\n', ""))?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		first.admin.id,
		first.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (ciphertext,): (Vec<u8>,) = mm.dbx().fetch_one(sqlx::query_as(
		"SELECT signing_pkcs12_ciphertext FROM organization_as2_profiles WHERE id=$1"
	).bind(profile.id)).await?;
	mm.dbx().commit_txn().await?;
	assert_ne!(ciphertext, decoded);
	let mut input = profile_input();
	input.signing_pkcs12_password = "incorrect-password".into();
	assert!(Profiles::upsert(&ctx, &mm, first.org_id, "fda", input)
		.await
		.is_err());
	let updated =
		Profiles::upsert(&ctx, &mm, first.org_id, "fda", profile_input()).await?;
	assert_eq!(updated.id, profile.id);
	assert_eq!(updated.version, 2);
	Profiles::deactivate(&ctx, &mm, first.org_id, "fda").await?;
	assert!(
		Profiles::active_for_submission(&ctx, &mm, first.org_id, "fda")
			.await
			.is_err()
	);
	// A rejected request aborts its transaction; the next request gets a fresh Dbx.
	assert!(!Profiles::runtime_profiles(&Ctx::root_ctx(), &mm.clone())
		.await?
		.iter()
		.any(|row| row.id == profile.id));
	clear_submission_env();
	Ok(())
}

#[serial]
#[tokio::test]
async fn runtime_profiles_require_token_and_disable_response_caching() -> Result<()>
{
	clear_submission_env();
	configure_as2_test_endpoint("http://127.0.0.1:9");
	let mm = init_test_mm().await?;
	let app = web_server::app(mm).expect("configuration");
	let uri = "/internal/submissions/as2-profiles";
	let denied = app
		.clone()
		.oneshot(Request::builder().uri(uri).body(Body::empty())?)
		.await?;
	assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
	let allowed = app
		.oneshot(
			Request::builder()
				.uri(uri)
				.header("x-callback-token", "callback-secret")
				.body(Body::empty())?,
		)
		.await?;
	assert_eq!(allowed.status(), StatusCode::OK);
	assert_eq!(allowed.headers()["cache-control"], "no-store");
	clear_submission_env();
	Ok(())
}
