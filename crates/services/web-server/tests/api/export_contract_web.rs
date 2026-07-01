use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO, ROLE_SYSTEM_ADMIN};
use lib_core::model::drug::{
	DosageInformationBmc, DosageInformationForCreate, DrugIndicationBmc,
	DrugIndicationForCreate, DrugInformationBmc, DrugInformationForCreate,
};
use lib_core::model::narrative::{
	NarrativeInformationBmc, NarrativeInformationForCreate,
};
use lib_core::model::patient::{PatientInformationBmc, PatientInformationForCreate};
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate};
use lib_core::model::safety_report::{
	PrimarySourceBmc, PrimarySourceForCreate, SafetyReportIdentificationBmc,
	SenderInformationBmc, SenderInformationForCreate,
};
use lib_core::model::store::{set_org_context, set_user_context};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use serial_test::serial;
use std::io::Cursor;
use time::{Date, Month};
use tower::ServiceExt;
use uuid::Uuid;
use zip::ZipArchive;

async fn get_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn get_response(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
) -> Result<axum::response::Response> {
	let req = Request::builder()
		.method("GET")
		.uri(uri)
		.header("cookie", cookie)
		.body(Body::empty())?;
	Ok(app.clone().oneshot(req).await?)
}

async fn post_json_response(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<axum::response::Response> {
	let req = Request::builder()
		.method("POST")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	Ok(app.clone().oneshot(req).await?)
}

async fn put_json(
	app: &axum::Router,
	cookie: &str,
	uri: &str,
	body: Value,
) -> Result<(StatusCode, Value)> {
	let req = Request::builder()
		.method("PUT")
		.uri(uri)
		.header("cookie", cookie)
		.header("content-type", "application/json")
		.body(Body::from(body.to_string()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let body = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, serde_json::from_slice::<Value>(&body)?))
}

async fn insert_validated_raw_case(
	mm: &lib_core::model::ModelManager,
	org_id: Uuid,
	user_id: Uuid,
	safety_report_id: &str,
) -> Result<Uuid> {
	let case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, user_id).await?;
	set_org_context(&mut tx, org_id, ROLE_SYSTEM_ADMIN).await?;
	sqlx::query(
		"INSERT INTO cases (
			id,
				organization_id,
				status,
				raw_xml,
				created_by,
				updated_by
			) VALUES ($1, $2, 'validated', $3, $4, $4)",
	)
	.bind(case_id)
	.bind(org_id)
	.bind(br#"<?xml version="1.0" encoding="UTF-8"?><test/>"#.as_slice())
	.bind(user_id)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO safety_report_identification (
			case_id,
			safety_report_id,
			version,
			created_by,
			updated_by
		) VALUES ($1, $2, 1, $3, $3)",
	)
	.bind(case_id)
	.bind(safety_report_id)
	.bind(user_id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	Ok(case_id)
}

async fn insert_validated_raw_case_with_xml(
	mm: &lib_core::model::ModelManager,
	org_id: Uuid,
	user_id: Uuid,
	safety_report_id: &str,
	raw_xml: &[u8],
) -> Result<Uuid> {
	let case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, user_id).await?;
	set_org_context(&mut tx, org_id, ROLE_SYSTEM_ADMIN).await?;
	sqlx::query(
		"INSERT INTO cases (
			id,
				organization_id,
				status,
				raw_xml,
				created_by,
				updated_by
			) VALUES ($1, $2, 'validated', $3, $4, $4)",
	)
	.bind(case_id)
	.bind(org_id)
	.bind(raw_xml)
	.bind(user_id)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO safety_report_identification (
			case_id,
			safety_report_id,
			version,
			created_by,
			updated_by
		) VALUES ($1, $2, 1, $3, $3)",
	)
	.bind(case_id)
	.bind(safety_report_id)
	.bind(user_id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;
	Ok(case_id)
}

