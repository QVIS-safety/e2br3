//! Persisted field -> unchanged exporter bytes -> different organization -> persisted field.
//! Runs only in the isolated test database; fixtures are not production evidence.
use super::fields::{build_path, contract_document, merge_json};
use super::support::{
	create_case_for_editor, create_reaction_contract_fixture, get_json,
	repeatable_row_id,
};
use crate::common::{
	cookie_header, create_test_import_product, init_test_mm, seed_org_with_users,
	Result,
};
use axum::{
	body::{to_bytes, Body},
	http::{Request, StatusCode},
};
use lib_auth::token::generate_web_token;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone)]
struct Transfer {
	app: axum::Router,
	source_cookie: String,
	target_cookie: String,
	source_org: Uuid,
	target_org: Uuid,
	product: Uuid,
}

async fn request(
	app: &axum::Router,
	cookie: &str,
	method: &str,
	uri: &str,
	body: Value,
) -> Result<Value> {
	let response = app
		.clone()
		.oneshot(
			Request::builder()
				.method(method)
				.uri(uri)
				.header("cookie", cookie)
				.header("content-type", "application/json")
				.header(
					"x-e2br3-reason-for-change",
					"Isolated XML dictionary roundtrip",
				)
				.body(Body::from(body.to_string()))?,
		)
		.await?;
	let status = response.status();
	let bytes = to_bytes(response.into_body(), usize::MAX).await?;
	let value: Value = serde_json::from_slice(&bytes)?;
	if !status.is_success() {
		return Err(format!("{method} {uri}: {status}: {value}").into());
	}
	Ok(value)
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
	value
		.get(key)
		.and_then(Value::as_str)
		.ok_or_else(|| format!("missing string {key}").into())
}

// Preserve every repeated row and its grouping, including an absent/missing leaf distinction.
fn project(value: &Value, path: &str) -> Result<Value> {
	let (head, tail) = match path.split_once('.') {
		Some((h, t)) => (h, Some(t)),
		None => (path, None),
	};
	let array = head.ends_with("[]");
	let key = head.trim_end_matches("[]");
	let child = if key.is_empty() {
		value
	} else {
		value
			.get(key)
			.ok_or_else(|| format!("missing projection {path}"))?
	};
	if array {
		child
			.as_array()
			.ok_or("invalid collection projection")?
			.iter()
			.map(|row| match tail {
				Some(rest) => project(row, rest),
				None => Ok(row.clone()),
			})
			.collect::<Result<Vec<_>>>()
			.map(Value::Array)
	} else {
		match tail {
			Some(rest) => project(child, rest),
			None => Ok(child.clone()),
		}
	}
}

#[test]
fn projection_preserves_all_rows_and_rejects_missing_leaves() -> Result<()> {
	let rows = json!({"rows":[{"children":[{"value":false},{"value":0}]},{"children":[{"value":null},{"value":""}]}]});
	assert_eq!(
		project(&rows, "rows[].children[].value")?,
		json!([[false, 0], [null, ""]])
	);
	assert!(project(&json!({"rows":[{}]}), "rows[].value").is_err());
	assert!(project(&json!({"rows":null}), "rows[].value").is_err());
	assert_ne!(
		project(&rows, "rows[].children[].value")?,
		json!([[false], [0, null, ""]])
	);
	Ok(())
}

impl Transfer {
	async fn projection(
		&self,
		cookie: &str,
		case: &str,
		page: &str,
	) -> Result<Value> {
		if page == "N" {
			let (status, body) = get_json(
				&self.app,
				cookie,
				&format!("/api/cases/{case}/message-header"),
			)
			.await?;
			if status != StatusCode::OK {
				return Err(format!("header read {status}: {body}").into());
			}
			return Ok(
				json!({"messageHeader": body.get("data").ok_or("missing header")?}),
			);
		}
		let base = format!("/api/cases/{case}/editor/pages/{page}");
		let (status, body) = get_json(&self.app, cookie, &base).await?;
		if status != StatusCode::OK {
			return Err(format!("page read {status}: {body}").into());
		}
		if ["AE", "DG", "LR", "LB", "DH"].contains(&page) {
			let rows = body["rows"]["rows"].as_array().ok_or("missing row list")?;
			let mut details = Vec::new();
			for row in rows {
				let id = string(row, "id")?;
				let (status, detail) =
					get_json(&self.app, cookie, &format!("{base}/rows/{id}"))
						.await?;
				if status != StatusCode::OK {
					return Err(format!("detail read {status}: {detail}").into());
				}
				details
					.push(detail.get("data").ok_or("missing detail data")?.clone());
			}
			Ok(Value::Array(details))
		} else {
			Ok(body.get("rows").ok_or("missing page rows")?.clone())
		}
	}

