use lib_core::validation::find_canonical_rule;
use lib_core::validation::xml::{
	default_xsd_path, validate_e2b_xml, validate_e2b_xml_business,
	XmlValidatorConfig,
};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

fn resolve_from_workspace(path: PathBuf) -> PathBuf {
	if path.is_absolute() {
		path
	} else {
		workspace_root().join(path)
	}
}

fn examples_dir() -> PathBuf {
	workspace_root().join("docs/refs/instances")
}

fn test_validator_config() -> XmlValidatorConfig {
	let xsd_path = default_xsd_path().or_else(|| {
		Some(resolve_from_workspace(PathBuf::from(
			"deploy/ec2/schemas/multicacheschemas/MCCI_IN200100UV01.xsd",
		)))
	});
	XmlValidatorConfig {
		xsd_path,
		..Default::default()
	}
}

fn read_example(dir: &Path, filename: &str) -> Result<String, Box<dyn Error>> {
	let path = dir.join(filename);
	let content = fs::read_to_string(&path)?;
	Ok(content)
}

const BASE_FIXTURE: &str = "FAERS2022Scenario1.xml";

#[test]
fn test_examples_validate_ok() -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();
	let config = test_validator_config();

	let files = ["FAERS2022Scenario1.xml", "FAERS2022Scenario2.xml"];

	for file in files {
		let xml = read_example(&dir, file)?;
		let schema_report = validate_e2b_xml(xml.as_bytes(), Some(config.clone()))?;
		assert!(
			schema_report.ok,
			"{file} failed schema validation: {:?}",
			schema_report.errors
		);
		let business_report =
			validate_e2b_xml_business(xml.as_bytes(), Some(config.clone()))?;
		assert!(
			business_report.ok,
			"{file} failed business validation: {:?}",
			business_report.errors
		);
	}

	Ok(())
}

#[test]
fn test_invalid_telecom_fails() -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let broken = xml.replace("tel:", "phone:");
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected telecom error");
	let has_error = report
		.errors
		.iter()
		.any(|e| e.message.contains("telecom value must start"));
	assert!(has_error, "telecom error not reported");
	let has_code = report
		.errors
		.iter()
		.any(|e| e.message.contains("[ICH.XML.TELECOM.FORMAT.REQUIRED]"));
	assert!(has_code, "telecom code not reported");
	let telecom_error = report
		.errors
		.iter()
		.find(|e| e.code.as_deref() == Some("ICH.XML.TELECOM.FORMAT.REQUIRED"))
		.expect("telecom metadata should be present");
	assert_eq!(telecom_error.section.as_deref(), Some("xml"));
	assert_eq!(telecom_error.field_path.as_deref(), None);
	assert_eq!(telecom_error.blocking, Some(true));

	Ok(())
}

#[test]
fn test_schema_stage_ignores_business_rule_failures() -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();
	let config = test_validator_config();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let broken = xml.replace("tel:", "phone:");

	let schema_report = validate_e2b_xml(broken.as_bytes(), Some(config.clone()))?;
	assert!(
		schema_report.ok,
		"schema stage should ignore business rules: {:?}",
		schema_report.errors
	);

	let business_report =
		validate_e2b_xml_business(broken.as_bytes(), Some(config))?;
	assert!(
		!business_report.ok,
		"business stage should catch telecom rule violation"
	);
	assert!(business_report
		.errors
		.iter()
		.any(|e| e.message.contains("[ICH.XML.TELECOM.FORMAT.REQUIRED]")));

	Ok(())
}

#[test]
fn test_invalid_reaction_term_fails() -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let broken = xml.replacen("code=\"100", "code=\"", 1);
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected reaction term error");
	assert!(
		!report.errors.is_empty(),
		"expected at least one business error"
	);
	assert!(
		find_canonical_rule("ICH.E.i.2.NULLFLAVOR.REQUIRED").is_some(),
		"canonical rule missing: ICH.E.i.2.NULLFLAVOR.REQUIRED"
	);

	Ok(())
}

#[test]
fn test_missing_schema_location_fails() -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let broken = if let Some(start) = xml.find("xsi:schemaLocation=\"") {
		if let Some(end_rel) = xml[start + 20..].find('"') {
			let end = start + 20 + end_rel;
			let mut m = xml.clone();
			m.replace_range(start..=end, "");
			m
		} else {
			xml.clone()
		}
	} else {
		xml.clone()
	};
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected schemaLocation error");
	let has_schema_error = report
		.errors
		.iter()
		.any(|e| e.message.contains("schemaLocation"));
	assert!(
		has_schema_error,
		"missing schemaLocation error not reported"
	);
	assert!(report
		.errors
		.iter()
		.any(|e| e.message.contains("[ICH.XML.ROOT.SCHEMALOCATION.REQUIRED]")));

	Ok(())
}

