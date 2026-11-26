//! User flow example (FDA minimal): login -> create case -> validate -> export.

#[path = "../common/mod.rs"]
mod common;

use common::{FlowClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
	let client = FlowClient::login_from_env().await?;

	let (case_id, safety_report_id) = client.create_case("FDA-MIN").await?;
	println!(
		"created FDA case: case_id={case_id} safety_report_id={safety_report_id}"
	);

	client
		.seed_minimum_case_data(&case_id, "CDER", "ZZFDA")
		.await?;

	let validation = client.validate_case(&case_id, "fda").await?;
	println!(
		"validation(fda): {}",
		serde_json::to_string_pretty(&validation)?
	);

	let xml = client.export_xml(&case_id).await?;
	let output = client.write_export_to_dir("fda_minimal", &xml)?;
	println!("exported XML: {output}");

	Ok(())
}
