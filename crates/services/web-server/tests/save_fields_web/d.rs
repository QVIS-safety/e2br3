use super::save_fields_common::{
	assert_bool, assert_date_tuple, assert_f64, assert_i64, assert_str, extract_id,
	get_ok, post_created, put_ok, FieldCase,
};
use crate::common::Result;
use crate::persist_workflow::{create_case, setup, PersistTestCtx};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

fn patient_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/patient",
	}
}

fn patient_identifier_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/patient/identifiers/{identifier_id}",
	}
}

fn medical_history_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/patient/medical-history/{history_id}",
	}
}

fn past_drug_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/patient/past-drugs/{past_drug_id}",
	}
}

fn death_info_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/patient/death-info/{death_info_id}",
	}
}

fn reported_cause_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/patient/death-info/{death_info_id}/reported-causes/{cause_id}",
	}
}

fn autopsy_cause_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/patient/death-info/{death_info_id}/autopsy-causes/{cause_id}",
	}
}

fn parent_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/patient/parents/{parent_id}",
	}
}

fn parent_medical_history_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/patient/parent/{parent_id}/medical-history/{history_id}",
	}
}

fn parent_past_drug_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/patient/parent/{parent_id}/past-drugs/{past_drug_id}",
	}
}

async fn create_patient(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		patient_field("D.1.2"),
		format!("/api/cases/{case_id}/patient"),
		json!({"data": {
			"case_id": case_id
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_patient_identifier(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	patient_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		patient_identifier_field("D.2.1.r"),
		format!("/api/cases/{case_id}/patient/identifiers"),
		json!({"data": {
			"patient_id": patient_id,
			"sequence_number": 1,
			"identifier_type_code": "1",
			"identifier_value": "123"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_medical_history(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	patient_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		medical_history_field("D.7"),
		format!("/api/cases/{case_id}/patient/medical-history"),
		json!({"data": {
			"patient_id": patient_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_past_drug(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	patient_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		past_drug_field("D.8.r"),
		format!("/api/cases/{case_id}/patient/past-drugs"),
		json!({"data": {
			"patient_id": patient_id,
			"sequence_number": 1,
			"drug_name_null_flavor": "NI",
			"start_date_null_flavor": "UNK",
			"end_date_null_flavor": "ASKU"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_death_info(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	patient_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		death_info_field("D.9"),
		format!("/api/cases/{case_id}/patient/death-info"),
		json!({"data": {
			"patient_id": patient_id,
			"date_of_death_null_flavor": "UNK",
			"autopsy_performed": false
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_reported_cause(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	death_info_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		reported_cause_field("D.9.1.r"),
		format!("/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes"),
		json!({"data": {
			"death_info_id": death_info_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_autopsy_cause(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	death_info_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		autopsy_cause_field("D.9.2.r"),
		format!(
			"/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes"
		),
		json!({"data": {
			"death_info_id": death_info_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_parent(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	patient_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		parent_field("D.10"),
		format!("/api/cases/{case_id}/patient/parents"),
		json!({"data": {
			"patient_id": patient_id,
			"sex": null,
			"medical_history_text": null
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_parent_medical_history(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	parent_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		parent_medical_history_field("D.10.6.r"),
		format!("/api/cases/{case_id}/patient/parent/{parent_id}/medical-history"),
		json!({"data": {
			"parent_id": parent_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_parent_past_drug(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	parent_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		parent_past_drug_field("D.10.7.r"),
		format!("/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs"),
		json!({"data": {
			"parent_id": parent_id,
			"sequence_number": 1,
			"drug_name_null_flavor": "NI",
			"start_date_null_flavor": "UNK",
			"end_date_null_flavor": "ASKU"
		}}),
	)
	.await?;
	extract_id(&value)
}

macro_rules! patient_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			create_patient(&ctx, case_id).await?;

			put_ok(
				&ctx,
				patient_field($canonical),
				format!("/api/cases/{case_id}/patient"),
				json!({ "data": $payload }),
			)
			.await?;

			let value =
				get_ok(&ctx, patient_field($canonical), format!("/api/cases/{case_id}/patient"))
					.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! patient_identifier_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let identifier_id = create_patient_identifier(&ctx, case_id, patient_id).await?;

			put_ok(
				&ctx,
				patient_identifier_field($canonical),
				format!("/api/cases/{case_id}/patient/identifiers/{identifier_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				patient_identifier_field($canonical),
				format!("/api/cases/{case_id}/patient/identifiers/{identifier_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! medical_history_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let history_id = create_medical_history(&ctx, case_id, patient_id).await?;

			put_ok(
				&ctx,
				medical_history_field($canonical),
				format!("/api/cases/{case_id}/patient/medical-history/{history_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				medical_history_field($canonical),
				format!("/api/cases/{case_id}/patient/medical-history/{history_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! past_drug_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let past_drug_id = create_past_drug(&ctx, case_id, patient_id).await?;

			put_ok(
				&ctx,
				past_drug_field($canonical),
				format!("/api/cases/{case_id}/patient/past-drugs/{past_drug_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				past_drug_field($canonical),
				format!("/api/cases/{case_id}/patient/past-drugs/{past_drug_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! death_info_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let death_info_id = create_death_info(&ctx, case_id, patient_id).await?;

			put_ok(
				&ctx,
				death_info_field($canonical),
				format!("/api/cases/{case_id}/patient/death-info/{death_info_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				death_info_field($canonical),
				format!("/api/cases/{case_id}/patient/death-info/{death_info_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! reported_cause_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let death_info_id = create_death_info(&ctx, case_id, patient_id).await?;
			let cause_id = create_reported_cause(&ctx, case_id, death_info_id).await?;

			put_ok(
				&ctx,
				reported_cause_field($canonical),
				format!(
					"/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{cause_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				reported_cause_field($canonical),
				format!(
					"/api/cases/{case_id}/patient/death-info/{death_info_id}/reported-causes/{cause_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! autopsy_cause_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let death_info_id = create_death_info(&ctx, case_id, patient_id).await?;
			let cause_id = create_autopsy_cause(&ctx, case_id, death_info_id).await?;

			put_ok(
				&ctx,
				autopsy_cause_field($canonical),
				format!(
					"/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{cause_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				autopsy_cause_field($canonical),
				format!(
					"/api/cases/{case_id}/patient/death-info/{death_info_id}/autopsy-causes/{cause_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! parent_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let parent_id = create_parent(&ctx, case_id, patient_id).await?;

			put_ok(
				&ctx,
				parent_field($canonical),
				format!("/api/cases/{case_id}/patient/parents/{parent_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				parent_field($canonical),
				format!("/api/cases/{case_id}/patient/parents/{parent_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! parent_medical_history_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let parent_id = create_parent(&ctx, case_id, patient_id).await?;
			let history_id =
				create_parent_medical_history(&ctx, case_id, parent_id).await?;

			put_ok(
				&ctx,
				parent_medical_history_field($canonical),
				format!(
					"/api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{history_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				parent_medical_history_field($canonical),
				format!(
					"/api/cases/{case_id}/patient/parent/{parent_id}/medical-history/{history_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! parent_past_drug_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let patient_id = create_patient(&ctx, case_id).await?;
			let parent_id = create_parent(&ctx, case_id, patient_id).await?;
			let past_drug_id = create_parent_past_drug(&ctx, case_id, parent_id).await?;

			put_ok(
				&ctx,
				parent_past_drug_field($canonical),
				format!(
					"/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{past_drug_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				parent_past_drug_field($canonical),
				format!(
					"/api/cases/{case_id}/patient/parent/{parent_id}/past-drugs/{past_drug_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

patient_single_field_test!(
	save_d_1_2_patient_initials_only,
	"D.1.2.patient_initials",
	json!({"patient_initials": "AB"}),
	|value| {
		assert_str(value, "patient_initials", "AB");
	}
);
patient_single_field_test!(
	save_d_1_2_patient_given_name_only,
	"D.1.2.patient_given_name",
	json!({"patient_given_name": "Alice"}),
	|value| {
		assert_str(value, "patient_given_name", "Alice");
	}
);
patient_single_field_test!(
	save_d_1_2_patient_family_name_only,
	"D.1.2.patient_family_name",
	json!({"patient_family_name": "Brown"}),
	|value| {
		assert_str(value, "patient_family_name", "Brown");
	}
);
patient_single_field_test!(
	save_d_1_2_patient_initials_null_flavor_only,
	"D.1.2.patient_initials_null_flavor",
	json!({"patient_initials_null_flavor": "NI"}),
	|value| {
		assert_str(value, "patient_initials_null_flavor", "NI");
	}
);
patient_single_field_test!(
	save_d_1_2_birth_date_only,
	"D.1.2.birth_date",
	json!({"birth_date": [2020, 1, 1]}),
	|value| {
		assert_date_tuple(value, "birth_date", &[2020, 1]);
	}
);
patient_single_field_test!(
	save_d_1_2_birth_date_null_flavor_only,
	"D.1.2.birth_date_null_flavor",
	json!({"birth_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "birth_date_null_flavor", "UNK");
	}
);
patient_single_field_test!(
	save_d_1_2_age_at_time_of_onset_only,
	"D.1.2.age_at_time_of_onset",
	json!({"age_at_time_of_onset": 33.0}),
	|value| {
		assert_f64(value, "age_at_time_of_onset", 33.0);
	}
);
patient_single_field_test!(
	save_d_1_2_age_at_time_of_onset_null_flavor_only,
	"D.1.2.age_at_time_of_onset_null_flavor",
	json!({"age_at_time_of_onset_null_flavor": "ASKU"}),
	|value| {
		assert_str(value, "age_at_time_of_onset_null_flavor", "ASKU");
	}
);
patient_single_field_test!(
	save_d_1_2_age_unit_only,
	"D.1.2.age_unit",
	json!({"age_unit": "801"}),
	|value| {
		assert_str(value, "age_unit", "801");
	}
);
patient_single_field_test!(
	save_d_1_2_gestation_period_only,
	"D.1.2.gestation_period",
	json!({"gestation_period": 10.0}),
	|value| {
		assert_f64(value, "gestation_period", 10.0);
	}
);
patient_single_field_test!(
	save_d_1_2_gestation_period_unit_only,
	"D.1.2.gestation_period_unit",
	json!({"gestation_period_unit": "804"}),
	|value| {
		assert_str(value, "gestation_period_unit", "804");
	}
);
patient_single_field_test!(
	save_d_1_2_age_group_only,
	"D.1.2.age_group",
	json!({"age_group": "1"}),
	|value| {
		assert_str(value, "age_group", "1");
	}
);
patient_single_field_test!(
	save_d_1_2_weight_kg_only,
	"D.1.2.weight_kg",
	json!({"weight_kg": 70.0}),
	|value| {
		assert_f64(value, "weight_kg", 70.0);
	}
);
patient_single_field_test!(
	save_d_1_2_height_cm_only,
	"D.1.2.height_cm",
	json!({"height_cm": 175.0}),
	|value| {
		assert_f64(value, "height_cm", 175.0);
	}
);
patient_single_field_test!(
	save_d_1_2_sex_only,
	"D.1.2.sex",
	json!({"sex": "2"}),
	|value| {
		assert_str(value, "sex", "2");
	}
);
patient_single_field_test!(
	save_d_1_2_sex_null_flavor_only,
	"D.1.2.sex_null_flavor",
	json!({"sex_null_flavor": "NI"}),
	|value| {
		assert_str(value, "sex_null_flavor", "NI");
	}
);
patient_single_field_test!(
	save_d_1_2_race_code_only,
	"D.1.2.race_code",
	json!({"race_code": "R1"}),
	|value| {
		assert_str(value, "race_code", "R1");
	}
);
patient_single_field_test!(
	save_d_1_2_ethnicity_code_only,
	"D.1.2.ethnicity_code",
	json!({"ethnicity_code": "E1"}),
	|value| {
		assert_str(value, "ethnicity_code", "E1");
	}
);
patient_single_field_test!(
	save_d_1_2_last_menstrual_period_date_only,
	"D.1.2.last_menstrual_period_date",
	json!({"last_menstrual_period_date": [2023, 12, 1]}),
	|value| {
		assert_date_tuple(value, "last_menstrual_period_date", &[2023, 335]);
	}
);
patient_single_field_test!(
	save_d_1_2_last_menstrual_period_date_null_flavor_only,
	"D.1.2.last_menstrual_period_date_null_flavor",
	json!({"last_menstrual_period_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "last_menstrual_period_date_null_flavor", "UNK");
	}
);
patient_single_field_test!(
	save_d_1_2_medical_history_text_only,
	"D.1.2.medical_history_text",
	json!({"medical_history_text": "History"}),
	|value| {
		assert_str(value, "medical_history_text", "History");
	}
);
patient_single_field_test!(
	save_d_1_2_concomitant_therapy_only,
	"D.1.2.concomitant_therapy",
	json!({"concomitant_therapy": true}),
	|value| {
		assert_bool(value, "concomitant_therapy", true);
	}
);

patient_identifier_single_field_test!(
	save_d_2_1_r_identifier_type_code_only,
	"D.2.1.r.identifier_type_code",
	json!({"identifier_type_code": "2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "identifier_type_code", "2");
	}
);
patient_identifier_single_field_test!(
	save_d_2_1_r_identifier_value_only,
	"D.2.1.r.identifier_value",
	json!({"identifier_value": "456"}),
	|value| {
		assert_str(value, "identifier_value", "456");
	}
);

medical_history_single_field_test!(
	save_d_7_meddra_version_only,
	"D.7.meddra_version",
	json!({"meddra_version": "27.0"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "meddra_version", "27.0");
	}
);
medical_history_single_field_test!(
	save_d_7_meddra_code_only,
	"D.7.meddra_code",
	json!({"meddra_code": "200"}),
	|value| {
		assert_str(value, "meddra_code", "200");
	}
);
medical_history_single_field_test!(
	save_d_7_start_date_only,
	"D.7.start_date",
	json!({"start_date": [2024, 1, 1]}),
	|value| {
		assert_date_tuple(value, "start_date", &[2024, 1]);
	}
);
medical_history_single_field_test!(
	save_d_7_start_date_null_flavor_only,
	"D.7.start_date_null_flavor",
	json!({"start_date_null_flavor": "NI"}),
	|value| {
		assert_str(value, "start_date_null_flavor", "NI");
	}
);
medical_history_single_field_test!(
	save_d_7_continuing_only,
	"D.7.continuing",
	json!({"continuing": true}),
	|value| {
		assert_bool(value, "continuing", true);
	}
);
medical_history_single_field_test!(
	save_d_7_end_date_only,
	"D.7.end_date",
	json!({"end_date": [2024, 1, 2]}),
	|value| {
		assert_date_tuple(value, "end_date", &[2024, 2]);
	}
);
medical_history_single_field_test!(
	save_d_7_end_date_null_flavor_only,
	"D.7.end_date_null_flavor",
	json!({"end_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "end_date_null_flavor", "UNK");
	}
);
medical_history_single_field_test!(
	save_d_7_comments_only,
	"D.7.comments",
	json!({"comments": "Comment"}),
	|value| {
		assert_str(value, "comments", "Comment");
	}
);
medical_history_single_field_test!(
	save_d_7_family_history_only,
	"D.7.family_history",
	json!({"family_history": false}),
	|value| {
		assert_bool(value, "family_history", false);
	}
);

past_drug_single_field_test!(
	save_d_8_r_drug_name_only,
	"D.8.r.drug_name",
	json!({"drug_name": "Drug 2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "drug_name", "Drug 2");
	}
);
past_drug_single_field_test!(
	save_d_8_r_drug_name_null_flavor_only,
	"D.8.r.drug_name_null_flavor",
	json!({"drug_name_null_flavor": "MSK"}),
	|value| {
		assert_str(value, "drug_name_null_flavor", "MSK");
	}
);
past_drug_single_field_test!(
	save_d_8_r_mpid_only,
	"D.8.r.mpid",
	json!({"mpid": "MPID2"}),
	|value| {
		assert_str(value, "mpid", "MPID2");
	}
);
past_drug_single_field_test!(
	save_d_8_r_mpid_version_only,
	"D.8.r.mpid_version",
	json!({"mpid_version": "2"}),
	|value| {
		assert_str(value, "mpid_version", "2");
	}
);
past_drug_single_field_test!(
	save_d_8_r_phpid_only,
	"D.8.r.phpid",
	json!({"phpid": "PHPID2"}),
	|value| {
		assert_str(value, "phpid", "PHPID2");
	}
);
past_drug_single_field_test!(
	save_d_8_r_phpid_version_only,
	"D.8.r.phpid_version",
	json!({"phpid_version": "3"}),
	|value| {
		assert_str(value, "phpid_version", "3");
	}
);
past_drug_single_field_test!(
	save_d_8_r_start_date_only,
	"D.8.r.start_date",
	json!({"start_date": [2024, 2, 1]}),
	|value| {
		assert_date_tuple(value, "start_date", &[2024, 32]);
	}
);
past_drug_single_field_test!(
	save_d_8_r_start_date_null_flavor_only,
	"D.8.r.start_date_null_flavor",
	json!({"start_date_null_flavor": "NI"}),
	|value| {
		assert_str(value, "start_date_null_flavor", "NI");
	}
);
past_drug_single_field_test!(
	save_d_8_r_end_date_only,
	"D.8.r.end_date",
	json!({"end_date": [2024, 2, 2]}),
	|value| {
		assert_date_tuple(value, "end_date", &[2024, 33]);
	}
);
past_drug_single_field_test!(
	save_d_8_r_end_date_null_flavor_only,
	"D.8.r.end_date_null_flavor",
	json!({"end_date_null_flavor": "ASKU"}),
	|value| {
		assert_str(value, "end_date_null_flavor", "ASKU");
	}
);
past_drug_single_field_test!(
	save_d_8_r_indication_meddra_version_only,
	"D.8.r.indication_meddra_version",
	json!({"indication_meddra_version": "28.0"}),
	|value| {
		assert_str(value, "indication_meddra_version", "28.0");
	}
);
past_drug_single_field_test!(
	save_d_8_r_indication_meddra_code_only,
	"D.8.r.indication_meddra_code",
	json!({"indication_meddra_code": "301"}),
	|value| {
		assert_str(value, "indication_meddra_code", "301");
	}
);
past_drug_single_field_test!(
	save_d_8_r_reaction_meddra_version_only,
	"D.8.r.reaction_meddra_version",
	json!({"reaction_meddra_version": "28.0"}),
	|value| {
		assert_str(value, "reaction_meddra_version", "28.0");
	}
);
past_drug_single_field_test!(
	save_d_8_r_reaction_meddra_code_only,
	"D.8.r.reaction_meddra_code",
	json!({"reaction_meddra_code": "401"}),
	|value| {
		assert_str(value, "reaction_meddra_code", "401");
	}
);

death_info_single_field_test!(
	save_d_9_date_of_death_only,
	"D.9.date_of_death",
	json!({"date_of_death": [2024, 2, 10]}),
	|value| {
		assert_date_tuple(value, "date_of_death", &[2024, 41]);
	}
);
death_info_single_field_test!(
	save_d_9_date_of_death_null_flavor_only,
	"D.9.date_of_death_null_flavor",
	json!({"date_of_death_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "date_of_death_null_flavor", "UNK");
	}
);
death_info_single_field_test!(
	save_d_9_autopsy_performed_only,
	"D.9.autopsy_performed",
	json!({"autopsy_performed": true}),
	|value| {
		assert_bool(value, "autopsy_performed", true);
	}
);

reported_cause_single_field_test!(
	save_d_9_1_r_meddra_version_only,
	"D.9.1.r.meddra_version",
	json!({"meddra_version": "27.0"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "meddra_version", "27.0");
	}
);
reported_cause_single_field_test!(
	save_d_9_1_r_meddra_code_only,
	"D.9.1.r.meddra_code",
	json!({"meddra_code": "501"}),
	|value| {
		assert_str(value, "meddra_code", "501");
	}
);
reported_cause_single_field_test!(
	save_d_9_1_r_comments_only,
	"D.9.1.r.comments",
	json!({"comments": "Comment"}),
	|value| {
		assert_str(value, "comments", "Comment");
	}
);

autopsy_cause_single_field_test!(
	save_d_9_2_r_meddra_version_only,
	"D.9.2.r.meddra_version",
	json!({"meddra_version": "27.0"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "meddra_version", "27.0");
	}
);
autopsy_cause_single_field_test!(
	save_d_9_2_r_meddra_code_only,
	"D.9.2.r.meddra_code",
	json!({"meddra_code": "601"}),
	|value| {
		assert_str(value, "meddra_code", "601");
	}
);
autopsy_cause_single_field_test!(
	save_d_9_2_r_comments_only,
	"D.9.2.r.comments",
	json!({"comments": "Comment"}),
	|value| {
		assert_str(value, "comments", "Comment");
	}
);

parent_single_field_test!(
	save_d_10_parent_identification_only,
	"D.10.parent_identification",
	json!({"parent_identification": "PARENT-1"}),
	|value| {
		assert_str(value, "parent_identification", "PARENT-1");
	}
);
parent_single_field_test!(
	save_d_10_parent_birth_date_only,
	"D.10.parent_birth_date",
	json!({"parent_birth_date": [1980, 1, 1]}),
	|value| {
		assert_date_tuple(value, "parent_birth_date", &[1980, 1]);
	}
);
parent_single_field_test!(
	save_d_10_parent_birth_date_null_flavor_only,
	"D.10.parent_birth_date_null_flavor",
	json!({"parent_birth_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "parent_birth_date_null_flavor", "UNK");
	}
);
parent_single_field_test!(
	save_d_10_parent_age_only,
	"D.10.parent_age",
	json!({"parent_age": 44.0}),
	|value| {
		assert_f64(value, "parent_age", 44.0);
	}
);
parent_single_field_test!(
	save_d_10_parent_age_null_flavor_only,
	"D.10.parent_age_null_flavor",
	json!({"parent_age_null_flavor": "ASKU"}),
	|value| {
		assert_str(value, "parent_age_null_flavor", "ASKU");
	}
);
parent_single_field_test!(
	save_d_10_parent_age_unit_only,
	"D.10.parent_age_unit",
	json!({"parent_age_unit": "801"}),
	|value| {
		assert_str(value, "parent_age_unit", "801");
	}
);
parent_single_field_test!(
	save_d_10_last_menstrual_period_date_only,
	"D.10.last_menstrual_period_date",
	json!({"last_menstrual_period_date": [2023, 12, 1]}),
	|value| {
		assert_date_tuple(value, "last_menstrual_period_date", &[2023, 335]);
	}
);
parent_single_field_test!(
	save_d_10_last_menstrual_period_date_null_flavor_only,
	"D.10.last_menstrual_period_date_null_flavor",
	json!({"last_menstrual_period_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "last_menstrual_period_date_null_flavor", "UNK");
	}
);
parent_single_field_test!(
	save_d_10_weight_kg_only,
	"D.10.weight_kg",
	json!({"weight_kg": 65.0}),
	|value| {
		assert_f64(value, "weight_kg", 65.0);
	}
);
parent_single_field_test!(
	save_d_10_height_cm_only,
	"D.10.height_cm",
	json!({"height_cm": 165.0}),
	|value| {
		assert_f64(value, "height_cm", 165.0);
	}
);
parent_single_field_test!(
	save_d_10_sex_only,
	"D.10.sex",
	json!({"sex": "1"}),
	|value| {
		assert_str(value, "sex", "1");
	}
);
parent_single_field_test!(
	save_d_10_medical_history_text_only,
	"D.10.medical_history_text",
	json!({"medical_history_text": "Parent history"}),
	|value| {
		assert_str(value, "medical_history_text", "Parent history");
	}
);

parent_medical_history_single_field_test!(
	save_d_10_6_r_meddra_version_only,
	"D.10.6.r.meddra_version",
	json!({"meddra_version": "27.0"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "meddra_version", "27.0");
	}
);
parent_medical_history_single_field_test!(
	save_d_10_6_r_meddra_code_only,
	"D.10.6.r.meddra_code",
	json!({"meddra_code": "701"}),
	|value| {
		assert_str(value, "meddra_code", "701");
	}
);
parent_medical_history_single_field_test!(
	save_d_10_6_r_start_date_only,
	"D.10.6.r.start_date",
	json!({"start_date": [2024, 3, 1]}),
	|value| {
		assert_date_tuple(value, "start_date", &[2024, 61]);
	}
);
parent_medical_history_single_field_test!(
	save_d_10_6_r_start_date_null_flavor_only,
	"D.10.6.r.start_date_null_flavor",
	json!({"start_date_null_flavor": "NI"}),
	|value| {
		assert_str(value, "start_date_null_flavor", "NI");
	}
);
parent_medical_history_single_field_test!(
	save_d_10_6_r_continuing_only,
	"D.10.6.r.continuing",
	json!({"continuing": false}),
	|value| {
		assert_bool(value, "continuing", false);
	}
);
parent_medical_history_single_field_test!(
	save_d_10_6_r_end_date_only,
	"D.10.6.r.end_date",
	json!({"end_date": [2024, 3, 2]}),
	|value| {
		assert_date_tuple(value, "end_date", &[2024, 62]);
	}
);
parent_medical_history_single_field_test!(
	save_d_10_6_r_end_date_null_flavor_only,
	"D.10.6.r.end_date_null_flavor",
	json!({"end_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "end_date_null_flavor", "UNK");
	}
);
parent_medical_history_single_field_test!(
	save_d_10_6_r_comments_only,
	"D.10.6.r.comments",
	json!({"comments": "Comment"}),
	|value| {
		assert_str(value, "comments", "Comment");
	}
);

parent_past_drug_single_field_test!(
	save_d_10_7_r_drug_name_only,
	"D.10.7.r.drug_name",
	json!({"drug_name": "Drug 2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "drug_name", "Drug 2");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_drug_name_null_flavor_only,
	"D.10.7.r.drug_name_null_flavor",
	json!({"drug_name_null_flavor": "MSK"}),
	|value| {
		assert_str(value, "drug_name_null_flavor", "MSK");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_mpid_only,
	"D.10.7.r.mpid",
	json!({"mpid": "MPID"}),
	|value| {
		assert_str(value, "mpid", "MPID");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_mpid_version_only,
	"D.10.7.r.mpid_version",
	json!({"mpid_version": "1"}),
	|value| {
		assert_str(value, "mpid_version", "1");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_phpid_only,
	"D.10.7.r.phpid",
	json!({"phpid": "PHPID"}),
	|value| {
		assert_str(value, "phpid", "PHPID");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_phpid_version_only,
	"D.10.7.r.phpid_version",
	json!({"phpid_version": "2"}),
	|value| {
		assert_str(value, "phpid_version", "2");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_start_date_only,
	"D.10.7.r.start_date",
	json!({"start_date": [2024, 4, 1]}),
	|value| {
		assert_date_tuple(value, "start_date", &[2024, 92]);
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_start_date_null_flavor_only,
	"D.10.7.r.start_date_null_flavor",
	json!({"start_date_null_flavor": "NI"}),
	|value| {
		assert_str(value, "start_date_null_flavor", "NI");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_end_date_only,
	"D.10.7.r.end_date",
	json!({"end_date": [2024, 4, 2]}),
	|value| {
		assert_date_tuple(value, "end_date", &[2024, 93]);
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_end_date_null_flavor_only,
	"D.10.7.r.end_date_null_flavor",
	json!({"end_date_null_flavor": "ASKU"}),
	|value| {
		assert_str(value, "end_date_null_flavor", "ASKU");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_indication_meddra_version_only,
	"D.10.7.r.indication_meddra_version",
	json!({"indication_meddra_version": "27.0"}),
	|value| {
		assert_str(value, "indication_meddra_version", "27.0");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_indication_meddra_code_only,
	"D.10.7.r.indication_meddra_code",
	json!({"indication_meddra_code": "800"}),
	|value| {
		assert_str(value, "indication_meddra_code", "800");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_reaction_meddra_version_only,
	"D.10.7.r.reaction_meddra_version",
	json!({"reaction_meddra_version": "27.0"}),
	|value| {
		assert_str(value, "reaction_meddra_version", "27.0");
	}
);
parent_past_drug_single_field_test!(
	save_d_10_7_r_reaction_meddra_code_only,
	"D.10.7.r.reaction_meddra_code",
	json!({"reaction_meddra_code": "801"}),
	|value| {
		assert_str(value, "reaction_meddra_code", "801");
	}
);
