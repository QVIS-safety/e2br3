use std::fmt::Write as _;
use std::path::PathBuf;
use web_server::web::rest::permission_contract::ENDPOINT_PERMISSION_CONTRACTS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let output = std::env::args_os()
		.nth(1)
		.map(PathBuf::from)
		.ok_or("usage: export_permission_contract <output.ts>")?;
	let mut contracts = ENDPOINT_PERMISSION_CONTRACTS.to_vec();
	contracts.sort_unstable_by_key(|contract| contract.key);
	let mut generated = String::from("// Generated from web-server endpoint permission contracts. Do not edit.\n\nexport const ENDPOINT_PERMISSIONS = {\n");
	for contract in contracts {
		write!(generated, "  \"{}\": [", contract.key)?;
		for (index, permission) in contract.permissions.iter().enumerate() {
			if index > 0 {
				generated.push_str(", ");
			}
			write!(generated, "\"{permission}\"")?;
		}
		writeln!(generated, "],")?;
	}
	generated.push_str("} as const;\n\nexport type EndpointPermissionKey = keyof typeof ENDPOINT_PERMISSIONS;\n");
	if let Some(parent) = output.parent() {
		std::fs::create_dir_all(parent)?;
	}
	std::fs::write(output, generated)?;
	Ok(())
}
