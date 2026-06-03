// Section G - Drug/Biological Information

use crate::ctx::Ctx;
use crate::model::base::base_uuid;
use crate::model::base::DbBmc;
use crate::model::modql_utils::uuid_to_sea_value;
use crate::model::store::set_full_context_dbx_or_rollback;
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValsValue};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use sqlx::types::time::{Date, OffsetDateTime, Time};
use sqlx::types::Uuid;
use sqlx::FromRow;

// -- DrugInformation

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DrugInformation {
	pub id: Uuid,
	pub case_id: Uuid,
	pub source_product_presave_id: Option<Uuid>,
	pub sequence_number: i32,

	// G.k.1 - Drug role (MANDATORY)
	pub drug_characterization: String,

	// G.k.2.2 - Product name
	pub medicinal_product: String,

	// G.k.2.4-5 - Product identifiers
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_mpid_version: Option<String>,
	pub mfds_mpid: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	// G.k.2.5 - Investigational Product Blinded
	pub investigational_product_blinded: Option<bool>,

	// G.k.3.1 - Obtain Drug Country
	pub obtain_drug_country: Option<String>,

	// G.k.3.2 - Brand Name
	pub brand_name: Option<String>,
	pub drug_generic_name: Option<String>,
	pub drug_authorization_number: Option<String>,

	// G.k.3.3 - Manufacturer
	pub manufacturer_name: Option<String>,
	pub manufacturer_country: Option<String>,

	// G.k.3.4 - Batch/Lot Number
	pub batch_lot_number: Option<String>,

	// G.k.5 - Cumulative Dose to First Reaction
	pub cumulative_dose_first_reaction_value: Option<Decimal>,
	pub cumulative_dose_first_reaction_unit: Option<String>,

	// G.k.6 - Gestation Period at Time of Exposure
	pub gestation_period_exposure_value: Option<Decimal>,
	pub gestation_period_exposure_unit: Option<String>,

	// G.k.4.r.8 / legacy app-level dosage text
	pub dosage_text: Option<String>,

	// G.k.7 - Action taken
	pub action_taken: Option<String>,

	// G.k.8 - Rechallenge/Recurrence
	pub rechallenge: Option<String>,

	// G.k.10 - Parent Route
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,

	// G.k.11 - Parent Dosage
	pub parent_dosage_text: Option<String>,

	// FDA.G.k.10a - Additional Information on Drug (coded)
	pub fda_additional_info_coded: Option<String>,
	pub drug_additional_info_codes_json: Option<JsonValue>,
	pub drug_additional_information: Option<String>,
	pub fda_specialized_product_category: Option<String>,
	pub fda_device_info_json: Option<JsonValue>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Default, Deserialize)]
pub struct DrugInformationForCreate {
	pub case_id: Uuid,
	pub source_product_presave_id: Option<Uuid>,
	pub sequence_number: i32,
	pub drug_characterization: String,
	pub medicinal_product: String,
	pub drug_generic_name: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub brand_name: Option<String>,
	pub manufacturer_name: Option<String>,
	pub manufacturer_country: Option<String>,
	pub batch_lot_number: Option<String>,
	pub cumulative_dose_first_reaction_value: Option<Decimal>,
	pub cumulative_dose_first_reaction_unit: Option<String>,
	pub gestation_period_exposure_value: Option<Decimal>,
	pub gestation_period_exposure_unit: Option<String>,
	pub dosage_text: Option<String>,
	pub action_taken: Option<String>,
	pub rechallenge: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_mpid_version: Option<String>,
	pub mfds_mpid: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub obtain_drug_country: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub parent_dosage_text: Option<String>,
	pub fda_additional_info_coded: Option<String>,
	pub drug_additional_info_codes_json: Option<JsonValue>,
	pub drug_additional_information: Option<String>,
	pub fda_specialized_product_category: Option<String>,
	pub fda_device_info_json: Option<JsonValue>,
}

