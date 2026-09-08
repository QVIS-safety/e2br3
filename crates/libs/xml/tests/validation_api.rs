use lib_core::regulatory::RegulatoryAuthority;
use xml::validation::{
	default_xsd_path, normalize_e2b_xml_for_import, validate_e2b_xml,
	validate_e2b_xml_basic, validate_e2b_xml_for_import, validate_e2b_xml_xsd,
	XmlValidatorConfig,
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
fn import_normalization_treats_empty_c_1_8_1_extension_as_absent() {
	let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
		"../../../docs/exporter/mfds/1-1_ExampleCase_literature_KR_initial_v1_0_샘플.xml",
	);
	let Ok(source) = std::fs::read_to_string(path) else {
		return;
	};
	let xml = source.replace(
		"<id extension=\"KIDS-81200923025560\" root=\"2.16.840.1.113883.3.989.2.1.3.2\"/>",
		"<id extension=\"\" root=\"2.16.840.1.113883.3.989.2.1.3.2\"/>",
	);
	let schema = default_xsd_path().expect("official ICH schema");
	let before = validate_e2b_xml_xsd(xml.as_bytes(), &schema).expect("validate");
	assert!(before
		.iter()
		.any(|error| error.message.contains("minLength")));

	let normalized =
		normalize_e2b_xml_for_import(xml.as_bytes()).expect("normalize");
	let normalized = String::from_utf8(normalized).expect("UTF-8 XML");
	assert!(!normalized.contains("extension=\"\""));
	assert!(normalized.contains("root=\"2.16.840.1.113883.3.989.2.1.3.2\""));
	assert!(validate_e2b_xml_xsd(normalized.as_bytes(), &schema)
		.expect("validate normalized XML")
		.is_empty());
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
	let normalized =
		normalize_e2b_xml_for_import(xml.as_bytes()).expect("normalize");
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
	let invalid = xml.replacen("<id ", "<id invalidAttribute=\"true\" ", 1);
	let report = validate_e2b_xml_for_import(invalid.as_bytes(), None).unwrap();
	assert!(
		!report.ok,
		"MFDS extension must not hide unrelated XSD errors"
	);
	assert!(!report.errors.is_empty());
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
fn import_normalization_treats_empty_code_system_version_as_absent() {
	let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
		"../../../docs/exporter/mfds/1-1_ExampleCase_literature_KR_initial_v1_0_샘플.xml",
	);
	let Ok(source) = std::fs::read_to_string(path) else {
		return;
	};
	let xml =
		source.replacen("codeSystemVersion=\"22.1\"", "codeSystemVersion=\"\"", 1);
	let schema = default_xsd_path().expect("official ICH schema");
	let before = validate_e2b_xml_xsd(xml.as_bytes(), &schema).expect("validate");
	assert!(before
		.iter()
		.any(|error| error.message.contains("minLength")));

	let normalized =
		normalize_e2b_xml_for_import(xml.as_bytes()).expect("normalize");
	let normalized = String::from_utf8(normalized).expect("UTF-8 XML");
	assert!(!normalized.contains("codeSystemVersion=\"\""));
	assert!(normalized.contains("codeSystemVersion=\"22.1\""));
	assert!(validate_e2b_xml_xsd(normalized.as_bytes(), &schema)
		.expect("validate normalized XML")
		.is_empty());
}
