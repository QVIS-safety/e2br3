use libxml::parser::Parser;
use libxml::xpath::Context;

use xml::export::roundtrip::{patch_c_safety_report, CSafetyReportPatch};

#[test]
fn patch_c_section_updates_values() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");
	let patch = CSafetyReportPatch {
		report_unique_id: "SR-TEST-123",
		transmission_date: Some("20240115"),
		report_type: "1",
		date_first_received: Some("20240110"),
		date_most_recent: Some("20240115"),
		fulfil_expedited: Some(true),
		additional_documents_available: Some(false),
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		worldwide_unique_id: Some("WW-TEST-999"),
		first_sender_type: None,
		local_criteria_report_type: Some("1"),
		combination_product_indicator: Some("false"),
		combination_product_indicator_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		sender_type: Some("1"),
		sender_health_professional_type_kr1: None,
		sender_org_name: Some("QVIS Safety CRO"),
		sender_department: Some("Drug Safety"),
		sender_street_address: Some("200 Regulatory Avenue"),
		sender_city: Some("Seoul"),
		sender_state: Some("Seoul"),
		sender_postcode: Some("04524"),
		sender_country_code: Some("KR"),
		sender_person_title: Some("Ms"),
		sender_person_given_name: Some("Sara"),
		sender_person_middle_name: Some("J"),
		sender_person_family_name: Some("Lee"),
		sender_telephone: Some("+82-2-555-0100"),
		sender_fax: Some("+82-2-555-0101"),
		sender_email: Some("safety.sender@example.com"),
		..Default::default()
	};

	let patched = patch_c_safety_report(&xml, &patch).expect("patch xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse patched");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

	let report_id = xpath
		.findvalue(
			"//hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.1']/@extension",
			None,
		)
		.unwrap();
	assert_eq!(report_id, "SR-TEST-123");

	let worldwide_id = xpath
		.findvalue(
			"//hl7:investigationEvent/hl7:id[@root='2.16.840.1.113883.3.989.2.1.3.2']/@extension",
			None,
		)
		.unwrap();
	assert_eq!(worldwide_id, "WW-TEST-999");

	for (path, expected) in [
		("//hl7:component/hl7:observationEvent[hl7:code/@code='1']/hl7:value/@value", "false"),
		("//hl7:component/hl7:observationEvent[hl7:code/@code='C54588']/hl7:value/@code", "1"),
		("//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:code/@code", "1"),
		("//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:representedOrganization/hl7:name", "Drug Safety"),
		("//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:representedOrganization/hl7:assignedEntity/hl7:representedOrganization/hl7:name", "QVIS Safety CRO"),
		("//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:assignedPerson/hl7:name/hl7:given[2]", "J"),
		("//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:addr/hl7:city", "Seoul"),
		("//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code/@code", "KR"),
		("//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:telecom[starts-with(@value,'tel:')]/@value", "tel:+82-2-555-0100"),
	] {
		assert_eq!(xpath.findvalue(path, None).unwrap(), expected, "{path}");
	}
}

fn sender_patch<'a>(
	telephone: Option<&'a str>,
	fax: Option<&'a str>,
	email: Option<&'a str>,
) -> CSafetyReportPatch<'a> {
	CSafetyReportPatch {
		report_unique_id: "SENDER-TEST",
		transmission_date: None,
		report_type: "1",
		date_first_received: None,
		date_most_recent: None,
		fulfil_expedited: Some(false),
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		local_criteria_report_type: None,
		combination_product_indicator: None,
		combination_product_indicator_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		sender_type: None,
		sender_health_professional_type_kr1: None,
		sender_org_name: None,
		sender_department: None,
		sender_street_address: None,
		sender_city: None,
		sender_state: None,
		sender_postcode: None,
		sender_country_code: None,
		sender_person_title: None,
		sender_person_given_name: None,
		sender_person_middle_name: None,
		sender_person_family_name: None,
		sender_telephone: telephone,
		sender_fax: fax,
		sender_email: email,
		..Default::default()
	}
}