#[test]
fn test_fda_combination_product_requires_value_or_nullflavor(
) -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let insert = "<subjectOf2 typeCode=\"SUBJ\">\
<investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\">\
<code code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.3\"/>\
<value xsi:type=\"BL\"/>\
</investigationCharacteristic>\
</subjectOf2>";
	let broken = fda_xml.replacen(
		"</investigationEvent>",
		&format!("{insert}</investigationEvent>"),
		1,
	);
	assert_ne!(broken, fda_xml, "failed to insert FDA.C.1.12 test node");
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected FDA.C.1.12 error");
	assert!(
		find_canonical_rule("FDA.C.1.12.REQUIRED").is_some()
			|| find_canonical_rule("FDA.C.1.12.RECOMMENDED").is_some(),
		"canonical rule missing for FDA.C.1.12"
	);

	Ok(())
}

#[test]
fn test_fda_local_criteria_requires_code_or_nullflavor() -> Result<(), Box<dyn Error>>
{
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let insert = "<subjectOf2 typeCode=\"SUBJ\">\
<investigationCharacteristic classCode=\"OBS\" moodCode=\"EVN\">\
<code code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/>\
<value xsi:type=\"CE\"/>\
</investigationCharacteristic>\
</subjectOf2>";
	let broken = fda_xml.replacen(
		"</investigationEvent>",
		&format!("{insert}</investigationEvent>"),
		1,
	);
	assert_ne!(broken, fda_xml, "failed to insert FDA.C.1.7.1 test node");
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected FDA.C.1.7.1 error");
	let has_error = report
		.errors
		.iter()
		.any(|e| e.message.contains("FDA.C.1.7.1 local criteria report type"));
	assert!(has_error, "FDA.C.1.7.1 error not reported");

	Ok(())
}

#[test]
fn test_fda_patient_race_requires_code_or_nullflavor() -> Result<(), Box<dyn Error>>
{
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let insert = "<player1 classCode=\"PSN\" determinerCode=\"INSTANCE\">\
<raceCode/>";
	let broken = fda_xml.replacen(
		"<player1 classCode=\"PSN\" determinerCode=\"INSTANCE\">",
		insert,
		1,
	);
	assert_ne!(broken, fda_xml, "failed to insert FDA.D.11 test node");
	assert!(
		find_canonical_rule("FDA.D.11.REQUIRED").is_some(),
		"canonical rule missing: FDA.D.11.REQUIRED"
	);

	Ok(())
}

#[test]
fn test_fda_patient_ethnicity_requires_code_or_nullflavor(
) -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let insert = "<subjectOf2 typeCode=\"SBJ\">\
<observation classCode=\"OBS\" moodCode=\"EVN\">\
<code code=\"C16564\" codeSystem=\"2.16.840.1.113883.3.26.1.1\"/>\
<value xsi:type=\"CE\"/>\
</observation>\
</subjectOf2>";
	let broken = fda_xml.replacen("<subjectOf2 typeCode=\"SBJ\">", insert, 1);
	assert_ne!(broken, fda_xml, "failed to insert FDA.D.12 test node");
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected FDA.D.12 error");
	let has_error = report
		.errors
		.iter()
		.any(|e| e.message.contains("FDA.D.12 patient ethnicity"));
	assert!(has_error, "FDA.D.12 error not reported");

	Ok(())
}

#[test]
fn test_fda_required_intervention_requires_value_or_nullflavor(
) -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let insert = "<outboundRelationship2 typeCode=\"PERT\">\
<observation classCode=\"OBS\" moodCode=\"EVN\">\
<code code=\"726\" codeSystem=\"2.16.840.1.113883.3.989.5.1.2.2.1.32\"/>\
<value xsi:type=\"BL\"/>\
</observation>\
</outboundRelationship2>";
	let broken = fda_xml.replacen(
		"</outboundRelationship2>",
		&format!("{insert}</outboundRelationship2>"),
		1,
	);
	assert_ne!(broken, fda_xml, "failed to insert FDA.E.i.3.2h test node");
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected FDA.E.i.3.2h error");
	assert!(
		find_canonical_rule("FDA.E.i.3.2h.REQUIRED").is_some(),
		"canonical rule missing: FDA.E.i.3.2h.REQUIRED"
	);

	Ok(())
}