	async fn roundtrip(
		&self,
		contract: Value,
		field: Value,
		authority: String,
		flavor: Option<String>,
		evidence: &mut Value,
	) -> Result<()> {
		let page = string(&contract, "pageId")?;
		let case = create_case_for_editor(
			&self.app,
			&self.source_cookie,
			"XML-FIELD",
			&[&authority],
		)
		.await?;
		let header_uri = format!("/api/cases/{case}/message-header");
		request(&self.app, &self.source_cookie, "POST", &header_uri, json!({"data":{
            "case_id":case, "message_number":format!("MSG-{case}"), "message_date":"20240229123059.1234+0900",
            "message_sender_identifier":"SOURCE", "message_receiver_identifier":"TARGET",
            "batch_sender_identifier":"SOURCE", "batch_receiver_identifier":"TARGET"
        }})).await?;
		let path = string(&field, "projectionPath")?;
		let mut expected = field
			.get("roundTripValue")
			.ok_or("missing fixture value")?
			.clone();
		if field["code"] == "C.1.1" {
			expected = json!(format!(
				"{}-{authority}",
				expected.as_str().ok_or("report ID fixture must be text")?
			));
		}
		let mut partner = None;
		if let Some(code) =
			field.get("nullFlavorPartnerCode").and_then(Value::as_str)
		{
			partner = Some(
				contract["fields"]
					.as_array()
					.ok_or("missing fields")?
					.iter()
					.find(|candidate| candidate["code"] == code)
					.ok_or("missing nullFlavor partner")?,
			);
		}
		if page == "N" {
			let column = path
				.strip_prefix("messageHeader.")
				.ok_or("invalid header path")?;
			if column == "batch_transmission_date" {
				// Public saves accept typed timestamps; import preserves the exact exported lexeme.
				expected = json!([2024, 60, 12, 30, 59, 123400000, 0, 0, 0]);
				request(
					&self.app,
					&self.source_cookie,
					"PUT",
					&header_uri,
					json!({"data":{"batch_transmission_date":expected}}),
				)
				.await?;
			} else if column != "message_type" {
				request(
					&self.app,
					&self.source_cookie,
					"PUT",
					&header_uri,
					json!({"data":{column:expected}}),
				)
				.await?;
			}
		} else {
			let row_id = if page == "LB" {
				// Result choices must start empty; a numeric fixture would mask unstructured text.
				let created = request(&self.app, &self.source_cookie, "POST", &format!("/api/cases/{case}/editor/pages/LB/rows"), json!({"authorities":[authority],"rows":{"testResult":{"sequenceNumber":1,"testName":"Fixture test"}}})).await?;
				Some(string(&created, "rowId")?.to_owned())
			} else {
				repeatable_row_id(page, &self.app, &self.source_cookie, &case)
					.await?
			};
			if field.get("roundTripReference").is_some() {
				if field["roundTripReference"] != "case-reaction" {
					return Err("unsupported XML reference fixture".into());
				}
				expected = json!(
					create_reaction_contract_fixture(
						&self.app,
						&self.source_cookie,
						&case
					)
					.await?
				);
			}
			let owner = string(&field["patch"], "owner")?;
			let mut row = match field.get("fixture") {
				Some(value) => value.clone(),
				None => json!({}),
			};
			merge_json(
				&mut row,
				build_path(
					string(&field, "payloadPath")?,
					if flavor.is_some() {
						Value::Null
					} else {
						expected.clone()
					},
				),
			);
			if let Some(nf) = &flavor {
				let nf_field = partner.ok_or(
					"dictionary allows nullFlavor but no storage partner exists",
				)?;
				merge_json(
					&mut row,
					build_path(string(nf_field, "payloadPath")?, json!(nf)),
				);
			} else if let Some(nf_field) = partner {
				merge_json(
					&mut row,
					build_path(string(nf_field, "payloadPath")?, Value::Null),
				);
			}
			if let Some(assessments) = row
				.get_mut("drugReactionAssessments")
				.and_then(Value::as_array_mut)
			{
				let reaction = if field["roundTripReference"] == "case-reaction" {
					expected.clone()
				} else {
					json!(
						create_reaction_contract_fixture(
							&self.app,
							&self.source_cookie,
							&case
						)
						.await?
					)
				};
				for assessment in assessments {
					assessment["reactionId"] = reaction.clone();
				}
			}
			let mut rows =
				match contract.get("rowPrerequisites").and_then(|v| v.get(owner)) {
					Some(v) => v.clone(),
					None => json!({}),
				};
			if let Some(fixture) = field.get("rowsFixture") {
				merge_json(&mut rows, fixture.clone());
			}
			merge_json(&mut rows, json!({owner: row}));
			if field["code"] == "C.1.10.r" {
				let second_value = json!(format!(
					"{}-SECOND",
					expected
						.as_str()
						.ok_or("linked report fixture must be text")?
				));
				let links = rows[owner]
					.as_array_mut()
					.ok_or("missing linked report rows")?;
				let mut second =
					links.first().ok_or("missing first linked report")?.clone();
				second["sequenceNumber"] = json!(2);
				second["linkedReportNumber"] = second_value.clone();
				links.push(second);
				expected = json!([expected, second_value]);
			}
			let uri = match row_id {
				Some(id) => {
					format!("/api/cases/{case}/editor/pages/{page}/rows/{id}")
				}
				None => format!("/api/cases/{case}/editor/pages/{page}"),
			};
			request(
				&self.app,
				&self.source_cookie,
				"PATCH",
				&uri,
				json!({"authorities":[authority],"rows":rows}),
			)
			.await?;
		}
		let before = self.projection(&self.source_cookie, &case, page).await?;
		let value_path = if ["AE", "DG", "LR", "LB", "DH"].contains(&page) {
			format!("[].{path}")
		} else {
			path.to_string()
		};
		let actual_path = &value_path;
		evidence["source"] = before.clone();
		evidence["projectionPath"] = json!(actual_path);
		let source_value = project(&before, actual_path)?;
		let expected = if flavor.is_some() {
			if field["roundTripValue"].is_array() {
				json!([])
			} else {
				Value::Null
			}
		} else {
			match field.get("projectionValue") {
				Some(v) => v.clone(),
				None => expected,
			}
		};
		fn leaves(value: &Value, expected: &Value) -> bool {
			if value == expected {
				return true;
			}
			match value {
				Value::Array(rows) => {
					!rows.is_empty() && rows.iter().all(|row| leaves(row, expected))
				}
				value => value == expected,
			}
		}
		if !leaves(&source_value, &expected) {
			return Err(format!(
				"fixture did not persist: expected {expected}, got {source_value}"
			)
			.into());
		}
		let response = self.app.clone().oneshot(Request::builder()
            .uri(format!("/api/cases/{case}/export/xml?authority={authority}&include_notation=false"))
            .header("cookie", &self.source_cookie).body(Body::empty())?).await?;
		let status = response.status();
		let bytes = to_bytes(response.into_body(), usize::MAX).await?;
		evidence["exportStatus"] = json!(status.as_u16());
		evidence["exportBody"] = json!(std::str::from_utf8(&bytes)?);
		if status != StatusCode::OK {
			return Err(format!(
				"export {status}: {}",
				String::from_utf8_lossy(&bytes)
			)
			.into());
		}
		let xml = std::str::from_utf8(&bytes)?;
		let boundary = "XML-FIELD-BOUNDARY";
		let payload = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\n{authority}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"productPresaveId\"\r\n\r\n{}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"field.xml\"\r\nContent-Type: application/xml\r\n\r\n{xml}\r\n--{boundary}--\r\n", self.product);
		let response = self
			.app
			.clone()
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/api/import/xml")
					.header("cookie", &self.target_cookie)
					.header(
						"content-type",
						format!("multipart/form-data; boundary={boundary}"),
					)
					.body(Body::from(payload))?,
			)
			.await?;
		let status = response.status();
		let imported: Value = serde_json::from_slice(
			&to_bytes(response.into_body(), usize::MAX).await?,
		)?;
		evidence["importStatus"] = json!(status.as_u16());
		evidence["importResponse"] = imported.clone();
		if status != StatusCode::OK {
			return Err(format!("import {status}: {imported}").into());
		}
		let cases = imported["data"]["importedCases"]
			.as_array()
			.ok_or("missing import summaries")?;
		if cases.len() != 1 || cases[0]["status"] != "success" {
			return Err(format!("import did not create a case: {imported}").into());
		}
		let target = string(&cases[0], "caseId")?;
		if target == case {
			return Err("import reused source case".into());
		}
		for (cookie, id, org) in [
			(&self.source_cookie, case.as_str(), self.source_org),
			(&self.target_cookie, target, self.target_org),
		] {
			let (status, body) =
				get_json(&self.app, cookie, &format!("/api/cases/{id}")).await?;
			if status != StatusCode::OK
				|| body["data"]["organization_id"] != json!(org)
			{
				return Err(format!("wrong organization: {body}").into());
			}
		}
		let after = self.projection(&self.target_cookie, target, page).await?;
		evidence["target"] = after.clone();
		if page == "N" && path == "messageHeader.batch_transmission_date" {
			let raw = project(&after, "messageHeader.batch_transmission_date_raw")?;
			if raw != json!("20240229123059.1234+0000")
				|| project(&after, path)? != Value::Null
			{
				return Err(format!("batch timestamp lexeme changed: {raw}").into());
			}
		} else {
			let target_value = project(&after, actual_path)?;
			if field["roundTripReference"] == "case-reaction" {
				let reactions =
					self.projection(&self.target_cookie, target, "AE").await?;
				let ids = project(&reactions, "[].reaction.id")?;
				if target_value != json!([ids]) || target_value == source_value {
					return Err(format!(
						"reaction relationship not remapped: {target_value}"
					)
					.into());
				}
			} else if source_value != target_value {
				return Err(format!(
					"value changed: {source_value} -> {target_value}"
				)
				.into());
			}
		}

		if let Some(nf_field) = partner {
			let nf_path = string(nf_field, "projectionPath")?;
			let nf_path = if ["AE", "DG", "LR", "LB", "DH"].contains(&page) {
				format!("[].{nf_path}")
			} else {
				nf_path.to_owned()
			};
			let before_nf = project(&before, &nf_path)?;
			if !leaves(&before_nf, &json!(flavor)) {
				return Err(format!(
					"nullFlavor fixture did not persist: {before_nf}"
				)
				.into());
			}
			let after_nf = project(&after, &nf_path)?;
			if before_nf != after_nf {
				return Err(format!(
					"nullFlavor changed: {before_nf} -> {after_nf}"
				)
				.into());
			}
		}
		if self.projection(&self.source_cookie, &case, page).await? != before {
			return Err("source changed during transfer".into());
		}
		Ok(())
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn all_dictionary_fields_cross_organization() -> Result<()> {
	let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
	let contracts = [
		"N", "CI", "RP", "SD", "SI", "DM", "NR", "AE", "DG", "LR", "LB", "DH",
	]
	.into_iter()
	.map(contract_document)
	.collect::<Result<Vec<_>>>()?;
	let mm = init_test_mm().await?;
	let source = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let target = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	assert_ne!(source.org_id, target.org_id);
	for seed in [&source, &target] {
		mm.dbx().begin_txn().await?;
		lib_core::model::store::set_full_context_dbx(
			mm.dbx(),
			seed.admin.id,
			seed.org_id,
			lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO,
		)
		.await?;
		mm.dbx()
			.execute(
				sqlx::query(
					"UPDATE users SET access_blind_allowed = true WHERE id = $1",
				)
				.bind(seed.admin.id),
			)
			.await?;
		mm.dbx().commit_txn().await?;
	}
	let transfer = Transfer {
		app: web_server::app(mm.clone())?,
		source_cookie: cookie_header(
			&generate_web_token(&source.admin.email, source.admin.token_salt)?
				.to_string(),
		),
		target_cookie: cookie_header(
			&generate_web_token(&target.admin.email, target.admin.token_salt)?
				.to_string(),
		),
		source_org: source.org_id,
		target_org: target.org_id,
		product: create_test_import_product(&mm, target.org_id, target.admin.id)
			.await?,
	};
	// Preservation is tested with the target organization's explicit overwrite settings disabled.
	request(&transfer.app, &transfer.target_cookie, "PUT", "/api/admin/settings", json!({"data":{
		"apply_sender_info_to_imported_cases":false,
		"import_date_update":{"date_of_creation":false,"most_recent_info_date":false,"report_first_received_date":false}
	}})).await?;
	let mut results = BTreeMap::new();
	for (dictionary, authority) in [
		("ich-e2br3", "ich"),
		("ich-e2br3", "fda"),
		("ich-e2br3", "mfds"),
		("fda-regional", "fda"),
		("mfds-regional", "mfds"),
	] {
		let document: Value = serde_json::from_str(&std::fs::read_to_string(
			root.join(format!("registry/dictionary/{dictionary}.json")),
		)?)?;
		for element in document["entries"]
			.as_array()
			.ok_or("missing dictionary entries")?
			.iter()
			.filter(|entry| entry["kind"] == "element")
		{
			let code = string(element, "code")?;
			let binding = if code == "FDA.D.1" { "D.1" } else { code };
			let matches = contracts
				.iter()
				.flat_map(|contract| {
					contract["fields"]
						.as_array()
						.into_iter()
						.flatten()
						.filter(move |field| field["code"] == binding)
						.map(move |field| (contract.clone(), field.clone()))
				})
				.collect::<Vec<_>>();
			let mut scenarios = vec![None];
			if let Some(flavors) = element.get("null_flavors") {
				for nf in flavors
					.as_array()
					.ok_or("invalid dictionary null flavors")?
				{
					scenarios.push(Some(
						nf.as_str().ok_or("invalid null flavor")?.to_owned(),
					));
				}
			}
			for flavor in scenarios {
				let label = format!(
					"{authority}/{dictionary}/{code}/{}",
					match &flavor {
						Some(nf) => nf.as_str(),
						None => "value",
					}
				);
				let spec = if code == "N.1.1" {
					Some((
						json!({"pageId":"N","fields":[]}),
						json!({"code":code,"projectionPath":"messageHeader.message_type","roundTripValue":"ichicsr"}),
					))
				} else if matches.len() == 1 {
					Some(matches[0].clone())
				} else {
					None
				};
				let result = match spec {
					Some((contract, field)) => {
						let transfer = transfer.clone();
						let authority = authority.to_owned();
						match tokio::spawn(async move {
							let mut evidence = json!({});
							let result = transfer
								.roundtrip(contract, field, authority, flavor, &mut evidence)
								.await;
							if result.is_err() {
								if let Some(xml) = evidence["exportBody"].as_str() {
									if evidence["exportStatus"] == 200 {
										let bytes = xml.as_bytes();
										evidence["parser"] = json!({
											"patient": format!("{:#?}", xml::import_sections::d_patient::parse_d_patient(bytes)),
											"reactions": format!("{:#?}", xml::import_sections::e_reaction::parse_e_reactions(bytes)),
											"tests": format!("{:#?}", xml::import_sections::f_test_result::parse_f_test_results(bytes)),
											"drugs": format!("{:#?}", xml::import_sections::g_drug::parse_g_drugs(bytes))
										});
									}
								}
							}
							(result, evidence)
						})
						.await
						{
							Ok((Ok(()), _)) => json!({"status":"passed"}),
							Ok((Err(error), evidence)) => {
								json!({"status":"failed","error":error.to_string(),"evidence":evidence})
							}
							Err(error) => {
								json!({"status":"failed","error":error.to_string()})
							}
						}
					}
					None => {
						json!({"status":"failed","error":format!("expected one storage binding, found {}",matches.len())})
					}
				};
				eprintln!(
					"XML_FIELD {label}: status={}, error={}",
					result["status"], result["error"]
				);
				assert!(
					results.insert(label, result).is_none(),
					"duplicate dictionary scenario"
				);
			}
		}
	}
	if let Ok(path) = std::env::var("E2BR3_XML_FIELD_REPORT") {
		let file = std::fs::OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(path)?;
		serde_json::to_writer_pretty(
			file,
			&json!({"scope":"isolated DB fixtures; full dictionary value and allowed nullFlavor scenarios","results":results}),
		)?;
	}
	let failed = results
		.iter()
		.filter(|(_, result)| result["status"] != "passed")
		.collect::<Vec<_>>();
	assert!(
		failed.is_empty(),
		"{} of {} XML dictionary scenarios failed: {failed:?}",
		failed.len(),
		results.len()
	);
	Ok(())
}
