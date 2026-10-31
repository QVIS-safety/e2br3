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
#[path = "tests/c_reporter_policy.rs"]
mod tests;
