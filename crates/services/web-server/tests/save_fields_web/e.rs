use super::save_fields_common::{
	assert_bool, assert_date_tuple, assert_f64, assert_i64, assert_str, extract_id,
	get_ok, post_created, FieldCase,
};
use crate::common::Result;
use crate::persist_workflow::{create_case, setup, PersistTestCtx};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

fn reaction_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/reactions/{reaction_id}",
	}
}

async fn create_reaction_with_payload(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	mut payload: serde_json::Value,
) -> Result<Uuid> {
	payload["case_id"] = json!(case_id);
	let value = post_created(
		ctx,
		reaction_field("E.i"),
		format!("/api/cases/{case_id}/reactions"),
		json!({"data": payload}),
	)
	.await?;
	extract_id(&value)
}

macro_rules! reaction_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let mut payload = $payload;
			if payload.get("sequence_number").is_none() {
				payload["sequence_number"] = json!(1);
			}
			if payload.get("primary_source_reaction").is_none() {
				payload["primary_source_reaction"] = json!("Seed reaction");
			}

			let reaction_id =
				create_reaction_with_payload(&ctx, case_id, payload).await?;

			let value = get_ok(
				&ctx,
				reaction_field($canonical),
				format!("/api/cases/{case_id}/reactions/{reaction_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

reaction_single_field_test!(
	save_e_i_primary_source_reaction_only,
	"E.i.1.1a",
	json!({"primary_source_reaction": "Migraine"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "primary_source_reaction", "Migraine");
	}
);

reaction_single_field_test!(
	save_e_i_primary_source_reaction_translation_only,
	"E.i.1.2",
	json!({"primary_source_reaction_translation": "Migraine EN"}),
	|value| {
		assert_str(value, "primary_source_reaction_translation", "Migraine EN");
	}
);

reaction_single_field_test!(
	save_e_i_reaction_language_only,
	"E.i.1.1b",
	json!({"reaction_language": "ko"}),
	|value| {
		assert_str(value, "reaction_language", "ko");
	}
);

reaction_single_field_test!(
	save_e_i_reaction_meddra_version_only,
	"E.i.2.1a",
	json!({"reaction_meddra_version": "27.0"}),
	|value| {
		assert_str(value, "reaction_meddra_version", "27.0");
	}
);

reaction_single_field_test!(
	save_e_i_reaction_meddra_code_only,
	"E.i.2.1b",
	json!({"reaction_meddra_code": "100"}),
	|value| {
		assert_str(value, "reaction_meddra_code", "100");
	}
);

reaction_single_field_test!(
	save_e_i_term_highlighted_only,
	"E.i.3.1",
	json!({"term_highlighted": true}),
	|value| {
		assert_bool(value, "term_highlighted", true);
	}
);

reaction_single_field_test!(
	save_e_i_serious_only,
	"E.i.serious",
	json!({"serious": true}),
	|value| {
		assert_bool(value, "serious", true);
	}
);

reaction_single_field_test!(
	save_e_i_criteria_death_only,
	"E.i.3.2a",
	json!({"criteria_death": true}),
	|value| {
		assert_bool(value, "criteria_death", true);
	}
);

reaction_single_field_test!(
	save_e_i_criteria_death_null_flavor_only,
	"E.i.criteria_death_null_flavor",
	json!({"criteria_death_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "criteria_death_null_flavor", "UNK");
	}
);

reaction_single_field_test!(
	save_e_i_criteria_life_threatening_only,
	"E.i.3.2b",
	json!({"criteria_life_threatening": true}),
	|value| {
		assert_bool(value, "criteria_life_threatening", true);
	}
);

reaction_single_field_test!(
	save_e_i_criteria_life_threatening_null_flavor_only,
	"E.i.criteria_life_threatening_null_flavor",
	json!({"criteria_life_threatening_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "criteria_life_threatening_null_flavor", "UNK");
	}
);

reaction_single_field_test!(
	save_e_i_criteria_hospitalization_only,
	"E.i.3.2c",
	json!({"criteria_hospitalization": true}),
	|value| {
		assert_bool(value, "criteria_hospitalization", true);
	}
);

reaction_single_field_test!(
	save_e_i_criteria_hospitalization_null_flavor_only,
	"E.i.criteria_hospitalization_null_flavor",
	json!({"criteria_hospitalization_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "criteria_hospitalization_null_flavor", "UNK");
	}
);

reaction_single_field_test!(
	save_e_i_criteria_disabling_only,
	"E.i.3.2d",
	json!({"criteria_disabling": true}),
	|value| {
		assert_bool(value, "criteria_disabling", true);
	}
);

reaction_single_field_test!(
	save_e_i_criteria_disabling_null_flavor_only,
	"E.i.criteria_disabling_null_flavor",
	json!({"criteria_disabling_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "criteria_disabling_null_flavor", "UNK");
	}
);

reaction_single_field_test!(
	save_e_i_criteria_congenital_anomaly_only,
	"E.i.3.2e",
	json!({"criteria_congenital_anomaly": true}),
	|value| {
		assert_bool(value, "criteria_congenital_anomaly", true);
	}
);

reaction_single_field_test!(
	save_e_i_criteria_congenital_anomaly_null_flavor_only,
	"E.i.criteria_congenital_anomaly_null_flavor",
	json!({"criteria_congenital_anomaly_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "criteria_congenital_anomaly_null_flavor", "UNK");
	}
);

reaction_single_field_test!(
	save_e_i_criteria_other_medically_important_only,
	"E.i.3.2f",
	json!({"criteria_other_medically_important": true}),
	|value| {
		assert_bool(value, "criteria_other_medically_important", true);
	}
);

reaction_single_field_test!(
	save_e_i_criteria_other_medically_important_null_flavor_only,
	"E.i.criteria_other_medically_important_null_flavor",
	json!({"criteria_other_medically_important_null_flavor": "UNK"}),
	|value| {
		assert_str(
			value,
			"criteria_other_medically_important_null_flavor",
			"UNK",
		);
	}
);

reaction_single_field_test!(
	save_e_i_required_intervention_only,
	"FDA.E.i.3.2h",
	json!({"required_intervention": "1"}),
	|value| {
		assert_str(value, "required_intervention", "1");
	}
);

reaction_single_field_test!(
	save_e_i_start_date_only,
	"E.i.4",
	json!({"start_date": [2024, 1, 1]}),
	|value| {
		assert_date_tuple(value, "start_date", &[2024, 1]);
	}
);

reaction_single_field_test!(
	save_e_i_end_date_only,
	"E.i.5",
	json!({"end_date": [2024, 1, 2]}),
	|value| {
		assert_date_tuple(value, "end_date", &[2024, 2]);
	}
);

reaction_single_field_test!(
	save_e_i_duration_value_only,
	"E.i.6a",
	json!({"duration_value": 2.0}),
	|value| {
		assert_f64(value, "duration_value", 2.0);
	}
);

reaction_single_field_test!(
	save_e_i_duration_unit_only,
	"E.i.6b",
	json!({"duration_unit": "d"}),
	|value| {
		assert_str(value, "duration_unit", "d");
	}
);

reaction_single_field_test!(
	save_e_i_outcome_only,
	"E.i.7",
	json!({"outcome": "1"}),
	|value| {
		assert_str(value, "outcome", "1");
	}
);

reaction_single_field_test!(
	save_e_i_medical_confirmation_only,
	"E.i.8",
	json!({"medical_confirmation": true}),
	|value| {
		assert_bool(value, "medical_confirmation", true);
	}
);

reaction_single_field_test!(
	save_e_i_country_code_only,
	"E.i.9",
	json!({"country_code": "KR"}),
	|value| {
		assert_str(value, "country_code", "KR");
	}
);

reaction_single_field_test!(
	save_e_i_start_date_null_flavor_only,
	"E.i.start_date_null_flavor",
	json!({"start_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "start_date_null_flavor", "UNK");
	}
);

reaction_single_field_test!(
	save_e_i_end_date_null_flavor_only,
	"E.i.end_date_null_flavor",
	json!({"end_date_null_flavor": "ASKU"}),
	|value| {
		assert_str(value, "end_date_null_flavor", "ASKU");
	}
);