#[test]
fn patch_c_sender_telecoms_omit_empty_uris_and_export_populated_values() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let skeleton = std::fs::read(
		root.join("crates/libs/xml/src/fixtures/base_export_skeleton.xml"),
	)
	.expect("read export skeleton");
	let parser = Parser::default();

	let empty = patch_c_safety_report(&skeleton, &sender_patch(None, None, None))
		.expect("patch empty sender telecoms");
	let empty_doc = parser.parse_string(&empty).expect("parse empty sender XML");
	let mut empty_xpath = Context::new(&empty_doc).expect("empty xpath");
	empty_xpath
		.register_namespace("hl7", "urn:hl7-org:v3")
		.unwrap();
	for prefix in ["tel:", "fax:", "mailto:"] {
		assert_eq!(
			empty_xpath
				.findvalue(
					&format!(
						"count(//hl7:investigationEvent/hl7:subjectOf1//hl7:telecom[@value='{prefix}'])"
					),
					None,
				)
				.unwrap(),
			"0",
		);
	}

	let populated = patch_c_safety_report(
		&skeleton,
		&sender_patch(
			Some("+82-2-5555-0102"),
			Some("fax:+82-2-5555-0103"),
			Some("pv@qvis-safety.example"),
		),
	)
	.expect("patch populated sender telecoms");
	let populated_doc = parser
		.parse_string(&populated)
		.expect("parse populated sender XML");
	let mut populated_xpath = Context::new(&populated_doc).expect("populated xpath");
	populated_xpath
		.register_namespace("hl7", "urn:hl7-org:v3")
		.unwrap();
	assert_eq!(
		populated_xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1//hl7:telecom[starts-with(@value,'tel:')]/@value",
				None,
			)
			.unwrap(),
		"tel:+82-2-5555-0102"
	);
	assert_eq!(
		populated_xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1//hl7:telecom[starts-with(@value,'fax:')]/@value",
				None,
			)
			.unwrap(),
		"fax:+82-2-5555-0103"
	);
	assert_eq!(
		populated_xpath
			.findvalue(
				"//hl7:investigationEvent/hl7:subjectOf1//hl7:telecom[starts-with(@value,'mailto:')]/@value",
				None,
			)
			.unwrap(),
		"mailto:pv@qvis-safety.example"
	);
}

