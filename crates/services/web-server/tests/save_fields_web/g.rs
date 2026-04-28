use super::save_fields_common::{
	assert_bool, assert_date_tuple, assert_f64, assert_i64, assert_str, extract_id,
	get_ok, post_created, put_ok, FieldCase,
};
use crate::common::Result;
use crate::persist_workflow::{create_case, setup, PersistTestCtx};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

fn drug_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/drugs/{drug_id}",
	}
}

fn active_substance_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/drugs/{drug_id}/active-substances/{substance_id}",
	}
}

fn dosage_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/drugs/{drug_id}/dosages/{dosage_id}",
	}
}

fn indication_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/drugs/{drug_id}/indications/{indication_id}",
	}
}

fn recurrence_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint: "/api/cases/{id}/drugs/{drug_id}/recurrences/{recurrence_id}",
	}
}

fn device_characteristic_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/drugs/{drug_id}/device-characteristics/{characteristic_id}",
	}
}

fn drug_reaction_assessment_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/drugs/{drug_id}/reaction-assessments/{assessment_id}",
	}
}

fn relatedness_field(id: &'static str) -> FieldCase {
	FieldCase {
		canonical_id: id,
		endpoint:
			"/api/cases/{id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{relatedness_id}",
	}
}

async fn create_drug(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		drug_field("G.k"),
		format!("/api/cases/{case_id}/drugs"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"drug_characterization": "1",
			"medicinal_product": "Seed product"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_reaction(ctx: &PersistTestCtx, case_id: Uuid) -> Result<Uuid> {
	let value = post_created(
		ctx,
		FieldCase {
			canonical_id: "G reaction prerequisite",
			endpoint: "/api/cases/{id}/reactions/{reaction_id}",
		},
		format!("/api/cases/{case_id}/reactions"),
		json!({"data": {
			"case_id": case_id,
			"sequence_number": 1,
			"primary_source_reaction": "Seed reaction"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_active_substance(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		active_substance_field("G.k.2.3.r"),
		format!("/api/cases/{case_id}/drugs/{drug_id}/active-substances"),
		json!({"data": {
			"drug_id": drug_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_dosage(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		dosage_field("G.k.4.r"),
		format!("/api/cases/{case_id}/drugs/{drug_id}/dosages"),
		json!({"data": {
			"drug_id": drug_id,
			"sequence_number": 1,
			"first_administration_date_null_flavor": "NI",
			"last_administration_date_null_flavor": "UNK"
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_indication(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		indication_field("G.k.6.r"),
		format!("/api/cases/{case_id}/drugs/{drug_id}/indications"),
		json!({"data": {
			"drug_id": drug_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_recurrence(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
) -> Result<Uuid> {
	create_recurrence_with_payload(
		ctx,
		case_id,
		drug_id,
		json!({
			"sequence_number": 1
		}),
	)
	.await
}

async fn create_recurrence_with_payload(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
	mut payload: serde_json::Value,
) -> Result<Uuid> {
	payload["drug_id"] = json!(drug_id);
	let value = post_created(
		ctx,
		recurrence_field("G.k.8.r"),
		format!("/api/cases/{case_id}/drugs/{drug_id}/recurrences"),
		json!({"data": payload}),
	)
	.await?;
	extract_id(&value)
}

async fn create_device_characteristic(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		device_characteristic_field("G.k.10"),
		format!("/api/cases/{case_id}/drugs/{drug_id}/device-characteristics"),
		json!({"data": {
			"drug_id": drug_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

async fn create_assessment(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
	reaction_id: Uuid,
) -> Result<Uuid> {
	create_assessment_with_payload(ctx, case_id, drug_id, reaction_id, json!({}))
		.await
}

async fn create_assessment_with_payload(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
	reaction_id: Uuid,
	mut payload: serde_json::Value,
) -> Result<Uuid> {
	payload["drug_id"] = json!(drug_id);
	payload["reaction_id"] = json!(reaction_id);
	let value = post_created(
		ctx,
		drug_reaction_assessment_field("G.k.9.i"),
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments"),
		json!({"data": payload}),
	)
	.await?;
	extract_id(&value)
}

async fn create_relatedness(
	ctx: &PersistTestCtx,
	case_id: Uuid,
	drug_id: Uuid,
	assessment_id: Uuid,
) -> Result<Uuid> {
	let value = post_created(
		ctx,
		relatedness_field("G.k.9.i.2.r"),
		format!(
			"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness"
		),
		json!({"data": {
			"drug_reaction_assessment_id": assessment_id,
			"sequence_number": 1
		}}),
	)
	.await?;
	extract_id(&value)
}

macro_rules! drug_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;

			put_ok(
				&ctx,
				drug_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				drug_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! drug_create_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let mut data = json!({
				"case_id": case_id,
				"sequence_number": 1,
				"drug_characterization": "1",
				"medicinal_product": "Seed product"
			});
			if let Some(data_obj) = data.as_object_mut() {
				if let Some(extra_obj) = json!($payload).as_object() {
					for (key, value) in extra_obj {
						data_obj.insert(key.clone(), value.clone());
					}
				}
			}

			let value = post_created(
				&ctx,
				drug_field($canonical),
				format!("/api/cases/{case_id}/drugs"),
				json!({ "data": data }),
			)
			.await?;
			let drug_id = extract_id(&value)?;

			let value = get_ok(
				&ctx,
				drug_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! active_substance_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;
			let substance_id = create_active_substance(&ctx, case_id, drug_id).await?;

			put_ok(
				&ctx,
				active_substance_field($canonical),
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{substance_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				active_substance_field($canonical),
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/active-substances/{substance_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! dosage_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;
			let dosage_id = create_dosage(&ctx, case_id, drug_id).await?;

			put_ok(
				&ctx,
				dosage_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}/dosages/{dosage_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				dosage_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}/dosages/{dosage_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! indication_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;
			let indication_id = create_indication(&ctx, case_id, drug_id).await?;

			put_ok(
				&ctx,
				indication_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}/indications/{indication_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				indication_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}/indications/{indication_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! recurrence_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;
			let mut payload = $payload;
			if payload.get("sequence_number").is_none() {
				payload["sequence_number"] = json!(1);
			}
			let recurrence_id = create_recurrence_with_payload(
				&ctx,
				case_id,
				drug_id,
				payload,
			)
			.await?;

			let value = get_ok(
				&ctx,
				recurrence_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}/recurrences/{recurrence_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! device_characteristic_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;
			let characteristic_id =
				create_device_characteristic(&ctx, case_id, drug_id).await?;

			put_ok(
				&ctx,
				device_characteristic_field($canonical),
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/device-characteristics/{characteristic_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				device_characteristic_field($canonical),
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/device-characteristics/{characteristic_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! assessment_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;
			let reaction_id = create_reaction(&ctx, case_id).await?;
			let assessment_id = create_assessment_with_payload(
				&ctx,
				case_id,
				drug_id,
				reaction_id,
				$payload,
			)
			.await?;

			let value = get_ok(
				&ctx,
				drug_reaction_assessment_field($canonical),
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! relatedness_single_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;
			let reaction_id = create_reaction(&ctx, case_id).await?;
			let assessment_id =
				create_assessment(&ctx, case_id, drug_id, reaction_id).await?;
			let relatedness_id =
				create_relatedness(&ctx, case_id, drug_id, assessment_id).await?;

			put_ok(
				&ctx,
				relatedness_field($canonical),
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{relatedness_id}"
				),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				relatedness_field($canonical),
				format!(
					"/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}/relatedness/{relatedness_id}"
				),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

macro_rules! drug_json_field_test {
	($name:ident, $canonical:literal, $payload:expr, $assert:expr) => {
		#[tokio::test]
		#[serial]
		async fn $name() -> Result<()> {
			let ctx = setup().await?;
			let case_id = create_case(&ctx).await?;
			let drug_id = create_drug(&ctx, case_id).await?;

			put_ok(
				&ctx,
				drug_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}"),
				json!({ "data": $payload }),
			)
			.await?;

			let value = get_ok(
				&ctx,
				drug_field($canonical),
				format!("/api/cases/{case_id}/drugs/{drug_id}"),
			)
			.await?;
			($assert)(&value);
			Ok(())
		}
	};
}

drug_single_field_test!(
	save_g_k_drug_characterization_only,
	"G.k.1",
	json!({"drug_characterization": "2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "drug_characterization", "2");
	}
);
drug_single_field_test!(
	save_g_k_medicinal_product_only,
	"G.k.2.2",
	json!({"medicinal_product": "Drug 2"}),
	|value| {
		assert_str(value, "medicinal_product", "Drug 2");
	}
);
drug_single_field_test!(
	save_g_k_brand_name_only,
	"G.k.brand_name",
	json!({"brand_name": "Brand"}),
	|value| {
		assert_str(value, "brand_name", "Brand");
	}
);
drug_single_field_test!(
	save_g_k_drug_generic_name_only,
	"G.k.drug_generic_name",
	json!({"drug_generic_name": "Generic"}),
	|value| {
		assert_str(value, "drug_generic_name", "Generic");
	}
);
drug_single_field_test!(
	save_g_k_drug_authorization_number_only,
	"G.k.3.1",
	json!({"drug_authorization_number": "AUTH"}),
	|value| {
		assert_str(value, "drug_authorization_number", "AUTH");
	}
);
drug_single_field_test!(
	save_g_k_manufacturer_name_only,
	"G.k.3.3",
	json!({"manufacturer_name": "Maker"}),
	|value| {
		assert_str(value, "manufacturer_name", "Maker");
	}
);
drug_single_field_test!(
	save_g_k_manufacturer_country_only,
	"G.k.3.2",
	json!({"manufacturer_country": "KR"}),
	|value| {
		assert_str(value, "manufacturer_country", "KR");
	}
);
drug_single_field_test!(
	save_g_k_batch_lot_number_only,
	"G.k.3.4",
	json!({"batch_lot_number": "LOT"}),
	|value| {
		assert_str(value, "batch_lot_number", "LOT");
	}
);
drug_single_field_test!(
	save_g_k_cumulative_dose_first_reaction_value_only,
	"G.k.cumulative_dose_first_reaction_value",
	json!({"cumulative_dose_first_reaction_value": 150.0}),
	|value| {
		assert_f64(value, "cumulative_dose_first_reaction_value", 150.0);
	}
);
drug_single_field_test!(
	save_g_k_cumulative_dose_first_reaction_unit_only,
	"G.k.cumulative_dose_first_reaction_unit",
	json!({"cumulative_dose_first_reaction_unit": "mg"}),
	|value| {
		assert_str(value, "cumulative_dose_first_reaction_unit", "mg");
	}
);
drug_single_field_test!(
	save_g_k_gestation_period_exposure_value_only,
	"G.k.gestation_period_exposure_value",
	json!({"gestation_period_exposure_value": 10.0}),
	|value| {
		assert_f64(value, "gestation_period_exposure_value", 10.0);
	}
);
drug_single_field_test!(
	save_g_k_gestation_period_exposure_unit_only,
	"G.k.gestation_period_exposure_unit",
	json!({"gestation_period_exposure_unit": "wk"}),
	|value| {
		assert_str(value, "gestation_period_exposure_unit", "wk");
	}
);
drug_single_field_test!(
	save_g_k_dosage_text_only,
	"G.k.dosage_text",
	json!({"dosage_text": "Dosage"}),
	|value| {
		assert_str(value, "dosage_text", "Dosage");
	}
);
drug_single_field_test!(
	save_g_k_action_taken_only,
	"G.k.7",
	json!({"action_taken": "1"}),
	|value| {
		assert_str(value, "action_taken", "1");
	}
);
drug_single_field_test!(
	save_g_k_rechallenge_only,
	"G.k.8",
	json!({"rechallenge": "2"}),
	|value| {
		assert_str(value, "rechallenge", "2");
	}
);
drug_single_field_test!(
	save_g_k_investigational_product_blinded_only,
	"G.k.2.5",
	json!({"investigational_product_blinded": false}),
	|value| {
		assert_bool(value, "investigational_product_blinded", false);
	}
);
drug_single_field_test!(
	save_g_k_mpid_only,
	"G.k.2.1.KR.1b",
	json!({"mpid": "MPID"}),
	|value| {
		assert_str(value, "mpid", "MPID");
	}
);
drug_single_field_test!(
	save_g_k_mpid_version_only,
	"G.k.2.1.KR.1a",
	json!({"mpid_version": "1"}),
	|value| {
		assert_str(value, "mpid_version", "1");
	}
);
drug_single_field_test!(
	save_g_k_phpid_only,
	"G.k.2.1.2b",
	json!({"phpid": "PHPID"}),
	|value| {
		assert_str(value, "phpid", "PHPID");
	}
);
drug_single_field_test!(
	save_g_k_phpid_version_only,
	"G.k.2.1.2a",
	json!({"phpid_version": "2"}),
	|value| {
		assert_str(value, "phpid_version", "2");
	}
);
drug_single_field_test!(
	save_g_k_obtain_drug_country_only,
	"G.k.2.4",
	json!({"obtain_drug_country": "US"}),
	|value| {
		assert_str(value, "obtain_drug_country", "US");
	}
);
drug_single_field_test!(
	save_g_k_parent_route_only,
	"G.k.parent_route",
	json!({"parent_route": "oral"}),
	|value| {
		assert_str(value, "parent_route", "oral");
	}
);
drug_single_field_test!(
	save_g_k_parent_route_termid_only,
	"G.k.parent_route_termid",
	json!({"parent_route_termid": "001"}),
	|value| {
		assert_str(value, "parent_route_termid", "001");
	}
);
drug_single_field_test!(
	save_g_k_parent_route_termid_version_only,
	"G.k.parent_route_termid_version",
	json!({"parent_route_termid_version": "1"}),
	|value| {
		assert_str(value, "parent_route_termid_version", "1");
	}
);
drug_single_field_test!(
	save_g_k_parent_dosage_text_only,
	"G.k.parent_dosage_text",
	json!({"parent_dosage_text": "Parent dose"}),
	|value| {
		assert_str(value, "parent_dosage_text", "Parent dose");
	}
);
drug_single_field_test!(
	save_g_k_fda_additional_info_coded_only,
	"FDA.G.k.10a",
	json!({"fda_additional_info_coded": "1"}),
	|value| {
		assert_str(value, "fda_additional_info_coded", "1");
	}
);
drug_single_field_test!(
	save_g_k_drug_additional_info_codes_json_only,
	"G.k.drug_additional_info_codes_json",
	json!({"drug_additional_info_codes_json": ["A", "B"]}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["drug_additional_info_codes_json"],
			json!(["A", "B"])
		);
	}
);
drug_single_field_test!(
	save_g_k_drug_additional_information_only,
	"G.k.11",
	json!({"drug_additional_information": "Additional information"}),
	|value| {
		assert_str(
			value,
			"drug_additional_information",
			"Additional information",
		);
	}
);
drug_single_field_test!(
	save_g_k_fda_specialized_product_category_only,
	"FDA.G.k.10.1",
	json!({"fda_specialized_product_category": "device"}),
	|value| {
		assert_str(value, "fda_specialized_product_category", "device");
	}
);
drug_single_field_test!(
	save_g_k_fda_device_info_json_only,
	"G.k.fda_device_info_json",
	json!({"fda_device_info_json": {"device": "x"}}),
	|value: &serde_json::Value| {
		assert_eq!(value["data"]["fda_device_info_json"], json!({"device":"x"}));
	}
);

drug_create_single_field_test!(
	save_g_k_drug_characterization_on_create_only,
	"G.k.1.create",
	json!({"drug_characterization": "2"}),
	|value| {
		assert_str(value, "drug_characterization", "2");
	}
);
drug_create_single_field_test!(
	save_g_k_medicinal_product_on_create_only,
	"G.k.2.2.create",
	json!({"medicinal_product": "Drug 2"}),
	|value| {
		assert_str(value, "medicinal_product", "Drug 2");
	}
);
drug_create_single_field_test!(
	save_g_k_mpid_on_create_only,
	"G.k.2.1.KR.1b.create",
	json!({"mpid": "MPID"}),
	|value| {
		assert_str(value, "mpid", "MPID");
	}
);
drug_create_single_field_test!(
	save_g_k_mpid_version_on_create_only,
	"G.k.2.1.KR.1a.create",
	json!({"mpid_version": "1"}),
	|value| {
		assert_str(value, "mpid_version", "1");
	}
);
drug_create_single_field_test!(
	save_g_k_phpid_on_create_only,
	"G.k.2.1.2b.create",
	json!({"phpid": "PHPID"}),
	|value| {
		assert_str(value, "phpid", "PHPID");
	}
);
drug_create_single_field_test!(
	save_g_k_phpid_version_on_create_only,
	"G.k.2.1.2a.create",
	json!({"phpid_version": "2"}),
	|value| {
		assert_str(value, "phpid_version", "2");
	}
);
drug_create_single_field_test!(
	save_g_k_obtain_drug_country_on_create_only,
	"G.k.2.4.create",
	json!({"obtain_drug_country": "US"}),
	|value| {
		assert_str(value, "obtain_drug_country", "US");
	}
);
drug_create_single_field_test!(
	save_g_k_investigational_product_blinded_on_create_only,
	"G.k.2.5.create",
	json!({"investigational_product_blinded": false}),
	|value| {
		assert_bool(value, "investigational_product_blinded", false);
	}
);
drug_create_single_field_test!(
	save_g_k_3_1_on_create_only,
	"G.k.3.1.create",
	json!({"drug_authorization_number": "AUTH"}),
	|value| {
		assert_str(value, "drug_authorization_number", "AUTH");
	}
);
drug_create_single_field_test!(
	save_g_k_3_2_on_create_only,
	"G.k.3.2.create",
	json!({"manufacturer_country": "KR"}),
	|value| {
		assert_str(value, "manufacturer_country", "KR");
	}
);
drug_create_single_field_test!(
	save_g_k_3_3_on_create_only,
	"G.k.3.3.create",
	json!({"manufacturer_name": "Maker"}),
	|value| {
		assert_str(value, "manufacturer_name", "Maker");
	}
);
drug_create_single_field_test!(
	save_g_k_3_4_on_create_only,
	"G.k.3.4.create",
	json!({"batch_lot_number": "LOT"}),
	|value| {
		assert_str(value, "batch_lot_number", "LOT");
	}
);
drug_create_single_field_test!(
	save_g_k_7_on_create_only,
	"G.k.7.create",
	json!({"action_taken": "1"}),
	|value| {
		assert_str(value, "action_taken", "1");
	}
);
drug_create_single_field_test!(
	save_g_k_8_on_create_only,
	"G.k.8.create",
	json!({"rechallenge": "2"}),
	|value| {
		assert_str(value, "rechallenge", "2");
	}
);
drug_create_single_field_test!(
	save_fda_g_k_10a_on_create_only,
	"FDA.G.k.10a.create",
	json!({"fda_additional_info_coded": "1"}),
	|value| {
		assert_str(value, "fda_additional_info_coded", "1");
	}
);
drug_create_single_field_test!(
	save_fda_g_k_10_1_on_create_only,
	"FDA.G.k.10.1.create",
	json!({"fda_specialized_product_category": "device"}),
	|value| {
		assert_str(value, "fda_specialized_product_category", "device");
	}
);
drug_create_single_field_test!(
	save_g_k_11_on_create_only,
	"G.k.11.create",
	json!({"drug_additional_information": "Additional information"}),
	|value| {
		assert_str(
			value,
			"drug_additional_information",
			"Additional information",
		);
	}
);

active_substance_single_field_test!(
	save_g_k_2_3_r_substance_name_only,
	"G.k.2.3.r.substance_name",
	json!({"substance_name": "Substance 2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "substance_name", "Substance 2");
	}
);
active_substance_single_field_test!(
	save_g_k_2_3_r_substance_termid_only,
	"G.k.2.3.r.2b",
	json!({"substance_termid": "S2"}),
	|value| {
		assert_str(value, "substance_termid", "S2");
	}
);
active_substance_single_field_test!(
	save_g_k_2_3_r_substance_termid_version_only,
	"G.k.2.3.r.substance_termid_version",
	json!({"substance_termid_version": "2"}),
	|value| {
		assert_str(value, "substance_termid_version", "2");
	}
);
active_substance_single_field_test!(
	save_g_k_2_3_r_strength_value_only,
	"G.k.2.3.r.strength_value",
	json!({"strength_value": 2.0}),
	|value| {
		assert_f64(value, "strength_value", 2.0);
	}
);
active_substance_single_field_test!(
	save_g_k_2_3_r_strength_unit_only,
	"G.k.2.3.r.strength_unit",
	json!({"strength_unit": "g"}),
	|value| {
		assert_str(value, "strength_unit", "g");
	}
);

dosage_single_field_test!(
	save_g_k_4_r_dose_value_only,
	"G.k.4.r.1a",
	json!({"dose_value": 2.0}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_f64(value, "dose_value", 2.0);
	}
);
dosage_single_field_test!(
	save_g_k_4_r_dose_unit_only,
	"G.k.4.r.1b",
	json!({"dose_unit": "g"}),
	|value| {
		assert_str(value, "dose_unit", "g");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_number_of_units_only,
	"G.k.4.r.2",
	json!({"number_of_units": 3}),
	|value| {
		assert_i64(value, "number_of_units", 3);
	}
);
dosage_single_field_test!(
	save_g_k_4_r_frequency_value_only,
	"G.k.4.r.frequency_value",
	json!({"frequency_value": 2.0}),
	|value| {
		assert_f64(value, "frequency_value", 2.0);
	}
);
dosage_single_field_test!(
	save_g_k_4_r_frequency_unit_only,
	"G.k.4.r.3",
	json!({"frequency_unit": "wk"}),
	|value| {
		assert_str(value, "frequency_unit", "wk");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_first_administration_date_only,
	"G.k.4.r.4",
	json!({"first_administration_date": [2024, 2, 1]}),
	|value| {
		assert_date_tuple(value, "first_administration_date", &[2024, 32]);
	}
);
dosage_single_field_test!(
	save_g_k_4_r_first_administration_date_null_flavor_only,
	"G.k.4.r.first_administration_date_null_flavor",
	json!({"first_administration_date_null_flavor": "NI"}),
	|value| {
		assert_str(value, "first_administration_date_null_flavor", "NI");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_last_administration_date_only,
	"G.k.4.r.5",
	json!({"last_administration_date": [2024, 2, 2]}),
	|value| {
		assert_date_tuple(value, "last_administration_date", &[2024, 33]);
	}
);
dosage_single_field_test!(
	save_g_k_4_r_last_administration_date_null_flavor_only,
	"G.k.4.r.last_administration_date_null_flavor",
	json!({"last_administration_date_null_flavor": "UNK"}),
	|value| {
		assert_str(value, "last_administration_date_null_flavor", "UNK");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_duration_value_only,
	"G.k.4.r.6a",
	json!({"duration_value": 3.0}),
	|value| {
		assert_f64(value, "duration_value", 3.0);
	}
);
dosage_single_field_test!(
	save_g_k_4_r_duration_unit_only,
	"G.k.4.r.6b",
	json!({"duration_unit": "wk"}),
	|value| {
		assert_str(value, "duration_unit", "wk");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_continuing_only,
	"DG_CONT",
	json!({"continuing": true}),
	|value| {
		assert_bool(value, "continuing", true);
	}
);
dosage_single_field_test!(
	save_g_k_4_r_batch_lot_number_only,
	"G.k.4.r.7",
	json!({"batch_lot_number": "LOT2"}),
	|value| {
		assert_str(value, "batch_lot_number", "LOT2");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_dosage_text_only,
	"G.k.4.r.8",
	json!({"dosage_text": "Dose 2"}),
	|value| {
		assert_str(value, "dosage_text", "Dose 2");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_dose_form_only,
	"G.k.4.r.9.1",
	json!({"dose_form": "Capsule"}),
	|value| {
		assert_str(value, "dose_form", "Capsule");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_9_2b_only,
	"G.k.4.r.9.2b",
	json!({"dose_form_termid": "DF2"}),
	|value| {
		assert_str(value, "dose_form_termid", "DF2");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_9_2a_only,
	"G.k.4.r.9.2a",
	json!({"dose_form_termid_version": "2"}),
	|value| {
		assert_str(value, "dose_form_termid_version", "2");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_route_of_administration_only,
	"G.k.4.r.10.1",
	json!({"route_of_administration": "IV"}),
	|value| {
		assert_str(value, "route_of_administration", "IV");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_10_2a_only,
	"G.k.4.r.10.2a",
	json!({"route_termid_version": "2"}),
	|value| {
		assert_str(value, "route_termid_version", "2");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_10_2b_only,
	"G.k.4.r.10.2b",
	json!({"route_termid": "RT2"}),
	|value| {
		assert_str(value, "route_termid", "RT2");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_parent_route_only,
	"G.k.4.r.11.1",
	json!({"parent_route": "iv"}),
	|value| {
		assert_str(value, "parent_route", "iv");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_11_2b_only,
	"G.k.4.r.11.2b",
	json!({"parent_route_termid": "002"}),
	|value| {
		assert_str(value, "parent_route_termid", "002");
	}
);
dosage_single_field_test!(
	save_g_k_4_r_11_2a_only,
	"G.k.4.r.11.2a",
	json!({"parent_route_termid_version": "2"}),
	|value| {
		assert_str(value, "parent_route_termid_version", "2");
	}
);

indication_single_field_test!(
	save_g_k_7_r_1_only,
	"G.k.7.r.1",
	json!({"indication_text": "Indication 2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "indication_text", "Indication 2");
	}
);
indication_single_field_test!(
	save_g_k_7_r_2a_only,
	"G.k.7.r.2a",
	json!({"indication_meddra_version": "28.0"}),
	|value| {
		assert_str(value, "indication_meddra_version", "28.0");
	}
);
indication_single_field_test!(
	save_g_k_7_r_2b_only,
	"G.k.7.r.2b",
	json!({"indication_meddra_code": "901"}),
	|value| {
		assert_str(value, "indication_meddra_code", "901");
	}
);

recurrence_single_field_test!(
	save_g_k_8_r_rechallenge_action_only,
	"G.k.8.r",
	json!({"rechallenge_action": "1"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "rechallenge_action", "1");
	}
);
recurrence_single_field_test!(
	save_g_k_8_r_reaction_meddra_version_only,
	"G.k.8.r.reaction_meddra_version",
	json!({"reaction_meddra_version": "27.0"}),
	|value| {
		assert_str(value, "reaction_meddra_version", "27.0");
	}
);
recurrence_single_field_test!(
	save_g_k_8_r_reaction_meddra_code_only,
	"G.k.8.r.reaction_meddra_code",
	json!({"reaction_meddra_code": "100"}),
	|value| {
		assert_str(value, "reaction_meddra_code", "100");
	}
);
recurrence_single_field_test!(
	save_g_k_8_r_reaction_recurred_only,
	"G.k.8.r.reaction_recurred",
	json!({"reaction_recurred": "2"}),
	|value| {
		assert_str(value, "reaction_recurred", "2");
	}
);

device_characteristic_single_field_test!(
	save_g_k_10_code_only,
	"G.k.10.code",
	json!({"code": "C2"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "code", "C2");
	}
);
device_characteristic_single_field_test!(
	save_g_k_10_code_system_only,
	"G.k.10.code_system",
	json!({"code_system": "CS2"}),
	|value| {
		assert_str(value, "code_system", "CS2");
	}
);
device_characteristic_single_field_test!(
	save_g_k_10_code_display_name_only,
	"G.k.10.code_display_name",
	json!({"code_display_name": "Device 2"}),
	|value| {
		assert_str(value, "code_display_name", "Device 2");
	}
);
device_characteristic_single_field_test!(
	save_g_k_10_value_type_only,
	"G.k.10.value_type",
	json!({"value_type": "ST"}),
	|value| {
		assert_str(value, "value_type", "ST");
	}
);
device_characteristic_single_field_test!(
	save_g_k_10_value_value_only,
	"G.k.10.value_value",
	json!({"value_value": "Value 2"}),
	|value| {
		assert_str(value, "value_value", "Value 2");
	}
);
device_characteristic_single_field_test!(
	save_g_k_10_value_code_only,
	"G.k.10.value_code",
	json!({"value_code": "VC2"}),
	|value| {
		assert_str(value, "value_code", "VC2");
	}
);
device_characteristic_single_field_test!(
	save_g_k_10_value_code_system_only,
	"G.k.10.value_code_system",
	json!({"value_code_system": "VCS2"}),
	|value| {
		assert_str(value, "value_code_system", "VCS2");
	}
);
device_characteristic_single_field_test!(
	save_g_k_10_value_display_name_only,
	"G.k.10.value_display_name",
	json!({"value_display_name": "VD2"}),
	|value| {
		assert_str(value, "value_display_name", "VD2");
	}
);

assessment_single_field_test!(
	save_g_k_9_i_administration_start_interval_value_only,
	"G.k.9.i.3.1a",
	json!({"administration_start_interval_value": 2.0}),
	|value| {
		assert_f64(value, "administration_start_interval_value", 2.0);
	}
);
assessment_single_field_test!(
	save_g_k_9_i_administration_start_interval_unit_only,
	"G.k.9.i.3.1b",
	json!({"administration_start_interval_unit": "d"}),
	|value| {
		assert_str(value, "administration_start_interval_unit", "d");
	}
);
assessment_single_field_test!(
	save_g_k_9_i_last_dose_interval_value_only,
	"G.k.9.i.3.2a",
	json!({"last_dose_interval_value": 1.0}),
	|value| {
		assert_f64(value, "last_dose_interval_value", 1.0);
	}
);
assessment_single_field_test!(
	save_g_k_9_i_last_dose_interval_unit_only,
	"G.k.9.i.3.2b",
	json!({"last_dose_interval_unit": "h"}),
	|value| {
		assert_str(value, "last_dose_interval_unit", "h");
	}
);
assessment_single_field_test!(
	save_g_k_9_i_recurrence_action_only,
	"G.k.9.i.4",
	json!({"recurrence_action": "3"}),
	|value| {
		assert_str(value, "recurrence_action", "3");
	}
);
assessment_single_field_test!(
	save_g_k_9_i_recurrence_meddra_version_only,
	"G.k.9.i.recurrence_meddra_version",
	json!({"recurrence_meddra_version": "27.0"}),
	|value| {
		assert_str(value, "recurrence_meddra_version", "27.0");
	}
);
assessment_single_field_test!(
	save_g_k_9_i_recurrence_meddra_code_only,
	"G.k.9.i.recurrence_meddra_code",
	json!({"recurrence_meddra_code": "100"}),
	|value| {
		assert_str(value, "recurrence_meddra_code", "100");
	}
);
assessment_single_field_test!(
	save_g_k_9_i_reaction_recurred_only,
	"G.k.9.i.reaction_recurred",
	json!({"reaction_recurred": "1"}),
	|value| {
		assert_str(value, "reaction_recurred", "1");
	}
);

#[tokio::test]
#[serial]
async fn save_g_k_9_i_reaction_id_on_create_only() -> Result<()> {
	let ctx = setup().await?;
	let case_id = create_case(&ctx).await?;
	let drug_id = create_drug(&ctx, case_id).await?;
	let reaction_id = create_reaction(&ctx, case_id).await?;
	let assessment_id =
		create_assessment(&ctx, case_id, drug_id, reaction_id).await?;

	let value = get_ok(
		&ctx,
		drug_reaction_assessment_field("G.k.9.i.1"),
		format!("/api/cases/{case_id}/drugs/{drug_id}/reaction-assessments/{assessment_id}"),
	)
	.await?;

	assert_eq!(value["data"]["reaction_id"], json!(reaction_id));
	Ok(())
}

relatedness_single_field_test!(
	save_g_k_9_i_2_r_source_of_assessment_only,
	"G.k.9.i.2.r.1",
	json!({"source_of_assessment": "Sponsor"}),
	|value| {
		assert_i64(value, "sequence_number", 1);
		assert_str(value, "source_of_assessment", "Sponsor");
	}
);
relatedness_single_field_test!(
	save_g_k_9_i_2_r_method_of_assessment_only,
	"G.k.9.i.2.r.2",
	json!({"method_of_assessment": "Naranjo"}),
	|value| {
		assert_str(value, "method_of_assessment", "Naranjo");
	}
);
relatedness_single_field_test!(
	save_g_k_9_i_2_r_result_of_assessment_only,
	"G.k.9.i.2.r.3",
	json!({"result_of_assessment": "not related"}),
	|value| {
		assert_str(value, "result_of_assessment", "not related");
	}
);
relatedness_single_field_test!(
	save_g_k_9_i_2_r_result_of_assessment_kr2_only,
	"G.k.9.i.2.r.result_of_assessment_kr2",
	json!({"result_of_assessment_kr2": "KR2"}),
	|value| {
		assert_str(value, "result_of_assessment_kr2", "KR2");
	}
);

drug_json_field_test!(
	save_g_k_10_r_only,
	"G.k.10.r",
	json!({"drug_additional_info_codes_json": [{"value_code": "A1"}]}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["drug_additional_info_codes_json"],
			json!([{"value_code":"A1"}])
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_1_malfunction_only,
	"FDA.G.k.12.r.1",
	json!({"fda_device_info_json": {"malfunction": true}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["malfunction"],
			json!(true)
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_2_r_only,
	"FDA.G.k.12.r.2.r",
	json!({"fda_device_info_json": {"follow_up_types": [{"value_code": "FUT"}]}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["follow_up_types"],
			json!([{"value_code":"FUT"}])
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_3_r_only,
	"FDA.G.k.12.r.3.r",
	json!({"fda_device_info_json": {"device_problem_codes": [{"value_code": "DPC"}]}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["device_problem_codes"],
			json!([{"value_code":"DPC"}])
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_4_device_brand_name_only,
	"FDA.G.k.12.r.4",
	json!({"fda_device_info_json": {"device_brand_name": "Brand"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["device_brand_name"],
			json!("Brand")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_5_common_device_name_only,
	"FDA.G.k.12.r.5",
	json!({"fda_device_info_json": {"common_device_name": "Common"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["common_device_name"],
			json!("Common")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_6_device_product_code_only,
	"FDA.G.k.12.r.6",
	json!({"fda_device_info_json": {"device_product_code": "P01"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["device_product_code"],
			json!("P01")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_7_1a_manufacturer_name_only,
	"FDA.G.k.12.r.7.1a",
	json!({"fda_device_info_json": {"manufacturer_name": "Maker"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["manufacturer_name"],
			json!("Maker")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_7_1b_manufacturer_address_only,
	"FDA.G.k.12.r.7.1b",
	json!({"fda_device_info_json": {"manufacturer_address": "Addr"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["manufacturer_address"],
			json!("Addr")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_7_1c_manufacturer_city_only,
	"FDA.G.k.12.r.7.1c",
	json!({"fda_device_info_json": {"manufacturer_city": "Seoul"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["manufacturer_city"],
			json!("Seoul")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_7_1d_manufacturer_state_only,
	"FDA.G.k.12.r.7.1d",
	json!({"fda_device_info_json": {"manufacturer_state": "CA"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["manufacturer_state"],
			json!("CA")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_7_1e_manufacturer_country_only,
	"FDA.G.k.12.r.7.1e",
	json!({"fda_device_info_json": {"manufacturer_country": "US"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["manufacturer_country"],
			json!("US")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_8_device_usage_only,
	"FDA.G.k.12.r.8",
	json!({"fda_device_info_json": {"device_usage": "2"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["device_usage"],
			json!("2")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_9_device_lot_number_only,
	"FDA.G.k.12.r.9",
	json!({"fda_device_info_json": {"device_lot_number": "LOT-1"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["device_lot_number"],
			json!("LOT-1")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_10_operator_of_device_only,
	"FDA.G.k.12.r.10",
	json!({"fda_device_info_json": {"operator_of_device": "1"}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["operator_of_device"],
			json!("1")
		);
	}
);
drug_json_field_test!(
	save_fda_g_k_12_r_11_r_only,
	"FDA.G.k.12.r.11.r",
	json!({"fda_device_info_json": {"remedial_actions": [{"value_code": "RA"}]}}),
	|value: &serde_json::Value| {
		assert_eq!(
			value["data"]["fda_device_info_json"]["remedial_actions"],
			json!([{"value_code":"RA"}])
		);
	}
);
