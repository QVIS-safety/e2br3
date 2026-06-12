//! User flow example (FDA import/export): login -> import XML -> validate -> export.

#[path = "../common/mod.rs"]
mod common;

use common::{FlowClient, Result};
use std::path::PathBuf;

fn default_xml_path() -> PathBuf {
	std::env::var("E2BR3_IMPORT_XML_FDA")
		.map(PathBuf::from)
		.unwrap_or_else(|_| {
			PathBuf::from("docs/exporter/fda/FAERS2022Scenario1.xml")
		})
}

#[tokio::main]
async fn main() -> Result<()> {
	let client = FlowClient::login_from_env().await?;

	let xml_path = default_xml_path();
	let case_id = client.import_xml_file(&xml_path).await?;
	println!(
		"imported FDA XML from {} -> case_id={case_id}",
		xml_path.display()
	);

	let validation = client.validate_case(&case_id, "fda").await?;
	println!(
		"validation(fda): {}",
		serde_json::to_string_pretty(&validation)?
	);

	client.mark_case_validated(&case_id).await?;
	let xml = client.export_xml(&case_id).await?;
	let output = client.write_export_to_dir("fda_import_roundtrip", &xml)?;
	println!("exported XML: {output}");

	Ok(())
}
