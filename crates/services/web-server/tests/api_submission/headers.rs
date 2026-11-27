use super::support::*;
use crate::xml_support::xpath_value;
use lib_core::ctx::Ctx;
use lib_core::regulatory::RegulatoryAuthority;
use xml::{export_case_xml_with_options, ExportXmlOptions};

#[serial]
#[tokio::test]
async fn incomplete_headers_download_as_xml_and_zip_without_invented_values(
) -> Result<()> {
	use std::io::{Cursor, Read};
	clear_submission_env();
	let mm = init_test_mm().await?;
	let seed =
		crate::common::seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("configuration");
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx().execute(sqlx::query("UPDATE safety_report_identification SET safety_report_id = NULL, transmission_date = NULL, report_type = NULL WHERE case_id = $1").bind(case_id)).await?;
	mm.dbx().execute(sqlx::query("UPDATE cases SET fda_report_type = NULL, mfds_report_type = NULL, dg_prd_key = NULL WHERE id = $1").bind(case_id)).await?;
	mm.dbx().commit_txn().await?;
	for authority in ["ich", "fda", "mfds"] {
		for bulk in [false, true] {
			let request = if bulk {
				Request::builder()
					.method("POST")
					.uri("/api/cases/export/xml")
					.header("content-type", "application/json")
					.header("cookie", &cookie)
					.body(Body::from(
						json!({"case_ids":[case_id],"authority":authority})
							.to_string(),
					))?
			} else {
				Request::builder()
					.uri(format!(
						"/api/cases/{case_id}/export/xml?authority={authority}"
					))
					.header("cookie", &cookie)
					.body(Body::empty())?
			};
			let response = app.clone().oneshot(request).await?;
			let status = response.status();
			let body = to_bytes(response.into_body(), usize::MAX).await?;
			assert_eq!(
				status,
				StatusCode::OK,
				"{authority} bulk={bulk}: {}",
				String::from_utf8_lossy(&body)
			);
			let xml = if bulk {
				let mut archive = zip::ZipArchive::new(Cursor::new(body))?;
				assert_eq!(archive.len(), 1);
				let mut xml = String::new();
				let mut file = archive.by_index(0)?;
				assert_eq!(file.name(), format!("{case_id}-{authority}.xml"));
				file.read_to_string(&mut xml)?;
				xml
			} else {
				String::from_utf8(body.to_vec())?
			};
			let parsed =
				xml::validation::validate_e2b_xml_basic(xml.as_bytes(), None)?;
			assert!(parsed.ok, "{:?}", parsed.errors);
			for path in [
				"//*[local-name()='PORR_IN049016UV']/*[local-name()='id']/@extension",
				"//*[local-name()='PORR_IN049016UV']/*[local-name()='creationTime']/@value",
				"//*[local-name()='sender']/*[local-name()='device']/*[local-name()='id']/@extension",
				"//*[local-name()='receiver']/*[local-name()='device']/*[local-name()='id']/@extension",
				"//@nullFlavor",
			] {
				assert_eq!(xpath_value(xml.as_bytes(), &format!("count({path})"))?, "0", "{path}");
			}
		}
	}
	let (status, history) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/exports/history"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{history}");
	let items = history["data"]["items"].as_array().ok_or("history")?;
	assert_eq!(items.len(), 6);
	assert!(items.iter().all(|item| item["status"] == "success"));
	let (status, case) =
		get_json(&app, &cookie, &format!("/api/cases/{case_id}")).await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(case["data"]["status"], "draft");
	Ok(())
}

