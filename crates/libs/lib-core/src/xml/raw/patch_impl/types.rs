use sqlx::types::time::Date;
use sqlx::types::time::OffsetDateTime;

pub struct CSafetyReportPatch<'a> {
	pub report_unique_id: &'a str,
	pub transmission_date: Date,
	pub transmission_date_value: Option<&'a str>,
	pub transmission_date_time: Option<OffsetDateTime>,
	pub report_type: &'a str,
	pub date_first_received: Date,
	pub date_most_recent: Date,
	pub fulfil_expedited: bool,
	pub worldwide_unique_id: Option<&'a str>,
	pub local_criteria_report_type: Option<&'a str>,
	pub combination_product_indicator: Option<&'a str>,
	pub nullification_code: Option<&'a str>,
	pub nullification_reason: Option<&'a str>,
	// C.3 Sender information (best-effort; patch only when values are provided)
	pub sender_type: Option<&'a str>,
	pub sender_org_name: Option<&'a str>,
	pub sender_department: Option<&'a str>,
	pub sender_street_address: Option<&'a str>,
	pub sender_city: Option<&'a str>,
	pub sender_state: Option<&'a str>,
	pub sender_postcode: Option<&'a str>,
	pub sender_country_code: Option<&'a str>,
	pub sender_person_title: Option<&'a str>,
	pub sender_person_given_name: Option<&'a str>,
	pub sender_person_middle_name: Option<&'a str>,
	pub sender_person_family_name: Option<&'a str>,
	pub sender_telephone: Option<&'a str>,
	pub sender_fax: Option<&'a str>,
	pub sender_email: Option<&'a str>,
}

pub struct DPatientPatch<'a> {
	pub patient_name: Option<&'a str>,
	pub sex: Option<&'a str>,
	pub birth_date: Option<Date>,
	pub age_value: Option<&'a str>,
	pub age_unit: Option<&'a str>,
	pub weight_kg: Option<&'a str>,
	pub height_cm: Option<&'a str>,
}
