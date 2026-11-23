use xml::import_sections::c_safety_report::parse_c_safety_report;

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
	assert_eq!(report.report_type.as_deref(), Some("1"));
}

#[test]
fn import_c_safety_report_preserves_missing_report_type() {
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

	let report = parse_c_safety_report(xml)
		.expect("draft import")
		.expect("report");
	assert_eq!(report.report_type, None);
}

#[test]
fn exported_draft_preserves_missing_identity_empty_text_false_and_timestamp() {
	use xml::export::roundtrip::{patch_c_safety_report, CSafetyReportPatch};
	let skeleton = include_bytes!("../../src/fixtures/base_export_skeleton.xml");
	for timestamp in [None, Some("20240229"), Some("20240229123059.123+0900")] {
		let patch = CSafetyReportPatch {
			transmission_date: timestamp,
			report_type: Some(""),
			worldwide_unique_id: Some(""),
			fulfil_expedited: Some(false),
			other_case_identifiers_exist_null_flavor: Some("NI"),
			combination_product_indicator: Some(""),
			..Default::default()
		};
		let exported =
			patch_c_safety_report(skeleton, &patch).expect("export draft");
		assert!(
			xml::validation::validate_e2b_xml_for_import(exported.as_bytes(), None)
				.expect("XML structure")
				.ok
		);
		assert_eq!(
			xml::import::extract_safety_report_id_from_xml(exported.as_bytes())
				.expect("identity"),
			None
		);
		let imported = parse_c_safety_report(exported.as_bytes())
			.expect("input contract")
			.expect("report");
		assert_eq!(imported.transmission_date.as_deref(), timestamp);
		assert_eq!(imported.report_type.as_deref(), Some(""));
		assert_eq!(imported.worldwide_unique_id.as_deref(), Some(""));
		assert_eq!(imported.fulfil_expedited_criteria, Some(false));
		assert_eq!(
			imported.combination_product_report_indicator.as_deref(),
			Some("")
		);
		assert_eq!(imported.other_case_identifiers_exist, None);
		assert_eq!(
			imported.other_case_identifiers_exist_null_flavor.as_deref(),
			Some("NI")
		);
	}
}

#[test]
fn exported_drafts_without_identity_or_creation_date_are_not_skipped() {
	use lib_core::model::xml_import_decision::{
		decide_xml_import, XmlImportDecisionAction, XmlImportExistingCase,
		XmlImportIncomingKey,
	};
	for id in [None, Some(""), Some("  "), Some("REPORT")] {
		let existing = XmlImportExistingCase {
			case_id: uuid::Uuid::nil(),
			safety_report_id: id.unwrap_or("").into(),
			version: 1,
			transmission_date: Some("".into()),
		};
		let incoming = XmlImportIncomingKey {
			safety_report_id: id.map(str::to_owned),
			transmission_date: Some("".into()),
		};
		let result = decide_xml_import(&incoming, &[existing], &[]);
		assert_ne!(result.action, XmlImportDecisionAction::Skip);
		if id != Some("REPORT") {
			assert_eq!(result.action, XmlImportDecisionAction::New);
		}
	}
}