async fn seed_cioms_case_data(
	mm: &lib_core::model::ModelManager,
	org_id: Uuid,
	user_id: Uuid,
	case_id: Uuid,
	safety_report_id: &str,
) -> Result<()> {
	let ctx = Ctx::new(user_id, org_id, ROLE_SPONSOR_ADMIN_CRO.to_string())?;
	SafetyReportIdentificationBmc::update_by_case(
		&ctx,
		mm,
		case_id,
		lib_core::model::safety_report::SafetyReportIdentificationForUpdate {
			safety_report_id: Some(safety_report_id.to_string()),
			version: None,
			transmission_date: Some("20260528000000".to_string()),
			report_type: lib_core::model::safety_report::PatchValue::Value(
				"1".to_string(),
			),
			date_first_received_from_source: Some(Date::from_calendar_date(
				2026,
				Month::May,
				20,
			)?),
			date_of_most_recent_information: Some(Date::from_calendar_date(
				2026,
				Month::May,
				27,
			)?),
			fulfil_expedited_criteria:
				lib_core::model::safety_report::PatchValue::Value(true),
			fulfil_expedited_criteria_null_flavor: None,
			local_criteria_report_type: Default::default(),
			combination_product_report_indicator: Default::default(),
			first_sender_type: Some("1".to_string()),
			additional_documents_available: Some(false),
			other_case_identifiers_exist: Some(false),
			other_case_identifiers_exist_null_flavor: None,
			worldwide_unique_id: Some("WW-CASE-9001".to_string()),
			nullification_code: None,
			nullification_reason: None,
			receiver_organization: Some("Global Receiver".to_string()),
		},
	)
	.await?;
	PatientInformationBmc::create(
		&ctx,
		mm,
		PatientInformationForCreate {
			case_id,
			patient_initials: Some("AB".to_string()),
			patient_given_name: None,
			patient_family_name: None,
			patient_initials_null_flavor: None,
			birth_date: Some(Date::from_calendar_date(1980, Month::March, 14)?),
			birth_date_null_flavor: None,
			age_at_time_of_onset: Some(Decimal::new(451, 1)),
			age_at_time_of_onset_null_flavor: None,
			age_unit: Some("a".to_string()),
			gestation_period: None,
			gestation_period_unit: None,
			age_group: None,
			weight_kg: None,
			weight_kg_null_flavor: None,
			height_cm: None,
			height_cm_null_flavor: None,
			sex: Some("1".to_string()),
			sex_null_flavor: None,
			race_code: None,
			race_code_null_flavor: None,
			ethnicity_code: None,
			ethnicity_code_null_flavor: None,
			last_menstrual_period_date: None,
			last_menstrual_period_date_null_flavor: None,
			medical_history_text: Some(
				"Hypertension; shellfish allergy".to_string(),
			),
			medical_history_text_null_flavor: None,
			concomitant_therapy: Some(true),
		},
	)
	.await?;
	ReactionBmc::create(
		&ctx,
		mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Anaphylactic reaction".to_string(),
			primary_source_reaction_translation: None,
			reaction_language: Some("en".to_string()),
			reaction_meddra_code: Some("10002198".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			term_highlighted: Some(true),
			serious: Some(true),
			criteria_death: Some(false),
			criteria_death_null_flavor: None,
			criteria_life_threatening: Some(true),
			criteria_life_threatening_null_flavor: None,
			criteria_hospitalization: Some(true),
			criteria_hospitalization_null_flavor: None,
			criteria_disabling: Some(false),
			criteria_disabling_null_flavor: None,
			criteria_congenital_anomaly: Some(false),
			criteria_congenital_anomaly_null_flavor: None,
			criteria_other_medically_important: Some(false),
			criteria_other_medically_important_null_flavor: None,
			required_intervention: None,
			required_intervention_null_flavor: None,
			included_in_ema_ime_list: None,
			expectedness: None,
			severity: None,
			mfds_device_ae_classification: None,
			mfds_device_ae_outcome: None,
			mfds_device_cause_medical_device: None,
			mfds_device_cause_procedure_issue: None,
			mfds_device_cause_patient_condition: None,
			mfds_device_cause_unable_to_assess: None,
			mfds_device_cause_other: None,
			mfds_device_action_reason: None,
			mfds_device_action_recall: None,
			mfds_device_action_repair: None,
			mfds_device_action_inspection: None,
			mfds_device_action_replacement: None,
			mfds_device_action_improvement: None,
			mfds_device_action_monitoring: None,
			mfds_device_action_notification: None,
			mfds_device_action_label_change: None,
			mfds_device_action_other: None,
			start_date: Some(Date::from_calendar_date(2026, Month::May, 18)?),
			start_date_null_flavor: None,
			end_date: Some(Date::from_calendar_date(2026, Month::May, 19)?),
			end_date_null_flavor: None,
			duration_value: None,
			duration_unit: None,
			outcome: Some("2".to_string()),
			medical_confirmation: Some(true),
			country_code: Some("KR".to_string()),
			deleted: Some(false),
		},
	)
	.await?;
	let drug_id = DrugInformationBmc::create(
		&ctx,
		mm,
		DrugInformationForCreate {
			case_id,
			sequence_number: 1,
			drug_characterization: "1".to_string(),
			medicinal_product: "Amoxicillin capsule".to_string(),
			drug_generic_name: Some("Amoxicillin".to_string()),
			brand_name: Some("Amoxil".to_string()),
			dosage_text: Some("500 mg twice daily".to_string()),
			action_taken: Some("1".to_string()),
			rechallenge: Some("3".to_string()),
			manufacturer_name: Some("Acme Pharma".to_string()),
			..Default::default()
		},
	)
	.await?;
	DosageInformationBmc::create(
		&ctx,
		mm,
		DosageInformationForCreate {
			drug_id,
			sequence_number: 1,
			dose_value: None,
			dose_unit: None,
			number_of_units: None,
			frequency_value: None,
			frequency_unit: None,
			first_administration_date: Some(Date::from_calendar_date(
				2026,
				Month::May,
				1,
			)?),
			first_administration_time: None,
			last_administration_date: Some(Date::from_calendar_date(
				2026,
				Month::May,
				10,
			)?),
			last_administration_time: None,
			duration_value: Some(Decimal::new(10, 0)),
			duration_unit: Some("d".to_string()),
			continuing: Some(false),
			batch_lot_number: None,
			dosage_text: Some("500 mg twice daily".to_string()),
			dose_form: None,
			dose_form_termid: None,
			dose_form_termid_version: None,
			route_of_administration: Some("PO".to_string()),
			route_termid: None,
			route_termid_version: None,
			parent_route: None,
			parent_route_termid: None,
			parent_route_termid_version: None,
			first_administration_date_null_flavor: None,
			last_administration_date_null_flavor: None,
		},
	)
	.await?;
	DrugIndicationBmc::create(
		&ctx,
		mm,
		DrugIndicationForCreate {
			drug_id,
			sequence_number: 1,
			indication_text: Some("Bacterial sinusitis".to_string()),
			indication_meddra_version: None,
			indication_meddra_code: None,
		},
	)
	.await?;
	PrimarySourceBmc::create(
		&ctx,
		mm,
		PrimarySourceForCreate {
			case_id,
			source_reporter_presave_id: None,
			sequence_number: 1,
			reporter_title: Some("Dr".to_string()),
			reporter_given_name: Some("Mina".to_string()),
			reporter_middle_name: None,
			reporter_family_name: Some("Kim".to_string()),
			reporter_name_null_flavor: None,
			organization: Some("Seoul General Hospital".to_string()),
			department: None,
			street: None,
			city: Some("Seoul".to_string()),
			state: None,
			postcode: None,
			telephone: None,
			reporter_address_null_flavor: None,
			country_code: Some("KR".to_string()),
			email: None,
			qualification: Some("1".to_string()),
			qualification_null_flavor: None,
			qualification_kr1: None,
			primary_source_regulatory: Some("1".to_string()),
		},
	)
	.await?;
	SenderInformationBmc::create(
		&ctx,
		mm,
		SenderInformationForCreate {
			case_id,
			source_sender_presave_id: None,
			sender_type: Some("2".to_string()),
			health_professional_type_kr1: None,
			organization_name: Some("Acme Safety".to_string()),
			department: Some("Pharmacovigilance".to_string()),
			street_address: Some("1 Safety-ro".to_string()),
			city: Some("Seoul".to_string()),
			state: None,
			postcode: Some("04524".to_string()),
			country_code: Some("KR".to_string()),
			person_title: None,
			person_given_name: Some("Jun".to_string()),
			person_middle_name: None,
			person_family_name: Some("Park".to_string()),
			telephone: Some("+82-2-555-0100".to_string()),
			fax: None,
			email: Some("pv@example.test".to_string()),
		},
	)
	.await?;
	NarrativeInformationBmc::create(
		&ctx,
		mm,
		NarrativeInformationForCreate {
			case_id,
			source_narrative_presave_id: None,
			case_narrative:
				"Patient developed throat tightness and urticaria after dosing."
					.to_string(),
			reporter_comments: Some(
				"Reporter considered the reaction related.".to_string(),
			),
			sender_comments: None,
			additional_information: None,
		},
	)
	.await?;
	Ok(())
}