#[derive(Deserialize)]
pub struct DrugInformationForUpdate {
	pub source_product_presave_id: Option<Uuid>,
	pub medicinal_product: Option<String>,
	pub drug_characterization: Option<String>,
	pub brand_name: Option<String>,
	pub drug_generic_name: Option<String>,
	pub drug_authorization_number: Option<String>,
	pub manufacturer_name: Option<String>,
	pub manufacturer_country: Option<String>,
	pub batch_lot_number: Option<String>,
	pub cumulative_dose_first_reaction_value: Option<Decimal>,
	pub cumulative_dose_first_reaction_unit: Option<String>,
	pub gestation_period_exposure_value: Option<Decimal>,
	pub gestation_period_exposure_unit: Option<String>,
	pub dosage_text: Option<String>,
	pub action_taken: Option<String>,
	pub rechallenge: Option<String>,
	pub investigational_product_blinded: Option<bool>,
	pub mpid: Option<String>,
	pub mpid_version: Option<String>,
	pub mfds_mpid_version: Option<String>,
	pub mfds_mpid: Option<String>,
	pub phpid: Option<String>,
	pub phpid_version: Option<String>,
	pub obtain_drug_country: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub parent_dosage_text: Option<String>,
	pub fda_additional_info_coded: Option<String>,
	pub drug_additional_info_codes_json: Option<JsonValue>,
	pub drug_additional_information: Option<String>,
	pub fda_specialized_product_category: Option<String>,
	pub fda_device_info_json: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FdaDeviceCodeEntry {
	pub value_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrugAdditionalInfoCodeEntry {
	pub value_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FdaDeviceInfoData {
	pub malfunction: Option<bool>,
	#[serde(default)]
	pub follow_up_types: Vec<FdaDeviceCodeEntry>,
	#[serde(default)]
	pub device_problem_codes: Vec<FdaDeviceCodeEntry>,
	pub device_brand_name: Option<String>,
	pub common_device_name: Option<String>,
	pub device_product_code: Option<String>,
	pub manufacturer_name: Option<String>,
	pub manufacturer_address: Option<String>,
	pub manufacturer_city: Option<String>,
	pub manufacturer_state: Option<String>,
	pub manufacturer_country: Option<String>,
	pub device_usage: Option<String>,
	pub device_lot_number: Option<String>,
	pub operator_of_device: Option<String>,
	#[serde(default)]
	pub remedial_actions: Vec<FdaDeviceCodeEntry>,
}

impl FdaDeviceInfoData {
	pub fn is_empty(&self) -> bool {
		self.malfunction.is_none()
			&& self.follow_up_types.iter().all(|entry| {
				entry
					.value_code
					.as_deref()
					.map(str::trim)
					.unwrap_or("")
					.is_empty()
			}) && self.device_problem_codes.iter().all(|entry| {
			entry
				.value_code
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
		}) && self
			.device_brand_name
			.as_deref()
			.map(str::trim)
			.unwrap_or("")
			.is_empty()
			&& self
				.common_device_name
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.device_product_code
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.manufacturer_name
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.manufacturer_address
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.manufacturer_city
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.manufacturer_state
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.manufacturer_country
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.device_usage
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.device_lot_number
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self
				.operator_of_device
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty()
			&& self.remedial_actions.iter().all(|entry| {
				entry
					.value_code
					.as_deref()
					.map(str::trim)
					.unwrap_or("")
					.is_empty()
			})
	}
}

pub fn parse_fda_device_info_json(
	value: Option<&JsonValue>,
) -> Option<FdaDeviceInfoData> {
	value
		.cloned()
		.and_then(|raw| serde_json::from_value::<FdaDeviceInfoData>(raw).ok())
		.filter(|parsed| !parsed.is_empty())
}

pub fn parse_drug_additional_info_codes_json(
	value: Option<&JsonValue>,
) -> Vec<DrugAdditionalInfoCodeEntry> {
	value
		.cloned()
		.and_then(|raw| {
			serde_json::from_value::<Vec<DrugAdditionalInfoCodeEntry>>(raw).ok()
		})
		.unwrap_or_default()
		.into_iter()
		.filter(|entry| {
			entry
				.value_code
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty() == false
		})
		.collect()
}

fn characteristic_text_entry(
	drug_id: Uuid,
	sequence_number: i32,
	code: &str,
	value: String,
) -> DrugDeviceCharacteristic {
	DrugDeviceCharacteristic {
		id: Uuid::nil(),
		drug_id,
		sequence_number,
		code: Some(code.to_string()),
		code_system: Some("2.16.840.1.113883.3.989.2.1.1.19".to_string()),
		code_display_name: None,
		value_type: Some("ST".to_string()),
		value_value: Some(value),
		value_code: None,
		value_code_system: None,
		value_display_name: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn characteristic_code_entry(
	drug_id: Uuid,
	sequence_number: i32,
	code: &str,
	value_code: String,
) -> DrugDeviceCharacteristic {
	DrugDeviceCharacteristic {
		id: Uuid::nil(),
		drug_id,
		sequence_number,
		code: Some(code.to_string()),
		code_system: Some("2.16.840.1.113883.3.989.2.1.1.19".to_string()),
		code_display_name: None,
		value_type: Some("CE".to_string()),
		value_value: None,
		value_code: Some(value_code),
		value_code_system: None,
		value_display_name: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

fn characteristic_boolean_entry(
	drug_id: Uuid,
	sequence_number: i32,
	code: &str,
	value: bool,
) -> DrugDeviceCharacteristic {
	DrugDeviceCharacteristic {
		id: Uuid::nil(),
		drug_id,
		sequence_number,
		code: Some(code.to_string()),
		code_system: Some("2.16.840.1.113883.3.989.2.1.1.19".to_string()),
		code_display_name: None,
		value_type: Some("BL".to_string()),
		value_value: Some(if value { "true" } else { "false" }.to_string()),
		value_code: None,
		value_code_system: None,
		value_display_name: None,
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		created_by: Uuid::nil(),
		updated_by: None,
	}
}

pub fn derive_fda_device_characteristics(
	drug: &DrugInformation,
) -> Vec<DrugDeviceCharacteristic> {
	fn push_text_row(
		rows: &mut Vec<DrugDeviceCharacteristic>,
		sequence_number: &mut i32,
		drug_id: Uuid,
		code: &str,
		value: Option<&str>,
	) {
		if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
			rows.push(characteristic_text_entry(
				drug_id,
				*sequence_number,
				code,
				value.to_string(),
			));
			*sequence_number += 1;
		}
	}
	fn push_code_row(
		rows: &mut Vec<DrugDeviceCharacteristic>,
		sequence_number: &mut i32,
		drug_id: Uuid,
		code: &str,
		value: Option<&str>,
	) {
		if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
			rows.push(characteristic_code_entry(
				drug_id,
				*sequence_number,
				code,
				value.to_string(),
			));
			*sequence_number += 1;
		}
	}
	fn push_bool_row(
		rows: &mut Vec<DrugDeviceCharacteristic>,
		sequence_number: &mut i32,
		drug_id: Uuid,
		code: &str,
		value: Option<bool>,
	) {
		if let Some(value) = value {
			rows.push(characteristic_boolean_entry(
				drug_id,
				*sequence_number,
				code,
				value,
			));
			*sequence_number += 1;
		}
	}

	let mut rows = Vec::new();
	let mut sequence_number = 1_i32;
	push_code_row(
		&mut rows,
		&mut sequence_number,
		drug.id,
		"FDA.G.k.10.1",
		drug.fda_specialized_product_category.as_deref(),
	);
	for entry in parse_drug_additional_info_codes_json(
		drug.drug_additional_info_codes_json.as_ref(),
	) {
		push_code_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"G.k.10.r",
			entry.value_code.as_deref(),
		);
	}

	if let Some(info) =
		parse_fda_device_info_json(drug.fda_device_info_json.as_ref())
	{
		push_bool_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDA.G.k.12.r.1",
			info.malfunction,
		);
		for entry in info.follow_up_types {
			push_code_row(
				&mut rows,
				&mut sequence_number,
				drug.id,
				"FDA.G.k.12.r.2.r",
				entry.value_code.as_deref(),
			);
		}
		for entry in info.device_problem_codes {
			push_code_row(
				&mut rows,
				&mut sequence_number,
				drug.id,
				"FDA.G.k.12.r.3.r",
				entry.value_code.as_deref(),
			);
		}
		push_text_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R4",
			info.device_brand_name.as_deref(),
		);
		push_text_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R5",
			info.common_device_name.as_deref(),
		);
		push_code_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R6",
			info.device_product_code.as_deref(),
		);
		push_text_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R71A",
			info.manufacturer_name.as_deref(),
		);
		push_text_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R71B",
			info.manufacturer_address.as_deref(),
		);
		push_text_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R71C",
			info.manufacturer_city.as_deref(),
		);
		push_text_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R71D",
			info.manufacturer_state.as_deref(),
		);
		push_code_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R71E",
			info.manufacturer_country.as_deref(),
		);
		push_code_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDA.G.k.12.r.8",
			info.device_usage.as_deref(),
		);
		push_text_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R9",
			info.device_lot_number.as_deref(),
		);
		push_code_row(
			&mut rows,
			&mut sequence_number,
			drug.id,
			"FDAGK12R10",
			info.operator_of_device.as_deref(),
		);
		for entry in info.remedial_actions {
			push_code_row(
				&mut rows,
				&mut sequence_number,
				drug.id,
				"FDA.G.k.12.r.11.r",
				entry.value_code.as_deref(),
			);
		}
	}

	rows
}

