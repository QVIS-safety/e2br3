use super::support::*;

#[serial]
#[tokio::test]
async fn test_import_export_submission_histories_follow_scope() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let scoped_manager = insert_user(
		&mm,
		seed.org_id,
		TEST_CUSTOM_MANAGER_ROLE,
		system_user_id(),
		Some("managerpwd"),
	)
	.await?;
	let admin_token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let manager_token =
		generate_web_token(&scoped_manager.email, scoped_manager.token_salt)?;
	let admin_cookie = cookie_header(&admin_token.to_string());
	let manager_cookie = cookie_header(&manager_token.to_string());
	let app = web_server::app(mm.clone());
	let sender_id =
		create_sender_presave(&app, &admin_cookie, "Sender Hist Org", "SEND-HIST")
			.await?;
	let product_allowed =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand A").await?;
	let product_hidden =
		create_product_presave(&app, &admin_cookie, sender_id, "Brand B").await?;

	let other_sender = create_sender_presave(
		&app,
		&admin_cookie,
		"Other History Sender",
		"OTHER-HIST",
	)
	.await?;
	let study_allowed =
		create_study_presave(&app, &admin_cookie, product_allowed, "Study A")
			.await?;
	let study_hidden =
		create_study_presave(&app, &admin_cookie, product_hidden, "Study B").await?;

	let case_allowed_number = format!("SR-HIST-A-{}", Uuid::new_v4());
	let case_hidden_number = format!("SR-HIST-B-{}", Uuid::new_v4());
	let case_allowed =
		create_case(&app, &admin_cookie, &case_allowed_number, Some("PROD-A"))
			.await?;
	let case_hidden =
		create_case(&app, &admin_cookie, &case_hidden_number, Some("PROD-B"))
			.await?;
	create_message_header(&app, &admin_cookie, case_allowed, "SEND-HIST").await?;
	create_message_header(&app, &admin_cookie, case_hidden, "SEND-HIST").await?;
	create_sender_information(&app, &admin_cookie, case_allowed, "Sender Hist Org")
		.await?;
	create_sender_information(&app, &admin_cookie, case_hidden, "Sender Hist Org")
		.await?;
	create_drug_with_brand(&app, &admin_cookie, case_allowed, "Brand A").await?;
	create_drug_with_brand(&app, &admin_cookie, case_hidden, "Brand B").await?;
	create_study(&app, &admin_cookie, case_allowed, "Study A").await?;
	create_study(&app, &admin_cookie, case_hidden, "Study B").await?;
	link_case_presave_sources(
		&mm,
		case_allowed,
		seed.admin.id,
		seed.org_id,
		Some(sender_id),
		Some(product_allowed),
		Some(study_allowed),
	)
	.await?;
	link_case_presave_sources(
		&mm,
		case_hidden,
		seed.admin.id,
		seed.org_id,
		Some(sender_id),
		Some(product_hidden),
		Some(study_hidden),
	)
	.await?;
	insert_history_rows_for_case(
		&mm,
		case_allowed,
		&case_allowed_number,
		seed.admin.id,
		seed.org_id,
		"allowed",
	)
	.await?;
	insert_history_rows_for_case(
		&mm,
		case_hidden,
		&case_hidden_number,
		seed.admin.id,
		seed.org_id,
		"hidden",
	)
	.await?;
	let unlinked_number = format!("SR-HIST-UNLINKED-{}", Uuid::new_v4());
	let unlinked_case =
		create_case(&app, &admin_cookie, &unlinked_number, None).await?;
	insert_history_rows_for_case(
		&mm,
		unlinked_case,
		&unlinked_number,
		seed.admin.id,
		seed.org_id,
		"unlinked",
	)
	.await?;

	// A failed import has no case: ownership/admin rules apply independently of case scope.
	use lib_core::ctx::Ctx;
	use lib_core::model::xml_import_history::{
		XmlImportHistoryBmc, XmlImportHistoryStatus,
	};
	let other_org =
		seed_org_with_users(&mm, "otheradminpwd", "otherviewpwd").await?;
	for (user, org_id, role, file) in [
		(
			&scoped_manager,
			seed.org_id,
			TEST_CUSTOM_MANAGER_ROLE,
			"own-error.xml",
		),
		(
			&seed.admin,
			seed.org_id,
			ROLE_SPONSOR_ADMIN_CRO,
			"admin-error.xml",
		),
		(
			&other_org.admin,
			other_org.org_id,
			ROLE_SPONSOR_ADMIN_CRO,
			"other-org-error.xml",
		),
	] {
		let ctx = Ctx::new(user.id, org_id, role.to_string())?;
		XmlImportHistoryBmc::record(
			&mm,
			&ctx,
			file,
			file,
			None,
			None,
			XmlImportHistoryStatus::Error,
			Some("invalid XML"),
		)
		.await?;
	}

	for (senders, products, studies, allowed_visible, hidden_visible) in [
		(vec![], vec![], vec![], true, true),
		(vec![sender_id], vec![], vec![], true, true),
		(vec![other_sender], vec![], vec![], false, false),
		(vec![sender_id], vec![product_allowed], vec![], true, false),
		(vec![sender_id], vec![], vec![study_allowed], true, false),
		(
			vec![sender_id],
			vec![product_allowed],
			vec![study_allowed],
			true,
			false,
		),
	] {
		update_user_scope(
			&app,
			&admin_cookie,
			scoped_manager.id,
			json!({
				"access_sender_ids": senders,
				"access_product_ids": products,
				"access_study_ids": studies,
			}),
		)
		.await?;
		for uri in [
			"/api/import/xml/history",
			"/api/exports/history",
			"/api/submissions/history",
		] {
			let (status, history) =
				request_json(&app, "GET", &manager_cookie, uri.to_string(), None)
					.await?;
			assert_eq!(status, StatusCode::OK, "{uri}: {history:?}");
			let items = history["data"]["items"]
				.as_array()
				.ok_or("missing history items")?;
			for (case_id, visible) in [
				(case_allowed, allowed_visible),
				(case_hidden, hidden_visible),
				(unlinked_case, true),
			] {
				assert_eq!(items.iter().any(|row| row["caseId"] == case_id.to_string()), visible,
					"{uri}, case={case_id}, senders={senders:?}, products={products:?}, studies={studies:?}: {history}");
			}
			if uri == "/api/exports/history" {
				assert!(
					items.iter().all(|row| row["exportedAt"].is_string()),
					"{history}"
				);
			}
			if uri == "/api/import/xml/history" {
				let errors: Vec<_> =
					items.iter().filter(|row| row["caseId"].is_null()).collect();
				assert_eq!(errors.len(), 1, "{history}");
				assert_eq!(errors[0]["sourceFileName"], "own-error.xml");
			}
		}
	}

	let (status, history) = request_json(
		&app,
		"GET",
		&admin_cookie,
		"/api/import/xml/history".to_string(),
		None,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{history}");
	let mut files: Vec<_> = history["data"]["items"]
		.as_array()
		.ok_or("missing import history")?
		.iter()
		.filter(|row| row["caseId"].is_null())
		.map(|row| row["sourceFileName"].as_str().unwrap())
		.collect();
	files.sort_unstable();
	assert_eq!(files, ["admin-error.xml", "own-error.xml"], "{history}");
	Ok(())
}
