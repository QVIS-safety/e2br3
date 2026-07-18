use lib_core::model::reaction::Reaction;
use lib_core::xml::export::sections::e::export_e_reactions_xml;
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
		deleted: false,
		primary_source_reaction: "Headache".to_string(),
		primary_source_reaction_translation: None,
		reaction_language: Some("en".to_string()),
		reaction_meddra_version: Some("24.1".to_string()),
		reaction_meddra_code: Some("10019211".to_string()),
		term_highlighted: Some(true),
		serious: Some(false),
		criteria_death: false,
		criteria_death_null_flavor: None,
		criteria_life_threatening: false,
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: false,
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: false,
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: false,
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: false,
		criteria_other_medically_important_null_flavor: None,
		required_intervention: None,
		required_intervention_null_flavor: None,
		expectedness: None,
		severity: None,
		mfds_device_ae_classification: None,
		mfds_device_ae_outcome: None,
		mfds_device_cause_medical_device: None,
		mfds_device_cause_procedure_issue: None,
		mfds_device_cause_patient_condition: None,
		mfds_device_cause_unable_to_assess: None,
		mfds_device_cause_other: None,
		mfds_device_action_reason: None,
		mfds_device_action_recall: None,
		mfds_device_action_repair: None,
		mfds_device_action_inspection: None,
		mfds_device_action_replacement: None,
		mfds_device_action_improvement: None,
		mfds_device_action_monitoring: None,
		mfds_device_action_notification: None,
		mfds_device_action_label_change: None,
		mfds_device_action_other: None,
		start_date: Some(Date::from_calendar_date(2024, Month::January, 2).unwrap()),
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
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
fn export_e_reaction_preserves_known_extension_fields() {
	let reaction = Reaction {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		deleted: false,
		primary_source_reaction: "Device site pain & swelling".to_string(),
		primary_source_reaction_translation: None,
		reaction_language: Some("en".to_string()),
		reaction_meddra_version: Some("24.1".to_string()),
		reaction_meddra_code: Some("10019211".to_string()),
		term_highlighted: Some(true),
		serious: Some(false),
		criteria_death: false,
		criteria_death_null_flavor: None,
		criteria_life_threatening: false,
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: false,
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: false,
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: false,
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: false,
		criteria_other_medically_important_null_flavor: None,
		required_intervention: Some("true".to_string()),
		required_intervention_null_flavor: None,
		expectedness: Some("2".to_string()),
		severity: Some("severe".to_string()),
		mfds_device_ae_classification: Some("0".to_string()),
		mfds_device_ae_outcome: Some("10".to_string()),
		mfds_device_cause_medical_device: Some(true),
		mfds_device_cause_procedure_issue: None,
		mfds_device_cause_patient_condition: None,
		mfds_device_cause_unable_to_assess: None,
		mfds_device_cause_other: Some("Other cause <device>".to_string()),
		mfds_device_action_reason: Some("Action reason & notes".to_string()),
		mfds_device_action_recall: Some(true),
		mfds_device_action_repair: None,
		mfds_device_action_inspection: None,
		mfds_device_action_replacement: None,
		mfds_device_action_improvement: None,
		mfds_device_action_monitoring: None,
		mfds_device_action_notification: None,
		mfds_device_action_label_change: Some(false),
		mfds_device_action_other: Some("Other action".to_string()),
		start_date: None,
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
		duration_value: None,
		duration_unit: None,
		outcome: Some("1".to_string()),
		medical_confirmation: None,
		country_code: None,
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

	assert!(!xml.contains("AE_IME_LIST"));
	let expectedness = xpath
		.findvalue(
			"//hl7:observation[hl7:code[@code='AE_EXPECTEDNESS']]/hl7:value/@code",
			None,
		)
		.unwrap();
	assert_eq!(expectedness, "2");
	let severity = xpath
		.findvalue(
			"//hl7:observation[hl7:code[@code='AE_SEVERITY']]/hl7:value/@code",
			None,
		)
		.unwrap();
	assert_eq!(severity, "severe");
	let classification = xpath
		.findvalue(
			"//hl7:observation[hl7:code[@code='KR_DVC_AECL']]/hl7:value/@code",
			None,
		)
		.unwrap();
	assert_eq!(classification, "0");
	let cause_other = xpath
		.findvalue(
			"//hl7:observation[hl7:code[@code='KR_DVC_CC_OTH']]/hl7:value",
			None,
		)
		.unwrap();
	assert_eq!(cause_other, "Other cause <device>");
	let action_label_change = xpath
		.findvalue(
			"//hl7:observation[hl7:code[@code='KR_DVC_ACT_CAS']]/hl7:value/@value",
			None,
		)
		.unwrap();
	assert_eq!(action_label_change, "false");

	let required_intervention = xpath
		.findvalue(
			"//hl7:observation[hl7:code[@code='7']]/hl7:value/@value",
			None,
		)
		.unwrap();
	assert_eq!(required_intervention, "true");
}

#[test]
fn export_e_reaction_requires_outcome() {
	let reaction = Reaction {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		deleted: false,
		primary_source_reaction: "Headache".to_string(),
		primary_source_reaction_translation: None,
		reaction_language: Some("en".to_string()),
		reaction_meddra_version: Some("24.1".to_string()),
		reaction_meddra_code: Some("10019211".to_string()),
		term_highlighted: Some(true),
		serious: Some(false),
		criteria_death: false,
		criteria_death_null_flavor: None,
		criteria_life_threatening: false,
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: false,
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: false,
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: false,
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: false,
		criteria_other_medically_important_null_flavor: None,
		required_intervention: Some("true".to_string()),
		required_intervention_null_flavor: None,
		expectedness: None,
		severity: None,
		mfds_device_ae_classification: None,
		mfds_device_ae_outcome: None,
		mfds_device_cause_medical_device: None,
		mfds_device_cause_procedure_issue: None,
		mfds_device_cause_patient_condition: None,
		mfds_device_cause_unable_to_assess: None,
		mfds_device_cause_other: None,
		mfds_device_action_reason: None,
		mfds_device_action_recall: None,
		mfds_device_action_repair: None,
		mfds_device_action_inspection: None,
		mfds_device_action_replacement: None,
		mfds_device_action_improvement: None,
		mfds_device_action_monitoring: None,
		mfds_device_action_notification: None,
		mfds_device_action_label_change: None,
		mfds_device_action_other: None,
		start_date: None,
		start_date_null_flavor: None,
		end_date: None,
		end_date_null_flavor: None,
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
