use crate::common::Result;
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn parse_json_or_raw(body: &[u8]) -> Value {
	let raw = String::from_utf8_lossy(body).trim().to_string();
	if raw.is_empty() {
		return json!({});
	}
	serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({ "raw": raw }))
}

pub async fn request_json(
	app: &Router,
	cookie: &str,
	method: Method,
	uri: String,
	body: Option<Value>,
) -> Result<(StatusCode, Value)> {
	let mut builder = Request::builder()
		.method(method)
		.uri(uri)
		.header("cookie", cookie);
	if body.is_some() {
		builder = builder.header("content-type", "application/json");
	}
	let req =
		builder.body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))?;
	let res = app.clone().oneshot(req).await?;
	let status = res.status();
	let bytes = to_bytes(res.into_body(), usize::MAX).await?;
	Ok((status, parse_json_or_raw(&bytes)))
}

pub async fn create_case(app: &Router, cookie: &str, org_id: Uuid) -> Result<Uuid> {
	let body = json!({
		"data": {
			"organization_id": org_id,
			"safety_report_id": format!("PS-{}", Uuid::new_v4()),
			"status": "draft",
			"appendices_json": "[\"fda\"]"
		}
	});
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		"/api/cases".to_string(),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create case failed: status={status} body={value}").into()
		);
	}
	let id = value["data"]["id"].as_str().ok_or("missing case data.id")?;
	Ok(Uuid::parse_str(id)?)
}

pub async fn create_template(
	app: &Router,
	cookie: &str,
	entity_type: &str,
	name: &str,
	data: Value,
) -> Result<(Uuid, Value)> {
	let body = json!({
		"data": {
			"entity_type": entity_type,
			"name": name,
			"description": format!("template for {entity_type}"),
			"data": data
		}
	});
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		"/api/presave-templates".to_string(),
		Some(body),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(format!(
			"create presave template {entity_type} failed: status={status} body={value}"
		)
		.into());
	}
	let id = value["data"]["id"]
		.as_str()
		.ok_or("missing template data.id")?;
	Ok((Uuid::parse_str(id)?, value["data"]["data"].clone()))
}

pub async fn get_template_data(
	app: &Router,
	cookie: &str,
	template_id: Uuid,
) -> Result<Value> {
	let (status, value) = request_json(
		app,
		cookie,
		Method::GET,
		format!("/api/presave-templates/{template_id}"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get template failed: status={status} template_id={template_id} body={value}"
		)
		.into());
	}
	Ok(value["data"]["data"].clone())
}