#[test]
fn patch_c_writes_sender_values_when_optional_nodes_are_absent() {
	let sparse = br#"<?xml version="1.0" encoding="UTF-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess classCode="CACT" moodCode="EVN">
      <subject><investigationEvent classCode="INVSTG" moodCode="EVN">
        <subjectOf1 typeCode="SUBJ"><controlActEvent classCode="CACT" moodCode="EVN">
          <author typeCode="AUT"><assignedEntity classCode="ASSIGNED"/>
          </author>
        </controlActEvent></subjectOf1>
      </investigationEvent></subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>"#;
	let mut patch = sender_patch(
		Some("tel:+82-2-5555-0102"),
		Some("fax:+82-2-5555-0103"),
		Some("mailto:pv@qvis-safety.example"),
	);
	patch.sender_type = Some("3");
	patch.sender_department = Some("Pharmacovigilance");
	patch.sender_person_title = Some("Dr");
	patch.sender_person_given_name = Some("Sora");
	patch.sender_person_family_name = Some("Kim");
	patch.sender_street_address = Some("1 Test Road");
	patch.sender_city = Some("Seoul");
	patch.sender_state = Some("Seoul");
	patch.sender_postcode = Some("04524");
	patch.sender_country_code = Some("KR");

	let patched =
		patch_c_safety_report(sparse, &patch).expect("patch sparse sender XML");
	let parser = Parser::default();
	let doc = parser
		.parse_string(&patched)
		.expect("parse sparse sender XML");
	let mut xpath = Context::new(&doc).expect("sender xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	let base = "//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity";
	for (path, expected) in [
		(format!("{base}/hl7:code/@code"), "3"),
		(
			format!("{base}/hl7:representedOrganization/hl7:name"),
			"Pharmacovigilance",
		),
		(
			format!("{base}/hl7:assignedPerson/hl7:name/hl7:prefix"),
			"Dr",
		),
		(
			format!("{base}/hl7:assignedPerson/hl7:name/hl7:given[1]"),
			"Sora",
		),
		(
			format!("{base}/hl7:assignedPerson/hl7:name/hl7:family"),
			"Kim",
		),
		(
			format!("{base}/hl7:addr/hl7:streetAddressLine"),
			"1 Test Road",
		),
		(format!("{base}/hl7:addr/hl7:city"), "Seoul"),
		(format!("{base}/hl7:addr/hl7:state"), "Seoul"),
		(format!("{base}/hl7:addr/hl7:postalCode"), "04524"),
		(
			format!(
				"{base}/hl7:assignedPerson/hl7:asLocatedEntity/hl7:location/hl7:code/@code"
			),
			"KR",
		),
	] {
		assert_eq!(xpath.findvalue(&path, None).unwrap(), expected, "{path}");
	}
	for (prefix, expected) in [
		("tel:", "tel:+82-2-5555-0102"),
		("fax:", "fax:+82-2-5555-0103"),
		("mailto:", "mailto:pv@qvis-safety.example"),
	] {
		assert_eq!(
			xpath
				.findvalue(
					&format!(
						"{base}/hl7:telecom[starts-with(@value,'{prefix}')]/@value"
					),
					None,
				)
				.unwrap(),
			expected,
			"{prefix} URI",
		);
	}
}

#[test]
fn patch_c_1_9_1_exports_boolean_value_without_ce_children() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read_to_string(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml")
		.replace(
			r#"<value xsi:type="BL" nullFlavor="NI"/>"#,
			r#"<value xsi:type="CE" code="legacy"><originalText>legacy</originalText></value>"#,
		);
	let patch = CSafetyReportPatch {
		report_unique_id: "FAERS2022Scenario1",
		transmission_date: None,
		report_type: "1",
		date_first_received: None,
		date_most_recent: None,
		fulfil_expedited: Some(true),
		additional_documents_available: None,
		other_case_identifiers_exist: Some(true),
		other_case_identifiers_exist_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		local_criteria_report_type: None,
		combination_product_indicator: None,
		combination_product_indicator_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		sender_type: None,
		sender_health_professional_type_kr1: None,
		sender_org_name: None,
		sender_department: None,
		sender_street_address: None,
		sender_city: None,
		sender_state: None,
		sender_postcode: None,
		sender_country_code: None,
		sender_person_title: None,
		sender_person_given_name: None,
		sender_person_middle_name: None,
		sender_person_family_name: None,
		sender_telephone: None,
		sender_fax: None,
		sender_email: None,
		..Default::default()
	};

	let patched = patch_c_safety_report(xml.as_bytes(), &patch).expect("patch xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse patched");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();
	xpath
		.register_namespace("xsi", "http://www.w3.org/2001/XMLSchema-instance")
		.unwrap();

	let value_path = "//hl7:investigationCharacteristic[hl7:code[@code='2' and @codeSystem='2.16.840.1.113883.3.989.2.1.1.23']]/hl7:value";
	assert_eq!(
		xpath
			.findvalue(&format!("{value_path}/@xsi:type"), None)
			.unwrap(),
		"BL"
	);
	assert_eq!(
		xpath
			.findvalue(&format!("{value_path}/@value"), None)
			.unwrap(),
		"true"
	);
	assert_eq!(
		xpath
			.findvalue(&format!("count({value_path}/hl7:originalText)"), None)
			.unwrap(),
		"0"
	);
	assert_eq!(
		xpath
			.findvalue(
				&format!("count({value_path}/@code | {value_path}/@codeSystem | {value_path}/@nullFlavor)"),
				None,
			)
			.unwrap(),
		"0"
	);
	assert!(
		xml::validate_e2b_xml_basic(patched.as_bytes(), None)
			.expect("validate XML")
			.ok
	);
}

#[test]
fn patch_c_uses_persisted_c1_2() {
	let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.expect("workspace root")
		.to_path_buf();
	let xml = std::fs::read(root.join("docs/exporter/fda/FAERS2022Scenario1.xml"))
		.expect("read sample xml");
	let patch = CSafetyReportPatch {
		report_unique_id: "SR-TEST-124",
		transmission_date: Some("20240115"),
		report_type: "1",
		date_first_received: Some("20240110"),
		date_most_recent: Some("20240115"),
		fulfil_expedited: Some(true),
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		local_criteria_report_type: None,
		combination_product_indicator: None,
		combination_product_indicator_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		sender_type: None,
		sender_health_professional_type_kr1: None,
		sender_org_name: None,
		sender_department: None,
		sender_street_address: None,
		sender_city: None,
		sender_state: None,
		sender_postcode: None,
		sender_country_code: None,
		sender_person_title: None,
		sender_person_given_name: None,
		sender_person_middle_name: None,
		sender_person_family_name: None,
		sender_telephone: None,
		sender_fax: None,
		sender_email: None,
		..Default::default()
	};

	let patched = patch_c_safety_report(&xml, &patch).expect("patch xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse patched");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

	let c1_2 = xpath
		.findvalue("//hl7:controlActProcess/hl7:effectiveTime/@value", None)
		.unwrap();
	assert_eq!(c1_2, "20240115");
}

#[test]
fn patch_c_keeps_investigation_event_order_when_adding_components() {
	let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess classCode="CACT" moodCode="EVN">
      <subject>
        <investigationEvent classCode="INVSTG" moodCode="EVN">
          <id root="2.16.840.1.113883.3.989.2.1.3.1" extension="CASE-1"/>
          <subjectOf2 typeCode="SUBJ">
            <investigationCharacteristic classCode="OBS" moodCode="EVN">
              <code code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.23"/>
              <value xsi:type="CE" code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.2"><originalText/></value>
            </investigationCharacteristic>
          </subjectOf2>
        </investigationEvent>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>"#;

	let patch = CSafetyReportPatch {
		report_unique_id: "CASE-1",
		transmission_date: Some("20240115"),
		report_type: "1",
		date_first_received: Some("20240110"),
		date_most_recent: Some("20240115"),
		fulfil_expedited: Some(true),
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		local_criteria_report_type: None,
		combination_product_indicator: Some("true"),
		combination_product_indicator_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		sender_type: None,
		sender_health_professional_type_kr1: None,
		sender_org_name: None,
		sender_department: None,
		sender_street_address: None,
		sender_city: None,
		sender_state: None,
		sender_postcode: None,
		sender_country_code: None,
		sender_person_title: None,
		sender_person_given_name: None,
		sender_person_middle_name: None,
		sender_person_family_name: None,
		sender_telephone: None,
		sender_fax: None,
		sender_email: None,
		..Default::default()
	};

	let patched = patch_c_safety_report(xml, &patch).expect("patch xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse patched");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

	let late_component_count = xpath
		.findvalue(
			"count(//hl7:investigationEvent/hl7:subjectOf2/following-sibling::hl7:component)",
			None,
		)
		.unwrap();
	assert_eq!(late_component_count, "0");

	let inserted_component_count = xpath
		.findvalue(
			"count(//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='C156384']])",
			None,
		)
		.unwrap();
	assert_eq!(inserted_component_count, "1");
}

#[test]
fn patch_c_keeps_order_when_adding_local_criteria_component() {
	let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess classCode="CACT" moodCode="EVN">
      <subject>
        <investigationEvent classCode="INVSTG" moodCode="EVN">
          <id root="2.16.840.1.113883.3.989.2.1.3.1" extension="CASE-2"/>
          <subjectOf2 typeCode="SUBJ">
            <investigationCharacteristic classCode="OBS" moodCode="EVN">
              <code code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.23"/>
              <value xsi:type="CE" code="2" codeSystem="2.16.840.1.113883.3.989.2.1.1.2"><originalText/></value>
            </investigationCharacteristic>
          </subjectOf2>
          <subjectOf1 typeCode="SUBJ">
            <controlActEvent classCode="CACT" moodCode="EVN">
              <author typeCode="AUT">
                <assignedEntity classCode="ASSIGNED">
                  <code code="1"/>
                </assignedEntity>
              </author>
            </controlActEvent>
          </subjectOf1>
        </investigationEvent>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>"#;

	let patch = CSafetyReportPatch {
		report_unique_id: "CASE-2",
		transmission_date: Some("20240115"),
		report_type: "2",
		date_first_received: Some("20240110"),
		date_most_recent: Some("20240115"),
		fulfil_expedited: Some(true),
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		local_criteria_report_type: Some("2"),
		combination_product_indicator: None,
		combination_product_indicator_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		sender_type: None,
		sender_health_professional_type_kr1: None,
		sender_org_name: None,
		sender_department: None,
		sender_street_address: None,
		sender_city: None,
		sender_state: None,
		sender_postcode: None,
		sender_country_code: None,
		sender_person_title: None,
		sender_person_given_name: None,
		sender_person_middle_name: None,
		sender_person_family_name: None,
		sender_telephone: None,
		sender_fax: None,
		sender_email: None,
		..Default::default()
	};

	let patched = patch_c_safety_report(xml, &patch).expect("patch xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse patched");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

	let late_component_count = xpath
		.findvalue(
			"count(//hl7:investigationEvent/hl7:subjectOf2/following-sibling::hl7:component)",
			None,
		)
		.unwrap();
	assert_eq!(late_component_count, "0");

	let inserted_component_count = xpath
		.findvalue(
			"count(//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='C54588']])",
			None,
		)
		.unwrap();
	assert_eq!(inserted_component_count, "1");

	let local_criteria_code = xpath
		.findvalue(
			"//hl7:investigationEvent/hl7:component/hl7:observationEvent[hl7:code[@code='C54588']]/hl7:value/@code",
			None,
		)
		.unwrap();
	assert_eq!(local_criteria_code, "2");
}

#[test]
fn patch_c_exports_sender_health_professional_type_kr1() {
	let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<MCCI_IN200100UV01 xmlns="urn:hl7-org:v3" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <PORR_IN049016UV>
    <controlActProcess classCode="CACT" moodCode="EVN">
      <subject>
        <investigationEvent classCode="INVSTG" moodCode="EVN">
          <id root="2.16.840.1.113883.3.989.2.1.3.1" extension="CASE-3"/>
          <subjectOf2 typeCode="SUBJ">
            <investigationCharacteristic classCode="OBS" moodCode="EVN">
              <code code="1" codeSystem="2.16.840.1.113883.3.989.2.1.1.23"/>
              <value xsi:type="CE" code="2" codeSystem="2.16.840.1.113883.3.989.2.1.1.2"><originalText/></value>
            </investigationCharacteristic>
          </subjectOf2>
          <subjectOf1 typeCode="SUBJ">
            <controlActEvent classCode="CACT" moodCode="EVN">
              <author typeCode="AUT">
                <assignedEntity classCode="ASSIGNED">
                  <code code="1"/>
                </assignedEntity>
              </author>
            </controlActEvent>
          </subjectOf1>
        </investigationEvent>
      </subject>
    </controlActProcess>
  </PORR_IN049016UV>
</MCCI_IN200100UV01>"#;

	let patch = CSafetyReportPatch {
		report_unique_id: "CASE-3",
		transmission_date: Some("20240115"),
		report_type: "2",
		date_first_received: Some("20240110"),
		date_most_recent: Some("20240115"),
		fulfil_expedited: Some(true),
		additional_documents_available: None,
		other_case_identifiers_exist: None,
		other_case_identifiers_exist_null_flavor: None,
		worldwide_unique_id: None,
		first_sender_type: None,
		local_criteria_report_type: None,
		combination_product_indicator: None,
		combination_product_indicator_null_flavor: None,
		nullification_code: None,
		nullification_reason: None,
		sender_type: Some("3"),
		sender_health_professional_type_kr1: Some("4"),
		sender_org_name: None,
		sender_department: None,
		sender_street_address: None,
		sender_city: None,
		sender_state: None,
		sender_postcode: None,
		sender_country_code: None,
		sender_person_title: None,
		sender_person_given_name: None,
		sender_person_middle_name: None,
		sender_person_family_name: None,
		sender_telephone: None,
		sender_fax: None,
		sender_email: None,
		..Default::default()
	};

	let patched = patch_c_safety_report(xml, &patch).expect("patch xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse patched");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

	let sender_kr1 = xpath
		.findvalue(
			"//hl7:investigationEvent/hl7:subjectOf1/hl7:controlActEvent/hl7:author/hl7:assignedEntity/hl7:assignedPerson/hl7:asQualifiedEntity/hl7:code[@codeSystem='2.16.840.1.113883.3.989.5.1.10.1.2']/@code",
			None,
		)
		.unwrap();
	assert_eq!(sender_kr1, "4");
}

#[test]
fn patch_c_inserts_first_sender_relationship_when_skeleton_lacks_it() {
	let xml = std::fs::read(
		std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("src/fixtures/base_export_skeleton.xml"),
	)
	.expect("read export skeleton");
	let mut patch = sender_patch(None, None, None);
	patch.first_sender_type = Some("1");

	let patched = patch_c_safety_report(&xml, &patch).expect("patch xml");
	let parser = Parser::default();
	let doc = parser.parse_string(&patched).expect("parse patched");
	let mut xpath = Context::new(&doc).expect("xpath");
	xpath.register_namespace("hl7", "urn:hl7-org:v3").unwrap();

	assert_eq!(
		xpath
			.findvalue(
				"count(//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']])",
				None,
			)
			.unwrap(),
		"1"
	);
	assert_eq!(
		xpath
			.findvalue(
				"//hl7:outboundRelationship[hl7:relatedInvestigation/hl7:code[@code='1']]//hl7:assignedEntity/hl7:code/@code",
				None,
			)
			.unwrap(),
		"1"
	);
}
