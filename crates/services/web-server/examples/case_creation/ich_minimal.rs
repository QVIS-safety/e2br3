//! User flow example (ICH minimal): login -> create case -> validate -> export.

#[path = "../common/mod.rs"]
mod common;

use common::{FlowClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
	let client = FlowClient::login_from_env().await?;

	let (case_id, safety_report_id) = client.create_case("ICH-MIN").await?;
	println!(
		"created ICH case: case_id={case_id} safety_report_id={safety_report_id}"
	);

	client
		.seed_minimum_case_data(&case_id, "ICHTEST", "ICHTEST")
		.await?;

	let validation = client.validate_case(&case_id, "ich").await?;
	println!(
		"validation(ich): {}",
		serde_json::to_string_pretty(&validation)?
	);

	let xml = client.export_xml(&case_id).await?;
	let output = client.write_export_to_dir("ich_minimal", &xml)?;
	println!("exported XML: {output}");

	Ok(())
}
