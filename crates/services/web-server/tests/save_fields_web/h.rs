use super::save_fields_common::{
	assert_i64, assert_str, extract_id, get_ok, post_created, put_ok, FieldCase,
};
use crate::common::Result;
use crate::persist_workflow::{create_case, setup, PersistTestCtx};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

fn narrative_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/narrative",
	}
}

fn sender_diagnosis_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/narrative/sender-diagnoses/{diagnosis_id}",
	}
}

fn summary_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/narrative/summaries/{summary_id}",
	}
}

async fn create_narrative(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		narrative_field("H.1.2.4"),
		format!("/api/cases/{case_id}/narrative"),
		json!({"data": {
			"case_id": case_id,
			"case_narrative": "Narrative"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_sender_diagnosis(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	narrative_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		sender_diagnosis_field("H.3.r"),
		format!("/api/cases/{case_id}/narrative/sender-diagnoses"),
		json!({"data": {
			"narrative_id": narrative_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_summary(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	narrative_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		summary_field("H.5.r"),
		format!("/api/cases/{case_id}/narrative/summaries"),
		json!({"data": {
			"narrative_id": narrative_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

macro_rules! narrative_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			create_narrative(&ctx, case_id).await?;

			put_ok(
				&ctx,
				narrative_field($canonical),
				format!("/api/cases/{case_id}/narrative"),
				json!({ "data": $payload }),
			)
			.await?;

			let value =
				get_ok(&ctx, narrative_field($canonical), format!("/api/cases/{case_id}/narrative"))
					.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! sender_diagnosis_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let narrative_id = create_narrative(&ctx, case_id).await?;
			let diagnosis_id = create_sender_diagnosis(&ctx, case_id, narrative_id).await?;

			put_ok(
				&ctx,
				sender_diagnosis_field($canonical),
				format!("/api/cases/{case_id}/narrative/sender-diagnoses/{diagnosis_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				sender_diagnosis_field($canonical),
				format!("/api/cases/{case_id}/narrative/sender-diagnoses/{diagnosis_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! summary_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let narrative_id = create_narrative(&ctx, case_id).await?;
			let summary_id = create_summary(&ctx, case_id, narrative_id).await?;

			put_ok(
				&ctx,
				summary_field($canonical),
				format!("/api/cases/{case_id}/narrative/summaries/{summary_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				summary_field($canonical),
				format!("/api/cases/{case_id}/narrative/summaries/{summary_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

narrative_single_field_test!(
	save_h_1_2_4_case_narrative_only,
	"H.1.2.4.case_narrative",
	json!({"case_narrative": "Narrative 2"}),
	|value| {
		assert_str(value, "case_narrative", "Narrative 2");
	}
);
narrative_single_field_test!(
	save_h_1_2_4_reporter_comments_only,
	"H.1.2.4.reporter_comments",
	json!({"reporter_comments": "Reporter"}),
	|value| {
		assert_str(value, "reporter_comments", "Reporter");
	}
);
narrative_single_field_test!(
	save_h_1_2_4_sender_comments_only,
	"H.1.2.4.sender_comments",
	json!({"sender_comments": "Sender"}),
	|value| {
		assert_str(value, "sender_comments", "Sender");
	}
);

sender_diagnosis_single_field_test!(
	save_h_3_r_diagnosis_meddra_version_only,
	"H.3.r.diagnosis_meddra_version",
	json!({"diagnosis_meddra_version": "27.0"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "diagnosis_meddra_version", "27.0");
	}
);
sender_diagnosis_single_field_test!(
	save_h_3_r_diagnosis_meddra_code_only,
	"H.3.r.diagnosis_meddra_code",
	json!({"diagnosis_meddra_code": "101"}),
	|value| {
		assert_str(value, "diagnosis_meddra_code", "101");
	}
);

summary_single_field_test!(
	save_h_5_r_summary_type_only,
	"H.5.r.summary_type",
	json!({"summary_type": "2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "summary_type", "2");
	}
);
summary_single_field_test!(
	save_h_5_r_language_code_only,
	"H.5.r.language_code",
	json!({"language_code": "en"}),
	|value| {
		assert_str(value, "language_code", "en");
	}
);
summary_single_field_test!(
	save_h_5_r_summary_text_only,
	"H.5.r.summary_text",
	json!({"summary_text": "Summary 2"}),
	|value| {
		assert_str(value, "summary_text", "Summary 2");
	}
);
