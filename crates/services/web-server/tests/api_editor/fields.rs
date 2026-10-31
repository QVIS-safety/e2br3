pub(super) mod coupled_clear;
pub(super) mod null_flavor;
pub(super) mod required_values;
use super::support::{
	create_case_for_editor, create_reaction_contract_fixture, get_json,
	patch_json_with_reason, repeatable_row_id,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::{Ctx, ROLE_SPONSOR_ADMIN_CRO};
use lib_core::model::store::set_full_context_dbx;
use lib_core::model::ModelManager;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

async fn case_status(
	mm: &ModelManager,
	user_id: Uuid,
	org_id: Uuid,
	case_id: &str,
) -> Result<String> {
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(mm.dbx(), user_id, org_id, ROLE_SPONSOR_ADMIN_CRO).await?;
	let status = mm
		.dbx()
		.fetch_one(
			sqlx::query_as::<_, (String,)>("SELECT status FROM cases WHERE id = $1")
				.bind(Uuid::parse_str(case_id)?),
		)
		.await?
		.0;
	mm.dbx().commit_txn().await?;
	Ok(status)
}

pub struct FieldSpec {
	pub page_id: &'static str,
	pub code: &'static str,
}

pub(super) fn contract_document(page_id: &str) -> Result<Value> {
	let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
	let path = root
		.join("registry/editor-contracts")
		.join(format!("{}.json", page_id.to_lowercase()));
	Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn contract_field(contract: &Value, page_id: &str, code: &str) -> Result<Value> {
	contract["fields"]
		.as_array()
		.and_then(|fields| fields.iter().find(|field| field["code"] == code))
		.cloned()
		.ok_or_else(|| format!("missing editor contract {page_id}/{code}").into())
}

pub(super) fn build_path(path: &str, value: Value) -> Value {
	fn build(segments: &[&str], value: Value) -> Value {
		if segments.is_empty() {
			return value;
		}
		let segment = segments[0];
		if segment == "[]" {
			let mut child = build(&segments[1..], value);
			if let Some(object) = child.as_object_mut() {
				object.insert("sequenceNumber".to_string(), json!(1));
			}
			return Value::Array(vec![child]);
		}
		if let Some(key) = segment.strip_suffix("[]") {
			let mut child = build(&segments[1..], value);
			if let Some(object) = child.as_object_mut() {
				object.insert("sequenceNumber".to_string(), json!(1));
			}
			return Value::Object(Map::from_iter([(
				key.to_string(),
				Value::Array(vec![child]),
			)]));
		}
		Value::Object(Map::from_iter([(
			segment.to_string(),
			build(&segments[1..], value),
		)]))
	}

	build(&path.split('.').collect::<Vec<_>>(), value)
}

pub(super) fn read_path<'a>(mut value: &'a Value, path: &str) -> Option<&'a Value> {
	for segment in path.split('.') {
		if segment == "[]" {
			value = value.as_array()?.first()?;
		} else if let Some(key) = segment.strip_suffix("[]") {
			value = value.get(key)?.as_array()?.first()?;
		} else {
			value = value.get(segment)?;
		}
	}
	Some(value)
}

pub(super) fn merge_json(base: &mut Value, overlay: Value) {
	match (base, overlay) {
		(Value::Object(base), Value::Object(overlay)) => {
			for (key, value) in overlay {
				merge_json(base.entry(key).or_insert(Value::Null), value);
			}
		}
		(Value::Array(base), Value::Array(overlay)) => {
			for (index, value) in overlay.into_iter().enumerate() {
				if index < base.len() {
					merge_json(&mut base[index], value);
				} else {
					base.push(value);
				}
			}
		}
		(base, overlay) => *base = overlay,
	}
}

