use crate::common::{date, fixture};
use lib_core::xml::import_sections::e_reaction::parse_e_reactions;
use rust_decimal::Decimal;
use sqlx::types::Uuid;

#[test]
fn import_e_section_all_fields_from_scenario6() {
	let xml = fixture("FAERS2022Scenario6.xml");

	let reactions = parse_e_reactions(&xml).expect("parse");
	assert_eq!(reactions.len(), 2);

	let first = &reactions[0];
	assert_eq!(
		first.xml_id,
		Some(
			Uuid::parse_str("154eb889-958b-45f2-a02f-42d4d6f4657f")
				.expect("valid uuid"),
		)
	);
	assert_eq!(first.primary_source_reaction, "consciência alterada");
	assert_eq!(
		first.primary_source_reaction_translation.as_deref(),
		Some("Altered Consciousness")
	);
	assert_eq!(first.reaction_language.as_deref(), Some("por"));
	assert_eq!(first.reaction_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(first.reaction_meddra_code.as_deref(), Some("10027940"));
	assert_eq!(first.term_highlighted, Some(false));
	assert_eq!(first.serious, Some(true));
	assert_eq!(first.criteria_death, Some(true));
	assert_eq!(first.criteria_death_null_flavor, None);
	assert_eq!(first.criteria_life_threatening, None);
	assert_eq!(
		first.criteria_life_threatening_null_flavor.as_deref(),
		Some("NI")
	);
	assert_eq!(first.criteria_hospitalization, Some(true));
	assert_eq!(first.criteria_hospitalization_null_flavor, None);
	assert_eq!(first.criteria_disabling, None);
	assert_eq!(first.criteria_disabling_null_flavor.as_deref(), Some("NI"));
	assert_eq!(first.criteria_congenital_anomaly, None);
	assert_eq!(
		first.criteria_congenital_anomaly_null_flavor.as_deref(),
		Some("NI")
	);
	assert_eq!(first.criteria_other_medically_important, Some(true));
	assert_eq!(first.criteria_other_medically_important_null_flavor, None);
	assert_eq!(first.required_intervention, None);
	assert_eq!(first.start_date, Some(date(2009, 1, 1)));
	assert_eq!(first.start_date_null_flavor, None);
	assert_eq!(first.end_date, Some(date(2009, 1, 2)));
	assert_eq!(first.end_date_null_flavor, None);
	assert_eq!(first.duration_value, None::<Decimal>);
	assert_eq!(first.duration_unit, None);
	assert_eq!(first.outcome.as_deref(), Some("3"));
	assert_eq!(first.medical_confirmation, Some(true));
	assert_eq!(first.country_code.as_deref(), Some("EU"));

	let second = &reactions[1];
	assert_eq!(
		second.xml_id,
		Some(
			Uuid::parse_str("2baa28d6-c9e8-4e6c-93e9-5b860b314220")
				.expect("valid uuid"),
		)
	);
	assert_eq!(second.primary_source_reaction, "");
	assert_eq!(second.primary_source_reaction_translation, None);
	assert_eq!(second.reaction_language, None);
	assert_eq!(second.reaction_meddra_version.as_deref(), Some("12.0"));
	assert_eq!(second.reaction_meddra_code.as_deref(), Some("10024381"));
	assert_eq!(second.term_highlighted, None);
	assert_eq!(second.serious, Some(true));
	assert_eq!(second.criteria_death, None);
	assert_eq!(second.criteria_death_null_flavor.as_deref(), Some("NI"));
	assert_eq!(second.criteria_life_threatening, None);
	assert_eq!(
		second.criteria_life_threatening_null_flavor.as_deref(),
		Some("NI")
	);
	assert_eq!(second.criteria_hospitalization, None);
	assert_eq!(
		second.criteria_hospitalization_null_flavor.as_deref(),
		Some("NI")
	);
	assert_eq!(second.criteria_disabling, Some(true));
	assert_eq!(second.criteria_disabling_null_flavor, None);
	assert_eq!(second.criteria_congenital_anomaly, None);
	assert_eq!(
		second.criteria_congenital_anomaly_null_flavor.as_deref(),
		Some("NI")
	);
	assert_eq!(second.criteria_other_medically_important, Some(true));
	assert_eq!(second.criteria_other_medically_important_null_flavor, None);
	assert_eq!(second.required_intervention, None);
	assert_eq!(second.start_date, None);
	assert_eq!(second.start_date_null_flavor.as_deref(), Some("NASK"));
	assert_eq!(second.end_date, None);
	assert_eq!(second.end_date_null_flavor, None);
	assert_eq!(second.duration_value, None::<Decimal>);
	assert_eq!(second.duration_unit, None);
	assert_eq!(second.outcome.as_deref(), Some("3"));
	assert_eq!(second.medical_confirmation, None);
	assert_eq!(second.country_code, None);
}
