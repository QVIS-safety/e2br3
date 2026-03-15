use crate::persist_workflow::{
	create_case, db_count_by_case, db_fetch_json_by_case, fill_section_g,
	request_json, save_case, setup,
};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn g_section_forms_persist_to_db_after_save() -> crate::common::Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	fill_section_g(&ctx, case_id).await?;
	save_case(&ctx, case_id).await?;

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

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!(
			"/api/cases/{case_id}/drugs/{}/active-substances",
			drug["id"].as_str().unwrap()
		),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	let substances = body["data"].as_array().cloned().unwrap_or_default();
	assert_eq!(substances[0]["substance_termid"], "SUB-TERM-2");
	assert_eq!(substances[0]["substance_termid_version"], "27.0");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!(
			"/api/cases/{case_id}/drugs/{}/dosages",
			drug["id"].as_str().unwrap()
		),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	let dosages = body["data"].as_array().cloned().unwrap_or_default();
	assert_eq!(dosages[0]["dosage_text"], "Persist dosage detail updated");
	assert_eq!(dosages[0]["dose_form_termid"], "DF-2");
	assert_eq!(dosages[0]["route_of_administration"], "061");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!(
			"/api/cases/{case_id}/drugs/{}/indications",
			drug["id"].as_str().unwrap()
		),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	let indications = body["data"].as_array().cloned().unwrap_or_default();
	assert_eq!(
		indications[0]["indication_text"],
		"Persist indication updated"
	);
	assert_eq!(indications[0]["indication_meddra_code"], "246810");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!(
			"/api/cases/{case_id}/drugs/{}/recurrences",
			drug["id"].as_str().unwrap()
		),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	let recurrences = body["data"].as_array().cloned().unwrap_or_default();
	assert_eq!(recurrences[0]["rechallenge_action"], "1");
	assert_eq!(recurrences[0]["reaction_meddra_code"], "10012345");

	let (status, body) = request_json(
		&ctx.app,
		&ctx.cookie,
		"GET",
		format!(
			"/api/cases/{case_id}/drugs/{}/reaction-assessments",
			drug["id"].as_str().unwrap()
		),
		None,
	)
	.await?;
	assert_eq!(status, axum::http::StatusCode::OK);
	let assessments = body["data"].as_array().cloned().unwrap_or_default();
	assert_eq!(
		assessments[0]["administration_start_interval_value"],
		"5.00"
	);
	assert_eq!(assessments[0]["recurrence_meddra_code"], "10054321");
	Ok(())
}
