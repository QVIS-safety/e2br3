use crate::has_text;
use lib_core::model::safety_report::PrimarySource;

pub fn has_any_primary_source_content(source: &PrimarySource) -> bool {
	has_text(source.reporter_title.as_deref())
		|| has_text(source.reporter_title_null_flavor.as_deref())
		|| has_text(source.reporter_given_name.as_deref())
		|| has_text(source.reporter_given_name_null_flavor.as_deref())
		|| has_text(source.reporter_middle_name.as_deref())
		|| has_text(source.reporter_middle_name_null_flavor.as_deref())
		|| has_text(source.reporter_family_name.as_deref())
		|| has_text(source.reporter_family_name_null_flavor.as_deref())
		|| has_text(source.organization.as_deref())
		|| has_text(source.organization_null_flavor.as_deref())
		|| has_text(source.department.as_deref())
		|| has_text(source.department_null_flavor.as_deref())
		|| has_text(source.street.as_deref())
		|| has_text(source.street_null_flavor.as_deref())
		|| has_text(source.city.as_deref())
		|| has_text(source.city_null_flavor.as_deref())
		|| has_text(source.state.as_deref())
		|| has_text(source.state_null_flavor.as_deref())
		|| has_text(source.postcode.as_deref())
		|| has_text(source.postcode_null_flavor.as_deref())
		|| has_text(source.telephone.as_deref())
		|| has_text(source.telephone_null_flavor.as_deref())
		|| has_text(source.country_code.as_deref())
		|| has_text(source.email.as_deref())
		|| has_text(source.qualification.as_deref())
		|| has_text(source.primary_source_regulatory.as_deref())
}

#[cfg(test)]
mod tests {
	use super::*;
	use sqlx::types::time::OffsetDateTime;
	use sqlx::types::Uuid;

	#[test]
	fn primary_source_payload_false_when_all_empty() {
		let source = PrimarySource {
			id: Default::default(),
			case_id: Default::default(),
			source_reporter_presave_id: None,
			sequence_number: 1,
			deleted: false,
			reporter_title: None,
			reporter_title_null_flavor: None,
			reporter_given_name: None,
			reporter_given_name_null_flavor: None,
			reporter_middle_name: None,
			reporter_middle_name_null_flavor: None,
			reporter_family_name: None,
			reporter_family_name_null_flavor: None,
			organization: None,
			organization_null_flavor: None,
			department: None,
			department_null_flavor: None,
			street: None,
			street_null_flavor: None,
			city: None,
			city_null_flavor: None,
			state: None,
			state_null_flavor: None,
			postcode: None,
			postcode_null_flavor: None,
			country_code: None,
			country_code_null_flavor: None,
			telephone: None,
			telephone_null_flavor: None,
			email: None,
			email_null_flavor: None,
			qualification: None,
			qualification_null_flavor: None,
			qualification_kr1: None,
			primary_source_regulatory: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::nil(),
			updated_by: None,
		};
		assert!(!has_any_primary_source_content(&source));
	}

	#[test]
	fn primary_source_payload_true_when_element_null_flavor_present() {
		let source = PrimarySource {
			id: Default::default(),
			case_id: Default::default(),
			source_reporter_presave_id: None,
			sequence_number: 1,
			deleted: false,
			reporter_title: None,
			reporter_title_null_flavor: None,
			reporter_given_name: None,
			reporter_given_name_null_flavor: Some("ASKU".to_string()),
			reporter_middle_name: None,
			reporter_middle_name_null_flavor: None,
			reporter_family_name: None,
			reporter_family_name_null_flavor: None,
			organization: None,
			organization_null_flavor: None,
			department: None,
			department_null_flavor: None,
			street: None,
			street_null_flavor: None,
			city: None,
			city_null_flavor: None,
			state: None,
			state_null_flavor: None,
			postcode: None,
			postcode_null_flavor: None,
			country_code: None,
			country_code_null_flavor: None,
			telephone: None,
			telephone_null_flavor: None,
			email: None,
			email_null_flavor: None,
			qualification: None,
			qualification_null_flavor: None,
			qualification_kr1: None,
			primary_source_regulatory: None,
			created_at: OffsetDateTime::now_utc(),
			updated_at: OffsetDateTime::now_utc(),
			created_by: Uuid::nil(),
			updated_by: None,
		};
		assert!(has_any_primary_source_content(&source));
	}
}
