use super::support::{
	create_product_presave, import_xml_fixture_with_product, post_json, put_json,
	scenario6,
};
use crate::common::{
	cookie_header, create_test_import_product, init_test_mm, seed_org_with_users,
	Result,
};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::{set_org_context, set_user_context};
use serde_json::json;
use serial_test::serial;
use time::Date;

#[serial]
#[tokio::test]
async fn test_import_settings_update_enabled_c1_dates() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let product_presave_id =
		create_test_import_product(&mm, seed.org_id, seed.admin.id)
			.await?
			.to_string();

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"timezone": "UTC",
				"import_date_update": {
					"date_of_creation": true,
					"most_recent_info_date": true,
					"report_first_received_date": false
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = scenario6(&root)?.replace(
		"US-APHARMA-8744554B",
		&format!("US-TEST-{}", uuid::Uuid::new_v4()),
	);
	let before_import = time::OffsetDateTime::now_utc().date();
	let (status, body) = import_xml_fixture_with_product(
		&app,
		&cookie,
		"FAERS2022Scenario6.xml",
		xml.as_bytes(),
		&product_presave_id,
	)
	.await?;
	let after_import = time::OffsetDateTime::now_utc().date();
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let case_id = body["data"]["importedCases"][0]["caseId"]
		.as_str()
		.ok_or_else(|| format!("missing imported case id in body {body:?}"))?;
	let case_id = uuid::Uuid::parse_str(case_id)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let row = sqlx::query_as::<_, (String, String, String)>(
		"SELECT transmission_date, date_first_received_from_source, date_of_most_recent_information
		 FROM safety_report_identification WHERE case_id = $1",
	)
	.bind(case_id)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;

	let format_e2b_date = |date: Date| {
		format!(
			"{:04}{:02}{:02}",
			date.year(),
			u8::from(date.month()),
			date.day()
		)
	};
	let before_date = format_e2b_date(before_import);
	let after_date = format_e2b_date(after_import);
	for (field, actual, suffix) in [
		("transmission_date", row.0.as_str(), "000000"),
		("date_of_most_recent_information", row.2.as_str(), ""),
	] {
		assert!(
			actual == format!("{before_date}{suffix}")
				|| actual == format!("{after_date}{suffix}"),
			"{field} must use the import date captured before or after the request: actual={actual}, before={before_date}{suffix}, after={after_date}{suffix}"
		);
	}
	assert_eq!(
		row.1, "20140614",
		"disabled report_first_received_date setting must preserve the source C.1.4 value"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_settings_apply_default_sender_only_when_enabled() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/senders",
		json!({
			"data": { "rows": {
				"sender": {
					"senderType": "2", "organizationName": "Admin Default Sender",
					"streetAddress": "10 Default Road", "city": "Seoul",
					"countryCode": "KR", "email": "default-sender@example.test",
					"isDefault": true
				},
				"gateways": [],
				"responsiblePersons": [{
					"sequenceNumber": 1,
					"department": "Import Ops",
					"personTitle": "Mr",
					"personGivenName": "Charles",
					"personFamilyName": "Conner"
				}]
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let default_sender_id = body["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing sender id in body {body:?}"))?;
	let product_presave_id = create_product_presave(
		&app,
		&cookie,
		default_sender_id,
		"IMPORT-DEFAULT-PRODUCT",
	)
	.await?;

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let source_xml = scenario6(&root)?;

	let disabled_case_number =
		format!("US-SENDER-DISABLED-{}", uuid::Uuid::new_v4());
	let disabled_xml =
		source_xml.replace("US-APHARMA-8744554B", &disabled_case_number);
	let (status, body) = import_xml_fixture_with_product(
		&app,
		&cookie,
		"FAERS2022Scenario6.xml",
		disabled_xml.as_bytes(),
		&product_presave_id,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let disabled_case_id = body["data"]["importedCases"][0]["caseId"]
		.as_str()
		.ok_or_else(|| format!("missing imported case id in body {body:?}"))?;
	let disabled_case_id = uuid::Uuid::parse_str(disabled_case_id)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let disabled_sender =
		sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
			"SELECT sender_type, organization_name, email
		 FROM sender_information WHERE case_id = $1 LIMIT 1",
		)
		.bind(disabled_case_id)
		.fetch_one(&mut *tx)
		.await?;
	tx.commit().await?;
	assert_eq!(disabled_sender.0.as_deref(), Some("1"));
	assert_eq!(disabled_sender.1.as_deref(), Some("Big Pharma"));
	assert_eq!(
		disabled_sender.2.as_deref(),
		Some("emailAddress@company.com")
	);

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"apply_sender_info_to_imported_cases": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let xml = disabled_xml
		.replace(
			&disabled_case_number,
			&format!("US-SENDER-{}", uuid::Uuid::new_v4()),
		)
		.replace("<low value=\"20090101\"/>", "<low value=\"20090103\"/>");
	let (status, body) = import_xml_fixture_with_product(
		&app,
		&cookie,
		"FAERS2022Scenario6.xml",
		xml.as_bytes(),
		&product_presave_id,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let case_id = body["data"]["importedCases"][0]["caseId"]
		.as_str()
		.ok_or_else(|| format!("missing imported case id in body {body:?}"))?;
	let case_id = uuid::Uuid::parse_str(case_id)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let sender = sqlx::query_as::<
		_,
		(
			Option<String>,
			Option<String>,
			Option<String>,
			Option<String>,
			Option<String>,
			Option<String>,
			Option<String>,
		),
	>(
		"SELECT sender_type, organization_name, email, person_title, person_given_name, person_family_name, fax
		 FROM sender_information WHERE case_id = $1 LIMIT 1",
	)
	.bind(case_id)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;

	assert_eq!(sender.0.as_deref(), Some("2"));
	assert_eq!(sender.1.as_deref(), Some("Admin Default Sender"));
	assert_eq!(sender.2.as_deref(), Some("default-sender@example.test"));
	assert_eq!(sender.3, None);
	assert_eq!(sender.4, None);
	assert_eq!(sender.5, None);
	assert_eq!(sender.6, None);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_import_settings_apply_product_linked_sender_by_imported_product_id(
) -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/senders",
		json!({
			"data": { "rows": {
				"sender": { "senderType": "2", "organizationName": "Fallback Default Sender", "email": "fallback-default@example.test", "isDefault": true },
				"gateways": [], "responsiblePersons": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/senders",
		json!({
			"data": { "rows": {
				"sender": { "senderType": "3", "organizationName": "Product Linked Sender", "email": "product-linked@example.test" },
				"gateways": [], "responsiblePersons": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let linked_sender_id = body["data"]["rows"]["sender"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing linked sender id in body {body:?}"))?;

	let (status, body) = post_json(
		&app,
		&cookie,
		"/api/presaves/products",
		json!({
			"data": { "rows": {
				"product": { "senderPresaveId": linked_sender_id, "productId": "Drug A", "medicinalProduct": "Drug A" },
				"activeSubstances": []
			} }
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body:?}");
	let product_presave_id = body["data"]["rows"]["product"]["id"]
		.as_str()
		.ok_or_else(|| format!("missing product id in body {body:?}"))?
		.to_string();

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"apply_sender_info_to_imported_cases": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = scenario6(&root)?.replace(
		"US-APHARMA-8744554B",
		&format!("US-PRODUCT-SENDER-{}", uuid::Uuid::new_v4()),
	);
	let (status, body) = import_xml_fixture_with_product(
		&app,
		&cookie,
		"FAERS2022Scenario6.xml",
		xml.as_bytes(),
		&product_presave_id,
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let case_id = body["data"]["importedCases"][0]["caseId"]
		.as_str()
		.ok_or_else(|| format!("missing imported case id in body {body:?}"))?;
	let case_id = uuid::Uuid::parse_str(case_id)?;

	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let sender =
		sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
			"SELECT sender_type, organization_name, email
		 FROM sender_information WHERE case_id = $1 LIMIT 1",
		)
		.bind(case_id)
		.fetch_one(&mut *tx)
		.await?;
	tx.commit().await?;

	assert_eq!(sender.0.as_deref(), Some("3"));
	assert_eq!(sender.1.as_deref(), Some("Product Linked Sender"));
	assert_eq!(sender.2.as_deref(), Some("product-linked@example.test"));

	Ok(())
}
