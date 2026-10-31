use super::support::{create_case_for_editor, patch_json, post_json};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[serial]
#[tokio::test]
async fn newly_added_pairs_persist_either_member_and_reject_both() -> Result<()> {
	let mm = init_test_mm().await?;
	let seed = seed_org_with_users(&mm, "adminpwd", "viewpwd").await?;
	let token = generate_web_token(&seed.admin.email, seed.admin.token_salt)?;
	let cookie = cookie_header(&token.to_string());
	let app = web_server::app(mm.clone());
	let case_id = create_case_for_editor(
		&app,
		&cookie,
		&format!("EDITOR-DB-PAIR-{}", Uuid::new_v4()),
		&["ich"],
	)
	.await?;

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/drugs"),
		json!({"data": {"case_id": case_id, "sequence_number": 1, "drug_characterization": "1", "medicinal_product": "Pair drug"}}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let drug_id = body["data"]["id"].as_str().ok_or("missing drug id")?;
	let dg_uri = format!("/api/cases/{case_id}/editor/pages/DG/rows/{drug_id}");

	let (status, body) = post_json(
		&app,
		&cookie,
		&format!("/api/cases/{case_id}/reactions"),
		json!({
			"data": {
				"case_id": case_id,
				"sequence_number": 1,
				"primary_source_reaction": "Assessment reaction"
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::CREATED, "{body}");
	let reaction_id = body["data"]["id"].as_str().ok_or("missing reaction id")?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&dg_uri,
		json!({
			"authorities": ["ich"],
			"rows": {
				"drug": {
					"drugReactionAssessments": [{
						"reactionId": reaction_id,
						"dechallengeResult": "1",
						"expectedness": "2"
					}]
				}
			}
		}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	assert_eq!(
		body["data"]["drug"]["drugReactionAssessments"][0]["expectedness"],
		json!("2")
	);
	let (status, body) = patch_json(
		&app,
		&cookie,
		&dg_uri,
		json!({"authorities": ["ich"], "rows": {"drug": {"dosageInformation": [{
			"sequenceNumber": 1,
			"doseForm": null, "doseFormNullFlavor": "UNK",
			"routeOfAdministration": null, "routeOfAdministrationNullFlavor": "ASKU",
			"parentRouteOfAdministration": null, "parentRouteOfAdministrationNullFlavor": "NASK"
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (dosage_id, dose_form, dose_form_nf, route, route_nf, parent_route, parent_route_nf): (Uuid, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) = mm.dbx().fetch_one(
		sqlx::query_as("SELECT id, dose_form, dose_form_null_flavor, route_of_administration, route_of_administration_null_flavor, parent_route, parent_route_null_flavor FROM dosage_information WHERE drug_id = $1")
			.bind(Uuid::parse_str(drug_id)?),
	).await?;
	assert_eq!(
		(
			dose_form,
			dose_form_nf,
			route,
			route_nf,
			parent_route,
			parent_route_nf
		),
		(
			None,
			Some("UNK".into()),
			None,
			Some("ASKU".into()),
			None,
			Some("NASK".into())
		)
	);
	mm.dbx().commit_txn().await?;

	let (status, body) = patch_json(
		&app,
		&cookie,
		&dg_uri,
		json!({"authorities": ["ich"], "rows": {"drug": {"dosageInformation": [{
			"id": dosage_id,
			"doseForm": "Tablet", "doseFormNullFlavor": null,
			"routeOfAdministration": "048", "routeOfAdministrationNullFlavor": null,
			"parentRouteOfAdministration": "Oral", "parentRouteOfAdministrationNullFlavor": null
		}]}}}),
	)
	.await?;
	assert_eq!(status, StatusCode::OK, "{body}");
	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	mm.dbx().commit_txn().await?;

	mm.dbx().begin_txn().await?;
	set_full_context_dbx(
		mm.dbx(),
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO,
	)
	.await?;
	let (assessment_id, stored_dechallenge, stored_expectedness): (
		Uuid,
		Option<String>,
		Option<String>,
	) = mm.dbx()
		.fetch_one(
			sqlx::query_as(
				"SELECT id, dechallenge_result, expectedness
				 FROM drug_reaction_assessments
				 WHERE drug_id = $1 AND reaction_id = $2",
			)
			.bind(Uuid::parse_str(drug_id)?)
			.bind(Uuid::parse_str(reaction_id)?),
		)
		.await?;
	assert_eq!(stored_dechallenge.as_deref(), Some("1"));
	assert_eq!(stored_expectedness.as_deref(), Some("2"));
	mm.dbx().commit_txn().await?;

	mm.dbx().begin_txn().await?;
	mm.dbx()
		.execute(sqlx::query("SET ROLE e2br3_auditor_role"))
		.await?;
	let (audit_count,): (i64,) = mm
		.dbx()
		.fetch_one(
			sqlx::query_as(
				"SELECT count(*)
				 FROM audit_logs
				 WHERE table_name = 'drug_reaction_assessments'
				   AND record_id = $1
				   AND changed_fields ? 'expectedness'",
			)
			.bind(assessment_id),
		)
		.await?;
	assert_eq!(audit_count, 1);
	mm.dbx().rollback_txn().await?;
	Ok(())
}
