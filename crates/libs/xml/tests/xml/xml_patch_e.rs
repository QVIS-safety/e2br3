use lib_core::model::reaction::Reaction;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::Uuid;
use time::OffsetDateTime;
use xml::export::roundtrip::patch_e_reactions;

#[test]
fn patch_e_reaction_updates_raw_xml() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let reaction = Reaction {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		deleted: false,
		primary_source_reaction: Some("Headache".to_string()),
		primary_source_reaction_translation: None,
		reaction_language: Some("en".to_string()),
		reaction_meddra_version: Some("24.1".to_string()),
		reaction_meddra_code: Some("10019211".to_string()),
		term_highlighted: Some("1".to_string()),
		serious: Some(false),
		criteria_death: Some(false),
		criteria_death_null_flavor: None,
		criteria_life_threatening: Some(false),
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: Some(false),
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: Some(false),
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: Some(false),
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: Some(false),
		criteria_other_medically_important_null_flavor: None,
		required_intervention: None,
		required_intervention_null_flavor: None,
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
		outcome: Some("3".to_string()),
		medical_confirmation: None,
		country_code: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let patched = patch_e_reactions(&xml, &[reaction]).expect("patch");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let code = xpath
		.findvalue(
			"//hl7:subjectOf2/hl7:observation/hl7:value[@codeSystem='2.16.840.1.113883.6.163']/@code",
			None,
		)
		.unwrap();
	assert_eq!(code, "10019211");
}

#[test]
fn patch_e_reaction_uses_comp_nodes_for_sxpr_effective_time() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let start_date = "20240102".to_string();
	let end_date = "20240105".to_string();

	let reaction = Reaction {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		deleted: false,
		primary_source_reaction: Some("Headache".to_string()),
		primary_source_reaction_translation: None,
		reaction_language: Some("en".to_string()),
		reaction_meddra_version: Some("24.1".to_string()),
		reaction_meddra_code: Some("10019211".to_string()),
		term_highlighted: Some("3".to_string()),
		serious: Some(false),
		criteria_death: Some(false),
		criteria_death_null_flavor: None,
		criteria_life_threatening: Some(false),
		criteria_life_threatening_null_flavor: None,
		criteria_hospitalization: Some(false),
		criteria_hospitalization_null_flavor: None,
		criteria_disabling: Some(false),
		criteria_disabling_null_flavor: None,
		criteria_congenital_anomaly: Some(false),
		criteria_congenital_anomaly_null_flavor: None,
		criteria_other_medically_important: Some(false),
		criteria_other_medically_important_null_flavor: None,
		required_intervention: None,
		required_intervention_null_flavor: None,
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
		start_date: Some(start_date),
		start_date_null_flavor: None,
		end_date: Some(end_date),
		end_date_null_flavor: None,
		duration_value: Some("3".to_string()),
		duration_unit: Some("d".to_string()),
		outcome: Some("3".to_string()),
		medical_confirmation: None,
		country_code: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let patched = patch_e_reactions(&xml, &[reaction]).expect("patch");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	xpath
		.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
		.unwrap();

	let low_under_comp = xpath
		.findnodes(
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='29']]/hl7:effectiveTime/hl7:comp/hl7:low",
			None,
		)
		.expect("low under comp query")
		.len();
	let direct_low = xpath
		.findnodes(
			"//hl7:subjectOf2/hl7:observation[hl7:code[@code='29']]/hl7:effectiveTime/hl7:low",
			None,
		)
		.expect("direct low query")
		.len();
	assert!(low_under_comp > 0);
	assert_eq!(direct_low, 0);
}
