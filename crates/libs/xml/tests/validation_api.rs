use lib_core::regulatory::RegulatoryAuthority;
use xml::validation::{
	default_xsd_path, validate_e2b_xml, validate_e2b_xml_basic,
	validate_e2b_xml_for_import, validate_e2b_xml_xsd, XmlValidatorConfig,
};

#[test]
fn generic_validation_api_is_owned_by_xml() {
	let config = XmlValidatorConfig {
		xsd_path: None,
		..Default::default()
	};
	let report = validate_e2b_xml_basic(
		b"<MCCI_IN200100UV01></MCCI_IN200100UV01>",
		Some(config),
	)
	.expect("basic validation");
	assert!(report.ok);
	let _ = default_xsd_path();
	let _ = validate_e2b_xml;
}

#[test]
fn malformed_xml_is_not_recovered_for_import_or_xsd_validation() {
	let malformed = b"<root>";
	assert!(validate_e2b_xml_for_import(malformed, None).is_err());
	let schema = default_xsd_path().expect("official ICH schema");
	assert!(validate_e2b_xml_xsd(malformed, &schema).is_err());
}

#[test]
fn import_accepts_empty_c_1_8_1_without_rewriting_it() {
	let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../../docs/exporter/fda/FAERS2022Scenario6.xml");
	let source = std::fs::read_to_string(path).expect("prepared FDA XML fixture");
	let xml = source.replace(
		"<id extension=\"US-APHARMA-8744554B\" root=\"2.16.840.1.113883.3.989.2.1.3.2\"/>",
		"<id extension=\"\" root=\"2.16.840.1.113883.3.989.2.1.3.2\"/>",
	);
	assert_ne!(xml, source, "fixture must contain the attribute to clear");
	let schema = default_xsd_path().expect("official ICH schema");
	let before = validate_e2b_xml_xsd(xml.as_bytes(), &schema).expect("validate");
	assert!(before
		.iter()
		.any(|error| error.message.contains("minLength")));

	assert!(
		validate_e2b_xml_for_import(xml.as_bytes(), None)
			.expect("import structure")
			.ok
	);
	let report =
		xml::import_sections::c_safety_report::parse_c_safety_report(xml.as_bytes())
			.expect("input contracts")
			.expect("report");
	assert_eq!(report.worldwide_unique_id.as_deref(), Some(""));
}

#[test]
fn mfds_import_accepts_korean_causality_result_value() {
	let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../../docs/exporter/fda/FAERS2022Scenario6.xml");
	let source = std::fs::read_to_string(path).expect("prepared XML fixture");
	let result = r#"<value xsi:type="ST">Highly related</value>"#;
	assert!(source.contains(result));
	let xml = source.replacen(result, &format!(
		"{result}<value xsi:type=\"CE\" code=\"3\" codeSystem=\"2.16.840.1.113883.3.989.5.1.10.1.5\"/>"
	), 1);
	let normalized = xml.as_bytes();
	let schema = default_xsd_path().expect("prepared XSD");
	assert!(validate_e2b_xml_xsd(&normalized, &schema)
		.unwrap()
		.iter()
		.any(|error| error.message.contains(
			"Element '{urn:hl7-org:v3}value': This element is not expected"
		)));
	let report = validate_e2b_xml_for_import(&normalized, None).expect("validate");
	assert!(report.ok, "unexpected errors: {:?}", report.errors);
	assert!(report.errors.is_empty());
	let invalid = xml.replace("</MCCI_IN200100UV01>", "");
	assert!(validate_e2b_xml_for_import(invalid.as_bytes(), None).is_err());
}

#[test]
fn xml_validation_does_not_run_case_business_rules() {
	let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../../docs/exporter/fda/FAERS2022Scenario6.xml");
	let source = std::fs::read_to_string(path)
		.expect("FDA fixture")
		.replace("US-APHARMA-8744554B", "invalid");
	let config = XmlValidatorConfig {
		authority: Some(RegulatoryAuthority::Fda),
		..Default::default()
	};
	let export_report = validate_e2b_xml(source.as_bytes(), Some(config.clone()))
		.expect("validate export");
	assert!(!export_report.errors.iter().any(|error| {
		matches!(error.code.as_deref(), Some("C.1.1" | "C.1.8.1"))
	}));
	let report = validate_e2b_xml_for_import(source.as_bytes(), Some(config))
		.expect("validate import structure");
	assert!(report.ok, "unexpected errors: {:?}", report.errors);
	assert!(!report.errors.iter().any(|error| {
		matches!(error.code.as_deref(), Some("C.1.1" | "C.1.8.1"))
	}));
}

#[test]
fn import_accepts_empty_code_system_version_without_rewriting_it() {
	let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../../docs/exporter/fda/FAERS2022Scenario6.xml");
	let source = std::fs::read_to_string(path).expect("prepared FDA XML fixture");
	let xml =
		source.replacen("codeSystemVersion=\"12.0\"", "codeSystemVersion=\"\"", 1);
	assert_ne!(xml, source, "fixture must contain the attribute to clear");
	let schema = default_xsd_path().expect("official ICH schema");
	let before = validate_e2b_xml_xsd(xml.as_bytes(), &schema).expect("validate");
	assert!(before
		.iter()
		.any(|error| error.message.contains("minLength")));

	assert!(
		validate_e2b_xml_for_import(xml.as_bytes(), None)
			.expect("import structure")
			.ok
	);
	assert!(xml.contains("codeSystemVersion=\"\""));
}

#[test]
fn import_rejects_multiple_reports_instead_of_merging_or_dropping_them() {
	let xml = br#"<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3"><investigationEvent/><investigationEvent/></MCCI_IN200100UV01>"#;
	assert!(validate_e2b_xml_for_import(xml, None).is_err());
}

#[test]
fn import_rejects_missing_or_unsupported_report_namespace() {
	for xml in [
		b"<MCCI_IN200100UV01 xmlns='urn:hl7-org:v3'/>".as_slice(),
		b"<MCCI_IN200100UV01 xmlns='urn:other'><investigationEvent/></MCCI_IN200100UV01>".as_slice(),
	] { assert!(validate_e2b_xml_for_import(xml, None).is_err()); }
}
