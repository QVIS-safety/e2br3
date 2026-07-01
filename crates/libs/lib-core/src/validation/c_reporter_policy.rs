use crate::model::safety_report::PrimarySource;
use crate::validation::has_text;

pub fn has_any_primary_source_content(source: &PrimarySource) -> bool {
	has_text(source.reporter_title.as_deref())
		|| has_text(source.reporter_given_name.as_deref())
		|| has_text(source.reporter_middle_name.as_deref())
		|| has_text(source.reporter_family_name.as_deref())
		|| has_text(source.organization.as_deref())
		|| has_text(source.department.as_deref())
		|| has_text(source.street.as_deref())
		|| has_text(source.city.as_deref())
		|| has_text(source.state.as_deref())
		|| has_text(source.postcode.as_deref())
		|| has_text(source.telephone.as_deref())
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
			reporter_given_name: None,
			reporter_middle_name: None,
			reporter_family_name: None,
			reporter_name_null_flavor: None,
			organization: None,
			department: None,
			street: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			reporter_address_null_flavor: None,
			email: None,
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
	fn primary_source_payload_true_when_any_present() {
		let source = PrimarySource {
			id: Default::default(),
			case_id: Default::default(),
			source_reporter_presave_id: None,
			sequence_number: 1,
			deleted: false,
			reporter_title: None,
			reporter_given_name: Some("Jane".to_string()),
			reporter_middle_name: None,
			reporter_family_name: None,
			reporter_name_null_flavor: None,
			organization: None,
			department: None,
			street: None,
			city: None,
			state: None,
			postcode: None,
			country_code: None,
			telephone: None,
			reporter_address_null_flavor: None,
			email: None,
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
