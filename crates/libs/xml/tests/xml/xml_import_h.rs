use xml::import_sections::h_narrative::parse_h_narrative;

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
	assert!(narrative
		.case_narrative
		.as_deref()
		.is_some_and(|text| !text.trim().is_empty()));
}

#[test]
fn import_h_narrative_preserves_absent_section() {
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

	assert!(parse_h_narrative(xml).expect("absent section H").is_none());
}
