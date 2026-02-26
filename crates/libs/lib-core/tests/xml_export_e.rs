use lib_core::model::reaction::Reaction;
use lib_core::xml::export_sections::e_reaction::export_e_reactions_xml;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::time::Date;
use sqlx::types::Uuid;
use time::Month;
use time::OffsetDateTime;

#[test]
fn export_e_reaction_basic() {
	let reaction = Reaction {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		primary_source_reaction: "Headache".to_string(),
		primary_source_reaction_translation: None,
		reaction_language: Some("en".to_string()),
		reaction_meddra_version: Some("24.1".to_string()),
		reaction_meddra_code: Some("10019211".to_string()),
		term_highlighted: Some(true),
		serious: Some(false),
		criteria_death: false,
		criteria_life_threatening: false,
		criteria_hospitalization: false,
		criteria_disabling: false,
		criteria_congenital_anomaly: false,
		criteria_other_medically_important: false,
		required_intervention: None,
		start_date: Some(Date::from_calendar_date(2024, Month::January, 2).unwrap()),
		end_date: None,
		duration_value: None,
		duration_unit: None,
		outcome: Some("1".to_string()),
		medical_confirmation: Some(true),
		country_code: Some("US".to_string()),
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let xml = export_e_reactions_xml(&[reaction]).expect("export xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&xml).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let text = xpath
		.findvalue("//hl7:subjectOf2/hl7:observation/hl7:value/@code", None)
		.unwrap();
	assert_eq!(text, "10019211");

	let outcome_code = xpath
		.findvalue(
			"//hl7:subjectOf2/hl7:observation/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='27']]/hl7:value/@code",
			None,
		)
		.unwrap();
	assert_eq!(outcome_code, "1");

	let required_intervention_null_flavor = xpath
		.findvalue(
			"//hl7:subjectOf2/hl7:observation/hl7:outboundRelationship2/hl7:observation[hl7:code[@code='7']]/hl7:value/@nullFlavor",
			None,
		)
		.unwrap();
	assert_eq!(required_intervention_null_flavor, "NI");
}

#[test]
fn export_e_reaction_requires_outcome() {
	let reaction = Reaction {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		primary_source_reaction: "Headache".to_string(),
		primary_source_reaction_translation: None,
		reaction_language: Some("en".to_string()),
		reaction_meddra_version: Some("24.1".to_string()),
		reaction_meddra_code: Some("10019211".to_string()),
		term_highlighted: Some(true),
		serious: Some(false),
		criteria_death: false,
		criteria_life_threatening: false,
		criteria_hospitalization: false,
		criteria_disabling: false,
		criteria_congenital_anomaly: false,
		criteria_other_medically_important: false,
		required_intervention: Some("true".to_string()),
		start_date: None,
		end_date: None,
		duration_value: None,
		duration_unit: None,
		outcome: None,
		medical_confirmation: None,
		country_code: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let err = export_e_reactions_xml(&[reaction])
		.expect_err("missing outcome should fail");
	let msg = format!("{err}");
	assert!(msg.contains("ICH.E.i.7.REQUIRED"));
}
