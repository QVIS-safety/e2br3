use super::common::{date, dec, finish, setup_case};
use crate::test_common::Result;
use lib_core::model::reaction::{ReactionBmc, ReactionForCreate, ReactionForUpdate};
use serial_test::serial;
use time::Month;

#[tokio::test]
#[serial]
async fn save_e_i_create() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	let row = ReactionBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.primary_source_reaction, "Headache");
	assert_eq!(row.primary_source_reaction_translation, None);
	assert_eq!(row.reaction_language, None);
	assert_eq!(row.reaction_meddra_version, None);
	assert_eq!(row.reaction_meddra_code, None);
	assert_eq!(row.term_highlighted, None);
	assert_eq!(row.serious, None);
	assert_eq!(row.criteria_death, false);
	assert_eq!(row.criteria_death_null_flavor, None);
	assert_eq!(row.criteria_life_threatening, false);
	assert_eq!(row.criteria_life_threatening_null_flavor, None);
	assert_eq!(row.criteria_hospitalization, false);
	assert_eq!(row.criteria_hospitalization_null_flavor, None);
	assert_eq!(row.criteria_disabling, false);
	assert_eq!(row.criteria_disabling_null_flavor, None);
	assert_eq!(row.criteria_congenital_anomaly, false);
	assert_eq!(row.criteria_congenital_anomaly_null_flavor, None);
	assert_eq!(row.criteria_other_medically_important, false);
	assert_eq!(row.criteria_other_medically_important_null_flavor, None);
	assert_eq!(row.required_intervention, None);
	assert_eq!(row.start_date, None);
	assert_eq!(row.start_date_null_flavor, None);
	assert_eq!(row.end_date, None);
	assert_eq!(row.end_date_null_flavor, None);
	assert_eq!(row.duration_value, None);
	assert_eq!(row.duration_unit, None);
	assert_eq!(row.outcome, None);
	assert_eq!(row.medical_confirmation, None);
	assert_eq!(row.country_code, None);
	finish(&mm).await
}

#[tokio::test]
#[serial]
async fn save_e_i_update() -> Result<()> {
	let (mm, ctx, case_id) = setup_case().await?;
	let id = ReactionBmc::create(
		&ctx,
		&mm,
		ReactionForCreate {
			case_id,
			sequence_number: 1,
			primary_source_reaction: "Headache".to_string(),
		},
	)
	.await?;
	ReactionBmc::update(
		&ctx,
		&mm,
		id,
		ReactionForUpdate {
			primary_source_reaction: Some("Migraine".to_string()),
			primary_source_reaction_translation: Some("Migraine EN".to_string()),
			reaction_language: Some("ko".to_string()),
			reaction_meddra_code: Some("100".to_string()),
			reaction_meddra_version: Some("27.0".to_string()),
			term_highlighted: Some(true),
			serious: Some(true),
			criteria_death: Some(true),
			criteria_death_null_flavor: None,
			criteria_life_threatening: Some(true),
			criteria_life_threatening_null_flavor: None,
			criteria_hospitalization: Some(true),
			criteria_hospitalization_null_flavor: None,
			criteria_disabling: Some(true),
			criteria_disabling_null_flavor: None,
			criteria_congenital_anomaly: Some(true),
			criteria_congenital_anomaly_null_flavor: None,
			criteria_other_medically_important: Some(true),
			criteria_other_medically_important_null_flavor: None,
			required_intervention: Some("1".to_string()),
			start_date: Some(date(2024, Month::January, 1)),
			start_date_null_flavor: None,
			end_date: Some(date(2024, Month::January, 2)),
			end_date_null_flavor: None,
			duration_value: Some(dec(2, 0)),
			duration_unit: Some("d".to_string()),
			outcome: Some("1".to_string()),
			medical_confirmation: Some(true),
			country_code: Some("KR".to_string()),
		},
	)
	.await?;
	let row = ReactionBmc::get(&ctx, &mm, id).await?;
	assert_eq!(row.case_id, case_id);
	assert_eq!(row.sequence_number, 1);
	assert_eq!(row.primary_source_reaction, "Migraine");
	assert_eq!(
		row.primary_source_reaction_translation.as_deref(),
		Some("Migraine EN")
	);
	assert_eq!(row.reaction_language.as_deref(), Some("ko"));
	assert_eq!(row.reaction_meddra_version.as_deref(), Some("27.0"));
	assert_eq!(row.reaction_meddra_code.as_deref(), Some("100"));
	assert_eq!(row.term_highlighted, Some(true));
	assert_eq!(row.serious, Some(true));
	assert_eq!(row.criteria_death, true);
	assert_eq!(row.criteria_death_null_flavor, None);
	assert_eq!(row.criteria_life_threatening, true);
	assert_eq!(row.criteria_life_threatening_null_flavor, None);
	assert_eq!(row.criteria_hospitalization, true);
	assert_eq!(row.criteria_hospitalization_null_flavor, None);
	assert_eq!(row.criteria_disabling, true);
	assert_eq!(row.criteria_disabling_null_flavor, None);
	assert_eq!(row.criteria_congenital_anomaly, true);
	assert_eq!(row.criteria_congenital_anomaly_null_flavor, None);
	assert_eq!(row.criteria_other_medically_important, true);
	assert_eq!(row.criteria_other_medically_important_null_flavor, None);
	assert_eq!(row.required_intervention.as_deref(), Some("1"));
	assert_eq!(row.start_date, Some(date(2024, Month::January, 1)));
	assert_eq!(row.start_date_null_flavor, None);
	assert_eq!(row.end_date, Some(date(2024, Month::January, 2)));
	assert_eq!(row.end_date_null_flavor, None);
	assert_eq!(row.duration_value, Some(dec(2, 0)));
	assert_eq!(row.duration_unit.as_deref(), Some("d"));
	assert_eq!(row.outcome.as_deref(), Some("1"));
	assert_eq!(row.medical_confirmation, Some(true));
	assert_eq!(row.country_code.as_deref(), Some("KR"));
	finish(&mm).await
}
