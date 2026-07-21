use lib_core::authorization::{export_contract, policy_registry};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let output = std::env::args_os()
		.nth(1)
		.map(PathBuf::from)
		.ok_or("usage: export_authorization_contract <output.ts>")?;
	let contract = export_contract(policy_registry())?;
	if let Some(parent) = output.parent() {
		std::fs::create_dir_all(parent)?;
	}
	std::fs::write(output, contract.typescript)?;
	Ok(())
}