pub async fn verify_field(spec: FieldSpec) -> Result<()> {
	let contract = contract_document(spec.page_id)?;
	let field = contract_field(&contract, spec.page_id, spec.code)?;
	let owner = field["patch"]["owner"]
		.as_str()
		.ok_or("missing patch owner")?;
	let payload_path = field["payloadPath"].as_str().ok_or("missing payloadPath")?;
	let projection_path = field["projectionPath"]
		.as_str()
		.ok_or("missing projectionPath")?;
	let round_trip = field["roundTripValue"].clone();
	let projection_value = field
		.get("projectionValue")
		.cloned()
		.unwrap_or_else(|| round_trip.clone());
	let authority = field["authority"]
		.as_str()
		.ok_or("missing authority")?
		.to_lowercase();

	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
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
				"UPDATE users SET access_blind_allowed = true WHERE id = $1",
			)
			.bind(seed.admin.id),
		)
		.await?;
	mm.dbx().commit_txn().await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-{}-{}", spec.page_id, Uuid::new_v4()),
		&[&authority],
	)
	.await?;
	let sentinel_status =
		case_status(&mm, seed.admin.id, seed.org_id, &case_id).await?;
	let row_id = repeatable_row_id(spec.page_id, &app, &cookie, &case_id).await?;
	let mut row = field.get("fixture").cloned().unwrap_or_else(|| json!({}));
	merge_json(&mut row, build_path(payload_path, round_trip.clone()));
	if let Some(assessments) = row
		.get_mut("drugReactionAssessments")
		.and_then(Value::as_array_mut)
	{
		let reaction_id =
			create_reaction_contract_fixture(&app, &cookie, &case_id).await?;
		for assessment in assessments {
			assessment
				.as_object_mut()
				.ok_or("drug reaction assessment fixture must be an object")?
				.insert("reactionId".to_string(), json!(reaction_id));
		}
	}
	let mut rows = contract
		.get("rowPrerequisites")
		.and_then(|fixtures| fixtures.get(owner))
		.cloned()
		.unwrap_or_else(|| json!({}));
	if let Some(field_fixture) = field.get("rowsFixture").cloned() {
		merge_json(&mut rows, field_fixture);
	}
	merge_json(&mut rows, json!({ owner: row }));
	let uri = row_id.as_ref().map_or_else(
		|| format!("/api/cases/{case_id}/editor/pages/{}", spec.page_id),
		|row_id| {
			format!(
				"/api/cases/{case_id}/editor/pages/{}/rows/{row_id}",
				spec.page_id
			)
		},
	);
	let ctx = Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let case = FieldCase {
		mm: &mm,
		ctx: &ctx,
		app: &app,
		cookie: &cookie,
		uri: &uri,
		case_id: Uuid::parse_str(&case_id)?,
		root: if row_id.is_some() { "data" } else { "rows" },
		authority: &authority,
		field: &field,
		identities: &contract["rowIdentities"],
	};
	let mut projection = case.reload().await?;
	case.check_patch(
		rows,
		&json!({"effect": "store", "value": projection_value}),
		"save",
		&mut projection,
	)
	.await?;
	let request_path = format!("{owner}.{payload_path}");
	for operation in ["omission", "clear", "blank"] {
		let outcome = if operation == "omission"
			&& field["persistence"]["omission"] == "preserve"
		{
			json!({"effect": "store", "value": projection_value})
		} else {
			field["persistence"][operation].clone()
		};
		if outcome["effect"] == "not_applicable" {
			assert!(outcome["reason"]
				.as_str()
				.is_some_and(|reason| !reason.is_empty()));
			continue;
		}
		// Each boundary starts from the saved value, including blank after null.
		if operation != "omission" {
			let mut restore = build_path(&request_path, round_trip.clone());
			pin_row_ids(
				&mut restore,
				&request_path,
				&projection,
				projection_path,
				&contract["rowIdentities"],
			)?;
			case.check_patch(
				restore,
				&json!({"effect":"store", "value":projection_value}),
				"set",
				&mut projection,
			)
			.await?;
		}
		let mut update = build_path(&request_path, outcome["input"].clone());
		pin_row_ids(
			&mut update,
			&request_path,
			&projection,
			projection_path,
			&contract["rowIdentities"],
		)?;
		if operation == "omission" {
			remove_leaf(&mut update, &request_path)?;
		}
		case.check_patch(update, &outcome, operation, &mut projection)
			.await?;
	}
	if let Some(invalid_value) = field["constraint"].get("invalidValue") {
		let invalid_path = match field["constraint"]["ruleCode"].as_str() {
			Some(rule)
				if rule.ends_with(".NULLFLAVOR.ALLOWED")
					&& !payload_path.ends_with("NullFlavor") =>
			{
				format!("{request_path}NullFlavor")
			}
			_ => request_path.clone(),
		};
		let mut invalid = build_path(&invalid_path, invalid_value.clone());
		pin_row_ids(
			&mut invalid,
			&request_path,
			&projection,
			projection_path,
			&contract["rowIdentities"],
		)?;
		case.check_patch(invalid, &json!({"effect":"reject", "status":422, "ruleCode":field["constraint"]["ruleCode"], "issuePath":field["constraint"]["issuePath"]}), "invalid", &mut projection).await?;
	}
	assert_eq!(
		case_status(&mm, seed.admin.id, seed.org_id, &case_id).await?,
		sentinel_status,
		"{}/{} changed unrelated case status",
		spec.page_id,
		spec.code
	);
	Ok(())
}

struct FieldCase<'a> {
	mm: &'a ModelManager,
	ctx: &'a Ctx,
	app: &'a axum::Router,
	cookie: &'a str,
	uri: &'a str,
	case_id: Uuid,
	root: &'a str,
	authority: &'a str,
	field: &'a Value,
	identities: &'a Value,
}

