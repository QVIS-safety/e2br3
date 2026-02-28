use lib_core::xml::{
	validate_e2b_xml, validate_e2b_xml_business, XmlValidatorConfig,
};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../..")
		.canonicalize()
		.expect("workspace root")
}

fn sample_xml_path() -> PathBuf {
	workspace_root().join("docs/refs/instances/FAERS2022Scenario2.xml")
}

fn xsd_path() -> PathBuf {
	workspace_root()
		.join("deploy/ec2/schemas/multicacheschemas/MCCI_IN200100UV01.xsd")
}

#[test]
fn schema_and_business_validation_pass_for_repo_fixture(
) -> Result<(), Box<dyn Error>> {
	let xml = fs::read(sample_xml_path())?;
	let config = XmlValidatorConfig {
		xsd_path: Some(xsd_path()),
		..XmlValidatorConfig::default()
	};

	let schema_report = validate_e2b_xml(&xml, Some(config.clone()))?;
	assert!(
		schema_report.ok,
		"schema validation failed for repo fixture: {:?}",
		schema_report.errors
	);

	let business_report = validate_e2b_xml_business(&xml, Some(config))?;
	assert!(
		business_report.ok,
		"business validation failed for repo fixture: {:?}",
		business_report.errors
	);

	Ok(())
}