#[serial]
#[tokio::test]
async fn outbound_header_is_transient_and_uses_receiver_presave_route() -> Result<()>
{
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone()).expect("web server configuration");
	let case_id = create_case(&app, &cookie, seed.org_id).await?;
	let sender_id = Uuid::new_v4();
	let receiver_id = Uuid::new_v4();
	let product_id = Uuid::new_v4();

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO sender_presaves
				 (id, organization_id, sender_type, organization_name, created_by)
				 VALUES ($1, $2, '1', 'Test Sender', $3)",
			)
			.bind(sender_id)
			.bind(seed.org_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO sender_presave_gateways
				 (sender_presave_id, sequence_number, gateway_authority,
				  sender_identifier, is_default_for_authority, created_by)
				 VALUES ($1, 1, 'fda', 'TEST-SENDER', true, $2)",
			)
			.bind(sender_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO receiver_presaves
				 (id, organization_id, receiver_type, organization_name, created_by)
				 VALUES ($1, $2, 'Regulatory Authority', 'FDA Receiver', $3)",
			)
			.bind(receiver_id)
			.bind(seed.org_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO receiver_presave_routes
				 (receiver_presave_id, sequence_number, authority, receiver_label,
				  batch_receiver_identifier, message_receiver_identifier,
				  condition_page, condition_field_code, condition_operator,
				  condition_value_code, condition_value_label, created_by)
				 VALUES ($1, 1, 'fda', 'FDA IND', 'ZZFDA_PREMKT', 'CDER_IND',
				  'CI', 'FDA_REPORT_TYPE', 'Equal', '1', 'CDER IND', $2)",
			)
			.bind(receiver_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO product_presaves
				 (id, organization_id, sender_presave_id, receiver_presave_id,
				  product_id, medicinal_product, created_by)
				 VALUES ($1, $2, $3, $4, 'TEST-PRODUCT', 'Test Product', $5)",
			)
			.bind(product_id)
			.bind(seed.org_id)
			.bind(sender_id)
			.bind(receiver_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"INSERT INTO sender_information
				 (case_id, source_sender_presave_id, sender_type, organization_name, created_by)
				 VALUES ($1, $2, '1', 'Test Sender', $3)",
			)
			.bind(case_id)
			.bind(sender_id)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE cases
				 SET dg_prd_key = 'TEST-PRODUCT', fda_report_type = '1'
				 WHERE id = $1",
			)
			.bind(case_id),
		)
		.await?;
	mm.dbx()
		.execute(
			sqlx::query(
				"UPDATE safety_report_identification
				 SET transmission_date = '20261126000000', report_type = '1'
				 WHERE case_id = $1",
			)
			.bind(case_id),
		)
		.await?;
	mm.dbx().commit_txn().await?;

	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let header = web_server::submission::prepare_outbound_message_header(
		&ctx,
		&mm,
		case_id,
		RegulatoryAuthority::Fda,
	)
	.await?;
	assert_eq!(
		header.message_sender_identifier.as_deref(),
		Some("TEST-SENDER")
	);
	assert_eq!(
		header.message_receiver_identifier.as_deref(),
		Some("CDER_IND")
	);
	assert_eq!(
		header.batch_receiver_identifier.as_deref(),
		Some("ZZFDA_PREMKT")
	);
	assert_eq!(
		header.batch_number.as_deref(),
		Some(format!("BATCH-{case_id}").as_str())
	);

	let exported = export_case_xml_with_options(
		&ctx,
		&mm,
		case_id,
		ExportXmlOptions {
			apply_comments: false,
			authority: RegulatoryAuthority::Fda,
			outbound_message_header: header,
		},
	)
	.await?;
	assert!(xml::validation::validate_export_message_header(
		exported.as_bytes(),
		RegulatoryAuthority::Fda,
	)?
	.is_empty());

	let table: Option<String> =
		sqlx::query_scalar("SELECT to_regclass('public.message_headers')::text")
			.fetch_one(mm.dbx().db())
			.await?;
	assert_eq!(table, None, "case message-header storage must not exist");

	for (change, sender, receiver) in [
		("UPDATE cases SET dg_prd_key = NULL WHERE id = $1", true, None),
		("UPDATE cases SET dg_prd_key = 'TEST-PRODUCT', fda_report_type = NULL WHERE id = $1", true, None),
		("UPDATE cases SET fda_report_type = 'unconfigured' WHERE id = $1", true, None),
		("UPDATE cases SET fda_report_type = '1' WHERE id = $1", true, Some("CDER_IND")),
		("UPDATE receiver_presave_routes SET message_receiver_identifier = 'INVALID-ROUTE' WHERE receiver_presave_id = (SELECT receiver_presave_id FROM product_presaves WHERE product_id = 'TEST-PRODUCT' AND organization_id = (SELECT organization_id FROM cases WHERE id = $1))", true, Some("INVALID-ROUTE")),
		("UPDATE sender_presave_gateways SET sender_identifier = NULL WHERE sender_presave_id = (SELECT source_sender_presave_id FROM sender_information WHERE case_id = $1)", false, Some("INVALID-ROUTE")),
		("UPDATE sender_information SET source_sender_presave_id = NULL WHERE case_id = $1", false, None),
	] {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx(mm.dbx(), seed.admin.id, seed.org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
		mm.dbx().execute(sqlx::query(change).bind(case_id)).await?;
		mm.dbx().commit_txn().await?;
		for bulk in [false, true] {
			use std::io::{Cursor, Read};
			let request = if bulk {
				Request::builder().method("POST").uri("/api/cases/export/xml")
					.header("content-type", "application/json").header("cookie", &cookie)
					.body(Body::from(json!({"case_ids":[case_id],"authority":"fda"}).to_string()))?
			} else {
				Request::builder().uri(format!("/api/cases/{case_id}/export/xml?authority=fda"))
					.header("cookie", &cookie).body(Body::empty())?
			};
			let response = app.clone().oneshot(request).await?;
			let status = response.status();
			let bytes = to_bytes(response.into_body(), usize::MAX).await?;
			assert_eq!(status, StatusCode::OK, "{change}: {}", String::from_utf8_lossy(&bytes));
			let xml = if bulk {
				let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
				let mut xml = String::new();
				archive.by_index(0)?.read_to_string(&mut xml)?;
				xml
			} else { String::from_utf8(bytes.to_vec())? };
			assert_eq!(xpath_value(xml.as_bytes(), "string(//*[local-name()='PORR_IN049016UV']/*[local-name()='sender']/*[local-name()='device']/*[local-name()='id']/@extension)")?, if sender { "TEST-SENDER" } else { "" });
			let value = xpath_value(xml.as_bytes(), "string(//*[local-name()='PORR_IN049016UV']/*[local-name()='receiver']/*[local-name()='device']/*[local-name()='id']/@extension)")?;
			assert_eq!(if value.is_empty() { None } else { Some(value.as_str()) }, receiver);
			if receiver != Some("CDER_IND") {
				assert!(!xml::validation::validate_export_message_header(xml.as_bytes(), RegulatoryAuthority::Fda)?.is_empty());
			}
		}
	}

	Ok(())
}