impl FieldCase<'_> {
	async fn reload(&self) -> Result<Value> {
		let (status, body) = get_json(self.app, self.cookie, self.uri).await?;
		assert_eq!(status, StatusCode::OK, "{body}");
		body.get(self.root)
			.cloned()
			.ok_or_else(|| "missing editor projection root".into())
	}

	async fn check_patch(
		&self,
		rows: Value,
		outcome: &Value,
		operation: &str,
		projection: &mut Value,
	) -> Result<()> {
		let audit_spec = &self.field["persistence"]["audit"];
		let before = super::audit::capture(
			self.mm,
			self.ctx,
			self.case_id,
			audit_spec,
			projection,
		)
		.await?;
		let reason = format!(
			"editor-contract:{}:{operation}:{}",
			self.field["code"],
			Uuid::new_v4()
		);
		let (status, body) = patch_json_with_reason(
			self.app,
			self.cookie,
			self.uri,
			json!({"authorities":[self.authority], "rows":rows}),
			Some(&reason),
		)
		.await?;
		let after = self.reload().await?;
		let label = format!("{} {operation}: {body}", self.field["code"]);
		match outcome["effect"].as_str() {
			Some("store") => {
				assert_eq!(status, StatusCode::OK, "{label}");
				let path = self.field["projectionPath"]
					.as_str()
					.ok_or("missing projectionPath")?;
				assert_eq!(
					read_path(&after, path).ok_or("missing saved field")?,
					outcome.get("value").ok_or("missing expected value")?,
					"{label}"
				);
				for collection in array_prefixes(path) {
					if let Some(previous) = read_path(projection, &collection) {
						let current = read_path(&after, &collection)
							.ok_or("saved collection disappeared")?;
						for key in self.identities[&collection]["keys"]
							.as_array()
							.ok_or("missing identity keys")?
						{
							let name = key["name"]
								.as_str()
								.ok_or("missing identity key name")?;
							assert_eq!(
								current.get(name),
								previous.get(name),
								"{label}: changed {collection}.{name}"
							);
						}
					}
				}
				super::audit::assert_mutation(
					self.mm,
					self.ctx,
					self.case_id,
					audit_spec,
					before,
					&after,
					&reason,
				)
				.await?;
			}
			Some("reject") => {
				assert_eq!(
					status.as_u16() as u64,
					outcome["status"]
						.as_u64()
						.ok_or("missing rejection status")?,
					"{label}"
				);
				if let Some(rule) = outcome["ruleCode"].as_str() {
					assert_eq!(
						body["error"]["message"], "CONSTRAINT_VIOLATION",
						"{label}"
					);
					assert_eq!(
						body["error"]["data"]["detail"]["ruleCode"], rule,
						"{label}"
					);
				} else {
					assert_eq!(
						body["error"]["message"],
						outcome["error"]
							.as_str()
							.ok_or("missing rejection error")?,
						"{label}"
					);
					assert_eq!(
						body["error"]["data"]["detail"],
						outcome
							.get("detail")
							.ok_or("missing rejection detail")?
							.clone(),
						"{label}"
					);
				}

				if let Some(path) = outcome["issuePath"].as_str() {
					assert_eq!(
						body["error"]["data"]["detail"]["path"],
						path.replace("[]", ".0"),
						"{label}"
					);
				}
				assert_eq!(
					&after, &*projection,
					"rejection changed persisted editor data: {label}"
				);
				super::audit::assert_rejected(
					self.mm,
					self.ctx,
					self.case_id,
					before,
				)
				.await?;
			}
			_ => return Err(format!("missing explicit outcome for {label}").into()),
		}
		*projection = after;
		Ok(())
	}
}

fn array_prefixes(path: &str) -> Vec<String> {
	let segments: Vec<_> = path.split('.').collect();
	segments
		.iter()
		.enumerate()
		.filter(|(_, segment)| segment.ends_with("[]"))
		.map(|(index, _)| segments[..=index].join("."))
		.collect()
}

fn read_path_mut<'a>(mut value: &'a mut Value, path: &str) -> Option<&'a mut Value> {
	for segment in path.split('.') {
		if segment == "[]" {
			value = value.as_array_mut()?.first_mut()?;
		} else if let Some(key) = segment.strip_suffix("[]") {
			value = value.get_mut(key)?.as_array_mut()?.first_mut()?;
		} else {
			value = value.get_mut(segment)?;
		}
	}
	Some(value)
}