pub fn structured_fda_device_info_to_json(
	info: Option<FdaDeviceInfoData>,
) -> Option<JsonValue> {
	info.filter(|value| !value.is_empty())
		.map(|value| json!(value))
}

pub fn structured_drug_additional_info_codes_to_json(
	codes: Vec<DrugAdditionalInfoCodeEntry>,
) -> Option<JsonValue> {
	let filtered: Vec<_> = codes
		.into_iter()
		.filter(|entry| {
			entry
				.value_code
				.as_deref()
				.map(str::trim)
				.unwrap_or("")
				.is_empty() == false
		})
		.collect();
	if filtered.is_empty() {
		None
	} else {
		Some(json!(filtered))
	}
}

// -- DrugActiveSubstance

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugActiveSubstance {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,

	// G.k.2.3.r.1 - Substance Name
	pub substance_name: Option<String>,

	// G.k.2.3.r.2 - Substance TermID
	pub substance_termid: Option<String>,
	pub substance_termid_version: Option<String>,

	// G.k.2.3.r.1.KR.1a/b - MFDS substance fields
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,

	// G.k.2.3.r.3 - Strength
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DrugActiveSubstanceForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub substance_name: Option<String>,
	pub substance_termid: Option<String>,
	pub substance_termid_version: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DrugActiveSubstanceForUpdate {
	pub substance_name: Option<String>,
	pub substance_termid: Option<String>,
	pub substance_termid_version: Option<String>,
	pub mfds_version: Option<String>,
	pub mfds_id: Option<String>,
	pub strength_value: Option<Decimal>,
	pub strength_unit: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DrugActiveSubstanceFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- DosageInformation

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DosageInformation {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,

	// G.k.4.r.1 - Dose
	pub dose_value: Option<Decimal>,
	pub dose_unit: Option<String>,

	// G.k.4.r.2 - Number of Separate Dosages
	pub number_of_units: Option<i32>,

	// G.k.4.r.3 - Dose Frequency
	pub frequency_value: Option<Decimal>,
	pub frequency_unit: Option<String>,

	// G.k.4.r.4 - Date/Time of First Administration
	pub first_administration_date: Option<Date>,
	pub first_administration_time: Option<Time>,

	// G.k.4.r.5 - Date/Time of Last Administration
	pub last_administration_date: Option<Date>,
	pub last_administration_time: Option<Time>,

	// G.k.4.r.6 - Duration
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub continuing: Option<bool>,

	// G.k.4.r.7 - Batch/Lot Number
	pub batch_lot_number: Option<String>,

	// G.k.4.r.8 - Dosage Text
	pub dosage_text: Option<String>,

	// G.k.4.r.9.1 - Pharmaceutical Dose Form
	pub dose_form: Option<String>,
	pub dose_form_termid: Option<String>,
	pub dose_form_termid_version: Option<String>,

	// G.k.4.r.10 - Route of Administration
	pub route_of_administration: Option<String>,
	pub route_termid: Option<String>,
	pub route_termid_version: Option<String>,

	// G.k.4.r.11 - Parent Route
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub first_administration_date_null_flavor: Option<String>,
	pub last_administration_date_null_flavor: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DosageInformationForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub dose_value: Option<Decimal>,
	pub dose_unit: Option<String>,
	pub number_of_units: Option<i32>,
	pub frequency_value: Option<Decimal>,
	pub frequency_unit: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub first_administration_date: Option<Date>,
	pub first_administration_time: Option<Time>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub last_administration_date: Option<Date>,
	pub last_administration_time: Option<Time>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub continuing: Option<bool>,
	pub batch_lot_number: Option<String>,
	pub dosage_text: Option<String>,
	pub dose_form: Option<String>,
	pub dose_form_termid: Option<String>,
	pub dose_form_termid_version: Option<String>,
	pub route_of_administration: Option<String>,
	pub route_termid: Option<String>,
	pub route_termid_version: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub first_administration_date_null_flavor: Option<String>,
	pub last_administration_date_null_flavor: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DosageInformationForUpdate {
	pub dose_value: Option<Decimal>,
	pub dose_unit: Option<String>,
	pub number_of_units: Option<i32>,
	pub frequency_value: Option<Decimal>,
	pub frequency_unit: Option<String>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub first_administration_date: Option<Date>,
	pub first_administration_time: Option<Time>,
	#[serde(
		default,
		deserialize_with = "crate::serde::flex_date::deserialize_option_date"
	)]
	pub last_administration_date: Option<Date>,
	pub last_administration_time: Option<Time>,
	pub duration_value: Option<Decimal>,
	pub duration_unit: Option<String>,
	pub continuing: Option<bool>,
	pub batch_lot_number: Option<String>,
	pub dosage_text: Option<String>,
	pub dose_form: Option<String>,
	pub dose_form_termid: Option<String>,
	pub dose_form_termid_version: Option<String>,
	pub route_of_administration: Option<String>,
	pub route_termid: Option<String>,
	pub route_termid_version: Option<String>,
	pub parent_route: Option<String>,
	pub parent_route_termid: Option<String>,
	pub parent_route_termid_version: Option<String>,
	pub first_administration_date_null_flavor: Option<String>,
	pub last_administration_date_null_flavor: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DosageInformationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- DrugIndication

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugIndication {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,

	// G.k.6.r.1 - Indication (free text)
	pub indication_text: Option<String>,

	// G.k.6.r.2 - Indication (MedDRA coded)
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,

	// Timestamps
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

#[derive(Fields, Deserialize)]
pub struct DrugIndicationForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub indication_text: Option<String>,
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DrugIndicationForUpdate {
	pub indication_text: Option<String>,
	pub indication_meddra_version: Option<String>,
	pub indication_meddra_code: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DrugIndicationFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

#[derive(Fields, Deserialize)]
pub struct DrugDeviceCharacteristicForCreate {
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub code: Option<String>,
	pub code_system: Option<String>,
	pub code_display_name: Option<String>,
	pub value_type: Option<String>,
	pub value_value: Option<String>,
	pub value_code: Option<String>,
	pub value_code_system: Option<String>,
	pub value_display_name: Option<String>,
}

#[derive(Fields, Deserialize)]
pub struct DrugDeviceCharacteristicForUpdate {
	pub code: Option<String>,
	pub code_system: Option<String>,
	pub code_display_name: Option<String>,
	pub value_type: Option<String>,
	pub value_value: Option<String>,
	pub value_code: Option<String>,
	pub value_code_system: Option<String>,
	pub value_display_name: Option<String>,
}

#[derive(FilterNodes, Deserialize, Default)]
pub struct DrugDeviceCharacteristicFilter {
	#[modql(to_sea_value_fn = "uuid_to_sea_value")]
	pub drug_id: Option<OpValsValue>,
	pub sequence_number: Option<OpValsValue>,
}

// -- DrugDeviceCharacteristic (FDA Scenario 7)

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct DrugDeviceCharacteristic {
	pub id: Uuid,
	pub drug_id: Uuid,
	pub sequence_number: i32,
	pub code: Option<String>,
	pub code_system: Option<String>,
	pub code_display_name: Option<String>,
	pub value_type: Option<String>,
	pub value_value: Option<String>,
	pub value_code: Option<String>,
	pub value_code_system: Option<String>,
	pub value_display_name: Option<String>,
	pub created_at: OffsetDateTime,
	pub updated_at: OffsetDateTime,
	pub created_by: Uuid,
	pub updated_by: Option<Uuid>,
}

// -- BMCs

pub struct DrugInformationBmc;
impl DbBmc for DrugInformationBmc {
	const TABLE: &'static str = "drug_information";
}

impl DrugInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		drug_c: DrugInformationForCreate,
	) -> Result<Uuid> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
				"INSERT INTO {} (
				     case_id, source_product_presave_id, sequence_number, drug_characterization, medicinal_product, drug_generic_name,
				     drug_authorization_number, brand_name, manufacturer_name, manufacturer_country,
				     batch_lot_number, cumulative_dose_first_reaction_value, cumulative_dose_first_reaction_unit,
				     gestation_period_exposure_value, gestation_period_exposure_unit, dosage_text,
			     action_taken, rechallenge, investigational_product_blinded, mpid, mpid_version,
			     mfds_mpid_version, mfds_mpid, phpid, phpid_version, obtain_drug_country, parent_route, parent_route_termid,
			     parent_route_termid_version, parent_dosage_text, fda_additional_info_coded,
			     drug_additional_info_codes_json, drug_additional_information, fda_specialized_product_category, fda_device_info_json,
			     created_at, updated_at, created_by
				 )
				 VALUES (
				     $1, $2, $3, $4, $5,
				     $6, $7, $8, $9, $10,
				     $11, $12, $13,
				     $14, $15, $16,
				     $17, $18, $19, $20, $21,
				     $22, $23, $24, $25, $26, $27, $28,
				     $29, $30, $31,
				     $32, $33, $34, $35,
				     now(), now(), $36
				 )
				 RETURNING id",
			Self::TABLE
		);
		let (id,) = mm
			.dbx()
			.fetch_one(
				sqlx::query_as::<_, (Uuid,)>(&sql)
					.bind(drug_c.case_id)
					.bind(drug_c.source_product_presave_id)
					.bind(drug_c.sequence_number)
					.bind(drug_c.drug_characterization)
					.bind(drug_c.medicinal_product)
					.bind(drug_c.drug_generic_name)
					.bind(drug_c.drug_authorization_number)
					.bind(drug_c.brand_name)
					.bind(drug_c.manufacturer_name)
					.bind(drug_c.manufacturer_country)
					.bind(drug_c.batch_lot_number)
					.bind(drug_c.cumulative_dose_first_reaction_value)
					.bind(drug_c.cumulative_dose_first_reaction_unit)
					.bind(drug_c.gestation_period_exposure_value)
					.bind(drug_c.gestation_period_exposure_unit)
					.bind(drug_c.dosage_text)
					.bind(drug_c.action_taken)
					.bind(drug_c.rechallenge)
					.bind(drug_c.investigational_product_blinded)
					.bind(drug_c.mpid)
					.bind(drug_c.mpid_version)
					.bind(drug_c.mfds_mpid_version)
					.bind(drug_c.mfds_mpid)
					.bind(drug_c.phpid)
					.bind(drug_c.phpid_version)
					.bind(drug_c.obtain_drug_country)
					.bind(drug_c.parent_route)
					.bind(drug_c.parent_route_termid)
					.bind(drug_c.parent_route_termid_version)
					.bind(drug_c.parent_dosage_text)
					.bind(drug_c.fda_additional_info_coded)
					.bind(drug_c.drug_additional_info_codes_json)
					.bind(drug_c.drug_additional_information)
					.bind(drug_c.fda_specialized_product_category)
					.bind(drug_c.fda_device_info_json)
					.bind(ctx.user_id()),
			)
			.await?;

		mm.dbx().commit_txn().await?;
		Ok(id)
	}

	pub async fn get(
		_ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugInformation> {
		let sql = format!("SELECT * FROM {} WHERE id = $1", Self::TABLE);
		let drug = mm
			.dbx()
			.fetch_optional(sqlx::query_as::<_, DrugInformation>(&sql).bind(id))
			.await?
			.ok_or(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			})?;
		Ok(drug)
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		drug_u: DrugInformationForUpdate,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {}
			 SET medicinal_product = COALESCE($2, medicinal_product),
			     drug_characterization = COALESCE($3, drug_characterization),
			     brand_name = COALESCE($4, brand_name),
			     drug_generic_name = COALESCE($5, drug_generic_name),
			     drug_authorization_number = COALESCE($6, drug_authorization_number),
			     manufacturer_name = COALESCE($7, manufacturer_name),
			     manufacturer_country = COALESCE($8, manufacturer_country),
			     batch_lot_number = COALESCE($9, batch_lot_number),
			     cumulative_dose_first_reaction_value = COALESCE($10, cumulative_dose_first_reaction_value),
			     cumulative_dose_first_reaction_unit = COALESCE($11, cumulative_dose_first_reaction_unit),
			     gestation_period_exposure_value = COALESCE($12, gestation_period_exposure_value),
			     gestation_period_exposure_unit = COALESCE($13, gestation_period_exposure_unit),
			     dosage_text = COALESCE($14, dosage_text),
			     action_taken = COALESCE($15, action_taken),
			     rechallenge = COALESCE($16, rechallenge),
			     investigational_product_blinded = COALESCE($17, investigational_product_blinded),
			     mpid = COALESCE($18, mpid),
			     mpid_version = COALESCE($19, mpid_version),
			     mfds_mpid_version = COALESCE($20, mfds_mpid_version),
			     mfds_mpid = COALESCE($21, mfds_mpid),
			     phpid = COALESCE($22, phpid),
			     phpid_version = COALESCE($23, phpid_version),
			     obtain_drug_country = COALESCE($24, obtain_drug_country),
			     parent_route = COALESCE($25, parent_route),
			     parent_route_termid = COALESCE($26, parent_route_termid),
			     parent_route_termid_version = COALESCE($27, parent_route_termid_version),
			     parent_dosage_text = COALESCE($28, parent_dosage_text),
			     fda_additional_info_coded = COALESCE($29, fda_additional_info_coded),
			     drug_additional_info_codes_json = COALESCE($30, drug_additional_info_codes_json),
			     drug_additional_information = COALESCE($31, drug_additional_information),
			     fda_specialized_product_category = COALESCE($32, fda_specialized_product_category),
				     fda_device_info_json = COALESCE($33, fda_device_info_json),
				     source_product_presave_id = COALESCE($34, source_product_presave_id),
				     updated_at = now(),
				     updated_by = $35
				 WHERE id = $1",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(drug_u.medicinal_product)
					.bind(drug_u.drug_characterization)
					.bind(drug_u.brand_name)
					.bind(drug_u.drug_generic_name)
					.bind(drug_u.drug_authorization_number)
					.bind(drug_u.manufacturer_name)
					.bind(drug_u.manufacturer_country)
					.bind(drug_u.batch_lot_number)
					.bind(drug_u.cumulative_dose_first_reaction_value)
					.bind(drug_u.cumulative_dose_first_reaction_unit)
					.bind(drug_u.gestation_period_exposure_value)
					.bind(drug_u.gestation_period_exposure_unit)
					.bind(drug_u.dosage_text)
					.bind(drug_u.action_taken)
					.bind(drug_u.rechallenge)
					.bind(drug_u.investigational_product_blinded)
					.bind(drug_u.mpid)
					.bind(drug_u.mpid_version)
					.bind(drug_u.mfds_mpid_version)
					.bind(drug_u.mfds_mpid)
					.bind(drug_u.phpid)
					.bind(drug_u.phpid_version)
					.bind(drug_u.obtain_drug_country)
					.bind(drug_u.parent_route)
					.bind(drug_u.parent_route_termid)
					.bind(drug_u.parent_route_termid_version)
					.bind(drug_u.parent_dosage_text)
					.bind(drug_u.fda_additional_info_coded)
					.bind(drug_u.drug_additional_info_codes_json)
					.bind(drug_u.drug_additional_information)
					.bind(drug_u.fda_specialized_product_category)
					.bind(drug_u.fda_device_info_json)
					.bind(drug_u.source_product_presave_id)
					.bind(ctx.user_id()),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn list_by_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
	) -> Result<Vec<DrugInformation>> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let sql = format!(
			"SELECT * FROM {} WHERE case_id = $1 ORDER BY sequence_number",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.fetch_all(sqlx::query_as::<_, DrugInformation>(&sql).bind(case_id))
			.await;
		match result {
			Ok(drugs) => {
				mm.dbx().commit_txn().await?;
				Ok(drugs)
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn get_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<DrugInformation> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;
		let sql = format!(
			"SELECT * FROM {} WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.fetch_optional(
				sqlx::query_as::<_, DrugInformation>(&sql)
					.bind(id)
					.bind(case_id),
			)
			.await;
		match result {
			Ok(Some(drug)) => {
				mm.dbx().commit_txn().await?;
				Ok(drug)
			}
			Ok(None) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(crate::model::Error::EntityUuidNotFound {
					entity: Self::TABLE,
					id,
				})
			}
			Err(err) => {
				let _ = mm.dbx().rollback_txn().await;
				Err(err.into())
			}
		}
	}

	pub async fn update_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
		drug_u: DrugInformationForUpdate,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!(
			"UPDATE {}
			 SET medicinal_product = COALESCE($3, medicinal_product),
			     drug_characterization = COALESCE($4, drug_characterization),
			     brand_name = COALESCE($5, brand_name),
			     drug_generic_name = COALESCE($6, drug_generic_name),
			     drug_authorization_number = COALESCE($7, drug_authorization_number),
			     manufacturer_name = COALESCE($8, manufacturer_name),
			     manufacturer_country = COALESCE($9, manufacturer_country),
			     batch_lot_number = COALESCE($10, batch_lot_number),
			     cumulative_dose_first_reaction_value = COALESCE($11, cumulative_dose_first_reaction_value),
			     cumulative_dose_first_reaction_unit = COALESCE($12, cumulative_dose_first_reaction_unit),
			     gestation_period_exposure_value = COALESCE($13, gestation_period_exposure_value),
			     gestation_period_exposure_unit = COALESCE($14, gestation_period_exposure_unit),
			     dosage_text = COALESCE($15, dosage_text),
			     action_taken = COALESCE($16, action_taken),
			     rechallenge = COALESCE($17, rechallenge),
			     investigational_product_blinded = COALESCE($18, investigational_product_blinded),
			     mpid = COALESCE($19, mpid),
			     mpid_version = COALESCE($20, mpid_version),
			     mfds_mpid_version = COALESCE($21, mfds_mpid_version),
			     mfds_mpid = COALESCE($22, mfds_mpid),
			     phpid = COALESCE($23, phpid),
			     phpid_version = COALESCE($24, phpid_version),
			     obtain_drug_country = COALESCE($25, obtain_drug_country),
			     parent_route = COALESCE($26, parent_route),
			     parent_route_termid = COALESCE($27, parent_route_termid),
			     parent_route_termid_version = COALESCE($28, parent_route_termid_version),
			     parent_dosage_text = COALESCE($29, parent_dosage_text),
			     fda_additional_info_coded = COALESCE($30, fda_additional_info_coded),
			     drug_additional_info_codes_json = COALESCE($31, drug_additional_info_codes_json),
			     drug_additional_information = COALESCE($32, drug_additional_information),
			     fda_specialized_product_category = COALESCE($33, fda_specialized_product_category),
			     fda_device_info_json = COALESCE($34, fda_device_info_json),
			     updated_at = now(),
			     updated_by = $35
			 WHERE id = $1 AND case_id = $2",
			Self::TABLE
		);
		let result = mm
			.dbx()
			.execute(
				sqlx::query(&sql)
					.bind(id)
					.bind(case_id)
					.bind(drug_u.medicinal_product)
					.bind(drug_u.drug_characterization)
					.bind(drug_u.brand_name)
					.bind(drug_u.drug_generic_name)
					.bind(drug_u.drug_authorization_number)
					.bind(drug_u.manufacturer_name)
					.bind(drug_u.manufacturer_country)
					.bind(drug_u.batch_lot_number)
					.bind(drug_u.cumulative_dose_first_reaction_value)
					.bind(drug_u.cumulative_dose_first_reaction_unit)
					.bind(drug_u.gestation_period_exposure_value)
					.bind(drug_u.gestation_period_exposure_unit)
					.bind(drug_u.dosage_text)
					.bind(drug_u.action_taken)
					.bind(drug_u.rechallenge)
					.bind(drug_u.investigational_product_blinded)
					.bind(drug_u.mpid)
					.bind(drug_u.mpid_version)
					.bind(drug_u.mfds_mpid_version)
					.bind(drug_u.mfds_mpid)
					.bind(drug_u.phpid)
					.bind(drug_u.phpid_version)
					.bind(drug_u.obtain_drug_country)
					.bind(drug_u.parent_route)
					.bind(drug_u.parent_route_termid)
					.bind(drug_u.parent_route_termid_version)
					.bind(drug_u.parent_dosage_text)
					.bind(drug_u.fda_additional_info_coded)
					.bind(drug_u.drug_additional_info_codes_json)
					.bind(drug_u.drug_additional_information)
					.bind(drug_u.fda_specialized_product_category)
					.bind(drug_u.fda_device_info_json)
					.bind(ctx.user_id()),
			)
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql = format!("DELETE FROM {} WHERE id = $1", Self::TABLE);
		let result = mm.dbx().execute(sqlx::query(&sql).bind(id)).await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}

	pub async fn delete_in_case(
		ctx: &Ctx,
		mm: &ModelManager,
		case_id: Uuid,
		id: Uuid,
	) -> Result<()> {
		mm.dbx().begin_txn().await?;
		set_full_context_dbx_or_rollback(
			mm.dbx(),
			ctx.user_id(),
			ctx.organization_id(),
			ctx.role(),
		)
		.await?;

		let sql =
			format!("DELETE FROM {} WHERE id = $1 AND case_id = $2", Self::TABLE);
		let result = mm
			.dbx()
			.execute(sqlx::query(&sql).bind(id).bind(case_id))
			.await?;
		if result == 0 {
			mm.dbx().rollback_txn().await?;
			return Err(crate::model::Error::EntityUuidNotFound {
				entity: Self::TABLE,
				id,
			});
		}
		mm.dbx().commit_txn().await?;
		Ok(())
	}
}