async fn apply_sender_presave(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	data: &Value,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		format!("/api/cases/{case_id}/safety-report/senders"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"sender_type": data["senderType"].as_str().unwrap_or("1"),
				"organization_name": data["senderOrganization"].as_str().unwrap_or(""),
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create sender failed: status={status} body={value}").into(),
		);
	}
	let id = value["data"]["id"].as_str().ok_or("missing sender id")?;
	let (status, value) = request_json(
		app,
		cookie,
		Method::PUT,
		format!("/api/cases/{case_id}/safety-report/senders/{id}"),
		Some(json!({
			"data": {
				"department": data["senderDepartment"].as_str(),
				"person_title": data["senderPersonTitle"].as_str(),
				"person_given_name": data["senderPersonGivenName"].as_str(),
				"person_middle_name": data["senderPersonMiddleName"].as_str(),
				"person_family_name": data["senderPersonFamilyName"].as_str(),
				"street_address": data["senderStreetAddress"].as_str(),
				"city": data["senderCity"].as_str(),
				"state": data["senderState"].as_str(),
				"postcode": data["senderPostcode"].as_str(),
				"country_code": data["senderCountryCode"].as_str(),
				"telephone": data["senderTelephone"].as_str(),
				"fax": data["senderFax"].as_str(),
				"email": data["senderEmail"].as_str()
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("update sender failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

async fn apply_reporter_presave(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	data: &Value,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		format!("/api/cases/{case_id}/safety-report/primary-sources"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"reporter_title": data["reporterTitle"].as_str(),
				"reporter_given_name": data["reporterGivenName"].as_str(),
				"reporter_middle_name": data["reporterMiddleName"].as_str(),
				"reporter_family_name": data["reporterFamilyName"].as_str(),
				"organization": data["reporterOrganization"].as_str(),
				"department": data["reporterDepartment"].as_str(),
				"street": data["reporterStreet"].as_str(),
				"city": data["reporterCity"].as_str(),
				"state": data["reporterState"].as_str(),
				"postcode": data["reporterPostcode"].as_str(),
				"telephone": data["reporterTelephone"].as_str(),
				"country_code": data["reporterCountry"].as_str(),
				"email": data["reporterEmail"].as_str(),
				"qualification": data["qualification"].as_str(),
				"primary_source_regulatory": data["primarySourceForRegulatoryPurposes"].as_str()
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create reporter failed: status={status} body={value}").into(),
		);
	}
	Ok(())
}

async fn apply_study_presave(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	data: &Value,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		format!("/api/cases/{case_id}/safety-report/studies"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"study_name": data["studyName"].as_str(),
				"sponsor_study_number": data["sponsorStudyNumber"].as_str(),
				"study_type_reaction": data["studyTypeReaction"].as_str()
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create study failed: status={status} body={value}").into()
		);
	}
	let study_id = value["data"]["id"].as_str().ok_or("missing study id")?;
	if let Some(reg_no) = data["studyRegistrationNumber"].as_str() {
		let (status, value) = request_json(
			app,
			cookie,
			Method::POST,
			format!("/api/cases/{case_id}/safety-report/studies/{study_id}/registrations"),
			Some(json!({
				"data": {
					"study_information_id": study_id,
					"sequence_number": 1,
					"registration_number": reg_no,
					"country_code": data["studyRegistrationCountry"].as_str()
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create study registration failed: status={status} body={value}"
			)
			.into());
		}
	}
	Ok(())
}

async fn apply_product_presave(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	data: &Value,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		format!("/api/cases/{case_id}/drugs"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": data["drugCharacterization"].as_str().unwrap_or("1"),
				"medicinal_product": data["medicinalProduct"].as_str().unwrap_or(""),
				"drug_generic_name": data["drugGenericName"].as_str()
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED {
		return Err(
			format!("create drug failed: status={status} body={value}").into()
		);
	}
	let drug_id = value["data"]["id"].as_str().ok_or("missing drug id")?;
	let (status, value) = request_json(
		app,
		cookie,
		Method::PUT,
		format!("/api/cases/{case_id}/drugs/{drug_id}"),
		Some(json!({
			"data": {
				"mpid_version": data["mpidVersion"].as_str(),
				"mpid": data["mpid"].as_str(),
				"phpid_version": data["phpidVersion"].as_str(),
				"phpid": data["phpid"].as_str(),
				"obtain_drug_country": data["obtainDrugCountry"].as_str(),
				"manufacturer_country": data["drugAuthorizationCountry"].as_str(),
				"manufacturer_name": data["drugAuthorizationHolder"].as_str(),
				"drug_authorization_number": data["drugAuthorizationNumber"].as_str(),
				"brand_name": data["drugBrandName"].as_str(),
				"drug_generic_name": data["drugGenericName"].as_str(),
				"batch_lot_number": data["drugBatchNumber"].as_str()
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(
			format!("update drug failed: status={status} body={value}").into()
		);
	}
	if let Some(substances) = data["activeSubstances"].as_array() {
		for (index, substance) in substances.iter().enumerate() {
			let (status, value) = request_json(
				app,
				cookie,
				Method::POST,
				format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
				Some(json!({
					"data": {
						"case_id": case_id,
						"drug_id": drug_id,
						"sequence_number": index + 1,
						"substance_name": substance["substanceName"].as_str(),
						"substance_termid": substance["substanceTermId"].as_str(),
						"substance_termid_version": substance["substanceTermIdVersion"].as_str(),
						"strength_value": substance["substanceStrengthValue"].as_f64(),
						"strength_unit": substance["substanceStrengthUnit"].as_str()
					}
				})),
			)
			.await?;
			if status != StatusCode::CREATED {
				return Err(format!(
					"create active substance failed: status={status} body={value}"
				)
				.into());
			}
		}
	}
	Ok(())
}

async fn apply_narrative_presave(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	data: &Value,
) -> Result<()> {
	let (status, value) = request_json(
		app,
		cookie,
		Method::POST,
		format!("/api/cases/{case_id}/narrative"),
		Some(json!({
			"data": {
				"case_id": case_id,
				"case_narrative": data["caseNarrative"].as_str().unwrap_or("")
			}
		})),
	)
	.await?;
	if status != StatusCode::CREATED && status != StatusCode::OK {
		return Err(format!(
			"create narrative failed: status={status} body={value}"
		)
		.into());
	}
	let (status, value) = request_json(
		app,
		cookie,
		Method::PUT,
		format!("/api/cases/{case_id}/narrative"),
		Some(json!({
			"data": {
				"reporter_comments": data["reporterComments"].as_str(),
				"sender_comments": data["senderComments"].as_str()
			}
		})),
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"update narrative failed: status={status} body={value}"
		)
		.into());
	}
	let (status, narrative) = request_json(
		app,
		cookie,
		Method::GET,
		format!("/api/cases/{case_id}/narrative"),
		None,
	)
	.await?;
	if status != StatusCode::OK {
		return Err(format!(
			"get narrative failed: status={status} body={narrative}"
		)
		.into());
	}
	let narrative_id = narrative["data"]["id"]
		.as_str()
		.ok_or("missing narrative id")?;
	if let Some(summary) = data["caseSummary"].as_str() {
		let (status, value) = request_json(
			app,
			cookie,
			Method::POST,
			format!("/api/cases/{case_id}/narrative/summaries"),
			Some(json!({
				"data": {
					"narrative_id": narrative_id,
					"sequence_number": 1,
					"summary_text": summary
				}
			})),
		)
		.await?;
		if status != StatusCode::CREATED {
			return Err(format!(
				"create case summary failed: status={status} body={value}"
			)
			.into());
		}
	}
	if let Some(diagnoses) = data["senderDiagnoses"].as_array() {
		for (index, diagnosis) in diagnoses.iter().enumerate() {
			let (status, value) = request_json(
				app,
				cookie,
				Method::POST,
				format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
				Some(json!({
					"data": {
						"narrative_id": narrative_id,
						"sequence_number": index + 1,
						"diagnosis_meddra_code": diagnosis["diagnosisMeddraCode"].as_str(),
						"diagnosis_meddra_version": diagnosis["diagnosisMeddraVersion"].as_str()
					}
				})),
			)
			.await?;
			if status != StatusCode::CREATED {
				return Err(format!(
					"create sender diagnosis failed: status={status} body={value}"
				)
				.into());
			}
			let diagnosis_id =
				value["data"]["id"].as_str().ok_or("missing diagnosis id")?;
			let (status, value) = request_json(
				app,
				cookie,
				Method::PUT,
				format!(
					"/api/cases/{case_id}/narrative/sender-diagnoses/{diagnosis_id}"
				),
				Some(json!({
					"data": {
						"diagnosis_meddra_code": diagnosis["diagnosisMeddraCode"].as_str(),
						"diagnosis_meddra_version": diagnosis["diagnosisMeddraVersion"].as_str()
					}
				})),
			)
			.await?;
			if status != StatusCode::OK {
				return Err(format!(
					"update sender diagnosis failed: status={status} body={value}"
				)
				.into());
			}
		}
	}
	Ok(())
}

pub async fn apply_authoring_presave(
	app: &Router,
	cookie: &str,
	case_id: Uuid,
	entity_type: &str,
	data: &Value,
) -> Result<()> {
	match entity_type {
		"sender" => apply_sender_presave(app, cookie, case_id, data).await,
		"reporter" => apply_reporter_presave(app, cookie, case_id, data).await,
		"study" => apply_study_presave(app, cookie, case_id, data).await,
		"product" => apply_product_presave(app, cookie, case_id, data).await,
		"narrative" => apply_narrative_presave(app, cookie, case_id, data).await,
		other => {
			Err(format!("unsupported authoring presave entity_type: {other}").into())
		}
	}
}
