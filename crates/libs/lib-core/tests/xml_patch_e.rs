use lib_core::model::reaction::Reaction;
use lib_core::xml::raw::patch::patch_e_reactions;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::Uuid;
use time::OffsetDateTime;

#[test]
fn patch_e_reaction_updates_raw_xml() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let reaction = Reaction {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		sequence_number: 1,
		primary_source_reaction: "Headache".to_string(),
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
		start_date: None,
		end_date: None,
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