pub(super) fn pin_row_ids(
	payload: &mut Value,
	payload_path: &str,
	projection: &Value,
	projection_path: &str,
	identities: &Value,
) -> Result<()> {
	let requests = array_prefixes(payload_path);
	let responses = array_prefixes(projection_path);
	assert_eq!(
		requests.len(),
		responses.len(),
		"contract array depth differs"
	);
	for (request, response) in requests.iter().zip(responses) {
		let identity = identities
			.get(&response)
			.ok_or("missing collection identity contract")?;
		assert_eq!(
			identity["requestPath"],
			request.as_str(),
			"collection identity request path"
		);
		let row =
			read_path(projection, &response).ok_or("missing persisted child")?;
		let update = read_path_mut(payload, request)
			.and_then(Value::as_object_mut)
			.ok_or("missing request child")?;
		for key in identity["keys"]
			.as_array()
			.ok_or("missing collection identity keys")?
		{
			let name = key["name"].as_str().ok_or("missing identity key name")?;
			match row.get(name) {
				Some(id) => {
					Uuid::parse_str(id.as_str().ok_or("child ID is not a string")?)?;
					update.insert(name.to_string(), id.clone());
				}
				None if key["required"] == false => {}
				None => {
					return Err(format!(
						"missing persisted child identity {response}.{name}"
					)
					.into())
				}
			}
		}
	}

	Ok(())
}

fn remove_leaf(payload: &mut Value, path: &str) -> Result<()> {
	let (parent, leaf) = path.rsplit_once('.').ok_or("field path has no owner")?;
	read_path_mut(payload, parent)
		.and_then(Value::as_object_mut)
		.ok_or("field owner is not an object")?
		.remove(leaf)
		.ok_or("missing omitted field")?;
	Ok(())
}

macro_rules! field_contract_test {
	($name:ident, $code:literal) => {
		#[serial_test::serial]
		#[tokio::test]
		async fn $name() -> crate::common::Result<()> {
			crate::case_editor_contract_web::fields::verify_field(
				crate::case_editor_contract_web::fields::FieldSpec {
					page_id: PAGE_ID,
					code: $code,
				},
			)
			.await
		}
	};
}

pub(crate) use field_contract_test;

#[test]
fn nested_row_patch_keeps_both_ids_and_can_omit_only_the_field() -> Result<()> {
	let parent = "00000000-0000-0000-0000-000000000001";
	let child = "00000000-0000-0000-0000-000000000002";
	let projection = json!({"drug":{"fdaDevices":[{"id":parent,"followUpTypes":[{"id":child,"value_code":"2"}]}]}});
	let identities = json!({
		"drug.fdaDevices[]":{"requestPath":"drug.fdaDevices[]","keys":[{"name":"id","required":true}]},
		"drug.fdaDevices[].followUpTypes[]":{"requestPath":"drug.fdaDevices[].followUpTypes[]","keys":[{"name":"id","required":true}]}
	});
	let request_path = "drug.fdaDevices[].followUpTypes[].valueCode";
	let mut payload = build_path(request_path, json!("1"));
	pin_row_ids(
		&mut payload,
		request_path,
		&projection,
		"drug.fdaDevices[].followUpTypes[].value_code",
		&identities,
	)?;
	assert_eq!(payload["drug"]["fdaDevices"][0]["id"], parent);
	assert_eq!(
		payload["drug"]["fdaDevices"][0]["followUpTypes"][0]["id"],
		child
	);
	remove_leaf(&mut payload, request_path)?;
	assert!(read_path(&payload, request_path).is_none());
	assert_eq!(
		payload["drug"]["fdaDevices"][0]["followUpTypes"][0]["id"],
		child
	);
	Ok(())
}

#[test]
fn assessment_identity_does_not_invent_a_relatedness_id() -> Result<()> {
	let parent = "00000000-0000-0000-0000-000000000001";
	let reaction = "00000000-0000-0000-0000-000000000002";
	let projection = json!({"drug":{"drugReactionAssessments":[{"drugReactionAssessmentId":parent,"reactionId":reaction}]}});
	let identities = json!({"drug.drugReactionAssessments[]":{"requestPath":"drug.drugReactionAssessments[]","keys":[
		{"name":"drugReactionAssessmentId","required":true},{"name":"reactionId","required":true},{"name":"id","required":false}
	]}});
	let path = "drug.drugReactionAssessments[].expectedness";
	let mut payload = build_path(path, Value::Null);
	pin_row_ids(&mut payload, path, &projection, path, &identities)?;
	let row = &payload["drug"]["drugReactionAssessments"][0];
	assert_eq!(row["drugReactionAssessmentId"], parent);
	assert_eq!(row["reactionId"], reaction);
	assert!(row.get("id").is_none());
	let invalid =
		json!({"drug":{"drugReactionAssessments":[{"reactionId":reaction}]}});
	assert!(pin_row_ids(&mut payload, path, &invalid, path, &identities).is_err());
	Ok(())
}