pub struct DrugActiveSubstanceBmc;
impl DbBmc for DrugActiveSubstanceBmc {
	const TABLE: &'static str = "drug_active_substances";
}

impl DrugActiveSubstanceBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DrugActiveSubstanceForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugActiveSubstance> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DrugActiveSubstanceFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DrugActiveSubstance>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DrugActiveSubstanceForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

pub struct DosageInformationBmc;
impl DbBmc for DosageInformationBmc {
	const TABLE: &'static str = "dosage_information";
}

impl DosageInformationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DosageInformationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DosageInformation> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DosageInformationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DosageInformation>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DosageInformationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

pub struct DrugIndicationBmc;
impl DbBmc for DrugIndicationBmc {
	const TABLE: &'static str = "drug_indications";
}

impl DrugIndicationBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DrugIndicationForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugIndication> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DrugIndicationFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DrugIndication>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DrugIndicationForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}

// -- DrugDeviceCharacteristic BMC

pub struct DrugDeviceCharacteristicBmc;
impl DbBmc for DrugDeviceCharacteristicBmc {
	const TABLE: &'static str = "drug_device_characteristics";
}

impl DrugDeviceCharacteristicBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		data: DrugDeviceCharacteristicForCreate,
	) -> Result<Uuid> {
		base_uuid::create::<Self, _>(ctx, mm, data).await
	}

	pub async fn get(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
	) -> Result<DrugDeviceCharacteristic> {
		base_uuid::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filters: Option<Vec<DrugDeviceCharacteristicFilter>>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<DrugDeviceCharacteristic>> {
		base_uuid::list::<Self, _, _>(ctx, mm, filters, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: Uuid,
		data: DrugDeviceCharacteristicForUpdate,
	) -> Result<()> {
		base_uuid::update::<Self, _>(ctx, mm, id, data).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: Uuid) -> Result<()> {
		base_uuid::delete::<Self>(ctx, mm, id).await
	}
}
