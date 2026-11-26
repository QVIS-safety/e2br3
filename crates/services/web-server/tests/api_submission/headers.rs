use super::support::*;
use lib_core::ctx::Ctx;
use lib_core::regulatory::RegulatoryAuthority;
use xml::{export_case_xml_with_options, ExportXmlOptions};

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
	assert_eq!(header.message_sender_identifier, "TEST-SENDER");
	assert_eq!(header.message_receiver_identifier, "CDER_IND");
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

	Ok(())
}
