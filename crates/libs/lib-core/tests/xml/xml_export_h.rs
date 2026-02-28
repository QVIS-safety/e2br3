use lib_core::model::narrative::NarrativeInformation;
use lib_core::xml::export_sections::h_narrative::export_h_narrative_xml;
use libxml::parser::Parser;
use libxml::xpath::Context;
use sqlx::types::Uuid;
use time::OffsetDateTime;

#[test]
fn export_h_narrative_basic() {
	let narrative = NarrativeInformation {
		id: Uuid::new_v4(),
		case_id: Uuid::new_v4(),
		case_narrative: "Patient improved.".to_string(),
		reporter_comments: Some("Reporter note".to_string()),
		sender_comments: Some("Sender note".to_string()),
		created_at: OffsetDateTime::now_utc(),
		updated_at: OffsetDateTime::now_utc(),
		created_by: Uuid::new_v4(),
		updated_by: None,
	};

	let xml = export_h_narrative_xml(&narrative).expect("export xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&xml).expect("parse");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let text = xpath
		.findvalue("//hl7:investigationEvent/hl7:text", None)
		.unwrap();
	assert_eq!(text, "Patient improved.");
}
