use lib_core::xml::import_sections::h_narrative::parse_h_narrative;
use lib_core::xml::Error as XmlError;

#[test]
fn import_h_narrative_basic() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let narrative = parse_h_narrative(&xml).expect("parse").unwrap();
	assert!(!narrative.case_narrative.trim().is_empty());
}

#[test]
fn import_h_narrative_requires_case_narrative() {
	let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess classCode="CACT" moodCode="EVN">
      <subject typeCode="SUBJ">
        <investigationEvent classCode="INVSTG" moodCode="EVN"/>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>"#;

	let err = parse_h_narrative(xml).expect_err("missing narrative should fail");
	match err {
		XmlError::InvalidXml { message, .. } => {
			assert!(message.contains("ICH.H.1.REQUIRED"));
		}
		other => panic!("unexpected error type: {other:?}"),
	}
}