#[serial]
#[tokio::test]
async fn test_cioms_pdf_export_returns_pdf() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-CIOMS-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"orientation": "Landscape",
				"data_ordering": "Primary data will appear first"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/cioms.pdf"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(
		response
			.headers()
			.get("content-type")
			.and_then(|value| value.to_str().ok()),
		Some("application/pdf")
	);
	let disposition = response
		.headers()
		.get("content-disposition")
		.and_then(|value| value.to_str().ok())
		.ok_or("missing content-disposition")?;
	assert!(
		disposition.contains("attachment; filename="),
		"{disposition}"
	);
	assert!(disposition.contains("cioms"), "{disposition}");
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	assert!(bytes.starts_with(b"%PDF-"), "{bytes:?}");
	let pdf = String::from_utf8_lossy(&bytes);
	assert!(pdf.contains("/MediaBox [0 0 842 595]"), "{pdf}");
	assert!(pdf.contains("CIOMS"), "{pdf}");
	assert!(pdf.contains(&safety_report_id), "{pdf}");
	assert!(pdf.contains("Landscape"), "{pdf}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_cioms_pdf_export_renders_case_data_in_cioms_form() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-CIOMS-DATA-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;
	seed_cioms_case_data(
		&mm,
		seed.org_id,
		seed.admin.id,
		case_id,
		&safety_report_id,
	)
	.await?;

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/cioms.pdf"),
	)
	.await?;
	let status = response.status();
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&bytes)
	);
	let pdf = String::from_utf8_lossy(&bytes);

	assert!(pdf.contains("CIOMS FORM"), "{pdf}");
	assert!(pdf.contains("SUSPECT ADVERSE REACTION REPORT"), "{pdf}");
	assert!(pdf.contains("I. REACTION INFORMATION"), "{pdf}");
	assert!(pdf.contains("II. SUSPECT DRUG\\(S\\) INFORMATION"), "{pdf}");
	assert!(pdf.contains("III. CONCOMITANT DRUGS AND HISTORY"), "{pdf}");
	assert!(pdf.contains("IV. MANUFACTURER INFORMATION"), "{pdf}");
	assert!(pdf.contains("AB"), "{pdf}");
	assert!(pdf.contains("1980-03-14"), "{pdf}");
	assert!(pdf.contains("45.1 years"), "{pdf}");
	assert!(pdf.contains("Male"), "{pdf}");
	assert!(pdf.contains("Anaphylactic reaction"), "{pdf}");
	assert!(
		pdf.contains(
			"Patient developed throat tightness and urticaria after dosing."
		),
		"{pdf}"
	);
	assert!(pdf.contains("Amoxicillin capsule"), "{pdf}");
	assert!(pdf.contains("500 mg twice daily"), "{pdf}");
	assert!(pdf.contains("PO"), "{pdf}");
	assert!(pdf.contains("Bacterial sinusitis"), "{pdf}");
	assert!(pdf.contains("2026-05-01 to 2026-05-10"), "{pdf}");
	assert!(pdf.contains("10 days"), "{pdf}");
	assert!(pdf.contains("Hypertension; shellfish allergy"), "{pdf}");
	assert!(pdf.contains("Acme Safety"), "{pdf}");
	assert!(pdf.contains(&safety_report_id), "{pdf}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_cioms_pdf_export_notation_query_controls_notation() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-CIOMS-NOTATION-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;
	seed_cioms_case_data(
		&mm,
		seed.org_id,
		seed.admin.id,
		case_id,
		&safety_report_id,
	)
	.await?;

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/cioms.pdf?include_notation=false"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let pdf = String::from_utf8_lossy(&bytes);
	assert!(!pdf.contains("CIOMS NOTATION"), "{pdf}");
	assert!(
		!pdf.contains("Reporter considered the reaction related."),
		"{pdf}"
	);

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/cioms.pdf?include_notation=true"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let pdf = String::from_utf8_lossy(&bytes);
	assert!(pdf.contains("CIOMS NOTATION"), "{pdf}");
	assert!(
		pdf.contains("Reporter: Reporter considered the reaction related."),
		"{pdf}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_cioms_pdf_export_portrait_setting_changes_layout() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-CIOMS-PORTRAIT-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;
	seed_cioms_case_data(
		&mm,
		seed.org_id,
		seed.admin.id,
		case_id,
		&safety_report_id,
	)
	.await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"orientation": "Portrait",
				"data_ordering": "Primary data will appear first"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/cioms.pdf"),
	)
	.await?;
	let status = response.status();
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(
		status,
		StatusCode::OK,
		"{}",
		String::from_utf8_lossy(&bytes)
	);
	let pdf = String::from_utf8_lossy(&bytes);

	assert!(pdf.contains("/MediaBox [0 0 595 842]"), "{pdf}");
	assert!(pdf.contains("CIOMS layout: Portrait"), "{pdf}");
	assert!(pdf.contains("Anaphylactic reaction"), "{pdf}");
	assert!(pdf.contains("Amoxicillin capsule"), "{pdf}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_xml_export_comments_setting_controls_comments() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE", "0");
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-COMMENTS-{}", Uuid::new_v4());
	let raw_xml = br#"<?xml version="1.0" encoding="UTF-8"?><root><!-- element label --><case>value</case></root>"#;
	let case_id = insert_validated_raw_case_with_xml(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
		raw_xml,
	)
	.await?;

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"apply_comments_on_exported_xml": false
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let response = get_response(
		&app,
		&cookie,
		&format!(
			"/api/cases/{case_id}/export/xml?authority=ich&include_notation=true"
		),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let xml = String::from_utf8(bytes.to_vec())?;
	assert!(xml.contains("<!-- element label -->"), "{xml}");
	assert!(xml.contains("<case>value</case>"), "{xml}");

	let (status, body) = put_json(
		&app,
		&cookie,
		"/api/admin/settings",
		json!({
			"data": {
				"apply_comments_on_exported_xml": true
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let response = get_response(
		&app,
		&cookie,
		&format!(
			"/api/cases/{case_id}/export/xml?authority=ich&include_notation=false"
		),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let xml = String::from_utf8(bytes.to_vec())?;
	assert!(!xml.contains("<!-- element label -->"), "{xml}");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_single_export_uses_explicit_profile() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-APPENDIX-FDA-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=mfds"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_single_export_uses_explicit_authority() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-APPENDIX-AUTHORITY-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=mfds"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_single_export_rejects_conflicting_authority_profile() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-APPENDIX-CONFLICT-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=fda&authority=mfds"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::BAD_REQUEST);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_bulk_export_writes_one_xml_for_explicit_profile() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "0");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-APPENDIX-MULTI-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;

	let response = post_json_response(
		&app,
		&cookie,
		"/api/cases/export/xml",
		serde_json::json!({ "case_ids": [case_id], "authority": "mfds" }),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let mut zip = ZipArchive::new(Cursor::new(bytes.to_vec()))?;
	let mut names = Vec::new();
	for index in 0..zip.len() {
		names.push(zip.by_index(index)?.name().to_string());
	}
	names.sort();

	assert_eq!(
		names,
		vec![format!("{safety_report_id}-{case_id}-mfds.xml")]
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_export_history_error_details_download_as_text() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SYSTEM_ADMIN).await?;
	let safety_report_id = format!("SR-EXPORT-{case_id}");
	sqlx::query(
		"INSERT INTO cases (id, organization_id, created_by, updated_by)
		 VALUES ($1, $2, $3, $3)",
	)
	.bind(case_id)
	.bind(seed.org_id)
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO safety_report_identification (case_id, safety_report_id, version, created_by, updated_by)
		 VALUES ($1, $2, 1, $3, $3)",
	)
	.bind(case_id)
	.bind(safety_report_id)
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	let (history_id,): (Uuid,) = sqlx::query_as(
		"INSERT INTO xml_export_history (
					case_id,
					case_number,
					file_name,
					status,
					error_message,
					exported_by
				) VALUES ($1, $2, $3, $4, $5, $6)
			RETURNING id",
	)
	.bind(case_id)
	.bind("SR-EXPORT-1")
	.bind("exported-case.xml")
	.bind("error")
	.bind("gateway rejected payload")
	.bind(seed.admin.id)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;

	let (status, body) = get_json(&app, &cookie, "/api/exports/history").await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	assert!(
		body["data"]["items"]
			.as_array()
			.is_some_and(|items| !items.is_empty()),
		"{body:?}"
	);
	let items = body["data"]["items"]
		.as_array()
		.ok_or("missing export history items")?;
	assert!(
		items[0].get("validationAuthority").is_none(),
		"export history must not expose legacy validationAuthority: {:?}",
		items[0]
	);

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/exports/history/{history_id}/error.txt"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(
		response
			.headers()
			.get("content-type")
			.and_then(|v| v.to_str().ok()),
		Some("text/plain; charset=utf-8")
	);
	let disposition = response
		.headers()
		.get("content-disposition")
		.and_then(|v| v.to_str().ok())
		.ok_or("missing content-disposition header")?;
	assert!(
		disposition.contains("attachment; filename="),
		"{disposition}"
	);
	assert!(
		disposition.contains("exported-case.xml.txt"),
		"{disposition}"
	);

	let body = to_bytes(response.into_body(), usize::MAX).await?;
	assert_eq!(std::str::from_utf8(&body)?, "gateway rejected payload");

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_failed_single_export_records_error_history() -> Result<()> {
	std::env::set_var("E2BR3_EXPORT_VALIDATE_FDA", "1");
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let safety_report_id = format!("SR-EXPORT-FAIL-{}", Uuid::new_v4());
	let case_id = insert_validated_raw_case(
		&mm,
		seed.org_id,
		seed.admin.id,
		&safety_report_id,
	)
	.await?;

	let response = get_response(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/export/xml?authority=fda"),
	)
	.await?;
	assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	let response_body = to_bytes(response.into_body(), usize::MAX).await?;
	let response_text = std::str::from_utf8(&response_body)?;
	assert!(
		response_text.contains("exported XML failed"),
		"{response_text}"
	);
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SYSTEM_ADMIN).await?;
	let raw_history_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM xml_export_history WHERE case_id = $1",
	)
	.bind(case_id)
	.fetch_one(&mut *tx)
	.await?;
	tx.commit().await?;
	assert_eq!(raw_history_count, 1, "failed export was not recorded");

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/exports/history"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let item = body["data"]["items"]
		.as_array()
		.and_then(|items| items.first())
		.cloned()
		.ok_or_else(|| format!("missing failed export history item: {body}"))?;
	assert_eq!(item["status"].as_str(), Some("error"), "{item:?}");
	assert!(
		item.get("validationAuthority").is_none(),
		"failed export history must not expose legacy validationAuthority: {item:?}"
	);
	assert_eq!(
		item["fileName"].as_str(),
		Some(format!("{safety_report_id}-{case_id}-fda.xml").as_str()),
		"{item:?}"
	);
	assert!(
		item["errorMessage"]
			.as_str()
			.is_some_and(|message| message.contains("exported XML failed")),
		"{item:?}"
	);

	Ok(())
}

#[serial]
#[tokio::test]
async fn test_case_scoped_export_history_only_returns_case_rows() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());

	let case_id = Uuid::new_v4();
	let other_case_id = Uuid::new_v4();
	let mut tx = mm.dbx().db().begin().await?;
	set_user_context(&mut tx, seed.admin.id).await?;
	set_org_context(&mut tx, seed.org_id, ROLE_SYSTEM_ADMIN).await?;
	for id in [case_id, other_case_id] {
		sqlx::query(
			"INSERT INTO cases (id, organization_id, created_by, updated_by)
			 VALUES ($1, $2, $3, $3)",
		)
		.bind(id)
		.bind(seed.org_id)
		.bind(seed.admin.id)
		.execute(&mut *tx)
		.await?;
		sqlx::query(
			"INSERT INTO safety_report_identification (case_id, safety_report_id, version, created_by, updated_by)
			 VALUES ($1, $2, 1, $3, $3)",
		)
		.bind(id)
		.bind(format!("SR-EXPORT-{id}"))
		.bind(seed.admin.id)
		.execute(&mut *tx)
		.await?;
	}
	sqlx::query(
		"INSERT INTO xml_export_history (
			case_id,
			case_number,
			file_name,
			status,
			exported_by
		) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind(case_id)
	.bind("SR-EXPORT-ONE")
	.bind("one.xml")
	.bind("success")
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	sqlx::query(
		"INSERT INTO xml_export_history (
			case_id,
			case_number,
			file_name,
			status,
			exported_by
		) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind(other_case_id)
	.bind("SR-EXPORT-TWO")
	.bind("two.xml")
	.bind("success")
	.bind(seed.admin.id)
	.execute(&mut *tx)
	.await?;
	tx.commit().await?;

	let (status, body) = get_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/exports/history"),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body:?}");
	let items = body["data"]["items"]
		.as_array()
		.ok_or("missing case export history items")?;
	assert_eq!(items.len(), 1, "{body:?}");
	assert_eq!(
		items[0]["caseId"].as_str(),
		Some(case_id.to_string().as_str())
	);
	assert_eq!(items[0]["fileName"].as_str(), Some("one.xml"));
	assert!(
		items[0].get("validationAuthority").is_none(),
		"case export history must not expose legacy validationAuthority: {:?}",
		items[0]
	);

	Ok(())
}
