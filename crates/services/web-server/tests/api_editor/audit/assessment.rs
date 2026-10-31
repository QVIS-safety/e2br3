use crate::case_editor_contract_web::support::{
	create_case_for_editor, patch_json, post_json,
};
use crate::common::{cookie_header, init_test_mm, seed_org_with_users, Result};
use axum::http::StatusCode;
use lib_auth::token::generate_web_token;
use lib_core::ctx::ROLE_SPONSOR_ADMIN_CRO;
use lib_core::model::store::set_full_context_dbx;
use serde_json::json;
use uuid::Uuid;

pub(crate) async fn assessment_values_and_single_audit_entry_persist(
	_page_id: &str,
	_check_id: &str,
) -> Result<()> {
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

	let ctx = lib_core::ctx::Ctx::new(
		seed.admin.id,
		seed.org_id,
		ROLE_SPONSOR_ADMIN_CRO.to_string(),
	)?;
	let logs = lib_core::model::audit::AuditLogBmc::list_by_record(
		&ctx,
		&mm,
		"drug_reaction_assessments",
		assessment_id,
	)
	.await?;
	let audit_count = logs
		.iter()
		.filter(|log| {
			log.changed_fields
				.as_ref()
				.and_then(|fields| fields.get("expectedness"))
				.is_some()
		})
		.count();
	assert_eq!(audit_count, 1);

	Ok(())
}