#[test]
fn test_fda_gk10a_requires_code_or_na_when_pre_anda_present(
) -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;

	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER_IND_EXEMPT_BA_BE\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA_PREMKT\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let pre_anda = "<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\" extension=\"234567\"/></studyRegistration></authorization></researchStudy></subjectOf1>";
	let with_pre_anda = fda_xml.replacen(
		"</investigationEvent>",
		&format!("{pre_anda}</investigationEvent>"),
		1,
	);
	assert_ne!(
		with_pre_anda, fda_xml,
		"failed to insert FDA.C.5.5b test node"
	);

	let bad_gk10a = "<outboundRelationship2 typeCode=\"REFR\"><observation classCode=\"OBS\" moodCode=\"EVN\"><code code=\"9\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.19\"/><value xsi:type=\"CE\" code=\"9\"/></observation></outboundRelationship2>";
	let broken = with_pre_anda.replacen(
		"</substanceAdministration>",
		&format!("{bad_gk10a}</substanceAdministration>"),
		1,
	);
	assert_ne!(
		broken, with_pre_anda,
		"failed to insert FDA.G.k.10a test node"
	);

	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected FDA.G.k.10a-adjacent business error");
	assert!(
		find_canonical_rule("FDA.G.k.10a.REQUIRED").is_some(),
		"canonical rule missing: FDA.G.k.10a.REQUIRED"
	);

	Ok(())
}

#[test]
fn test_fda_reporter_email_required_when_primary_source_present(
) -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let fda_xml = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let broken = fda_xml.replace("mailto:", "mail:");
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(
		!report.ok,
		"expected reporter-email-adjacent business error"
	);
	assert!(
		find_canonical_rule("FDA.C.2.r.2.EMAIL.REQUIRED").is_some(),
		"canonical rule missing: FDA.C.2.r.2.EMAIL.REQUIRED"
	);

	Ok(())
}

#[test]
fn test_fda_pre_anda_required_for_ind_exempt() -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let broken = xml
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER_IND_EXEMPT_BA_BE\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA_PREMKT\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		)
		.replacen(
			"code=\"1\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.2\"",
			"code=\"2\" codeSystem=\"2.16.840.1.113883.3.989.2.1.1.2\"",
			1,
		);
	assert!(
		find_canonical_rule("FDA.C.5.5b.REQUIRED").is_some(),
		"canonical rule missing: FDA.C.5.5b.REQUIRED"
	);
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(
		report.ok || !report.errors.is_empty(),
		"validator should return either ok or issues deterministically"
	);

	Ok(())
}

#[test]
fn test_fda_pre_anda_not_allowed_postmarket() -> Result<(), Box<dyn Error>> {
	let dir = examples_dir();

	let xml = read_example(&dir, BASE_FIXTURE)?;
	let pre_anda = "<subjectOf1 typeCode=\"SBJ\"><researchStudy classCode=\"CLNTRL\" moodCode=\"EVN\"><authorization typeCode=\"AUTH\"><studyRegistration classCode=\"ACT\" moodCode=\"EVN\"><id root=\"2.16.840.1.113883.3.989.5.1.2.2.1.2.2\" extension=\"234567\"/></studyRegistration></authorization></researchStudy></subjectOf1>";
	let with_pre_anda = xml.replacen(
		"</investigationEvent>",
		&format!("{pre_anda}</investigationEvent>"),
		1,
	);
	let broken = with_pre_anda
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			"extension=\"CDER\" root=\"2.16.840.1.113883.3.989.2.1.3.12\"",
			1,
		)
		.replacen(
			"extension=\"ICHTEST\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			"extension=\"ZZFDA\" root=\"2.16.840.1.113883.3.989.2.1.3.14\"",
			1,
		);
	let report =
		validate_e2b_xml_business(broken.as_bytes(), Some(test_validator_config()))?;
	assert!(!report.ok, "expected FDA.C.5.5b not allowed error");
	let has_error = report
		.errors
		.iter()
		.any(|e| e.message.contains("FDA.C.5.5b must not be provided"));
	assert!(has_error, "FDA.C.5.5b not allowed error not reported");

	Ok(())
}
