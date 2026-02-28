use lib_core::model::narrative::NarrativeInformation;
use lib_core::xml::raw::patch::patch_h_narrative;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::Uuid;
use time::OffsetDateTime;

#[test]
fn patch_h_narrative_updates_raw_xml() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/refs/instances/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let narrative = NarrativeInformation {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		case_narrative: "Updated narrative".to_string(),
		reporter_comments: Some("Reporter note".to_string()),
		sender_comments: None,
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let patched = patch_h_narrative(&xml, &narrative).expect("patch");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let text = xpath
		.findvalue("//hl7:investigationEvent/hl7:text", None)
		.unwrap();
	assert!(text.contains("Updated narrative"));
}
