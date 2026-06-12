use lib_core::xml::import_sections::c_safety_report::parse_c_safety_report;
use lib_core::xml::Error as XmlError;

#[test]
fn import_c_safety_report_basic() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");

	let report = parse_c_safety_report(&xml).expect("parse").unwrap();
	assert!(!report.report_type.trim().is_empty());
}

#[test]
fn import_c_safety_report_requires_report_type() {
	let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess classCode="CACT" moodCode="EVN">
      <effectiveTime value="20260101"/>
      <subject typeCode="SUBJ">
        <investigationEvent classCode="INVSTG" moodCode="EVN">
          <effectiveTime><low value="20260101"/></effectiveTime>
          <availabilityTime value="20260101"/>
        </investigationEvent>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>"#;

	let err =
		parse_c_safety_report(xml).expect_err("missing report type should fail");
	match err {
		XmlError::InvalidXml { message, .. } => {
			assert!(message.contains("ICH.C.1.3.REQUIRED"));
		}
		other => panic!("unexpected error type: {other:?}"),
	}
}
