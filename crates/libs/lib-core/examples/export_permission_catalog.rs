use lib_core::model::acs::all_permissions;
use std::fmt::Write as _;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let output = std::env::args_os()
		.nth(1)
		.map(PathBuf::from)
		.ok_or("usage: export_permission_catalog <output.ts>")?;
	let mut permissions = all_permissions()
		.iter()
		.map(ToString::to_string)
		.collect::<Vec<_>>();
	permissions.sort_unstable();
	permissions.dedup();

	let mut generated = String::from(
		"// Generated from lib-core's permission catalog. Do not edit.\n\nexport const Permission = {\n",
	);
	for permission in &permissions {
		let name = permission.replace('.', "");
		writeln!(generated, "  {name}: \"{permission}\",")?;
	}
	generated.push_str(
		"} as const;\n\nexport type PermissionValue = typeof Permission[keyof typeof Permission];\n\nexport const ALL_PERMISSIONS: readonly PermissionValue[] = Object.values(Permission);\n",
	);
	if let Some(parent) = output.parent() {
		std::fs::create_dir_all(parent)?;
	}
	std::fs::write(output, generated)?;
	Ok(())
}
