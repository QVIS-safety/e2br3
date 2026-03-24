use crate::persist_workflow::{
	create_case, db_count_by_case, db_fetch_json_by_case,
	disable_export_validation_for_test, export_case_xml, fill_section_g,
	force_case_validated_for_export, request_json, save_case, setup,
	validate_case_fda,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn g_section_save_validate_export_roundtrip() -> crate::common::Result<()> {
	disable_export_validation_for_test();
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_g(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;
	validate_case_fda(&ctx, case_id).await?;
	force_case_validated_for_export(&ctx, case_id).await?;

	let count = db_count_by_case(&ctx, "drug_information", case_id).await?;
	assert!(count >= 1, "expected drugs row for case {case_id}");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!("/api/cases/{case_id}/drugs"),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	let drugs = body["data"].as_array().cloned().unwrap_or_default();
	assert!(
		!drugs.is_empty(),
		"expected at least one persisted drug row"
	);
	let drug = &drugs[0];
	assert_eq!(drug["mpid"], "Persist-MPID");
	assert_eq!(drug["mpid_version"], "2026.03");
	assert_eq!(drug["phpid"], "Persist-PhPID");
	assert_eq!(drug["brand_name"], "Persist Brand");
	assert_eq!(drug["drug_generic_name"], "Persist Generic");
	assert_eq!(drug["drug_authorization_number"], "AUTH-123");
	assert_eq!(drug["manufacturer_name"], "Persist Manufacturer");
	assert_eq!(drug["obtain_drug_country"], "KR");
	assert_eq!(drug["parent_route_termid"], "PARENT-ROUTE-ID");
	assert_eq!(drug["fda_additional_info_coded"], "FDA-ADD-1");
	assert_eq!(drug["fda_specialized_product_category"], "COMBINATION");

	let db_drug = db_fetch_json_by_case(
		&ctx,
		"SELECT json_build_object(
			'mpid', mpid,
			'mpid_version', mpid_version,
			'phpid', phpid,
			'phpid_version', phpid_version,
			'drug_generic_name', drug_generic_name,
			'drug_authorization_number', drug_authorization_number,
			'obtain_drug_country', obtain_drug_country,
			'drug_additional_info_codes_json', drug_additional_info_codes_json,
			'fda_device_info_json', fda_device_info_json,
			'fda_specialized_product_category', fda_specialized_product_category
		) FROM drug_information WHERE case_id = $1 ORDER BY sequence_number LIMIT 1",
		case_id,
	)
	.await?;
	assert_eq!(db_drug["mpid"], "Persist-MPID");
	assert_eq!(db_drug["phpid_version"], "2026.04");
	assert_eq!(db_drug["drug_generic_name"], "Persist Generic");
	assert_eq!(db_drug["drug_authorization_number"], "AUTH-123");
	assert_eq!(db_drug["obtain_drug_country"], "KR");
	assert_eq!(
		db_drug["drug_additional_info_codes_json"][0]["value_code"],
		"ADD-CODE-1"
	);
	assert_eq!(
		db_drug["drug_additional_info_codes_json"][1]["value_code"],
		"ADD-CODE-2"
	);
	assert_eq!(
		db_drug["fda_device_info_json"]["device_brand_name"],
		"Persist Device Brand"
	);
	assert_eq!(
		db_drug["fda_device_info_json"]["device_problem_codes"][0]["value_code"],
		"DP-1"
	);
	assert_eq!(
		db_drug["fda_device_info_json"]["remedial_actions"][0]["value_code"],
		"RA-1"
	);

	let xml = export_case_xml(&ctx, case_id).await?;
	assert!(xml.contains("Persist Drug"));
	assert!(xml.contains("Persist Substance"));
	assert!(xml.contains("Persist Device Brand"));
	assert!(xml.contains("Persist Common Device"));
	assert!(xml.contains("COMBINATION"));
	assert!(xml.contains("ADD-CODE-1"));
	Ok(())
}
